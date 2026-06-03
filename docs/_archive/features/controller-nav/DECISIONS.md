# Controller Navigation — Decisions

## 2026-05-26 — Pad source: Web Gamepad API (frontend rAF poll)

**Decision:** Read gamepad state in the frontend via `navigator.getGamepads()`
polled in a `requestAnimationFrame` loop, rather than piping events from the
Rust gilrs poller via Tauri events.

**Why:**
- Zero new Rust↔JS plumbing. No event channel, no state machine for
  who-owns-input-this-frame.
- The existing gilrs poller in `crates/oa-input/src/lib.rs` is already gated
  via `set_enabled` to game-window focus, so it only fires when the emulator
  is running. The UI poller fires when the emulator is NOT running. Two
  pollers, two non-overlapping contexts, no conflict.
- Web Gamepad API is a stable browser standard; no surprise compat issues in
  the Tauri WebView (uses the host browser engine).
- If we later need richer device info (rumble, advanced axes) for UI feedback
  we can revisit — but Phase 0 only needs button + direction events.

## 2026-05-26 — POC scope: library grid + left sidebar (no dialogs)

**Decision:** Slice D wires VirtualLibraryGrid + LeftSidebar tree as the
proof-of-concept. Settings dialogs come in Phase 1 alongside wizard step
components, not Phase 0.

**Why:**
- The couch-gamer audience (primary per `docs/PLANS/guided-setup.md` §3)
  spends 95% of their time in the library view. Proving the primitives
  there proves them for the audience that matters.
- Dialogs add nested focus-trap complexity (escape key, focus restore on
  close) that's better solved once the base primitives are debugged in a
  simpler context.
- Shoulder-bumper navigation between sidebar and grid demonstrates focus
  groups + cross-group jumps, which is the harder concept to validate
  before the wizard adopts the same pattern.

## 2026-05-26 — Focus ring: 2px subtle outline

**Decision:** Focus indicator is a 2px solid outline in the system accent
color, 8px corner radius matching the focused element. No glow halo, no
scale-up / shadow lift.

**Why:**
- Quiet at desk distance (the existing tertiary audience), still visible at
  10ft (the primary couch audience). The accent color carries the visibility
  burden.
- Doesn't fight per-system theming: the outline picks up the existing
  `--color-oa-accent` CSS variable per registry.
- Future per-system-UI Stage 1 may upgrade signature systems (Vectrex, etc.)
  to a custom ring style via the SystemUIConfig escape hatch. Keep the
  baseline boring so the upgrades feel special.

## 2026-05-26 — Dialog primitive auto-publishes B-close handler + baseline HintRegion

**Decision:** Every `Show when={open}` branch of the `Dialog` primitive
(frontend/src/layout/Dialog.tsx) mounts a `DialogBackHandler` that
registers `props.onClose` with the global back stack, plus a baseline
`HintRegion ({ b: "Close" })`. Individual dialogs don't need to wire
back-on-B themselves — they get it for free.

**Why:**
- The shell has ~20 modal dialogs (Display, Audio, Game Properties,
  Cheats, Core Options, Platform Media, Debug Log, Widget Customizer,
  …). Hand-wiring B-close into each one would be ~20 identical
  `useBackHandler(props.onClose)` calls scattered through the tree
  with the same likelihood of being forgotten as it is of being
  remembered.
- Innermost-wins on both the back stack and the HintRegion stack
  means a dialog can still override the baseline by mounting its own
  `useBackHandler` (handles confirm-on-close, etc.) or its own
  HintRegion deeper in the tree. The default is the right default.
- This is the same architectural argument as "every Tauri command goes
  through `withWindow`" — load-bearing infrastructure belongs in the
  primitive, not in every caller.

## 2026-05-26 — Read-only / utility surfaces stay mouse-only in v1

**Decision:** Controller-nav v1 wires the **play path** end-to-end but
leaves read-only widgets and utility chrome on a mouse + keyboard
fallback. Concretely: right-sidebar read-only widgets above the action
row, the pin toggle / sidebar-hide button in the right sidebar header,
QuickSettings sub-views (rewind / TAS / memory / video), and dynamic-
during-open menu content all stay non-focusable for now.

**Why:**
- The couch flow (primary audience per `docs/PLANS/guided-setup.md` §3)
  is "library grid → pick a tile → launch / saves / info." That path
  is now fully controller-navigable through Slice M.
- Configuration / utility flows are desk-and-mouse flows in practice.
  Operators sitting on the couch don't open the widget customizer or
  the TAS sub-view. Wiring those for a controller without an actual
  use case adds focus-trap complexity for zero observed benefit.
- Inventory matters: ROADMAP.md tracks each non-focusable surface as a
  `⬜` polish item with a written rationale, so we know exactly which
  bits to revisit if operator feedback says they're needed.

## 2026-05-26 — Suppress frontend Web Gamepad poll while gilrs owns input

**Decision:** When the emulator has focus, the frontend Web Gamepad
poller is suppressed so a single button press doesn't drive both the
running game and the UI. Gate logic lives in `App.tsx`. Two-window
shell uses DOM `focus` / `blur` events on the library WebView's
`window` rather than Tauri 2's `is_focused`.

**Why:**
- Two pollers (gilrs in Rust for the emulator, navigator.getGamepads
  in the WebView for UI) is the right architecture — confirmed in the
  Phase 0 design (see entry above) — but they need a clear handoff.
  Without one, operator reported menus opening mid-gameplay.
- Tauri 2's `is_focused` returns false for the no-WebView game window
  even when it has user focus (see feedback memory:
  `feedback_tauri_no_webview_is_focused_unreliable`). DOM focus events
  on the library WebView's `window` ARE reliable cross-platform and
  fire on every click-through between the two HWNDs (see reference
  memory: `reference_tauri_dom_focus_reliable`).
- The four-case gate (nav disabled / no game / single-window+game /
  two-window+game) is explicit rather than collapsed into a single
  boolean so each case is auditable from the source — important
  because future work (kiosk shell, in-game overlays) may add more
  cases.
