// Browser → Rust name mappings for the rebind UI.
//
// The Rust side stores bindings using device_query::Keycode variant names
// (e.g. "Up", "Z", "RShift", "Key0") and gilrs::Button variant names
// (e.g. "DPadUp", "East", "Start"). The browser emits its own naming:
// KeyboardEvent.code values like "ArrowUp" / "KeyZ" / "ShiftRight", and
// Gamepad button indices follow the Standard Gamepad layout. This module
// translates both directions and provides display-friendly labels.

const BROWSER_CODE_TO_KEYCODE: Record<string, string> = {
  // Arrows
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  // Letters
  KeyA: "A", KeyB: "B", KeyC: "C", KeyD: "D", KeyE: "E", KeyF: "F",
  KeyG: "G", KeyH: "H", KeyI: "I", KeyJ: "J", KeyK: "K", KeyL: "L",
  KeyM: "M", KeyN: "N", KeyO: "O", KeyP: "P", KeyQ: "Q", KeyR: "R",
  KeyS: "S", KeyT: "T", KeyU: "U", KeyV: "V", KeyW: "W", KeyX: "X",
  KeyY: "Y", KeyZ: "Z",
  // Digits
  Digit0: "Key0", Digit1: "Key1", Digit2: "Key2", Digit3: "Key3", Digit4: "Key4",
  Digit5: "Key5", Digit6: "Key6", Digit7: "Key7", Digit8: "Key8", Digit9: "Key9",
  // Numpad
  Numpad0: "Numpad0", Numpad1: "Numpad1", Numpad2: "Numpad2", Numpad3: "Numpad3",
  Numpad4: "Numpad4", Numpad5: "Numpad5", Numpad6: "Numpad6", Numpad7: "Numpad7",
  Numpad8: "Numpad8", Numpad9: "Numpad9",
  NumpadAdd: "NumpadAdd",
  NumpadSubtract: "NumpadSubtract",
  NumpadMultiply: "NumpadMultiply",
  NumpadDivide: "NumpadDivide",
  NumpadDecimal: "NumpadDecimal",
  NumpadEnter: "NumpadEnter",
  NumpadEqual: "NumpadEquals",
  // Function
  F1: "F1", F2: "F2", F3: "F3", F4: "F4", F5: "F5", F6: "F6",
  F7: "F7", F8: "F8", F9: "F9", F10: "F10", F11: "F11", F12: "F12",
  // Modifiers
  ShiftLeft: "LShift", ShiftRight: "RShift",
  ControlLeft: "LControl", ControlRight: "RControl",
  AltLeft: "LAlt", AltRight: "RAlt",
  MetaLeft: "LMeta", MetaRight: "RMeta",
  CapsLock: "CapsLock",
  // Navigation
  Home: "Home", End: "End", PageUp: "PageUp", PageDown: "PageDown",
  Insert: "Insert", Delete: "Delete",
  // Other
  Enter: "Enter",
  Space: "Space",
  Tab: "Tab",
  Backspace: "Backspace",
  Minus: "Minus",
  Equal: "Equal",
  BracketLeft: "LeftBracket",
  BracketRight: "RightBracket",
  Backslash: "BackSlash",
  Semicolon: "Semicolon",
  Quote: "Apostrophe",
  Comma: "Comma",
  Period: "Dot",
  Slash: "Slash",
  Backquote: "Grave",
};

/// Browser KeyboardEvent.code → Rust Keycode variant name. Returns null for
/// keys we don't surface (consumer keys, IME, etc.).
export function eventCodeToRustKey(code: string): string | null {
  return BROWSER_CODE_TO_KEYCODE[code] ?? null;
}

const KEY_LABELS: Record<string, string> = {
  Up: "↑", Down: "↓", Left: "←", Right: "→",
  Enter: "Enter", Space: "Space", Tab: "Tab", Backspace: "Backspace",
  Escape: "Esc", CapsLock: "Caps", Insert: "Ins", Delete: "Del",
  Home: "Home", End: "End", PageUp: "PgUp", PageDown: "PgDn",
  LShift: "L-Shift", RShift: "R-Shift",
  LControl: "L-Ctrl", RControl: "R-Ctrl",
  LAlt: "L-Alt", RAlt: "R-Alt",
  LMeta: "L-Meta", RMeta: "R-Meta",
  Minus: "-", Equal: "=", LeftBracket: "[", RightBracket: "]",
  BackSlash: "\\", Semicolon: ";", Apostrophe: "'",
  Comma: ",", Dot: ".", Slash: "/", Grave: "`",
  NumpadAdd: "Num +", NumpadSubtract: "Num −", NumpadMultiply: "Num ×",
  NumpadDivide: "Num ÷", NumpadDecimal: "Num .",
  NumpadEnter: "Num ⏎", NumpadEquals: "Num =",
};

/// Display label for a stored Rust Keycode name. Falls back to the raw name
/// for letters/digits/function keys where the name itself is the label.
export function formatKey(name: string | null | undefined): string {
  if (!name) return "—";
  if (KEY_LABELS[name]) return KEY_LABELS[name];
  if (/^Key[0-9]$/.test(name)) return name.slice(3);
  if (/^Numpad[0-9]$/.test(name)) return `Num ${name.slice(6)}`;
  return name;
}

// Standard Gamepad layout (https://w3c.github.io/gamepad/#dfn-standard-gamepad)
// button-index → gilrs::Button variant name.
const STANDARD_GAMEPAD_INDEX_TO_BUTTON: Record<number, string> = {
  0: "South",
  1: "East",
  2: "West",
  3: "North",
  4: "LeftTrigger",
  5: "RightTrigger",
  6: "LeftTrigger2",
  7: "RightTrigger2",
  8: "Select",
  9: "Start",
  10: "LeftThumb",
  11: "RightThumb",
  12: "DPadUp",
  13: "DPadDown",
  14: "DPadLeft",
  15: "DPadRight",
  16: "Mode",
};

export function gamepadIndexToRustButton(index: number): string | null {
  return STANDARD_GAMEPAD_INDEX_TO_BUTTON[index] ?? null;
}

const PAD_LABELS: Record<string, string> = {
  South: "South (A)",
  East: "East (B)",
  West: "West (X)",
  North: "North (Y)",
  LeftTrigger: "LB",
  RightTrigger: "RB",
  LeftTrigger2: "LT",
  RightTrigger2: "RT",
  Select: "Select",
  Start: "Start",
  Mode: "Guide",
  LeftThumb: "L3",
  RightThumb: "R3",
  DPadUp: "D-Pad ↑",
  DPadDown: "D-Pad ↓",
  DPadLeft: "D-Pad ←",
  DPadRight: "D-Pad →",
  C: "C",
  Z: "Z",
};

export function formatGamepadButton(name: string | null | undefined): string {
  if (!name) return "—";
  return PAD_LABELS[name] ?? name;
}

/// Poll navigator.getGamepads() once and return the first pressed Standard
/// Gamepad button index, or null. Used by the rebind UI's capture loop.
export function firstPressedGamepadIndex(): number | null {
  const pads = (navigator.getGamepads?.() ?? []) as (Gamepad | null)[];
  for (const pad of pads) {
    if (!pad) continue;
    for (let i = 0; i < pad.buttons.length; i++) {
      if (pad.buttons[i]?.pressed) return i;
    }
  }
  return null;
}
