# pokemini Decisions

Per-core architectural decisions. Date entries.

---

## 2026-05-20 — Default core: PokeMini

PokeMini libretro is the only mature libretro option for the platform. Standalone PokeMini upstream has been the canonical emulator since ~2012; libretro wrapper is a straightforward port. No competition / alternates worth surfacing.

## 2026-05-20 — Phase 0 shake-sensor deferral

A couple of pack-in titles (Pokémon Pinball Mini, parts of Pokémon Party Mini) use the platform's shake sensor for paddle force / dice rolls. Phase 0 ships without shake mapping — those games are playable but a touch awkward. Phase 2.5 polish handles via gamepad rumble or a dedicated key.

## 2026-05-20 — Theme: sunny yellow 95° L=0.85

Period-correct to the 2001 launch palette (PokeMini shells shipped in 5 candy colors — Chickorita Green, Smoochum Purple, Wooper Blue, Smeargle White, and a yellow "Surfing Pikachu" promotional shell). Yellow is the dominant marketing color. L=0.85 makes this the brightest tile in OA's lineup, fitting the tiniest, friendliest first-party Nintendo platform.

## 2026-05-20 — A is primary, B is secondary, C is Power/Menu

Nintendo platform convention: A = east / primary (Z key), B = south / secondary (X key). C is the Power/Menu key (PokeMini-specific — most platforms don't have a third face button); mapped to RShift + libretro SELECT for parity with handheld SELECT functions.
