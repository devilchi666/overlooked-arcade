# tg16 Decisions Log

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

## 2026-05-15 — `RETRO_ENVIRONMENT_GET_GAME_INFO_EXT` is required for in-memory ROM loading

**Decision:** `shim.cpp`'s environment callback implements `RETRO_ENVIRONMENT_GET_GAME_INFO_EXT` and returns a populated `retro_game_info_ext` struct (data pointer + size + lowercase extension, e.g. `"pce"`). We do **not** rely on the `data` / `size` fields of the `retro_game_info` struct passed to `retro_load_game`.

**Why:** Beetle PCE Fast (and most modern libretro cores) bypass `info->data` when no file path is provided, going through `GET_GAME_INFO_EXT` instead to get extension + data + size in one shot. First Bonk load returned `retro_load_game` status 2 (rejection) because we only populated `info->data`; adding the env callback handler fixed it immediately. Flagged in `shim.cpp` comments for next-system bring-up — applies to any modern libretro core, not just PCE.

**Considered and rejected:**
- **Write the ROM to a temp file and pass `info->path`.** Works but adds disk I/O for every load, complicates lifetime management, and breaks the clean in-memory pipeline we want for streaming ROMs from compressed archives later.
