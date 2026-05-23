# tg16 Decisions Log

> **Architectural note (2026-05-16 libretro pivot):** The 2026-05-15 entries below describe the original `crates/oa-pce-sys` + `crates/oa-pce` vendored static-crate integration. That integration was retired by the project-wide 2026-05-16 "Architecture pivot: libretro frontend" decision in `docs/DECISIONS.md`. Entries from 2026-05-16 forward describe the libretro-frontend tier. The 2026-05-15 entries are kept as historical reference for the reasoning behind the original FFI surface and Mednafen choice.

Append-only. Newest at the bottom. Every entry: **what** we decided, **when**, **why**, and **what we considered and rejected**.

tg16-specific integration choices live here. Project-wide decisions (engine stack, license, vendoring policy, no-per-core-ARCHITECTURE rule, etc.) live in `docs/DECISIONS.md`.

---

## 2026-05-15 — Use Beetle PCE Fast (not full Mednafen PCE) for the HuCard core

**Decision:** `crates/oa-pce-sys` vendors the libretro Beetle PCE Fast core. The full Mednafen PCE module is not vendored.

**Why:** Beetle PCE Fast is the de-facto HuCard reference in the libretro ecosystem — Mednafen PCE Fast lineage, smaller surface area than full Mednafen, ships its own copy of pcecd if we ever want it. For Phase 1 (HuCard end-to-end) it gives us everything we need with the smallest C/C++ blast radius. License (GPLv2) matches the project-wide GPLv2-binary-wide decision.

**Considered and rejected:**
- **Full Mednafen PCE.** Bigger build, more sub-systems we don't need yet. May still vendor separately for Phase 5 (PCE-CD) if Beetle PCE Fast's `pcecd.cpp` proves insufficient — see `ROADMAP.md` Phase 5 open question.
- **MAME PCE driver.** Compatibility weaker than Mednafen for the HuCard library; licensing flexibility (BSD-3-like) doesn't matter since we're already GPLv2 elsewhere.

---

## 2026-05-15 — Hand-written `extern "C"` FFI surface (no bindgen)

**Decision:** `oa-pce-sys/src/lib.rs` declares the `oa_pce_*` surface by hand. No `bindgen` build step.

**Why:** This is the application of the project-wide policy from `docs/DECISIONS.md` ("Spike 3 outcome"). For tg16 specifically: the `oa_pce_*` surface is ~9 functions. Hand-writing them costs one line in `lib.rs` per function; bindgen would add 27 transitive crates + libclang dependency + 4.8× longer cold builds for zero benefit on a 9-function surface.

---

## 2026-05-15 — Shim layer wraps libretro frontend callbacks, not Mednafen internals

**Decision:** `crates/oa-pce-sys/shim.cpp` implements the libretro frontend (`retro_set_video_refresh`, `retro_set_audio_sample_batch`, `retro_set_environment`, `retro_set_input_state`, `retro_set_input_poll`, log callback) and exposes a small `oa_pce_*` C surface on top of libretro's `retro_init / retro_load_game / retro_run / retro_serialize / retro_unload_game / retro_deinit` lifecycle. We do **not** call Mednafen's `MDFN_*` APIs directly.

**Why:** The libretro layer already exists in the vendored core, fully wired to Mednafen's internals. Bypassing it would mean re-implementing what `libretro.cpp` already does (environment setup, controller port management, audio batching, video frame buffering) — pure duplicated work, and we'd lose the framework that future libretro-derived cores use. Going through libretro also means the same shim shape works for any future libretro core (sets us up for the per-system 8-step recipe in `feedback_multi_core_architecture_ready`).

**Considered and rejected:**
- **Direct `MDFN_*` calls into Mednafen.** Cleaner in theory; in practice would mean re-doing libretro's frontend job for every core. The libretro shim is the abstraction layer we're meant to build against.
- **Use upstream libretro `libretro.cpp` as-is and Tauri-side handle frontend callbacks.** Cross-language callback hops at 60 Hz, brittle lifetime management. Our `shim.cpp` is the right place to keep that complexity in C++.

---

## 2026-05-15 — PCE button bitfield order is ours, not libretro's

**Decision:** `oa_pce::buttons` defines a stable Rust-side bitfield layout (`I=1<<0`, `II=1<<1`, `SELECT=1<<2`, `RUN=1<<3`, `UP=1<<4`, `RIGHT=1<<5`, `DOWN=1<<6`, `LEFT=1<<7`). `PceCore::set_input` translates this to libretro's `RETRO_DEVICE_ID_JOYPAD_*` numbering inside the wrapper.

**Why:** libretro's button numbering is quirky (`A=8`, `B=0`, `SELECT=2`, `START=3`, dpad split across 4-7) for backward-compatibility reasons. Pushing that onto the shell + input-mapper would leak libretro-specific knowledge into otherwise system-agnostic code. Keeping the translation inside `oa-pce` means the shell and `oa-input` only know the system's logical button names, the same way they'll know SMS buttons or Lynx buttons. Translation is a single function (`pce_to_retro_bits`) with a unit test.

**Considered and rejected:**
- **Expose libretro numbering directly to `oa-input`.** Would force every future core to learn libretro's quirks even when wrapping a non-libretro core (Mednafen-direct, MAME-direct, custom).

---

## 2026-05-15 — Default gamepad face buttons: South = II, East = I

**Decision:** Default PCE pad map: `East = buttons::I`, `South = buttons::II` (gilrs naming — `South` is the bottom face button, `East` is the right face button; e.g. Xbox A and B respectively, or Switch B and A).

**Why:** Matches RetroArch's Beetle PCE Fast default. PCE's physical I button sits to the right of II on the controller, so mapping I → "right face" / II → "south face" preserves the spatial intuition users coming from RetroArch already have. Avoids surprises for the existing PCE community.

**Considered and rejected:**
- **South = I, East = II.** Closer to "primary action = bottom button" convention from modern consoles, but breaks muscle memory for anyone who's played PCE on RetroArch.

---

## 2026-05-15 — `retro_set_controller_port_device` runs AFTER `retro_load_game`, not before

**Decision:** In `shim.cpp`, `retro_set_controller_port_device(port, RETRO_DEVICE_JOYPAD)` is called only **after** `retro_load_game` returns successfully. Pre-load wiring is silently dropped by Mednafen-derived cores.

**Why:** Mednafen's `MDFNI_LoadGame` re-initializes the core and resets `pce_fast/input.c`'s `data_ptr[]` array. Any pre-load `retro_set_controller_port_device` call gets clobbered — the array points nowhere, so `INPUT_Frame` reads zeros, and the player can't move. Took 4 iterations of diagnostic logging in `cb_input_state` to localize the first time. Captured in memory as `reference_libretro_controller_after_load_game` because it applies to **every** Mednafen-derived libretro core we wrap, not just PCE.

**Considered and rejected:**
- **Pre-load wiring "just to be safe."** Looks fine on paper, silently breaks input at runtime. Don't.

---

## 2026-05-16 — Vendored zlib 1.2.11 patched for modern macOS (`fdopen=NULL` redef)

**Decision:** Apply a one-line patch to `vendor/deps/zlib-1.2.11/zutil.h` adding `!defined(__APPLE__)` to the inner guard around `#define fdopen(fd,mode) NULL`. Patch captured at `vendor/PATCHES/0001-zutil-skip-fdopen-NULL-on-modern-macOS.patch` — first entry in the patch series.

**Why:** Vendored zlib 1.2.11 (Jan 2017) carries a Mac OS Classic / CodeWarrior branch that redefines `fdopen` to `NULL` on `TARGET_OS_MAC` unless `__MWERKS__` is set. On modern macOS, Xcode's `<TargetConditionals.h>` auto-defines `TARGET_OS_MAC=1`, the redef fires, and when other zlib .c files include `<stdio.h>`, the system header's `FILE *fdopen(int, const char *) __DARWIN_ALIAS_STARTING(...)` gets macro-expanded into `FILE *NULL(int, const char *)` — clang parse error. First-pass CI on macOS arm64 / Xcode 16.4 broke on this. The patch preserves the original MWERKS branch (still valid for real Mac OS Classic builds) and short-circuits the redef on `__APPLE__`.

**Considered and rejected:**
- **Upgrade vendored zlib to 1.2.13+.** Bigger drift from Beetle PCE Fast's upstream vendoring, more risk for marginal benefit.
- **Link system zlib on non-MSVC.** Workable but introduces a platform conditional in `build.rs` and a system-deps split that's hard to maintain when libchdr depends on specific zlib header layout.
- **Pre-define `fdopen` to itself to short-circuit `#ifndef fdopen`.** Hack; hides the real fix from future readers.

---

## 2026-05-16 — Save state surface goes through libretro `retro_serialize` / `retro_unserialize`

**Decision:** `shim.cpp` exposes three save-state functions — `oa_pce_serialize_size` / `oa_pce_serialize` / `oa_pce_unserialize` — that wrap libretro's `retro_serialize_size` / `retro_serialize` / `retro_unserialize` directly. `PceCore::save_state` queries size, allocates a buffer, calls serialize, writes to the supplied `Write`. `load_state` reads to the supplied `Read`, then calls unserialize.

**Why:** Beetle PCE Fast's `libretro.cpp` already implements `retro_serialize/_unserialize` on top of Mednafen's `MDFNSS_*` state machinery. Going through libretro keeps the surface symmetric with every other libretro core we'll wrap later (same `oa_<sys>_serialize*` shape applies to Lynx, 7800, SMS, etc.). Bypassing libretro to call `MDFNSS_*` directly would duplicate work that the upstream layer already does, and we'd lose the per-core compatibility tracking Mednafen already publishes.

**Considered and rejected:**
- **Direct `MDFNSS_*` calls into Mednafen.** Same reasoning as the project-wide "shim wraps libretro frontend, not Mednafen" decision — works in theory, costs maintenance every time the upstream interface shifts.
- **Custom serialization format.** Defeats the point of having a battle-tested state machinery already shipping.

---

## 2026-05-15 — `RETRO_ENVIRONMENT_GET_GAME_INFO_EXT` is required for in-memory ROM loading

**Decision:** `shim.cpp`'s environment callback implements `RETRO_ENVIRONMENT_GET_GAME_INFO_EXT` and returns a populated `retro_game_info_ext` struct (data pointer + size + lowercase extension, e.g. `"pce"`). We do **not** rely on the `data` / `size` fields of the `retro_game_info` struct passed to `retro_load_game`.

**Why:** Beetle PCE Fast (and most modern libretro cores) bypass `info->data` when no file path is provided, going through `GET_GAME_INFO_EXT` instead to get extension + data + size in one shot. First Bonk load returned `retro_load_game` status 2 (rejection) because we only populated `info->data`; adding the env callback handler fixed it immediately. Flagged in `shim.cpp` comments for next-system bring-up — applies to any modern libretro core, not just PCE.

**Considered and rejected:**
- **Write the ROM to a temp file and pass `info->path`.** Works but adds disk I/O for every load, complicates lifetime management, and breaks the clean in-memory pipeline we want for streaming ROMs from compressed archives later.

---

## 2026-05-16 — libretro `GET_LOG_INTERFACE` is mandatory for CD-capable Mednafen cores

**Decision:** `crates/oa-libretro` ships a C variadic trampoline (`src/log_trampoline.c`, compiled via `cc` in `build.rs`) and `state.rs::cb_environment` returns a valid `retro_log_callback.log` pointer for `RETRO_ENVIRONMENT_GET_LOG_INTERFACE`. We do **not** return false there.

**Why:** Beetle PCE Fast (and very likely every other Mednafen-derived libretro core — Beetle PCE, Beetle SuperGrafx, Beetle PSX, Beetle Saturn, Beetle WonderSwan, Beetle NGP, Beetle VB, Beetle Lynx) initializes its internal `log_cb` to NULL if `GET_LOG_INTERFACE` returns false. Then the CD-init code path calls `log_cb(level, fmt, ...)` *without null-checking*, NULL function-pointer deref → STATUS_ACCESS_VIOLATION. The HuCard path doesn't hit this — explains why our shell worked for Bonk's Adventure for weeks while CDs crashed instantly. The trampoline lives in C because Rust on stable can't write a `void(level, fmt, ...)` variadic function (`c_variadic` is unstable); the C side `vsnprintf`s into a stack buffer and calls `oa_libretro_log_forward` in Rust which routes through the `log` crate so core output interleaves with our regular logging at the right level.

**Considered and rejected:**
- **Return false from `GET_LOG_INTERFACE` and let cores fall back to `fprintf(stderr)`.** What we shipped originally. Two failures: (1) Beetle PCE Fast doesn't fall back, it crashes; (2) even if a core *did* fall back, a Windows GUI-subsystem app has no attached console so stderr goes to /dev/null, defeating the diagnostic value of having any log output at all.
- **Use the unstable `c_variadic` feature.** Avoids the C file but pins the crate to a nightly Rust toolchain. Not worth it for ~20 lines of C.
- **Hand-roll printf parsing in Rust.** Reinvent libc's vsnprintf badly. No.

---

## 2026-05-18 — Beetle PCE Fast handles PCE-CD; full Mednafen PCE not vendored

**Decision:** The `mednafen_pce_fast_libretro.dll` we already ship for TG-16
HuCards also drives PCE-CD. Phase 0 Spike 2's hint ("Beetle PCE Fast ships
`pcecd.cpp`") was the right one — operator-validated 2026-05-18 with
Castlevania: Rondo of Blood (CHD) end-to-end. Full Beetle PCE Mednafen
(`mednafen_pce_libretro.dll`) is kept available as a per-game core override
fallback (PerSystemSettingsPage → Cores or the per-game override) but is not
the default and is not vendored / built by us today.

**Why:** Fast worked. No reason to ship a second .dll for the same set of
titles when the cheaper one runs them. Per-system + per-game core override
already exists for the rare title that regresses on Fast.

**Considered and rejected:**

- **Spike full Mednafen separately before committing.** Bypassed because
  Fast already passed validation — spiking what we don't need is
  premature.
- **Default to full Mednafen "to be safe."** Heavier load, larger memory
  footprint, no observable compatibility win for the titles tested.

**Resolves** the open question carried in `docs/cores/tg16/ROADMAP.md`
Phase 5 ("separate core or this one?") in favor of "same .dll, separate
SystemId in the frontend registry." The SystemId split decision lives in
`docs/cores/pce-cd/DECISIONS.md`.

---

## 2026-05-17 — libretro-database `metadat/` (plain-text DATs) is the v1 metadata source

**Decision:** Per-game metadata (year / genre / developer / publisher / players) is sourced from libretro-database's `metadat/<kind>/<system>.dat` plain-text files, fetched once per system + cached on disk for 24h, then matched locally via the same fuzzy `normalize::match_score` pipeline used for cover art. No per-launch API hits, no account flow, no rate limits. The `metadata.rs` module owns this entirely. v1 ships for TG-16 / TG-CD / SGX (the three systems we register today); same module handles any future system that has a libretro-database name mapped via `metadat_system_name_for_extension`.

**Why:**
- **Offline-after-sync posture.** Matches the libretro-thumbnails approach we already shipped: sync the system's data files once, then resolve every ROM locally. No "the API is down" failure mode at launch time, no rate-limit anxiety, no per-launch latency. Users on metered connections sync once and never round-trip again.
- **Dataset overlap.** Any game libretro-thumbnails covers is also in libretro-database — same source of truth, same curation, same "if it works for one, it works for both" guarantee. Adoption-rate symmetry across systems.
- **Parser cost is trivial.** The `metadat/` files are clrmamepro-style — a 25-line line-by-line state machine extracts `name`/`value` pairs from each `game (...)` block. Three unit tests cover the parser. No DAT XML, no RDB binary format, no third-party parser dep. If we later need fields not in `metadat/` (e.g. `description`, which libretro-database doesn't ship), we can add a second source for those specific fields without invalidating this layer.
- **No new dependencies.** Reuses `reqwest` + `serde_json` + `crate::normalize`, all of which the cover-sync path already brings in.

**Considered and rejected:**
- **ScreenScraper.** Best metadata quality (descriptions, scoring, multiple media types), but requires a user account + free-tier rate limits + per-launch online dependency. Adds an integration surface (registration flow, credential storage, throttle handling) we don't want to maintain for v1. Reconsider for v2 if libretro-database has critical gaps for systems we add.
- **TheGamesDB.** API-only, rate-limited, requires API key. Same "online dependency we don't want" issue. Mid-tier metadata, no clear win over libretro-database for our use case.
- **Parse the binary RDB format in `libretro-database/rdb/`.** Richer data than `metadat/` (carries everything in one file), but binary format means a custom parser + maintenance burden when upstream bumps the schema. The text-format `metadat/` files cover everything we ship in v1 (`GameMetadata { year, genre, developer, publisher, players, description }` minus description) at far lower implementation cost. RDB path stays open for a future "all fields, one fetch" upgrade.
- **Local-only metadata DB shipped with installer.** Would mean we're responsible for keeping it fresh across releases. The 24h fetch lets users get newer entries the day after libretro-database lands them, no installer redeploy needed.

**Scope note:** Description is not in `metadat/` (libretro-database ships descriptions only in RDB). The Game Info modal's description block is intentionally rendered behind a `Show when={metadata()?.description}` — empty by default until we add a description source. The data model field is already there (was shipped in the cover-art bundle), so wiring it later is a one-line `entry.description = Some(...)` in the parser path, no schema migration.
