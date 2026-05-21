# pcfx — NEC PC-FX

Onboarded 2026-05-20 (paired with jaguar + 3do). Drives the NEC PC-FX
via the libretro **Beetle PC-FX** core (`mednafen_pcfx_libretro.dll`).
Mednafen lineage shared with Beetle PCE Fast (pce-cd), Beetle Saturn,
Beetle PSX, Beetle VB, Beetle WonderSwan, Beetle Lynx, Beetle NeoPop.

The PC-FX was NEC's 1994-1998 32-bit CD-ROM successor to the PC Engine
— a Japan-only platform that bet the farm on anime FMV + visual
novel + dating sim content rather than the 3D polygon arms race that
defined the era. ~62 retail releases over the platform's 4-year run.
Standout titles: Battle Heat, Tyoushin Heiki Zeroigar, Team Innocent,
Chip-Chan Kick!, Last Imperial Prince, Pia Carrot Wonder, Power Dolls
FX.

## Upstream

- **Default core:** Beetle PC-FX — https://github.com/libretro/beetle-pcfx-libretro
- **Vendored:** No.

## ROM format

Standard libretro CD container set (`.cue` + `.bin` / `.chd` / `.iso` /
`.m3u` / `.ccd` / `.toc`); per-folder Import Wizard disambiguation.

## BIOS

Required `pcfx.rom` in `<exe_dir>/system/`. Single canonical BIOS —
PC-FX was Japan-only with no regional variants. Pre-checked by
`check_pcfx_bios`; slots into the CD-launch BIOS dispatch arm.

## Input

12-button PC Engine 6-button pad — d-pad + I/II/III/IV/V/VI + RUN +
SELECT. Same hardware layout as the post-1993 PCE 6-button controller.
PCFX gets its own bindings module (separate from `pce::*` which is
2-button only for TG-16 / PCE-CD).

Keyboard defaults: Z=I primary, X=II secondary, A=III, S=IV, Q=V, W=VI;
Enter=RUN, RShift=SELECT.

## Current status (2026-05-20)

Phase 0 onboarded. Awaits operator validation.

**Test discs:** Battle Heat (most playable PC-FX action game),
Tyoushin Heiki Zeroigar, Team Innocent.

## Per-core docs

- `ROADMAP.md`, `SESSION_LOG.md`, `KNOWN_GAME_BUGS.md`, `DECISIONS.md`.
