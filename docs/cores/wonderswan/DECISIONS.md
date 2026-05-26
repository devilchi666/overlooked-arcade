# wonderswan Decisions Log

Bandai WonderSwan-specific integration choices. Project-wide decisions live in `docs/DECISIONS.md`. Append-only.

---

## 2026-05-20 — Beetle WonderSwan as the default core

**Decision:** `default_core_dll_for_system("wonderswan") → "mednafen_wswan_libretro.dll"`. No widely-shipped alternate.

---

## 2026-05-20 — Single slug covers WS + WS Color + SwanCrystal

**Decision:** One `wonderswan` slug for original mono WS (1999), WonderSwan Color (2000), and SwanCrystal (2002 — same hardware as WSC with an improved LCD).

**Why:** Same single-slug-covers-multi-hardware reasoning as `gb` (DMG + CGB). The three WS variants share CPU + memory architecture; Beetle WS auto-detects mono vs color from the ROM header and renders accordingly. Users mentally bucket their library as "my WonderSwan games" rather than splitting mono / color / SwanCrystal.

The libretro-database keeps WS + WSC in separate dat files; we merge them via `fetch_and_parse_all`. The libretro-thumbnails repos are also separate (`Bandai_-_WonderSwan` + `Bandai_-_WonderSwan_Color`); Phase 0 uses the mono repo as primary, WSC covers via the parallel repo is a documented Phase 2 follow-up (same multi-repo gap as `gb` ↔ GBC).

---

## 2026-05-20 — 7-button layout; dual-physical-D-pad rotation handled core-side

**Decision:** `WONDERSWAN_BUTTONS` ships 7 entries — D-pad + A + B + START. No SELECT (the WS hardware has none); the hardware Sound (volume) button doesn't get a RetroPad bit (Beetle WS core option only).

**Why:** The WonderSwan's UNIQUE design is the dual D-pad layout — X-pad and Y-pad mounted at right angles so the operator can rotate the device 90° for vertical games. Beetle WS reads the ROM header's orientation flag and auto-routes the active D-pad accordingly: in horizontal mode, X-pad → libretro D-pad; in vertical mode, Y-pad → libretro D-pad.

From the OA bindings layer this is invisible. Exposing both physical D-pads as bindable inputs would require:
1. Disabling Beetle WS's auto-rotation (operator opt-in core option), AND
2. Surfacing 8 d-pad bindings (4 per physical D-pad) instead of 4.

Phase 0 prefers the core-managed rotation since it "just works" out-of-box for both horizontal AND vertical games. Operators wanting manual rotation control configure Beetle WS's core options to disable auto-rotation, then bind to the per-system Bindings UI as needed (Phase 2 polish — adds dedicated `X_UP/DOWN/LEFT/RIGHT` + `Y_UP/DOWN/LEFT/RIGHT` bindings as an alternate layout).

---

## 2026-05-20 — Pearl-lavender accent at hue 305° / L=0.70 / C=0.14

**Decision:** `[data-system="wonderswan"]` ships `oklch(0.70 0.14 305)`.

**Why:** Period-correct for the WonderSwan Color sherbet/pearl shell variants (1999-2004 era — pearl white, sherbet pink, frost crystal, sky blue, lavender purple translucent shells were the dominant WS Color aesthetic). The lavender / pearl tone differs from the saturated purples (SNES 270° L=0.62 violet, Lynx 290° L=0.65 purple, GBA 285° L=0.55 indigo) by sitting in the open 295-320° hue range, with lighter lightness (L=0.70) and lower chroma (0.14) reading as "pearl" rather than "vivid".

The 20° gap to GBA on one side and the 20° gap to O2 (325° L=0.62 C=0.18 rose-fuchsia) on the other gives clean separation in a mixed library.

---

## 2026-05-20 — `.ws` + `.wsc`; no `.bin` collision concern

**Decision:** `extensions = ["ws", "wsc"]`. `.bin` not registered.

**Why:** Same cross-system policy. Both `.ws` and `.wsc` are well-standardized No-Intro extensions and were never replaced by `.bin` in the WS community's dump sets.

---

## 2026-05-20 — BIOS optional

**Decision:** Document `bios.ws` + `bios.wsc` as OPTIONAL in `<exe_dir>/system/`.

**Why:** Beetle WS has internal BIOS replacement; games launch without external BIOS files. The original BIOSes handled the WS boot splash + name-entry / clock-init screens — period-correct but not gameplay-affecting. Standard "BIOS optional" documentation pattern matches GB / GG / O2 / Channel F.

---

## 2026-05-20 — `crt-lite` shader default; no Virtual-Boy-style `plain` exception

**Decision:** `systemThemes.wonderswan.defaultShaderPreset = "crt-lite"`.

**Why:** Unlike Virtual Boy (LED projector — no scanlines period), the WonderSwan used a passive-matrix LCD. LCD source is closer to the "smooth, no scanlines" end of the spectrum than CRT, but `crt-lite` is still the OA-wide handheld convention (Lynx, GG, GB, GBA, WS) since the dedicated `lcd-handheld` shader preset hasn't shipped yet. Following the family convention.

**2026-05-24 — Superseded.** Default flipped to `lcd-handheld` in `frontend/src/themes/registry.ts::systemThemes.wonderswan.defaultShaderPreset` as part of the handheld-family wave (gb / gbc / gba / gamegear / ngp / wonderswan / pokemini / psp). The 2026-05-20 reasoning — "passive-matrix LCD source is closer to smooth than CRT" — is exactly what `lcd-handheld` was built for; defaulting to it now is the correct successor decision, not a reversal.
