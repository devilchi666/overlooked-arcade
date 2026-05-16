# Overlooked Arcade — Project Context

This file loads automatically every Claude Code session. **Read it first, every time.**

## What this project is

A premium emulator frontend for the consoles modern emulators forgot — TurboGrafx-16/PC Engine + CD-ROM², Atari Lynx, Atari 7800, SMS/Game Gear, MSX/MSX2, ColecoVision, Vectrex, Virtual Boy, WonderSwan. Each system gets a polished, dedicated home with per-system theming.

Built on **forked C cores** (Beetle PCE Fast, Mednafen, MAME modules) wrapped in Rust crates via FFI. We own the source and modify it freely — that's the whole point of forking instead of using vanilla libretro. The shell is a Tauri WebView + Solid UI; rendering is wgpu+WGSL hitting DX12/Vulkan/Metal/OpenGL from one pipeline.

**Non-commercial.** This is a gift to the retro community. License is GPLv2 binary-wide.

## Working model

The human is the **project owner** and operator at the keyboard. Claude (me) is the **developer** — I write the code, design the architecture, and shape the assets. The human runs builds, tests gameplay, validates audio, and gives creative direction.

## Locked design pillars (do not relitigate)

- **Stack:** Rust + Tauri 2 + wgpu (WGSL) + forked C cores via FFI.
- **UI:** Solid + TypeScript + Tailwind + Vite. Heroic Games Launcher visual ceiling. Per-system theming.
- **License:** GPLv2 binary-wide. Repo public from Day 1; binaries link to source.
- **Cores vendored in-tree.** `crates/oa-<sys>-sys/vendor/` with a maintained patch series (`vendor/PATCHES/`). Not git submodules.
- **TG-16/PCE first.** HuCard cart → PCE-CD. CPU/PSG/VDC familiarity established before Lynx, 7800.
- **Two-window Phase 1, single-window spike in Phase 2.** Library = Tauri WebView; game = dedicated native window with wgpu surface. Single-window (transparent WebView over wgpu) re-evaluated when UI shell is mature.
- **Playable, not cycle-accurate.** Top ~80% of each system's library running well. Forked cores already encode the chip-level correctness — we don't second-guess upstream.
- **No per-core ARCHITECTURE.md.** Chip behavior lives in the vendored C source's own comments and upstream docs. Per-core docs we DO keep: README (upstream info + our patches summary), ROADMAP, SESSION_LOG, KNOWN_GAME_BUGS, DECISIONS (our integration choices).
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

- Every core implements the `oa_core::Core` trait: `reset()`, `run_frame()`, `framebuffer()`, `audio_samples()`, `set_input()`, `save_state()`, `load_state()`. The shell is system-agnostic.
- Hot paths run on dedicated threads. The emulator core runs on its own thread; renderer pulls the latest framebuffer; audio is event-driven via cpal callback. Don't block the UI thread.
- FFI boundaries are thin and explicit. `oa-<sys>-sys` is the raw unsafe bindings; `oa-<sys>` is the safe wrapper that impls `Core`.
- Shaders are WGSL only. wgpu translates to DX12/Vulkan/Metal/GL. Avoid features that don't translate cleanly to GL fallback unless behind a backend cap check.
- No network calls from emulator code. The emulator runs fully offline.
- Tests live next to the code they test. Test ROMs go under `assets/test-roms/` (gitignored).

## Out of scope right now

See `docs/PARKING_LOT.md`. If something comes up that isn't current-phase work for the active core, write it there and move on. Do not pursue it this session.

## Naming and file conventions

- Workspace prefix: `oa-` (Overlooked Arcade).
- One binary: `apps/oa-shell/` (Tauri app).
- Per-core: `crates/oa-<sys>-sys/` (raw FFI) + `crates/oa-<sys>/` (idiomatic wrapper). Examples: `oa-pce-sys`/`oa-pce`, `oa-lynx-sys`/`oa-lynx`.
- Shared: `crates/oa-core/`, `oa-render/`, `oa-audio/`, `oa-input/`, `oa-platform/`, `oa-content/`, `oa-savestate/`, `oa-cdrom/`.
- Frontend is NOT a Cargo crate. Lives at `frontend/` with `package.json`, built by Vite.
- Shaders in `crates/oa-render/shaders/*.wgsl` (engine-level) and `shaders/presets/*.preset.toml` (shipped presets).

## Reference

The approved setup plan: `C:\Users\Devilchi\.claude\plans\jazzy-spinning-blum.md`. Read it for the full Cargo layout, directory tree, Tauri+wgpu integration approach, license discussion, build/dev workflow, phase plan, and risk list.
