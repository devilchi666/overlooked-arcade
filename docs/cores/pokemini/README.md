# Nintendo Pokémon Mini

Pokémon Mini (2001-2002) — the smallest first-party Nintendo platform ever shipped. Monochrome 96×64 LCD, ~10 official games + a thriving homebrew scene. Released JP/US/EU; only 10 official titles in the catalog total (5 of them Pokémon spin-offs); discontinued after ~16 months due to weak sales.

## Default core

`pokemini_libretro.dll` — the libretro port of the standalone PokeMini emulator. The only libretro option for the platform. Mature, accurate, light CPU.

## BIOS

Required: `bios.min` (4 KB) at `<exe_dir>/system/bios.min`. The Pokémon Mini BIOS is the platform's boot ROM — PokeMini won't launch without it.

Pre-checked by `check_pokemini_bios` (`apps/oa-shell/src/main.rs`). SHA-1 sourced from libretro-database (`DAAD4113713ED776FBD47727762BCA81BA74915F`).

## Extensions

`.min` — canonical Pokémon Mini cart dump. Single extension; PokeMini reads it directly.

## Controller

The PokeMini's input layout is tiny:

- 4-direction D-pad
- A button (libretro B — primary action; Z key)
- B button (libretro A — secondary action; X key)
- C button (libretro SELECT — Power / Menu; RShift key)

Phase 2.5 polish: the shake sensor (used in some titles like Pokémon Pinball Mini for paddle force) is deferred. IR is niche enough to skip entirely.

## Status

- Phase 0 onboarding: ✅ 2026-05-20 (this session)
- Phase 1 operator validation: ⬜ — drop `pokemini_libretro.dll` + `bios.min` into the appropriate folders, scan a `.min` library, launch Pokémon Pinball Mini / Pokémon Party Mini / Pichu Bros. Mini.

## See also

- `docs/cores/pokemini/ROADMAP.md` — phase tracking
- `docs/cores/pokemini/SESSION_LOG.md` — what last session shipped
- libretro-thumbnails: `Nintendo_-_Pokemon_Mini`
- libretro-database dat: `metadat/no-intro/Nintendo - Pokemon Mini.dat`
