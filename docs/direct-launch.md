# Direct-launch CLI mode

Overlooked Arcade can be invoked with a ROM path, in which case the library UI
is skipped and the shell boots straight into the game. This lets external
frontends (**LaunchBox**, **BigBox**, **EmulationStation**, etc.) treat
`oa-shell.exe` as a standalone emulator, the way they treat RetroArch.

Default zero-argument behavior is unchanged — running `oa-shell.exe` with no
arguments still opens the library.

## Usage

```
oa-shell.exe "C:\ROMs\Mega Man 2.nes"            # positional ROM
oa-shell.exe --rom "C:\ROMs\Mega Man 2.nes"      # equivalent
oa-shell.exe --system snes --rom "Mario.sfc"     # explicit system
oa-shell.exe "ActRaiser 2 (USA).zip"             # single-ROM .zip auto-extracts
oa-shell.exe "GoodSNES.zip#Super Mario World.sfc"  # multi-ROM .zip via explicit inner
oa-shell.exe --system psx "PSX.zip#FF7 Disc 1.cue" # CD-in-archive (extracted to temp)
oa-shell.exe "FF7 (Disc 1).cue" --system psx     # ambiguous ext needs --system
oa-shell.exe --rom Bonk.pce --slot 3 --fullscreen
oa-shell.exe --state-file "C:\saves\snap.state" --rom "ActRaiser 2 (USA).zip"
oa-shell.exe --core fceumm_libretro.dll --rom Mega Man 2.nes
```

Run `oa-shell.exe --help` for the full list of flags.

## What happens

1. **CLI parse** — flags are validated up front; errors print a banner and
   exit with status 2.
2. **System inference** — if `--system` is not supplied, the file extension
   determines the system. Cart extensions are unambiguous (`.nes`→`nes`,
   `.pce`→`tg16`, `.sfc`→`snes`, `.a78`→`atari7800`, …). CD-shaped extensions
   (`.cue` / `.chd` / `.iso` / `.m3u` / `.pbp`) are **ambiguous** and require
   `--system <slug>`.
3. **Archive auto-extract** — `.zip` / `.7z` archives are peeked inside.
   Exactly one ROM (cart OR CD entry-point) inside → it's used transparently
   (system inferred from the inner cart extension if `--system` wasn't
   supplied; CD inners require `--system`). MAME / Neo Geo passes the .zip
   through as-is when `--system mame` (or `neogeo`) is supplied, or when the
   `.p1` + `.s1` Neo Geo signature is detected. Empty / multi-ROM archives
   error out with a list.
4. **Explicit `<path>#<inner>` syntax** — power-user form for multi-game
   archives: `"GoodSNES.zip#Super Mario World (USA).sfc"` skips the
   single-ROM requirement and addresses one ROM out of many. Cart inners
   auto-infer the system; CD inners require `--system`. Inner path is
   validated against the archive's table of contents — typos error with the
   available inner list.
3. **Forced single-window** — direct-launch overrides any `OA_SHELL_MODE` /
   `shell.json` preference to single-window for the duration of this launch.
   Your operator preference on disk is **not** changed.
4. **SHA-1 hash lookup** — for cart-shaped ROMs only, the file's SHA-1 is
   matched against the library DB. On hit, the matched row's per-game
   overrides (patches, custom core options, shader, rewind config, analog
   routing, bezel, etc.) apply through the standard launch cascade. CD images
   skip this step (multi-GB hashes are expensive at boot).
5. **Chrome hidden** — the menubar, toolbar, sidebars, library grid, settings
   dialogs, and import wizard never render. In-game overlays remain available:
   **Quick Settings (Esc)**, save-state hotkeys (F5/F8), rewind, TAS, video
   capture, memory inspector, screenshot, performance HUD, toasts.
6. **Exit** — closing the window exits the process. From Quick Settings the
   "Quit" action unloads the ROM, finalizes save-state / temp dirs, then ends
   the process. There is no library to return to.

## Flags

| Flag | Description |
| --- | --- |
| *(positional)* | ROM path. LaunchBox / EmulationStation compat. |
| `--rom PATH` | ROM path (alternative to positional). Mutually exclusive with positional. |
| `--core DLL` | Override the libretro core .dll filename (resolved against `<exe_dir>/cores/`). |
| `--system SLUG` | Force the system. Required for ambiguous extensions. |
| `--slot N` | Restore from per-game save slot N (0–9) after the ROM loads. Mutually exclusive with `--state-file`. |
| `--state-file PATH` | Restore an arbitrary save-state file (absolute path) after the ROM loads. Mutually exclusive with `--slot`. |
| `--tas-replay PATH` | Play back a `.oatas` TAS recording at launch. |
| `--fullscreen` | Switch to fullscreen window mode after launch. |

## Errors

Errors print a multi-line banner to stderr and exit with status **2**:

- **ROM file not found** — `--rom` / positional path doesn't exist.
- **`--state-file` not found** — the state-file path doesn't exist.
- **Unknown ROM extension** — the file extension isn't in the mapping table.
- **Ambiguous ROM extension** — CD-shaped extension without `--system`; the
  message lists the candidate systems.
- **Archive contains no ROMs** — `.zip` / `.7z` peek found zero ROM-shaped
  entries; suggests `--system mame|neogeo` for romsets.
- **Archive contains multiple ROMs** — peek found more than one; lists them and
  suggests the explicit `<path>#<inner>` syntax or scanning via Import Wizard.
- **Inner ROM not found in archive** — explicit `#inner` doesn't exist in the
  archive's table of contents; lists available inners.
- **Archive read failed** — `.zip` / `.7z` is corrupt or unsupported.
- **Unknown system slug** — `--system` value isn't recognized; the message
  lists common slugs.
- **Conflicting arguments** — positional path and `--rom` both supplied, OR
  `--slot` and `--state-file` both supplied.

## Environment variables

The pre-existing `OA_ROM` env-var is still honored as a silent fallback when no
CLI args are supplied (preserving the dev loop). When both `OA_ROM` and CLI
args are set, CLI args win and a log line records the override.

`OA_LIBRETRO_CORE` continues to act as a developer bootstrap-core override
independent of the per-launch `--core` flag.

`OA_SHELL_MODE` is honored in library mode and **runtime-overridden to
single-window in direct-launch** (no disk write).

## Compatibility with launchers

- **LaunchBox / BigBox** — set the emulator command to:
  ```
  oa-shell.exe "%ROM%"
  ```
  Positional argument; OA auto-infers the system for cart-shaped extensions.
  Multi-system archives (`.cue`/`.chd`/.iso`) require `--system` per-platform
  emulator entry in LaunchBox.
- **EmulationStation** — `oa-shell.exe %ROM%` (positional only).
- **RetroArch-style short flags** (`-L`, `-c`, `--load-state`) are not
  recognized — OA only ships its own surface. Configure your launcher to use
  the long-form flags above.

## Limitations (v1)

These are explicitly out of scope for this version. Track in
`docs/PARKING_LOT.md` for future work:

- **Multi-instance** — running two `oa-shell.exe` direct-launches in parallel
  isn't supported (log-file locking, single-singleton libretro state).
- **ARGV batch processing** — one ROM per process invocation.
- **Persistent kiosk profiles** — OA always reverts to library mode after exit.
- **Steam Big Picture controller-launch** — separate problem.
- **Launcher-parity flags** — `--monitor N`, `--no-fullscreen`, `--scaling MODE`,
  `--shader NAME` aren't wired; tune via per-system / per-game library settings.
- **Kiosk / arcade flags** — `--kiosk`, `--auto-restart`, `--idle-timeout`,
  `--no-overlays` for museum installs / dedicated cabinets aren't shipped.
- **Diagnostics flags** — `--verbose`, `--probe` / `--dry-run`, `--list-systems`
  would help debug launcher configs but aren't shipped.

## How it works internally

- `apps/oa-shell/src/cli.rs` — clap derive struct, `DirectLaunchConfig`,
  `infer_system_from_extension`, error banners.
- `apps/oa-shell/src/main.rs::main()` — CLI parsing runs before
  `tauri::Builder::default()`; the resolved `DirectLaunchConfig` lives on
  `AppState`.
- `apps/oa-shell/src/main.rs` setup closure — forces single-window mode and
  runs the SHA-1 hash lookup against `library_db::find_game_by_sha1` for
  cart-shaped ROMs.
- `get_direct_launch_config` Tauri command — frontend reads this on mount.
- `frontend/src/App.tsx` — `directLaunchConfig` resource gates
  `isDirectLaunch`, which hides chrome via `Shell.fullBleed` + JSX `<Show>`
  guards, and drives the auto-launch effect that re-uses the existing
  `handleLaunch` cascade.
- `oa://rom-unloaded` event — emu thread emits after `UnloadRom` drain; the
  frontend listener calls `quit_app` when in direct-launch.
