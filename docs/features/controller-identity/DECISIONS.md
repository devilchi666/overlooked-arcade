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
