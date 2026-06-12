# Controller Identity & Auto-Config — Session Log

Newest first. Three lines per entry: **Shipped / Almost / Next**.

---

## 2026-06-12 — Phase 0 identity spike (code landed; hardware-validation pending)

- **Shipped:** R1 resolved by reading the actual crate source — `gilrs 0.11.1`
  (already in-build) exposes `vendor_id()`/`product_id()`/`uuid()`/`os_name()`
  natively, so **no upgrade and no Windows raw-input needed**; decision =
  **(a) Rust reads VID/PID directly + (c) frontend stays profile authority**
  (b rejected). Cross-layer `device-key` spec written down:
  `vidpid:<vid>:<pid>` (lowercase 4-hex) else `name:<slug>` (DECISIONS
  D9–D11). Implemented identically on both layers —
  `frontend/src/platform/nav/deviceKey.ts` (`deriveDeviceIdentity`, parses
  Chrome + Firefox `id` forms, R5 name-hash fallback) and
  `crates/oa-input/src/device_key.rs` (`device_key`). Connect-time logs on
  both pollers (`gamepad.ts` + `assign_pad`) emit the derived key for
  hardware cross-check. Tests: 7 frontend (vitest) + 3 Rust, all green;
  `cargo test -p oa-input` (22) + `-p oa-shell` (837) pass; frontend
  typecheck + lint clean. Branch `feat/controller-identity`.
- **Almost:** the spike's last exit-criterion — VID/PID validated on **real
  hardware** for the operator's wired Switch Pro + one XInput pad. Code is
  ready; needs the operator to plug each pad and paste the
  `[oa-gamepad] connected` (frontend) + `oa-input: identity device-key=`
  (Rust) log lines so we confirm the keys match (esp. the XInput no-VID/PID
  fallback case).
- **Next:** operator validates the two pads (pause-gate per their "Phase 0
  only, then pause" call), then decide Phase 1 (thread `device-key` through
  `NavEvent` + key `port_pads` by device-key for replug stability) vs.
  jumping to Phase 2 (the Switch Pro normalization fix).

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
