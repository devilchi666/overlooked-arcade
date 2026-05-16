# Vendored upstream — Beetle PCE Fast

This directory is a vendored copy of the libretro Beetle PCE Fast core (a port of
Mednafen's PCE Fast module). It lives in-tree because we modify it; the
`PATCHES/` directory tracks our local diffs versus upstream.

## Upstream

- **URL:** https://github.com/libretro/beetle-pce-fast-libretro
- **Commit vendored:** (shallow clone) — see git history of that repo
- **Date vendored:** 2026-05-15
- **License:** GPL-2.0-or-later (see `COPYING` here and the project root `LICENSE`)

## Layout

- `mednafen/` — Mednafen-derived core proper (the part we want)
  - `mednafen/pce_fast/` — PCE-specific CPU/VDC/PSG/CD source
  - `mednafen/` (top level) — supporting infrastructure (streams, state, helpers)
  - `mednafen/cdrom/`, `hw_misc/`, `sound/`, `tremor/` — utilities
- `libretro.cpp`, `libretro-common/` — libretro glue layer
  - We **intentionally do not link** `libretro.cpp` or most of `libretro-common/*.c`.
  - We DO need `libretro-common/include/retro_inline.h` on the include path because
    Mednafen headers reference `INLINE`. We inject that via cc::Build define rather
    than including the file directly.
- `intl/`, `jni/`, `deps/` — internationalization, Android JNI, third-party deps. Not used.

## Local modifications

See `PATCHES/`. Empty at vendor time. Each future modification gets a numbered
`.patch` file describing what changed and why.

## Re-vendoring

`scripts/vendor-update.ps1` (TBD) will diff this tree against a fresh upstream
checkout to regenerate the patch series. Not yet implemented.
