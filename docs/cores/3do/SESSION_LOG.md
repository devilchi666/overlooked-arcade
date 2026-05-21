# 3do Session Log

---

## 2026-05-20 — Phase 0 onboarding (paired with jaguar + pcfx)

- **Shipped:** SystemId variant (ThreeDo), parse_system_id arm,
  `bindings.rs::threedo` module (11-button: d-pad + A/B/C + L/R +
  STOP + PLAY + START; no SELECT — 3DO controller doesn't have one),
  default core Opera (formerly 4DO), media + rom_hashes arms
  (NO_DAT_SYSTEMS for CD-shape), `check_3do_bios` +
  `THREEDO_BIOS_KNOWN_HASHES` table with 4 canonical regional/manufacturer
  entries (Panasonic FZ-1, FZ-10, GoldStar GDO-101M, Sanyo Try),
  CD-launch BIOS dispatch arm extended with `"3do"` branch.
  CSS theme: deep 3DO purple-magenta 297° L=0.55 C=0.22 in the tight
  Lynx 290° → WS 305° gap. Per-core docs scaffold.
- **Almost:** Phase 1 operator validation.
- **Next:** Operator drops `opera_libretro.dll` + a regional 3DO BIOS,
  marks a 3DO folder via Import Wizard, launches Star Control II /
  Road Rash / The Need for Speed.
