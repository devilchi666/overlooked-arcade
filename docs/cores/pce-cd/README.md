# pce-cd — TurboGrafx-CD / PC Engine CD-ROM²

Split from `tg16` on 2026-05-18 (Phase 5 close). Cart-based PC Engine games stay
under `tg16`; CD images (`.cue` / `.chd` / `.ccd` / `.toc` / `.m3u` / `.iso`)
live here so they get their own sidebar entry, their own theme, and their own
per-system settings/bindings file.

## Core

Shipped with **Beetle PCE Fast** (`mednafen_pce_fast_libretro.dll`), the same
.dll the cart core uses. Operator-validated on 2026-05-18: Castlevania: Rondo
of Blood (CHD) boots from the existing CD load path, CDDA + in-game audio
play, gameplay starts. Full Beetle PCE Mednafen (`mednafen_pce_libretro.dll`)
remains a per-game fallback via PerSystemSettingsPage → Cores or the per-game
core override — drop it in `<exe_dir>/cores/` if a title needs it.

## BIOS

Drop a PCE-CD System Card into `<exe_dir>/system/`. `syscard3.pce` is the
recommended canonical pick — it supersedes v1/v2 and unlocks Super CD-ROM²
titles. The shell pre-checks the SHA-1 against the Mednafen-canonical hash
table (`PCE_BIOS_KNOWN_HASHES` in `apps/oa-shell/src/main.rs`); wrong content
under the right filename surfaces a toast naming the canonical hash, missing
BIOS surfaces a clean error toast rather than letting the core crash deep in
CD init.

| Filename       | SHA-1                                      | Description |
|----------------|--------------------------------------------|-------------|
| `syscard3.pce` | `1F8B161A2DB40DBA2079A87C10C0A3340B56ED3B` | US TurboGrafx-CD System Card v3.00 (preferred) |
| `syscard2.pce` | `056E3A8A7F3B7BE60EE6DEAEB0BAA67E1BA62B18` | US System Card v2.00 |
| `syscard1.pce` | `6DCA8A0AFD0CB1C14CFFC1CFFEA34915CD496E44` | US System Card v1.00 |
| `syscard3j.pce`| `A01CE5F5A90F9F3A2E76EC3D34D8B03B9BD9E62A` | JP Super CD-ROM² System Card v3.00 |
| `gexpress.pce` | `F8A06F08F8E7BF4D7117F1B22DA5074E0F49C2BC` | Games Express CD Card |

## Image formats

The shell routes any of `.cue` / `.chd` / `.ccd` / `.toc` / `.m3u` / `.iso`
through `oa_libretro::RomSource::Path` (vs. the bytes-in-memory path used for
HuCards) because multi-track CD sets need filesystem context for track
lookup. CHD is the cleanest single-file container; CUE+BIN works if both
files sit alongside each other. `.m3u` playlists work for multi-disc titles —
the core handles disc swap via the libretro disc-control extension.

## Controller

Identical to TG-16 — same PCE 6-button pad, same default bindings
(`default_pce_bindings()` in `apps/oa-shell/src/bindings.rs`), same
PCE→libretro bit remap (`pce_to_libretro_bits`). The `pce-cd` system shares
the entire input pipeline with `tg16`; the split is purely about library
grouping, theming, and per-system settings (e.g. a user can pick a different
default core or scaling mode for CD games without touching cart games).

## Per-core docs

- `ROADMAP.md` — phase tracking for pce-cd specifically.
- `SESSION_LOG.md` — Shipped / Almost / Next per session.
- `KNOWN_GAME_BUGS.md` — per-game compatibility issues as they surface.
- `DECISIONS.md` — pce-cd-specific integration choices.

Project-wide context (license, stack, libretro pivot) lives in
`docs/DECISIONS.md` and `CLAUDE.md`. TG-16 cart-specific context lives in
`docs/cores/tg16/`.
