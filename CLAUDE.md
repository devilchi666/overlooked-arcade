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

## How to start a session

1. Read `CLAUDE.md` (this file).
2. Read `docs/ACTIVE_CORE.md` — one word: which core is active.
3. Read `docs/cores/<active>/README.md` — current status, upstream version vendored, what works/doesn't.
4. Read `docs/cores/<active>/ROADMAP.md` — phase tracking for the active core.
5. Read the most recent entry in `docs/cores/<active>/SESSION_LOG.md` — what last session shipped, what's next.
6. Read `docs/cores/<active>/KNOWN_GAME_BUGS.md` if working on per-game compatibility.
7. Read `docs/cores/<active>/DECISIONS.md` if a per-core architectural topic comes up.
8. Read project-wide `docs/DECISIONS.md` for project-wide topics.
9. **Summarize back to the human:** "Last session shipped X, next task is Y, confirming we're still on that?"
10. Wait for confirmation before doing work.

## Switching cores

Edit `docs/ACTIVE_CORE.md` to the target core's id (e.g. `lynx`, `atari7800`). All per-core docs persist under `docs/cores/<previous>/` for when you return.

## How to end a session

1. Append to `docs/cores/<active>/SESSION_LOG.md` — three lines: **Shipped / Almost / Next**.
2. If a design decision was made, append to `docs/cores/<active>/DECISIONS.md` (per-core) or `docs/DECISIONS.md` (project-wide) with the date and the *why*.
3. If an out-of-scope idea came up that's worth keeping, append to `docs/PARKING_LOT.md`.
4. Confirm with the human before closing.

## Architectural rules (apply once Rust code exists)

- Every core implements the `oa_core::Core` trait: `reset()`, `run_frame()`, `framebuffer()`, `drain_audio()`, `set_input()`, `save_state()`, `load_state()`. The shell is system-agnostic. The shipped impl is `oa_libretro::LibretroCore` (wraps a loaded libretro .dll).
- Hot paths run on dedicated threads. The emulator core runs on its own thread; renderer pulls the latest framebuffer; audio is event-driven via cpal callback. Don't block the UI thread.
- libretro is the only FFI boundary. `oa-libretro::ffi` declares the ABI typedefs; `oa-libretro::loader` resolves symbols via `libloading`; `oa-libretro::state` owns the singleton callback state. New cores arrive as `.dll` files, not as new Rust crates.
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
- **`appData/logs/oa-current.log`** — stable path, truncated each launch. Read this file when investigating bugs. On Windows: `C:\Users\<user>\AppData\Roaming\com.oa.overlooked-arcade\logs\oa-current.log`.
- **`appData/logs/oa-<YYYYMMDD-HHmmss>.log`** — per-session archive, last 5 retained.
- **In-app**: `Help → Debug log…` opens a live filterable view of the in-memory ring (last 2000 entries).

When the human reports a bug:
1. Ask them to open `Help → Debug log…` and click **Copy path** — they can paste it back so I can `Read` the file directly.
2. The format is `ISO-8601 LEVEL [target] message`. `target` is the Rust module path (`oa_shell::media`) for Rust logs, or `frontend::<bracket-prefix>` (e.g. `frontend::oa-launch`) for frontend logs.
3. Frontend logs come from existing `console.log("[oa-launch] …")` call sites — the bracket prefix is parsed into the target by `frontend/src/lib/logbridge.ts`. New code can just keep using `console.*` and it lands in the same stream.

When the bug is "X stopped working in the new session" but the old session showed something useful, ask for one of the timestamped archives (same folder).

## Reference

The approved setup plan: `C:\Users\Devilchi\.claude\plans\jazzy-spinning-blum.md`. Read it for the full Cargo layout, directory tree, Tauri+wgpu integration approach, license discussion, build/dev workflow, phase plan, and risk list.
