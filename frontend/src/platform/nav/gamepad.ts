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

import { createSignal, onCleanup } from "solid-js";
import { deriveDeviceIdentity } from "./deviceKey";
import { resolveLayout, layoutSource, type ResolvedLayout, type LayoutSource } from "./controllerProfiles";
import { resolveFamily, type ControllerFamily } from "./controllerFamily";
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

/// The standard-layout fallback used for pads the browser canonicalized
/// (mapping === "standard") or pads with no curated profile. Non-standard
/// pads with a controllers.json profile get a remapped layout instead — that
/// is the Phase-2 normalization (see ./controllerProfiles).
const DEFAULT_LAYOUT: ResolvedLayout = {
  buttons: BUTTON_NAMES,
  dpad: DPAD_DIRS,
  hatAxis: null,
};

/// A pad's resolved identity + the effective layout the poller is actually
/// using for it. Powers the Settings → Controllers test window so it shows
/// EXACTLY what nav sees (no duplicated mapping logic). Pure — safe to call
/// each frame from a diagnostic UI.
export type PadDiagnostic = {
  index: number;
  id: string;
  deviceKey: string;
  vid: string | null;
  pid: string | null;
  name: string;
  /** Web Gamepad mapping: "standard" (browser-canonical) or "" (raw). */
  mapping: string;
  /** True when a curated/bulk profile remapped this (non-standard) pad. */
  profiled: boolean;
  /** Which layer supplied the layout: curated override, SDL DB, or none. */
  profileSource: LayoutSource;
  /** The effective layout (profile remap, or the standard fallback). */
  layout: ResolvedLayout;
  /** Label family (Nintendo/Xbox/PlayStation/generic) for button labels. */
  family: ControllerFamily;
  buttonCount: number;
  axisCount: number;
};

export function describePad(pad: Gamepad): PadDiagnostic {
  const identity = deriveDeviceIdentity(pad.id);
  const resolved = resolveLayout(identity.key, pad.mapping);
  return {
    index: pad.index,
    id: pad.id,
    deviceKey: identity.key,
    vid: identity.vid,
    pid: identity.pid,
    name: identity.name,
    mapping: pad.mapping,
    profiled: resolved !== null,
    profileSource: layoutSource(identity.key, pad.mapping),
    layout: resolved ?? DEFAULT_LAYOUT,
    family: resolveFamily(identity.key, identity.name),
    buttonCount: pad.buttons.length,
    axisCount: pad.axes.length,
  };
}

const INITIAL_REPEAT_MS = 400;
const REPEAT_INTERVAL_MS = 80;
const STICK_DEADZONE = 0.4;

let inputEnabled = true;

/// Master enable / disable. Settings calls this when the operator
/// flips the "Controller navigation" toggle. False suppresses all
/// emit() calls — listeners stay subscribed but receive nothing.
export function setNavEnabled(on: boolean): void {
  inputEnabled = on;
}

/// Current enable state. The keyboard nav path in `./focus` reads this so
/// gamepad + keyboard share one gate (the shell nav layer is suppressed while
/// a game is running / a non-navigable surface is up).
export function isNavEnabled(): boolean {
  return inputEnabled;
}

type ButtonState = { pressedAt: number; lastRepeatAt: number };
type StickState = {
  direction: NavDirection | null;
  enteredAt: number;
  lastRepeatAt: number;
};
type HatState = {
  direction: NavDirection | null;
  enteredAt: number;
  lastRepeatAt: number;
};

// HID HAT switch buckets — the 8 discrete fractional values an
// analog axis emits when the controller reports its DPad as a
// "POV hat" instead of buttons. Encoding `(n - 3.5) / 3.5` for
// n = 0..7 (8 cardinal + diagonal directions). Idle sentinel sits
// outside [-1, 1] — varies per controller (Faceoff Switch reports
// 3.286), but anything beyond ±1.1 is treated as idle.
//
// Diagonals collapse to a cardinal so DPad-press-during-diagonal-mash
// still produces a usable nav event. Choice of cardinal for each
// diagonal: NE→up, SE→right, SW→down, NW→left (the more "first half"
// of the diagonal).
const HAT_BUCKETS: { value: number; dir: NavDirection }[] = [
  { value: -1.000, dir: "up" },     // N
  { value: -0.714, dir: "up" },     // NE → up
  { value: -0.428, dir: "right" },  // E
  { value: -0.143, dir: "right" },  // SE → right
  { value:  0.143, dir: "down" },   // S
  { value:  0.428, dir: "down" },   // SW → down
  { value:  0.714, dir: "left" },   // W
  { value:  1.000, dir: "left" },   // NW → left
];

export function decodeHat(v: number): NavDirection | null {
  // Idle sentinel — anything outside the [-1, 1] range can't be a
  // valid HAT value, so we treat it as "no direction".
  if (Math.abs(v) > 1.1) return null;
  let bestDelta = Infinity;
  let bestDir: NavDirection | null = null;
  for (const b of HAT_BUCKETS) {
    const d = Math.abs(b.value - v);
    if (d < bestDelta) {
      bestDelta = d;
      bestDir = b.dir;
    }
  }
  // Tolerance: HAT values are spaced ~0.286 apart, so 0.1 keeps us
  // safely on the nearest bucket without ambiguity.
  return bestDelta < 0.1 ? bestDir : null;
}

type Listener = (event: NavEvent) => void;

const listeners = new Set<Listener>();
// padIdx -> stable device-key (derived once from gamepad.id). Phase-1
// identity: lets every emitted NavEvent carry the controller identity
// alongside its volatile gamepad index.
const padDeviceKeys = new Map<number, string>(); // padIdx -> device-key
// padIdx -> resolved button/axis layout. Default standard layout unless a
// controllers.json profile remaps a non-standard pad (Phase-2 normalization).
const padLayouts = new Map<number, ResolvedLayout>(); // padIdx -> layout
const buttonStates = new Map<string, ButtonState>(); // `${padIdx}:${btnIdx}`
const stickStates = new Map<number, StickState>(); // padIdx -> state
const hatStates = new Map<string, HatState>(); // `${padIdx}:${axisIdx}` -> state
// Set of (padIdx, axisIdx) we've detected as HAT axes — populated on
// the first tick after connect by spotting axes with an idle value
// outside [-1, 1] (real stick axes never produce out-of-range values;
// HID HAT switches use sentinels like 3.286 / -3.0 / etc. for "idle").
const hatAxes = new Map<number, Set<number>>(); // padIdx -> axisIdxs
const hatAxesDetected = new Set<number>(); // padIdx — true after first detection pass
let connectedPads = 0;
let rafHandle: number | null = null;
let started = false;
const [sessionEverSawGamepadSig, setSessionEverSawGamepad] = createSignal(false);

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

/// Has any gamepad been seen this session? Reactive Solid accessor —
/// the hint bar derives from this to decide whether to render at all
/// (auto-hide if no controller has ever appeared).
export const hasSeenGamepad = sessionEverSawGamepadSig;

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
      setSessionEverSawGamepad(true);
      const key = deriveDeviceIdentity(pad.id).key;
      padDeviceKeys.set(pad.index, key);
      padLayouts.set(pad.index, resolveLayout(key, pad.mapping) ?? DEFAULT_LAYOUT);
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
  padDeviceKeys.clear();
  padLayouts.clear();
  connectedPads = 0;
}

function handleConnect(e: GamepadEvent): void {
  connectedPads++;
  setSessionEverSawGamepad(true);
  // Diagnostic: log the pad's identity + mapping + raw button / axis
  // counts on connect so we can spot non-standard layouts (DPad on
  // axes instead of buttons, etc.). Once.
  // Phase-0 identity spike: derive the cross-layer device-key from the
  // Web `id` string so it can be cross-checked against the Rust gilrs
  // poller's `oa-input: identity device-key=…` line for the same pad.
  const identity = deriveDeviceIdentity(e.gamepad.id);
  padDeviceKeys.set(e.gamepad.index, identity.key);
  const layout = resolveLayout(identity.key, e.gamepad.mapping);
  padLayouts.set(e.gamepad.index, layout ?? DEFAULT_LAYOUT);
  console.log(
    "[oa-gamepad] connected",
    JSON.stringify({
      id: e.gamepad.id,
      deviceKey: identity.key,
      vid: identity.vid,
      pid: identity.pid,
      name: identity.name,
      mapping: e.gamepad.mapping,
      // Phase-2: did a controllers.json profile remap this (non-standard) pad?
      profiled: layout !== null,
      buttons: e.gamepad.buttons.length,
      axes: e.gamepad.axes.length,
      index: e.gamepad.index,
    }),
  );
  if (rafHandle === null) {
    rafHandle = requestAnimationFrame(tick);
  }
}

function handleDisconnect(e: GamepadEvent): void {
  connectedPads = Math.max(0, connectedPads - 1);
  // Drop state for the disconnected pad so a reconnect on the same
  // index starts clean.
  stickStates.delete(e.gamepad.index);
  hatAxes.delete(e.gamepad.index);
  hatAxesDetected.delete(e.gamepad.index);
  padDeviceKeys.delete(e.gamepad.index);
  padLayouts.delete(e.gamepad.index);
  for (const key of Array.from(buttonStates.keys())) {
    if (key.startsWith(`${e.gamepad.index}:`)) buttonStates.delete(key);
  }
  for (const key of Array.from(hatStates.keys())) {
    if (key.startsWith(`${e.gamepad.index}:`)) hatStates.delete(key);
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
    // Lazily backfill identity + layout for pads that surfaced via
    // getGamepads() without a connect event (some browsers populate the
    // array first).
    if (!padDeviceKeys.has(pad.index)) {
      const key = deriveDeviceIdentity(pad.id).key;
      padDeviceKeys.set(pad.index, key);
      padLayouts.set(pad.index, resolveLayout(key, pad.mapping) ?? DEFAULT_LAYOUT);
    }
    detectHatAxes(pad);
    pollButtons(pad, now);
    pollStick(pad, now);
    pollHat(pad, now);
  }
  if (connectedPads > 0) {
    rafHandle = requestAnimationFrame(tick);
  } else {
    rafHandle = null;
  }
}

/// Identify HAT-style axes on first tick after a pad connects. Any
/// axis beyond the first two (left stick) whose value is outside the
/// [-1, 1] range at detection time is treated as a HAT switch (real
/// stick axes can't produce out-of-range values; HID HAT switches
/// commonly report idle sentinels like 3.286 or similar). One-shot
/// per pad — subsequent ticks don't re-scan.
function detectHatAxes(pad: Gamepad): void {
  if (hatAxesDetected.has(pad.index)) return;
  hatAxesDetected.add(pad.index);
  const set = new Set<number>();
  for (let i = 2; i < pad.axes.length; i++) {
    const v = pad.axes[i] ?? 0;
    if (Math.abs(v) > 1.1) {
      set.add(i);
      console.log(
        `[oa-gamepad] detected HAT axis ${i} on pad ${pad.index} (idle=${v.toFixed(3)})`,
      );
    }
  }
  // Phase-2: honor a profile-declared HAT axis even if the idle-sentinel
  // heuristic above didn't flag it (some pads idle their HAT at 0, in range).
  const profileHat = padLayouts.get(pad.index)?.hatAxis;
  if (profileHat != null) set.add(profileHat);
  if (set.size > 0) hatAxes.set(pad.index, set);
}

/// Read each detected HAT axis, decode to a NavDirection, and emit
/// DPad-source events with the same down/up/repeat semantics as the
/// analog stick. Operators with non-standard controllers (third-party
/// Switch pads, generic USB pads, etc.) gain full DPad support.
function pollHat(pad: Gamepad, now: number): void {
  const axes = hatAxes.get(pad.index);
  if (!axes) return;
  for (const axisIdx of axes) {
    const v = pad.axes[axisIdx] ?? 0;
    const direction = decodeHat(v);
    const key = `${pad.index}:${axisIdx}`;
    let state = hatStates.get(key);
    if (!state) {
      state = { direction: null, enteredAt: 0, lastRepeatAt: 0 };
      hatStates.set(key, state);
    }
    if (direction !== state.direction) {
      if (state.direction !== null) {
        emitDirection(state.direction, "up", "dpad", pad.index);
      }
      if (direction !== null) {
        emitDirection(direction, "down", "dpad", pad.index);
      }
      state.direction = direction;
      state.enteredAt = now;
      state.lastRepeatAt = now;
    } else if (direction !== null) {
      const held = now - state.enteredAt;
      if (held >= INITIAL_REPEAT_MS && now - state.lastRepeatAt >= REPEAT_INTERVAL_MS) {
        state.lastRepeatAt = now;
        emitDirection(direction, "repeat", "dpad", pad.index);
      }
    }
  }
}

function pollButtons(pad: Gamepad, now: number): void {
  // Per-pad layout: standard tables for browser-canonicalized pads, or a
  // controllers.json remap for a profiled non-standard pad (Phase-2 fix).
  const layout = padLayouts.get(pad.index) ?? DEFAULT_LAYOUT;
  for (let i = 0; i < pad.buttons.length; i++) {
    const isPressed = pad.buttons[i]?.pressed ?? false;
    const key = `${pad.index}:${i}`;
    const prev = buttonStates.get(key);
    const buttonName = layout.buttons[i];
    const dpadDir = layout.dpad[i];
    // Diagnostic: log ANY button press at any index, even unmapped
    // ones, so we can spot DPad on non-standard indices. Only logs on
    // the down edge.
    if (isPressed && !prev) {
      console.log(`[oa-gamepad] raw button ${i} pressed`);
    }
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

// Track per-axis last-logged value so we surface every distinct DPad
// or HAT-style axis position at least once but don't spam every frame
// while it idles at the same value. Catches non-standard DPad
// layouts where DPad reports as analog-axis values (typical on
// third-party Switch controllers, HAT-style POV switches, etc.).
const lastLoggedAxis = new Map<string, number>();

function pollStick(pad: Gamepad, now: number): void {
  if (pad.axes.length < 2) return;
  // Diagnostic: log every distinct value on axes beyond the first two
  // (left stick). HAT-style DPads can report a single axis with one
  // of 8 quantised positions (e.g. axis 9 on this controller); other
  // layouts use two axes (6+7). Threshold lowered to 0.1 so even
  // gentle DPad deflections surface, and the "distinct value" gate
  // logs each new position rather than each frame.
  for (let i = 2; i < pad.axes.length; i++) {
    const v = pad.axes[i] ?? 0;
    if (Math.abs(v) <= 0.1) continue;
    const key = `${pad.index}:${i}`;
    const prev = lastLoggedAxis.get(key);
    // Log if we've never logged this axis OR the value changed by > 0.1
    // (so we capture HAT transitions like 1.0 → 0.43 → -0.14 → ...).
    if (prev === undefined || Math.abs(prev - v) > 0.1) {
      lastLoggedAxis.set(key, v);
      console.log(`[oa-gamepad] raw axis ${i} = ${v.toFixed(3)}`);
    }
  }
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
  if (!inputEnabled) return;
  const deviceKey = padDeviceKeys.get(gamepadIndex) ?? "name:unknown";
  const event: NavEvent = { kind: "button", button, phase, gamepadIndex, deviceKey };
  for (const l of listeners) l(event);
}

function emitDirection(
  direction: NavDirection,
  phase: NavPhase,
  source: "dpad" | "stick-left",
  gamepadIndex: number,
): void {
  if (!inputEnabled) return;
  const deviceKey = padDeviceKeys.get(gamepadIndex) ?? "name:unknown";
  const event: NavEvent = { kind: "direction", direction, phase, source, gamepadIndex, deviceKey };
  for (const l of listeners) l(event);
}
