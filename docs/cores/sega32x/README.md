# sega32x — Sega 32X

Onboarded 2026-05-20 (paired with Sega CD). Drives the Sega 32X via the
libretro **PicoDrive** core (`picodrive_libretro.dll`) by default — the
only mainstream libretro core with Sega 32X support.

The Sega 32X was Sega's 1994 cart-slot expansion for the Mega Drive,
adding two Hitachi SH-2 RISC CPUs and a dedicated VDP for enhanced
graphics. The intended bridge between Mega Drive and Saturn — it shipped
with only ~36 retail cart releases over its 18-month lifespan
(Doom 32X, Virtua Racing Deluxe, Knuckles' Chaotix, Star Wars Arcade,
After Burner Complete, Kolibri, NBA Jam Tournament Edition 32X) before
Sega pivoted full attention to Saturn. One of the most-overlooked
consoles in OA's thesis — small library, mostly forgotten, hardware
that's genuinely interesting (twin SH-2s in a cart was unusual for the
era).

OA wires the 32X cart-shape path here. 32X-CD games (Night Trap 32X,
Corpse Killer 32X, Slam City) layer 32X on top of Sega CD — they
route through `segacd` with a stacked per-game core override and need
both the Sega CD BIOS and the 32X .dll loaded together. Phase 3+ work.

## Upstream

- **Default core (this onboarding):** PicoDrive — https://github.com/libretro/picodrive
  - Lightweight multi-Sega core that pairs MD emulation with dedicated
    SH-2 emulation for the 32X's twin RISC CPUs.
  - Buildbot path: https://buildbot.libretro.com/nightly/windows/x86_64/latest/picodrive_libretro.dll.zip
  - License: MAME-like (LGPL components + non-commercial restriction
    on the SH-2 emulator).
- **Alternates (per-system Cores override):** None practical. Genesis
  Plus GX doesn't emulate 32X at all. ClownMDEmu is MD-cart-only.
  Standalone Sega 32X emulators exist (e.g. Gens/GS) but aren't shipped
  as libretro cores.
- **Vendored:** No. Operator drops the buildbot .dll into
  `<exe_dir>/cores/`. If we ever need to fork for an OA-specific
  extension, we maintain our own libretro-frontend build per the
  project DECISIONS 2026-05-16 pivot.

## ROM format

- **`.32x`** — canonical headerless 32X cart dump (No-Intro standard).
  PicoDrive reads this directly. Most modern dump sets ship `.32x` as
  the primary extension.
- **`.bin`** is intentionally NOT registered. Same collision rationale
  Genesis uses (PCE-CD track files, future Atari sets, audio tracks).
  Operators with `.bin` 32X dumps rename to `.32x` — PicoDrive doesn't
  care about the extension.
- **`.md` / `.smd`** intentionally NOT cross-registered. Even though
  32X games are technically Mega Drive carts with extra hardware,
  classifying a `.md` 32X dump under the cart-Genesis slug would route
  it to ClownMDEmu (which can't emulate the SH-2s) and the game would
  boot to a blank screen. The slug separation forces the right core
  selection.

## BIOS

**None required** for stock 32X cart playback. PicoDrive's 32X path
synthesizes the SH-2 boot vector internally — the operator doesn't
need a 32X-specific BIOS file.

(32X-CD games stack on top of Sega CD and DO need the Sega CD BIOS in
`<exe_dir>/system/`; those route through `segacd` and are covered by
that slug's BIOS pre-check. The cart-only 32X path is BIOS-free.)

## Native timing

- **NTSC:** 59.92 Hz, 320×224 visible (same as Mega Drive cart games —
  the 32X output overlays the MD framebuffer).
- **PAL:** 49.70 Hz, 320×240 visible.
- PicoDrive reports timing per-loaded-ROM via `retro_system_av_info`.
  The 32X's dedicated VDP can output 256×224 / 320×224 / 320×240
  framebuffer modes; PicoDrive composes the 32X VDP output with the
  MD VDP output into a single framebuffer the renderer takes as-is.

## Input

Identical to Genesis — the 32X uses the same 6-button Mega Drive
controller slot on the parent Mega Drive (the 32X cart-slot addon
doesn't add controller ports). `bindings::defaults_for("sega32x")`
shares the `default_genesis_bindings()` path; `bit_for` / `buttons_for`
/ `to_libretro_bits` all dispatch `genesis` + `segacd` + `sega32x` to
the same `GENESIS_BUTTONS` table and identity remap.

Per the cross-system "Z is primary" rule:
- **Z** → MD **B** (middle face, libretro bit 0) — primary action.
- **X** → MD **C** (right face, libretro bit 8) — secondary.
- **A** → MD **A** (left face, libretro Y bit 1) — tertiary.
- Q/S/W → MD X/Y/Z (top row of 6-button face).
- Enter → START, RShift → MODE.

The 32X's small library leans heavily on the 6-button pad for action /
fighting / racing titles (Virtua Fighter 32X, Cosmic Carnage), so the
default 6-button announce works well out of the box.

## Current status (2026-05-20)

**Works:**
- Core resolves via `default_core_dll_for_system("sega32x") →
  "picodrive_libretro.dll"`.
- 10-button input mapped via the shared genesis dispatch arm (identity
  to libretro RetroPad).
- Library scanner classifies `.32x` files as `sega32x`.
- Theme accent: neon orange at hue 42° + L=0.68 + C=0.22 — period-
  accurate to the 1994 32X marketing palette (the mushroom-cap unit +
  the "32X" logotype were both fiery orange-on-black).
- BIOS-free path — no pre-check needed.

**Not yet validated:**
- Real game launch — needs operator validation against a known-good
  ROM. Suggested test cart set: **Knuckles' Chaotix**, **Virtua Racing
  Deluxe**, **Doom 32X**, **Star Wars Arcade**, **Kolibri**.
- Save state F5/F8 round-trip. PicoDrive supports `retro_serialize`;
  should work via the existing path but needs live validation against
  twin-SH-2 state.
- Multi-region testing: NTSC US + NTSC JP + PAL EU 32X carts to
  confirm region auto-detect (NTSC 59.92 Hz vs PAL 49.70 Hz timing).
- libretro-database hash matching against `metadat/no-intro/Sega - 32X.dat`
  — wired but needs operator-run `Settings → Library → Identify ROMs`
  pass to confirm canonical title lookup.
- Cover sync via libretro-thumbnails `Sega_-_32X` — wired but needs
  operator validation.

## Per-core docs

- `ROADMAP.md` — phase tracking for Sega 32X specifically.
- `SESSION_LOG.md` — Shipped / Almost / Next per session.
- `KNOWN_GAME_BUGS.md` — per-game compatibility issues.
- `DECISIONS.md` — 32X-specific integration choices.
