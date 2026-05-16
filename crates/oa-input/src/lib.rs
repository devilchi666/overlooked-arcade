//! oa-input — keyboard + gamepad polling, mapped to `oa_core::InputState`.
//!
//! Cross-platform global-keyboard polling via `device_query` plus gamepad polling
//! via `gilrs`. Two flat 32-bit-per-port mapping tables (`KeyboardMapping`,
//! `GamepadMapping`) keep the layer system-agnostic — `oa-input` doesn't know
//! what the bits mean for any particular system. The shell (or a future settings
//! UI) configures bindings using its core wrapper's button constants
//! (e.g. `oa_pce::buttons::I`).
//!
//! Gamepads bind to ports in connection order: the first pad seen takes Port0,
//! the second Port1, and so on up to Port4. Disconnects free the slot for the
//! next plug-in. Keyboard and gamepad bits are OR-ed together each frame.
//!
//! Global keyboard polling means presses in other windows are seen too. The
//! shell gates polling on game-window focus via `set_enabled` to keep that
//! contained.

#![deny(rust_2018_idioms)]

use oa_core::{InputState, PortIndex};

pub use device_query::Keycode;
pub use gilrs::Button as GamepadButton;

use device_query::{DeviceQuery, DeviceState};
use gilrs::{Event, EventType, GamepadId, Gilrs};

/// Per-port keyboard mapping. Index = bit position in `InputState::buttons`
/// (0..32). `None` means "no key mapped to this bit".
#[derive(Clone)]
pub struct KeyboardMapping {
    map: [[Option<Keycode>; 32]; 5],
}

impl KeyboardMapping {
    /// Empty map — no keys bound on any port.
    pub fn empty() -> Self {
        Self { map: [[None; 32]; 5] }
    }

    /// Bind a key to a specific button bit on a specific port.
    ///
    /// `button_mask` is a single-bit value (e.g. `oa_pce::buttons::I`); only
    /// the lowest set bit is honoured.
    pub fn bind(&mut self, port: PortIndex, button_mask: u32, key: Keycode) {
        if button_mask == 0 {
            return;
        }
        let bit = button_mask.trailing_zeros() as usize;
        if bit >= 32 {
            return;
        }
        self.map[port as usize][bit] = Some(key);
    }
}

impl Default for KeyboardMapping {
    fn default() -> Self {
        Self::empty()
    }
}

/// Per-port gamepad mapping. Index = bit position in `InputState::buttons`
/// (0..32). `None` means "no button mapped to this bit".
#[derive(Clone)]
pub struct GamepadMapping {
    map: [[Option<GamepadButton>; 32]; 5],
}

impl GamepadMapping {
    /// Empty map — no pad buttons bound on any port.
    pub fn empty() -> Self {
        Self { map: [[None; 32]; 5] }
    }

    /// Bind a pad button to a specific button bit on a specific port.
    pub fn bind(&mut self, port: PortIndex, button_mask: u32, button: GamepadButton) {
        if button_mask == 0 {
            return;
        }
        let bit = button_mask.trailing_zeros() as usize;
        if bit >= 32 {
            return;
        }
        self.map[port as usize][bit] = Some(button);
    }
}

impl Default for GamepadMapping {
    fn default() -> Self {
        Self::empty()
    }
}

/// Polls keyboard + gamepad state each frame and returns a populated `InputState`.
pub struct InputPoller {
    device_state: DeviceState,
    keyboard: KeyboardMapping,
    gamepad: GamepadMapping,
    gilrs: Option<Gilrs>,
    /// Connection-order port assignment. `port_pads[i] == Some(id)` means the
    /// pad with that gilrs id drives `PortIndex::Port{i}`.
    port_pads: [Option<GamepadId>; 5],
    /// When false, `poll()` returns zeroed state regardless of input. Gilrs
    /// events are still pumped so connection state stays current.
    enabled: bool,
}

impl InputPoller {
    /// Build a poller with keyboard mapping only (gamepad map starts empty).
    pub fn new(keyboard: KeyboardMapping) -> Self {
        Self::with_mappings(keyboard, GamepadMapping::empty())
    }

    /// Build a poller with both keyboard and gamepad mappings.
    pub fn with_mappings(keyboard: KeyboardMapping, gamepad: GamepadMapping) -> Self {
        let gilrs = match Gilrs::new() {
            Ok(g) => Some(g),
            Err(e) => {
                log::warn!("oa-input: gilrs init failed ({e:?}); gamepad disabled");
                None
            }
        };

        let mut poller = Self {
            device_state: DeviceState::new(),
            keyboard,
            gamepad,
            gilrs,
            port_pads: [None; 5],
            enabled: true,
        };

        // Snapshot already-connected pads so they get a port without waiting
        // for a Connected event (gilrs only reports events for new state).
        if let Some(g) = poller.gilrs.as_ref() {
            let ids: Vec<GamepadId> = g.gamepads().map(|(id, _)| id).collect();
            for id in ids {
                poller.assign_pad(id);
            }
        }

        poller
    }

    /// Enable or disable polling — call this when the game window gains or
    /// loses focus to keep keystrokes/pad presses from leaking through.
    pub fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
    }

    /// Immediate-mode keyboard check, bypassing the enabled gate. The shell
    /// uses this for hotkeys (save state, pause, etc.) that should fire even
    /// when game-input is gated off (e.g. menu visible).
    pub fn is_pressed(&self, key: Keycode) -> bool {
        self.device_state.get_keys().contains(&key)
    }

    /// Read the current input state for a given port.
    pub fn poll(&mut self, port: PortIndex) -> InputState {
        // Always pump gilrs so connect/disconnect tracking stays live even
        // when the game window is unfocused.
        self.pump_gilrs_events();

        if !self.enabled {
            return InputState::default();
        }
        let port_idx = port as usize;
        if port_idx >= 5 {
            return InputState::default();
        }

        let mut bits: u32 = 0;

        let pressed = self.device_state.get_keys();
        for (bit, slot) in self.keyboard.map[port_idx].iter().enumerate() {
            if let Some(kc) = slot {
                if pressed.contains(kc) {
                    bits |= 1 << bit;
                }
            }
        }

        if let (Some(gilrs), Some(pad_id)) = (self.gilrs.as_ref(), self.port_pads[port_idx]) {
            let pad = gilrs.gamepad(pad_id);
            for (bit, slot) in self.gamepad.map[port_idx].iter().enumerate() {
                if let Some(btn) = slot {
                    if pad.is_pressed(*btn) {
                        bits |= 1 << bit;
                    }
                }
            }
        }

        InputState { buttons: bits, axes: [0; 4] }
    }

    fn pump_gilrs_events(&mut self) {
        let mut connects: Vec<GamepadId> = Vec::new();
        let mut disconnects: Vec<GamepadId> = Vec::new();
        if let Some(g) = self.gilrs.as_mut() {
            while let Some(Event { id, event, .. }) = g.next_event() {
                match event {
                    EventType::Connected => connects.push(id),
                    EventType::Disconnected => disconnects.push(id),
                    _ => {}
                }
            }
        }
        for id in connects {
            self.assign_pad(id);
        }
        for id in disconnects {
            self.release_pad(id);
        }
    }

    fn assign_pad(&mut self, id: GamepadId) {
        if self.port_pads.iter().any(|p| *p == Some(id)) {
            return;
        }
        let mut assigned = None;
        for (idx, slot) in self.port_pads.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(id);
                assigned = Some(idx);
                break;
            }
        }
        if let Some(idx) = assigned {
            log::info!("oa-input: gamepad {id:?} assigned to port {idx}");
        } else {
            log::warn!("oa-input: gamepad {id:?} connected but all 5 ports are taken");
        }
    }

    fn release_pad(&mut self, id: GamepadId) {
        for (idx, slot) in self.port_pads.iter_mut().enumerate() {
            if *slot == Some(id) {
                *slot = None;
                log::info!("oa-input: gamepad {id:?} released from port {idx}");
            }
        }
    }
}
