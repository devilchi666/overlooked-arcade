# Controller Identity & Auto-Config — Session Log

Newest first. Three lines per entry: **Shipped / Almost / Next**.

---

## 2026-06-12 — Arc planned + scoped

- **Shipped:** Planning locked. 4-subagent read-only survey mapped both input
  pipelines (frontend Web-Gamepad-API nav poller + Rust gilrs emulator
  poller), confirmed neither has stable controller identity and both assume
  the Web "standard layout" (the Switch Pro break). Plan written
  ([../../PLANS/controller-identity-substrate.md](../../PLANS/controller-identity-substrate.md))
  with decisions D1–D8 + a 7-phase foundation-first roadmap + risks. Feature
  folder + NEXT.md HIGH-band Phase-0 queue created. Operator answered all six
  design forks: two-pollers-one-config, VID/PID identity, three data files
  (controllers/systems-input/default-maps), SDL canonical + gamecontrollerdb
  format, per-system override depth (no matrix), foundation-first, wizard for
  unknowns, separate-arc-that-composes.
- **Almost:** nothing in code — paperwork only.
- **Next:** **Phase 0 — identity spike.** Prove a stable cross-layer
  `device-key` (VID/PID): parse `gamepad.id` frontend-side; resolve the
  Rust-side VID/PID unknown (lean: upgrade gilrs for the SDL GUID + let the
  frontend be the identity authority at launch). Output = a documented
  device-key format + the (a)/(b)/(c) decision. R1 (Rust VID/PID) is the
  highest risk and gates everything. Then Phase 1 (identity in both layers +
  replug-stable port assignment).
