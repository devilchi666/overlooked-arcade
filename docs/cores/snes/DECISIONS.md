# snes — Decisions

Per-core integration choices. Project-wide architectural decisions live in `docs/DECISIONS.md`.

---

## 2026-05-18 — Snes9x as the default, bsnes as the accuracy alternative

OA defaults to `snes9x_libretro.dll` because (a) it's the standard libretro SNES core most operators already have, (b) it handles every cart in the canonical compatibility set including the special-chip games (SuperFX / SA-1 / DSP) without separate config, (c) it runs comfortably at native speed on modest hardware. bsnes is the per-system Cores swap for power users who want cycle-accuracy.

---

## 2026-05-18 — Diamond-layout face button keyboard defaults

Default keyboard mapping puts B/A on Z/X (lower-right diamond) and Y/X on A/S (upper diamond). This mirrors the ZSNES-derived convention going back to the late 90s and matches what most SNES PC emulator users already have muscle memory for. The diamond is intentional — B (south, "primary action") on Z keeps the same key as NES B, since most SNES games use B as the primary button.

---

## 2026-05-18 — Violet at 270° accent (not Lynx purple)

Both SNES and Lynx use a purple-family accent, but they have to read distinct in the sidebar. Lynx is a saturated purple at 290° (Epyx '89 box palette); SNES is a slightly cooler violet at 270° (closer to the SNES launch palette's diamond-button hue). Side by side they're visually different — Lynx leans warm-purple-toward-magenta, SNES leans cool-violet-toward-blue.

---

## 2026-05-18 — All 4 SNES ROM extensions in the scanner

`.sfc` / `.smc` / `.fig` / `.swc` are all included even though `.sfc` is the only "canonical" modern dump format. The others are legacy copier formats from the 90s that still appear in older dump collections. Both Snes9x and bsnes handle all four via header detection, so including them in the scan is free.
