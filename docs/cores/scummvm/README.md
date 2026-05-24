# ScummVM

[ScummVM](https://www.scummvm.org/) — community-maintained adventure-game engine launcher (since 2001) covering 200+ titles across SCUMM, AGI, SCI, Lure, MADE, and dozens of other point-and-click engines. Not a hardware platform; OA ships it as an ordinary system alongside consoles so adventure games live in the same sidebar / library / per-game settings model as everything else.

## Default core

`scummvm_libretro.dll` — the libretro port of the standalone ScummVM engine. The only libretro option for the engine. Mature, active development, broad compat. Includes most upstream ScummVM engine plugins built-in; some titles need extra files (engine helper data, fan-translation patches) which go under `<exe_dir>/system/scummvm/extra/`.

## BIOS

None. ScummVM has no BIOS in the platform sense — it ships its own engine plugins inside the .dll, with helper data living in `<exe_dir>/system/scummvm/`. OA creates that directory on first launch (`system_dir_for` in `apps/oa-shell/src/main.rs`) so the engine's themes/, extra/, and saves/ subdirectories have a stable per-core home.

## Extensions

`.scummvm` — a tiny single-line text descriptor file (e.g. `monkey:scumm`) that names the game ID and engine for ScummVM to load. OA scans recursively for these files at any depth under the library folder. The actual game data files (`MONKEY.000`, `MONKEY.001`, …) live next to the descriptor; ScummVM opens them via the descriptor's path automatically.

**Operator workflow:** drop a folder containing the game data (e.g. `Monkey Island/`) into the library, then create a sibling `Monkey Island.scummvm` text file with the engine-ID line. LaunchBox's ScummVM importer creates these automatically; operators doing it manually can crib the ID list from the [ScummVM compatibility table](https://www.scummvm.org/compatibility/).

## Controller

ScummVM is mouse-primary: the cursor flows through OA's shared POINTER infrastructure (mouse-as-pointer for desktop, gamepad right-stick for handheld kiosks). The per-system Bindings page exposes 8 fallback bits for users without a mouse:

- 4-direction D-pad — cursor movement when no mouse is attached
- LMB (libretro B — primary click; Z key)
- RMB (libretro A — secondary / right-click context menu; X key)
- ESCAPE (libretro SELECT — in-engine "back" / cancel; Esc key)
- PAUSE (libretro START — ScummVM main menu / save+restore; Enter key)

Keyboard passthrough is enabled by default (`system_settings::default_keyboard_passthrough("scummvm") = true`) because text input drives a meaningful slice of the canonical library — the Monkey Island sword-fighting insults, password prompts in Indiana Jones / Loom / Zak McKracken, typed verb input in select SCI titles.

## Status

- Phase 0 onboarding: ✅ 2026-05-24 (this session)
- Phase 1 operator validation: ⬜ — drop `scummvm_libretro.dll` into `<exe_dir>/cores/`, drop a `<game>/` directory + sibling `<game>.scummvm` file into a library folder, scan, launch.

## See also

- `docs/cores/scummvm/ROADMAP.md` — phase tracking
- `docs/cores/scummvm/SESSION_LOG.md` — what last session shipped
- libretro-thumbnails: `ScummVM`
- LaunchBox platform name: `ScummVM` (mapped via `art_pack_importer::launchbox_platform_to_system_id`)
- Plan: [features/dosbox-and-scummvm/README.md](../../features/dosbox-and-scummvm/README.md)
