# virtualboy Decisions Log

Nintendo Virtual Boy-specific integration choices. Project-wide decisions live in `docs/DECISIONS.md`. Append-only.

---

## 2026-05-20 — Beetle VB as the default Virtual Boy core

**Decision:** `default_core_dll_for_system("virtualboy") → "mednafen_vb_libretro.dll"`. No widely-shipped alternate.

---

## 2026-05-20 — Single LEFT D-pad in Phase 0; right D-pad deferred

**Decision:** `VIRTUALBOY_BUTTONS` ships 10 entries — LEFT D-pad + A + B + L + R + START + SELECT. The Virtual Boy's UNIQUE second D-pad (right side) is NOT exposed in Phase 0.

**Why:** Beetle VB maps the right D-pad through libretro's right analog stick by default. Exposing it as 4 additional digital bindings would require both:
1. A core-options reconfiguration in Beetle VB (route right analog to libretro L2/R2/L3/R3 bits), AND
2. OA's shared analog-input infrastructure (currently deferred — same gate as 2600 paddles, Intv 16-direction disc, Channel F plunger axes).

Most VB games (~17 of 22) use only the left D-pad — Mario's Tennis, V-Tetris, Wario Cruise, Jack Bros, Galactic Pinball, the puzzle-likes. The 5 dual-D-pad games (Mario Clash, Wario Land VB, Teleroboxer, Red Alarm, Vertical Force) are playable single-D-pad but lose authentic feel; documented in KNOWN_GAME_BUGS as the Phase 2 polish gate.

This is a deliberate Phase 0 scope cut — shipping 10 buttons with a clear "Phase 2 unlocks the right D-pad" path is better than shipping a 14-button layout where 4 of the buttons don't actually do anything in-game.

---

## 2026-05-20 — Deep VB red accent at hue 7° / L=0.55 / C=0.26

**Decision:** `[data-system="virtualboy"]` ships `oklch(0.55 0.26 7)`.

**Why:** Period-correct for the iconic Virtual Boy monochrome-red LED palette — the entire system's visual identity was "red on black, no other colors". Choosing any non-red theme accent would feel categorically wrong.

The collision risk with MAME (12°), NES (28°), and Channel F (25°, low-C) is resolved on the lightness + chroma axes:

| System | Hue | L | C | Reads as |
|---|---|---|---|---|
| **VB** | 7° | 0.55 | **0.26** | Deep neon-LED red (highest chroma) |
| MAME | 12° | 0.64 | 0.24 | Bright scarlet (lighter) |
| NES | 28° | 0.62 | 0.22 | Big-Box crimson (warmer, lighter) |
| Channel F | 25° | 0.45 | 0.06 | Cedar earth-tone (low-chroma) |

The VB's L=0.55 makes it darker than MAME + NES; the C=0.26 makes it the most saturated red — the visual hierarchy reads as "VB is the deep red, MAME is the bright red, NES is the medium red, Channel F is the brown".

---

## 2026-05-20 — `plain` default shader, NOT crt-lite

**Decision:** `systemThemes.virtualboy.defaultShaderPreset = "plain"`. Crt-lite is the project-wide handheld default but VB is the exception.

**Why:** The Virtual Boy displayed via LED projectors + mirror arrays — NOT a CRT. There were no scanlines, no shadow mask, no phosphor — just bright red pixels on absolute black. The OA dark surface already provides the black background; adding CRT scanlines + bloom would actively MUDDY the crisp red-on-black aesthetic that defines the VB's look.

`plain` shader preserves the raw red-on-black pixel art the games shipped with. Operators wanting CRT-style modulation can switch per-system; the default respects the period reality.

This is the FIRST OA system that explicitly chose `plain` over `crt-lite` for the default shader. Documented here so future LED/LCD-but-not-CRT systems (e.g. early VFD-display arcade ports) follow the same precedent.

---

## 2026-05-20 — No BIOS — distinguish from "BIOS optional" pattern

**Decision:** Document VB explicitly as "no BIOS" rather than "BIOS optional".

**Why:** The Virtual Boy never had a BIOS at all — the cart ROM was the entire firmware. This differs from systems like Channel F or O2 which have OPTIONAL BIOSes (the system did ship with one, but emulators can substitute). Documenting "no BIOS" explicitly prevents the false impression that the operator should hunt for a `vb_bios.bin` file.

---

## 2026-05-20 — `.vb` only; no `.bin` collision concern

**Decision:** `extensions = ["vb"]`. `.bin` not registered.

**Why:** Same cross-system policy. `.vb` is well-standardized in modern No-Intro sets; the VB community hasn't relied on `.bin` the way Coleco / O2 / Channel F have.
