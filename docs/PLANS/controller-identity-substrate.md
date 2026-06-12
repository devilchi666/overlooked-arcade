# Controller Identity & Auto-Config Substrate

**Status:** Planning locked 2026-06-12 (operator design Q&A this session).
Execution queued, **foundation-first**. Owner-of-decisions: the operator.

**One-line goal:** a foundational layer — *under both* the menu-nav poller
and the emulator poller — that gives every controller a **stable identity**
(VID/PID) and **auto-configures** both shell-nav and per-system gameplay
bindings from a small set of shared data files, with a press-the-buttons
wizard for unknown pads. Fixes the "non-standard pad's buttons land wrong"
problem (e.g. a wired Switch Pro) and the "replug shuffles the ports"
problem in one stroke.

**Origin:** surfaced while wiring controller nav for the Metadata editor
(2026-06-12). The operator's wired Switch Pro only half-worked in menus
("Y selects, nothing else does"), which traced to OA having **no controller
identity layer at all** — both input pipelines are positional and assume the
Web Gamepad "standard layout." This was parked intent (PARKING_LOT
2026-06-11 "TODO 3 — per-controller-ID gameplay-binding auto-config") that
had never been designed. This is that design.

> All `file:line` anchors below came from a 2026-06-12 read-only survey
> (4 parallel subagents). They're approximate — **verify at execution time**.

---

## Problem statement — what exists today (surveyed 2026-06-12)

**Two completely separate input pipelines, both positional, neither with
stable identity:**

| | Shell nav (menus) | Emulator (in-game) |
| --- | --- | --- |
| Layer | Frontend, Web Gamepad API (`frontend/src/platform/nav/gamepad.ts`) | Rust, `gilrs` 0.11 (`crates/oa-input/src/lib.rs`) |
| Identifies a pad by | volatile `gamepad.index` | ephemeral `gilrs::GamepadId`, connection-order port assignment (`port_pads: [Option<GamepadId>; 5]` ~`:265`) |
| Button model | assumes Web "standard layout" (`BUTTON_NAMES` index→name ~`:24`) → **breaks non-standard pads** | per-system bit tables (`bindings.rs`) → libretro |
| Bindings stored | OA-wide verb map (`nav_bindings.json`, `navBindings.ts`) | per-**system** JSON (`appDataDir/bindings/<systemId>.json`, `bindings.rs` ~`:3`) — **no device field** |
| Device identity in storage | **none** | **none** |

**The keystone gap:** `gamepad.id` *contains* the stable USB **VID/PID**
(Switch Pro = `Vendor: 057e Product: 2009`) and `gilrs` + a bundled
`gamecontrollerdb.txt` are *already in the build graph* — but **nothing
parses or uses any of it.** `gamepad.id` is logged on connect
(`gamepad.ts` ~`:201`) and discarded. RetroArch has the same gap; the
industry fix (ES-DE / Batocera / RetroBat / OpenEmu) is **SDL
`gamecontrollerdb` + stable per-device UIDs + a press-the-buttons wizard.**

**Why two pipelines?** A performance/simplicity choice, *not* a requirement:
menus run in the WebView (free Web Gamepad API), the emulator runs in Rust
(gilrs) — and they never run simultaneously. Piping Rust→WebView for menu
nav would add IPC latency for no benefit (DECISIONS 2026-05-26 "two
pollers, two non-overlapping contexts"). **We keep the two pollers; we
unify the config they read.**

**Prior art already in the tree (build on, don't redo):**
- Verb-native nav (theming D18) + nav-remap UI (D30) + A/B-swap overlay
  (`navBindings.ts`). The OA-wide verb map (`A→Confirm`) stays.
- HID HAT-axis decoding (`gamepad.ts` ~`:79–117`) — prior art for runtime
  non-standard-pad handling.
- `dynamic-controller-info` plan (core's *supported* device lists, env
  `SET_CONTROLLER_INFO`) + `dynamic-input-descriptors` plan (per-game
  button *labels*, env `SET_INPUT_DESCRIPTORS`) + their v21/v22 caches.
  **These are the EMULATED-console side; this arc is the PHYSICAL-pad
  side. They compose, they don't merge.**
- Shared analog infra Phases A–G (multi-port device dispatch, pressure,
  rumble, sensors) — shipped.

---

## Locked decisions (operator Q&A 2026-06-12)

- **D1 — Two pollers, one shared config.** Keep the separate menu + emulator
  pollers; unify the *data* (the three files below) both read. *Why:* the
  pollers are separate for good hot-path reasons; the actual flaw is that
  each invented its own (nonexistent) controller config.

- **D2 — Identity key = VID/PID (controller TYPE).** Stable across replug +
  reboot, and the only thing BOTH the frontend (`gamepad.id` parse) and Rust
  (gilrs GUID / platform) can derive. All Xbox pads share one profile, all
  Switch Pros another. Per-physical-unit (serial) distinction is a later
  refinement, NOT v1.

- **D3 — Three data files, normalize-once.**
  1. `controllers.json` — VID/PID → **canonical layout** (raw buttons/axes →
     standard model). SDL `gamecontrollerdb` *format* + OA curation.
  2. `systems-input.json` — per-system control schema (what each emulated
     system needs bound; mostly already in `bindings.rs`).
  3. `default-maps.json` — **canonical → each system's controls**, ONE map
     per system.
  *Why:* because #1 normalizes *every* pad to the same canonical layout, #3
  needs only one default per system — NOT a controllers×systems matrix. New
  pad = one line in #1; new system = its #2 schema + #3 map. The engine never
  changes. This is the "bulletproof for expansion" property.

- **D4 — Standardize on the SDL/Xbox canonical layout + the SDL
  `gamecontrollerdb` file format** for #1. *Why:* inherit thousands of pads
  for free + interop with the ES-DE/scraper community; `gamecontrollerdb.txt`
  is already bundled by gilrs.

- **D5 — Override depth = per-system (+ per-controller layout in the
  profile). NO controllers×systems matrix.** A hand-edited gameplay binding
  applies to that SYSTEM across all controllers; a pad whose physical buttons
  sit oddly is fixed once in its own profile (#1 / the wizard). *Why:* covers
  ~all real needs, keeps menus simple, and avoids the per-controller-per-
  system grid that made other launchers' setup a maze. (Per-game overrides
  remain a separate, already-shipped layer.)

- **D6 — Foundation-first phasing.** Build the stable-identity layer in BOTH
  pipelines (incl. the Rust VID/PID spike) before any normalization/auto-
  config UI. *Why:* everything hangs on a stable, cross-layer device key;
  prove it first.

- **D7 — Wizard for unknown controllers.** A pad with no `gamecontrollerdb`
  match gets a "press each button" capture flow (first-connect prompt +
  Settings entry) that authors a `controllers.json` override entry. *Why:*
  the zero-config "it just works" moment; gamecontrollerdb won't cover
  everything.

- **D8 — Separate arc that composes.** Stays its own `controller-identity`
  arc (physical-pad identity + mapping). `dynamic-controller-info` /
  `dynamic-input-descriptors` stay the core-side plans. They meet in the
  bindings UI; they don't merge.

---

## Architecture — the canonical pivot

The whole system hinges on **one canonical controller model** (the
normalization target). Define it once (the SDL/Xbox standard):

```
faces:   south(A) east(B) west(X) north(Y)
dpad:    up down left right
sticks:  left(x,y,click) right(x,y,click)
bumpers: l1 r1        triggers: l2 r2 (analog)
system:  start select guide
```

Two distinct concerns that today are tangled (and caused the operator's
original confusion — the remap dropdown conflated them):

1. **Layout** (`controllers.json`): "what IS this pad's physical A button?"
   — per-controller, from the DB / wizard. *This is the half OA is missing.*
2. **Semantics** (canonical → meaning): "what should A *do*?"
   - Menus: canonical → **verb** (the existing OA-wide `navBindings` map +
     A/B-swap; preference, e.g. Nintendo B-confirms).
   - Gameplay: canonical → **system control** (`default-maps.json` +
     per-system override).

Once #1 normalizes every pad to canonical, **the existing nav verb map and
the per-system gameplay maps "just work" for any controller** — including
the Switch Pro — because they operate on canonical, not raw.

**Data flow (both pipelines, shared files):**
```
physical press
  → [poller: gamepad.ts (menu) | gilrs (game)]
  → identity: VID/PID  ── controllers.json ──▶ canonical button
                                                   │
              menus ◀── navBindings (canonical→verb) ┤
              game  ◀── default-maps + per-system override (canonical→system control)
                                                   │
        compose with: dynamic-controller-info (device-type dropdown)
                     + dynamic-input-descriptors (semantic label in the UI)
```

---

## Phases (foundation-first, D6)

### Phase 0 — Identity spike (de-risk; tiny, FIRST) — CODE LANDED 2026-06-12; hardware-validation pending
Prove the cross-layer device key before building on it.
**Status:** parser + Rust VID/PID read + connect-time logs + unit tests
shipped on `feat/controller-identity`; device-key spec + (a)+(c) decision
recorded (DECISIONS D9–D11). Remaining exit-criterion = operator pastes the
two `[oa-gamepad] connected` + `oa-input: identity device-key=` log lines for
the Switch Pro + an XInput pad to confirm the keys match expectations.
- Frontend: parse `gamepad.id` → `{vid, pid, name}` → a canonical
  `device-key` string. (VID/PID regex; `gamepad.id` embeds
  `Vendor: xxxx Product: yyyy`.)
- Rust: get a stable VID/PID from `gilrs` 0.11 — resolve the unknown:
  - **(a)** upgrade gilrs (newer versions expose `Gamepad::uuid()` = SDL
    GUID, which embeds VID/PID), or
  - **(b)** a Windows platform API (raw input), or
  - **(c)** **frontend-as-identity-authority**: the frontend (which has the
    richest identity) resolves the profile and hands Rust the normalized
    mapping per-port at launch.
  **Lean: (a) for Rust-side stable port assignment + (c) for profile
  resolution.** If (a) is hard, (c) alone carries v1.
- **Output:** a documented `device-key` format + a decision on (a)/(b)/(c).
  Everything downstream depends on this.

### Phase 1 — Identity foundation (both layers)
- Define the canonical model (above) in a shared spec/type.
- Frontend: thread `device-key` through `NavEvent` (alongside, not replacing,
  `gamepadIndex`); persist nothing yet — just make identity available.
- Rust: key `port_pads` assignment by `device-key` (replug-stability
  groundwork — a reconnecting pad reclaims its prior port).
- No behavior change beyond identity availability + replug-stable ports.

### Phase 2 — `controllers.json` (normalization → the Switch Pro fix)
- Bundle SDL `gamecontrollerdb` (+ an OA-overrides layer) as `controllers.json`.
- Frontend: match VID/PID → profile → normalize raw buttons/axes → canonical
  in `gamepad.ts` (replaces the blind `BUTTON_NAMES` standard-layout
  assumption). **Non-standard pads (incl. the Switch Pro) start working in
  menus here.**
- Rust: same normalization for the gameplay poller.
- Note: foundation-first means the operator's personal Switch Pro fix lands
  *here* (Phase 2), not Phase 0 — an accepted trade for the right architecture.

### Phase 3 — Unknown-controller wizard
- "Press each button" capture flow (canonical button → wait for press →
  record raw index/axis) → authors a `controllers.json` OA-override entry.
- First-connect prompt (pad with no match) + a Settings entry point.
- Reuses the existing raw-button diagnostics (`gamepad.ts` already logs every
  raw index) + the HAT-axis detector as the capture primitives.

### Phase 4 — `systems-input.json` + `default-maps.json` (gameplay auto-config)
- Formalize the per-system control schema (extract from `bindings.rs` bit
  tables) → `systems-input.json`.
- `default-maps.json`: canonical → per-system default binding (one per
  system; sensible conventions).
- Auto-bind at launch: `controllers.json` (normalize) ∘ `default-maps` →
  applied to `oa-input` per port — no hand-binding any system.
- Per-system override layer (D5): operator edits the canonical→system map;
  applies to all pads. (Surfaces in `SystemBindingsEditor`.)

### Phase 5 — Replug stability + multi-controller polish
- Persist `port ↔ device-key` assignments; restore on reconnect (both
  layers). Closes the "unplug P2, replug → it's now P1" problem (the
  RetroArch gap).
- Multi-pad assignment by identity, not connection order.

### Phase 6 — Compose with the core-side plans
- Bindings UI shows the full chain: physical (profile) → canonical →
  system control (schema) → core's semantic label
  (`dynamic-input-descriptors`) → device-type dropdown from the core's
  supported list (`dynamic-controller-info`). The convergence surface.

---

## Open questions / risks

- **R1 (highest) — Rust VID/PID. RESOLVED 2026-06-12 (Phase 0 spike).** The
  premise was wrong: `gilrs 0.11.1` (already in the build) exposes
  `Gamepad::vendor_id()` / `product_id()` / `uuid()` / `os_name()` natively
  (`gilrs-0.11.1/src/gamepad.rs:807–840`). Decision = **(a) + (c)**: read
  VID/PID directly in Rust (no upgrade, no platform API), frontend stays the
  profile-resolution authority. See feature DECISIONS D9–D11.
- **R2 — Cross-layer pad↔port matching.** Even with VID/PID both sides, two
  identical pads (same VID/PID) can't be told apart at type-level (D2). For
  v1, connection order disambiguates same-type pads; per-unit (serial) is
  deferred. Document the limitation.
- **R3 — `gamecontrollerdb` button model vs OA canonical.** The SDL format
  maps to SDL's standard; confirm a clean 1:1 to OA's canonical (it should —
  OA's canonical *is* the SDL/Xbox model per D4).
- **R4 — DB freshness.** `gamecontrollerdb` updates upstream; decide bundle-
  and-update cadence (ship a snapshot; allow operator refresh later).
- **R5 — Web Gamepad `id` string variance.** Format varies across
  browsers/OS; VID/PID extraction must be defensive (regex with fallback to
  a name hash when VID/PID absent).

## Composition with existing plans

- `docs/PLANS/dynamic-controller-info.md` — core's per-port supported device
  list (Zapper / Super Scope / …). Feeds the device-type dropdown; THIS arc
  feeds the physical→canonical→system mapping underneath it.
- `docs/PLANS/dynamic-input-descriptors.md` — per-game semantic labels
  ("B = Whip"). The label layer on top of this arc's bindings.
- The metadata-editor controller-nav close-out (region nav in the takeover)
  is downstream of Phase 2 (the Switch Pro must work before that nav is
  testable) — it stays parked behind this arc.

## Critical files (anchors — verify at execution)

- `frontend/src/platform/nav/gamepad.ts` — `BUTTON_NAMES` ~`:24`,
  `handleConnect` id-logging ~`:201`, HAT decode ~`:79`, poll/emit ~`:310`.
  (Identity parse + normalization land here.)
- `frontend/src/platform/nav/navBindings.ts` — canonical→verb map (stays).
- `frontend/src/platform/nav/types.ts` — `NavEvent` (add `device-key`).
- `crates/oa-input/src/lib.rs` — `port_pads` ~`:265`, `assign_pad` ~`:800`
  (identity-keyed assignment + replug).
- `crates/oa-input/Cargo.toml` — `gilrs = "0.11"` (the spike's upgrade
  target).
- `apps/oa-shell/src/bindings.rs` — per-system bit tables + JSON store (→
  `systems-input.json` schema source + per-system override target).
- `apps/oa-shell/src/main.rs` — `apply_bindings_to_poller` ~`:9023`,
  `to_libretro_bits` ~`:9041` (the apply path).
- `frontend/src/engine/SystemBindingsEditor.tsx` — the bindings UI (Phase 4 +
  6 surface).
- New: `assets/controllers/gamecontrollerdb.txt` (bundled), `controllers/`
  override layer, `systems-input.json`, `default-maps.json` (exact homes TBD
  in Phase 0).

## Verification

- Phase 0: a written device-key spec + a working VID/PID read on BOTH layers
  for at least the operator's Switch Pro + one XInput pad (validated against
  the `[oa-gamepad] connected` log).
- Each phase: `cargo test -p oa-shell` + `cargo test -p oa-input` green;
  frontend typecheck + lint silent.
- Operator playtest gates: Phase 2 (Switch Pro nav works), Phase 4 (a fresh
  system auto-binds on connect with zero hand-binding), Phase 5 (replug
  doesn't shuffle ports).

## Reference

- PARKING_LOT 2026-06-11 "TODO 3 — per-controller-ID gameplay-binding
  auto-config" (the parked intent this formalizes).
- LAUNCHBOX_RESEARCH §3.4 + LAUNCHER-LANDSCAPE M16 + RETROARCH-FEATURE-SURVEY
  (SDL gamecontrollerdb auto-config + stable-UID benchmark).
- theming DECISIONS D18 (verb-native nav) + D30 (remap UI).
