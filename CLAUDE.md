# Overlooked Arcade — Project Context

This file loads automatically every Claude Code session. **Read it first, every time.**

## What this project is

A premium emulator frontend for the consoles modern emulators forgot — TurboGrafx-16/PC Engine + CD-ROM², Atari Lynx, Atari 7800, SMS/Game Gear, MSX/MSX2, ColecoVision, Vectrex, Virtual Boy, WonderSwan. Each system gets a polished, dedicated home with per-system theming.

Built on **forked C cores** (Beetle PCE Fast, Mednafen, MAME modules) wrapped in Rust crates via FFI. We own the source and modify it freely — that's the whole point of forking instead of using vanilla libretro. The shell is a Tauri WebView + Solid UI; rendering is wgpu+WGSL hitting DX12/Vulkan/Metal/OpenGL from one pipeline.

**Non-commercial.** This is a gift to the retro community. License is GPLv2 binary-wide.

## Working model

The human is the **project owner** and operator at the keyboard. Claude (me) is the **developer** — I write the code, design the architecture, and shape the assets. The human runs builds, tests gameplay, validates audio, and gives creative direction.

## Locked design pillars (do not relitigate)

- **Stack:** Rust + Tauri 2 + wgpu (WGSL) + libretro cores loaded dynamically via `libloading` (the `oa-libretro` crate). See the 2026-05-16 "Architecture pivot: libretro frontend" entry in `docs/DECISIONS.md` for why.
- **UI:** Solid + TypeScript + Tailwind + Vite. Heroic Games Launcher visual ceiling. Per-system theming.
- **License:** Shell is GPL-2.0 in the workspace metadata today; the dynamic-loading pivot severs binary-wide GPL propagation, so this can move to a permissive license once the install ships with our own .dll builds. GPL cores stay GPL in their .dll. Repo public from Day 1.
- **Cores live next to the .exe** in `<exe_dir>/cores/` as libretro `.dll` / `.so` / `.dylib` files. Users can use community-built nightlies (https://buildbot.libretro.com/) or .dlls we build ourselves from forked source. BIOS files (PCE-CD `syscard3.pce` etc.) live in `<exe_dir>/system/`. User prefs (saves, bindings, audio.json) stay in `appDataDir` because they're per-user, not per-install.
- **Forked-core philosophy preserved via our own .dll builds.** When we want to modify a core, we maintain a separate libretro-frontend build of the patched source (e.g. our Beetle PCE Fast fork) that produces a .dll we ship in the installer's `cores/` folder. Vendored static crates (`oa-pce-sys`, `oa-pce`) are retired (kept in-tree as historical reference, excluded from the workspace build).
- **TG-16/PCE first.** HuCard cart → PCE-CD. Now translates to: "ship + validate Beetle PCE Fast .dll first" rather than vendoring source.
- **Two-window Phase 1, single-window shipped in Phase 2.** Both modes are now production; selectable via Settings → Display → Shell mode.
- **Playable, not cycle-accurate.** Top ~80% of each system's library running well. Cores already encode chip-level correctness — we don't second-guess upstream.
- **No per-core ARCHITECTURE.md.** Chip behavior lives in upstream documentation. Per-core docs we DO keep: README (upstream info + our patches summary), ROADMAP, SESSION_LOG, KNOWN_GAME_BUGS, DECISIONS (integration choices).
- **One core at a time.** `docs/ACTIVE_CORE.md` is the source of truth. Don't start Lynx work while TG-16 is mid-phase. Scope creep goes in `docs/PARKING_LOT.md`.

## How to start a session — context loading discipline

The docs tree is split into **active** (load on session start) and
**archived** (read on explicit need). Keeping a tight active surface is
deliberate — the goal is to always have what you need, never EVERYTHING.

### Always load at session start

1. `CLAUDE.md` (this file).
2. `docs/INDEX.md` — routing table.
3. `docs/ACTIVE_WORK.md` — **read the "In flight" section only**. The
   "Recently completed" section is intentionally a compressed log of
   1-liners; do not read past it preemptively.
4. For each in-flight stream listed in the "In flight" section:
   - Its `README.md` (per-feature or per-core).
   - The most recent entry of its `SESSION_LOG.md` — what last session
     shipped, what's next. Stop after the most recent entry; older
     history is reference-only.
5. **Summarize back to the human:** "Active streams are X (current
   task Y) and Z (current task W); confirming we're still on those?"
6. Wait for confirmation before doing work.

### Read on explicit need (do NOT preemptively load)

- **`docs/_archive/`** — all shipped feature folders + closed plans
  live here. Manifest at [docs/_archive/INDEX.md](docs/_archive/INDEX.md).
  Open ONLY when:
  - Investigating "why was X done that way?" — read the relevant
    closed plan or feature's DECISIONS.
  - Matching a past pattern — open the closest analogous feature's
    README or DECISIONS.
  - Reconstructing context after a regression appears in code from a
    shipped arc.
- **`docs/ACTIVE_WORK.md` "Recently completed" section** — only
  read past the line `## Recently completed` when a current task
  explicitly references a recently-shipped arc.
- **`docs/PARKING_LOT.md`** — read when a feature idea comes up that
  might already be parked.
- **`docs/DECISIONS.md`** — read when an architectural topic comes up.
- **`docs/NEXT.md` MEDIUM / LOWER / DEFERRED bands** — only when
  picking next work; HIGH band is the primary surface.
- **Per-core `KNOWN_GAME_BUGS.md` / `DECISIONS.md`** — only when
  working on that system.
- **Per-stream `DECISIONS.md`** — only when an architectural topic in
  that stream surfaces.

### Never load proactively

- Old SESSION_LOG archive files (rolled-over historical entries).
- Closed-plan files under `docs/_archive/PLANS/`.
- Closed-feature SESSION_LOGs under `docs/_archive/features/<name>/SESSION_LOG.md`.

Grep into these on demand only — the files are searchable, just not
worth burning context window on at session start.

## Switching streams

Edit `docs/ACTIVE_WORK.md` to update what's in flight. Each stream lives under either `docs/cores/<id>/` (per-core integration) or `docs/features/<name>/` (cross-cutting work like sidebar / ui-polish / library-import / kiosk-shell). When picking up a returned-to stream, its full history persists in place.

## How to end a session

1. Append to the SESSION_LOG of whichever stream(s) you worked on — either `docs/features/<name>/SESSION_LOG.md` or `docs/cores/<id>/SESSION_LOG.md`. Three lines: **Shipped / Almost / Next**. Cross-cutting work goes under features/, not under whichever core happened to be active.
2. If a design decision was made, append to the stream's `DECISIONS.md` (per-stream) or `docs/DECISIONS.md` (project-wide) with the date and the *why*.
3. If an out-of-scope idea came up that's worth keeping, append to `docs/PARKING_LOT.md`.
4. If a stream's work wrapped, update `docs/ACTIVE_WORK.md`.
5. Confirm with the human before closing.

## Architectural rules (apply once Rust code exists)

- Every core implements the `oa_core::Core` trait: `reset()`, `run_frame()`, `framebuffer()`, `drain_audio()`, `set_input()`, `save_state()`, `load_state()`. The shell is system-agnostic. The shipped impl is `oa_libretro::LibretroCore` (wraps a loaded libretro .dll).
- Hot paths run on dedicated threads. The emulator core runs on its own thread; renderer pulls the latest framebuffer; audio is event-driven via cpal callback. Don't block the UI thread.
- libretro is the **primary** launcher boundary; external standalone emulators reach the shell via the `Launcher` trait + `ExternalProcessLauncher` impl (per the 2026-06-03 DECISIONS reversal). `oa-libretro::ffi` declares the libretro ABI typedefs; `oa-libretro::loader` resolves symbols via `libloading`; `oa-libretro::state` owns the singleton callback state. New libretro cores arrive as `.dll` files; new external-emulator support arrives as `config/emulators/<id>.yaml` profiles consumed by the shared `Launcher` trait. See `docs/PLANS/virtual-library-and-launcher-arc.md` for the arc; the libretro path stays unchanged for systems that have a working libretro core.
- Shaders are WGSL only. wgpu translates to DX12/Vulkan/Metal/GL. Avoid features that don't translate cleanly to GL fallback unless behind a backend cap check.
- No network calls from emulator code. The emulator runs fully offline.
- Tests live next to the code they test. Test ROMs go under `assets/test-roms/` (gitignored).

## Out of scope right now

See `docs/PARKING_LOT.md`. If something comes up that isn't current-phase work for the active core, write it there and move on. Do not pursue it this session.

## Naming and file conventions

- Workspace prefix: `oa-` (Overlooked Arcade).
- One binary: `apps/oa-shell/` (Tauri app).
- Single Rust libretro frontend: `crates/oa-libretro/`. Per-system Rust crates are no longer added — new systems arrive as `.dll`s in `<exe_dir>/cores/`.
- Shared: `crates/oa-core/`, `oa-render/`, `oa-audio/`, `oa-input/`, `oa-platform/`, `oa-content/`, `oa-savestate/`, `oa-cdrom/`.
- Retired static-core crates (`crates/oa-pce-sys/`, `crates/oa-pce/`) stay on disk as historical reference but are excluded from the workspace build.
- Frontend is NOT a Cargo crate. Lives at `frontend/` with `package.json`, built by Vite.
- Shaders in `crates/oa-render/shaders/*.wgsl` (engine-level) and `shaders/presets/*.preset.toml` (shipped presets).

## ROADMAP hygiene — close items with their PR

The per-core `docs/cores/<id>/ROADMAP.md` files are the authoritative status surface for what's shipped vs open per system. Earlier audits found them drifting weeks behind reality because nobody had a clear "when do these get updated?" policy. The rule going forward:

**If a PR ships work that closes a ROADMAP bullet, the same PR flips that bullet from `⬜` to `✅` in the corresponding `docs/cores/<id>/ROADMAP.md` — in the same commit.**

Concrete checklist when wrapping up a code change:
1. Did this PR close any ⬜ items in `docs/cores/<active>/ROADMAP.md`? Flip them to ✅ and add a short citation: `✅ Feature (in apps/oa-shell/src/foo.rs::bar)`.
2. If the PR touched code shared across systems, check the other affected systems' ROADMAPs too. Cross-system features (the shader pipeline, save states, the libretro loader) commonly close items on multiple ROADMAPs at once.
3. If the work surfaced a NEW item that wasn't on the ROADMAP, add it as a fresh `⬜` bullet rather than carrying it in your head.
4. If the work is partial (e.g. "shipped detection but not dispatch"), keep the bullet `⬜` and append a status note: `⬜ Light gun support — detection in place (in cd_id.rs); dispatch pending operator validation`.

Two surfaces sit beside the per-core ROADMAPs:

- `docs/NEXT.md` — cross-system priority queue (HIGH / MEDIUM / LOWER / DEFERRED / DATA bands + a cross-system infrastructure inventory). Updated when items move between bands or land. Read this when picking up a fresh session without a specific assignment.
- `docs/cores/AUDIT_<date>.md` — one-shot cross-system sweep, only created when ROADMAPs drift far enough that a full re-derivation is needed. The 2026-05-21 audit was the first; its findings have been migrated back into the per-core ROADMAPs and the audit doc removed.

Per-core ROADMAPs are the source of truth for per-system status. If you find yourself updating `NEXT.md` or an audit doc instead of the per-core ROADMAP, stop and update the ROADMAP first.

## Debugging — where the logs live

A unified Rust + frontend log stream lands in three places at runtime (see `docs/DECISIONS.md` 2026-05-18 "Three-output logger" entry for the full design):

- **stderr** — visible when running via `cargo tauri dev`.
- **`<data_dir>/logs/oa-current.log`** — stable path, truncated each launch. Read this file when investigating bugs. `<data_dir>` resolves to one of two places per [docs/features/portable-install/](docs/features/portable-install/):
  - **AppData mode (default)**: `C:\Users\<user>\AppData\Roaming\dev.overlookedarcade.shell\logs\oa-current.log` on Windows.
  - **Portable mode** (when a `portable.txt` marker file sits next to `oa-shell.exe`): `<exe_dir>\settings\logs\oa-current.log`. The startup line `oa-shell: data dir = <path> (portable|appdata)` in the log itself confirms which.
- **`<data_dir>/logs/oa-<YYYYMMDD-HHmmss>.log`** — per-session archive, last 5 retained.
- **In-app**: `Help → Debug log…` opens a live filterable view of the in-memory ring (last 2000 entries). The **Copy path** button always returns the right path for the current mode.

When the human reports a bug:
1. Ask them to open `Help → Debug log…` and click **Copy path** — they can paste it back so I can `Read` the file directly.
2. The format is `ISO-8601 LEVEL [target] message`. `target` is the Rust module path (`oa_shell::media`) for Rust logs, or `frontend::<bracket-prefix>` (e.g. `frontend::oa-launch`) for frontend logs.
3. Frontend logs come from existing `console.log("[oa-launch] …")` call sites — the bracket prefix is parsed into the target by `frontend/src/lib/logbridge.ts`. New code can just keep using `console.*` and it lands in the same stream.

When the bug is "X stopped working in the new session" but the old session showed something useful, ask for one of the timestamped archives (same folder).

## Reference

The approved setup plan: `C:\Users\Devilchi\.claude\plans\jazzy-spinning-blum.md`. Read it for the full Cargo layout, directory tree, Tauri+wgpu integration approach, license discussion, build/dev workflow, phase plan, and risk list.
