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
