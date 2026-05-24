# DOSBox

[DOSBox](https://www.dosbox.com/) — the canonical x86 / DOS emulator (since 2002), wrapped for libretro as `dosbox_pure_libretro.dll`. Not a hardware platform; OA ships it as an ordinary system alongside consoles so DOS games (Doom, Wolfenstein 3D, Commander Keen, X-COM, SimCity, Civilization) live in the same sidebar / library / per-game settings model as everything else.

## Default core

`dosbox_pure_libretro.dll` — the libretro port of DOSBox with auto-config baked in (handles cycles, sound card detection, expanded memory, mount layout without operator config). The only mature libretro DOS option. Active development. Includes its own DOSBox runtime — no external DOSBox install required.

## BIOS

None. DOSBox-pure ships its own runtime + the upstream DOSBox `system/` directory layout. OA creates `<exe_dir>/system/dosbox/` on first launch (`system_dir_for` in `apps/oa-shell/src/main.rs`) so the core's config cache, save states, and screenshots have a stable per-core home.

## Storage shape

A "game" is a directory containing the game's executable + data files. Example layout:

```
<library>/DOS Games/
  Doom/
    DOOM.EXE
    DOOM.WAD
    DOOM2.WAD
    ...
  Wing Commander/
    WC.EXE
    *.DAT
    DOSBOX.CONF              ← optional per-game DOSBox conf overrides
  X-COM UFO Defense/
    ufo.exe
    ...
```

OA scans `<library>/DOS Games/` at exactly 1 level deep — each direct subdirectory is one game; nested directories inside are content (game data files), not nested games.

### Per-game entry-point override

dosbox-pure auto-detects the boot path by inspecting the directory contents and applying its heuristics. ~10% of titles need an explicit override — typically install utilities (`INSTALL.EXE`) or DOS shells sitting next to the real game binary. Set `GameOverrides.dosbox_entry_point` to the path relative to the game directory (e.g. `"DOOM.EXE"`, `"DOSBOX/AUTOEXEC.BAT"`); the launch path passes `<game_dir>/<entry_point>` to `retro_load_game` instead of the bare directory, and dosbox-pure treats the explicit path as the boot target.

### Per-game `dosbox.conf`

Operators with existing `dosbox.conf` files (typical of Pure / Daum / Staging migrations) can drop them into the game directory unchanged. dosbox-pure reads `dosbox.conf` from the game directory automatically and applies its `[autoexec]` + sound / cpu / memory tuning. No OA wiring needed.

## Controller

DOSBox-pure runs a wide range of DOS games whose input shapes vary enormously:

- **Action / arcade** (Doom, Wolfenstein 3D, Commander Keen, Jazz Jackrabbit): 12-button RetroPad — d-pad + A/B/X/Y face diamond + L/R shoulders + START + SELECT. **Z = A (PRIMARY — jump / use)**, **X = B (secondary — shoot / attack)** per the cross-system Z=primary rule.
- **Mouse-driven sims** (X-COM, SimCity, Civilization, Master of Magic): flow through OA's shared POINTER infrastructure (mouse-as-pointer). The RetroPad bindings are inert for these titles.
- **Keyboard-heavy** (everything with typed commands, the Sierra adventures): flow through libretro KEYBOARD via OA's keyboard passthrough (enabled by default for dosbox per `system_settings::default_keyboard_passthrough`).

Operators rebind freely via the per-system Bindings UI for non-action DOS games.

## Status

- Phase 0 onboarding: ✅ 2026-05-24 (this session)
- Phase 1 operator validation: ⬜ — drop `dosbox_pure_libretro.dll` into `<exe_dir>/cores/`, drop a game directory (e.g. `Doom/`) into a library folder, mark that library folder with a `dosbox` rule, scan, launch.

## See also

- `docs/cores/dosbox/ROADMAP.md` — phase tracking
- `docs/cores/dosbox/SESSION_LOG.md` — what last session shipped
- libretro-thumbnails: `DOS`
- LaunchBox platform names: `MS-DOS` (modern), `DOS` (legacy)
- Plan: [features/dosbox-and-scummvm/README.md](../../features/dosbox-and-scummvm/README.md)
