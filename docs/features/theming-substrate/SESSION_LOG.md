# Theming Substrate — Session Log

Each session that touches this feature appends a 3-line entry:
**Shipped / Almost / Next.** Older entries roll to
`SESSION_LOG_ARCHIVE.md` when this file passes ~150 lines.

---

## 2026-06-06 — Planning locked

- **Shipped:** Full plan + feature folder scaffold. Plan at
  [docs/PLANS/theming-substrate.md](../../PLANS/theming-substrate.md);
  feature folder this directory. Operator decisions: one unified
  premium frontend (no LaunchBox/BigBox split); engine vs theme
  territory inside one window; engine summon = fullscreen takeover,
  top-right corner, `F12` / `Select+Start`; manifest = TOML; Kiosk
  plan's 4-layer substrate absorbed (renamed). 3-arc structure
  (ARC 1 = Minimum Viable Substrate, ~22-26 weeks; ARCs 2-3 add
  Rhai + WGSL + Theme Studio).
- **Almost:** Nothing — pure planning session, no code touched.
- **Next:** Phase 1 of ARC 1 — engine/theme surface separation.
  Extract SETTINGS + Library Manager + Import Wizard + BIOS +
  Core installer + System Health + Background Jobs from
  Retroverse into engine-owned fullscreen takeover. Write
  SURFACES.md as part of the phase. See plan §6 Phase 1 for the
  full deliverable list + acceptance gate.
