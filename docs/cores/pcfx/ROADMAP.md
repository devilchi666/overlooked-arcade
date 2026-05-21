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

- ⬜ Operator validation: Battle Heat / Tyoushin Heiki Zeroigar / Team Innocent — operator playtest.
- ⬜ FMV streaming validation — operator playtest (PC-FX library is FMV-heavy).
- ✅ Save state F5/F8 round-trip mid-disc — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).

---

## ✅ Phase 2 — Polish

- ✅ Disc-id extraction — shipped via `apps/oa-shell/src/cd_id.rs::extractors::pcfx` (finds the FX...-prefix serial after the "PC-FX:" signature); `rom_hashes` points at `metadat/redump/NEC - PC-FX & PC-FXGA`.
- ✅ Japanese-text fonts in the library tile — shipped via CJK font fallbacks in `frontend/src/index.css::--font-display`.
