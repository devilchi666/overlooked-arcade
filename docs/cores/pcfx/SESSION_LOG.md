# pcfx Session Log

---

## 2026-05-20 — Phase 0 onboarding (paired with jaguar + 3do)

- **Shipped:** SystemId variant (PcFx), parse_system_id arm,
  `bindings.rs::pcfx` module (12-button: d-pad + I-VI + RUN + SELECT;
  separate from `pce::*` which is 2-button only), default core
  Beetle PC-FX (Mednafen lineage), media + rom_hashes arms
  (NO_DAT_SYSTEMS for CD-shape), `check_pcfx_bios` +
  `PCFX_BIOS_KNOWN_HASHES` table (1 entry — single canonical
  `pcfx.rom`, PC-FX was Japan-only), CD-launch BIOS dispatch arm
  extended with `"pcfx"` branch. CSS theme: anime pink-magenta
  320° L=0.62 C=0.24 in the tight WS 305° → O2 325° gap. Per-core
  docs scaffold.
- **Almost:** Phase 1 operator validation.
- **Next:** Operator drops `mednafen_pcfx_libretro.dll` + `pcfx.rom`,
  marks a PC-FX folder via Import Wizard, launches Battle Heat /
  Tyoushin Heiki Zeroigar.
