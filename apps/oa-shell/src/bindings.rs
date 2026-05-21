//! Per-system input bindings — disk-backed, runtime-editable.
//!
//! Storage format (one file per system, JSON):
//!   `appDataDir/bindings/<systemId>.json`
//!
//! ```json
//! {
//!   "UP":     { "keyboard": "Up",       "gamepad": "DPadUp" },
//!   "DOWN":   { "keyboard": "Down",     "gamepad": "DPadDown" },
//!   "I":      { "keyboard": "Z",        "gamepad": "East" },
//!   ...
//! }
//! ```
//!
//! Key names are device_query::Keycode variant names (e.g. "Up", "Z", "RShift");
//! gamepad names are gilrs::Button variant names (e.g. "DPadUp", "East",
//! "Start"). `null` for either field means the slot is unbound.
//!
//! When the file is missing or malformed, the system's compiled-in defaults are
//! returned instead. The defaults function per system lives next to its core
//! integration — for now there's only `default_pce_bindings()`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use oa_input::{GamepadButton, Keycode};
use serde::{Deserialize, Serialize};

/// PCE button bit positions used by the shell's input binding layer. These
/// were previously imported from `oa_pce::buttons`, but with the libretro
/// pivot the static `oa-pce` crate is retired and the constants live here.
/// Bits are a shell-internal layout — `pce_to_libretro_bits` translates them
/// to the `RETRO_DEVICE_ID_JOYPAD_*` layout the libretro core expects.
pub mod pce {
    pub const I: u32      = 1 << 0;
    pub const II: u32     = 1 << 1;
    pub const SELECT: u32 = 1 << 2;
    pub const RUN: u32    = 1 << 3;
    pub const UP: u32     = 1 << 4;
    pub const RIGHT: u32  = 1 << 5;
    pub const DOWN: u32   = 1 << 6;
    pub const LEFT: u32   = 1 << 7;
}

/// Atari Lynx button bit positions. Deliberately laid out to match the
/// `RETRO_DEVICE_ID_JOYPAD_*` positions directly — the Lynx → libretro
/// remap is the identity function, so `lynx_to_libretro_bits` is a no-op
/// pass-through. Lynx had two action buttons (A, B), two top option
/// buttons (Opt1, Opt2), a pause button, and a d-pad — modern Mednafen
/// Lynx maps option-1 to libretro START and option-2 to libretro SELECT,
/// pause to libretro L (the convention RetroArch users see).
pub mod lynx {
    pub const B: u32     = 1 << 0;  // libretro B (primary action)
    pub const SELECT: u32 = 1 << 2; // libretro SELECT — Lynx Option 2
    pub const START: u32  = 1 << 3; // libretro START — Lynx Option 1
    pub const UP: u32    = 1 << 4;
    pub const DOWN: u32  = 1 << 5;
    pub const LEFT: u32  = 1 << 6;
    pub const RIGHT: u32 = 1 << 7;
    pub const A: u32     = 1 << 8;  // libretro A (secondary action)
    pub const PAUSE: u32 = 1 << 10; // libretro L (Lynx's dedicated pause)
}

/// NES / Famicom button bit positions. NES had 8 buttons total (A, B,
/// SELECT, START, 4-way d-pad) and the layout maps cleanly to libretro's
/// standard joypad — identity remap. Same shape pattern as Lynx.
pub mod nes {
    pub const B: u32     = 1 << 0; // libretro B
    pub const SELECT: u32 = 1 << 2;
    pub const START: u32  = 1 << 3;
    pub const UP: u32    = 1 << 4;
    pub const DOWN: u32  = 1 << 5;
    pub const LEFT: u32  = 1 << 6;
    pub const RIGHT: u32 = 1 << 7;
    pub const A: u32     = 1 << 8; // libretro A
}

/// SNES / Super Famicom button bit positions. SNES added two face
/// buttons (X, Y) above the original A/B and two shoulder buttons
/// (L, R). Layout matches libretro's standard joypad directly — identity
/// remap. RetroArch convention: X = libretro X (bit 9), Y = libretro Y
/// (bit 1), L = libretro L (bit 10), R = libretro R (bit 11).
pub mod snes {
    pub const B: u32     = 1 << 0; // libretro B (lower-right diamond)
    pub const Y: u32     = 1 << 1; // libretro Y (lower-left diamond)
    pub const SELECT: u32 = 1 << 2;
    pub const START: u32  = 1 << 3;
    pub const UP: u32    = 1 << 4;
    pub const DOWN: u32  = 1 << 5;
    pub const LEFT: u32  = 1 << 6;
    pub const RIGHT: u32 = 1 << 7;
    pub const A: u32     = 1 << 8; // libretro A (upper-right diamond)
    pub const X: u32     = 1 << 9; // libretro X (upper-left diamond)
    pub const L: u32     = 1 << 10;
    pub const R: u32     = 1 << 11;
}

/// Atari 7800 button bit positions. The Pro-Line joystick had a d-pad
/// (8-way) plus two fire buttons (Button 1 = primary, Button 2 =
/// secondary), with Pause / Select / Reset on the console. The libretro
/// ProSystem core surfaces Pause + Select via the standard
/// `RETRO_DEVICE_ID_JOYPAD_START` / `_SELECT` bits; Reset is operator-
/// side (hardware switch) so the frontend exposes it via the per-tile
/// context menu's Reset action, not via bindings.
///
/// Bits are laid out to match the libretro `RETRO_DEVICE_ID_JOYPAD_*`
/// positions directly so the remap is identity — same pattern as Lynx /
/// NES / SNES / MAME.
pub mod atari7800 {
    pub const B1: u32     = 1 << 0;  // libretro B  — Button 1 (right fire / primary)
    pub const SELECT: u32 = 1 << 2;
    pub const PAUSE: u32  = 1 << 3;  // libretro START → Pause/Start
    pub const UP: u32     = 1 << 4;
    pub const DOWN: u32   = 1 << 5;
    pub const LEFT: u32   = 1 << 6;
    pub const RIGHT: u32  = 1 << 7;
    pub const B2: u32     = 1 << 8;  // libretro A  — Button 2 (left fire / secondary)
}

/// Sega Mega Drive / Genesis button bit positions. 6-button-MD layout
/// (A/B/C + X/Y/Z + Start + Mode + d-pad) — the post-1993 6-button pad
/// that Capcom's MD ports of Street Fighter II shipped support for.
/// Modern Mega Drive cores (ClownMDEmu / Genesis Plus GX / PicoDrive)
/// all announce 6-button by default and the dump-set conventions assume
/// it's available; titles that misbehave with 6-button announce get
/// worked around via the per-game Input override.
///
/// libretro convention for MD across every major core: B → MD B, Y → MD A,
/// A → MD C, X → MD Y, L → MD X, R → MD Z, START → MD Start, SELECT → MD
/// Mode. Bits are laid out to match the libretro `RETRO_DEVICE_ID_JOYPAD_*`
/// positions directly so the remap is identity — same pattern as Lynx /
/// NES / SNES / MAME / Atari 7800.
pub mod genesis {
    pub const B: u32     = 1 << 0;  // libretro B — MD B (primary action; the middle face button)
    pub const A: u32     = 1 << 1;  // libretro Y — MD A (lower-left of 3-button row; tertiary)
    pub const MODE: u32  = 1 << 2;  // libretro SELECT — MD Mode (3/6-button toggle on real HW)
    pub const START: u32 = 1 << 3;  // libretro START — MD Start
    pub const UP: u32    = 1 << 4;
    pub const DOWN: u32  = 1 << 5;
    pub const LEFT: u32  = 1 << 6;
    pub const RIGHT: u32 = 1 << 7;
    pub const C: u32     = 1 << 8;  // libretro A — MD C (lower-right of 3-button row; secondary)
    pub const Y: u32     = 1 << 9;  // libretro X — MD Y (top-middle of 6-button extras)
    pub const X: u32     = 1 << 10; // libretro L — MD X (top-left of 6-button extras)
    pub const Z: u32     = 1 << 11; // libretro R — MD Z (top-right of 6-button extras)
}

/// Nintendo Game Boy + Game Boy Color button bit positions. Both DMG
/// (original Game Boy) and CGB (Game Boy Color) share the same
/// controller layout — 4-way d-pad + A + B + START + SELECT — so a
/// single `gb` module / SystemId covers both. Identical to NES in
/// shape; Gambatte announces this as the standard libretro RetroPad,
/// so the bit layout matches the libretro `RETRO_DEVICE_ID_JOYPAD_*`
/// positions directly — identity remap.
pub mod gb {
    pub const B: u32     = 1 << 0;  // libretro B
    pub const SELECT: u32 = 1 << 2;
    pub const START: u32  = 1 << 3;
    pub const UP: u32    = 1 << 4;
    pub const DOWN: u32  = 1 << 5;
    pub const LEFT: u32  = 1 << 6;
    pub const RIGHT: u32 = 1 << 7;
    pub const A: u32     = 1 << 8;  // libretro A
}

/// GCE Vectrex button bit positions. The Vectrex was a vector-display
/// console (1982-1984) with a built-in CRT and a unique controller —
/// 4-direction joystick (digital despite being analog hardware; most
/// games used it digitally) + 4 face buttons (1, 2, 3, 4) in a
/// horizontal row. The vecx libretro core maps the 4 face buttons to
/// libretro B / Y / X / A respectively, with identity remap.
pub mod vectrex {
    pub const B1: u32 = 1 << 0;  // libretro B — button 1 (leftmost face, primary)
    pub const B3: u32 = 1 << 1;  // libretro Y — button 3
    // bit 2 (SELECT) — unused on Vectrex hardware
    // bit 3 (START) — unused on Vectrex hardware
    pub const UP: u32    = 1 << 4;
    pub const DOWN: u32  = 1 << 5;
    pub const LEFT: u32  = 1 << 6;
    pub const RIGHT: u32 = 1 << 7;
    pub const B2: u32 = 1 << 8;  // libretro A — button 2 (secondary)
    pub const B4: u32 = 1 << 9;  // libretro X — button 4 (rightmost face)
}

/// Nintendo Virtual Boy button bit positions. The VB controller was
/// unique — dual 4-way D-pads (left + right) + A + B + L + R + START +
/// SELECT, designed for stereoscopic 3D gameplay where each hand
/// independently moves a screen-space anchor. Beetle VB exposes the
/// LEFT D-pad via standard libretro D-pad bits; the RIGHT D-pad goes
/// through the right analog stick by default (Phase 2 polish needed to
/// surface it as bindable digital input — that work also depends on
/// shared analog-input infra).
///
/// Phase 0 layout: LEFT D-pad + A + B + L + R + START + SELECT (10
/// buttons). Right D-pad deferred. Mario Clash, Wario Land VB, and
/// Teleroboxer are documented in KNOWN_GAME_BUGS as "playable single-
/// D-pad-only until Phase 2."
pub mod virtualboy {
    pub const B: u32     = 1 << 0;  // libretro B
    pub const SELECT: u32 = 1 << 2;
    pub const START: u32  = 1 << 3;
    pub const UP: u32    = 1 << 4;
    pub const DOWN: u32  = 1 << 5;
    pub const LEFT: u32  = 1 << 6;
    pub const RIGHT: u32 = 1 << 7;
    pub const A: u32     = 1 << 8;  // libretro A
    pub const L: u32     = 1 << 10;
    pub const R: u32     = 1 << 11;
}

/// Bandai WonderSwan / WonderSwan Color button bit positions. The WS
/// hardware has DUAL 4-way D-pads (X-pad and Y-pad, mounted at right
/// angles to support horizontal AND vertical game orientations) + A +
/// B + Start + a hardware Sound button. Beetle WonderSwan handles the
/// X-pad ↔ Y-pad rotation per-game-header transparently — vertical
/// games auto-swap which physical D-pad maps to libretro D-pad — so
/// from the OA bindings layer the controller looks like a single
/// 4-way D-pad + 3 buttons. The hardware Sound button (volume
/// control) doesn't surface as a RetroPad bit; it stays in the
/// per-system Core Options.
pub mod wonderswan {
    pub const B: u32     = 1 << 0;  // libretro B
    pub const START: u32  = 1 << 3;
    pub const UP: u32    = 1 << 4;
    pub const DOWN: u32  = 1 << 5;
    pub const LEFT: u32  = 1 << 6;
    pub const RIGHT: u32 = 1 << 7;
    pub const A: u32     = 1 << 8;  // libretro A
}

/// ColecoVision button bit positions. The Coleco controller had a
/// 4-way joystick + 2 side fire buttons (yellow left, red right) + a
/// 12-key keypad (0-9, *, #). blueMSX libretro maps everything to
/// RetroPad bits — fires on B/A, keypad numbers spread across Y/X,
/// L/R/L2/R2/L3/R3, Start, Select.
///
/// Bits laid out to match libretro `RETRO_DEVICE_ID_JOYPAD_*` positions
/// directly — identity remap.
pub mod coleco {
    pub const L_FIRE: u32 = 1 << 0;  // libretro B  — yellow side button (left)
    pub const KP1: u32    = 1 << 1;  // libretro Y  — keypad 1
    pub const KP0: u32    = 1 << 2;  // libretro SELECT — keypad 0
    pub const KP9: u32    = 1 << 3;  // libretro START  — keypad 9
    pub const UP: u32     = 1 << 4;
    pub const DOWN: u32   = 1 << 5;
    pub const LEFT: u32   = 1 << 6;
    pub const RIGHT: u32  = 1 << 7;
    pub const R_FIRE: u32 = 1 << 8;  // libretro A  — red side button (right)
    pub const KP2: u32    = 1 << 9;  // libretro X  — keypad 2
    pub const KP3: u32    = 1 << 10; // libretro L  — keypad 3
    pub const KP4: u32    = 1 << 11; // libretro R  — keypad 4
    pub const KP5: u32    = 1 << 12; // libretro L2 — keypad 5
    pub const KP6: u32    = 1 << 13; // libretro R2 — keypad 6
    pub const KP7: u32    = 1 << 14; // libretro L3 — keypad 7
    pub const KP8: u32    = 1 << 15; // libretro R3 — keypad 8
}

/// Mattel Intellivision button bit positions. The Intv controller had
/// a 16-direction analog disc (mapped to libretro D-pad as 8-way in
/// Phase 0; full 16-direction is a Phase 2 analog-input dependency) +
/// 4 side action buttons (upper-L, upper-R, lower-L, lower-R) + a
/// 12-key keypad. FreeIntv libretro maps the action buttons to L/R/B/A
/// and keypad numbers spread across the remaining bits.
pub mod intv {
    pub const LOWER_L: u32 = 1 << 0;  // libretro B — lower-left side button
    pub const KP1: u32     = 1 << 1;  // libretro Y — keypad 1
    pub const SELECT: u32  = 1 << 2;  // libretro SELECT — keypad CLEAR
    pub const START: u32   = 1 << 3;  // libretro START — keypad ENTER
    pub const UP: u32      = 1 << 4;
    pub const DOWN: u32    = 1 << 5;
    pub const LEFT: u32    = 1 << 6;
    pub const RIGHT: u32   = 1 << 7;
    pub const LOWER_R: u32 = 1 << 8;  // libretro A — lower-right side button
    pub const KP2: u32     = 1 << 9;  // libretro X — keypad 2
    pub const UPPER_L: u32 = 1 << 10; // libretro L — upper-left side button
    pub const UPPER_R: u32 = 1 << 11; // libretro R — upper-right side button
}

/// Magnavox Odyssey² / Videopac button bit positions. Simplest
/// controller after the 2600 — 4-way joystick + a single ACTION button.
/// The 47-key alphanumeric keyboard for game-specific input goes
/// through libretro RETRO_DEVICE_KEYBOARD, not RetroPad bits, and is
/// handled by OA's keyboard passthrough mechanism (same path MAME uses).
pub mod o2 {
    pub const ACTION: u32 = 1 << 0;  // libretro B — the single action button
    pub const UP: u32     = 1 << 4;
    pub const DOWN: u32   = 1 << 5;
    pub const LEFT: u32   = 1 << 6;
    pub const RIGHT: u32  = 1 << 7;
}

/// Fairchild Channel F button bit positions. The Channel F plunger
/// controller was a multi-axis stick with: push/pull (vertical) + twist
/// (rotational) + plunger button (push-in). FreeChaF libretro maps the
/// 4 directional axes to D-pad and the plunger to libretro A (push-in
/// fire). Console-side TIME (game select), MODE (game mode), HOLD
/// (pause), START buttons map to libretro SELECT / Y / L / START
/// respectively per FreeChaF convention.
pub mod channelf {
    pub const FIRE: u32   = 1 << 0;  // libretro B — plunger push-in
    pub const MODE: u32   = 1 << 1;  // libretro Y — console MODE switch
    pub const TIME: u32   = 1 << 2;  // libretro SELECT — console TIME / Game Select
    pub const START: u32  = 1 << 3;  // libretro START — console START button
    pub const UP: u32     = 1 << 4;  // plunger pull-up
    pub const DOWN: u32   = 1 << 5;  // plunger push-down
    pub const LEFT: u32   = 1 << 6;  // plunger twist-left
    pub const RIGHT: u32  = 1 << 7;  // plunger twist-right
    pub const HOLD: u32   = 1 << 10; // libretro L — console HOLD / pause
}

/// Atari 5200 SuperSystem button bit positions. The 5200's iconic
/// (and notoriously fragile) controller had a self-centering analog
/// joystick + two side fire buttons + a 12-key keypad (0-9, *, #) +
/// START / PAUSE / RESET on the keypad surface. Atari800 libretro maps
/// the joystick to the d-pad (digital fallback at Phase 0; full analog
/// via the shared analog-input infra is Phase 2.5 polish) and the two
/// side buttons to libretro B / A. START / PAUSE / RESET land on
/// libretro START / SELECT / R.
///
/// The 12-key keypad (0-9, *, #) flows through the libretro KEYBOARD
/// device (enabled by default via `system_settings::default_keyboard_passthrough("5200") = true`).
/// Operators press the numeric / symbol keys directly while the game
/// window is focused; Atari800 receives them through `retro_keyboard_event`
/// the same way MSX / MAME wire keyboard input. Required for Missile
/// Command (coord-shooting), RealSports Football (play selection),
/// and the long tail of keypad-using titles.
pub mod atari5200 {
    pub const FIRE1: u32  = 1 << 0;  // libretro B — bottom-side fire
    pub const FIRE2: u32  = 1 << 8;  // libretro A — top-side fire
    pub const SELECT: u32 = 1 << 2;  // libretro SELECT — keypad PAUSE
    pub const START: u32  = 1 << 3;  // libretro START — keypad START
    pub const UP: u32     = 1 << 4;
    pub const DOWN: u32   = 1 << 5;
    pub const LEFT: u32   = 1 << 6;
    pub const RIGHT: u32  = 1 << 7;
    pub const RESET: u32  = 1 << 11; // libretro R — keypad RESET
}

/// Nintendo Pokémon Mini button bit positions. The smallest Nintendo
/// first-party platform — d-pad + A + B + C (power/menu) + the
/// shake sensor. IR is niche enough to skip entirely. PokeMini libretro
/// maps the keys identity-style to RetroPad face buttons; the shake
/// sensor lands on libretro R shoulder (the conventional libretro
/// PokeMini binding — operators can rebind via the per-system Bindings
/// page).
pub mod pokemini {
    pub const A: u32      = 1 << 0;  // libretro B — primary action
    pub const B: u32      = 1 << 8;  // libretro A — secondary action
    pub const C: u32      = 1 << 2;  // libretro SELECT — Power / Menu
    pub const UP: u32     = 1 << 4;
    pub const DOWN: u32   = 1 << 5;
    pub const LEFT: u32   = 1 << 6;
    pub const RIGHT: u32  = 1 << 7;
    /// Shake sensor — Pokémon Pinball Mini paddle force, parts of
    /// Pokémon Party Mini. Lives on libretro R shoulder; operators
    /// without a gamepad can use the keyboard binding (Space by default).
    pub const SHAKE: u32  = 1 << 11; // libretro R
}

/// Atari 2600 / VCS button bit positions. The 2600 controller had a
/// single fire button — by far the simplest controller in OA's
/// lineup. Console hardware switches (Game Select + Game Reset) map
/// to libretro SELECT + START respectively per the Stella convention.
/// Difficulty A/B and Color/B&W switches go through Stella's core
/// options rather than RetroPad bits.
///
/// Bits are laid out to match libretro `RETRO_DEVICE_ID_JOYPAD_*`
/// positions directly — identity remap.
pub mod atari2600 {
    pub const FIRE: u32   = 1 << 0;  // libretro B — the single fire button
    pub const SELECT: u32 = 1 << 2;  // libretro SELECT — Game Select switch
    pub const RESET: u32  = 1 << 3;  // libretro START — Game Reset switch
    pub const UP: u32     = 1 << 4;
    pub const DOWN: u32   = 1 << 5;
    pub const LEFT: u32   = 1 << 6;
    pub const RIGHT: u32  = 1 << 7;
}

/// Nintendo Game Boy Advance button bit positions. The GBA pad extends
/// the Game Boy / NES face layout with two shoulders (L + R) for a
/// 10-button total — D-pad + A + B + L + R + START + SELECT. Identity-
/// mapped to libretro RetroPad bits, same convention as SNES (which
/// also has L/R shoulders).
pub mod gba {
    pub const B: u32     = 1 << 0;  // libretro B (lower-right of GBA face)
    pub const SELECT: u32 = 1 << 2;
    pub const START: u32  = 1 << 3;
    pub const UP: u32    = 1 << 4;
    pub const DOWN: u32  = 1 << 5;
    pub const LEFT: u32  = 1 << 6;
    pub const RIGHT: u32 = 1 << 7;
    pub const A: u32     = 1 << 8;  // libretro A (upper-left of GBA face)
    pub const L: u32     = 1 << 10;
    pub const R: u32     = 1 << 11;
}

/// Sega Saturn button bit positions. The Saturn 6-button face pad has
/// 6 face buttons in a 2x3 grid (bottom row A/B/C, top row X/Y/Z) +
/// L/R shoulder triggers + Start + d-pad. The Saturn 3D Pad's analog
/// stick (used by NiGHTS / Sonic R / Sega Rally) is deferred to Phase 2
/// alongside shared analog-input infra.
///
/// libretro convention for Saturn (Beetle Saturn / Kronos / YabaSanshiro
/// all share the same mapping):
///   B  → Saturn A (primary, bottom-left face)
///   A  → Saturn B (secondary, bottom-middle face)
///   R2 → Saturn C (tertiary, bottom-right face — face button spilled to trigger slot since the diamond only has 4 face slots)
///   Y  → Saturn X (top-left face)
///   X  → Saturn Y (top-middle face)
///   L2 → Saturn Z (top-right face — spilled to trigger slot)
///   L  → Saturn L shoulder
///   R  → Saturn R shoulder
///   START → Saturn START
/// Bits are laid out to match the libretro `RETRO_DEVICE_ID_JOYPAD_*`
/// positions directly — identity remap.
pub mod saturn {
    pub const A: u32     = 1 << 0;   // libretro B  — Saturn A (primary action; bottom-left face)
    pub const X: u32     = 1 << 1;   // libretro Y  — Saturn X (top-left face)
    pub const START: u32 = 1 << 3;   // libretro START — Saturn START
    pub const UP: u32    = 1 << 4;
    pub const DOWN: u32  = 1 << 5;
    pub const LEFT: u32  = 1 << 6;
    pub const RIGHT: u32 = 1 << 7;
    pub const B: u32     = 1 << 8;   // libretro A  — Saturn B (secondary; bottom-middle face)
    pub const Y: u32     = 1 << 9;   // libretro X  — Saturn Y (top-middle face)
    pub const L: u32     = 1 << 10;  // libretro L  — Saturn L shoulder
    pub const R: u32     = 1 << 11;  // libretro R  — Saturn R shoulder
    pub const Z: u32     = 1 << 12;  // libretro L2 — Saturn Z (top-right face; spilled to trigger slot)
    pub const C: u32     = 1 << 13;  // libretro R2 — Saturn C (bottom-right face; spilled to trigger slot)
}

/// Sony PlayStation (PS1) digital DualPad button bit positions. The
/// DualShock's analog sticks (Left/Right) + L3/R3 stick clicks ship as
/// Phase 2 work alongside the shared analog-input infra. Phase 0 layout
/// is the digital DualPad shape — d-pad + 4 face buttons (Triangle/
/// Circle/Cross/Square) + L1/R1 front shoulders + L2/R2 rear triggers +
/// START + SELECT (14 buttons).
///
/// libretro convention for PSX (Beetle PSX HW / Beetle PSX SW /
/// SwanStation all share):
///   B  → Cross    (× — primary action in Western releases)
///   Y  → Square   (□ — left of diamond)
///   A  → Circle   (○ — right of diamond; primary in JP releases — but OA pins primary to libretro B per cross-system rule)
///   X  → Triangle (△ — top of diamond)
///   L  → L1, R → R1, L2 → L2, R2 → R2
///   START → Start, SELECT → Select
/// Bits laid out as libretro RetroPad positions — identity remap.
pub mod psx {
    pub const CROSS: u32    = 1 << 0;   // libretro B      — PSX × (primary action, Western convention)
    pub const SQUARE: u32   = 1 << 1;   // libretro Y      — PSX □ (left of diamond)
    pub const SELECT: u32   = 1 << 2;   // libretro SELECT — PSX Select
    pub const START: u32    = 1 << 3;   // libretro START  — PSX Start
    pub const UP: u32       = 1 << 4;
    pub const DOWN: u32     = 1 << 5;
    pub const LEFT: u32     = 1 << 6;
    pub const RIGHT: u32    = 1 << 7;
    pub const CIRCLE: u32   = 1 << 8;   // libretro A  — PSX ○ (right of diamond; secondary)
    pub const TRIANGLE: u32 = 1 << 9;   // libretro X  — PSX △ (top of diamond)
    pub const L1: u32       = 1 << 10;  // libretro L  — PSX L1 (front-left shoulder)
    pub const R1: u32       = 1 << 11;  // libretro R  — PSX R1 (front-right shoulder)
    pub const L2: u32       = 1 << 12;  // libretro L2 — PSX L2 (rear-left trigger)
    pub const R2: u32       = 1 << 13;  // libretro R2 — PSX R2 (rear-right trigger)
}

/// SNK Neo Geo (AES home + MVS arcade) button bit positions. The Neo
/// Geo's iconic controller has 4 face buttons (A/B/C/D in a horizontal
/// row on AES home, or diamond/square arrangements on the arcade
/// joystick) + d-pad/joystick + START + SELECT (which doubles as COIN
/// in arcade mode). FBNeo's standard libretro mapping:
///   B → Neo Geo A (primary attack)
///   A → Neo Geo B (secondary attack)
///   Y → Neo Geo C (third — kick / heavy attack in fighters)
///   X → Neo Geo D (fourth — heavy kick / special)
///   SELECT → COIN / Select
///   START → Start
/// Bits laid out to match libretro RetroPad positions — identity remap.
///
/// This same module covers Neo Geo CD via the dispatch arms — the CD
/// variant uses the same 4-button controller as the cart AES.
pub mod neogeo {
    pub const A: u32     = 1 << 0;   // libretro B  — Neo Geo A (primary)
    pub const C: u32     = 1 << 1;   // libretro Y  — Neo Geo C (third)
    pub const COIN: u32  = 1 << 2;   // libretro SELECT — Neo Geo Select / Coin
    pub const START: u32 = 1 << 3;   // libretro START
    pub const UP: u32    = 1 << 4;
    pub const DOWN: u32  = 1 << 5;
    pub const LEFT: u32  = 1 << 6;
    pub const RIGHT: u32 = 1 << 7;
    pub const B: u32     = 1 << 8;   // libretro A  — Neo Geo B (secondary)
    pub const D: u32     = 1 << 9;   // libretro X  — Neo Geo D (fourth)
}

/// SNK Neo Geo Pocket / Color (NGP + NGPC) button bit positions. The
/// NGP/NGPC has the simplest controller after the 2600 — d-pad + 2
/// face buttons (A, B) + a single OPTION button (which doubles as
/// pause/menu, like Game Boy's START). The hardware sound/power
/// button doesn't surface as a RetroPad bit.
///
/// Beetle NeoPop maps:
///   B → NGP A (primary action)
///   A → NGP B (secondary action)
///   START → OPTION (pause / menu)
pub mod ngp {
    pub const A: u32      = 1 << 0;   // libretro B  — NGP A (primary)
    pub const OPTION: u32 = 1 << 3;   // libretro START — NGP OPTION button
    pub const UP: u32     = 1 << 4;
    pub const DOWN: u32   = 1 << 5;
    pub const LEFT: u32   = 1 << 6;
    pub const RIGHT: u32  = 1 << 7;
    pub const B: u32      = 1 << 8;   // libretro A  — NGP B (secondary)
}

/// Atari Jaguar button bit positions. The Jaguar Pro Controller has a
/// notoriously complex layout: d-pad + 3 face action buttons (A/B/C
/// in a vertical row) + OPTION + PAUSE (small buttons above the face
/// row) + a 12-key telephone-style keypad (1/2/3, 4/5/6, 7/8/9,
/// */0/#). Many Jaguar titles (Iron Soldier weapon select, AvP
/// inventory/map, Cybermorph radar) use the keypad heavily, so OA
/// surfaces the full 12-key set in the bindings module.
///
/// RetroPad only has 16 bits but Jaguar needs 21 entries (9 core +
/// 12 keypad). The lower 7 keypad keys (KP1-KP7) get the spare
/// RetroPad bits (libretro X / L / R / L2 / R2 / L3 / R3); the upper
/// 5 keys (KP8, KP9, KP_STAR, KP0, KP_HASH) live in shell-reserved
/// high bits — surfaced in the per-system Bindings page so operators
/// can assign keyboard keys, but pad-binding for these 5 is
/// unsupported (Phase 2 polish would need a "secondary pad bit-set"
/// abstraction). Keyboard-passthrough dispatch to the Virtual Jaguar
/// core for the high-bit keypad keys is also Phase 2 work.
pub mod jaguar {
    pub const A: u32       = 1 << 0;   // libretro B  — A button (primary)
    pub const C: u32       = 1 << 1;   // libretro Y  — C button (tertiary)
    pub const PAUSE: u32   = 1 << 2;   // libretro SELECT — Pause
    pub const OPTION: u32  = 1 << 3;   // libretro START — Option
    pub const UP: u32      = 1 << 4;
    pub const DOWN: u32    = 1 << 5;
    pub const LEFT: u32    = 1 << 6;
    pub const RIGHT: u32   = 1 << 7;
    pub const B: u32       = 1 << 8;   // libretro A  — B button (secondary)
    pub const KP1: u32     = 1 << 9;   // libretro X  — keypad 1
    pub const KP2: u32     = 1 << 10;  // libretro L  — keypad 2
    pub const KP3: u32     = 1 << 11;  // libretro R  — keypad 3
    pub const KP4: u32     = 1 << 12;  // libretro L2 — keypad 4
    pub const KP5: u32     = 1 << 13;  // libretro R2 — keypad 5
    pub const KP6: u32     = 1 << 14;  // libretro L3 — keypad 6
    pub const KP7: u32     = 1 << 15;  // libretro R3 — keypad 7
    // Above-RetroPad keypad keys — surfaced in bindings module for the
    // per-system page UI; Phase 2 work wires keyboard-passthrough
    // dispatch to the core for these.
    pub const KP8: u32     = 1 << 16;
    pub const KP9: u32     = 1 << 17;
    pub const KP_STAR: u32 = 1 << 18;
    pub const KP0: u32     = 1 << 19;
    pub const KP_HASH: u32 = 1 << 20;
}

/// 3DO Interactive Multiplayer button bit positions. The 3DO standard
/// controller has 3 face buttons (A/B/C in red/green/yellow), L/R
/// shoulder triggers, P (Play/Pause for in-game), X (Stop), and START.
/// No SELECT — the 3DO controller doesn't have one. Opera (formerly
/// 4DO) libretro maps:
///   B → A, A → B, Y → C, L → L, R → R, L2 → STOP, R2 → PLAY,
///   START → START
/// Bits laid out as libretro RetroPad positions — identity remap.
pub mod threedo {
    pub const A: u32      = 1 << 0;   // libretro B  — A (red face, primary)
    pub const C: u32      = 1 << 1;   // libretro Y  — C (yellow face, tertiary)
    pub const START: u32  = 1 << 3;   // libretro START — START
    pub const UP: u32     = 1 << 4;
    pub const DOWN: u32   = 1 << 5;
    pub const LEFT: u32   = 1 << 6;
    pub const RIGHT: u32  = 1 << 7;
    pub const B: u32      = 1 << 8;   // libretro A  — B (green face, secondary)
    pub const L: u32      = 1 << 10;  // libretro L  — left shoulder
    pub const R: u32      = 1 << 11;  // libretro R  — right shoulder
    pub const STOP: u32   = 1 << 12;  // libretro L2 — X (Stop)
    pub const PLAY: u32   = 1 << 13;  // libretro R2 — P (Play/Pause)
}

/// NEC PC-FX button bit positions. PC-FX uses the PC Engine 6-button
/// pad — same hardware as the post-1993 PCE 6-button controllers, with
/// the same I-VI naming. Beetle PC-FX's libretro mapping:
///   B → I, A → II, L2 → III, R2 → IV, L → V, R → VI
///   START → RUN, SELECT → SELECT
/// Bits laid out as libretro RetroPad positions — identity remap.
/// Different from the existing `pce::*` module (which is 2-button
/// only for TG-16 / PCE-CD) — PCFX gets its own module since the
/// 6-button extras need their own bit slots.
pub mod pcfx {
    pub const I: u32      = 1 << 0;   // libretro B  — PCFX I (primary)
    pub const SELECT: u32 = 1 << 2;   // libretro SELECT — PCFX Select
    pub const RUN: u32    = 1 << 3;   // libretro START — PCFX Run
    pub const UP: u32     = 1 << 4;
    pub const DOWN: u32   = 1 << 5;
    pub const LEFT: u32   = 1 << 6;
    pub const RIGHT: u32  = 1 << 7;
    pub const II: u32     = 1 << 8;   // libretro A  — PCFX II
    pub const V: u32      = 1 << 10;  // libretro L  — PCFX V
    pub const VI: u32     = 1 << 11;  // libretro R  — PCFX VI
    pub const III: u32    = 1 << 12;  // libretro L2 — PCFX III
    pub const IV: u32     = 1 << 13;  // libretro R2 — PCFX IV
}

/// Nintendo 64 button bit positions. 14 digital buttons — d-pad +
/// A/B face + L/R shoulders + Z trigger + START + 4 C-buttons
/// (C-Up/Down/Left/Right). The N64 controller's main analog stick is
/// the primary movement input for almost every game; analog dispatch
/// flows separately through `InputState.axes` (gamepad LeftStick =
/// `axes[0..2]`). Keyboard-only users enable Mupen64Plus-Next's
/// "Map d-pad to analog stick" core option to get full-tilt movement
/// from digital arrow keys.
///
/// libretro convention for N64 (Mupen64Plus-Next / parallel_n64):
///   B → A button (primary), Y → B button (secondary)
///   X → C-Up, A → C-Right, R2 → C-Down, L3 → C-Left
///   L → L, R → R, L2 → Z trigger
///   START → Start
/// Bits laid out as libretro RetroPad positions — identity remap.
pub mod n64 {
    pub const A: u32       = 1 << 0;   // libretro B  — A button (primary)
    pub const B: u32       = 1 << 1;   // libretro Y  — B button (secondary)
    pub const START: u32   = 1 << 3;   // libretro START
    pub const UP: u32      = 1 << 4;
    pub const DOWN: u32    = 1 << 5;
    pub const LEFT: u32    = 1 << 6;
    pub const RIGHT: u32   = 1 << 7;
    pub const C_RIGHT: u32 = 1 << 8;   // libretro A  — C-Right
    pub const C_UP: u32    = 1 << 9;   // libretro X  — C-Up
    pub const L: u32       = 1 << 10;  // libretro L  — L shoulder
    pub const R: u32       = 1 << 11;  // libretro R  — R shoulder
    pub const Z: u32       = 1 << 12;  // libretro L2 — Z trigger (under-controller)
    pub const C_DOWN: u32  = 1 << 13;  // libretro R2 — C-Down
    pub const C_LEFT: u32  = 1 << 14;  // libretro L3 — C-Left
}

/// Nintendo GameCube + Wii button bit positions. 12 digital buttons —
/// d-pad + A/B/X/Y face + L/R shoulders + Z trigger + START. **No
/// digital C-stick** — GC's C-stick is genuinely analog on real
/// hardware and flows exclusively through `InputState.axes` (gamepad
/// RightStick = `axes[2..4]`). Main analog stick = `axes[0..2]`
/// (gamepad LeftStick). Analog L/R triggers exist on real GC pads but
/// the libretro mapping treats them as digital (Dolphin synthesizes
/// the analog values from digital press).
///
/// Wii Remote / Nunchuk / Classic Controller variants are NOT covered
/// by this binding layout — motion-controls + the Nunchuk's separate
/// stick are deferred to Phase 2.5.
///
/// libretro convention (Dolphin):
///   B → A button (primary), Y → B button (secondary)
///   A → X button, X → Y button
///   L → L, R → R, R2 → Z trigger
///   START → Start
/// Bits laid out as libretro RetroPad positions — identity remap.
pub mod gamecube {
    pub const A: u32     = 1 << 0;   // libretro B  — A button (primary)
    pub const B: u32     = 1 << 1;   // libretro Y  — B button (secondary)
    pub const START: u32 = 1 << 3;   // libretro START
    pub const UP: u32    = 1 << 4;
    pub const DOWN: u32  = 1 << 5;
    pub const LEFT: u32  = 1 << 6;
    pub const RIGHT: u32 = 1 << 7;
    pub const X: u32     = 1 << 8;   // libretro A  — X button (east face)
    pub const Y: u32     = 1 << 9;   // libretro X  — Y button (north face)
    pub const L: u32     = 1 << 10;  // libretro L  — L analog trigger (digital fallback)
    pub const R: u32     = 1 << 11;  // libretro R  — R analog trigger (digital fallback)
    pub const Z: u32     = 1 << 13;  // libretro R2 — Z trigger
}

/// Sega Dreamcast button bit positions. 11 digital buttons — d-pad +
/// A/B/X/Y face diamond + L/R analog triggers (digital fallback) +
/// START. No SELECT on the Dreamcast pad. Single analog stick flows
/// through `InputState.axes[0..2]` (gamepad LeftStick) via the
/// cross-cutting analog infra. VMU peripheral (memory card with
/// screen) + light gun (House of the Dead 2, Confidential Mission)
/// deferred to Phase 2.5.
///
/// Flycast libretro mapping:
///   B → A button (south face, primary), A → B (east face, secondary)
///   Y → X (west face), X → Y (north face)
///   L → L analog trigger, R → R analog trigger
///   START → Start
/// Bits laid out as libretro RetroPad positions — identity remap.
pub mod dreamcast {
    pub const A: u32     = 1 << 0;   // libretro B  — A button (south face, primary)
    pub const X: u32     = 1 << 1;   // libretro Y  — X button (west face)
    pub const START: u32 = 1 << 3;   // libretro START
    pub const UP: u32    = 1 << 4;
    pub const DOWN: u32  = 1 << 5;
    pub const LEFT: u32  = 1 << 6;
    pub const RIGHT: u32 = 1 << 7;
    pub const B: u32     = 1 << 8;   // libretro A  — B button (east face, secondary)
    pub const Y: u32     = 1 << 9;   // libretro X  — Y button (north face)
    pub const L: u32     = 1 << 10;  // libretro L  — L analog trigger (digital fallback)
    pub const R: u32     = 1 << 11;  // libretro R  — R analog trigger (digital fallback)
}

/// Sony PlayStation Portable button bit positions. 12 digital buttons
/// — d-pad + 4 face diamond (Triangle/Circle/Cross/Square) + L/R
/// shoulders + START + SELECT. **No L2/R2** (PSP hardware has only
/// L and R triggers; PSP Go added L2/R2 but it's rare hardware).
/// Single analog stick flows via `InputState.axes[0..2]` (gamepad
/// LeftStick). PSP Go's right stick is Phase 2.5 polish.
///
/// PPSSPP libretro mapping (identity remap):
///   B → Cross (×, primary), A → Circle (○, secondary)
///   Y → Square (□), X → Triangle (△)
///   L → L, R → R, START → Start, SELECT → Select
pub mod psp {
    pub const CROSS: u32    = 1 << 0;   // libretro B  — × (primary)
    pub const SQUARE: u32   = 1 << 1;   // libretro Y  — □
    pub const SELECT: u32   = 1 << 2;   // libretro SELECT
    pub const START: u32    = 1 << 3;   // libretro START
    pub const UP: u32       = 1 << 4;
    pub const DOWN: u32     = 1 << 5;
    pub const LEFT: u32     = 1 << 6;
    pub const RIGHT: u32    = 1 << 7;
    pub const CIRCLE: u32   = 1 << 8;   // libretro A  — ○ (secondary)
    pub const TRIANGLE: u32 = 1 << 9;   // libretro X  — △
    pub const L: u32        = 1 << 10;  // libretro L  — L shoulder
    pub const R: u32        = 1 << 11;  // libretro R  — R shoulder
}

/// Sony PlayStation 2 button bit positions. 16 digital buttons —
/// DualShock 2 layout, PSX-shape + L3/R3 stick-click buttons.
/// d-pad + 4 face diamond + L1/R1/L2/R2 + START + SELECT + L3/R3.
/// Dual analog sticks flow via `InputState.axes` (LeftStick → main,
/// RightStick → secondary). Pressure-sensitive face buttons + analog
/// L2/R2 triggers are real DS2 hardware features deferred to Phase
/// 2.5 (same as GameCube's analog L/R).
///
/// LRPS2 / PCSX2 libretro mapping (identity remap):
///   Same as PSX (B → Cross, A → Circle, etc.) + L3/R3 stick clicks.
pub mod ps2 {
    pub const CROSS: u32    = 1 << 0;   // libretro B  — × (primary)
    pub const SQUARE: u32   = 1 << 1;   // libretro Y  — □
    pub const SELECT: u32   = 1 << 2;   // libretro SELECT
    pub const START: u32    = 1 << 3;   // libretro START
    pub const UP: u32       = 1 << 4;
    pub const DOWN: u32     = 1 << 5;
    pub const LEFT: u32     = 1 << 6;
    pub const RIGHT: u32    = 1 << 7;
    pub const CIRCLE: u32   = 1 << 8;   // libretro A  — ○ (secondary)
    pub const TRIANGLE: u32 = 1 << 9;   // libretro X  — △
    pub const L1: u32       = 1 << 10;  // libretro L  — L1 shoulder
    pub const R1: u32       = 1 << 11;  // libretro R  — R1 shoulder
    pub const L2: u32       = 1 << 12;  // libretro L2 — L2 trigger (digital fallback)
    pub const R2: u32       = 1 << 13;  // libretro R2 — R2 trigger (digital fallback)
    pub const L3: u32       = 1 << 14;  // libretro L3 — left stick click
    pub const R3: u32       = 1 << 15;  // libretro R3 — right stick click
}

/// Nintendo DS button bit positions. 12 digital buttons — d-pad + 4
/// face (A/B/X/Y in Nintendo diamond layout: A east, B south, X
/// north, Y west) + L/R shoulders + START + SELECT. **Touch screen**
/// flows through the new RETRO_DEVICE_POINTER dispatch
/// (`InputState.pointer`) — mouse-as-touch at Phase 0. Mic input
/// (Phantom Hourglass / Brain Age / Hotel Dusk voice puzzles) +
/// rumble pak deferred to Phase 2.5.
///
/// melonDS libretro mapping (identity remap):
///   B → B (primary), A → A (secondary, east face on Nintendo diamond)
///   Y → Y, X → X, L → L, R → R, START → Start, SELECT → Select
pub mod nds {
    // Nintendo diamond layout — A is east face (primary in Nintendo
    // convention: confirm, run, spin; matches nes/snes/gb/gba where
    // keyboard Z → A button). B is south face (secondary; jump in
    // Mario per Nintendo handheld muscle memory).
    pub const B: u32      = 1 << 0;   // libretro B  — B button (south face, SECONDARY)
    pub const Y: u32      = 1 << 1;   // libretro Y  — Y button (west face)
    pub const SELECT: u32 = 1 << 2;   // libretro SELECT
    pub const START: u32  = 1 << 3;   // libretro START
    pub const UP: u32     = 1 << 4;
    pub const DOWN: u32   = 1 << 5;
    pub const LEFT: u32   = 1 << 6;
    pub const RIGHT: u32  = 1 << 7;
    pub const A: u32      = 1 << 8;   // libretro A  — A button (east face, PRIMARY)
    pub const X: u32      = 1 << 9;   // libretro X  — X button (north face)
    pub const L: u32      = 1 << 10;  // libretro L  — L shoulder
    pub const R: u32      = 1 << 11;  // libretro R  — R shoulder
}

/// Sega Master System button bit positions. The SMS controller is the
/// simplest in OA's Sega lineup — a single D-pad plus two face buttons
/// (Button 1 + Button 2). Pause lived on the console hardware, not the
/// controller; Genesis Plus GX maps SMS Pause to libretro
/// `RETRO_DEVICE_ID_JOYPAD_START`, so the binding sits on bit 3 here.
/// Bits are laid out to match the libretro `RETRO_DEVICE_ID_JOYPAD_*`
/// positions directly — identity remap, same pattern as Lynx / NES /
/// SNES / MAME / Atari 7800 / Genesis.
pub mod sms {
    pub const B1: u32     = 1 << 0;  // libretro B  — Button 1 (primary)
    pub const PAUSE: u32  = 1 << 3;  // libretro START — SMS Pause (mapped by GPGX)
    pub const UP: u32     = 1 << 4;
    pub const DOWN: u32   = 1 << 5;
    pub const LEFT: u32   = 1 << 6;
    pub const RIGHT: u32  = 1 << 7;
    pub const B2: u32     = 1 << 8;  // libretro A  — Button 2 (secondary)
}

/// Sega Game Gear button bit positions. Same controller shape as SMS —
/// D-pad + Button 1 + Button 2 — but the Game Gear's hardware Start
/// button sits on the unit itself (top-left edge), not the console.
/// Genesis Plus GX maps it to libretro `RETRO_DEVICE_ID_JOYPAD_START`,
/// same as SMS Pause. Identity remap.
pub mod gamegear {
    pub const B1: u32     = 1 << 0;  // libretro B  — Button 1 (primary)
    pub const START: u32  = 1 << 3;  // libretro START — GG Start
    pub const UP: u32     = 1 << 4;
    pub const DOWN: u32   = 1 << 5;
    pub const LEFT: u32   = 1 << 6;
    pub const RIGHT: u32  = 1 << 7;
    pub const B2: u32     = 1 << 8;  // libretro A  — Button 2 (secondary)
}

/// MAME (arcade) button bit positions. 6 face buttons (Capcom / SNK
/// fighter layout — Street Fighter II's punch/kick triplets), plus
/// START + COIN, plus the d-pad / 8-way stick. Buttons map identity-style
/// onto libretro RetroPad bits — same convention RetroArch's MAME core
/// ships with. Coin lands on libretro SELECT (the common operator-mode
/// "insert coin" mapping); P1 Start on libretro START.
///
/// Phase-1.5 extras (`SERVICE`, `MAME_MENU`, `P2_START`, `P2_COIN`) park
/// on the four otherwise-unused RetroPad bits (L2/R2/L3/R3). MAME's
/// libretro core doesn't wire its operator-mode functions to RetroPad
/// bits directly today — Service/Test/Menu are reachable through the
/// `RETRO_DEVICE_KEYBOARD` device (F2 / Tab) and the per-driver TAB menu.
/// Phase 2 keyboard-passthrough work will hook those up; until then
/// the keyboard defaults below are the live path. P2_START / P2_COIN
/// likewise need per-port wiring (port 1 START / SELECT) which is a
/// follow-up — for now they exist as port-0 placeholder bindings so the
/// per-system Bindings UI can register them.
pub mod mame {
    pub const B1: u32        = 1 << 0;  // libretro B  — Button 1 (SF: weak punch)
    pub const B3: u32        = 1 << 1;  // libretro Y  — Button 3 (SF: strong punch)
    pub const COIN: u32      = 1 << 2;  // libretro SELECT — insert coin (P1)
    pub const START: u32     = 1 << 3;  // libretro START — P1 start
    pub const UP: u32        = 1 << 4;
    pub const DOWN: u32      = 1 << 5;
    pub const LEFT: u32      = 1 << 6;
    pub const RIGHT: u32     = 1 << 7;
    pub const B2: u32        = 1 << 8;  // libretro A  — Button 2 (SF: medium punch)
    pub const B4: u32        = 1 << 9;  // libretro X  — Button 4 (SF: weak kick)
    pub const B5: u32        = 1 << 10; // libretro L  — Button 5 (SF: medium kick)
    pub const B6: u32        = 1 << 11; // libretro R  — Button 6 (SF: strong kick)
    pub const P2_COIN: u32   = 1 << 12; // libretro L2 — placeholder; needs port-1 wiring
    pub const P2_START: u32  = 1 << 13; // libretro R2 — placeholder; needs port-1 wiring
    pub const SERVICE: u32   = 1 << 14; // libretro L3 — operator service/test (keyboard F2)
    pub const MAME_MENU: u32 = 1 << 15; // libretro R3 — MAME TAB menu (keyboard Tab)
}

/// A single binding slot. Either field can be `None` to leave that input kind
/// unbound for this button.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BindingPair {
    #[serde(default)]
    pub keyboard: Option<String>,
    #[serde(default)]
    pub gamepad: Option<String>,
}

/// Map of system-button name → bound key/pad pair.
/// Button names are the system's own button identifiers (e.g. "UP", "I", "RUN"
/// for PCE), which the frontend renders verbatim.
pub type Bindings = BTreeMap<String, BindingPair>;

/// PCE button bits in declaration order. Used both for the default-bindings
/// table and as the canonical iteration order when the UI renders rows.
pub const PCE_BUTTONS: &[(&str, u32)] = &[
    ("UP",     pce::UP),
    ("DOWN",   pce::DOWN),
    ("LEFT",   pce::LEFT),
    ("RIGHT",  pce::RIGHT),
    ("I",      pce::I),
    ("II",     pce::II),
    ("RUN",    pce::RUN),
    ("SELECT", pce::SELECT),
];

/// Resolve a system-button name to its PCE bit mask.
pub fn pce_bit_for(button: &str) -> Option<u32> {
    PCE_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Lynx button bits in declaration order. Same shape as `PCE_BUTTONS`.
/// The frontend renders these names verbatim in the per-system bindings
/// editor — keep them human-readable.
pub const LYNX_BUTTONS: &[(&str, u32)] = &[
    ("UP",     lynx::UP),
    ("DOWN",   lynx::DOWN),
    ("LEFT",   lynx::LEFT),
    ("RIGHT",  lynx::RIGHT),
    ("A",      lynx::A),
    ("B",      lynx::B),
    ("OPT1",   lynx::START),  // Lynx Option 1 — libretro START
    ("OPT2",   lynx::SELECT), // Lynx Option 2 — libretro SELECT
    ("PAUSE",  lynx::PAUSE),
];

/// Resolve a system-button name to its Lynx bit mask.
pub fn lynx_bit_for(button: &str) -> Option<u32> {
    LYNX_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// NES button bits in declaration order.
pub const NES_BUTTONS: &[(&str, u32)] = &[
    ("UP",     nes::UP),
    ("DOWN",   nes::DOWN),
    ("LEFT",   nes::LEFT),
    ("RIGHT",  nes::RIGHT),
    ("A",      nes::A),
    ("B",      nes::B),
    ("START",  nes::START),
    ("SELECT", nes::SELECT),
];

/// Resolve a system-button name to its NES bit mask.
pub fn nes_bit_for(button: &str) -> Option<u32> {
    NES_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// SNES button bits in declaration order. 12 total — d-pad + 4 face
/// buttons (A/B/X/Y) + 2 shoulders (L/R) + START + SELECT.
pub const SNES_BUTTONS: &[(&str, u32)] = &[
    ("UP",     snes::UP),
    ("DOWN",   snes::DOWN),
    ("LEFT",   snes::LEFT),
    ("RIGHT",  snes::RIGHT),
    ("A",      snes::A),
    ("B",      snes::B),
    ("X",      snes::X),
    ("Y",      snes::Y),
    ("L",      snes::L),
    ("R",      snes::R),
    ("START",  snes::START),
    ("SELECT", snes::SELECT),
];

/// Resolve a system-button name to its SNES bit mask.
pub fn snes_bit_for(button: &str) -> Option<u32> {
    SNES_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Atari 7800 button bits in declaration order. 8 entries — 4-way
/// d-pad, 2 fire buttons (B1 = primary / Button 1, B2 = secondary /
/// Button 2), Pause, Select.
pub const ATARI7800_BUTTONS: &[(&str, u32)] = &[
    ("UP",     atari7800::UP),
    ("DOWN",   atari7800::DOWN),
    ("LEFT",   atari7800::LEFT),
    ("RIGHT",  atari7800::RIGHT),
    ("B1",     atari7800::B1),
    ("B2",     atari7800::B2),
    ("PAUSE",  atari7800::PAUSE),
    ("SELECT", atari7800::SELECT),
];

/// Resolve a system-button name to its Atari 7800 bit mask.
pub fn atari7800_bit_for(button: &str) -> Option<u32> {
    ATARI7800_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Genesis button bits in declaration order. 12 entries — 4-way d-pad,
/// 6 face buttons (A/B/C lower row + X/Y/Z upper row), START + MODE.
pub const GENESIS_BUTTONS: &[(&str, u32)] = &[
    ("UP",    genesis::UP),
    ("DOWN",  genesis::DOWN),
    ("LEFT",  genesis::LEFT),
    ("RIGHT", genesis::RIGHT),
    ("A",     genesis::A),
    ("B",     genesis::B),
    ("C",     genesis::C),
    ("X",     genesis::X),
    ("Y",     genesis::Y),
    ("Z",     genesis::Z),
    ("START", genesis::START),
    ("MODE",  genesis::MODE),
];

/// Resolve a system-button name to its Genesis bit mask.
pub fn genesis_bit_for(button: &str) -> Option<u32> {
    GENESIS_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Sega Saturn button bits in declaration order. 13 entries — 4-way d-pad,
/// 6 face buttons (A/B/C bottom + X/Y/Z top), L/R shoulders, START.
pub const SATURN_BUTTONS: &[(&str, u32)] = &[
    ("UP",    saturn::UP),
    ("DOWN",  saturn::DOWN),
    ("LEFT",  saturn::LEFT),
    ("RIGHT", saturn::RIGHT),
    ("A",     saturn::A),
    ("B",     saturn::B),
    ("C",     saturn::C),
    ("X",     saturn::X),
    ("Y",     saturn::Y),
    ("Z",     saturn::Z),
    ("L",     saturn::L),
    ("R",     saturn::R),
    ("START", saturn::START),
];

/// Resolve a system-button name to its Saturn bit mask.
pub fn saturn_bit_for(button: &str) -> Option<u32> {
    SATURN_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Sony PlayStation digital DualPad button bits in declaration order.
/// 14 entries — 4-way d-pad, 4 face (TRIANGLE/CIRCLE/CROSS/SQUARE),
/// L1/R1/L2/R2, START, SELECT. DualShock analog sticks + L3/R3 deferred
/// to Phase 2.
pub const PSX_BUTTONS: &[(&str, u32)] = &[
    ("UP",       psx::UP),
    ("DOWN",     psx::DOWN),
    ("LEFT",     psx::LEFT),
    ("RIGHT",    psx::RIGHT),
    ("TRIANGLE", psx::TRIANGLE),
    ("CIRCLE",   psx::CIRCLE),
    ("CROSS",    psx::CROSS),
    ("SQUARE",   psx::SQUARE),
    ("L1",       psx::L1),
    ("R1",       psx::R1),
    ("L2",       psx::L2),
    ("R2",       psx::R2),
    ("START",    psx::START),
    ("SELECT",   psx::SELECT),
];

/// Resolve a system-button name to its PSX bit mask.
pub fn psx_bit_for(button: &str) -> Option<u32> {
    PSX_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// SNK Neo Geo button bits in declaration order. 10 entries — 4-way
/// joystick + 4 face buttons (A/B/C/D) + START + COIN. Shared between
/// the cart-shape `neogeo` slug and the CD-shape `neocd` slug since
/// both use the same 4-button controller.
pub const NEOGEO_BUTTONS: &[(&str, u32)] = &[
    ("UP",    neogeo::UP),
    ("DOWN",  neogeo::DOWN),
    ("LEFT",  neogeo::LEFT),
    ("RIGHT", neogeo::RIGHT),
    ("A",     neogeo::A),
    ("B",     neogeo::B),
    ("C",     neogeo::C),
    ("D",     neogeo::D),
    ("START", neogeo::START),
    ("COIN",  neogeo::COIN),
];

/// Resolve a system-button name to its Neo Geo bit mask.
pub fn neogeo_bit_for(button: &str) -> Option<u32> {
    NEOGEO_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// SNK Neo Geo Pocket / Color button bits in declaration order.
/// 7 entries — 4-way d-pad + A + B + OPTION.
pub const NGP_BUTTONS: &[(&str, u32)] = &[
    ("UP",     ngp::UP),
    ("DOWN",   ngp::DOWN),
    ("LEFT",   ngp::LEFT),
    ("RIGHT",  ngp::RIGHT),
    ("A",      ngp::A),
    ("B",      ngp::B),
    ("OPTION", ngp::OPTION),
];

/// Resolve a system-button name to its NGP bit mask.
pub fn ngp_bit_for(button: &str) -> Option<u32> {
    NGP_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Atari Jaguar button bits in declaration order. 21 entries —
/// 4-way d-pad + A + B + C + OPTION + PAUSE + 12-key keypad
/// (1-9 + * + 0 + #). The upper 5 keypad keys (KP8/KP9/KP_STAR/KP0/
/// KP_HASH) live in shell-reserved high bits above the 16-bit
/// RetroPad range; pad-binding for those is unsupported in Phase 0.
pub const JAGUAR_BUTTONS: &[(&str, u32)] = &[
    ("UP",      jaguar::UP),
    ("DOWN",    jaguar::DOWN),
    ("LEFT",    jaguar::LEFT),
    ("RIGHT",   jaguar::RIGHT),
    ("A",       jaguar::A),
    ("B",       jaguar::B),
    ("C",       jaguar::C),
    ("OPTION",  jaguar::OPTION),
    ("PAUSE",   jaguar::PAUSE),
    ("KP1",     jaguar::KP1),
    ("KP2",     jaguar::KP2),
    ("KP3",     jaguar::KP3),
    ("KP4",     jaguar::KP4),
    ("KP5",     jaguar::KP5),
    ("KP6",     jaguar::KP6),
    ("KP7",     jaguar::KP7),
    ("KP8",     jaguar::KP8),
    ("KP9",     jaguar::KP9),
    ("KP_STAR", jaguar::KP_STAR),
    ("KP0",     jaguar::KP0),
    ("KP_HASH", jaguar::KP_HASH),
];

/// Resolve a system-button name to its Jaguar bit mask.
pub fn jaguar_bit_for(button: &str) -> Option<u32> {
    JAGUAR_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// 3DO button bits in declaration order. 11 entries — 4-way d-pad +
/// 3 face buttons (A/B/C) + L/R shoulders + START + STOP + PLAY.
/// No SELECT on the 3DO standard controller.
pub const THREEDO_BUTTONS: &[(&str, u32)] = &[
    ("UP",    threedo::UP),
    ("DOWN",  threedo::DOWN),
    ("LEFT",  threedo::LEFT),
    ("RIGHT", threedo::RIGHT),
    ("A",     threedo::A),
    ("B",     threedo::B),
    ("C",     threedo::C),
    ("L",     threedo::L),
    ("R",     threedo::R),
    ("START", threedo::START),
    ("STOP",  threedo::STOP),
    ("PLAY",  threedo::PLAY),
];

/// Resolve a system-button name to its 3DO bit mask.
pub fn threedo_bit_for(button: &str) -> Option<u32> {
    THREEDO_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// PC-FX button bits in declaration order. 12 entries — 4-way d-pad +
/// 6-button face (I-VI) + RUN + SELECT.
pub const PCFX_BUTTONS: &[(&str, u32)] = &[
    ("UP",     pcfx::UP),
    ("DOWN",   pcfx::DOWN),
    ("LEFT",   pcfx::LEFT),
    ("RIGHT",  pcfx::RIGHT),
    ("I",      pcfx::I),
    ("II",     pcfx::II),
    ("III",    pcfx::III),
    ("IV",     pcfx::IV),
    ("V",      pcfx::V),
    ("VI",     pcfx::VI),
    ("RUN",    pcfx::RUN),
    ("SELECT", pcfx::SELECT),
];

/// Resolve a system-button name to its PC-FX bit mask.
pub fn pcfx_bit_for(button: &str) -> Option<u32> {
    PCFX_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Nintendo 64 button bits in declaration order. 14 entries — 4-way
/// d-pad + A + B + START + L + R + Z + 4 C-buttons. Main analog
/// stick is NOT in this table; it flows through `InputState.axes`.
pub const N64_BUTTONS: &[(&str, u32)] = &[
    ("UP",      n64::UP),
    ("DOWN",    n64::DOWN),
    ("LEFT",    n64::LEFT),
    ("RIGHT",   n64::RIGHT),
    ("A",       n64::A),
    ("B",       n64::B),
    ("L",       n64::L),
    ("R",       n64::R),
    ("Z",       n64::Z),
    ("C_UP",    n64::C_UP),
    ("C_DOWN",  n64::C_DOWN),
    ("C_LEFT",  n64::C_LEFT),
    ("C_RIGHT", n64::C_RIGHT),
    ("START",   n64::START),
];

/// Resolve a system-button name to its N64 bit mask.
pub fn n64_bit_for(button: &str) -> Option<u32> {
    N64_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Nintendo GameCube button bits in declaration order. 12 entries —
/// 4-way d-pad + 4-face (A/B/X/Y) + L/R shoulders + Z trigger +
/// START. C-stick is analog-only (flows through `InputState.axes`).
pub const GAMECUBE_BUTTONS: &[(&str, u32)] = &[
    ("UP",    gamecube::UP),
    ("DOWN",  gamecube::DOWN),
    ("LEFT",  gamecube::LEFT),
    ("RIGHT", gamecube::RIGHT),
    ("A",     gamecube::A),
    ("B",     gamecube::B),
    ("X",     gamecube::X),
    ("Y",     gamecube::Y),
    ("L",     gamecube::L),
    ("R",     gamecube::R),
    ("Z",     gamecube::Z),
    ("START", gamecube::START),
];

/// Resolve a system-button name to its GameCube bit mask.
pub fn gamecube_bit_for(button: &str) -> Option<u32> {
    GAMECUBE_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Sega Dreamcast button bits in declaration order. 11 entries —
/// 4-way d-pad + 4 face buttons (A/B/X/Y diamond) + L/R analog
/// triggers + START. No SELECT (Dreamcast pad doesn't have one).
/// Single analog stick flows via `InputState.axes[0..2]`.
pub const DREAMCAST_BUTTONS: &[(&str, u32)] = &[
    ("UP",    dreamcast::UP),
    ("DOWN",  dreamcast::DOWN),
    ("LEFT",  dreamcast::LEFT),
    ("RIGHT", dreamcast::RIGHT),
    ("A",     dreamcast::A),
    ("B",     dreamcast::B),
    ("X",     dreamcast::X),
    ("Y",     dreamcast::Y),
    ("L",     dreamcast::L),
    ("R",     dreamcast::R),
    ("START", dreamcast::START),
];

/// Resolve a system-button name to its Dreamcast bit mask.
pub fn dreamcast_bit_for(button: &str) -> Option<u32> {
    DREAMCAST_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Sony PlayStation Portable button bits in declaration order. 12
/// entries — d-pad + 4 face diamond + L/R + START + SELECT.
pub const PSP_BUTTONS: &[(&str, u32)] = &[
    ("UP",       psp::UP),
    ("DOWN",     psp::DOWN),
    ("LEFT",     psp::LEFT),
    ("RIGHT",    psp::RIGHT),
    ("TRIANGLE", psp::TRIANGLE),
    ("CIRCLE",   psp::CIRCLE),
    ("CROSS",    psp::CROSS),
    ("SQUARE",   psp::SQUARE),
    ("L",        psp::L),
    ("R",        psp::R),
    ("START",    psp::START),
    ("SELECT",   psp::SELECT),
];

/// Resolve a system-button name to its PSP bit mask.
pub fn psp_bit_for(button: &str) -> Option<u32> {
    PSP_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Sony PlayStation 2 (DualShock 2) button bits in declaration order.
/// 16 entries — PSX-shape (d-pad + 4 face + L1/R1/L2/R2 + START +
/// SELECT) + L3/R3 stick clicks.
pub const PS2_BUTTONS: &[(&str, u32)] = &[
    ("UP",       ps2::UP),
    ("DOWN",     ps2::DOWN),
    ("LEFT",     ps2::LEFT),
    ("RIGHT",    ps2::RIGHT),
    ("TRIANGLE", ps2::TRIANGLE),
    ("CIRCLE",   ps2::CIRCLE),
    ("CROSS",    ps2::CROSS),
    ("SQUARE",   ps2::SQUARE),
    ("L1",       ps2::L1),
    ("R1",       ps2::R1),
    ("L2",       ps2::L2),
    ("R2",       ps2::R2),
    ("L3",       ps2::L3),
    ("R3",       ps2::R3),
    ("START",    ps2::START),
    ("SELECT",   ps2::SELECT),
];

/// Resolve a system-button name to its PS2 bit mask.
pub fn ps2_bit_for(button: &str) -> Option<u32> {
    PS2_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Nintendo DS button bits in declaration order. 12 entries — d-pad
/// + 4 face (A/B/X/Y Nintendo diamond) + L/R + START + SELECT.
/// Touch screen flows via `InputState.pointer` (not in this table).
pub const NDS_BUTTONS: &[(&str, u32)] = &[
    ("UP",     nds::UP),
    ("DOWN",   nds::DOWN),
    ("LEFT",   nds::LEFT),
    ("RIGHT",  nds::RIGHT),
    ("A",      nds::A),
    ("B",      nds::B),
    ("X",      nds::X),
    ("Y",      nds::Y),
    ("L",      nds::L),
    ("R",      nds::R),
    ("START",  nds::START),
    ("SELECT", nds::SELECT),
];

/// Resolve a system-button name to its NDS bit mask.
pub fn nds_bit_for(button: &str) -> Option<u32> {
    NDS_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Game Boy / Game Boy Color button bits in declaration order. 8
/// entries — 4-way d-pad + A + B + START + SELECT. Same shape as NES.
pub const GB_BUTTONS: &[(&str, u32)] = &[
    ("UP",     gb::UP),
    ("DOWN",   gb::DOWN),
    ("LEFT",   gb::LEFT),
    ("RIGHT",  gb::RIGHT),
    ("A",      gb::A),
    ("B",      gb::B),
    ("START",  gb::START),
    ("SELECT", gb::SELECT),
];

/// Resolve a system-button name to its Game Boy bit mask.
pub fn gb_bit_for(button: &str) -> Option<u32> {
    GB_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// GCE Vectrex button bits in declaration order. 8 entries — D-pad +
/// 4 face buttons (B1 leftmost primary, B2 secondary, B3 tertiary,
/// B4 rightmost). The B1/B2/B3/B4 labels match the operator-facing
/// convention; the libretro B/A/Y/X bit assignment is internal.
pub const VECTREX_BUTTONS: &[(&str, u32)] = &[
    ("UP",    vectrex::UP),
    ("DOWN",  vectrex::DOWN),
    ("LEFT",  vectrex::LEFT),
    ("RIGHT", vectrex::RIGHT),
    ("B1",    vectrex::B1),
    ("B2",    vectrex::B2),
    ("B3",    vectrex::B3),
    ("B4",    vectrex::B4),
];

/// Resolve a system-button name to its Vectrex bit mask.
pub fn vectrex_bit_for(button: &str) -> Option<u32> {
    VECTREX_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Virtual Boy button bits in declaration order. 10 entries — LEFT
/// D-pad + A + B + L + R + START + SELECT. RIGHT D-pad is Phase 2
/// polish (deferred, see module docs).
pub const VIRTUALBOY_BUTTONS: &[(&str, u32)] = &[
    ("UP",     virtualboy::UP),
    ("DOWN",   virtualboy::DOWN),
    ("LEFT",   virtualboy::LEFT),
    ("RIGHT",  virtualboy::RIGHT),
    ("A",      virtualboy::A),
    ("B",      virtualboy::B),
    ("L",      virtualboy::L),
    ("R",      virtualboy::R),
    ("START",  virtualboy::START),
    ("SELECT", virtualboy::SELECT),
];

/// Resolve a system-button name to its Virtual Boy bit mask.
pub fn virtualboy_bit_for(button: &str) -> Option<u32> {
    VIRTUALBOY_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// WonderSwan button bits in declaration order. 7 entries — D-pad +
/// A + B + START. The dual-physical-D-pad (X and Y) rotation is
/// handled by Beetle WonderSwan per game-header orientation flag.
pub const WONDERSWAN_BUTTONS: &[(&str, u32)] = &[
    ("UP",    wonderswan::UP),
    ("DOWN",  wonderswan::DOWN),
    ("LEFT",  wonderswan::LEFT),
    ("RIGHT", wonderswan::RIGHT),
    ("A",     wonderswan::A),
    ("B",     wonderswan::B),
    ("START", wonderswan::START),
];

/// Resolve a system-button name to its WonderSwan bit mask.
pub fn wonderswan_bit_for(button: &str) -> Option<u32> {
    WONDERSWAN_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// ColecoVision button bits in declaration order. 16 entries — D-pad +
/// 2 fires + 10 keypad numbers. Operator-facing labels match the Coleco
/// hardware ("L_FIRE" / "R_FIRE" for the yellow/red side buttons;
/// "KP0"..."KP9" for the numeric keypad). *, # keypad keys + per-game
/// mappings go through Stella's core options surface.
pub const COLECO_BUTTONS: &[(&str, u32)] = &[
    ("UP",     coleco::UP),
    ("DOWN",   coleco::DOWN),
    ("LEFT",   coleco::LEFT),
    ("RIGHT",  coleco::RIGHT),
    ("L_FIRE", coleco::L_FIRE),
    ("R_FIRE", coleco::R_FIRE),
    ("KP0",    coleco::KP0),
    ("KP1",    coleco::KP1),
    ("KP2",    coleco::KP2),
    ("KP3",    coleco::KP3),
    ("KP4",    coleco::KP4),
    ("KP5",    coleco::KP5),
    ("KP6",    coleco::KP6),
    ("KP7",    coleco::KP7),
    ("KP8",    coleco::KP8),
    ("KP9",    coleco::KP9),
];

/// Resolve a system-button name to its ColecoVision bit mask.
pub fn coleco_bit_for(button: &str) -> Option<u32> {
    COLECO_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Mattel Intellivision button bits in declaration order. 10 entries —
/// D-pad (disc as 8-way) + 4 side action buttons (UPPER_L / UPPER_R /
/// LOWER_L / LOWER_R) + SELECT (keypad CLEAR) + START (keypad ENTER).
/// The remaining 10 keypad numbers (KP1-KP9 + KP0) are a Phase 2 polish
/// item; full keypad coverage needs the same per-game core-options
/// surface ColecoVision's keypad uses.
pub const INTV_BUTTONS: &[(&str, u32)] = &[
    ("UP",      intv::UP),
    ("DOWN",    intv::DOWN),
    ("LEFT",    intv::LEFT),
    ("RIGHT",   intv::RIGHT),
    ("UPPER_L", intv::UPPER_L),
    ("UPPER_R", intv::UPPER_R),
    ("LOWER_L", intv::LOWER_L),
    ("LOWER_R", intv::LOWER_R),
    ("START",   intv::START),
    ("SELECT",  intv::SELECT),
];

/// Resolve a system-button name to its Intellivision bit mask.
pub fn intv_bit_for(button: &str) -> Option<u32> {
    INTV_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Magnavox Odyssey² / Videopac button bits in declaration order. 5
/// entries — 4-way joystick + the single ACTION button. The 47-key
/// alphanumeric keyboard for game-specific input doesn't appear here;
/// it routes through libretro RETRO_DEVICE_KEYBOARD via OA's keyboard
/// passthrough.
pub const O2_BUTTONS: &[(&str, u32)] = &[
    ("UP",     o2::UP),
    ("DOWN",   o2::DOWN),
    ("LEFT",   o2::LEFT),
    ("RIGHT",  o2::RIGHT),
    ("ACTION", o2::ACTION),
];

/// Resolve a system-button name to its Odyssey² bit mask.
pub fn o2_bit_for(button: &str) -> Option<u32> {
    O2_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Fairchild Channel F button bits in declaration order. 9 entries —
/// 4-axis plunger (mapped to D-pad) + FIRE (plunger push-in) + console
/// buttons MODE / TIME / START / HOLD.
pub const CHANNELF_BUTTONS: &[(&str, u32)] = &[
    ("UP",    channelf::UP),
    ("DOWN",  channelf::DOWN),
    ("LEFT",  channelf::LEFT),
    ("RIGHT", channelf::RIGHT),
    ("FIRE",  channelf::FIRE),
    ("MODE",  channelf::MODE),
    ("TIME",  channelf::TIME),
    ("START", channelf::START),
    ("HOLD",  channelf::HOLD),
];

/// Resolve a system-button name to its Channel F bit mask.
pub fn channelf_bit_for(button: &str) -> Option<u32> {
    CHANNELF_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Atari 2600 button bits in declaration order. 7 entries — 4-way
/// d-pad + FIRE + SELECT + RESET. Single-button system; no secondary
/// face button.
pub const ATARI2600_BUTTONS: &[(&str, u32)] = &[
    ("UP",     atari2600::UP),
    ("DOWN",   atari2600::DOWN),
    ("LEFT",   atari2600::LEFT),
    ("RIGHT",  atari2600::RIGHT),
    ("FIRE",   atari2600::FIRE),
    ("SELECT", atari2600::SELECT),
    ("RESET",  atari2600::RESET),
];

/// Resolve a system-button name to its Atari 2600 bit mask.
pub fn atari2600_bit_for(button: &str) -> Option<u32> {
    ATARI2600_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Atari 5200 button bits in declaration order. 9 entries — d-pad
/// (digital fallback for the analog stick) + 2 fire buttons + SELECT
/// (PAUSE) + START + RESET. 12-key keypad lives behind libretro
/// KEYBOARD device (Phase 2 polish).
pub const ATARI5200_BUTTONS: &[(&str, u32)] = &[
    ("UP",     atari5200::UP),
    ("DOWN",   atari5200::DOWN),
    ("LEFT",   atari5200::LEFT),
    ("RIGHT",  atari5200::RIGHT),
    ("FIRE1",  atari5200::FIRE1),
    ("FIRE2",  atari5200::FIRE2),
    ("START",  atari5200::START),
    ("SELECT", atari5200::SELECT),
    ("RESET",  atari5200::RESET),
];

/// Resolve a system-button name to its Atari 5200 bit mask.
pub fn atari5200_bit_for(button: &str) -> Option<u32> {
    ATARI5200_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Pokémon Mini button bits in declaration order. 8 entries — d-pad +
/// A + B + C + SHAKE. Smallest face-button set in OA's lineup; the
/// shake sensor is the one quirky input the platform has beyond the
/// face buttons.
pub const POKEMINI_BUTTONS: &[(&str, u32)] = &[
    ("UP",    pokemini::UP),
    ("DOWN",  pokemini::DOWN),
    ("LEFT",  pokemini::LEFT),
    ("RIGHT", pokemini::RIGHT),
    ("A",     pokemini::A),
    ("B",     pokemini::B),
    ("C",     pokemini::C),
    ("SHAKE", pokemini::SHAKE),
];

/// Resolve a system-button name to its Pokémon Mini bit mask.
pub fn pokemini_bit_for(button: &str) -> Option<u32> {
    POKEMINI_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Game Boy Advance button bits in declaration order. 10 entries —
/// 4-way d-pad + A + B + L + R + START + SELECT.
pub const GBA_BUTTONS: &[(&str, u32)] = &[
    ("UP",     gba::UP),
    ("DOWN",   gba::DOWN),
    ("LEFT",   gba::LEFT),
    ("RIGHT",  gba::RIGHT),
    ("A",      gba::A),
    ("B",      gba::B),
    ("L",      gba::L),
    ("R",      gba::R),
    ("START",  gba::START),
    ("SELECT", gba::SELECT),
];

/// Resolve a system-button name to its Game Boy Advance bit mask.
pub fn gba_bit_for(button: &str) -> Option<u32> {
    GBA_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Sega Master System button bits in declaration order. 7 entries —
/// 4-way d-pad + Button 1 + Button 2 + Pause. The label "PAUSE" tracks
/// the SMS hardware (a button on the console rather than the pad); it
/// binds to libretro START per the Genesis Plus GX convention.
pub const SMS_BUTTONS: &[(&str, u32)] = &[
    ("UP",    sms::UP),
    ("DOWN",  sms::DOWN),
    ("LEFT",  sms::LEFT),
    ("RIGHT", sms::RIGHT),
    ("B1",    sms::B1),
    ("B2",    sms::B2),
    ("PAUSE", sms::PAUSE),
];

/// Resolve a system-button name to its SMS bit mask.
pub fn sms_bit_for(button: &str) -> Option<u32> {
    SMS_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Sega Game Gear button bits in declaration order. 7 entries — same
/// shape as SMS, but the operator-facing label is "START" because the
/// Game Gear's start button lives on the unit itself, not the console.
pub const GAMEGEAR_BUTTONS: &[(&str, u32)] = &[
    ("UP",    gamegear::UP),
    ("DOWN",  gamegear::DOWN),
    ("LEFT",  gamegear::LEFT),
    ("RIGHT", gamegear::RIGHT),
    ("B1",    gamegear::B1),
    ("B2",    gamegear::B2),
    ("START", gamegear::START),
];

/// Resolve a system-button name to its Game Gear bit mask.
pub fn gamegear_bit_for(button: &str) -> Option<u32> {
    GAMEGEAR_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// MAME button bits in declaration order. 16 entries — 4-way d-pad,
/// 6 face buttons (B1-B6 in the SF-fighter convention), P1 START + COIN,
/// and four Phase-1.5 system buttons (P2_START / P2_COIN / SERVICE /
/// MAME_MENU) parked on otherwise-free libretro RetroPad bits.
pub const MAME_BUTTONS: &[(&str, u32)] = &[
    ("UP",        mame::UP),
    ("DOWN",      mame::DOWN),
    ("LEFT",      mame::LEFT),
    ("RIGHT",     mame::RIGHT),
    ("B1",        mame::B1),
    ("B2",        mame::B2),
    ("B3",        mame::B3),
    ("B4",        mame::B4),
    ("B5",        mame::B5),
    ("B6",        mame::B6),
    ("START",     mame::START),
    ("COIN",      mame::COIN),
    ("P2_START",  mame::P2_START),
    ("P2_COIN",   mame::P2_COIN),
    ("SERVICE",   mame::SERVICE),
    ("MAME_MENU", mame::MAME_MENU),
];

/// Resolve a system-button name to its MAME bit mask.
pub fn mame_bit_for(button: &str) -> Option<u32> {
    MAME_BUTTONS.iter().find(|(n, _)| *n == button).map(|(_, b)| *b)
}

/// Per-system dispatch for button-name → bit-mask lookup. Drives the
/// `apply_bindings_to_poller` + `set_binding` paths once the active system
/// becomes per-launch instead of hardcoded PCE.
pub fn bit_for(system_id: &str, button: &str) -> Option<u32> {
    match system_id {
        // PCE-CD shares the PCE controller — same buttons, same bit layout.
        "tg16" | "pce-cd" => pce_bit_for(button),
        "lynx" => lynx_bit_for(button),
        "nes" => nes_bit_for(button),
        "snes" => snes_bit_for(button),
        "mame" => mame_bit_for(button),
        "atari7800" => atari7800_bit_for(button),
        // Sega CD + 32X both use the 6-button Mega Drive controller —
        // share genesis dispatch directly, same precedent PCE-CD set
        // with TG-16 (the addon and the parent system are pad-identical).
        "genesis" | "segacd" | "sega32x" => genesis_bit_for(button),
        "saturn" => saturn_bit_for(button),
        "psx" => psx_bit_for(button),
        // Neo Geo CD shares the AES home controller — same precedent
        // PCE-CD set with TG-16 and segacd set with genesis.
        "neogeo" | "neocd" => neogeo_bit_for(button),
        "ngp" => ngp_bit_for(button),
        "jaguar" => jaguar_bit_for(button),
        "3do" => threedo_bit_for(button),
        "pcfx" => pcfx_bit_for(button),
        "n64" => n64_bit_for(button),
        "gamecube" => gamecube_bit_for(button),
        "dreamcast" => dreamcast_bit_for(button),
        "psp" => psp_bit_for(button),
        "ps2" => ps2_bit_for(button),
        "nds" => nds_bit_for(button),
        "sms" => sms_bit_for(button),
        "gamegear" => gamegear_bit_for(button),
        "gb" => gb_bit_for(button),
        "gba" => gba_bit_for(button),
        "2600" => atari2600_bit_for(button),
        "5200" => atari5200_bit_for(button),
        "pokemini" => pokemini_bit_for(button),
        "coleco" => coleco_bit_for(button),
        "intv" => intv_bit_for(button),
        "o2" => o2_bit_for(button),
        "channelf" => channelf_bit_for(button),
        "vectrex" => vectrex_bit_for(button),
        "virtualboy" => virtualboy_bit_for(button),
        "wonderswan" => wonderswan_bit_for(button),
        _ => None,
    }
}

/// Per-system dispatch for the canonical button iteration order. Returns
/// the empty slice for unknown systems — the calling UI will render no
/// rows, which is the right behavior for "we haven't registered this
/// system yet."
pub fn buttons_for(system_id: &str) -> &'static [(&'static str, u32)] {
    match system_id {
        "tg16" | "pce-cd" => PCE_BUTTONS,
        "lynx" => LYNX_BUTTONS,
        "nes" => NES_BUTTONS,
        "snes" => SNES_BUTTONS,
        "mame" => MAME_BUTTONS,
        "atari7800" => ATARI7800_BUTTONS,
        "genesis" | "segacd" | "sega32x" => GENESIS_BUTTONS,
        "saturn" => SATURN_BUTTONS,
        "psx" => PSX_BUTTONS,
        "neogeo" | "neocd" => NEOGEO_BUTTONS,
        "ngp" => NGP_BUTTONS,
        "jaguar" => JAGUAR_BUTTONS,
        "3do" => THREEDO_BUTTONS,
        "pcfx" => PCFX_BUTTONS,
        "n64" => N64_BUTTONS,
        "gamecube" => GAMECUBE_BUTTONS,
        "dreamcast" => DREAMCAST_BUTTONS,
        "psp" => PSP_BUTTONS,
        "ps2" => PS2_BUTTONS,
        "nds" => NDS_BUTTONS,
        "sms" => SMS_BUTTONS,
        "gamegear" => GAMEGEAR_BUTTONS,
        "gb" => GB_BUTTONS,
        "gba" => GBA_BUTTONS,
        "2600" => ATARI2600_BUTTONS,
        "5200" => ATARI5200_BUTTONS,
        "pokemini" => POKEMINI_BUTTONS,
        "coleco" => COLECO_BUTTONS,
        "intv" => INTV_BUTTONS,
        "o2" => O2_BUTTONS,
        "channelf" => CHANNELF_BUTTONS,
        "vectrex" => VECTREX_BUTTONS,
        "virtualboy" => VIRTUALBOY_BUTTONS,
        "wonderswan" => WONDERSWAN_BUTTONS,
        _ => &[],
    }
}

/// Remap our PCE-native button bits (`oa_pce::buttons::*`) to the libretro
/// `RETRO_DEVICE_ID_JOYPAD_*` layout that a libretro core's input_state
/// callback expects. The static `oa-pce::PceCore` does this inside its FFI
/// wrapper; for the libretro loader path the shell has to do it before
/// calling `set_input` since `oa-libretro` is system-agnostic.
///
/// libretro joypad bit positions:
///   B=0 (PCE II), Y=1, SELECT=2, START=3 (PCE RUN),
///   UP=4, DOWN=5, LEFT=6, RIGHT=7,
///   A=8 (PCE I).
pub fn pce_to_libretro_bits(b: u32) -> u32 {
    let mut out: u32 = 0;
    if b & pce::I      != 0 { out |= 1 << 8; } // PCE I → libretro A
    if b & pce::II     != 0 { out |= 1 << 0; } // PCE II → libretro B
    if b & pce::SELECT != 0 { out |= 1 << 2; }
    if b & pce::RUN    != 0 { out |= 1 << 3; } // RUN → START
    if b & pce::UP     != 0 { out |= 1 << 4; }
    if b & pce::DOWN   != 0 { out |= 1 << 5; }
    if b & pce::LEFT   != 0 { out |= 1 << 6; }
    if b & pce::RIGHT  != 0 { out |= 1 << 7; }
    out
}

/// Lynx → libretro bit remap. Identity by construction — `lynx::*` bits are
/// laid out to match the `RETRO_DEVICE_ID_JOYPAD_*` positions directly. The
/// function is kept for symmetry with `pce_to_libretro_bits` and as the call
/// site the dispatch table in `set_input_remapped` looks up.
pub fn lynx_to_libretro_bits(b: u32) -> u32 {
    // Mask out any high bits that aren't part of the Lynx layout to defend
    // against a stale or out-of-range value sneaking through.
    b & (lynx::B
        | lynx::SELECT
        | lynx::START
        | lynx::UP
        | lynx::DOWN
        | lynx::LEFT
        | lynx::RIGHT
        | lynx::A
        | lynx::PAUSE)
}

/// NES → libretro bit remap. Identity by construction; mask trims to the
/// 8-bit NES button set.
pub fn nes_to_libretro_bits(b: u32) -> u32 {
    b & (nes::B | nes::SELECT | nes::START | nes::UP | nes::DOWN | nes::LEFT | nes::RIGHT | nes::A)
}

/// SNES → libretro bit remap. Identity by construction; mask trims to the
/// 12-bit SNES button set.
pub fn snes_to_libretro_bits(b: u32) -> u32 {
    b & (snes::B
        | snes::Y
        | snes::SELECT
        | snes::START
        | snes::UP
        | snes::DOWN
        | snes::LEFT
        | snes::RIGHT
        | snes::A
        | snes::X
        | snes::L
        | snes::R)
}

/// Atari 7800 → libretro bit remap. Identity by construction (the
/// `atari7800::*` constants are laid out as libretro RetroPad bits
/// directly); mask trims to the 8-bit Atari 7800 button set so stray
/// high bits get dropped.
pub fn atari7800_to_libretro_bits(b: u32) -> u32 {
    b & (atari7800::B1
        | atari7800::B2
        | atari7800::SELECT
        | atari7800::PAUSE
        | atari7800::UP
        | atari7800::DOWN
        | atari7800::LEFT
        | atari7800::RIGHT)
}

/// Sony PSP → libretro bit remap. Identity by construction; mask
/// trims to the 12-bit PSP button set. Analog stick flows via axes.
pub fn psp_to_libretro_bits(b: u32) -> u32 {
    b & (psp::CROSS
        | psp::CIRCLE
        | psp::TRIANGLE
        | psp::SQUARE
        | psp::L
        | psp::R
        | psp::START
        | psp::SELECT
        | psp::UP
        | psp::DOWN
        | psp::LEFT
        | psp::RIGHT)
}

/// Sony PS2 → libretro bit remap. Identity by construction; mask trims
/// to the 16-bit DualShock 2 button set. Dual analog sticks flow via
/// axes; pressure-sensitive buttons + analog L2/R2 = Phase 2.5.
pub fn ps2_to_libretro_bits(b: u32) -> u32 {
    b & (ps2::CROSS
        | ps2::CIRCLE
        | ps2::TRIANGLE
        | ps2::SQUARE
        | ps2::L1
        | ps2::R1
        | ps2::L2
        | ps2::R2
        | ps2::L3
        | ps2::R3
        | ps2::START
        | ps2::SELECT
        | ps2::UP
        | ps2::DOWN
        | ps2::LEFT
        | ps2::RIGHT)
}

/// Nintendo DS → libretro bit remap. Identity by construction; mask
/// trims to the 12-bit DS button set. Touch screen flows via
/// `InputState.pointer` (separate from the bit-set).
pub fn nds_to_libretro_bits(b: u32) -> u32 {
    b & (nds::A
        | nds::B
        | nds::X
        | nds::Y
        | nds::L
        | nds::R
        | nds::START
        | nds::SELECT
        | nds::UP
        | nds::DOWN
        | nds::LEFT
        | nds::RIGHT)
}

/// Sega Dreamcast → libretro bit remap. Identity by construction;
/// mask trims to the 11-bit DC button set. Analog stick flows via
/// `InputState.axes` (not bits).
pub fn dreamcast_to_libretro_bits(b: u32) -> u32 {
    b & (dreamcast::A
        | dreamcast::B
        | dreamcast::X
        | dreamcast::Y
        | dreamcast::L
        | dreamcast::R
        | dreamcast::START
        | dreamcast::UP
        | dreamcast::DOWN
        | dreamcast::LEFT
        | dreamcast::RIGHT)
}

/// Nintendo 64 → libretro bit remap. Identity by construction; mask
/// trims to the 14-bit N64 button set. Main analog stick flows
/// through `InputState.axes` (not bits).
pub fn n64_to_libretro_bits(b: u32) -> u32 {
    b & (n64::A
        | n64::B
        | n64::START
        | n64::L
        | n64::R
        | n64::Z
        | n64::C_UP
        | n64::C_DOWN
        | n64::C_LEFT
        | n64::C_RIGHT
        | n64::UP
        | n64::DOWN
        | n64::LEFT
        | n64::RIGHT)
}

/// Nintendo GameCube → libretro bit remap. Identity by construction;
/// mask trims to the 12-bit GC button set. Main stick + C-stick flow
/// through `InputState.axes` (not bits).
pub fn gamecube_to_libretro_bits(b: u32) -> u32 {
    b & (gamecube::A
        | gamecube::B
        | gamecube::X
        | gamecube::Y
        | gamecube::L
        | gamecube::R
        | gamecube::Z
        | gamecube::START
        | gamecube::UP
        | gamecube::DOWN
        | gamecube::LEFT
        | gamecube::RIGHT)
}

/// Atari Jaguar → libretro bit remap. Identity by construction for
/// the lower 16 bits (RetroPad range); shell-reserved high bits
/// (KP8 / KP9 / KP_STAR / KP0 / KP_HASH = bits 16-20) get masked off
/// before reaching the core — keyboard-passthrough dispatch for those
/// 5 keypad keys is Phase 2 work. The mask trims to the 16-bit
/// RetroPad button set so high bits and other stray bits get dropped.
pub fn jaguar_to_libretro_bits(b: u32) -> u32 {
    b & (jaguar::A
        | jaguar::B
        | jaguar::C
        | jaguar::OPTION
        | jaguar::PAUSE
        | jaguar::UP
        | jaguar::DOWN
        | jaguar::LEFT
        | jaguar::RIGHT
        | jaguar::KP1
        | jaguar::KP2
        | jaguar::KP3
        | jaguar::KP4
        | jaguar::KP5
        | jaguar::KP6
        | jaguar::KP7)
}

/// 3DO → libretro bit remap. Identity by construction; mask trims to
/// the 11-bit 3DO button set so stray high bits get dropped.
pub fn threedo_to_libretro_bits(b: u32) -> u32 {
    b & (threedo::A
        | threedo::B
        | threedo::C
        | threedo::L
        | threedo::R
        | threedo::START
        | threedo::STOP
        | threedo::PLAY
        | threedo::UP
        | threedo::DOWN
        | threedo::LEFT
        | threedo::RIGHT)
}

/// PC-FX → libretro bit remap. Identity by construction; mask trims to
/// the 12-bit PC-FX button set.
pub fn pcfx_to_libretro_bits(b: u32) -> u32 {
    b & (pcfx::I
        | pcfx::II
        | pcfx::III
        | pcfx::IV
        | pcfx::V
        | pcfx::VI
        | pcfx::RUN
        | pcfx::SELECT
        | pcfx::UP
        | pcfx::DOWN
        | pcfx::LEFT
        | pcfx::RIGHT)
}

/// SNK Neo Geo → libretro bit remap. Identity by construction; mask
/// trims to the 10-bit Neo Geo button set so stray high bits get
/// dropped. Same fn drives both `neogeo` cart and `neocd` CD paths.
pub fn neogeo_to_libretro_bits(b: u32) -> u32 {
    b & (neogeo::A
        | neogeo::B
        | neogeo::C
        | neogeo::D
        | neogeo::START
        | neogeo::COIN
        | neogeo::UP
        | neogeo::DOWN
        | neogeo::LEFT
        | neogeo::RIGHT)
}

/// SNK Neo Geo Pocket → libretro bit remap. Identity by construction;
/// mask trims to the 7-bit NGP button set.
pub fn ngp_to_libretro_bits(b: u32) -> u32 {
    b & (ngp::A
        | ngp::B
        | ngp::OPTION
        | ngp::UP
        | ngp::DOWN
        | ngp::LEFT
        | ngp::RIGHT)
}

/// Sega Saturn → libretro bit remap. Identity by construction (the
/// `saturn::*` constants are laid out as libretro RetroPad bits
/// directly); mask trims to the 13-bit Saturn button set so stray high
/// bits get dropped. Saturn C and Z legitimately live in the L2/R2
/// trigger slots since the diamond has only 4 face slots.
pub fn saturn_to_libretro_bits(b: u32) -> u32 {
    b & (saturn::A
        | saturn::B
        | saturn::C
        | saturn::X
        | saturn::Y
        | saturn::Z
        | saturn::L
        | saturn::R
        | saturn::START
        | saturn::UP
        | saturn::DOWN
        | saturn::LEFT
        | saturn::RIGHT)
}

/// Sony PlayStation → libretro bit remap. Identity by construction;
/// mask trims to the 14-bit digital DualPad button set. DualShock
/// analog sticks (Left/Right + L3/R3) are deferred to Phase 2.
pub fn psx_to_libretro_bits(b: u32) -> u32 {
    b & (psx::CROSS
        | psx::CIRCLE
        | psx::TRIANGLE
        | psx::SQUARE
        | psx::L1
        | psx::R1
        | psx::L2
        | psx::R2
        | psx::START
        | psx::SELECT
        | psx::UP
        | psx::DOWN
        | psx::LEFT
        | psx::RIGHT)
}

/// Genesis → libretro bit remap. Identity by construction (the
/// `genesis::*` constants are laid out as libretro RetroPad bits
/// directly); mask trims to the 12-bit Genesis button set so stray
/// high bits get dropped.
pub fn genesis_to_libretro_bits(b: u32) -> u32 {
    b & (genesis::A
        | genesis::B
        | genesis::C
        | genesis::X
        | genesis::Y
        | genesis::Z
        | genesis::START
        | genesis::MODE
        | genesis::UP
        | genesis::DOWN
        | genesis::LEFT
        | genesis::RIGHT)
}

/// Vectrex → libretro bit remap. Identity by construction.
pub fn vectrex_to_libretro_bits(b: u32) -> u32 {
    b & (vectrex::B1 | vectrex::B2 | vectrex::B3 | vectrex::B4
        | vectrex::UP | vectrex::DOWN | vectrex::LEFT | vectrex::RIGHT)
}

/// Virtual Boy → libretro bit remap. Identity by construction.
pub fn virtualboy_to_libretro_bits(b: u32) -> u32 {
    b & (virtualboy::A | virtualboy::B | virtualboy::L | virtualboy::R
        | virtualboy::START | virtualboy::SELECT
        | virtualboy::UP | virtualboy::DOWN | virtualboy::LEFT | virtualboy::RIGHT)
}

/// WonderSwan → libretro bit remap. Identity by construction.
pub fn wonderswan_to_libretro_bits(b: u32) -> u32 {
    b & (wonderswan::A | wonderswan::B | wonderswan::START
        | wonderswan::UP | wonderswan::DOWN | wonderswan::LEFT | wonderswan::RIGHT)
}

/// ColecoVision → libretro bit remap. Identity by construction; mask
/// trims to the 16-bit Coleco button set so stray high bits get dropped.
pub fn coleco_to_libretro_bits(b: u32) -> u32 {
    b & (coleco::L_FIRE | coleco::R_FIRE
        | coleco::KP0 | coleco::KP1 | coleco::KP2 | coleco::KP3 | coleco::KP4
        | coleco::KP5 | coleco::KP6 | coleco::KP7 | coleco::KP8 | coleco::KP9
        | coleco::UP | coleco::DOWN | coleco::LEFT | coleco::RIGHT)
}

/// Intellivision → libretro bit remap. Identity by construction.
pub fn intv_to_libretro_bits(b: u32) -> u32 {
    b & (intv::UPPER_L | intv::UPPER_R | intv::LOWER_L | intv::LOWER_R
        | intv::START | intv::SELECT
        | intv::UP | intv::DOWN | intv::LEFT | intv::RIGHT)
}

/// Magnavox Odyssey² → libretro bit remap. Identity by construction.
pub fn o2_to_libretro_bits(b: u32) -> u32 {
    b & (o2::ACTION | o2::UP | o2::DOWN | o2::LEFT | o2::RIGHT)
}

/// Fairchild Channel F → libretro bit remap. Identity by construction.
pub fn channelf_to_libretro_bits(b: u32) -> u32 {
    b & (channelf::FIRE | channelf::MODE | channelf::TIME | channelf::START | channelf::HOLD
        | channelf::UP | channelf::DOWN | channelf::LEFT | channelf::RIGHT)
}

/// Atari 2600 → libretro bit remap. Identity by construction; mask
/// trims to the 7-bit 2600 button set so stray high bits get dropped.
pub fn atari2600_to_libretro_bits(b: u32) -> u32 {
    b & (atari2600::FIRE
        | atari2600::SELECT
        | atari2600::RESET
        | atari2600::UP
        | atari2600::DOWN
        | atari2600::LEFT
        | atari2600::RIGHT)
}

/// Atari 5200 → libretro bit remap. Identity by construction; mask
/// trims to the 9-bit 5200 button set (no keypad bits — those flow via
/// libretro KEYBOARD device on Phase 2 polish).
pub fn atari5200_to_libretro_bits(b: u32) -> u32 {
    b & (atari5200::FIRE1
        | atari5200::FIRE2
        | atari5200::SELECT
        | atari5200::START
        | atari5200::RESET
        | atari5200::UP
        | atari5200::DOWN
        | atari5200::LEFT
        | atari5200::RIGHT)
}

/// Pokémon Mini → libretro bit remap. Identity by construction; mask
/// trims to the 8-bit PokeMini button set (d-pad + A + B + C + SHAKE).
pub fn pokemini_to_libretro_bits(b: u32) -> u32 {
    b & (pokemini::A
        | pokemini::B
        | pokemini::C
        | pokemini::UP
        | pokemini::DOWN
        | pokemini::LEFT
        | pokemini::RIGHT
        | pokemini::SHAKE)
}

/// Game Boy Advance → libretro bit remap. Identity by construction;
/// mask trims to the 10-bit GBA button set so stray high bits get
/// dropped.
pub fn gba_to_libretro_bits(b: u32) -> u32 {
    b & (gba::B
        | gba::A
        | gba::L
        | gba::R
        | gba::SELECT
        | gba::START
        | gba::UP
        | gba::DOWN
        | gba::LEFT
        | gba::RIGHT)
}

/// Game Boy / Game Boy Color → libretro bit remap. Identity by
/// construction (the `gb::*` constants are laid out as libretro
/// RetroPad bits directly); mask trims to the 8-bit GB button set so
/// stray high bits get dropped.
pub fn gb_to_libretro_bits(b: u32) -> u32 {
    b & (gb::B | gb::SELECT | gb::START | gb::UP | gb::DOWN | gb::LEFT | gb::RIGHT | gb::A)
}

/// Sega Master System → libretro bit remap. Identity by construction
/// (the `sms::*` constants are laid out as libretro RetroPad bits
/// directly); mask trims to the 7-bit SMS button set so stray high
/// bits get dropped.
pub fn sms_to_libretro_bits(b: u32) -> u32 {
    b & (sms::B1
        | sms::B2
        | sms::PAUSE
        | sms::UP
        | sms::DOWN
        | sms::LEFT
        | sms::RIGHT)
}

/// Sega Game Gear → libretro bit remap. Identity by construction;
/// mask trims to the 7-bit GG button set.
pub fn gamegear_to_libretro_bits(b: u32) -> u32 {
    b & (gamegear::B1
        | gamegear::B2
        | gamegear::START
        | gamegear::UP
        | gamegear::DOWN
        | gamegear::LEFT
        | gamegear::RIGHT)
}

/// MAME → libretro bit remap. Identity by construction (the `mame::*`
/// constants are laid out as libretro RetroPad bits directly); mask
/// trims to the 16-bit MAME button set so stray high bits get dropped.
pub fn mame_to_libretro_bits(b: u32) -> u32 {
    b & (mame::B1
        | mame::B2
        | mame::B3
        | mame::B4
        | mame::B5
        | mame::B6
        | mame::COIN
        | mame::START
        | mame::UP
        | mame::DOWN
        | mame::LEFT
        | mame::RIGHT
        | mame::P2_COIN
        | mame::P2_START
        | mame::SERVICE
        | mame::MAME_MENU)
}

/// Per-system dispatch for the shell-internal → libretro bit remap. Called
/// from the emu thread's `set_input_remapped` once per port per frame.
/// Unknown system ids fall back to identity (no remap) so a brand-new
/// system can ship a working input pipeline before bindings.rs gets its
/// dedicated table — the bindings UI just writes libretro bits directly.
pub fn to_libretro_bits(system_id: &str, b: u32) -> u32 {
    match system_id {
        "tg16" | "pce-cd" => pce_to_libretro_bits(b),
        "lynx" => lynx_to_libretro_bits(b),
        "nes" => nes_to_libretro_bits(b),
        "snes" => snes_to_libretro_bits(b),
        "mame" => mame_to_libretro_bits(b),
        "atari7800" => atari7800_to_libretro_bits(b),
        "genesis" | "segacd" | "sega32x" => genesis_to_libretro_bits(b),
        "saturn" => saturn_to_libretro_bits(b),
        "psx" => psx_to_libretro_bits(b),
        "neogeo" | "neocd" => neogeo_to_libretro_bits(b),
        "ngp" => ngp_to_libretro_bits(b),
        "jaguar" => jaguar_to_libretro_bits(b),
        "3do" => threedo_to_libretro_bits(b),
        "pcfx" => pcfx_to_libretro_bits(b),
        "n64" => n64_to_libretro_bits(b),
        "gamecube" => gamecube_to_libretro_bits(b),
        "dreamcast" => dreamcast_to_libretro_bits(b),
        "psp" => psp_to_libretro_bits(b),
        "ps2" => ps2_to_libretro_bits(b),
        "nds" => nds_to_libretro_bits(b),
        "sms" => sms_to_libretro_bits(b),
        "gamegear" => gamegear_to_libretro_bits(b),
        "gb" => gb_to_libretro_bits(b),
        "gba" => gba_to_libretro_bits(b),
        "2600" => atari2600_to_libretro_bits(b),
        "5200" => atari5200_to_libretro_bits(b),
        "pokemini" => pokemini_to_libretro_bits(b),
        "coleco" => coleco_to_libretro_bits(b),
        "intv" => intv_to_libretro_bits(b),
        "o2" => o2_to_libretro_bits(b),
        "channelf" => channelf_to_libretro_bits(b),
        "vectrex" => vectrex_to_libretro_bits(b),
        "virtualboy" => virtualboy_to_libretro_bits(b),
        "wonderswan" => wonderswan_to_libretro_bits(b),
        _ => b,
    }
}

/// Compiled-in PCE keyboard + gamepad defaults — matches RetroArch's Beetle
/// PCE Fast layout (east=I/south=II/start=RUN/select=SELECT, arrows + Z/X +
/// Enter/RShift on the keyboard).
pub fn default_pce_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",     Some("Up"),     Some("DPadUp")),
        ("DOWN",   Some("Down"),   Some("DPadDown")),
        ("LEFT",   Some("Left"),   Some("DPadLeft")),
        ("RIGHT",  Some("Right"),  Some("DPadRight")),
        ("I",      Some("Z"),      Some("East")),
        ("II",     Some("X"),      Some("South")),
        ("RUN",    Some("Enter"),  Some("Start")),
        ("SELECT", Some("RShift"), Some("Select")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Compiled-in Lynx keyboard + gamepad defaults. Maps Lynx Option 1 / Option
/// 2 to libretro START / SELECT per the RetroArch convention; Pause sits on
/// libretro L which most modern controllers expose as the left bumper or
/// a dedicated face button. Keyboard layout mirrors PCE's Z/X muscle memory:
/// **Z = primary action (libretro A)**, **X = secondary (libretro B)**.
/// Was originally swapped (X=A, Z=B) following PC emulator conventions, but
/// users expect Z to be "jump / primary" across every system in the launcher
/// — and PCE established that pattern in Phase 1.
pub fn default_lynx_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",    Some("Up"),     Some("DPadUp")),
        ("DOWN",  Some("Down"),   Some("DPadDown")),
        ("LEFT",  Some("Left"),   Some("DPadLeft")),
        ("RIGHT", Some("Right"),  Some("DPadRight")),
        ("A",     Some("Z"),      Some("East")),    // libretro A — primary (matches PCE I)
        ("B",     Some("X"),      Some("South")),   // libretro B — secondary (matches PCE II)
        ("OPT1",  Some("Enter"),  Some("Start")),
        ("OPT2",  Some("RShift"), Some("Select")),
        ("PAUSE", Some("Space"),  Some("LeftTrigger")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// NES defaults — **Z = A (primary)**, **X = B (secondary)** to match the
/// project-wide PCE convention (Z is "jump / primary action" on every
/// system). The PC NES emulator convention (X=A, Z=B going back to
/// Nestopia / FCEUX) was historically the other way, but consistency
/// across all systems wins over preserving any one emulator's tradition.
/// D-pad on arrows, START on Enter, SELECT on RShift.
pub fn default_nes_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",     Some("Up"),     Some("DPadUp")),
        ("DOWN",   Some("Down"),   Some("DPadDown")),
        ("LEFT",   Some("Left"),   Some("DPadLeft")),
        ("RIGHT",  Some("Right"),  Some("DPadRight")),
        ("A",      Some("Z"),      Some("East")),
        ("B",      Some("X"),      Some("South")),
        ("START",  Some("Enter"),  Some("Start")),
        ("SELECT", Some("RShift"), Some("Select")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// SNES defaults — **Z = A (primary)**, **X = B (secondary)** to match the
/// project-wide PCE convention. **S = X (top of diamond)**, **A = Y (left
/// of diamond)**. Z/X are the two bottom-row keys directly under the
/// resting hand for the most-used buttons; S/A sit on the top QWERTY row
/// for the less-used SNES X/Y. Shoulders on Q/W, START/SELECT as elsewhere.
/// Departs from the classic ZSNES convention (which had A=A, B=S etc.) in
/// favor of cross-system consistency — Z is "jump / primary action"
/// across every system in the launcher.
pub fn default_snes_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",     Some("Up"),     Some("DPadUp")),
        ("DOWN",   Some("Down"),   Some("DPadDown")),
        ("LEFT",   Some("Left"),   Some("DPadLeft")),
        ("RIGHT",  Some("Right"),  Some("DPadRight")),
        // Diamond layout: A right, B bottom, X top, Y left.
        ("A",      Some("Z"),      Some("East")),
        ("B",      Some("X"),      Some("South")),
        ("X",      Some("S"),      Some("North")),
        ("Y",      Some("A"),      Some("West")),
        ("L",      Some("Q"),      Some("LeftTrigger")),
        ("R",      Some("W"),      Some("RightTrigger")),
        ("START",  Some("Enter"),  Some("Start")),
        ("SELECT", Some("RShift"), Some("Select")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Atari 7800 defaults — Pro-Line joystick layout. **Z = Button 1
/// (primary fire)**, **X = Button 2** per the cross-system "Z is
/// primary" rule. The 7800's two-button design is the simplest in OA's
/// lineup beyond the single-button Atari 2600; most games use Button 1
/// for the main action (shoot / jump) and Button 2 for a secondary
/// (kick / weapon switch / lightning). Enter for Pause; RShift for
/// Select (the 7800 console had a hardware Select switch the libretro
/// core surfaces via `RETRO_DEVICE_ID_JOYPAD_SELECT`).
pub fn default_atari7800_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",     Some("Up"),     Some("DPadUp")),
        ("DOWN",   Some("Down"),   Some("DPadDown")),
        ("LEFT",   Some("Left"),   Some("DPadLeft")),
        ("RIGHT",  Some("Right"),  Some("DPadRight")),
        ("B1",     Some("Z"),      Some("East")),    // libretro B — primary (Button 1)
        ("B2",     Some("X"),      Some("South")),   // libretro A — secondary (Button 2)
        ("PAUSE",  Some("Enter"),  Some("Start")),
        ("SELECT", Some("RShift"), Some("Select")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Genesis defaults — 6-button Mega Drive controller. Bottom-row A/B/C
/// follows the cross-system "Z is primary action" rule (locked by the
/// `z_is_the_primary_action_button_on_every_system` test): keyboard **Z
/// = B (libretro bit 0)** — the middle face button — which most MD
/// games use for the main action (jump in Sonic, attack in Streets of
/// Rage). Keyboard X = C (right face, libretro bit 8) for the secondary
/// action; keyboard A = A (left face, libretro Y bit 1) for tertiary.
///
/// **Gamepad B → East** matches the lynx/nes/snes/atari7800 console-shape
/// convention (every console system in OA pins primary action to East),
/// **C → South** is the natural secondary, **A → West** is tertiary.
/// The MD pad's horizontal A-B-C row is _not_ mapped left-to-right onto
/// West-South-East because that would route primary to South and break
/// the cross-system rule.
///
/// 6-button top row (X/Y/Z) mirrors the SNES shoulder + diamond pattern:
/// X = Q + LeftTrigger, Y = S + North, Z = W + RightTrigger.
/// START/MODE as elsewhere (Enter + Start, RShift + Select).
pub fn default_genesis_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",    Some("Up"),     Some("DPadUp")),
        ("DOWN",  Some("Down"),   Some("DPadDown")),
        ("LEFT",  Some("Left"),   Some("DPadLeft")),
        ("RIGHT", Some("Right"),  Some("DPadRight")),
        // Bottom row of the 6-button face: A-B-C. Z = B = primary.
        ("A",     Some("A"),      Some("West")),    // libretro Y — tertiary
        ("B",     Some("Z"),      Some("East")),    // libretro B — primary
        ("C",     Some("X"),      Some("South")),   // libretro A — secondary
        // Top row of the 6-button face: X-Y-Z. SNES-shoulder pattern.
        ("X",     Some("Q"),      Some("LeftTrigger")),
        ("Y",     Some("S"),      Some("North")),
        ("Z",     Some("W"),      Some("RightTrigger")),
        ("START", Some("Enter"),  Some("Start")),
        ("MODE",  Some("RShift"), Some("Select")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Sony PSP defaults — d-pad + 4 face diamond + L/R + START + SELECT.
/// **Z = Cross (primary)**, **X = Circle (secondary)** per the cross-
/// system rule (same shape as PSX). Single analog stick flows via
/// `InputState.axes[0..2]`.
pub fn default_psp_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",       Some("Up"),     Some("DPadUp")),
        ("DOWN",     Some("Down"),   Some("DPadDown")),
        ("LEFT",     Some("Left"),   Some("DPadLeft")),
        ("RIGHT",    Some("Right"),  Some("DPadRight")),
        ("CROSS",    Some("Z"),      Some("East")),
        ("CIRCLE",   Some("X"),      Some("South")),
        ("SQUARE",   Some("A"),      Some("West")),
        ("TRIANGLE", Some("S"),      Some("North")),
        ("L",        Some("Q"),      Some("LeftTrigger")),
        ("R",        Some("W"),      Some("RightTrigger")),
        ("START",    Some("Enter"),  Some("Start")),
        ("SELECT",   Some("RShift"), Some("Select")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Sony PS2 (DualShock 2) defaults — PSX-shape + L3/R3 stick clicks.
/// **Z = Cross (primary)**, **X = Circle (secondary)** per cross-
/// system rule. L1/R1 on Q/W shoulders, L2/R2 on E/R rear triggers
/// (same as PSX defaults). L3/R3 stick clicks bind to pad LeftThumb /
/// RightThumb on the gamepad side; keyboard defaults unbound (stick
/// clicks are a rare game-action key on keyboard). Dual analog
/// sticks flow via `InputState.axes`.
pub fn default_ps2_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",       Some("Up"),     Some("DPadUp")),
        ("DOWN",     Some("Down"),   Some("DPadDown")),
        ("LEFT",     Some("Left"),   Some("DPadLeft")),
        ("RIGHT",    Some("Right"),  Some("DPadRight")),
        ("CROSS",    Some("Z"),      Some("East")),
        ("CIRCLE",   Some("X"),      Some("South")),
        ("SQUARE",   Some("A"),      Some("West")),
        ("TRIANGLE", Some("S"),      Some("North")),
        ("L1",       Some("Q"),      Some("LeftTrigger")),
        ("R1",       Some("W"),      Some("RightTrigger")),
        ("L2",       Some("E"),      Some("LeftTrigger2")),
        ("R2",       Some("R"),      Some("RightTrigger2")),
        ("L3",       None,           Some("LeftThumb")),
        ("R3",       None,           Some("RightThumb")),
        ("START",    Some("Enter"),  Some("Start")),
        ("SELECT",   Some("RShift"), Some("Select")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Nintendo DS defaults — d-pad + A/B/X/Y Nintendo diamond + L/R +
/// START + SELECT. **Z = A (primary)**, **X = B (secondary)** per
/// cross-system rule (same shape as nes/snes/gb/gba). X/Y diamond:
/// A=East, B=South, X=North, Y=West. Touch screen flows via
/// `InputState.pointer` (mouse-as-touch).
pub fn default_nds_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",     Some("Up"),     Some("DPadUp")),
        ("DOWN",   Some("Down"),   Some("DPadDown")),
        ("LEFT",   Some("Left"),   Some("DPadLeft")),
        ("RIGHT",  Some("Right"),  Some("DPadRight")),
        ("A",      Some("Z"),      Some("East")),    // libretro A — PRIMARY
        ("B",      Some("X"),      Some("South")),   // libretro B — secondary
        ("X",      Some("S"),      Some("North")),   // libretro X — top face
        ("Y",      Some("A"),      Some("West")),    // libretro Y — left face
        ("L",      Some("Q"),      Some("LeftTrigger")),
        ("R",      Some("W"),      Some("RightTrigger")),
        ("START",  Some("Enter"),  Some("Start")),
        ("SELECT", Some("RShift"), Some("Select")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Sega Dreamcast defaults — d-pad + A/B/X/Y face diamond + L/R
/// analog triggers + START. **Z keyboard = A (primary)**, **X
/// keyboard = B (secondary)** per the cross-system rule. X/Y diamond
/// face on A/S keyboard (west/north of diamond); L/R triggers on
/// Q/W shoulders. No SELECT — the DC pad doesn't have one. Analog
/// stick flows via `InputState.axes[0..2]` (gamepad LeftStick).
pub fn default_dreamcast_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",    Some("Up"),     Some("DPadUp")),
        ("DOWN",  Some("Down"),   Some("DPadDown")),
        ("LEFT",  Some("Left"),   Some("DPadLeft")),
        ("RIGHT", Some("Right"),  Some("DPadRight")),
        ("A",     Some("Z"),      Some("East")),    // libretro B — PRIMARY
        ("B",     Some("X"),      Some("South")),   // libretro A — secondary
        ("X",     Some("A"),      Some("West")),    // libretro Y — west face
        ("Y",     Some("S"),      Some("North")),   // libretro X — north face
        ("L",     Some("Q"),      Some("LeftTrigger")),
        ("R",     Some("W"),      Some("RightTrigger")),
        ("START", Some("Enter"),  Some("Start")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Nintendo 64 defaults — d-pad + A/B + L/R/Z + START + 4 C-buttons.
/// **Z keyboard = A button (primary)**, **X keyboard = B (secondary)**
/// per the cross-system rule. C-buttons map to the QWERTY home-row's
/// adjacent letters around S (the natural "right-side cluster" beside
/// Z/X for game-action keys): C-Up = T, C-Down = G, C-Left = F,
/// C-Right = H. Gamepad: A on East, B on South, C-buttons on the
/// right analog stick directions are implicit via the analog axes —
/// we still bind digital C-button pad slots (LeftTrigger2 /
/// RightTrigger2 / etc.) so operators without a right analog stick
/// can play.
///
/// The N64's MAIN analog stick is NOT in this binding table; it flows
/// through `InputState.axes[0..2]` from the gamepad's LeftStick.
/// Keyboard-only users enable Mupen64Plus-Next's "Map d-pad to analog
/// stick" core option to get full-tilt movement from arrow keys.
pub fn default_n64_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",      Some("Up"),     Some("DPadUp")),
        ("DOWN",    Some("Down"),   Some("DPadDown")),
        ("LEFT",    Some("Left"),   Some("DPadLeft")),
        ("RIGHT",   Some("Right"),  Some("DPadRight")),
        ("A",       Some("Z"),      Some("East")),    // libretro B — PRIMARY
        ("B",       Some("X"),      Some("South")),   // libretro Y — secondary
        ("START",   Some("Enter"),  Some("Start")),
        ("L",       Some("Q"),      Some("LeftTrigger")),
        ("R",       Some("W"),      Some("RightTrigger")),
        ("Z",       Some("Space"),  Some("LeftTrigger2")), // Z trigger is the iconic N64 "use" button
        ("C_UP",    Some("T"),      Some("North")),    // C-stick directions on QWERTY T/G/F/H
        ("C_DOWN",  Some("G"),      None),
        ("C_LEFT",  Some("F"),      None),
        ("C_RIGHT", Some("H"),      None),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Nintendo GameCube defaults — d-pad + A/B/X/Y + L/R/Z + START.
/// **Z keyboard = A button (primary)**, **X keyboard = B (secondary)**
/// per the cross-system rule. X/Y diamond on A/S keyboard, L/R on
/// Q/W shoulders, Z trigger on Space (it's the "use" button for most
/// GC titles, parallel to N64's Z usage).
///
/// Main stick + C-stick are NOT in this binding table — both flow
/// through `InputState.axes` from gamepad analog sticks (LeftStick →
/// main stick, RightStick → C-stick). Keyboard-only users enable
/// Dolphin's "Map d-pad to analog stick" core option for main-stick
/// movement; C-stick has no keyboard fallback at Phase 0 (Phase 2.5
/// polish for per-axis keyboard binding).
pub fn default_gamecube_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",    Some("Up"),     Some("DPadUp")),
        ("DOWN",  Some("Down"),   Some("DPadDown")),
        ("LEFT",  Some("Left"),   Some("DPadLeft")),
        ("RIGHT", Some("Right"),  Some("DPadRight")),
        ("A",     Some("Z"),      Some("East")),    // libretro B — PRIMARY
        ("B",     Some("X"),      Some("South")),   // libretro Y — secondary
        ("X",     Some("A"),      Some("West")),    // libretro A — east face
        ("Y",     Some("S"),      Some("North")),   // libretro X — north face
        ("L",     Some("Q"),      Some("LeftTrigger")),
        ("R",     Some("W"),      Some("RightTrigger")),
        ("Z",     Some("Space"),  Some("RightTrigger2")),
        ("START", Some("Enter"),  Some("Start")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Atari Jaguar defaults — d-pad + A/B/C face + OPTION + PAUSE + 12-key
/// keypad. **Z = A (primary)**, **X = B (secondary)** per the
/// cross-system rule. C on keyboard A / pad West (tertiary face). The
/// 12-key keypad mirrors the Jaguar Pro Pad's physical 4×3 layout on
/// the QWERTY numpad-equivalent home-row area:
///
/// ```text
/// Keyboard layout (game-action cluster + numpad-style keypad):
///   Q W                      L/R (unused on Jaguar pad)
///   A S D F G H              C (=A pad) / pad shoulders
///   Z X C V B N M            A/B/C bottom face row
///   1 2 3 4 5 6 7 8 9 0      keypad row 1-9 + 0
///   - =                      * / # (assigned to common spare keys)
/// ```
///
/// In practice OA's `Bindings` keyboard layer maps each numeric key
/// directly (`Key1`-`Key9`, `Key0`). For KP_STAR / KP_HASH, the
/// closest QWERTY equivalents are `Minus` and `Equals` — but those
/// labels feel arbitrary on the per-system page, so they're left
/// unbound by default and operators set them via the Bindings dialog.
/// Gamepad bindings: the 7 mappable keypad keys (KP1-KP7) get
/// LeftTrigger / RightTrigger / LeftTrigger2 / RightTrigger2 / LeftThumb /
/// RightThumb assignments + libretro X for KP1 — but for default
/// usability we leave KP1-KP7 unbound on pad so the user opts-in via
/// the Bindings page (most Jaguar games are playable without the
/// numpad).
pub fn default_jaguar_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",      Some("Up"),     Some("DPadUp")),
        ("DOWN",    Some("Down"),   Some("DPadDown")),
        ("LEFT",    Some("Left"),   Some("DPadLeft")),
        ("RIGHT",   Some("Right"),  Some("DPadRight")),
        ("A",       Some("Z"),      Some("East")),    // libretro B — PRIMARY
        ("B",       Some("X"),      Some("South")),   // libretro A — secondary
        ("C",       Some("A"),      Some("West")),    // libretro Y — tertiary
        ("OPTION",  Some("Enter"),  Some("Start")),
        ("PAUSE",   Some("RShift"), Some("Select")),
        ("KP1",     Some("Key1"),   None),
        ("KP2",     Some("Key2"),   None),
        ("KP3",     Some("Key3"),   None),
        ("KP4",     Some("Key4"),   None),
        ("KP5",     Some("Key5"),   None),
        ("KP6",     Some("Key6"),   None),
        ("KP7",     Some("Key7"),   None),
        ("KP8",     Some("Key8"),   None),
        ("KP9",     Some("Key9"),   None),
        ("KP_STAR", None,           None), // Operator-assignable; no obvious QWERTY default
        ("KP0",     Some("Key0"),   None),
        ("KP_HASH", None,           None), // Operator-assignable
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// 3DO defaults — d-pad + A/B/C face + L/R shoulders + STOP + PLAY +
/// START. **Z = A (primary)**, **X = B (secondary)** per the
/// cross-system rule. STOP/PLAY map to LeftTrigger2 / RightTrigger2
/// on pad and `Key1`/`Key2` on keyboard (rare-use system buttons).
pub fn default_threedo_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",    Some("Up"),     Some("DPadUp")),
        ("DOWN",  Some("Down"),   Some("DPadDown")),
        ("LEFT",  Some("Left"),   Some("DPadLeft")),
        ("RIGHT", Some("Right"),  Some("DPadRight")),
        ("A",     Some("Z"),      Some("East")),    // libretro B — PRIMARY
        ("B",     Some("X"),      Some("South")),   // libretro A — secondary
        ("C",     Some("A"),      Some("West")),    // libretro Y — tertiary
        ("L",     Some("Q"),      Some("LeftTrigger")),
        ("R",     Some("W"),      Some("RightTrigger")),
        ("STOP",  Some("Key1"),   Some("LeftTrigger2")),
        ("PLAY",  Some("Key2"),   Some("RightTrigger2")),
        ("START", Some("Enter"),  Some("Start")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// PC-FX defaults — d-pad + I/II/III/IV/V/VI + RUN + SELECT. **Z = I
/// (primary)**, **X = II (secondary)** per the cross-system rule.
/// PCFX 6-button face uses the SNES diamond + shoulder pattern: I/II
/// on East/South (bottom), III/IV on West/North (top of diamond),
/// V/VI on the shoulders. RUN on Enter / pad Start, SELECT on RShift /
/// pad Select — same as TG-16/PCE-CD convention.
pub fn default_pcfx_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",     Some("Up"),     Some("DPadUp")),
        ("DOWN",   Some("Down"),   Some("DPadDown")),
        ("LEFT",   Some("Left"),   Some("DPadLeft")),
        ("RIGHT",  Some("Right"),  Some("DPadRight")),
        ("I",      Some("Z"),      Some("East")),    // OA-internal bit 0 (libretro B)  — PRIMARY
        ("II",     Some("X"),      Some("South")),   // OA-internal bit 8 (libretro A)  — secondary
        ("III",    Some("A"),      Some("West")),    // OA-internal bit 12 (libretro L2)
        ("IV",     Some("S"),      Some("North")),   // OA-internal bit 13 (libretro R2)
        ("V",      Some("Q"),      Some("LeftTrigger")),  // OA-internal bit 10 (libretro L)
        ("VI",     Some("W"),      Some("RightTrigger")), // OA-internal bit 11 (libretro R)
        ("RUN",    Some("Enter"),  Some("Start")),
        ("SELECT", Some("RShift"), Some("Select")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// SNK Neo Geo defaults — 4-button arcade-style face + d-pad + START
/// + COIN. **Z = A (primary)**, **X = B (secondary)** per the
/// cross-system "Z is primary" rule. Neo Geo's A/B/C/D layout matches
/// SF/arcade fighter conventions where A is light attack, B is medium,
/// C is heavy, D is special. The keyboard layout follows the diamond
/// pattern Genesis uses for its 4 primary face buttons:
/// - A → Z / East (primary)
/// - B → X / South (secondary)
/// - C → A / West (tertiary, third attack button)
/// - D → S / North (quaternary, fourth attack button)
/// COIN binds to Key5 / Select per the MAME convention (5 = insert
/// coin P1) so operators with MAME muscle memory can play Neo Geo
/// arcade titles without remapping.
pub fn default_neogeo_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",    Some("Up"),     Some("DPadUp")),
        ("DOWN",  Some("Down"),   Some("DPadDown")),
        ("LEFT",  Some("Left"),   Some("DPadLeft")),
        ("RIGHT", Some("Right"),  Some("DPadRight")),
        ("A",     Some("Z"),      Some("East")),    // libretro B — Neo Geo A PRIMARY
        ("B",     Some("X"),      Some("South")),   // libretro A — Neo Geo B secondary
        ("C",     Some("A"),      Some("West")),    // libretro Y — Neo Geo C tertiary
        ("D",     Some("S"),      Some("North")),   // libretro X — Neo Geo D quaternary
        ("START", Some("Enter"),  Some("Start")),
        ("COIN",  Some("Key5"),   Some("Select")),  // MAME-convention "5 = insert coin"
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// SNK Neo Geo Pocket / Color defaults — d-pad + A + B + OPTION.
/// **Z = A (primary)**, **X = B (secondary)** per the cross-system
/// "Z is primary" rule. OPTION on Enter / pad Start — the NGP's
/// OPTION button serves as pause/menu, same role as Game Boy's
/// START or WonderSwan's START.
pub fn default_ngp_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",     Some("Up"),     Some("DPadUp")),
        ("DOWN",   Some("Down"),   Some("DPadDown")),
        ("LEFT",   Some("Left"),   Some("DPadLeft")),
        ("RIGHT",  Some("Right"),  Some("DPadRight")),
        ("A",      Some("Z"),      Some("East")),    // libretro B — primary
        ("B",      Some("X"),      Some("South")),   // libretro A — secondary
        ("OPTION", Some("Enter"),  Some("Start")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Saturn defaults — 6-button face in a 2x3 grid + L/R shoulders.
/// **Z = Saturn A (primary)**, **X = Saturn B (secondary)** per the
/// cross-system "Z is primary" rule.
///
/// The keyboard layout deliberately mirrors the Saturn's physical 2x3
/// face button grid, which lands on a clean QWERTY pattern:
///
/// ```text
/// Keyboard cluster:       Saturn pad face:
///   Q W                     L R         (shoulders)
///   A S D                   X Y Z       (top row)
///   Z X C                   A B C       (bottom row)
/// ```
///
/// This is the most ergonomic layout for Saturn fighter muscle memory
/// (Virtua Fighter / Fighters Megamix / Capcom-vs-SNK arcade ports all
/// use the 2x3 grid). Note: Saturn buttons named X, Y, Z map to
/// keyboard A, S, D respectively — the letter "X" on the Saturn pad is
/// the top-left face button, not the keyboard X key. Keyboard X is
/// bound to Saturn-button B (secondary). Pad: Saturn A on East
/// (cross-system primary), Saturn C and Z spilled to RightTrigger2 /
/// LeftTrigger2 per Beetle Saturn's libretro mapping (the diamond
/// doesn't hold 6 face buttons).
pub fn default_saturn_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",    Some("Up"),     Some("DPadUp")),
        ("DOWN",  Some("Down"),   Some("DPadDown")),
        ("LEFT",  Some("Left"),   Some("DPadLeft")),
        ("RIGHT", Some("Right"),  Some("DPadRight")),
        // Bottom-row face: A B C → keyboard Z X C (physical-position match).
        ("A",     Some("Z"),      Some("East")),          // libretro B  — Saturn A PRIMARY (cross-system rule)
        ("B",     Some("X"),      Some("South")),         // libretro A  — Saturn B secondary
        ("C",     Some("C"),      Some("RightTrigger2")), // libretro R2 — Saturn C (face button spilled to trigger slot)
        // Top-row face: X Y Z → keyboard A S D (physical-position match).
        ("X",     Some("A"),      Some("West")),          // libretro Y  — Saturn X (top-left)
        ("Y",     Some("S"),      Some("North")),         // libretro X  — Saturn Y (top-middle)
        ("Z",     Some("D"),      Some("LeftTrigger2")),  // libretro L2 — Saturn Z (top-right, spilled to trigger slot)
        // Shoulders: L R → keyboard Q W (above the home-row face cluster).
        ("L",     Some("Q"),      Some("LeftTrigger")),
        ("R",     Some("W"),      Some("RightTrigger")),
        ("START", Some("Enter"),  Some("Start")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Sony PlayStation digital DualPad defaults — d-pad + 4 face + L1/R1/L2/R2 +
/// START + SELECT. **Z = Cross (primary)**, **X = Circle (secondary)** per
/// the cross-system "Z is primary" rule. The PSX physical-layout convention
/// (Cross on south pad, Circle on east pad) is intentionally NOT followed
/// because OA's cross-system "primary on East" rule wins over period-correct
/// PSX muscle memory — operators with strong PSX muscle memory remap via
/// the per-system Bindings dialog. Front shoulders (L1/R1) on Q/W; rear
/// triggers (L2/R2) on E/R, the row above.
pub fn default_psx_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",       Some("Up"),     Some("DPadUp")),
        ("DOWN",     Some("Down"),   Some("DPadDown")),
        ("LEFT",     Some("Left"),   Some("DPadLeft")),
        ("RIGHT",    Some("Right"),  Some("DPadRight")),
        // Face diamond. CROSS primary on East via cross-system rule;
        // CIRCLE secondary on South; SQUARE west; TRIANGLE north.
        ("CROSS",    Some("Z"),      Some("East")),    // libretro B — PRIMARY
        ("CIRCLE",   Some("X"),      Some("South")),   // libretro A — secondary
        ("SQUARE",   Some("A"),      Some("West")),    // libretro Y — tertiary
        ("TRIANGLE", Some("S"),      Some("North")),   // libretro X — quaternary
        // Front shoulders + rear triggers.
        ("L1",       Some("Q"),      Some("LeftTrigger")),
        ("R1",       Some("W"),      Some("RightTrigger")),
        ("L2",       Some("E"),      Some("LeftTrigger2")),
        ("R2",       Some("R"),      Some("RightTrigger2")),
        ("START",    Some("Enter"),  Some("Start")),
        ("SELECT",   Some("RShift"), Some("Select")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// GCE Vectrex defaults — D-pad + 4 face buttons. **Z = B1 (primary)**,
/// **X = B2 (secondary)** per the cross-system "Z is primary" rule. B3
/// on A (keyboard) / West (pad) for tertiary; B4 on S (keyboard) /
/// North (pad) for quaternary — same convention as Genesis 6-button
/// (top row Q/S/W) and SNES diamond (Z/X/A/S).
pub fn default_vectrex_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",    Some("Up"),     Some("DPadUp")),
        ("DOWN",  Some("Down"),   Some("DPadDown")),
        ("LEFT",  Some("Left"),   Some("DPadLeft")),
        ("RIGHT", Some("Right"),  Some("DPadRight")),
        ("B1",    Some("Z"),      Some("East")),    // libretro B — primary
        ("B2",    Some("X"),      Some("South")),   // libretro A — secondary
        ("B3",    Some("A"),      Some("West")),    // libretro Y — tertiary
        ("B4",    Some("S"),      Some("North")),   // libretro X — quaternary
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Nintendo Virtual Boy defaults — LEFT D-pad + A + B + L + R + START +
/// SELECT. Same layout as Game Boy Advance + Game Boy: Z = A (primary),
/// X = B (secondary), Q/W = L/R shoulders, Enter = START, RShift =
/// SELECT.
pub fn default_virtualboy_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",     Some("Up"),     Some("DPadUp")),
        ("DOWN",   Some("Down"),   Some("DPadDown")),
        ("LEFT",   Some("Left"),   Some("DPadLeft")),
        ("RIGHT",  Some("Right"),  Some("DPadRight")),
        ("A",      Some("Z"),      Some("East")),
        ("B",      Some("X"),      Some("South")),
        ("L",      Some("Q"),      Some("LeftTrigger")),
        ("R",      Some("W"),      Some("RightTrigger")),
        ("START",  Some("Enter"),  Some("Start")),
        ("SELECT", Some("RShift"), Some("Select")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// WonderSwan defaults — D-pad + A + B + START. The dual-physical-D-pad
/// rotation is core-side per game header; from the bindings layer, a
/// single D-pad covers both orientations. The 8-direction layout works
/// for both horizontal-mode (X-pad active) and vertical-mode (Y-pad
/// active) games.
pub fn default_wonderswan_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",    Some("Up"),    Some("DPadUp")),
        ("DOWN",  Some("Down"),  Some("DPadDown")),
        ("LEFT",  Some("Left"),  Some("DPadLeft")),
        ("RIGHT", Some("Right"), Some("DPadRight")),
        ("A",     Some("Z"),     Some("East")),     // libretro A — primary
        ("B",     Some("X"),     Some("South")),    // libretro B — secondary
        ("START", Some("Enter"), Some("Start")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// ColecoVision defaults — 4 d-pad + 2 fires + 10 keypad numbers.
/// **Z = L_FIRE (yellow side button, primary)**, **X = R_FIRE (red,
/// secondary)** per the cross-system "Z is primary" rule. Keypad 1-9
/// land on the keyboard number row 1-9; 0 lands on number-row 0.
/// Gamepad fires on East / South; keypad numbers map to the remaining
/// face buttons + triggers + thumb-clicks (Y/X/L/R/L2/R2/L3/R3 + Start
/// + Select per blueMSX's libretro convention).
pub fn default_coleco_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",     Some("Up"),     Some("DPadUp")),
        ("DOWN",   Some("Down"),   Some("DPadDown")),
        ("LEFT",   Some("Left"),   Some("DPadLeft")),
        ("RIGHT",  Some("Right"),  Some("DPadRight")),
        ("L_FIRE", Some("Z"),      Some("East")),    // libretro B — yellow fire (primary)
        ("R_FIRE", Some("X"),      Some("South")),   // libretro A — red fire (secondary)
        ("KP1",    Some("Key1"),   Some("West")),    // libretro Y
        ("KP2",    Some("Key2"),   Some("North")),   // libretro X
        ("KP3",    Some("Key3"),   Some("LeftTrigger")),  // libretro L
        ("KP4",    Some("Key4"),   Some("RightTrigger")), // libretro R
        ("KP5",    Some("Key5"),   Some("LeftTrigger2")), // libretro L2
        ("KP6",    Some("Key6"),   Some("RightTrigger2")), // libretro R2
        ("KP7",    Some("Key7"),   Some("LeftThumb")), // libretro L3
        ("KP8",    Some("Key8"),   Some("RightThumb")), // libretro R3
        ("KP9",    Some("Key9"),   Some("Start")),    // libretro START
        ("KP0",    Some("Key0"),   Some("Select")),   // libretro SELECT
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Mattel Intellivision defaults — D-pad disc-as-8-way + 4 side action
/// buttons + START (keypad ENTER) + SELECT (keypad CLEAR). The 4 side
/// buttons split into upper (top corners — the most-used buttons for
/// most games) and lower (paired buttons each side; "fire" in most
/// shooters).
///
/// Keyboard: **Z = LOWER_L** (libretro B) — most games use the lower
/// side buttons for the primary action (Astrosmash fire, etc.), so
/// keyboard Z = primary fire matches the cross-system "Z is primary"
/// rule. **X = LOWER_R** (libretro A) — secondary fire. Upper buttons
/// land on Q/W per the shoulder convention.
pub fn default_intv_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",      Some("Up"),     Some("DPadUp")),
        ("DOWN",    Some("Down"),   Some("DPadDown")),
        ("LEFT",    Some("Left"),   Some("DPadLeft")),
        ("RIGHT",   Some("Right"),  Some("DPadRight")),
        ("LOWER_L", Some("Z"),      Some("East")),    // libretro B — lower-left side (primary)
        ("LOWER_R", Some("X"),      Some("South")),   // libretro A — lower-right side (secondary)
        ("UPPER_L", Some("Q"),      Some("LeftTrigger")),
        ("UPPER_R", Some("W"),      Some("RightTrigger")),
        ("START",   Some("Enter"),  Some("Start")),   // libretro START — keypad ENTER
        ("SELECT",  Some("RShift"), Some("Select")),  // libretro SELECT — keypad CLEAR
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Magnavox Odyssey² defaults — 4 d-pad + single ACTION button. The
/// O2 joins the 2600 as a second single-action-button system; **Z =
/// ACTION** matches the cross-system "Z is primary" rule.
///
/// Like the 2600, O2 doesn't appear in the
/// `z_is_the_primary_action_button_on_every_system` fixture (which
/// requires a secondary). The Z=ACTION assertion lives in
/// `defaults_cover_every_o2_button` instead.
pub fn default_o2_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",     Some("Up"),    Some("DPadUp")),
        ("DOWN",   Some("Down"),  Some("DPadDown")),
        ("LEFT",   Some("Left"),  Some("DPadLeft")),
        ("RIGHT",  Some("Right"), Some("DPadRight")),
        ("ACTION", Some("Z"),     Some("East")),    // libretro B — single action button (primary)
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Fairchild Channel F defaults — plunger controller (4-axis stick
/// mapped to D-pad) + FIRE (plunger push-in) + 4 console switches
/// (MODE / TIME / START / HOLD). **Z = FIRE** primary action.
/// Console-switch labels match the labels printed on the original
/// 1976 console.
pub fn default_channelf_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",    Some("Up"),     Some("DPadUp")),
        ("DOWN",  Some("Down"),   Some("DPadDown")),
        ("LEFT",  Some("Left"),   Some("DPadLeft")),
        ("RIGHT", Some("Right"),  Some("DPadRight")),
        ("FIRE",  Some("Z"),      Some("East")),    // libretro B — plunger push-in (primary)
        ("MODE",  Some("M"),      Some("North")),   // libretro Y — console MODE switch
        ("TIME",  Some("T"),      Some("Select")),  // libretro SELECT — Game Select
        ("START", Some("Enter"),  Some("Start")),   // libretro START — console START
        ("HOLD",  Some("H"),      Some("LeftTrigger")), // libretro L — Pause
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Atari 2600 defaults — single fire button. **Z = FIRE** per the
/// cross-system "Z is primary" rule. The 2600 is single-button so it
/// has no "secondary" — it doesn't appear in the
/// `z_is_the_primary_action_button_on_every_system` fixture (which
/// asserts BOTH primary and secondary land on Z and X); the single-
/// button assertion is covered implicitly by this defaults function
/// (Z = FIRE) plus the `defaults_cover_every_2600_button` test.
///
/// Enter binds to RESET (libretro START → Game Reset switch). The
/// 2600 had no "start" button on the controller — Game Reset is the
/// closest equivalent in the libretro mapping, and Stella treats it
/// as "begin / restart the game" so the operator's muscle memory
/// (Enter = start a game) carries.
pub fn default_atari2600_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",     Some("Up"),     Some("DPadUp")),
        ("DOWN",   Some("Down"),   Some("DPadDown")),
        ("LEFT",   Some("Left"),   Some("DPadLeft")),
        ("RIGHT",  Some("Right"),  Some("DPadRight")),
        ("FIRE",   Some("Z"),      Some("East")),    // libretro B — single fire button
        ("SELECT", Some("RShift"), Some("Select")),  // libretro SELECT — Game Select switch
        ("RESET",  Some("Enter"),  Some("Start")),   // libretro START — Game Reset switch
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Atari 5200 defaults — d-pad joystick + 2 side fire buttons + console
/// keypad buttons (START / PAUSE / RESET). **Z is primary fire**
/// (matches the cross-system "Z = primary" convention). Keypad digits
/// are not bound at Phase 0 — they live behind libretro KEYBOARD device
/// for Phase 2 polish (same approach as Jaguar's keypad).
pub fn default_atari5200_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",     Some("Up"),     Some("DPadUp")),
        ("DOWN",   Some("Down"),   Some("DPadDown")),
        ("LEFT",   Some("Left"),   Some("DPadLeft")),
        ("RIGHT",  Some("Right"),  Some("DPadRight")),
        ("FIRE1",  Some("Z"),      Some("East")),     // libretro B — bottom-side fire
        ("FIRE2",  Some("X"),      Some("South")),    // libretro A — top-side fire
        ("START",  Some("Enter"),  Some("Start")),    // libretro START — keypad START
        ("SELECT", Some("RShift"), Some("Select")),   // libretro SELECT — keypad PAUSE
        ("RESET",  Some("F4"),     Some("RightTrigger")), // libretro R — keypad RESET
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Pokémon Mini defaults — 8-button handheld layout. **Z = A (primary)**,
/// **X = B (secondary)** — matches the cross-system convention used by
/// GB / GBA / WonderSwan. C (Power / Menu) on RShift / SELECT. SHAKE
/// (the platform's quirky motion input) on Space / R shoulder so
/// Pokémon Pinball Mini paddle force + Party Mini dice rolls work.
pub fn default_pokemini_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",    Some("Up"),     Some("DPadUp")),
        ("DOWN",  Some("Down"),   Some("DPadDown")),
        ("LEFT",  Some("Left"),   Some("DPadLeft")),
        ("RIGHT", Some("Right"),  Some("DPadRight")),
        ("A",     Some("Z"),      Some("East")),
        ("B",     Some("X"),      Some("South")),
        ("C",     Some("RShift"), Some("Select")),  // Power / Menu
        ("SHAKE", Some("Space"),  Some("RightTrigger")), // shake sensor
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Game Boy Advance defaults — 10-button layout (GB-shape plus
/// shoulders). **Z = A (primary)**, **X = B (secondary)** per the
/// cross-system "Z is primary" rule (locked by the
/// `z_is_the_primary_action_button_on_every_system` test). L/R land
/// on Q/W per the SNES + Genesis 6-button convention; pad shoulders
/// route to LeftTrigger / RightTrigger.
pub fn default_gba_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",     Some("Up"),     Some("DPadUp")),
        ("DOWN",   Some("Down"),   Some("DPadDown")),
        ("LEFT",   Some("Left"),   Some("DPadLeft")),
        ("RIGHT",  Some("Right"),  Some("DPadRight")),
        ("A",      Some("Z"),      Some("East")),
        ("B",      Some("X"),      Some("South")),
        ("L",      Some("Q"),      Some("LeftTrigger")),
        ("R",      Some("W"),      Some("RightTrigger")),
        ("START",  Some("Enter"),  Some("Start")),
        ("SELECT", Some("RShift"), Some("Select")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Game Boy / Game Boy Color defaults — same shape as NES (4-way d-pad
/// + A + B + START + SELECT). **Z = A (primary)**, **X = B (secondary)**
/// per the cross-system "Z is primary" rule (locked by the
/// `z_is_the_primary_action_button_on_every_system` test). Identical
/// keybindings to NES — these two systems share a controller layout
/// and the muscle memory carries over directly.
pub fn default_gb_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",     Some("Up"),     Some("DPadUp")),
        ("DOWN",   Some("Down"),   Some("DPadDown")),
        ("LEFT",   Some("Left"),   Some("DPadLeft")),
        ("RIGHT",  Some("Right"),  Some("DPadRight")),
        ("A",      Some("Z"),      Some("East")),
        ("B",      Some("X"),      Some("South")),
        ("START",  Some("Enter"),  Some("Start")),
        ("SELECT", Some("RShift"), Some("Select")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Sega Master System defaults — 2-button face layout. **Z = Button 1
/// (primary)**, **X = Button 2 (secondary)** per the cross-system "Z is
/// primary" rule (locked by the `z_is_the_primary_action_button_on_every_system`
/// test). PAUSE on Enter / pad Start — the SMS hardware Pause was a
/// console button rather than a controller button, but Genesis Plus GX
/// maps it to libretro START so a single keybinding handles it.
pub fn default_sms_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",    Some("Up"),     Some("DPadUp")),
        ("DOWN",  Some("Down"),   Some("DPadDown")),
        ("LEFT",  Some("Left"),   Some("DPadLeft")),
        ("RIGHT", Some("Right"),  Some("DPadRight")),
        ("B1",    Some("Z"),      Some("East")),    // libretro B — Button 1 (primary)
        ("B2",    Some("X"),      Some("South")),   // libretro A — Button 2 (secondary)
        ("PAUSE", Some("Enter"),  Some("Start")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Sega Game Gear defaults — identical to SMS shape with the operator-
/// facing label "START" instead of "PAUSE" (the GG had a hardware Start
/// button on the unit; the SMS had a hardware Pause on the console).
pub fn default_gamegear_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",    Some("Up"),     Some("DPadUp")),
        ("DOWN",  Some("Down"),   Some("DPadDown")),
        ("LEFT",  Some("Left"),   Some("DPadLeft")),
        ("RIGHT", Some("Right"),  Some("DPadRight")),
        ("B1",    Some("Z"),      Some("East")),    // libretro B — Button 1 (primary)
        ("B2",    Some("X"),      Some("South")),   // libretro A — Button 2 (secondary)
        ("START", Some("Enter"),  Some("Start")),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// MAME defaults — six-button arcade fighter layout (Street Fighter II
/// punch/kick triplets). **Z = Button 1 (primary)**, **X = Button 2** to
/// match the project-wide PCE convention. SF veterans expect the punches
/// on the top row (B1 B2 B3 = LP MP HP — A, S, D) and kicks on the
/// bottom row (B4 B5 B6 = LK MK HK — Z, X, C); we deliberately depart
/// from that mapping in favor of the cross-system "Z is primary" rule
/// (per the test in this file). Users who want SF-native layouts can
/// remap via the per-system Bindings dialog.
///
/// COIN sits on `5` — RetroArch's standard "insert coin" key on the
/// keyboard side. START on `1` for player-1 start.
pub fn default_mame_bindings() -> Bindings {
    let mut b = Bindings::new();
    let pairs: &[(&str, Option<&str>, Option<&str>)] = &[
        ("UP",    Some("Up"),     Some("DPadUp")),
        ("DOWN",  Some("Down"),   Some("DPadDown")),
        ("LEFT",  Some("Left"),   Some("DPadLeft")),
        ("RIGHT", Some("Right"),  Some("DPadRight")),
        // Button 1-2: primary action duo (Z/X — cross-system convention).
        ("B1",    Some("Z"),      Some("South")),    // libretro B
        ("B2",    Some("X"),      Some("East")),     // libretro A
        // Button 3-4: secondary face buttons (top row of SF diamond).
        ("B3",    Some("A"),      Some("West")),     // libretro Y
        ("B4",    Some("S"),      Some("North")),    // libretro X
        // Button 5-6: shoulder buttons (SF heavy kicks / Capcom triplets).
        ("B5",    Some("Q"),      Some("LeftTrigger")),
        ("B6",    Some("W"),      Some("RightTrigger")),
        ("START",     Some("Key1"), Some("Start")),  // RetroArch standard: 1 = P1 Start
        ("COIN",      Some("Key5"), Some("Select")), // RetroArch standard: 5 = Insert Coin P1
        // Phase-1.5 system buttons. Keyboard mirrors RetroArch / MAME's
        // own muscle memory: 2 = P2 Start, 6 = P2 Coin, F2 = Service /
        // Test, Tab = MAME's per-driver input menu. Gamepad slots stay
        // unbound by default — these are keyboard-first system controls.
        ("P2_START",  Some("Key2"), None),
        ("P2_COIN",   Some("Key6"), None),
        ("SERVICE",   Some("F2"),   None),
        ("MAME_MENU", Some("Tab"),  None),
    ];
    for (name, kb, pad) in pairs {
        b.insert(
            (*name).into(),
            BindingPair {
                keyboard: kb.map(|s| s.to_string()),
                gamepad: pad.map(|s| s.to_string()),
            },
        );
    }
    b
}

/// Return the compiled-in defaults for a given system, or `None` if the system
/// has no registered defaults.
pub fn defaults_for(system_id: &str) -> Option<Bindings> {
    match system_id {
        "tg16" | "pce-cd" => Some(default_pce_bindings()),
        "lynx" => Some(default_lynx_bindings()),
        "nes" => Some(default_nes_bindings()),
        "snes" => Some(default_snes_bindings()),
        "mame" => Some(default_mame_bindings()),
        "atari7800" => Some(default_atari7800_bindings()),
        "genesis" | "segacd" | "sega32x" => Some(default_genesis_bindings()),
        "saturn" => Some(default_saturn_bindings()),
        "psx" => Some(default_psx_bindings()),
        "neogeo" | "neocd" => Some(default_neogeo_bindings()),
        "ngp" => Some(default_ngp_bindings()),
        "jaguar" => Some(default_jaguar_bindings()),
        "3do" => Some(default_threedo_bindings()),
        "pcfx" => Some(default_pcfx_bindings()),
        "n64" => Some(default_n64_bindings()),
        "gamecube" => Some(default_gamecube_bindings()),
        "dreamcast" => Some(default_dreamcast_bindings()),
        "psp" => Some(default_psp_bindings()),
        "ps2" => Some(default_ps2_bindings()),
        "nds" => Some(default_nds_bindings()),
        "sms" => Some(default_sms_bindings()),
        "gamegear" => Some(default_gamegear_bindings()),
        "gb" => Some(default_gb_bindings()),
        "gba" => Some(default_gba_bindings()),
        "2600" => Some(default_atari2600_bindings()),
        "5200" => Some(default_atari5200_bindings()),
        "pokemini" => Some(default_pokemini_bindings()),
        "coleco" => Some(default_coleco_bindings()),
        "intv" => Some(default_intv_bindings()),
        "o2" => Some(default_o2_bindings()),
        "channelf" => Some(default_channelf_bindings()),
        "vectrex" => Some(default_vectrex_bindings()),
        "virtualboy" => Some(default_virtualboy_bindings()),
        "wonderswan" => Some(default_wonderswan_bindings()),
        _ => None,
    }
}

/// Per-system declaration of which analog sticks the system's controller
/// uses. Drives the per-system Bindings page's Analog section — systems
/// returning `AnalogSticks::None` hide the analog UI entirely; systems
/// returning `AnalogSticks::Single` show one panel; `AnalogSticks::Dual`
/// shows two panels.
///
/// Friendly labels keep the UI period-correct: "Analog Stick" for N64,
/// "Main Stick" + "C-Stick" for GameCube, "Left Stick" + "Right Stick"
/// for DualShock-family systems, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalogSticks {
    /// No analog input on this system. UI hides the Analog section.
    None,
    /// One analog stick. The label is what shows in the UI panel
    /// header (e.g. "Analog Stick" for N64 / Dreamcast).
    Single { left_label: &'static str },
    /// Two analog sticks. Both labels show as panel headers.
    Dual { left_label: &'static str, right_label: &'static str },
}

pub fn analog_sticks_for(system_id: &str) -> AnalogSticks {
    match system_id {
        // Nintendo N64 — one analog stick, the iconic 3-prong controller.
        "n64" => AnalogSticks::Single { left_label: "Analog Stick" },
        // Nintendo GameCube — main stick + C-stick (genuinely analog,
        // not a 4-button cluster). Wii Classic Controller / Wavebird
        // share this shape.
        "gamecube" => AnalogSticks::Dual {
            left_label: "Main Stick",
            right_label: "C-Stick",
        },
        // Sega Dreamcast — single analog stick on the standard pad.
        // (The Twin-Stick controller for Virtual On has two but ships
        // as a per-game alternate, not the default.)
        "dreamcast" => AnalogSticks::Single { left_label: "Analog Stick" },
        // Sony PSP — single analog nub. The PSP Go added a similar one;
        // the right-side "analog" on the Vita is a separate slug.
        "psp" => AnalogSticks::Single { left_label: "Analog Nub" },
        // Sony PSX DualShock + PS2 DualShock 2 — left + right sticks.
        // Pressure-sensitive face buttons + analog L2/R2 are a separate
        // Phase 2.5 polish dimension (not surfaced here).
        "psx" | "ps2" => AnalogSticks::Dual {
            left_label: "Left Stick",
            right_label: "Right Stick",
        },
        // Sega Saturn 3D Pad — single analog stick + analog L/R triggers.
        // Default Saturn pad is digital-only; the 3D Pad is selected
        // via core options per-game (NiGHTS, Sega Rally, Panzer Dragoon
        // Saga). Surface the analog UI even on the digital default —
        // operators using 3D Pad mode need it.
        "saturn" => AnalogSticks::Single { left_label: "Analog Stick (3D Pad)" },
        // Everything else: no analog inputs.
        _ => AnalogSticks::None,
    }
}

fn bindings_path(app_data_dir: &Path, system_id: &str) -> PathBuf {
    app_data_dir.join("bindings").join(format!("{system_id}.json"))
}

/// Read bindings from disk, falling back to defaults if the file is missing or
/// malformed. Logs a warning on parse error so the user can investigate without
/// the app refusing to launch.
pub fn load(app_data_dir: &Path, system_id: &str) -> Bindings {
    let defaults = defaults_for(system_id).unwrap_or_default();
    let path = bindings_path(app_data_dir, system_id);
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return defaults,
    };
    match serde_json::from_str::<Bindings>(&raw) {
        Ok(mut parsed) => {
            // Backfill any buttons missing from the file with defaults so
            // adding a new button in code doesn't require deleting the file.
            for (name, pair) in defaults {
                parsed.entry(name).or_insert(pair);
            }
            parsed
        }
        Err(e) => {
            log::warn!(
                "oa-shell: bindings file {} malformed ({e}); using defaults",
                path.display()
            );
            defaults
        }
    }
}

/// Write bindings to disk. Creates the parent directory if needed.
pub fn save(app_data_dir: &Path, system_id: &str, bindings: &Bindings) -> std::io::Result<()> {
    let path = bindings_path(app_data_dir, system_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(bindings)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, body)
}

/// Resolve a stored keyboard-name string to a device_query Keycode.
pub fn keycode_from_name(name: &str) -> Option<Keycode> {
    Keycode::from_str(name).ok()
}

/// Resolve a stored gamepad-name string to a gilrs Button. The list mirrors
/// the variants we expect from a Standard Gamepad mapping plus the rare
/// `C`/`Z` / `LeftTrigger2`/`RightTrigger2` extras.
pub fn gamepad_from_name(name: &str) -> Option<GamepadButton> {
    Some(match name {
        "South" => GamepadButton::South,
        "East" => GamepadButton::East,
        "North" => GamepadButton::North,
        "West" => GamepadButton::West,
        "C" => GamepadButton::C,
        "Z" => GamepadButton::Z,
        "LeftTrigger" => GamepadButton::LeftTrigger,
        "LeftTrigger2" => GamepadButton::LeftTrigger2,
        "RightTrigger" => GamepadButton::RightTrigger,
        "RightTrigger2" => GamepadButton::RightTrigger2,
        "Select" => GamepadButton::Select,
        "Start" => GamepadButton::Start,
        "Mode" => GamepadButton::Mode,
        "LeftThumb" => GamepadButton::LeftThumb,
        "RightThumb" => GamepadButton::RightThumb,
        "DPadUp" => GamepadButton::DPadUp,
        "DPadDown" => GamepadButton::DPadDown,
        "DPadLeft" => GamepadButton::DPadLeft,
        "DPadRight" => GamepadButton::DPadRight,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_cover_every_pce_button() {
        let b = default_pce_bindings();
        for (name, _) in PCE_BUTTONS {
            assert!(b.contains_key(*name), "default missing: {name}");
        }
    }

    #[test]
    fn default_keys_round_trip_to_keycode() {
        // Cover every registered system's defaults — a new system that
        // ships a default keyboard name device_query doesn't recognize
        // would silently fail to bind without this check.
        for sys in &["tg16", "pce-cd", "lynx", "nes", "snes", "mame", "atari7800", "genesis", "segacd", "sega32x", "saturn", "psx", "neogeo", "neocd", "ngp", "jaguar", "3do", "pcfx", "n64", "gamecube", "dreamcast", "psp", "ps2", "nds", "sms", "gamegear", "gb", "gba", "2600", "coleco", "intv", "o2", "channelf", "vectrex", "virtualboy", "wonderswan", "5200", "pokemini"] {
            let bindings = defaults_for(sys).expect("defaults registered");
            for (button, pair) in &bindings {
                if let Some(name) = &pair.keyboard {
                    assert!(
                        keycode_from_name(name).is_some(),
                        "bad default key for {sys}/{button}: {name}",
                    );
                }
            }
        }
    }

    #[test]
    fn default_pads_round_trip_to_button() {
        for sys in &["tg16", "pce-cd", "lynx", "nes", "snes", "mame", "atari7800", "genesis", "segacd", "sega32x", "saturn", "psx", "neogeo", "neocd", "ngp", "jaguar", "3do", "pcfx", "n64", "gamecube", "dreamcast", "psp", "ps2", "nds", "sms", "gamegear", "gb", "gba", "2600", "coleco", "intv", "o2", "channelf", "vectrex", "virtualboy", "wonderswan", "5200", "pokemini"] {
            let bindings = defaults_for(sys).expect("defaults registered");
            for (button, pair) in &bindings {
                if let Some(name) = &pair.gamepad {
                    assert!(
                        gamepad_from_name(name).is_some(),
                        "bad default pad for {sys}/{button}: {name}",
                    );
                }
            }
        }
    }

    #[test]
    fn defaults_cover_every_lynx_button() {
        let b = default_lynx_bindings();
        for (name, _) in LYNX_BUTTONS {
            assert!(b.contains_key(*name), "lynx default missing: {name}");
        }
    }

    #[test]
    fn lynx_remap_is_identity() {
        // Lynx bits are laid out to match libretro bits — the remap should
        // be a no-op for every defined button. Locks the layout: if anyone
        // reorders `lynx::*` constants by accident, this test catches it.
        for (_, bit) in LYNX_BUTTONS {
            assert_eq!(
                lynx_to_libretro_bits(*bit), *bit,
                "lynx_to_libretro_bits should be identity for {:#x}", bit
            );
        }
        // Combined d-pad + A + B + START + SELECT + PAUSE.
        let all = lynx::UP | lynx::DOWN | lynx::LEFT | lynx::RIGHT
                | lynx::A | lynx::B | lynx::START | lynx::SELECT | lynx::PAUSE;
        assert_eq!(lynx_to_libretro_bits(all), all);
        // Stray high bits get masked off (no spurious libretro buttons).
        assert_eq!(lynx_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn to_libretro_bits_dispatches_by_system() {
        // tg16 → PCE remap path. PCE I bit (1<<0) should route to libretro
        // A (1<<8), not pass through unchanged.
        assert_eq!(to_libretro_bits("tg16", pce::I), 1 << 8);
        // pce-cd shares the PCE controller — same remap, same defaults,
        // same buttons. If the dispatch arm regresses to identity, CD
        // input dies silently.
        assert_eq!(to_libretro_bits("pce-cd", pce::I), 1 << 8);
        assert_eq!(to_libretro_bits("pce-cd", pce::II), 1 << 0);
        assert_eq!(to_libretro_bits("pce-cd", pce::RUN), 1 << 3);
        assert_eq!(buttons_for("pce-cd").len(), PCE_BUTTONS.len());
        assert!(defaults_for("pce-cd").is_some());
        // lynx → identity. libretro A bit (1<<8) stays at 1<<8.
        assert_eq!(to_libretro_bits("lynx", lynx::A), lynx::A);
        // nes / snes → identity (libretro-aligned by construction).
        assert_eq!(to_libretro_bits("nes", nes::A), nes::A);
        assert_eq!(to_libretro_bits("nes", nes::B), nes::B);
        assert_eq!(to_libretro_bits("snes", snes::X), snes::X);
        assert_eq!(to_libretro_bits("snes", snes::L), snes::L);
        // sms / gamegear → identity (libretro-aligned by construction).
        assert_eq!(to_libretro_bits("sms", sms::B1), sms::B1);
        assert_eq!(to_libretro_bits("sms", sms::PAUSE), sms::PAUSE);
        assert_eq!(to_libretro_bits("gamegear", gamegear::B2), gamegear::B2);
        assert_eq!(to_libretro_bits("gamegear", gamegear::START), gamegear::START);
        // gb → identity (libretro-aligned by construction).
        assert_eq!(to_libretro_bits("gb", gb::A), gb::A);
        assert_eq!(to_libretro_bits("gb", gb::START), gb::START);
        // gba → identity (libretro-aligned by construction).
        assert_eq!(to_libretro_bits("gba", gba::A), gba::A);
        assert_eq!(to_libretro_bits("gba", gba::L), gba::L);
        assert_eq!(to_libretro_bits("gba", gba::R), gba::R);
        // 2600 → identity (libretro-aligned by construction).
        assert_eq!(to_libretro_bits("2600", atari2600::FIRE), atari2600::FIRE);
        assert_eq!(to_libretro_bits("2600", atari2600::RESET), atari2600::RESET);
        // coleco / intv / o2 / channelf → identity.
        assert_eq!(to_libretro_bits("coleco", coleco::L_FIRE), coleco::L_FIRE);
        assert_eq!(to_libretro_bits("coleco", coleco::KP5), coleco::KP5);
        assert_eq!(to_libretro_bits("intv", intv::LOWER_L), intv::LOWER_L);
        assert_eq!(to_libretro_bits("intv", intv::UPPER_R), intv::UPPER_R);
        assert_eq!(to_libretro_bits("o2", o2::ACTION), o2::ACTION);
        assert_eq!(to_libretro_bits("channelf", channelf::FIRE), channelf::FIRE);
        assert_eq!(to_libretro_bits("channelf", channelf::HOLD), channelf::HOLD);
        // vectrex / virtualboy / wonderswan → identity.
        assert_eq!(to_libretro_bits("vectrex", vectrex::B1), vectrex::B1);
        assert_eq!(to_libretro_bits("vectrex", vectrex::B4), vectrex::B4);
        assert_eq!(to_libretro_bits("virtualboy", virtualboy::A), virtualboy::A);
        assert_eq!(to_libretro_bits("virtualboy", virtualboy::L), virtualboy::L);
        assert_eq!(to_libretro_bits("wonderswan", wonderswan::A), wonderswan::A);
        assert_eq!(to_libretro_bits("wonderswan", wonderswan::START), wonderswan::START);
        // Unknown system → identity (defensive default).
        assert_eq!(to_libretro_bits("unknown", 0x42), 0x42);
    }

    #[test]
    fn defaults_cover_every_nes_button() {
        let b = default_nes_bindings();
        for (name, _) in NES_BUTTONS {
            assert!(b.contains_key(*name), "nes default missing: {name}");
        }
    }

    #[test]
    fn defaults_cover_every_snes_button() {
        let b = default_snes_bindings();
        for (name, _) in SNES_BUTTONS {
            assert!(b.contains_key(*name), "snes default missing: {name}");
        }
    }

    #[test]
    fn defaults_cover_every_mame_button() {
        let b = default_mame_bindings();
        for (name, _) in MAME_BUTTONS {
            assert!(b.contains_key(*name), "mame default missing: {name}");
        }
    }

    #[test]
    fn mame_remap_is_identity() {
        for (_, bit) in MAME_BUTTONS {
            assert_eq!(mame_to_libretro_bits(*bit), *bit);
        }
        let all = mame::B1 | mame::B2 | mame::B3 | mame::B4 | mame::B5 | mame::B6
                | mame::COIN | mame::START
                | mame::UP | mame::DOWN | mame::LEFT | mame::RIGHT;
        assert_eq!(mame_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn dpad_lands_on_correct_libretro_bits_for_every_system() {
        // Regression-lock for the bug where switching from PCE to
        // NES/SNES/Lynx left the InputPoller pointed at PCE's clockwise
        // d-pad bit layout (UP=4, RIGHT=5, DOWN=6, LEFT=7) while the new
        // system's identity remap read those bits as libretro's straight
        // order (UP=4, DOWN=5, LEFT=6, RIGHT=7) — so pressing down-arrow
        // on an NES game flipped libretro bit 6, which the core read as
        // LEFT instead of DOWN.
        //
        // The shell fix re-applies bindings on system swap. This test
        // catches the pre-remap bit-layout half: for every system, the
        // bit returned by `bit_for(sys, "UP")` after `to_libretro_bits`
        // is libretro UP (4), DOWN → 5, LEFT → 6, RIGHT → 7. If a future
        // system ships a swapped d-pad layout without matching its remap,
        // this test trips before it ships.
        const LIBRETRO_UP: u32    = 1 << 4;
        const LIBRETRO_DOWN: u32  = 1 << 5;
        const LIBRETRO_LEFT: u32  = 1 << 6;
        const LIBRETRO_RIGHT: u32 = 1 << 7;
        for sys in &["tg16", "pce-cd", "lynx", "nes", "snes", "mame", "atari7800", "genesis", "segacd", "sega32x", "saturn", "psx", "neogeo", "neocd", "ngp", "jaguar", "3do", "pcfx", "n64", "gamecube", "dreamcast", "psp", "ps2", "nds", "sms", "gamegear", "gb", "gba", "2600", "coleco", "intv", "o2", "channelf", "vectrex", "virtualboy", "wonderswan", "5200", "pokemini"] {
            let up    = bit_for(sys, "UP").expect("UP bit registered");
            let down  = bit_for(sys, "DOWN").expect("DOWN bit registered");
            let left  = bit_for(sys, "LEFT").expect("LEFT bit registered");
            let right = bit_for(sys, "RIGHT").expect("RIGHT bit registered");
            assert_eq!(to_libretro_bits(sys, up),    LIBRETRO_UP,    "{sys}: UP -> libretro UP");
            assert_eq!(to_libretro_bits(sys, down),  LIBRETRO_DOWN,  "{sys}: DOWN -> libretro DOWN");
            assert_eq!(to_libretro_bits(sys, left),  LIBRETRO_LEFT,  "{sys}: LEFT -> libretro LEFT");
            assert_eq!(to_libretro_bits(sys, right), LIBRETRO_RIGHT, "{sys}: RIGHT -> libretro RIGHT");
        }
    }

    #[test]
    fn z_is_the_primary_action_button_on_every_system() {
        // Locks the cross-system muscle-memory convention: pressing Z on
        // the keyboard should always trigger the system's primary action
        // (libretro A bit, which is bit 8 after the per-system remap).
        // Pressing X is always the secondary action (libretro B, bit 0).
        // PCE established this in Phase 1; if a future system arrives with
        // its defaults swapped, this test catches it.
        //
        // ⚠ Exceptions — systems that legitimately don't fit this fixture:
        //   - "2600" (Atari VCS) is single-button. There IS no secondary
        //     action button, so the test omits it; the Z=FIRE half is
        //     covered explicitly by an assertion inside
        //     `defaults_cover_every_2600_button`.
        //   - "o2" (Magnavox Odyssey²) is single-button — same shape.
        //     Z=ACTION is asserted inside `defaults_cover_every_o2_button`.
        //   - "channelf" (Fairchild Channel F) has FIRE as the only game-
        //     action button; MODE / TIME / START / HOLD are CONSOLE
        //     switches with hardware-label keyboards (M, T, Enter, H),
        //     not game-action secondaries. Z=FIRE is asserted inside
        //     `defaults_cover_every_channelf_button`.
        for (sys, primary_name, secondary_name) in &[
            ("tg16", "I", "II"),
            ("pce-cd", "I", "II"),
            ("lynx", "A", "B"),
            ("nes", "A", "B"),
            ("snes", "A", "B"),
            ("mame", "B1", "B2"),
            ("atari7800", "B1", "B2"),
            ("genesis", "B", "C"),
            // Sega CD + 32X share the 6-button MD controller — same
            // primary/secondary as genesis.
            ("segacd", "B", "C"),
            ("sega32x", "B", "C"),
            // Saturn 6-button face — A is libretro B bit 0 (primary),
            // B is libretro A bit 8 (secondary). Z keyboard → Saturn A,
            // X keyboard → Saturn B.
            ("saturn", "A", "B"),
            // PSX digital DualPad — CROSS is libretro B bit 0 (primary
            // in Western releases), CIRCLE is libretro A bit 8
            // (secondary). Z keyboard → Cross, X keyboard → Circle.
            ("psx", "CROSS", "CIRCLE"),
            // Neo Geo 4-button arcade face — A is libretro B bit 0
            // (primary attack), B is libretro A bit 8 (secondary).
            // Same layout for cart AES (neogeo) and CD (neocd).
            ("neogeo", "A", "B"),
            ("neocd", "A", "B"),
            // Neo Geo Pocket / Color — A is libretro B bit 0 (primary),
            // B is libretro A bit 8 (secondary).
            ("ngp", "A", "B"),
            // Atari Jaguar — A is libretro B bit 0 (primary), B is
            // libretro A bit 8 (secondary).
            ("jaguar", "A", "B"),
            // 3DO — A is libretro B bit 0 (primary), B is libretro A
            // bit 8 (secondary).
            ("3do", "A", "B"),
            // PC-FX — I is libretro B bit 0 (primary), II is libretro
            // A bit 8 (secondary).
            ("pcfx", "I", "II"),
            // N64 — A is libretro B bit 0 (primary), B is libretro Y
            // bit 1 (secondary). Z keyboard → N64 A, X keyboard → N64 B.
            ("n64", "A", "B"),
            // GameCube — A is libretro B bit 0 (primary), B is libretro
            // Y bit 1 (secondary).
            ("gamecube", "A", "B"),
            // Dreamcast — A is libretro B bit 0 (primary), B is
            // libretro A bit 8 (secondary).
            ("dreamcast", "A", "B"),
            // PSP — CROSS is libretro B bit 0 (primary, Western
            // convention), CIRCLE is libretro A bit 8 (secondary).
            ("psp", "CROSS", "CIRCLE"),
            // PS2 (DualShock 2) — same shape as PSX.
            ("ps2", "CROSS", "CIRCLE"),
            // NDS — A is libretro A bit 8 (Nintendo east face PRIMARY,
            // matches nes/snes/gb/gba precedent), B is libretro B bit 0
            // (south face secondary).
            ("nds", "A", "B"),
            ("sms", "B1", "B2"),
            ("gamegear", "B1", "B2"),
            ("gb", "A", "B"),
            ("gba", "A", "B"),
            ("coleco", "L_FIRE", "R_FIRE"),
            ("intv", "LOWER_L", "LOWER_R"),
            ("vectrex", "B1", "B2"),
            ("virtualboy", "A", "B"),
            ("wonderswan", "A", "B"),
        ] {
            let bindings = defaults_for(sys).expect("defaults registered");
            let primary = bindings.get(*primary_name).expect("primary button present");
            let secondary = bindings.get(*secondary_name).expect("secondary button present");
            assert_eq!(
                primary.keyboard.as_deref(), Some("Z"),
                "{sys}: primary action ({primary_name}) must be on Z",
            );
            assert_eq!(
                secondary.keyboard.as_deref(), Some("X"),
                "{sys}: secondary action ({secondary_name}) must be on X",
            );
        }
    }

    #[test]
    fn nes_and_snes_remap_is_identity() {
        for (_, bit) in NES_BUTTONS {
            assert_eq!(nes_to_libretro_bits(*bit), *bit);
        }
        for (_, bit) in SNES_BUTTONS {
            assert_eq!(snes_to_libretro_bits(*bit), *bit);
        }
        // High-bit garbage gets masked off.
        let all_nes = nes::A | nes::B | nes::SELECT | nes::START | nes::UP | nes::DOWN | nes::LEFT | nes::RIGHT;
        assert_eq!(nes_to_libretro_bits(all_nes | (1 << 20)), all_nes);
        let all_snes = snes::A | snes::B | snes::X | snes::Y | snes::L | snes::R
                     | snes::SELECT | snes::START
                     | snes::UP | snes::DOWN | snes::LEFT | snes::RIGHT;
        assert_eq!(snes_to_libretro_bits(all_snes | (1 << 20)), all_snes);
    }

    #[test]
    fn defaults_cover_every_atari7800_button() {
        let b = default_atari7800_bindings();
        for (name, _) in ATARI7800_BUTTONS {
            assert!(b.contains_key(*name), "atari7800 default missing: {name}");
        }
    }

    #[test]
    fn atari7800_remap_is_identity() {
        for (_, bit) in ATARI7800_BUTTONS {
            assert_eq!(atari7800_to_libretro_bits(*bit), *bit);
        }
        let all = atari7800::B1 | atari7800::B2 | atari7800::SELECT | atari7800::PAUSE
                | atari7800::UP | atari7800::DOWN | atari7800::LEFT | atari7800::RIGHT;
        assert_eq!(atari7800_to_libretro_bits(all), all);
        // Stray high bits get masked off.
        assert_eq!(atari7800_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn defaults_cover_every_genesis_button() {
        let b = default_genesis_bindings();
        for (name, _) in GENESIS_BUTTONS {
            assert!(b.contains_key(*name), "genesis default missing: {name}");
        }
    }

    #[test]
    fn genesis_remap_is_identity() {
        for (_, bit) in GENESIS_BUTTONS {
            assert_eq!(genesis_to_libretro_bits(*bit), *bit);
        }
        let all = genesis::A | genesis::B | genesis::C
                | genesis::X | genesis::Y | genesis::Z
                | genesis::START | genesis::MODE
                | genesis::UP | genesis::DOWN | genesis::LEFT | genesis::RIGHT;
        assert_eq!(genesis_to_libretro_bits(all), all);
        // Stray high bits get masked off.
        assert_eq!(genesis_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn genesis_dispatch_round_trips() {
        // Sanity-check the per-system dispatch: bit_for / buttons_for /
        // to_libretro_bits / defaults_for all return populated values for
        // "genesis". A missed dispatch arm would silently fall through
        // to "unknown" defaults and the operator would see empty bindings.
        assert!(buttons_for("genesis").len() == GENESIS_BUTTONS.len());
        assert!(defaults_for("genesis").is_some());
        assert_eq!(bit_for("genesis", "B"), Some(genesis::B));
        assert_eq!(bit_for("genesis", "MODE"), Some(genesis::MODE));
        assert_eq!(bit_for("genesis", "Z"), Some(genesis::Z));
        assert_eq!(to_libretro_bits("genesis", genesis::B), genesis::B);
    }

    #[test]
    fn defaults_cover_every_segacd_button() {
        // Sega CD shares the 6-button MD controller via the genesis
        // dispatch arm — defaults_for("segacd") must return the same
        // 12-entry binding set genesis exposes. A regressed dispatch arm
        // (silent fall-through to None) would surface as the per-system
        // Bindings page rendering an empty list for Sega CD.
        let b = defaults_for("segacd").expect("segacd defaults registered");
        for (name, _) in GENESIS_BUTTONS {
            assert!(b.contains_key(*name), "segacd default missing: {name}");
        }
    }

    #[test]
    fn segacd_remap_is_identity() {
        // Sega CD's controller IS the 6-button Mega Drive pad — the
        // remap routes through genesis_to_libretro_bits which is identity
        // by construction. Mask trims stray high bits, locks the layout.
        for (_, bit) in GENESIS_BUTTONS {
            assert_eq!(to_libretro_bits("segacd", *bit), *bit);
        }
        let all = genesis::A | genesis::B | genesis::C
                | genesis::X | genesis::Y | genesis::Z
                | genesis::START | genesis::MODE
                | genesis::UP | genesis::DOWN | genesis::LEFT | genesis::RIGHT;
        assert_eq!(to_libretro_bits("segacd", all), all);
        assert_eq!(to_libretro_bits("segacd", all | (1 << 20)), all);
    }

    #[test]
    fn segacd_dispatch_round_trips() {
        // Lock the 4 dispatch arms (bit_for / buttons_for /
        // to_libretro_bits / defaults_for) for "segacd". Same shape as
        // genesis_dispatch_round_trips — catches a regression where
        // someone splits the genesis dispatch back into per-arm matches
        // and forgets to include segacd.
        assert!(buttons_for("segacd").len() == GENESIS_BUTTONS.len());
        assert!(defaults_for("segacd").is_some());
        assert_eq!(bit_for("segacd", "B"), Some(genesis::B));
        assert_eq!(bit_for("segacd", "MODE"), Some(genesis::MODE));
        assert_eq!(bit_for("segacd", "Z"), Some(genesis::Z));
        assert_eq!(to_libretro_bits("segacd", genesis::B), genesis::B);
    }

    #[test]
    fn defaults_cover_every_sega32x_button() {
        // Sega 32X also shares the 6-button MD controller — cart-shape
        // addon, same pad. Same defaults verification as segacd.
        let b = defaults_for("sega32x").expect("sega32x defaults registered");
        for (name, _) in GENESIS_BUTTONS {
            assert!(b.contains_key(*name), "sega32x default missing: {name}");
        }
    }

    #[test]
    fn sega32x_remap_is_identity() {
        // 32X via PicoDrive uses the standard MD RetroPad layout; remap
        // is identity through the genesis_to_libretro_bits path.
        for (_, bit) in GENESIS_BUTTONS {
            assert_eq!(to_libretro_bits("sega32x", *bit), *bit);
        }
        let all = genesis::A | genesis::B | genesis::C
                | genesis::X | genesis::Y | genesis::Z
                | genesis::START | genesis::MODE
                | genesis::UP | genesis::DOWN | genesis::LEFT | genesis::RIGHT;
        assert_eq!(to_libretro_bits("sega32x", all), all);
        assert_eq!(to_libretro_bits("sega32x", all | (1 << 20)), all);
    }

    #[test]
    fn sega32x_dispatch_round_trips() {
        // Lock the 4 dispatch arms for "sega32x".
        assert!(buttons_for("sega32x").len() == GENESIS_BUTTONS.len());
        assert!(defaults_for("sega32x").is_some());
        assert_eq!(bit_for("sega32x", "B"), Some(genesis::B));
        assert_eq!(bit_for("sega32x", "MODE"), Some(genesis::MODE));
        assert_eq!(bit_for("sega32x", "Z"), Some(genesis::Z));
        assert_eq!(to_libretro_bits("sega32x", genesis::B), genesis::B);
    }

    #[test]
    fn defaults_cover_every_saturn_button() {
        let b = default_saturn_bindings();
        for (name, _) in SATURN_BUTTONS {
            assert!(b.contains_key(*name), "saturn default missing: {name}");
        }
    }

    #[test]
    fn saturn_remap_is_identity() {
        // Saturn bits are laid out to match libretro RetroPad bits; the
        // remap is identity. Mask trims stray high bits. Saturn C and Z
        // legitimately live in L2/R2 slots since the diamond's 4 face
        // slots can't hold 6 face buttons.
        for (_, bit) in SATURN_BUTTONS {
            assert_eq!(saturn_to_libretro_bits(*bit), *bit);
        }
        let all = saturn::A | saturn::B | saturn::C
                | saturn::X | saturn::Y | saturn::Z
                | saturn::L | saturn::R | saturn::START
                | saturn::UP | saturn::DOWN | saturn::LEFT | saturn::RIGHT;
        assert_eq!(saturn_to_libretro_bits(all), all);
        assert_eq!(saturn_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn saturn_dispatch_round_trips() {
        // Lock the 4 dispatch arms for "saturn". A missed arm would
        // silently fall through to "unknown" defaults and the operator
        // would see empty bindings.
        assert!(buttons_for("saturn").len() == SATURN_BUTTONS.len());
        assert!(defaults_for("saturn").is_some());
        assert_eq!(bit_for("saturn", "A"), Some(saturn::A));
        assert_eq!(bit_for("saturn", "C"), Some(saturn::C));
        assert_eq!(bit_for("saturn", "Z"), Some(saturn::Z));
        assert_eq!(to_libretro_bits("saturn", saturn::A), saturn::A);
    }

    #[test]
    fn defaults_cover_every_psx_button() {
        let b = default_psx_bindings();
        for (name, _) in PSX_BUTTONS {
            assert!(b.contains_key(*name), "psx default missing: {name}");
        }
    }

    #[test]
    fn psx_remap_is_identity() {
        // PSX bits are laid out to match libretro RetroPad bits — the
        // remap is identity. Mask trims stray high bits. DualShock
        // analog sticks (libretro analog axes, not RetroPad bits) ship
        // as Phase 2 work alongside shared analog-input infra.
        for (_, bit) in PSX_BUTTONS {
            assert_eq!(psx_to_libretro_bits(*bit), *bit);
        }
        let all = psx::CROSS | psx::CIRCLE | psx::TRIANGLE | psx::SQUARE
                | psx::L1 | psx::R1 | psx::L2 | psx::R2
                | psx::START | psx::SELECT
                | psx::UP | psx::DOWN | psx::LEFT | psx::RIGHT;
        assert_eq!(psx_to_libretro_bits(all), all);
        assert_eq!(psx_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn psx_dispatch_round_trips() {
        // Lock the 4 dispatch arms for "psx".
        assert!(buttons_for("psx").len() == PSX_BUTTONS.len());
        assert!(defaults_for("psx").is_some());
        assert_eq!(bit_for("psx", "CROSS"), Some(psx::CROSS));
        assert_eq!(bit_for("psx", "L2"), Some(psx::L2));
        assert_eq!(bit_for("psx", "SELECT"), Some(psx::SELECT));
        assert_eq!(to_libretro_bits("psx", psx::CROSS), psx::CROSS);
    }

    #[test]
    fn defaults_cover_every_neogeo_button() {
        let b = default_neogeo_bindings();
        for (name, _) in NEOGEO_BUTTONS {
            assert!(b.contains_key(*name), "neogeo default missing: {name}");
        }
    }

    #[test]
    fn neogeo_remap_is_identity() {
        // Neo Geo bits laid out as libretro RetroPad positions —
        // identity remap. Mask trims stray high bits.
        for (_, bit) in NEOGEO_BUTTONS {
            assert_eq!(neogeo_to_libretro_bits(*bit), *bit);
        }
        let all = neogeo::A | neogeo::B | neogeo::C | neogeo::D
                | neogeo::START | neogeo::COIN
                | neogeo::UP | neogeo::DOWN | neogeo::LEFT | neogeo::RIGHT;
        assert_eq!(neogeo_to_libretro_bits(all), all);
        assert_eq!(neogeo_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn neogeo_dispatch_round_trips() {
        // Lock the 4 dispatch arms for "neogeo". Also verifies the
        // "neocd" alias path dispatches through the same neogeo_*
        // functions (same precedent PCE-CD set with TG-16).
        assert!(buttons_for("neogeo").len() == NEOGEO_BUTTONS.len());
        assert!(defaults_for("neogeo").is_some());
        assert_eq!(bit_for("neogeo", "A"), Some(neogeo::A));
        assert_eq!(bit_for("neogeo", "D"), Some(neogeo::D));
        assert_eq!(bit_for("neogeo", "COIN"), Some(neogeo::COIN));
        assert_eq!(to_libretro_bits("neogeo", neogeo::A), neogeo::A);
        // Neo Geo CD shares the cart controller.
        assert!(buttons_for("neocd").len() == NEOGEO_BUTTONS.len());
        assert!(defaults_for("neocd").is_some());
        assert_eq!(bit_for("neocd", "A"), Some(neogeo::A));
        assert_eq!(to_libretro_bits("neocd", neogeo::A), neogeo::A);
    }

    #[test]
    fn defaults_cover_every_ngp_button() {
        let b = default_ngp_bindings();
        for (name, _) in NGP_BUTTONS {
            assert!(b.contains_key(*name), "ngp default missing: {name}");
        }
    }

    #[test]
    fn ngp_remap_is_identity() {
        for (_, bit) in NGP_BUTTONS {
            assert_eq!(ngp_to_libretro_bits(*bit), *bit);
        }
        let all = ngp::A | ngp::B | ngp::OPTION
                | ngp::UP | ngp::DOWN | ngp::LEFT | ngp::RIGHT;
        assert_eq!(ngp_to_libretro_bits(all), all);
        assert_eq!(ngp_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn ngp_dispatch_round_trips() {
        assert!(buttons_for("ngp").len() == NGP_BUTTONS.len());
        assert!(defaults_for("ngp").is_some());
        assert_eq!(bit_for("ngp", "A"), Some(ngp::A));
        assert_eq!(bit_for("ngp", "OPTION"), Some(ngp::OPTION));
        assert_eq!(to_libretro_bits("ngp", ngp::A), ngp::A);
    }

    #[test]
    fn defaults_cover_every_jaguar_button() {
        let b = default_jaguar_bindings();
        for (name, _) in JAGUAR_BUTTONS {
            assert!(b.contains_key(*name), "jaguar default missing: {name}");
        }
    }

    #[test]
    fn jaguar_remap_drops_high_bits() {
        // Core buttons + KP1-KP7 are RetroPad bits — identity remap.
        let core = jaguar::A | jaguar::B | jaguar::C
                 | jaguar::OPTION | jaguar::PAUSE
                 | jaguar::UP | jaguar::DOWN | jaguar::LEFT | jaguar::RIGHT
                 | jaguar::KP1 | jaguar::KP2 | jaguar::KP3 | jaguar::KP4
                 | jaguar::KP5 | jaguar::KP6 | jaguar::KP7;
        assert_eq!(jaguar_to_libretro_bits(core), core);
        // KP8-KP_HASH live above the 16-bit RetroPad range — must
        // get masked off so the core only sees RetroPad bits. Phase 2
        // polish will route them through keyboard-passthrough instead.
        let high_bit_keypad = jaguar::KP8 | jaguar::KP9
                            | jaguar::KP_STAR | jaguar::KP0 | jaguar::KP_HASH;
        assert_eq!(jaguar_to_libretro_bits(high_bit_keypad), 0);
        // Combined: core stays, high bits drop.
        assert_eq!(jaguar_to_libretro_bits(core | high_bit_keypad), core);
        // Stray non-Jaguar high bits also get masked.
        assert_eq!(jaguar_to_libretro_bits(core | (1 << 25)), core);
    }

    #[test]
    fn jaguar_dispatch_round_trips() {
        assert!(buttons_for("jaguar").len() == JAGUAR_BUTTONS.len());
        assert!(defaults_for("jaguar").is_some());
        assert_eq!(bit_for("jaguar", "A"), Some(jaguar::A));
        assert_eq!(bit_for("jaguar", "PAUSE"), Some(jaguar::PAUSE));
        assert_eq!(bit_for("jaguar", "KP1"), Some(jaguar::KP1));
        assert_eq!(bit_for("jaguar", "KP_HASH"), Some(jaguar::KP_HASH));
        assert_eq!(to_libretro_bits("jaguar", jaguar::A), jaguar::A);
    }

    #[test]
    fn defaults_cover_every_3do_button() {
        let b = default_threedo_bindings();
        for (name, _) in THREEDO_BUTTONS {
            assert!(b.contains_key(*name), "3do default missing: {name}");
        }
    }

    #[test]
    fn threedo_remap_is_identity() {
        for (_, bit) in THREEDO_BUTTONS {
            assert_eq!(threedo_to_libretro_bits(*bit), *bit);
        }
        let all = threedo::A | threedo::B | threedo::C
                | threedo::L | threedo::R | threedo::START
                | threedo::STOP | threedo::PLAY
                | threedo::UP | threedo::DOWN | threedo::LEFT | threedo::RIGHT;
        assert_eq!(threedo_to_libretro_bits(all), all);
        assert_eq!(threedo_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn threedo_dispatch_round_trips() {
        assert!(buttons_for("3do").len() == THREEDO_BUTTONS.len());
        assert!(defaults_for("3do").is_some());
        assert_eq!(bit_for("3do", "A"), Some(threedo::A));
        assert_eq!(bit_for("3do", "STOP"), Some(threedo::STOP));
        assert_eq!(bit_for("3do", "PLAY"), Some(threedo::PLAY));
        assert_eq!(to_libretro_bits("3do", threedo::A), threedo::A);
    }

    #[test]
    fn defaults_cover_every_pcfx_button() {
        let b = default_pcfx_bindings();
        for (name, _) in PCFX_BUTTONS {
            assert!(b.contains_key(*name), "pcfx default missing: {name}");
        }
    }

    #[test]
    fn pcfx_remap_is_identity() {
        for (_, bit) in PCFX_BUTTONS {
            assert_eq!(pcfx_to_libretro_bits(*bit), *bit);
        }
        let all = pcfx::I | pcfx::II | pcfx::III | pcfx::IV | pcfx::V | pcfx::VI
                | pcfx::RUN | pcfx::SELECT
                | pcfx::UP | pcfx::DOWN | pcfx::LEFT | pcfx::RIGHT;
        assert_eq!(pcfx_to_libretro_bits(all), all);
        assert_eq!(pcfx_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn pcfx_dispatch_round_trips() {
        assert!(buttons_for("pcfx").len() == PCFX_BUTTONS.len());
        assert!(defaults_for("pcfx").is_some());
        assert_eq!(bit_for("pcfx", "I"), Some(pcfx::I));
        assert_eq!(bit_for("pcfx", "VI"), Some(pcfx::VI));
        assert_eq!(bit_for("pcfx", "SELECT"), Some(pcfx::SELECT));
        assert_eq!(to_libretro_bits("pcfx", pcfx::I), pcfx::I);
    }

    #[test]
    fn defaults_cover_every_n64_button() {
        let b = default_n64_bindings();
        for (name, _) in N64_BUTTONS {
            assert!(b.contains_key(*name), "n64 default missing: {name}");
        }
    }

    #[test]
    fn n64_remap_is_identity() {
        // N64 bits laid out as libretro RetroPad positions — identity
        // remap. Main analog stick is NOT in this bit-set (flows via
        // InputState.axes); the mask trims to the 14-bit digital
        // button set only.
        for (_, bit) in N64_BUTTONS {
            assert_eq!(n64_to_libretro_bits(*bit), *bit);
        }
        let all = n64::A | n64::B | n64::START
                | n64::L | n64::R | n64::Z
                | n64::C_UP | n64::C_DOWN | n64::C_LEFT | n64::C_RIGHT
                | n64::UP | n64::DOWN | n64::LEFT | n64::RIGHT;
        assert_eq!(n64_to_libretro_bits(all), all);
        assert_eq!(n64_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn n64_dispatch_round_trips() {
        assert!(buttons_for("n64").len() == N64_BUTTONS.len());
        assert!(defaults_for("n64").is_some());
        assert_eq!(bit_for("n64", "A"), Some(n64::A));
        assert_eq!(bit_for("n64", "Z"), Some(n64::Z));
        assert_eq!(bit_for("n64", "C_UP"), Some(n64::C_UP));
        assert_eq!(bit_for("n64", "C_RIGHT"), Some(n64::C_RIGHT));
        assert_eq!(to_libretro_bits("n64", n64::A), n64::A);
    }

    #[test]
    fn defaults_cover_every_gamecube_button() {
        let b = default_gamecube_bindings();
        for (name, _) in GAMECUBE_BUTTONS {
            assert!(b.contains_key(*name), "gamecube default missing: {name}");
        }
    }

    #[test]
    fn gamecube_remap_is_identity() {
        // GameCube bits laid out as libretro RetroPad positions —
        // identity remap. Main stick + C-stick flow via
        // InputState.axes; this bit-set covers only the 12 digital
        // buttons.
        for (_, bit) in GAMECUBE_BUTTONS {
            assert_eq!(gamecube_to_libretro_bits(*bit), *bit);
        }
        let all = gamecube::A | gamecube::B | gamecube::X | gamecube::Y
                | gamecube::L | gamecube::R | gamecube::Z | gamecube::START
                | gamecube::UP | gamecube::DOWN | gamecube::LEFT | gamecube::RIGHT;
        assert_eq!(gamecube_to_libretro_bits(all), all);
        assert_eq!(gamecube_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn gamecube_dispatch_round_trips() {
        assert!(buttons_for("gamecube").len() == GAMECUBE_BUTTONS.len());
        assert!(defaults_for("gamecube").is_some());
        assert_eq!(bit_for("gamecube", "A"), Some(gamecube::A));
        assert_eq!(bit_for("gamecube", "Z"), Some(gamecube::Z));
        assert_eq!(bit_for("gamecube", "Y"), Some(gamecube::Y));
        assert_eq!(to_libretro_bits("gamecube", gamecube::A), gamecube::A);
    }

    #[test]
    fn defaults_cover_every_dreamcast_button() {
        let b = default_dreamcast_bindings();
        for (name, _) in DREAMCAST_BUTTONS {
            assert!(b.contains_key(*name), "dreamcast default missing: {name}");
        }
    }

    #[test]
    fn dreamcast_remap_is_identity() {
        for (_, bit) in DREAMCAST_BUTTONS {
            assert_eq!(dreamcast_to_libretro_bits(*bit), *bit);
        }
        let all = dreamcast::A | dreamcast::B | dreamcast::X | dreamcast::Y
                | dreamcast::L | dreamcast::R | dreamcast::START
                | dreamcast::UP | dreamcast::DOWN | dreamcast::LEFT | dreamcast::RIGHT;
        assert_eq!(dreamcast_to_libretro_bits(all), all);
        assert_eq!(dreamcast_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn dreamcast_dispatch_round_trips() {
        assert!(buttons_for("dreamcast").len() == DREAMCAST_BUTTONS.len());
        assert!(defaults_for("dreamcast").is_some());
        assert_eq!(bit_for("dreamcast", "A"), Some(dreamcast::A));
        assert_eq!(bit_for("dreamcast", "Y"), Some(dreamcast::Y));
        assert_eq!(bit_for("dreamcast", "START"), Some(dreamcast::START));
        assert_eq!(to_libretro_bits("dreamcast", dreamcast::A), dreamcast::A);
    }

    #[test]
    fn defaults_cover_every_psp_button() {
        let b = default_psp_bindings();
        for (name, _) in PSP_BUTTONS {
            assert!(b.contains_key(*name), "psp default missing: {name}");
        }
    }

    #[test]
    fn psp_remap_is_identity() {
        for (_, bit) in PSP_BUTTONS {
            assert_eq!(psp_to_libretro_bits(*bit), *bit);
        }
        let all = psp::CROSS | psp::CIRCLE | psp::TRIANGLE | psp::SQUARE
                | psp::L | psp::R | psp::START | psp::SELECT
                | psp::UP | psp::DOWN | psp::LEFT | psp::RIGHT;
        assert_eq!(psp_to_libretro_bits(all), all);
        assert_eq!(psp_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn psp_dispatch_round_trips() {
        assert!(buttons_for("psp").len() == PSP_BUTTONS.len());
        assert!(defaults_for("psp").is_some());
        assert_eq!(bit_for("psp", "CROSS"), Some(psp::CROSS));
        assert_eq!(bit_for("psp", "TRIANGLE"), Some(psp::TRIANGLE));
        assert_eq!(bit_for("psp", "SELECT"), Some(psp::SELECT));
        assert_eq!(to_libretro_bits("psp", psp::CROSS), psp::CROSS);
    }

    #[test]
    fn defaults_cover_every_ps2_button() {
        let b = default_ps2_bindings();
        for (name, _) in PS2_BUTTONS {
            assert!(b.contains_key(*name), "ps2 default missing: {name}");
        }
    }

    #[test]
    fn ps2_remap_is_identity() {
        for (_, bit) in PS2_BUTTONS {
            assert_eq!(ps2_to_libretro_bits(*bit), *bit);
        }
        let all = ps2::CROSS | ps2::CIRCLE | ps2::TRIANGLE | ps2::SQUARE
                | ps2::L1 | ps2::R1 | ps2::L2 | ps2::R2 | ps2::L3 | ps2::R3
                | ps2::START | ps2::SELECT
                | ps2::UP | ps2::DOWN | ps2::LEFT | ps2::RIGHT;
        assert_eq!(ps2_to_libretro_bits(all), all);
        assert_eq!(ps2_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn ps2_dispatch_round_trips() {
        assert!(buttons_for("ps2").len() == PS2_BUTTONS.len());
        assert!(defaults_for("ps2").is_some());
        assert_eq!(bit_for("ps2", "CROSS"), Some(ps2::CROSS));
        assert_eq!(bit_for("ps2", "L3"), Some(ps2::L3));
        assert_eq!(bit_for("ps2", "R3"), Some(ps2::R3));
        assert_eq!(to_libretro_bits("ps2", ps2::CROSS), ps2::CROSS);
    }

    #[test]
    fn defaults_cover_every_nds_button() {
        let b = default_nds_bindings();
        for (name, _) in NDS_BUTTONS {
            assert!(b.contains_key(*name), "nds default missing: {name}");
        }
    }

    #[test]
    fn nds_remap_is_identity() {
        for (_, bit) in NDS_BUTTONS {
            assert_eq!(nds_to_libretro_bits(*bit), *bit);
        }
        let all = nds::A | nds::B | nds::X | nds::Y
                | nds::L | nds::R | nds::START | nds::SELECT
                | nds::UP | nds::DOWN | nds::LEFT | nds::RIGHT;
        assert_eq!(nds_to_libretro_bits(all), all);
        assert_eq!(nds_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn nds_dispatch_round_trips() {
        assert!(buttons_for("nds").len() == NDS_BUTTONS.len());
        assert!(defaults_for("nds").is_some());
        assert_eq!(bit_for("nds", "A"), Some(nds::A));
        assert_eq!(bit_for("nds", "Y"), Some(nds::Y));
        assert_eq!(bit_for("nds", "SELECT"), Some(nds::SELECT));
        assert_eq!(to_libretro_bits("nds", nds::A), nds::A);
    }

    #[test]
    fn atari7800_dispatch_round_trips() {
        // Sanity-check the per-system dispatch: bit_for / buttons_for /
        // to_libretro_bits / defaults_for all return populated values for
        // "atari7800". A missed dispatch arm would silently fall through
        // to "unknown" defaults and the operator would see empty bindings.
        assert!(buttons_for("atari7800").len() == ATARI7800_BUTTONS.len());
        assert!(defaults_for("atari7800").is_some());
        assert_eq!(bit_for("atari7800", "B1"), Some(atari7800::B1));
        assert_eq!(bit_for("atari7800", "PAUSE"), Some(atari7800::PAUSE));
        assert_eq!(to_libretro_bits("atari7800", atari7800::B1), atari7800::B1);
    }

    #[test]
    fn defaults_cover_every_sms_button() {
        let b = default_sms_bindings();
        for (name, _) in SMS_BUTTONS {
            assert!(b.contains_key(*name), "sms default missing: {name}");
        }
    }

    #[test]
    fn defaults_cover_every_gamegear_button() {
        let b = default_gamegear_bindings();
        for (name, _) in GAMEGEAR_BUTTONS {
            assert!(b.contains_key(*name), "gamegear default missing: {name}");
        }
    }

    #[test]
    fn sms_remap_is_identity() {
        for (_, bit) in SMS_BUTTONS {
            assert_eq!(sms_to_libretro_bits(*bit), *bit);
        }
        let all = sms::B1 | sms::B2 | sms::PAUSE
                | sms::UP | sms::DOWN | sms::LEFT | sms::RIGHT;
        assert_eq!(sms_to_libretro_bits(all), all);
        // Stray high bits get masked off.
        assert_eq!(sms_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn gamegear_remap_is_identity() {
        for (_, bit) in GAMEGEAR_BUTTONS {
            assert_eq!(gamegear_to_libretro_bits(*bit), *bit);
        }
        let all = gamegear::B1 | gamegear::B2 | gamegear::START
                | gamegear::UP | gamegear::DOWN | gamegear::LEFT | gamegear::RIGHT;
        assert_eq!(gamegear_to_libretro_bits(all), all);
        assert_eq!(gamegear_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn sms_dispatch_round_trips() {
        // Sanity-check the per-system dispatch: bit_for / buttons_for /
        // to_libretro_bits / defaults_for all return populated values for
        // "sms". A missed dispatch arm would silently fall through to
        // "unknown" defaults and the operator would see empty bindings.
        assert!(buttons_for("sms").len() == SMS_BUTTONS.len());
        assert!(defaults_for("sms").is_some());
        assert_eq!(bit_for("sms", "B1"), Some(sms::B1));
        assert_eq!(bit_for("sms", "PAUSE"), Some(sms::PAUSE));
        assert_eq!(to_libretro_bits("sms", sms::B1), sms::B1);
    }

    #[test]
    fn gamegear_dispatch_round_trips() {
        assert!(buttons_for("gamegear").len() == GAMEGEAR_BUTTONS.len());
        assert!(defaults_for("gamegear").is_some());
        assert_eq!(bit_for("gamegear", "B1"), Some(gamegear::B1));
        assert_eq!(bit_for("gamegear", "START"), Some(gamegear::START));
        assert_eq!(to_libretro_bits("gamegear", gamegear::B1), gamegear::B1);
    }

    #[test]
    fn defaults_cover_every_gb_button() {
        let b = default_gb_bindings();
        for (name, _) in GB_BUTTONS {
            assert!(b.contains_key(*name), "gb default missing: {name}");
        }
    }

    #[test]
    fn gb_remap_is_identity() {
        for (_, bit) in GB_BUTTONS {
            assert_eq!(gb_to_libretro_bits(*bit), *bit);
        }
        let all = gb::A | gb::B | gb::SELECT | gb::START
                | gb::UP | gb::DOWN | gb::LEFT | gb::RIGHT;
        assert_eq!(gb_to_libretro_bits(all), all);
        // Stray high bits get masked off.
        assert_eq!(gb_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn gb_dispatch_round_trips() {
        assert!(buttons_for("gb").len() == GB_BUTTONS.len());
        assert!(defaults_for("gb").is_some());
        assert_eq!(bit_for("gb", "A"), Some(gb::A));
        assert_eq!(bit_for("gb", "START"), Some(gb::START));
        assert_eq!(to_libretro_bits("gb", gb::A), gb::A);
    }

    #[test]
    fn defaults_cover_every_gba_button() {
        let b = default_gba_bindings();
        for (name, _) in GBA_BUTTONS {
            assert!(b.contains_key(*name), "gba default missing: {name}");
        }
    }

    #[test]
    fn gba_remap_is_identity() {
        for (_, bit) in GBA_BUTTONS {
            assert_eq!(gba_to_libretro_bits(*bit), *bit);
        }
        let all = gba::A | gba::B | gba::L | gba::R
                | gba::SELECT | gba::START
                | gba::UP | gba::DOWN | gba::LEFT | gba::RIGHT;
        assert_eq!(gba_to_libretro_bits(all), all);
        assert_eq!(gba_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn gba_dispatch_round_trips() {
        assert!(buttons_for("gba").len() == GBA_BUTTONS.len());
        assert!(defaults_for("gba").is_some());
        assert_eq!(bit_for("gba", "A"), Some(gba::A));
        assert_eq!(bit_for("gba", "L"), Some(gba::L));
        assert_eq!(bit_for("gba", "R"), Some(gba::R));
        assert_eq!(to_libretro_bits("gba", gba::A), gba::A);
    }

    #[test]
    fn defaults_cover_every_2600_button() {
        let b = default_atari2600_bindings();
        for (name, _) in ATARI2600_BUTTONS {
            assert!(b.contains_key(*name), "2600 default missing: {name}");
        }
        // Explicit assertion of the cross-system "Z is primary" rule
        // for the single-button case — since 2600 can't appear in the
        // z_is_the_primary_action_button_on_every_system fixture (which
        // requires a secondary), pin it here.
        assert_eq!(
            b.get("FIRE").and_then(|p| p.keyboard.as_deref()),
            Some("Z"),
            "2600 FIRE must be on Z (single-button primary action)",
        );
    }

    #[test]
    fn atari2600_remap_is_identity() {
        for (_, bit) in ATARI2600_BUTTONS {
            assert_eq!(atari2600_to_libretro_bits(*bit), *bit);
        }
        let all = atari2600::FIRE | atari2600::SELECT | atari2600::RESET
                | atari2600::UP | atari2600::DOWN | atari2600::LEFT | atari2600::RIGHT;
        assert_eq!(atari2600_to_libretro_bits(all), all);
        assert_eq!(atari2600_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn atari2600_dispatch_round_trips() {
        assert!(buttons_for("2600").len() == ATARI2600_BUTTONS.len());
        assert!(defaults_for("2600").is_some());
        assert_eq!(bit_for("2600", "FIRE"), Some(atari2600::FIRE));
        assert_eq!(bit_for("2600", "RESET"), Some(atari2600::RESET));
        assert_eq!(to_libretro_bits("2600", atari2600::FIRE), atari2600::FIRE);
    }

    // --- ColecoVision ----------------------------------------------------

    #[test]
    fn defaults_cover_every_coleco_button() {
        let b = default_coleco_bindings();
        for (name, _) in COLECO_BUTTONS {
            assert!(b.contains_key(*name), "coleco default missing: {name}");
        }
    }

    #[test]
    fn coleco_remap_is_identity() {
        for (_, bit) in COLECO_BUTTONS {
            assert_eq!(coleco_to_libretro_bits(*bit), *bit);
        }
        // All 16 bits combined fit cleanly.
        let mut all = 0u32;
        for (_, bit) in COLECO_BUTTONS { all |= *bit; }
        assert_eq!(coleco_to_libretro_bits(all), all);
        assert_eq!(coleco_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn coleco_dispatch_round_trips() {
        assert!(buttons_for("coleco").len() == COLECO_BUTTONS.len());
        assert!(defaults_for("coleco").is_some());
        assert_eq!(bit_for("coleco", "L_FIRE"), Some(coleco::L_FIRE));
        assert_eq!(bit_for("coleco", "KP5"), Some(coleco::KP5));
        assert_eq!(to_libretro_bits("coleco", coleco::L_FIRE), coleco::L_FIRE);
    }

    // --- Intellivision ---------------------------------------------------

    #[test]
    fn defaults_cover_every_intv_button() {
        let b = default_intv_bindings();
        for (name, _) in INTV_BUTTONS {
            assert!(b.contains_key(*name), "intv default missing: {name}");
        }
    }

    #[test]
    fn intv_remap_is_identity() {
        for (_, bit) in INTV_BUTTONS {
            assert_eq!(intv_to_libretro_bits(*bit), *bit);
        }
        let all = intv::UPPER_L | intv::UPPER_R | intv::LOWER_L | intv::LOWER_R
                | intv::START | intv::SELECT
                | intv::UP | intv::DOWN | intv::LEFT | intv::RIGHT;
        assert_eq!(intv_to_libretro_bits(all), all);
        assert_eq!(intv_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn intv_dispatch_round_trips() {
        assert!(buttons_for("intv").len() == INTV_BUTTONS.len());
        assert!(defaults_for("intv").is_some());
        assert_eq!(bit_for("intv", "LOWER_L"), Some(intv::LOWER_L));
        assert_eq!(bit_for("intv", "UPPER_R"), Some(intv::UPPER_R));
        assert_eq!(to_libretro_bits("intv", intv::LOWER_L), intv::LOWER_L);
    }

    // --- Magnavox Odyssey² -----------------------------------------------

    #[test]
    fn defaults_cover_every_o2_button() {
        let b = default_o2_bindings();
        for (name, _) in O2_BUTTONS {
            assert!(b.contains_key(*name), "o2 default missing: {name}");
        }
        // Single-button exception: Z=ACTION pinned here (the o2 doesn't
        // appear in z_is_the_primary_action_button_on_every_system because
        // it has no secondary action).
        assert_eq!(
            b.get("ACTION").and_then(|p| p.keyboard.as_deref()),
            Some("Z"),
            "o2 ACTION must be on Z (single-button primary action)",
        );
    }

    #[test]
    fn o2_remap_is_identity() {
        for (_, bit) in O2_BUTTONS {
            assert_eq!(o2_to_libretro_bits(*bit), *bit);
        }
        let all = o2::ACTION | o2::UP | o2::DOWN | o2::LEFT | o2::RIGHT;
        assert_eq!(o2_to_libretro_bits(all), all);
        assert_eq!(o2_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn o2_dispatch_round_trips() {
        assert!(buttons_for("o2").len() == O2_BUTTONS.len());
        assert!(defaults_for("o2").is_some());
        assert_eq!(bit_for("o2", "ACTION"), Some(o2::ACTION));
        assert_eq!(to_libretro_bits("o2", o2::ACTION), o2::ACTION);
    }

    // --- Fairchild Channel F ---------------------------------------------

    #[test]
    fn defaults_cover_every_channelf_button() {
        let b = default_channelf_bindings();
        for (name, _) in CHANNELF_BUTTONS {
            assert!(b.contains_key(*name), "channelf default missing: {name}");
        }
        // Effectively-single-action exception: Z=FIRE pinned here (the
        // channelf doesn't appear in z_is_the_primary_action_button_on_every_system
        // because MODE/TIME/START/HOLD are console switches with
        // hardware-label keyboards, not game-action secondaries).
        assert_eq!(
            b.get("FIRE").and_then(|p| p.keyboard.as_deref()),
            Some("Z"),
            "channelf FIRE must be on Z (primary action)",
        );
    }

    #[test]
    fn channelf_remap_is_identity() {
        for (_, bit) in CHANNELF_BUTTONS {
            assert_eq!(channelf_to_libretro_bits(*bit), *bit);
        }
        let all = channelf::FIRE | channelf::MODE | channelf::TIME
                | channelf::START | channelf::HOLD
                | channelf::UP | channelf::DOWN | channelf::LEFT | channelf::RIGHT;
        assert_eq!(channelf_to_libretro_bits(all), all);
        assert_eq!(channelf_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn channelf_dispatch_round_trips() {
        assert!(buttons_for("channelf").len() == CHANNELF_BUTTONS.len());
        assert!(defaults_for("channelf").is_some());
        assert_eq!(bit_for("channelf", "FIRE"), Some(channelf::FIRE));
        assert_eq!(bit_for("channelf", "HOLD"), Some(channelf::HOLD));
        assert_eq!(to_libretro_bits("channelf", channelf::FIRE), channelf::FIRE);
    }

    // --- Vectrex ---------------------------------------------------------

    #[test]
    fn defaults_cover_every_vectrex_button() {
        let b = default_vectrex_bindings();
        for (name, _) in VECTREX_BUTTONS {
            assert!(b.contains_key(*name), "vectrex default missing: {name}");
        }
    }

    #[test]
    fn vectrex_remap_is_identity() {
        for (_, bit) in VECTREX_BUTTONS {
            assert_eq!(vectrex_to_libretro_bits(*bit), *bit);
        }
        let all = vectrex::B1 | vectrex::B2 | vectrex::B3 | vectrex::B4
                | vectrex::UP | vectrex::DOWN | vectrex::LEFT | vectrex::RIGHT;
        assert_eq!(vectrex_to_libretro_bits(all), all);
        assert_eq!(vectrex_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn vectrex_dispatch_round_trips() {
        assert!(buttons_for("vectrex").len() == VECTREX_BUTTONS.len());
        assert!(defaults_for("vectrex").is_some());
        assert_eq!(bit_for("vectrex", "B1"), Some(vectrex::B1));
        assert_eq!(bit_for("vectrex", "B4"), Some(vectrex::B4));
        assert_eq!(to_libretro_bits("vectrex", vectrex::B1), vectrex::B1);
    }

    // --- Virtual Boy -----------------------------------------------------

    #[test]
    fn defaults_cover_every_virtualboy_button() {
        let b = default_virtualboy_bindings();
        for (name, _) in VIRTUALBOY_BUTTONS {
            assert!(b.contains_key(*name), "virtualboy default missing: {name}");
        }
    }

    #[test]
    fn virtualboy_remap_is_identity() {
        for (_, bit) in VIRTUALBOY_BUTTONS {
            assert_eq!(virtualboy_to_libretro_bits(*bit), *bit);
        }
        let all = virtualboy::A | virtualboy::B | virtualboy::L | virtualboy::R
                | virtualboy::START | virtualboy::SELECT
                | virtualboy::UP | virtualboy::DOWN | virtualboy::LEFT | virtualboy::RIGHT;
        assert_eq!(virtualboy_to_libretro_bits(all), all);
        assert_eq!(virtualboy_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn virtualboy_dispatch_round_trips() {
        assert!(buttons_for("virtualboy").len() == VIRTUALBOY_BUTTONS.len());
        assert!(defaults_for("virtualboy").is_some());
        assert_eq!(bit_for("virtualboy", "A"), Some(virtualboy::A));
        assert_eq!(bit_for("virtualboy", "L"), Some(virtualboy::L));
        assert_eq!(to_libretro_bits("virtualboy", virtualboy::A), virtualboy::A);
    }

    // --- WonderSwan ------------------------------------------------------

    #[test]
    fn defaults_cover_every_wonderswan_button() {
        let b = default_wonderswan_bindings();
        for (name, _) in WONDERSWAN_BUTTONS {
            assert!(b.contains_key(*name), "wonderswan default missing: {name}");
        }
    }

    #[test]
    fn wonderswan_remap_is_identity() {
        for (_, bit) in WONDERSWAN_BUTTONS {
            assert_eq!(wonderswan_to_libretro_bits(*bit), *bit);
        }
        let all = wonderswan::A | wonderswan::B | wonderswan::START
                | wonderswan::UP | wonderswan::DOWN | wonderswan::LEFT | wonderswan::RIGHT;
        assert_eq!(wonderswan_to_libretro_bits(all), all);
        assert_eq!(wonderswan_to_libretro_bits(all | (1 << 20)), all);
    }

    #[test]
    fn wonderswan_dispatch_round_trips() {
        assert!(buttons_for("wonderswan").len() == WONDERSWAN_BUTTONS.len());
        assert!(defaults_for("wonderswan").is_some());
        assert_eq!(bit_for("wonderswan", "A"), Some(wonderswan::A));
        assert_eq!(bit_for("wonderswan", "START"), Some(wonderswan::START));
        assert_eq!(to_libretro_bits("wonderswan", wonderswan::A), wonderswan::A);
    }

    #[test]
    fn pce_to_libretro_bit_remap() {
        // Spot-check the four bits where PCE-native layout diverges from
        // libretro RETRO_DEVICE_ID_JOYPAD_* — these are the ones that go
        // wrong when the dynamic core path skips the remap.
        assert_eq!(pce_to_libretro_bits(pce::I),     1 << 8); // → A
        assert_eq!(pce_to_libretro_bits(pce::II),    1 << 0); // → B
        assert_eq!(pce_to_libretro_bits(pce::RUN),   1 << 3); // → START
        assert_eq!(pce_to_libretro_bits(pce::UP),    1 << 4); // unchanged
        assert_eq!(pce_to_libretro_bits(pce::DOWN),  1 << 5); // PCE bit 6 → libretro bit 5
        assert_eq!(pce_to_libretro_bits(pce::LEFT),  1 << 6); // PCE bit 7 → libretro bit 6
        assert_eq!(pce_to_libretro_bits(pce::RIGHT), 1 << 7); // PCE bit 5 → libretro bit 7
        // Combined: full d-pad + I + RUN.
        let all = pce::UP | pce::DOWN | pce::LEFT | pce::RIGHT | pce::I | pce::RUN;
        let expect = (1u32 << 4) | (1 << 5) | (1 << 6) | (1 << 7) | (1 << 8) | (1 << 3);
        assert_eq!(pce_to_libretro_bits(all), expect);
    }
}
