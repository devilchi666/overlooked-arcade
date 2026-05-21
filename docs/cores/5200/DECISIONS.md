# 5200 Decisions

Per-core architectural decisions. Date entries.

---

## 2026-05-20 — Default core: Atari800

The libretro Atari800 core covers the full 8-bit Atari family (400/800/XL/XE home computers + 5200 console) in one .dll. Mature codebase, ~25 years of game-compat fixes, light CPU. No widely-shipped alternate in the libretro buildbot.

The home-computer side of the family is deferred from OA's wiring plan per the 2026-05-19 "consoles only" filter — but the 5200 rides into OA on the same .dll.

## 2026-05-20 — Phase 0 keypad deferral

The 5200 controller's 12-key keypad (0-9, *, #) needs the libretro KEYBOARD device to route correctly — Phase 2 polish, same approach as Jaguar's keypad. Phase 0 ships the d-pad joystick fallback + FIRE1/FIRE2/START/SELECT/RESET only. Games requiring keypad input (Missile Command's screen-coord shooting, RealSports Football's play selection) will need Phase 2 to be fully playable.

## 2026-05-20 — Phase 0 digital d-pad for analog joystick

The 5200's self-centering joystick was analog (each axis 0-228 native value). Phase 0 ships a digital d-pad fallback because most games are playable that way (Pac-Man, Star Raiders mostly fly by joystick direction, not continuous axis value). Per-game analog routing via the existing per-system Analog Bindings UI handles the rest as Phase 2 polish for the games that need it (Pole Position II steering, etc.).

## 2026-05-20 — Slug name

The slug stays `"5200"` (with a Rust variant `Atari5200` to dodge the no-leading-digit identifier rule). Matches the 2600 / 7800 pattern; the `5200`-vs-`atari5200` ambiguity is handled in `parse_system_id` with both accepted as aliases.

## 2026-05-20 — Theme: saturated red 18°

Period-correct to the 5200's iconic black-and-red faceplate + the bold angular red logo. Sits between VB 7° and MAME 12° in the warm-red cluster but separated by L+C profile: 5200 reads as the brightest, mid-chroma red of the cluster.
