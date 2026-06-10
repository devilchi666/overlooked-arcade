// Typed Tauri bridge — input domain (button bindings / input descriptors /
// controller devices / analog routing / light-gun detection).
//
// Theming Phase 4 Slice 4. The per-system input surface: the keyboard/gamepad
// button-binding table, the per-game input descriptors a core advertises
// (SET_INPUT_DESCRIPTORS), the controller-device options a core advertises per
// port (SET_CONTROLLER_INFO), the analog-stick routing prefs (per-system +
// per-game), and the light-gun capability flag. Same convention as the other
// platform/api modules (see docs/PLANS/theming-platform-api-bridge.md): one
// typed named export per command, thin pass-through, no error handling here,
// command-name string lives ONLY in this file.
//
// `getBindings` / `setBinding` are generic (D14) — the binding rows are read
// with two shapes (the editor carries `libretroId`; the reference cards read a
// subset), so call sites keep their local view via the type arg. The
// analog-routing `routing` blob is generic on `R` so this module needn't pull
// in the component-local analog-prefs type family (the platform↛components
// boundary forbids importing it here anyway).

import { invoke } from "@tauri-apps/api/core";

// --- Backend-contract types this domain owns ----------------------------

/// One button-binding row. The editor's full shape (carries `libretroId`); the
/// reference cards read the first three fields only and pass their subset type.
/// The canonical default for `getBindings` / `setBinding`.
export type ButtonBinding = {
  button: string;
  keyboard: string | null;
  gamepad: string | null;
  /// libretro RETRO_DEVICE_ID_JOYPAD_* bit index, after the per-system
  /// shell-bits→libretro-bits remap. `4294967295` (u32::MAX) = "no remap".
  libretroId: number;
};

/// One per-game button-label tuple a core advertises via
/// SET_INPUT_DESCRIPTORS. Mirror of Rust `oa_core::InputDescriptor`.
export type InputDescriptor = {
  port: number;
  device: number;
  index: number;
  id: number;
  description: string;
};

/// One device a core advertises it supports on a port, from
/// SET_CONTROLLER_INFO. Mirror of Rust `oa_core::ControllerDeviceDescriptor`.
export type ControllerDeviceDescriptor = {
  label: string;
  id: number;
};

/// What analog sticks a system's effective core exposes
/// (`analog_sticks_for_system`).
export type AnalogSticksInfo = {
  kind: "none" | "single" | "dual";
  leftLabel: string | null;
  rightLabel: string | null;
};

/// The capture kind a `set_binding` write targets.
export type BindingKind = "keyboard" | "gamepad";

// --- Button bindings ----------------------------------------------------

/// The button-binding table for a system. Generic (D14): canonical shape is
/// `ButtonBinding`; the reference cards read a subset.
export function getBindings<T = ButtonBinding>(systemId: string): Promise<T[]> {
  return invoke<T[]>("get_bindings", { systemId });
}

/// Set (or clear, with null) one button binding; returns the updated table.
export function setBinding<T = ButtonBinding>(
  systemId: string,
  button: string,
  kind: BindingKind,
  value: string | null,
): Promise<T[]> {
  return invoke<T[]>("set_binding", { systemId, button, kind, value });
}

/// Reset all bindings for a system to its defaults.
export function resetBindings(systemId: string): Promise<void> {
  return invoke("reset_bindings", { systemId });
}

/// The input descriptors the live core published for a system.
export function getInputDescriptors(systemId: string): Promise<InputDescriptor[]> {
  return invoke<InputDescriptor[]>("get_input_descriptors", { systemId });
}

/// Push the per-game controller-device selection to the running emulator at
/// launch.
export function armLibretroDevice(gameId: string): Promise<void> {
  return invoke<void>("arm_libretro_device", { gameId });
}

/// The controller-device options a core advertises for one port.
export function getControllerDevices(
  systemId: string,
  port: number,
): Promise<ControllerDeviceDescriptor[]> {
  return invoke<ControllerDeviceDescriptor[]>("get_controller_devices", { port, systemId });
}

// --- Analog routing -----------------------------------------------------

/// What analog sticks a system exposes.
export function analogSticksForSystem(systemId: string): Promise<AnalogSticksInfo> {
  return invoke<AnalogSticksInfo>("analog_sticks_for_system", { systemId });
}

/// Persist the per-system analog routing for one port. `routing` is generic so
/// callers keep their analog-prefs type (the api layer can't import it).
export function setAnalogRouting<R>(systemId: string, port: number, routing: R): Promise<void> {
  return invoke("set_analog_routing", { systemId, port, routing });
}

/// Persist a per-game analog-routing override for one port.
export function setAnalogRoutingForGame<R>(gameId: string, port: number, routing: R): Promise<void> {
  return invoke("set_analog_routing_for_game", { gameId, port, routing });
}

/// Push the resolved per-game analog routing to the running emulator at launch.
export function armAnalogRouting(gameId: string): Promise<void> {
  return invoke<void>("arm_analog_routing", { gameId });
}

// --- Light gun ----------------------------------------------------------

/// Whether a system's effective core advertises a light-gun device.
export function systemHasLightGun(systemId: string): Promise<boolean> {
  return invoke<boolean>("system_has_light_gun", { systemId });
}
