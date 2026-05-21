# neogeo — SNK Neo Geo (AES + MVS)

Onboarded 2026-05-20 (paired with neocd + ngp). Drives the SNK Neo Geo
home (AES) + arcade (MVS) variants via the libretro **FBNeo** core
(`fbneo_libretro.dll`) by default.

The Neo Geo was SNK's 1990 24-bit cartridge platform — the same
hardware shipped in arcades (MVS, cabinets accepting multiple game
carts per board) and in homes (AES, premium domestic console with the
same carts at retail prices that became legendary in the era — 200+
USD per cart in 1990 was unheard of). The library is overwhelmingly
SNK's own first-party output: King of Fighters series, Samurai
Shodown, Fatal Fury, Art of Fighting, Metal Slug, Garou: Mark of the
Wolves, Last Blade, Twinkle Star Sprites, Magician Lord, Neo Turf
Masters. ~150 retail cart releases over 1990-2004.

OA wires the AES + MVS pair as one slug since they share hardware,
controller, and ROM format. The Neo Geo CD variant lives at the
`neocd` slug because the load path differs (CD images need BIOS
pre-check; carts don't, but the cart-shape Neo Geo BIOS lives in
neogeo.zip).

## Upstream

- **Default core (this onboarding):** FBNeo (Final Burn Neo) — https://github.com/libretro/FBNeo
  - The canonical libretro Neo Geo emulator. Multi-arcade core that
    also drives CPS-1/2/3, Toaplan, Cave, etc., but Neo Geo support is
    its most-validated subset.
  - Buildbot path: https://buildbot.libretro.com/nightly/windows/x86_64/latest/fbneo_libretro.dll.zip
  - License: non-commercial (FBNeo's BSD-ish license with non-commercial
    use clause for the SNK content).
- **Alternates (per-system Cores override):** None practical for
  Phase 0. MAME proper drives Neo Geo with much higher CPU cost; FBNeo
  is the buildbot default. Standalone Neo Geo emulators (NeoRageX, etc.)
  aren't libretro cores.
- **Vendored:** No. Operator drops the buildbot .dll.

## ROM format

Neo Geo games ship in two canonical formats:

- **`.neo`** — single-file No-Intro Neo Geo dump. Modern format,
  preferred for archival. FBNeo reads `.neo` directly.
- **`.zip`** — MAME-compatible ROM-set. Multi-file zip containing
  per-game `<game>.p1` (program), `<game>.c1`-`<game>.cN` (graphics),
  `<game>.m1` (sound CPU), `<game>.s1` (sprite font), `<game>.v1`-`<game>.vN`
  (samples). FBNeo reads these directly.

The `.zip` extension collides with MAME (both MAME and Neo Geo use
MAME-style ROM-sets). The library scanner runs a **content-peek
disambiguation** (see `apps/oa-shell/src/archive.rs::peek_zip_for_neogeo`):
if a `.zip` file contains files matching `*.p1` AND `*.s1` (the
characteristic Neo Geo ROM-set signature), the scanner emits a
`systemHint: "neogeo"` so the frontend ingest classifies it as Neo
Geo. Other `.zip` files fall through to MAME by default.

## BIOS

Neo Geo cart playback **requires** `neogeo.zip` in `<exe_dir>/system/` —
a multi-ROM zip containing the canonical BIOS ROMs (`sp-s2.sp1`,
`sm1.sm1`, `lo-s.s2`, etc.). FBNeo reads the BIOS ROMs out of the zip
rather than expecting a single `.bin` file.

The shell pre-checks file existence via `check_neogeo_bios` in
`apps/oa-shell/src/main.rs` — Phase 0 ships an existence-only check
because the zip's content SHA-1 varies by MAME revision + Universe
BIOS presence. FBNeo handles content validation internally if the
file exists; missing-BIOS surfaces a clean error toast.

**Phase 2 polish:** peek into the zip and verify canonical BIOS ROM
files are present (rather than just zip existence) — would catch
the "operator placed a wrong zip there" failure mode that the current
existence-only check misses.

## Native timing

- **NTSC:** 59.18 Hz (the Neo Geo's actual refresh rate — slightly
  off from standard 60 Hz), 320×224 visible.
- **PAL:** Some MVS cabinets shipped in PAL regions at 50 Hz; FBNeo
  handles region selection via core options.

## Input

10-button arcade layout — defined in `apps/oa-shell/src/bindings.rs::neogeo`:

- 4-way (or 8-way) joystick — UP/DOWN/LEFT/RIGHT
- 4-button face (A/B/C/D in fighter convention: A=weak/light, B=medium,
  C=heavy, D=special/CD button)
- START + COIN (COIN doubles as SELECT in operator mode)

Identity-mapped to libretro RetroPad bits. FBNeo's standard libretro
mapping:

| Neo Geo button | libretro bit | Keyboard | Gamepad |
|---|---|---|---|
| A (primary) | B (0) | Z | East |
| B (secondary) | A (8) | X | South |
| C (tertiary) | Y (1) | A | West |
| D (quaternary) | X (9) | S | North |
| START | START (3) | Enter | Start |
| COIN | SELECT (2) | Key5 | Select |
| UP/DOWN/LEFT/RIGHT | (4-7) | Arrows | DPad |

Per the cross-system "Z is primary" rule, keyboard **Z** → Neo Geo A
(libretro B). COIN sits on **Key5** matching the RetroArch / MAME
convention ("5 = insert coin P1") so operators with arcade muscle
memory don't need to remap.

## Current status (2026-05-20)

**Works:**
- Core resolves via `default_core_dll_for_system("neogeo") →
  "fbneo_libretro.dll"`.
- 10-button input mapped via `neogeo_to_libretro_bits` (identity).
- Library scanner content-peek disambiguates Neo Geo `.zip` from
  MAME `.zip` via the `.p1+.s1` signature; single-file `.neo` dumps
  classify directly by extension.
- Theme accent: deepest+most-saturated red in the lineup at hue 18° +
  L=0.50 + C=0.27 (cluster bottom alongside VB 7° / MAME 12° / NES 28°).
- BIOS pre-check via `check_neogeo_bios` in main.rs — existence-only at
  Phase 0; missing `neogeo.zip` surfaces a clean error toast.

**Not yet validated:**
- Real game launch — needs operator validation against known-good
  ROM-sets. Suggested test set: **Metal Slug 1/2/3/X**, **King of
  Fighters '97 / '98**, **Samurai Shodown II**, **Garou: Mark of the
  Wolves**, **Magician Lord**, **Last Blade 2**, **Pulstar**.
- ROM-set hash matching against libretro-database — wired for `.neo`
  single-file dumps. ROM-set (multi-file `.zip`) hash matching is
  set-based and not currently supported (same Phase 2 gap MAME has).
- Cover sync via libretro-thumbnails `SNK_-_Neo_Geo` — wired but
  needs operator validation pass.

## Per-core docs

- `ROADMAP.md` — phase tracking for Neo Geo specifically.
- `SESSION_LOG.md` — Shipped / Almost / Next per session.
- `KNOWN_GAME_BUGS.md` — per-game compatibility issues.
- `DECISIONS.md` — Neo Geo-specific integration choices.
