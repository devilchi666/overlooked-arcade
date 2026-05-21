# intv Decisions Log

Mattel Intellivision-specific integration choices. Project-wide decisions live in `docs/DECISIONS.md`. Append-only.

---

## 2026-05-19 — FreeIntv as the default Intv core

**Decision:** `default_core_dll_for_system("intv") → "freeintv_libretro.dll"`.

**Why:** FreeIntv is the only actively-maintained libretro Intellivision core shipped through the buildbot. jzIntv (the long-standing standalone Intv emulator) doesn't have a maintained libretro wrapper. No alternate to consider for Phase 0.

---

## 2026-05-19 — 10-button layout (D-pad + 4 sides + START + SELECT); keypad deferred to Phase 2

**Decision:** `INTV_BUTTONS` ships 10 entries — D-pad + UPPER_L + UPPER_R + LOWER_L + LOWER_R + START (keypad ENTER) + SELECT (keypad CLEAR). The 10 numeric keypad buttons (KP0-KP9) are documented as Phase 2 polish, NOT included in Phase 0.

**Why:** The Intellivision side action buttons cover the vast majority of gameplay; most games map their primary actions to the 4 corner buttons + the 2 paired-lower / paired-upper combos. The full 12-key numeric keypad is mostly used at game start (mode/difficulty selection) and in a small subset of titles (Utopia, B-17 Bomber) for mid-game input.

Phase 0 ships the 80% case — playable 8-button games — and defers the 12-keypad coverage to Phase 2 where the same work also lands ColecoVision's full keypad surface and a per-game keypad-mapping UI. Phase 0 maps keypad CLEAR + ENTER to libretro SELECT + START as the most-used pair across the library.

---

## 2026-05-19 — 16-direction disc as D-pad 8-way (analog deferred to Phase 2)

**Decision:** The Intellivision disc controller (16-direction analog) maps to libretro D-pad (8-way) in Phase 0.

**Why:** Same shared analog-input infrastructure dependency as Atari 2600 paddles, Atari 7800 Trak-Ball, Robotron 2084 twin-stick, and SMS Light Phaser. Phase 0 takes the 8-way approximation as "playable but slightly less precise than original hardware" — Phase 2 lands the analog work once shared infra is ready.

FreeIntv has a core option to enable 16-direction mapping when an analog stick is bound; this becomes accessible to operators once OA's input layer routes analog axes through.

---

## 2026-05-19 — Deep Mattel navy at hue 260° / L=0.50 / C=0.17

**Decision:** `[data-system="intv"]` ships `oklch(0.50 0.17 260)`.

**Why:** Period-correct Mattel branding (the late-'70s / early-'80s "Intelligent Television" marketing leaned heavily on saturated navy — the Intellivision logo, console face, and most US/UK box art all used this hue). Sits 10° from SNES violet (270°) and 15° from Genesis cobalt (245°), but the lightness axis separates them in mixed library tiles: Intv = deep dark navy (L=0.50), SNES = mid violet (L=0.62), Genesis = bright cobalt (L=0.62). The visual hierarchy reads as a deliberate family of related-but-distinct blues/purples.

---

## 2026-05-19 — `.int` only; exclude `.bin` globally

**Decision:** `extensions = ["int"]`. `.bin` excluded.

**Why:** Same rationale as 2600 / Coleco. `.int` is No-Intro standard for Intellivision dumps; users with `.bin`-shaped libraries configure per-folder `*.bin → intv` rules.

---

## 2026-05-19 — Two-file BIOS requirement (exec.bin + grom.bin)

**Decision:** Document both `exec.bin` (Executive ROM, 4 KB) AND `grom.bin` (Graphics ROM, 2 KB) as REQUIRED in `<exe_dir>/system/`. No pre-check yet — Phase 2 polish item.

**Why:** Intellivision is unusual in that it needs TWO BIOS files (not the typical one). exec.bin handles boot + I/O; grom.bin holds sprite + font data used by the STIC video chip. Games refuse to render correctly without grom.bin (sprites would be garbage); they fail to boot at all without exec.bin. Both must be present.

A Phase 2 BIOS pre-check (mirroring the planned PCE-CD syscard one) would surface a clear "missing exec.bin / grom.bin" error instead of FreeIntv's failure mode.
