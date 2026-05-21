# pcfx — Roadmap

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

- ✅ `oa_core::SystemId::PcFx` variant + `parse_system_id` arm
  (`pcfx | pc-fx | pcefx`).
- ✅ `bindings.rs::pcfx` module — 12-button PCE 6-button pad layout
  (d-pad + I-VI + RUN + SELECT). Separate from `pce::*` which is
  2-button only.
- ✅ `default_core_dll_for_system("pcfx") → "mednafen_pcfx_libretro.dll"`.
- ✅ `rom_hashes` → `&[]` with NO_DAT_SYSTEMS entry.
- ✅ `media::repo_for_system_id` → `NEC_-_PC-FX`.
- ✅ `check_pcfx_bios` + `PCFX_BIOS_KNOWN_HASHES` (single canonical
  `pcfx.rom` entry — PC-FX was Japan-only). Slotted into CD-launch
  dispatch.
- ✅ Frontend SystemId / systemThemes / CSS (Plan A: anime pink-magenta
  `oklch(0.62 0.24 320)` in tight WS→O2 gap).
- ✅ Per-core docs scaffold.

---

## ⬜ Phase 1 — First PCFX game running

- ⬜ Operator validation: Battle Heat / Tyoushin Heiki Zeroigar /
  Team Innocent.
- ⬜ FMV streaming validation — PC-FX library is FMV-heavy.
- ⬜ Save state F5/F8 round-trip mid-disc.

---

## ⬜ Phase 2 — Polish

- ⬜ Disc-id extraction — PC-FX shares the PCE-CD-family disc-id
  format. Extend cd_id.rs with PCFX branch.
- ⬜ Japanese-text fonts in the library tile (PCFX titles are nearly
  all Japanese-only; library cover art is the operator's lifeline).

---

## 2026-05-21 — Stale-cleanup audit

The Phase 1+ items above were written when this system onboarded, before cross-system infrastructure (Phases 1.5 / 2.5–2.8 / 3 / 4 + direct-launch CLI) landed. Many `⬜` items are actually shipped — see `docs/cores/AUDIT_2026-05-21.md` for the per-item breakdown (stale vs open-code vs open-operator) for this system.
