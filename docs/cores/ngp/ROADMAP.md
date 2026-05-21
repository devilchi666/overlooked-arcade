# ngp — Roadmap

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

- ✅ `oa_core::SystemId::NeoGeoPocket` variant + parse_system_id arm
  (`ngp | ngpc | neopocket | neo-geo-pocket`).
- ✅ `bindings.rs::ngp` module — 7-button handheld pad
  (d-pad + A + B + OPTION). All dispatch arms wired.
- ✅ `default_core_dll_for_system("ngp") → "mednafen_ngp_libretro.dll"`.
- ✅ `rom_hashes` → two no-intro dats merged (NGP + NGPC, gb/wonderswan
  pattern).
- ✅ `media::repo_for_system_id` → `SNK_-_Neo_Geo_Pocket_Color`.
- ✅ Frontend SystemId / systemThemes / CSS (Plan A: pearl yellow-green
  105° / L=0.80 / C=0.12, open band, evokes NGPC translucent shell).
- ✅ No BIOS pre-check needed (BIOS-free).
- ✅ Per-core docs scaffold.

**Acceptance gate:** Operator drops `mednafen_ngp_libretro.dll`, scans
NGP/NGPC ROMs, sees pearl-yellow-green tiles, launches a known-good
ROM.

---

## ⬜ Phase 1 — First NGP/NGPC game running

- ⬜ Operator validation. Suggested: **SNK vs Capcom: Card Fighter's
  Clash 1+2**, **SNK vs Capcom: Match of the Millennium**, **Sonic
  Pocket Adventure**, **KOF R-2**, **Magical Drop Pocket**.
- ⬜ Mono (.ngp) + color (.ngc) auto-detect validation.
- ⬜ Save state F5/F8 round-trip.
- ⬜ Cover sync via libretro-thumbnails — operator pass.

---

## ⬜ Phase 2 — Polish

- ⬜ `lcd-handheld` shader preset (same gap gb/gba/gg/ws have — crt-lite
  is the temporary default).
- ⬜ NGP-mono vs NGPC visual differentiation in the library tile (e.g.
  badge or subtitle).

---

## Scope clarifications

- **NGP + NGPC single slug.** Beetle NeoPop handles both via header
  auto-detect, same pattern as gb (DMG+CGB) and wonderswan (WS+WSC).
- **No BIOS required.**
- **No analog input.** NGP/NGPC was strictly digital.

---

## 2026-05-21 — Stale-cleanup audit

The Phase 1+ items above were written when this system onboarded, before cross-system infrastructure (Phases 1.5 / 2.5–2.8 / 3 / 4 + direct-launch CLI) landed. Many `⬜` items are actually shipped — see `docs/cores/AUDIT_2026-05-21.md` for the per-item breakdown (stale vs open-code vs open-operator) for this system.
