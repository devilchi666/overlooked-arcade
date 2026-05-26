// Web Gamepad API rAF poller + synthetic NavEvent bus.
//
// Lifecycle: `startGamepadInput()` wires the connect/disconnect
// listeners and starts the rAF loop when at least one pad is present.
// The loop suspends automatically when the window is hidden (rAF
// browser semantics), which is the right behaviour — no nav input
// while the user isn't looking.
//
// Coexistence with the emulator: the Rust gilrs poller in
// crates/oa-input is `set_enabled`-gated to game-window focus, so it
// only fires while the emulator is running. This frontend poller fires
// when the emulator is NOT running (the operator is in the library /
// menus). Two pollers, two non-overlapping contexts.
//
// State model: per-(pad, control) pressed timestamps for buttons +
// per-pad stick direction. Edge-triggered down/up + auto-repeat for
// directions only.

import { onCleanup } from "solid-js";
import type { NavButton, NavDirection, NavEvent, NavPhase } from "./types";

/// Web Gamepad API "standard layout" mapping. Skips the dpad slots
/// (12..15) — those resolve to NavDirection events, not button events.
const BUTTON_NAMES: Record<number, NavButton> = {
  0: "a",
  1: "b",
  2: "x",
  3: "y",
  4: "l1",
  5: "r1",
  6: "l2",
  7: "r2",
  8: "select",
  9: "start",
  10: "l3",
  11: "r3",
  16: "home",
};

const DPAD_DIRS: Record<number, NavDirection> = {
  12: "up",
  13: "down",
  14: "left",
  15: "right",
};

const INITIAL_REPEAT_MS = 400;
const REPEAT_INTERVAL_MS = 80;
const STICK_DEADZONE = 0.4;

type ButtonState = { pressedAt: number; lastRepeatAt: number };
type StickState = {
  direction: NavDirection | null;
  enteredAt: number;
  lastRepeatAt: number;
};

type Listener = (event: NavEvent) => void;

const listeners = new Set<Listener>();
const buttonStates = new Map<string, ButtonState>(); // `${padIdx}:${btnIdx}`
const stickStates = new Map<number, StickState>(); // padIdx -> state
let connectedPads = 0;
let rafHandle: number | null = null;
let started = false;
let sessionEverSawGamepad = false;

/// Subscribe to nav events. Returns an unsubscribe function — call it
/// on dispose to avoid leaks. Re-subscribing the same handler is a
/// no-op (listeners is a Set).
export function onNavEvent(handler: Listener): () => void {
  listeners.add(handler);
  return () => {
    listeners.delete(handler);
  };
}

/// Solid-flavoured wrapper around onNavEvent. Auto-unsubscribes when
/// the enclosing reactive scope disposes.
export function useNavEvent(handler: Listener): void {
  const dispose = onNavEvent(handler);
  onCleanup(dispose);
}

/// Has any gamepad been seen this session? Used by the hint bar to
/// decide whether to render anything (auto-hide if no controller has
/// ever appeared).
export function hasSeenGamepad(): boolean {
  return sessionEverSawGamepad;
}

/// Start the poller. Idempotent — safe to call multiple times. Should
/// be called once at app mount.
export function startGamepadInput(): void {
  if (started) return;
  started = true;
  window.addEventListener("gamepadconnected", handleConnect);
  window.addEventListener("gamepaddisconnected", handleDisconnect);
  // Some browsers populate getGamepads() before firing connect — sweep
  // once at start so a pre-attached pad is picked up.
  const initial = navigator.getGamepads?.() ?? [];
  for (const pad of initial) {
    if (pad) {
      sessionEverSawGamepad = true;
      connectedPads++;
    }
  }
  if (connectedPads > 0 && rafHandle === null) {
    rafHandle = requestAnimationFrame(tick);
  }
}

/// Stop the poller and unwire listeners. Mostly for tests / hot
/// reload; production code wires this once and leaves it.
export function stopGamepadInput(): void {
  if (!started) return;
  started = false;
  window.removeEventListener("gamepadconnected", handleConnect);
  window.removeEventListener("gamepaddisconnected", handleDisconnect);
  if (rafHandle !== null) {
    cancelAnimationFrame(rafHandle);
    rafHandle = null;
  }
  buttonStates.clear();
  stickStates.clear();
  connectedPads = 0;
}

function handleConnect(_e: GamepadEvent): void {
  connectedPads++;
  sessionEverSawGamepad = true;
  if (rafHandle === null) {
    rafHandle = requestAnimationFrame(tick);
  }
}

function handleDisconnect(e: GamepadEvent): void {
  connectedPads = Math.max(0, connectedPads - 1);
  // Drop state for the disconnected pad so a reconnect on the same
  // index starts clean.
  stickStates.delete(e.gamepad.index);
  for (const key of Array.from(buttonStates.keys())) {
    if (key.startsWith(`${e.gamepad.index}:`)) buttonStates.delete(key);
  }
  if (connectedPads === 0 && rafHandle !== null) {
    cancelAnimationFrame(rafHandle);
    rafHandle = null;
  }
}

function tick(now: DOMHighResTimeStamp): void {
  const pads = navigator.getGamepads?.() ?? [];
  for (let i = 0; i < pads.length; i++) {
    const pad = pads[i];
    if (!pad) continue;
    pollButtons(pad, now);
    pollStick(pad, now);
  }
  if (connectedPads > 0) {
    rafHandle = requestAnimationFrame(tick);
  } else {
    rafHandle = null;
  }
}

function pollButtons(pad: Gamepad, now: number): void {
  for (let i = 0; i < pad.buttons.length; i++) {
    const isPressed = pad.buttons[i]?.pressed ?? false;
    const key = `${pad.index}:${i}`;
    const prev = buttonStates.get(key);
    const buttonName = BUTTON_NAMES[i];
    const dpadDir = DPAD_DIRS[i];
    if (!buttonName && !dpadDir) continue;

    if (isPressed && !prev) {
      buttonStates.set(key, { pressedAt: now, lastRepeatAt: now });
      if (buttonName) {
        emitButton(buttonName, "down", pad.index);
      } else if (dpadDir) {
        emitDirection(dpadDir, "down", "dpad", pad.index);
      }
    } else if (!isPressed && prev) {
      buttonStates.delete(key);
      if (buttonName) {
        emitButton(buttonName, "up", pad.index);
      } else if (dpadDir) {
        emitDirection(dpadDir, "up", "dpad", pad.index);
      }
    } else if (isPressed && prev && dpadDir) {
      // Repeat only fires for directions, never for face/shoulder buttons.
      const held = now - prev.pressedAt;
      if (held >= INITIAL_REPEAT_MS && now - prev.lastRepeatAt >= REPEAT_INTERVAL_MS) {
        prev.lastRepeatAt = now;
        emitDirection(dpadDir, "repeat", "dpad", pad.index);
      }
    }
  }
}

function pollStick(pad: Gamepad, now: number): void {
  if (pad.axes.length < 2) return;
  const x = pad.axes[0] ?? 0;
  const y = pad.axes[1] ?? 0;
  const direction = stickToDirection(x, y);

  let state = stickStates.get(pad.index);
  if (!state) {
    state = { direction: null, enteredAt: 0, lastRepeatAt: 0 };
    stickStates.set(pad.index, state);
  }

  if (direction !== state.direction) {
    if (state.direction !== null) {
      emitDirection(state.direction, "up", "stick-left", pad.index);
    }
    if (direction !== null) {
      emitDirection(direction, "down", "stick-left", pad.index);
    }
    state.direction = direction;
    state.enteredAt = now;
    state.lastRepeatAt = now;
  } else if (direction !== null) {
    const held = now - state.enteredAt;
    if (held >= INITIAL_REPEAT_MS && now - state.lastRepeatAt >= REPEAT_INTERVAL_MS) {
      state.lastRepeatAt = now;
      emitDirection(direction, "repeat", "stick-left", pad.index);
    }
  }
}

/// Discretize a stick (x, y) into one cardinal direction or null.
/// Outside the deadzone, the dominant axis wins (diagonal-up-left
/// resolves to whichever of left/up has the larger absolute value).
export function stickToDirection(x: number, y: number): NavDirection | null {
  const ax = Math.abs(x);
  const ay = Math.abs(y);
  if (ax < STICK_DEADZONE && ay < STICK_DEADZONE) return null;
  if (ax >= ay) return x < 0 ? "left" : "right";
  return y < 0 ? "up" : "down";
}

function emitButton(button: NavButton, phase: NavPhase, gamepadIndex: number): void {
  const event: NavEvent = { kind: "button", button, phase, gamepadIndex };
  for (const l of listeners) l(event);
}

function emitDirection(
  direction: NavDirection,
  phase: NavPhase,
  source: "dpad" | "stick-left",
  gamepadIndex: number,
): void {
  const event: NavEvent = { kind: "direction", direction, phase, source, gamepadIndex };
  for (const l of listeners) l(event);
}
