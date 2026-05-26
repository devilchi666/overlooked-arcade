// Controller-nav event types shared across the gamepad poller, focus
// manager, and hint bar. See docs/features/controller-nav/README.md.

/// Logical button names. Maps the Web Gamepad API "standard layout"
/// indices to friendly names. `select` is button 8 (back/share);
/// `start` is button 9 (start/options). `home` is button 16 (guide).
export type NavButton =
  | "a"
  | "b"
  | "x"
  | "y"
  | "l1"
  | "r1"
  | "l2"
  | "r2"
  | "start"
  | "select"
  | "l3"
  | "r3"
  | "home";

/// 4-way logical direction. Diagonal stick positions resolve to the
/// dominant axis; the deadzone keeps idle drift from firing.
export type NavDirection = "up" | "down" | "left" | "right";

/// Phase markers. `down` and `up` are edge-triggered. `repeat` fires
/// for held directions after an initial delay, at a steady rate.
/// Buttons do NOT auto-repeat (button mashing is the caller's job).
export type NavPhase = "down" | "up" | "repeat";

export type NavButtonEvent = {
  kind: "button";
  button: NavButton;
  phase: NavPhase;
  /** Web Gamepad API index of the pad that fired the event. */
  gamepadIndex: number;
};

export type NavDirectionEvent = {
  kind: "direction";
  direction: NavDirection;
  phase: NavPhase;
  /** Which physical control produced the direction. */
  source: "dpad" | "stick-left";
  gamepadIndex: number;
};

export type NavEvent = NavButtonEvent | NavDirectionEvent;
