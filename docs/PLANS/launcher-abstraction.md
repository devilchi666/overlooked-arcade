# Phase C — Launcher abstraction (sub-phase plan)

Parent arc: [virtual-library-and-launcher-arc.md](virtual-library-and-launcher-arc.md)
§6 Phase C. Branch family: `feat/virtual-library-phase-c*`.
Scoped 2026-06-07 with the operator after Phase E completed; the
theming arc's Phase-2 pause gates on this phase (theming plan §7).

## Goal in one line

Teach OA a second way to run a game: alongside loading a libretro
core `.dll` in-process, it can spawn a standalone external emulator
as its own process — same library, same tiles, same launch gesture.

## Operator-locked decisions (2026-06-07)

### D1 — Launcher is a lifecycle contract ABOVE Core, not a Core rewrite

`oa_core::Core` (frame-level: run_frame / framebuffer / drain_audio)
stays exactly as-is. The new `Launcher` trait covers lifecycle only:
`prepare → launch → is_alive → terminate` + capability flags.
`LibretroLauncher` wraps today's in-process pipeline bit-for-bit;
`ExternalProcessLauncher` spawns a binary and tracks the child PID.
The 46 working libretro systems never touch the new code path's
external half. Zero-regression design.

### D2 — Pilot: Dolphin standalone against the EXISTING `gamecube` system

No new systems in Phase C. The pilot trio from arc decision S8
(Cemu / RPCS3 / Lime3DS) implies onboarding wiiu / ps3 / 3ds —
three system registrations OA doesn't have. Those ride Phase D
alongside the installer that makes them meaningful. Phase C proves
the machinery where it's a pure platform change: the operator's
existing GameCube library, launchable via the libretro core OR a
Dolphin install they point OA at. Frontend looks identical either
way — that's the acceptance posture: **nothing visibly different
except the Settings field and which window opens.**

### D3 — External session UX: minimize OA while running

On external spawn, OA minimizes its window; on process exit, OA
restores. Session is still tracked: play time = process lifetime
(lands on the game row via the existing `update_play_session`
path), stdout/stderr captured into the OA debug log, terminate
affordance available from the restored window if the process hangs.

### D4 — Profiles are operator-editable YAML

`config/emulators/<id>.yaml`, mirroring the per-system descriptor
pattern. Fields per arc plan: id, display_name, vendor,
official_download_url, binary_name, supported_systems,
launch_args_template, capabilities. Phase C adds `binary_path`
(operator-set, points at an existing install — Phase D's installer
will fill it automatically). Reload on restart.

### D5 — Capability flags gate QuickSettings

`LauncherCapabilities { supports_rewind, supports_savestate,
supports_run_ahead, supports_input_remap, … }`. Libretro = all true
(today's behavior). External = all false in v1 (Dolphin manages its
own saves/input). QuickSettings toggles gray out with a "managed by
<emulator>" hint when the active launcher lacks the capability.

## Sub-phases

### C1 — `Launcher` trait + `LibretroLauncher` refactor

- Define `Launcher` trait + `LaunchPrepared` / `LaunchedSession` /
  `LauncherCapabilities` types (location: `oa-core` alongside the
  `Core` trait — it's the shell-facing contract).
- `LibretroLauncher` wraps the existing launch pipeline. Pure
  internal refactor — the launch command's behavior is preserved
  bit-for-bit; full test gate; operator playtest = "everything
  launches exactly like yesterday".
- No external launching yet.

### C2 — Profile registry + `ExternalProcessLauncher` + first Dolphin launch

- `apps/oa-shell/src/emulator_profiles.rs` + `config/emulators/
  dolphin.yaml` (supported_systems: [gamecube]).
- `ExternalProcessLauncher`: spawn via launch_args_template,
  capture stdout/stderr to the debug log, PID liveness, terminate
  with kill-after-timeout fallback.
- Settings surface: per-emulator "binary path" field + per-system
  "Default launcher" pref (libretro core vs named profile;
  unset = libretro, today's behavior).
- Minimize-on-spawn / restore-on-exit per D3.
- Exit: operator's GameCube game launches through their Dolphin
  install from the same tile that launched it via libretro
  yesterday.

### C3 — Capability gating + session polish

- QuickSettings toggles gate on the active launcher's capabilities.
- Play-time tracking for external sessions via process lifetime.
- Graceful-terminate affordance + hang handling.
- Per-system default-launcher pref surfaced in the Per-system
  SETTINGS drill-in section.

## Exit criteria (from the arc plan, adjusted per D2)

- Operator points the Dolphin profile at an installed binary and
  launches a GameCube game from its normal library tile.
- The same game launches identically via the libretro core when
  the per-system pref says so.
- QuickSettings toggles correctly disable for capabilities the
  external launcher doesn't support.
- oa-shell tests + frontend typecheck green throughout.
