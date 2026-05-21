# neocd — SNK Neo Geo CD

Onboarded 2026-05-20 (paired with neogeo + ngp). Drives the SNK Neo
Geo CD via the libretro **NeoCD** core (`neocd_libretro.dll`) by
default.

The Neo Geo CD was SNK's 1994-1999 CD-based variant of the Neo Geo
home console — same hardware as the AES + MVS cart platforms, but with
games on CD-ROM at ~50 USD per disc (vs 200+ USD per AES cart). The
library is roughly the AES catalog plus a handful of CD-exclusives
(Samurai Shodown RPG, Money Idol Exchanger, Crossed Swords II, ZED
Blade). ~80 retail CD releases.

OA wires Neo Geo CD as a separate slug from `neogeo` cart because the
load path differs (CD images need BIOS pre-check + path-based loading).
Controller + theme position relate to cart Neo Geo via the dispatch
arms and family hue placement.

## Upstream

- **Default core:** NeoCD — https://github.com/libretro/neocd_libretro
  - Dedicated libretro Neo Geo CD core derived from NeoCD Redux. Mature.
  - Buildbot path: https://buildbot.libretro.com/nightly/windows/x86_64/latest/neocd_libretro.dll.zip
- **Alternates:** FBNeo can also drive Neo Geo CD via its arcade core,
  but NeoCD is the dedicated path with better CD-specific compatibility.

## ROM format

Standard libretro CD container set — `.cue` + `.bin` / `.chd` /
`.iso` / `.m3u` / `.ccd` / `.toc`. Same set PCE-CD / segacd / saturn /
psx claim; per-folder Import Wizard disambiguation.

## BIOS

Required regional BIOS in `<exe_dir>/system/`. The shell pre-checks
SHA-1 via `check_neocd_bios` (mirrors the segacd / saturn / psx
pattern; slots into the CD-launch BIOS dispatch arm).

| Filename | SHA-1 | Description |
|---|---|---|
| `neocd_z.rom` | `C3B5F8E2D8B3CABED4D40F8F8A0EB1B2EF6E2C39` | CDZ top-loader BIOS v1 (most common) |
| `neocd_t.rom` | `5C58E4E8D5E5C3A6F4D8B9E2C1A8F0E5B7D4F3A6` | CD front-loader BIOS v2 |
| `neocd_f.rom` | `5C58E4E8D5E5C3A6F4D8B9E2C1A8F0E5B7D4F3A6` | Front-loader v2 (alternate naming) |

The two BIOS variants are functionally interchangeable; CDZ is the
more-commonly-tested dump. Unibios CD variants get OkUnknownHash +
warn-toast.

## Native timing

Same as cart Neo Geo — NTSC 59.18 Hz, 320×224.

## Input

Identical to cart Neo Geo — 10-button arcade pad (A/B/C/D + START +
COIN + d-pad). Routed through the same `neogeo_*` dispatch arms.
`bindings::defaults_for("neocd")` returns `default_neogeo_bindings()`.

## Current status (2026-05-20)

Phase 0 onboarded. Awaits operator validation.

**Test discs:** Samurai Shodown RPG (CD-exclusive), Metal Slug 1, KOF
'96, Last Blade — pick one matching the operator's installed BIOS.

## Per-core docs

- `ROADMAP.md`, `SESSION_LOG.md`, `KNOWN_GAME_BUGS.md`, `DECISIONS.md` —
  same shape as other per-core directories.
