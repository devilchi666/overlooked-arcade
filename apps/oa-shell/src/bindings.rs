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
        _ => None,
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
        for sys in &["tg16", "pce-cd", "lynx", "nes", "snes", "mame"] {
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
        for sys in &["tg16", "pce-cd", "lynx", "nes", "snes", "mame"] {
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
        for sys in &["tg16", "pce-cd", "lynx", "nes", "snes", "mame"] {
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
        // its defaults swapped, this test catches it. ⚠ If any system
        // legitimately needs an exception (e.g. it has more than one
        // "primary" button), document that here.
        for (sys, primary_name, secondary_name) in &[
            ("tg16", "I", "II"),
            ("pce-cd", "I", "II"),
            ("lynx", "A", "B"),
            ("nes", "A", "B"),
            ("snes", "A", "B"),
            ("mame", "B1", "B2"),
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
