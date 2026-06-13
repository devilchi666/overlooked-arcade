# Controller Identity & Auto-Config — Decisions

Append-only. Newest at the bottom. The *why* matters more than the *what*.
Full context in [../../PLANS/controller-identity-substrate.md](../../PLANS/controller-identity-substrate.md).

---

## 2026-06-12 — D1–D8 locked at planning (operator design Q&A)

- **D1 — Two pollers, one shared config.** Keep the separate menu (WebView /
  Web Gamepad API) + emulator (Rust / gilrs) pollers; unify the *data* they
  read, not the pollers. *Why:* the pollers are separate for valid hot-path
  reasons (no IPC on either path; non-overlapping contexts); the real flaw is
  that each invented its own nonexistent controller config.

- **D2 — Identity = VID/PID (controller TYPE).** The shared cross-layer key.
  Stable across replug + reboot; the only id BOTH layers can derive (frontend
  from `gamepad.id`, Rust from gilrs GUID / platform). Per-physical-unit
  (serial) distinction deferred past v1. *Why:* type-level solves the auto-
  config + layout-fix problem; per-unit is overkill and serials aren't always
  exposed.

- **D3 — Three data files, normalize-once.** `controllers.json` (VID/PID →
  canonical layout), `systems-input.json` (per-system schema), `default-
  maps.json` (canonical → per-system defaults, ONE per system). *Why:*
  normalizing every pad to one canonical layout collapses the controller
  dimension, so defaults need only one map per system — not a
  controllers×systems matrix. New pad = one DB line; new system = one schema
  + one map. The engine never changes (expansion-proof).

- **D4 — SDL/Xbox canonical layout + SDL `gamecontrollerdb` format.** Adopt
  the SDL standard as the normalization target AND the file format for
  `controllers.json`. *Why:* thousands of pads for free + community interop;
  the DB is already bundled by gilrs.

- **D5 — Override depth = per-system; no controllers×systems matrix.** A
  hand-edited gameplay binding applies to that SYSTEM across all controllers;
  a pad with oddly-placed buttons is fixed once in its own profile (the DB /
  wizard). *Why:* covers ~all real needs, keeps menus simple, avoids the
  per-controller-per-system grid that made other launchers a maze. (Per-game
  overrides stay a separate shipped layer.)

- **D6 — Foundation-first phasing.** Stable identity in BOTH layers (incl.
  the Rust VID/PID spike) before any normalization/auto-config UI. *Why:*
  everything depends on a stable cross-layer device key; prove it first
  (operator chose this over a faster path to their personal Switch Pro fix).

- **D7 — Wizard for unknown controllers.** No-match pad → "press each button"
  capture that authors a `controllers.json` override entry; first-connect
  prompt + Settings entry. *Why:* the zero-config "it just works" moment;
  gamecontrollerdb won't cover everything.

- **D8 — Separate arc that composes.** Stays distinct from
  `dynamic-controller-info` (core's supported devices) +
  `dynamic-input-descriptors` (per-game labels) — physical-pad side vs
  emulated-console side. They meet in the bindings UI; they don't merge.
  *Why:* clean slicing; each ships incrementally.

### Architectural note — the two tangled concerns (resolves the original confusion)
The existing remap dropdown conflated **layout** ("what IS the physical A
button?" — per-controller, missing today) with **semantics** ("what should A
*do*?" — canonical→verb for menus, canonical→system control for gameplay).
This arc separates them: `controllers.json` owns layout; the existing
OA-wide verb map + the new `default-maps.json` own semantics. Once layout is
normalized, semantics "just work" for any pad.

---

## 2026-06-12 — Phase 0 spike findings (R1 resolved + device-key spec)

- **D9 — Rust VID/PID = option (a), and it costs nothing.** R1's premise was
  wrong: the plan assumed *"gilrs 0.11 doesn't expose a UID publicly."* It
  does. The exact version already in our build graph — `gilrs 0.11.1` /
  `gilrs-core 0.6.7` — exposes on the high-level `gilrs::Gamepad`:
  `vendor_id() -> Option<u16>`, `product_id() -> Option<u16>`,
  `uuid() -> [u8; 16]` (SDL GUID), and `os_name() -> &str`
  (verified at `gilrs-0.11.1/src/gamepad.rs:807–840`). So **no gilrs upgrade
  and no Windows raw-input (option b) are needed** — Rust reads VID/PID
  natively. (b) is rejected; the GUID is kept available for Phase 5 per-unit
  refinement but VID/PID is read directly, not parsed out of the GUID.
  *Why it matters:* R1 was the highest-risk item gating the whole arc; it
  collapsed to a few lines of existing API.

- **D10 — Keep (c) frontend-as-identity-authority too, for the XInput gap.**
  (a) gives Rust a self-consistent key; (c) stays the *profile-resolution*
  authority. Reason: the Web Gamepad `id` for XInput pads on Chrome/WebView
  is often `"Xbox 360 Controller (XInput STANDARD GAMEPAD)"` with **no
  VID/PID** (risk R5), while gilrs *does* see the VID/PID. So for XInput pads
  the two layers' keys can diverge. We don't try to reconcile the fallback
  keys; instead the frontend (richest identity + owns `controllers.json`
  matching) resolves the profile and hands Rust the normalized per-port
  mapping at launch. **Final call: (a) + (c). (b) rejected.**

- **D11 — `device-key` format (cross-layer spec).** Both layers derive the
  same string for a VID/PID-bearing pad:
  - VID/PID present → `vidpid:<vid>:<pid>` — each lowercase, zero-padded to
    4 hex digits (Switch Pro = `vidpid:057e:2009`).
  - VID/PID absent → `name:<slug>` — `slug` = lowercase, runs of
    non-alphanumerics collapsed to a single `-`, leading/trailing `-`
    trimmed, empty → `unknown`.
  Implemented identically in `frontend/src/platform/nav/deviceKey.ts`
  (`deriveDeviceIdentity`) and `crates/oa-input/src/device_key.rs`
  (`device_key`), each with unit tests. The frontend additionally parses two
  `id` formats — Chrome `"…Vendor: 057e Product: 2009"` and Firefox
  `"057e-2009-<name>"` — both defended by regex with the name-slug fallback
  (R5). *Limitation (documented, not fixed in v1):* the `name:` fallback is
  **not** guaranteed to agree across layers (Web `id` name ≠ OS name); D10's
  (c) covers that case.

---

## 2026-06-12 — Phase 2 findings + the per-layer normalization split

- **D12 — `controllers.json` (frontend) is OA-curated in Web-Gamepad-index
  space, NOT a raw SDL `gamecontrollerdb` import.** Refines (does not overturn)
  D4. *Finding:* SDL `gamecontrollerdb` numbers buttons in SDL-joystick-index
  space, which does NOT align 1:1 with the Web Gamepad API's raw `buttons[]` /
  `axes[]` indices. So pasting SDL rows into the *frontend* poller would
  mis-map. Instead the bundled `controllers.json` holds OA-curated profiles in
  Web-index space, keyed by device-key, applied ONLY when the browser reports a
  non-standard mapping (`mapping !== "standard"`). The Phase-3 wizard appends
  to it; a later runtime override layer (appDataDir) will merge over the seed.
  *Why D4 still holds:* the "thousands of pads for free" + community-interop
  benefit is realized fully on the **Rust** side (see D13); the canonical model
  is still the SDL/Xbox layout; only the frontend's data SOURCE differs.

- **D13 — Phase 2 is frontend-only; the Rust gameplay poller already
  normalizes.** *Finding:* gilrs (via its Windows Gaming Input backend + built-
  in SDL mappings) already hands `oa-input` canonical `gilrs::Button` values,
  and `apps/oa-shell/src/bindings.rs` already binds by canonical name
  (`"South" => GamepadButton::South`). So a non-standard pad like the Switch
  Pro is *already* canonical in-game — matching the operator's symptom that
  only MENUS were broken. Phase 2 therefore touches only the Web-Gamepad menu
  poller; no Rust changes. (The per-system canonical→control default maps are a
  separate concern — Phase 4.)

- **Architecture — layout vs semantics, realized.** `controllers.json` owns
  LAYOUT (raw index → canonical). The fixed `CANONICAL_TO_NAV` table
  (`controllerProfiles.ts`) + the existing `navBindings` verb map own
  SEMANTICS (canonical → NavButton → verb, incl. the A/B-swap for Nintendo "B
  confirms"). Face buttons map by POSITION (south = bottom = Confirm-default),
  so Nintendo's physical A/B swap is a preference layer, not a layout concern.

---

## 2026-06-12 — Phase 2.5 diagnostics: controller test window (operator Q&A)

Operator couldn't tell whether Phase 2 works — no in-app observability (only
console logs), glyphs looked wrong, and it was unclear which panes are even
controller-navigable. Reframed the confusion as **three independent layers**
that were tangled: (1) mapping (raw→logical button), (2) nav logic (focus
system response), (3) presentation (glyphs/hints). A test window instruments
Layer 1 so the other two can be debugged with confidence.

- **D14 — Build a controller test window, MVP first, under a new
  Settings → Controllers section.** MVP = passive live display: identity
  (name / device-key / VID-PID / `mapping` / which profile matched) + raw
  input (button indices lit, axis bars incl. the HAT axis) + the normalized
  chain (raw N → canonical → NavButton/Dir). Reuses `resolveLayout` +
  `deriveDeviceIdentity` (no logic duplication; a `describePad()` helper in
  `gamepad.ts` returns exactly what the poller uses). *Why:* it's the
  observability that unblocks validating the whole arc, AND it's the capture
  primitive the Phase-3 wizard + a future remap UI both reuse. Visual gamepad
  diagram deferred to a later full version. **Settings → Controllers is the
  home for all this arc's UI** (test → wizard → remap → glyph choice).

- **D15 — Per-controller glyph sets PARKED until mapping is proven solid.**
  (Layer 3.) "Glyphs don't match my controller" is a real but SEPARATE problem
  from mapping correctness — Nintendo/Xbox/PlayStation button symbols keyed by
  identity. Parked (PARKING_LOT 2026-06-12) to avoid conflating cosmetic
  labeling with functional mapping; revisit once the test window proves Layer 1.

- **Noted for later phases (not now):** (a) **reduced-layout control schemes**
  — pads lacking shoulders/sticks/buttons need nav verbs reachable via
  alternative inputs (a verb-fallback concern for the remap/wizard design);
  (b) **arcade-cabinet keyboard-encoder input** (iPAC-style joystick/button/
  spinner→keystroke routing) — major separate planning, parked. Both flagged
  by the operator 2026-06-12. A **nav-coverage audit** (which panes are
  controller-navigable) was requested and is in flight.
