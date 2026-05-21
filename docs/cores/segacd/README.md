# segacd — Sega CD / Mega-CD

Onboarded 2026-05-20 (paired with Sega 32X). Drives the Sega CD (NA) /
Mega-CD (JP / EU) via the libretro **Genesis Plus GX** core
(`genesis_plus_gx_libretro.dll`) by default — the same .dll already
shipping for `sms` + `gamegear`. One install lights up four Sega systems
(SMS, Game Gear, segacd, and genesis-via-override).

The Sega CD was Sega's 1991 (JP) / 1992 (US/EU) CD-ROM add-on for the
Mega Drive, packaging FMV, CDDA redbook audio, and 60-100 MB of game
data per disc. It was the era's most-overlooked premium console — only
~200 retail releases worldwide, but the lineup carries multiple Sega
flagship titles (Sonic CD, Snatcher, Lunar: The Silver Star, Popful
Mail, Final Fight CD) plus the early-FMV experiments (Night Trap,
Sewer Shark) that defined the platform's brand.

OA wires the Sega CD cart-shape parent (Mega Drive) and the CD-shape
addon as separate slugs — `genesis` for cart games, `segacd` for CD
images — same split TG-16 / PCE-CD navigated, for the same reasons
(distinct library shelves, distinct theme, independent per-system
settings).

## Upstream

- **Default core (this onboarding):** Genesis Plus GX — https://github.com/libretro/Genesis-Plus-GX
  - Long-standing libretro multi-Sega core (one .dll covers SMS / GG /
    MD / Sega CD).
  - Buildbot path: https://buildbot.libretro.com/nightly/windows/x86_64/latest/genesis_plus_gx_libretro.dll.zip
  - License: distributable non-commercial (per upstream LICENSE.txt;
    permissively licensed for OA's shell-as-library use).
- **Alternates (per-system Cores override):**
  - `picodrive_libretro.dll` — lighter core, also covers 32X + Sega CD.
- **Vendored:** No. Operator drops the buildbot .dll into
  `<exe_dir>/cores/`. If we ever need to fork for an OA-specific
  extension, we maintain our own libretro-frontend build per the
  project DECISIONS 2026-05-16 pivot.

## ROM format

Sega CD games are CD images — the standard libretro CD container shapes:

- **`.cue` + `.bin`** — canonical multi-track layout. Cue references
  the data + audio tracks alongside it. Sega CD discs typically have
  one data track + multiple CDDA tracks.
- **`.chd`** — single-file MAME-derived compressed CD container.
  Cleanest archival format; one file per disc.
- **`.iso`** — single-track data-only ISO. Loses CDDA tracks — only
  appropriate for non-CDDA Sega CD games (rare; most use CDDA).
- **`.m3u`** — multi-disc playlist. Lunar: Eternal Blue's two-disc
  release uses this through the libretro disc-control extension.
- **`.ccd` / `.toc`** — alternate CD container metadata formats
  CloneCD / cdrdao produce. Genesis Plus GX reads both.

`.bin` is intentionally NOT a top-level registered extension — it's a
CD track file referenced by `.cue`, not a stand-alone ROM.

**Extension collision with PCE-CD:** all six CD container extensions
are also claimed by PCE-CD. Disambiguation happens at Import Wizard
time via per-folder hint (the operator marks a folder as `segacd` and
matching extensions inside that folder route to Sega CD). Same path
PCE-CD navigated; documented in `DECISIONS.md`.

## BIOS

Sega CD playback **requires** a regional BIOS in `<exe_dir>/system/`
matching the disc's region. The shell pre-checks SHA-1 against
canonical Genesis Plus GX-blessed dumps (`SEGA_CD_BIOS_KNOWN_HASHES`
in `apps/oa-shell/src/main.rs`) and surfaces a toast naming the
expected filename if the BIOS is missing or its hash doesn't match.

| Filename         | SHA-1                                      | Description |
|------------------|--------------------------------------------|-------------|
| `bios_CD_U.bin`  | `F891E0EA651E2232AF0C5C4CB46A0CAE2EE8F356` | US Sega CD Model 1 v1.10 (1992) |
| `bios_CD_U.bin`  | `4846F448160059A7DA0215A5DF12CA160F26DD69` | US Sega CD Model 2 v2.00 (1993, most common) |
| `bios_CD_U.bin`  | `8AF162223BB12FD72D7D4C4F5DA6DF7012BD3F3E` | US Sega CD Model 2 v2.00w (1993 revised) |
| `bios_CD_J.bin`  | `BC99B1B27C39DB3E6E9B0FBED1ABEC0E08E2B6D2` | JP Mega-CD Model 1 v1.00p (1991 launch) |
| `bios_CD_J.bin`  | `4F3A48D6DBA2C83F2D2C30C9B75C9D77C44EE62C` | JP Mega-CD Model 1 v1.00s (1992 revision) |
| `bios_CD_E.bin`  | `F25B1CC6B71FE4DBBF17AFE2BB09BBCFB5E5B88E` | EU Mega-CD Model 1 v1.00 (PAL) |

Wrong-content BIOSes with the right filename typically cause Genesis
Plus GX to fail CD-init with an unrelated-looking access violation, so
the pre-check refuses early with a clear error toast rather than let
the core crash. BIOSes whose SHA-1 doesn't match a known canonical
entry still load with a warn-level toast — the launch proceeds, and
the operator can validate against their dump's documented hash.

## Native timing

- **NTSC:** 59.92 Hz, 320×224 visible (same as Mega Drive cart games).
- **PAL:** 49.70 Hz, 320×240 visible.
- Genesis Plus GX reports timing per-loaded-image via `retro_system_av_info`
  — the renderer takes whatever dimensions the core hands it. Most Sega
  CD games run at the same 320×224 H40 mode as their cart counterparts;
  CDDA-streamed FMV titles use the same framebuffer (the FMV is
  decoded into MD chunks rather than running at a separate resolution).

## Input

Identical to Genesis — the Sega CD uses the same 6-button Mega Drive
controller. `bindings::defaults_for("segacd")` shares the
`default_genesis_bindings()` path; `bit_for` / `buttons_for` /
`to_libretro_bits` all dispatch `genesis` + `segacd` + `sega32x` to the
same `GENESIS_BUTTONS` table and identity remap.

Per the cross-system "Z is primary" rule (locked by the
`z_is_the_primary_action_button_on_every_system` test):
- **Z** → MD **B** (middle face, libretro bit 0) — primary action.
- **X** → MD **C** (right face, libretro bit 8) — secondary.
- **A** → MD **A** (left face, libretro Y bit 1) — tertiary.
- Q/S/W → MD X/Y/Z (top row of 6-button face) — SNES-shoulder pattern.
- Enter → START, RShift → MODE.

Sega CD games' control schemes overwhelmingly assume 3-button compat;
6-button announce works because Genesis Plus GX defaults to it (same
behavior as cart Genesis). Per-game 3-button override available via
the per-game Input drawer if a specific title misbehaves.

## Current status (2026-05-20)

**Works:**
- Core resolves via `default_core_dll_for_system("segacd") →
  "genesis_plus_gx_libretro.dll"`.
- 10-button input mapped via the shared genesis dispatch arm (identity
  to libretro RetroPad).
- Library scanner classifies `.cue / .chd / .iso / .m3u / .ccd / .toc`
  as `segacd` once the operator marks the folder via Import Wizard
  (the same per-folder disambiguation PCE-CD uses for the same
  extension set).
- Theme accent: sapphire blue at hue 235° + L=0.55 + C=0.20, family-
  cousin to Genesis cobalt (245°) but visually distinct.
- BIOS pre-check via `check_sega_cd_bios` in main.rs — six canonical
  SHA-1s across US / JP / EU variants; missing BIOS surfaces a clean
  error toast naming the expected filenames.

**Not yet validated:**
- Real game launch — needs operator validation against a known-good
  ROM + BIOS combo. Suggested test discs: **Sonic CD** (US v2.00),
  **Lunar: The Silver Star Complete** (US v2.00), **Snatcher** (US v2.11
  or JP), **Popful Mail** (US v2.00). Pick one that matches a BIOS
  region the operator has on hand.
- Multi-disc title via `.m3u` — Lunar: Eternal Blue's two-disc release
  is the canonical test for the libretro disc-control extension. Disc
  swap UX work overlaps with PCE-CD's deferred multi-disc Phase 5.5
  validation.
- CDDA streaming — Sega CD games are CDDA-heavy. The audio sink path
  through the libretro audio callback is shared with PCE-CD (validated
  there 2026-05-18), but Sega CD's specific channel layout / mixing
  needs operator-confirmation against Sonic CD's iconic soundtrack.
- libretro-database hash matching against redump — wired to `[]` at
  onboarding (CD images aren't single-file SHA-1 matched). Disc-id
  extraction via `cd_id.rs` is Phase 2 polish per DECISIONS.
- Cover sync via libretro-thumbnails `Sega_-_Mega-CD_-_Sega_CD` —
  wired but needs operator validation pass.

## Per-core docs

- `ROADMAP.md` — phase tracking for Sega CD specifically.
- `SESSION_LOG.md` — Shipped / Almost / Next per session.
- `KNOWN_GAME_BUGS.md` — per-game compatibility issues as they surface.
- `DECISIONS.md` — Sega CD-specific integration choices.
