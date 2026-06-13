# Controller Identity & Auto-Config — Session Log

Newest first. Three lines per entry: **Shipped / Almost / Next**.

---

## 2026-06-12 — Phase 2.5: controller test window + nav-coverage audit

- **Shipped:** Operator couldn't tell whether Phase 2 worked (no in-app
  observability). Built the **controller test window** under a new
  **Settings → Controllers** category (the home for all this arc's device UI):
  - `engine/ControllerTestPanel.tsx` (`ControllersSettings`): live read-out
    per connected pad — identity (name / device-key / VID:PID / browser
    `mapping` / **which profile matched**), raw buttons (lit on press) with the
    **normalized result** each maps to (raw b1 → "A"/Confirm), and axis bars
    incl. the decoded HAT direction. Pauses nav while open (test presses don't
    toggle settings); leave with the mouse. Uses `Index` for the live arrays.
  - `gamepad.ts`: exported `describePad()` (returns exactly what the poller
    uses — single source of truth) + `decodeHat()`; wired the category into
    `SettingsPanel.tsx`. tsc + lint + nav tests (28) green.
  - This panel is also the capture primitive the Phase-3 wizard + a future
    remap UI reuse (D14).
  - **Decisions recorded** (D14 test window/home, D15 glyph sets parked);
    **parked** (PARKING_LOT): per-controller glyph sets, reduced-layout control
    schemes, arcade-cabinet keyboard-encoder input, and the updatable-data-files
    refresh story.
  - **Nav-coverage audit** ran (Explore subagent): the 5 Retroverse tabs +
    modals/menus have solid nav; the **engine surface (Settings bodies,
    metadata editor, dialogs) is the big keyboard-only gap**. Full table:
    [NAV_COVERAGE_AUDIT_2026-06-12.md](NAV_COVERAGE_AUDIT_2026-06-12.md). That's
    Layer 2 (nav wiring), distinct from this arc's Layer 1 (mapping).
- **Almost:** test window not yet operator-exercised; the Faceoff mapping is
  authored + cross-checked but still wants the live confirmation the test
  window now makes trivial.
- **Next:** operator opens Settings → Controllers, presses each button, confirms
  raw→normalized is correct (bottom = "A"/Confirm). If right → the full SDL
  gamecontrollerdb import ("test my pad first, then build it"). If wrong →
  one-line fix in controllers.json. Glyphs + nav-coverage gaps are parked
  follow-ups.

## 2026-06-12 — Phase 2 normalization infra (frontend-only; Switch Pro profile pending capture)

- **Shipped:** Phase 1 validated by operator (replug-stable ports + identity
  threading working). Phase 2 normalization **infrastructure** landed:
  - **Two findings that shaped scope** (DECISIONS D12/D13): (1) SDL
    `gamecontrollerdb` numbers buttons in SDL-joystick-index space, which
    doesn't align with Web-Gamepad raw indices → the frontend DB is
    OA-curated in Web-index space, not a raw SDL import; (2) the Rust gameplay
    poller already normalizes via gilrs' native SDL mappings (binds by
    canonical name in `bindings.rs`), so **Phase 2 is frontend-only** — which
    matches the "only menus were broken" symptom.
  - **`controllers.json`** (seed DB) + **`controllerProfiles.ts`** (resolver +
    `CANONICAL_TO_NAV` table). `gamepad.ts` now keeps a per-pad `padLayouts`
    map: standard-mapping pads use the default `BUTTON_NAMES` layout (no
    regression), non-standard pads with a profile get raw-index→canonical→Nav
    remapping. Profile-declared HAT axis seeded into the HAT detector. Connect
    log gains a `profiled` flag.
  - Tests: 6 new (`controllerProfiles.test.ts`); nav suite (28) + typecheck +
    lint green. No Rust changes.
- **Almost:** the **Switch Pro profile is UNVERIFIED** — seeded with the
  standard Switch-Pro HID button order + HAT axis 9 as a prior, but the
  operator's third-party wired "Faceoff" pad's real face/shoulder raw indices
  must be captured to confirm/correct it. The infra is done; only the data row
  is provisional.
- **Next:** operator menu playtest of the verified Faceoff profile, then the
  full-DB-import decision (see below).

## 2026-06-12 — Operator pad identified + SDL-derived profile (correcting the seed)

- **Shipped:** Read the operator's runtime log directly. Their pad is **NOT
  `057e:2009`** — it's a PDP/Faceoff clone, **`vidpid:0e6f:0184`** ("Faceoff
  Premiere Wired Pro Controller"), `mapping:""`, 14 buttons / 10 axes, DPad =
  HAT axis 9. Found the exact pad in SDL `gamecontrollerdb` (GUID
  `030000006f0e00008401000000000000`) and **cross-validated SDL↔Web index
  alignment** (14 buttons matches SDL's b0..b13; DPad-hat matches axis 9).
  Replaced the wrong seeded entry in `controllers.json` with the SDL-derived,
  cross-checked profile (south=raw 1 → "a"/Confirm — the fix). Tests updated;
  typecheck + profile tests + lint green.
- **Almost:** operator hasn't yet playtested menu nav with the corrected
  profile (should now work end-to-end).
- **Next (decision):** the operator's instinct — "import the standardized
  list" — is validated. Options: (A) keep hand-curating `controllers.json`
  per-pad (current), or (B) **bundle the full SDL `gamecontrollerdb` + a parser
  that matches by device-key and applies it on the menu poller** (DPad still via
  the runtime HAT detector). (B) realizes "thousands of pads for free" on the
  menu side and is now de-risked by the per-pad alignment check. Recommend (B)
  as the next slice; wizard (Phase 3) remains the fallback for pads no list
  covers.

## 2026-06-12 — Phase 0 validated + Phase 1 identity foundation shipped

- **Shipped:** Phase 0 hardware validation **PASSED** (operator, Switch Pro +
  XInput pad — device-key reads correctly on both layers); Phase 0 closed.
  Phase 1 landed on `feat/controller-identity` (fresh branch off the merged
  main):
  - **Canonical model** as a shared contract — `nav/canonical.ts` +
    `oa-input/src/canonical.rs` (SDL/Xbox vocabulary per D4; types only, no
    normalization yet — that's Phase 2).
  - **Frontend identity threading** — `NavEvent` gains `deviceKey`;
    `gamepad.ts` keeps a per-pad `padDeviceKeys` map (connect + initial sweep
    + lazy tick backfill), every emitted event carries the key; keyboard-synth
    events use `"keyboard"`. No persistence, no routing change yet.
  - **Rust replug-stable ports** — new `port_keys` reservation array +
    pure `choose_port` (reclaim prior port → fresh unreserved → clobber
    stale); `release_pad` keeps the reservation so a reconnecting pad reclaims
    its port instead of shuffling. R2 caveat (same-type pads) documented.
  - Tests: 6 new Rust (`port_assignment_tests`) + existing; `oa-input` (28) +
    `oa-shell` (837) green; frontend nav (22) + typecheck + lint clean.
- **Almost:** nothing partial — Phase 1 is feature-complete. Replug stability
  is unit-proven but not yet operator-playtested end-to-end (unplug P2 →
  replug → still P2 in a real multi-pad session).
- **Next:** **Phase 2 — `controllers.json` normalization** (bundle SDL
  `gamecontrollerdb` + OA overrides → map raw buttons/axes onto the canonical
  model in both pollers). **This is where the Switch Pro starts working
  correctly in menus** — the operator's original payoff. Optional pre-step:
  operator multi-pad replug playtest to confirm Phase 1 stability in the wild.

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
