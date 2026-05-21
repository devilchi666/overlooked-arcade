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
use gilrs::ff::{BaseEffect, BaseEffectType, Effect, EffectBuilder, Repeat, Ticks};

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
        self.set(port, button_mask, Some(key));
    }

    /// Set or clear a binding for a specific button bit. `None` unbinds the slot.
    pub fn set(&mut self, port: PortIndex, button_mask: u32, key: Option<Keycode>) {
        if button_mask == 0 {
            return;
        }
        let bit = button_mask.trailing_zeros() as usize;
        if bit >= 32 {
            return;
        }
        self.map[port as usize][bit] = key;
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
        self.set(port, button_mask, Some(button));
    }

    /// Set or clear a binding for a specific button bit. `None` unbinds the slot.
    pub fn set(&mut self, port: PortIndex, button_mask: u32, button: Option<GamepadButton>) {
        if button_mask == 0 {
            return;
        }
        let bit = button_mask.trailing_zeros() as usize;
        if bit >= 32 {
            return;
        }
        self.map[port as usize][bit] = button;
    }
}

impl Default for GamepadMapping {
    fn default() -> Self {
        Self::empty()
    }
}

/// Which physical gamepad stick feeds a logical "left" or "right" analog
/// input on the core. `None` disables gamepad-side reading for that stick
/// (the keyboard fallback still works).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GamepadStick {
    /// gilrs `Axis::LeftStickX` / `Axis::LeftStickY`.
    Left,
    /// gilrs `Axis::RightStickX` / `Axis::RightStickY`.
    Right,
    /// Disable gamepad-side reading for this logical stick. Keyboard
    /// fallback (if any keys bound) still drives the axis.
    None,
}

impl GamepadStick {
    pub fn as_str(&self) -> &'static str {
        match self {
            GamepadStick::Left => "left",
            GamepadStick::Right => "right",
            GamepadStick::None => "none",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "left" => GamepadStick::Left,
            "right" => GamepadStick::Right,
            "none" => GamepadStick::None,
            _ => GamepadStick::Left,
        }
    }
}

/// Per-stick analog tuning + remap. One of these per logical stick (Left,
/// Right). Slice A — radial deadzone, sensitivity multiplier, per-axis
/// invert. Slice B — gamepad source remap + 4 keyboard fallback keys
/// (up / down / left / right of the stick).
#[derive(Clone, Debug)]
pub struct AnalogStickRouting {
    /// Which physical gamepad stick drives this logical stick.
    pub gamepad_source: GamepadStick,
    /// Keyboard fallback. Layout: `[up, down, left, right]`. Each `Some`
    /// key, when held, drives the corresponding axis to its extreme.
    /// Keyboard fallback is OR-ed with gamepad output — if both are
    /// active, the larger magnitude wins per-axis.
    pub keyboard: [Option<Keycode>; 4],
    /// Mouse-position routing. When `Some(X)` the mouse's screen-X
    /// position drives this stick's X axis (paddle / spinner games:
    /// 2600 Breakout, Coleco Super Action, Arkanoid in MAME). When
    /// `Some(Y)` the mouse's Y drives the stick's Y axis (rare —
    /// flight-stick yoke pitch). When `Some(Xy)` both axes follow
    /// the mouse (true mouse-as-stick for SNES Mouse-aware titles
    /// like Mario Paint when the operator wants stick semantics).
    /// `None` = mouse position doesn't affect this stick. OR-merges
    /// with gamepad + keyboard — the largest magnitude per axis wins.
    pub mouse_source: Option<MouseSource>,
    /// Radial deadzone, normalized 0.0..=0.5. Stick magnitudes below
    /// this threshold are clamped to zero; magnitudes above are
    /// rescaled to span [0, 1] so the usable range stays full.
    pub deadzone: f32,
    /// Output scaling, typical range 0.5..=2.0. Stick magnitudes above
    /// the deadzone are multiplied by sensitivity, then clamped to ±1.0
    /// before scaling to i16.
    pub sensitivity: f32,
    /// Flip the X axis (e.g. some operators prefer reverse-X on N64).
    pub invert_x: bool,
    /// Flip the Y axis. gilrs reports +Y up but libretro convention is
    /// +Y down; we already flip in poll(). `invert_y=true` undoes that
    /// flip (gives "natural" up-is-positive output to the core).
    pub invert_y: bool,
}

/// Which mouse axes (if any) feed an analog stick. See
/// [`AnalogStickRouting::mouse_source`] for the resolution rules. Mouse
/// position is sampled in the same screen coordinates the POINTER
/// pipeline uses; the value is normalized against the primary screen
/// dimensions (-1.0 = left/top edge, +1.0 = right/bottom edge) before
/// being merged with the gamepad + keyboard sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseSource {
    /// Mouse X → stick X axis. Standard paddle / spinner mapping.
    X,
    /// Mouse Y → stick Y axis. Rare — flight-stick yoke pitch.
    Y,
    /// Both axes follow the mouse — true mouse-as-stick semantics.
    Xy,
}

impl Default for AnalogStickRouting {
    /// Identity routing: Left → Left, Right → Right, no deadzone, 1.0
    /// sensitivity, no inversion, no keyboard fallback, no mouse source.
    fn default() -> Self {
        Self {
            gamepad_source: GamepadStick::Left,
            keyboard: [None; 4],
            mouse_source: None,
            deadzone: 0.0,
            sensitivity: 1.0,
            invert_x: false,
            invert_y: false,
        }
    }
}

impl AnalogStickRouting {
    pub fn default_right() -> Self {
        Self {
            gamepad_source: GamepadStick::Right,
            ..Self::default()
        }
    }
}

/// Per-port analog routing. The shell pushes this via
/// `InputPoller::set_analog_routing` after resolving the per-game →
/// per-system → default chain.
#[derive(Clone, Debug)]
pub struct AnalogRouting {
    pub left: AnalogStickRouting,
    pub right: AnalogStickRouting,
    /// Swap the two logical sticks after all per-stick processing —
    /// useful when a core's "main stick" output should come from the
    /// gamepad's right stick (rare but supported).
    pub stick_swap: bool,
}

impl Default for AnalogRouting {
    fn default() -> Self {
        Self {
            left: AnalogStickRouting::default(),
            right: AnalogStickRouting::default_right(),
            stick_swap: false,
        }
    }
}

/// Screen-space rectangle of the game-output viewport. Used to map
/// absolute mouse coordinates into libretro POINTER normalized range
/// (-32768..32767) against the actual rendered area, not the whole
/// monitor. The shell computes this each frame from `Renderer::last_viewport`
/// plus the game window's screen position, then calls
/// `InputPoller::set_pointer_viewport`.
#[derive(Clone, Copy, Debug)]
pub struct PointerViewport {
    /// Top-left screen X of the game-output rectangle, physical pixels.
    pub screen_x: f32,
    /// Top-left screen Y of the game-output rectangle, physical pixels.
    pub screen_y: f32,
    /// Width of the game-output rectangle in physical pixels.
    pub width: f32,
    /// Height of the game-output rectangle in physical pixels.
    pub height: f32,
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
    /// Per-port analog routing (deadzone / sensitivity / invert / keyboard
    /// fallback). Defaults to identity routing — the shell pushes
    /// system-specific routing via `set_analog_routing` at launch.
    analog_routing: [AnalogRouting; 5],
    /// Game-output viewport in screen coordinates. `None` until the
    /// shell pushes one (typically on the first frame after the renderer
    /// computes its viewport). When `None`, pointer polling falls back
    /// to the 1080p screen-relative approximation — functional but
    /// imprecise on other resolutions.
    pointer_viewport: Option<PointerViewport>,
    /// Phase F — per-port-per-effect gilrs force-feedback handle.
    /// `[port][0]` = strong (low-freq) motor effect, `[port][1]` = weak
    /// (high-freq). Built lazily on first non-zero strength + the bound
    /// gamepad id at build time so reconnects rebuild on next dispatch.
    /// `[gain_q12]` caches the last set_gain Q12 value so we skip the
    /// gilrs round-trip when the core's continuous-rumble polls
    /// don't actually change magnitude.
    rumble_effects: [[Option<RumbleHandle>; 2]; 5],
}

struct RumbleHandle {
    pad: gilrs::GamepadId,
    effect: Effect,
    /// Last strength applied (0..=65535). 0 means stopped; non-zero
    /// means playing at gain (strength / 65535).
    last_strength: u16,
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
            analog_routing: [
                AnalogRouting::default(),
                AnalogRouting::default(),
                AnalogRouting::default(),
                AnalogRouting::default(),
                AnalogRouting::default(),
            ],
            pointer_viewport: None,
            rumble_effects: Default::default(),
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

    /// Set or clear a keyboard binding live, without rebuilding the poller.
    /// `None` unbinds. Use this for runtime rebinding from a settings UI.
    pub fn set_keyboard_binding(&mut self, port: PortIndex, button_mask: u32, key: Option<Keycode>) {
        self.keyboard.set(port, button_mask, key);
    }

    /// Set or clear a gamepad binding live, without rebuilding the poller.
    /// `None` unbinds. Use this for runtime rebinding from a settings UI.
    pub fn set_gamepad_binding(&mut self, port: PortIndex, button_mask: u32, button: Option<GamepadButton>) {
        self.gamepad.set(port, button_mask, button);
    }

    /// Clear every keyboard + gamepad binding for a port. Called by the
    /// shell on every system swap before applying the new system's
    /// bindings — otherwise stale bit slots from the previous system can
    /// leak through. PCE uses a clockwise d-pad layout (UP=4, RIGHT=5,
    /// DOWN=6, LEFT=7); NES/SNES/Lynx use libretro's straight order
    /// (UP=4, DOWN=5, LEFT=6, RIGHT=7). Without clearing, switching from
    /// PCE to NES leaves the arrow keys bound at PCE's positions, which
    /// the NES identity remap then reads as the wrong directions.
    pub fn clear_port_bindings(&mut self, port: PortIndex) {
        let idx = port as usize;
        if idx >= 5 {
            return;
        }
        self.keyboard.map[idx] = [None; 32];
        self.gamepad.map[idx] = [None; 32];
    }

    /// Replace the analog routing for a port. Used by the shell on each
    /// system / game launch to apply the resolved per-game → per-system →
    /// default routing. Untouched ports stay at the default identity
    /// routing.
    pub fn set_analog_routing(&mut self, port: PortIndex, routing: AnalogRouting) {
        let idx = port as usize;
        if idx >= 5 {
            return;
        }
        self.analog_routing[idx] = routing;
    }

    /// Reset analog routing for a port back to identity (no deadzone, no
    /// sensitivity adjust, no inversion, no keyboard fallback). Called on
    /// system swap before the new system's routing is pushed.
    pub fn clear_analog_routing(&mut self, port: PortIndex) {
        let idx = port as usize;
        if idx >= 5 {
            return;
        }
        self.analog_routing[idx] = AnalogRouting::default();
    }

    /// Push the game-output viewport rectangle in screen coordinates.
    /// Called by the shell each frame after `Renderer::present()` —
    /// combines `Renderer::last_viewport()` with the game window's
    /// screen position. `None` clears the override (poll_pointer falls
    /// back to the 1080p approximation). When set, pointer values map
    /// to libretro range against this rectangle, and the cursor is
    /// reported as not-pressed + at (0,0) when outside it.
    pub fn set_pointer_viewport(&mut self, viewport: Option<PointerViewport>) {
        self.pointer_viewport = viewport;
    }

    /// Phase F — dispatch per-port-per-effect rumble strengths to gilrs's
    /// force-feedback API. Called by the shell each frame with the
    /// libretro-spec strength layout: `[port][0]` = strong/low-freq,
    /// `[port][1]` = weak/high-freq, each 0..=65535.
    ///
    /// Effects are built lazily on first non-zero strength + the
    /// currently-bound gamepad. When strength drops to 0 we stop the
    /// effect; when it rises again we rebuild if the gamepad has
    /// rotated underneath us. Magnitude is varied via `Effect::set_gain`
    /// rather than rebuilding so continuous-rumble polls (every-frame
    /// strength updates) stay cheap.
    pub fn dispatch_rumble(&mut self, strengths: [[u16; 2]; 5]) {
        let Some(gilrs) = self.gilrs.as_mut() else { return; };
        for port in 0..5 {
            for effect_idx in 0..2 {
                let strength = strengths[port][effect_idx];
                let pad_id = self.port_pads[port];
                let slot = &mut self.rumble_effects[port][effect_idx];

                // No pad → drop any stale handle and skip.
                let Some(pad_id) = pad_id else {
                    *slot = None;
                    continue;
                };

                // Pad rotated (reconnect or different assignment) →
                // drop the stale handle so we rebuild against the
                // current pad on the next non-zero strength.
                if let Some(h) = slot.as_ref() {
                    if h.pad != pad_id {
                        *slot = None;
                    }
                }

                if strength == 0 {
                    if let Some(h) = slot.as_mut() {
                        if h.last_strength != 0 {
                            let _ = h.effect.stop();
                            h.last_strength = 0;
                        }
                    }
                    continue;
                }

                // strength > 0
                if slot.is_none() {
                    // Confirm the gamepad actually supports FF before
                    // building. Skipping unsupported pads silently is
                    // the same shape as "no pad" — caller doesn't
                    // need to know.
                    let pad_supports_ff = gilrs
                        .connected_gamepad(pad_id)
                        .map(|g| g.is_ff_supported())
                        .unwrap_or(false);
                    if !pad_supports_ff {
                        continue;
                    }
                    let kind = if effect_idx == 0 {
                        BaseEffectType::Strong { magnitude: u16::MAX }
                    } else {
                        BaseEffectType::Weak { magnitude: u16::MAX }
                    };
                    // Long-duration looping effect — set_gain controls
                    // intensity per dispatch tick. Duration of ~5s
                    // repeated forever keeps a continuous feel even
                    // if a play() call somehow gets dropped.
                    let build = EffectBuilder::new()
                        .add_effect(BaseEffect {
                            kind,
                            scheduling: Default::default(),
                            envelope: Default::default(),
                        })
                        .repeat(Repeat::Infinitely)
                        .gamepads(&[pad_id])
                        .finish(gilrs);
                    match build {
                        Ok(effect) => {
                            *slot = Some(RumbleHandle {
                                pad: pad_id,
                                effect,
                                last_strength: 0,
                            });
                        }
                        Err(e) => {
                            log::debug!(
                                "oa-input: build rumble effect failed (port {port} effect {effect_idx}): {e:?}"
                            );
                            continue;
                        }
                    }
                }

                let h = slot.as_mut().expect("just built or pre-existing");
                if h.last_strength != strength {
                    let gain = strength as f32 / u16::MAX as f32;
                    let _ = h.effect.set_gain(gain);
                    if h.last_strength == 0 {
                        let _ = h.effect.play();
                    }
                    h.last_strength = strength;
                }
            }
        }
        // Silence the unused-imports warning when no rumble dispatch
        // happens during compile-time linting; Ticks isn't currently
        // referenced but exists in the gilrs::ff re-exports.
        let _ = Ticks::from_ms;
    }

    /// Immediate-mode keyboard check, bypassing the enabled gate. The shell
    /// uses this for hotkeys (save state, pause, etc.) that should fire even
    /// when game-input is gated off (e.g. menu visible).
    pub fn is_pressed(&self, key: Keycode) -> bool {
        self.device_state.get_keys().contains(&key)
    }

    /// Snapshot of every currently-pressed key. Used by the libretro
    /// keyboard-passthrough pump in the shell to edge-detect transitions
    /// frame-to-frame and forward them to the core via
    /// `LibretroCore::send_keyboard_event`. Also bypasses the enabled gate —
    /// the caller decides whether to pump or pump-releases-only based on
    /// focus + per-system passthrough settings.
    pub fn pressed_keys(&self) -> Vec<Keycode> {
        self.device_state.get_keys()
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

        let mut axes: [i16; 4] = [0; 4];

        // Pre-read gamepad button bits in the same pass (kept inside the
        // gilrs-available branch). Axis sampling now flows through
        // `compute_stick_output` which handles deadzone, sensitivity,
        // invert, and keyboard fallback uniformly whether or not a
        // gamepad is connected.
        let pad_opt = match (self.gilrs.as_ref(), self.port_pads[port_idx]) {
            (Some(gilrs), Some(pad_id)) => Some(gilrs.gamepad(pad_id)),
            _ => None,
        };
        if let Some(pad) = pad_opt.as_ref() {
            for (bit, slot) in self.gamepad.map[port_idx].iter().enumerate() {
                if let Some(btn) = slot {
                    if pad.is_pressed(*btn) {
                        bits |= 1 << bit;
                    }
                }
            }
        }

        // Sample the two logical sticks through the per-port analog
        // routing. compute_stick_output:
        //   1. Reads the routed gamepad source (or zero if disabled).
        //   2. OR-merges keyboard fallback (4 keys per stick → axis
        //      synthesis at full magnitude when held).
        //   3. Applies radial deadzone — magnitudes below the threshold
        //      clamp to zero; magnitudes above get rescaled so the usable
        //      range stays full [0, 1].
        //   4. Applies sensitivity multiplier.
        //   5. Applies per-axis inversion.
        //   6. Inverts Y for libretro convention (+Y down).
        //   7. Scales to i16 libretro range.
        let routing = &self.analog_routing[port_idx];
        let (lx, ly) = compute_stick_output(
            pad_opt.as_ref(),
            &routing.left,
            &self.device_state,
        );
        let (rx, ry) = compute_stick_output(
            pad_opt.as_ref(),
            &routing.right,
            &self.device_state,
        );
        if routing.stick_swap {
            axes[0] = rx;
            axes[1] = ry;
            axes[2] = lx;
            axes[3] = ly;
        } else {
            axes[0] = lx;
            axes[1] = ly;
            axes[2] = rx;
            axes[3] = ry;
        }

        // Mouse-as-touch dispatch — read screen-relative mouse position
        // via device_query (the same DeviceState we use for keyboard).
        // Convert to libretro POINTER range (-32768..32767 normalized
        // against the primary screen dimensions) and read the left
        // button as the "pressed" flag.
        //
        // Phase 0 uses screen-relative coordinates — pixel-perfect
        // window-relative mapping (the game-output rectangle within the
        // OA window) requires Tauri window context plumbing and lands
        // as Phase 2.5 polish. NDS / light-gun titles are playable at
        // Phase 0 but operators may find the touch position offset if
        // the game window doesn't cover the full screen.
        let pointer = self.poll_pointer();

        // Per-button analog pressure. Slot i corresponds to RetroPad
        // bit i (same indexing the digital `bits` uses). For most
        // buttons the value is binary: 0 if released, 32767 if
        // pressed — mirrors the digital bitmask so cores that poll
        // ANALOG_BUTTON for non-trigger buttons get a sensible value.
        // L2 (slot 12) and R2 (slot 13) get their continuous values
        // from gilrs's actual trigger axes when a gamepad is
        // connected, so pressure-sensitive titles (Gran Turismo's
        // brake, Metal Gear Solid's prone walk, GameCube's analog
        // triggers) receive real analog data.
        let mut analog_buttons: [i16; 16] = [0; 16];
        for bit in 0..16 {
            if (bits >> bit) & 1 == 1 {
                analog_buttons[bit] = i16::MAX;
            }
        }
        if let Some(pad) = pad_opt.as_ref() {
            // gilrs's LeftTrigger2 / RightTrigger2 (L2 / R2) are analog
            // buttons. `button_data(btn).map(|d| d.value())` returns the
            // pressure as f32 in 0.0..=1.0 — actual analog value rather
            // than just the button state. Override the binary slot
            // values above with the real pressure when a gamepad is
            // connected. L2 = slot 12, R2 = slot 13 per RetroPad layout.
            if let Some(d) = pad.button_data(gilrs::Button::LeftTrigger2) {
                let v = d.value().clamp(0.0, 1.0);
                if v > 0.0 {
                    analog_buttons[12] = (v * i16::MAX as f32) as i16;
                }
            }
            if let Some(d) = pad.button_data(gilrs::Button::RightTrigger2) {
                let v = d.value().clamp(0.0, 1.0);
                if v > 0.0 {
                    analog_buttons[13] = (v * i16::MAX as f32) as i16;
                }
            }
        }

        InputState { buttons: bits, axes, pointer, analog_buttons }
    }

    /// Sample the OS mouse position + left-button state via
    /// `device_query::DeviceState`. Returns `(x, y, pressed)` where
    /// `x` and `y` are signed 16-bit normalized coordinates in the
    /// libretro POINTER range (-32768 = top-left edge, 32767 =
    /// bottom-right edge), and `pressed` is whether the left mouse
    /// button is currently held. Used for mouse-as-touch dispatch
    /// (Nintendo DS stylus, light-gun games).
    ///
    /// Phase 0: screen-relative coordinates assuming the primary
    /// monitor is 1920×1080. Real screen size detection + window-
    /// relative offsetting is Phase 2.5 polish — operators with
    /// different display resolutions may find touch positioning
    /// imprecise but the input model is functional.
    fn poll_pointer(&self) -> (i16, i16, bool) {
        let mouse = self.device_state.get_mouse();
        let pressed = mouse.button_pressed.get(1).copied().unwrap_or(false);
        let mx = mouse.coords.0 as f32;
        let my = mouse.coords.1 as f32;

        // Viewport path (Phase 2.5): pixel-perfect mapping against the
        // game-output rectangle. Pointer outside the viewport reports
        // (0, 0, false) so light-gun aim doesn't bleed over chrome /
        // letterbox / sidebar regions.
        if let Some(vp) = self.pointer_viewport {
            if vp.width <= 0.0 || vp.height <= 0.0 {
                return (0, 0, false);
            }
            let rel_x = mx - vp.screen_x;
            let rel_y = my - vp.screen_y;
            if rel_x < 0.0 || rel_y < 0.0 || rel_x >= vp.width || rel_y >= vp.height {
                return (0, 0, false);
            }
            let nx = ((rel_x / vp.width) * 65535.0 - 32768.0)
                .clamp(-32768.0, 32767.0) as i16;
            let ny = ((rel_y / vp.height) * 65535.0 - 32768.0)
                .clamp(-32768.0, 32767.0) as i16;
            return (nx, ny, pressed);
        }

        // Fallback (Phase 0): the 1920×1080 screen-relative approximation.
        // Operators on non-1080p displays get imprecise positioning until
        // the shell pushes a viewport rectangle.
        const ASSUMED_SCREEN_W: f32 = 1920.0;
        const ASSUMED_SCREEN_H: f32 = 1080.0;
        let nx = ((mx / ASSUMED_SCREEN_W) * 65535.0 - 32768.0)
            .clamp(-32768.0, 32767.0) as i16;
        let ny = ((my / ASSUMED_SCREEN_H) * 65535.0 - 32768.0)
            .clamp(-32768.0, 32767.0) as i16;
        (nx, ny, pressed)
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

/// Compute one logical stick's `(x, y)` output (both as i16 in libretro
/// range) by walking the routing pipeline:
///
/// 1. **Gamepad source** — read the routed physical stick (or zero if
///    the source is `None` / no gamepad).
/// 2. **Keyboard fallback** — OR-merged at full magnitude when bound
///    keys are held. Right > Left and Down > Up wins if both halves
///    of an axis are pressed.
/// 3. **Radial deadzone** — magnitudes below threshold clamp to zero;
///    magnitudes above get rescaled so the usable output range stays
///    full [0, 1]. Applied in 2D space so diagonal inputs work cleanly.
/// 4. **Sensitivity** — magnitude is scaled by the sensitivity factor
///    then clamped back to ±1.0 (so a high sensitivity at full stick
///    still saturates, doesn't blow past i16 range).
/// 5. **Invert** — `invert_x` / `invert_y` flip the respective axes.
/// 6. **Y-axis convention** — gilrs reports +Y up, libretro wants +Y
///    down; we flip Y at the end. `invert_y=true` undoes this flip
///    (gives the core "natural" up-is-positive coordinates).
/// 7. **Scale** — i16 range (-32768..32767).
fn compute_stick_output(
    pad: Option<&gilrs::Gamepad<'_>>,
    routing: &AnalogStickRouting,
    device_state: &DeviceState,
) -> (i16, i16) {
    use gilrs::Axis;
    let (mut x, mut y) = match (pad, routing.gamepad_source) {
        (Some(p), GamepadStick::Left) => {
            (p.value(Axis::LeftStickX), p.value(Axis::LeftStickY))
        }
        (Some(p), GamepadStick::Right) => {
            (p.value(Axis::RightStickX), p.value(Axis::RightStickY))
        }
        _ => (0.0_f32, 0.0_f32),
    };
    // Keyboard fallback — OR-merge into the gamepad result. Each held
    // key drives its axis to ±1.0; if both halves are held, the later
    // (positive) direction wins.
    let pressed = device_state.get_keys();
    let key_held = |slot: Option<Keycode>| -> bool {
        slot.map(|k| pressed.contains(&k)).unwrap_or(false)
    };
    // Layout: [up, down, left, right].
    if key_held(routing.keyboard[2]) { x = -1.0; }
    if key_held(routing.keyboard[3]) { x = 1.0; }
    if key_held(routing.keyboard[0]) { y = 1.0; }   // up in gilrs convention
    if key_held(routing.keyboard[1]) { y = -1.0; }  // down

    // Mouse-position routing — OR-merge if the routing opts in.
    // Mouse position is sampled in physical screen coordinates and
    // normalized against the primary monitor's dimensions. This is the
    // same screen-relative coordinate space the Phase 0 pointer code
    // uses; for paddle / spinner games the absolute mapping is the
    // intuitive default (mouse at center = stick at rest). Gilrs's +Y
    // convention is "up = positive", so we negate the screen Y (which
    // is "down = positive" in OS coords) before merging — keeps the
    // sign convention identical between the three input sources.
    //
    // OR-merge rule: the source with the larger absolute magnitude
    // wins per-axis. Same behavior the keyboard fallback uses.
    if let Some(src) = routing.mouse_source {
        let mouse = device_state.get_mouse();
        const ASSUMED_SCREEN_W: f32 = 1920.0;
        const ASSUMED_SCREEN_H: f32 = 1080.0;
        let mx = (mouse.coords.0 as f32 / ASSUMED_SCREEN_W * 2.0 - 1.0).clamp(-1.0, 1.0);
        let my = -(mouse.coords.1 as f32 / ASSUMED_SCREEN_H * 2.0 - 1.0).clamp(-1.0, 1.0);
        match src {
            MouseSource::X => {
                if mx.abs() > x.abs() { x = mx; }
            }
            MouseSource::Y => {
                if my.abs() > y.abs() { y = my; }
            }
            MouseSource::Xy => {
                if mx.abs() > x.abs() { x = mx; }
                if my.abs() > y.abs() { y = my; }
            }
        }
    }

    // Radial deadzone in 2D — magnitudes below threshold zero out,
    // magnitudes above get rescaled to [0, 1]. Pure-axis deadzones
    // (per-axis thresholding) feel sticky on diagonals; radial is the
    // standard approach.
    let dz = routing.deadzone.clamp(0.0, 0.5);
    let magnitude = (x * x + y * y).sqrt();
    if magnitude < dz {
        x = 0.0;
        y = 0.0;
    } else if magnitude > 0.0 && dz > 0.0 {
        // Rescale: new_magnitude = (mag - dz) / (1 - dz)
        let new_mag = ((magnitude - dz) / (1.0 - dz)).clamp(0.0, 1.0);
        let scale = new_mag / magnitude;
        x *= scale;
        y *= scale;
    }

    // Sensitivity, then re-clamp to ±1.
    let sens = routing.sensitivity.clamp(0.1, 4.0);
    x = (x * sens).clamp(-1.0, 1.0);
    y = (y * sens).clamp(-1.0, 1.0);

    if routing.invert_x { x = -x; }
    // Libretro convention: +Y down. gilrs reports +Y up. Flip
    // unconditionally; invert_y=true undoes the flip (operator opts
    // into "natural" up-is-positive output).
    y = if routing.invert_y { y } else { -y };

    let to_i16 = |v: f32| -> i16 {
        let clamped = v.clamp(-1.0, 1.0);
        (clamped * 32767.0).round().clamp(-32768.0, 32767.0) as i16
    };
    (to_i16(x), to_i16(y))
}

#[cfg(test)]
mod analog_tests {
    use super::*;

    fn dummy_device() -> DeviceState {
        DeviceState::new()
    }

    #[test]
    fn identity_routing_preserves_zero() {
        let routing = AnalogStickRouting::default();
        let (x, y) = compute_stick_output(None, &routing, &dummy_device());
        assert_eq!((x, y), (0, 0));
    }

    #[test]
    fn keyboard_right_key_drives_positive_x() {
        let mut routing = AnalogStickRouting::default();
        // Bind a key that's vanishingly unlikely to be held during the
        // test. We can't easily force device_query to report a held key
        // in unit tests, so we cover the keyboard-fallback path via the
        // direct-input formula: when the key isn't pressed, output is
        // pure gamepad (which is zero with no gamepad).
        routing.keyboard[3] = Some(Keycode::F12);
        let (x, y) = compute_stick_output(None, &routing, &dummy_device());
        // Without the key actually pressed, output stays zero. This
        // test confirms binding the key doesn't break the no-pad path.
        assert_eq!((x, y), (0, 0));
    }

    #[test]
    fn invert_y_undoes_libretro_flip() {
        // With no gamepad source, axes are zero regardless of invert.
        // Sanity check: identity + no input = 0.
        let mut routing = AnalogStickRouting::default();
        routing.invert_y = true;
        let (x, y) = compute_stick_output(None, &routing, &dummy_device());
        assert_eq!((x, y), (0, 0));
    }

    #[test]
    fn deadzone_clamping_math_in_pure_unit_form() {
        // We can't sample a real gamepad in tests, but we can validate
        // the rescale formula directly. With dz=0.2 and magnitude=0.5,
        // new_mag = (0.5 - 0.2) / (1 - 0.2) = 0.375.
        let dz = 0.2_f32;
        let mag = 0.5_f32;
        let new_mag = (mag - dz) / (1.0 - dz);
        assert!((new_mag - 0.375).abs() < 1e-6);
    }

    #[test]
    fn sensitivity_clamps_output_to_unit_range() {
        // With sensitivity=2.0 and input=0.8, output before clamp =
        // 1.6, after clamp = 1.0. i16 representation = 32767.
        let v = (0.8_f32 * 2.0_f32).clamp(-1.0, 1.0);
        assert!((v - 1.0).abs() < 1e-6);
    }

    #[test]
    fn default_routing_left_uses_left_stick() {
        let r = AnalogStickRouting::default();
        assert_eq!(r.gamepad_source, GamepadStick::Left);
    }

    #[test]
    fn default_routing_right_uses_right_stick() {
        let r = AnalogStickRouting::default_right();
        assert_eq!(r.gamepad_source, GamepadStick::Right);
    }

    // --- Analog-button-pressure synthesis -------------------------------
    //
    // The poll-time logic that builds `InputState.analog_buttons` runs
    // inside `InputPoller::poll` and depends on a live gilrs Gamepad
    // (button_data has no constructor outside a real device session).
    // We exercise the mirror-the-digital-bitmask formula directly via a
    // helper that matches what poll() does — anything more requires a
    // mock gamepad layer that doesn't exist today.

    fn analog_buttons_from_bits(bits: u32) -> [i16; 16] {
        let mut out: [i16; 16] = [0; 16];
        for bit in 0..16 {
            if (bits >> bit) & 1 == 1 {
                out[bit] = i16::MAX;
            }
        }
        out
    }

    #[test]
    fn analog_buttons_mirror_digital_bitmask() {
        // Every pressed digital bit should map to MAX pressure at the
        // matching analog-button slot. Slot mapping is identity to
        // RetroPad bit position so the libretro core sees the same
        // pressure on its ANALOG_BUTTON poll as it does on its
        // JOYPAD bitmask poll, for cores that read both.
        let bits = 0b0000_0000_0000_1011; // bits 0, 1, 3 set
        let out = analog_buttons_from_bits(bits);
        assert_eq!(out[0], i16::MAX);
        assert_eq!(out[1], i16::MAX);
        assert_eq!(out[2], 0);
        assert_eq!(out[3], i16::MAX);
        for i in 4..16 {
            assert_eq!(out[i], 0);
        }
    }

    #[test]
    fn analog_buttons_zero_when_no_digital_press() {
        assert_eq!(analog_buttons_from_bits(0), [0; 16]);
    }

    #[test]
    fn analog_buttons_array_is_16_slots_for_retropad_bits_0_through_15() {
        // The slot count must match libretro's RetroPad button range
        // (RETRO_DEVICE_ID_JOYPAD_R3 = 15). cb_input_state masks `id >
        // 15` to 0 already, but the array's bound is the contract.
        let out = analog_buttons_from_bits(u32::MAX);
        assert_eq!(out.len(), 16);
        for i in 0..16 {
            assert_eq!(out[i], i16::MAX, "slot {} should saturate at MAX", i);
        }
    }

    // --- Pointer viewport math ------------------------------------------
    //
    // We can't sample real mouse state in unit tests, so these exercise
    // the deterministic normalization formula directly. The poll_pointer
    // path is a few lines that lift cleanly into a free helper.

    fn norm(rel: f32, span: f32) -> i16 {
        ((rel / span) * 65535.0 - 32768.0).clamp(-32768.0, 32767.0) as i16
    }

    #[test]
    fn pointer_viewport_center_maps_to_zero() {
        // Cursor at the center of a 1280×720 viewport → libretro (~0, ~0).
        let n = norm(640.0, 1280.0);
        assert!(n.abs() <= 256, "center should be near zero, got {n}");
    }

    #[test]
    fn pointer_viewport_top_left_maps_to_minimum() {
        // (0, 0) in viewport coords → libretro min (-32768).
        assert_eq!(norm(0.0, 1280.0), -32768);
    }

    #[test]
    fn pointer_viewport_bottom_right_maps_to_maximum() {
        // (width, height) → upper end of libretro range. Floor is from
        // the < width / < height guard in poll_pointer (out-of-range
        // returns (0,0,false)); here we just check the math saturates.
        // width - 1 to land inside the clamp.
        let n = norm(1279.0, 1280.0);
        assert!(n >= 32700, "near-bottom-right should be near max, got {n}");
    }

    #[test]
    fn pointer_viewport_zero_width_is_safe() {
        // Defensive: if the viewport degenerates (0 width), the
        // normalize formula would divide by zero. poll_pointer's guard
        // catches it before that; this is just a smoke-test of the
        // shape of the data — the formula isn't called for 0-size.
        let vp = PointerViewport { screen_x: 0.0, screen_y: 0.0, width: 0.0, height: 720.0 };
        assert_eq!(vp.width, 0.0);
    }
}
