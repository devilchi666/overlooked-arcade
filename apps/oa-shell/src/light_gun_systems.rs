//! Declarative catalogue of light-gun-capable systems + the libretro
//! device type each known core expects.
//!
//! This table exists to make the operator's playtest mechanical:
//! given a system + a flagship light-gun title, the table says
//! exactly which libretro device id the core will poll
//! (RETRO_DEVICE_LIGHTGUN for the classical guns,
//! RETRO_DEVICE_POINTER for the touch-shaped ones). The harness
//! tests in `crates/oa-libretro/src/state.rs::tests` verify that
//! BOTH device dispatch paths produce correct field values from a
//! given `input_pointer[port]` tuple, so this table tells us which
//! path matters for which system.
//!
//! Status field on each entry:
//!   - `WiringValidated` — the device dispatch is wired AND the
//!     operator confirmed the gun reaches the core in a real game.
//!   - `WiringShipped` — the dispatch is wired (POINTER + LIGHTGUN
//!     branches both land 2026-05-25); operator playtest pending.
//!   - `Blocked` — known gap that prevents shipping, with a note.
//!
//! Per-core ROADMAPs link back to this table for the "light gun"
//! bullet. When operator playtest validates a system, flip its
//! status here AND update the corresponding ROADMAP.
//!
//! Not used by runtime code (yet). Lives next to the bindings module
//! as system-metadata reference. Future work: a Tauri command that
//! reads this table to drive an operator-facing "test the light gun"
//! debug overlay.

#![allow(dead_code)] // reference table; consumed by tests + future debug UI

use oa_libretro::ffi::{RETRO_DEVICE_LIGHTGUN, RETRO_DEVICE_POINTER};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WiringStatus {
    /// Dispatch wired + operator confirmed gun reaches the core in
    /// a real game.
    WiringValidated,
    /// Dispatch wired; operator playtest pending. The default state
    /// for everything that landed in the 2026-05-25 LIGHTGUN-branch
    /// addition since none of these have been validated end-to-end.
    WiringShipped,
    /// Dispatch wired but operator hit a blocker (e.g. core ignores
    /// the polls, off-screen reload missing, etc.). The `notes` field
    /// describes the gap.
    Blocked,
}

#[derive(Debug, Clone, Copy)]
pub struct LightGunSystem {
    /// `oa_core::SystemId` slug — matches the `parse_system_id` arms.
    pub system_id: &'static str,
    /// Canonical libretro device id the typical core polls.
    /// `RETRO_DEVICE_LIGHTGUN` for classical guns; `RETRO_DEVICE_POINTER`
    /// for touch-shaped systems that re-use the gun input model.
    pub device: u32,
    /// Flagship title — the canonical "does the gun work?" test ROM
    /// for this system. Operator drops this ROM, aims at a target,
    /// pulls the trigger; success = on-screen feedback.
    pub flagship_title: &'static str,
    /// Default libretro core for this system.
    pub default_core_dll: &'static str,
    /// Current wiring + validation state.
    pub status: WiringStatus,
    /// Free-form note. Caveats, alternate cores, known per-game
    /// quirks. Read by the operator before playtest so they know what
    /// to expect.
    pub notes: &'static str,
}

/// Six known light-gun-capable systems. NES + SMS + Saturn use the
/// classical Zapper / Light Phaser / Virtua Gun via RETRO_DEVICE_LIGHTGUN.
/// PSX (GunCon) + Dreamcast (HotD) also use LIGHTGUN — both Beetle PSX
/// and Flycast map mouse to gun coords through the LIGHTGUN id space.
/// NDS uses POINTER for its stylus, which doubles as the touch input
/// model some homebrew gun-like games use; included here for
/// completeness since the same dispatch path serves both.
pub const LIGHT_GUN_SYSTEMS: &[LightGunSystem] = &[
    LightGunSystem {
        system_id: "nes",
        device: RETRO_DEVICE_LIGHTGUN,
        flagship_title: "Duck Hunt",
        default_core_dll: "fceumm_libretro.dll",
        status: WiringStatus::WiringShipped,
        notes: "FCEUmm polls LIGHTGUN SCREEN_X/Y/TRIGGER. \
                Operator binds left-click to the trigger; aim by moving \
                mouse inside the game window. Duck Hunt + Hogan's Alley \
                + Wild Gunman are the canonical Zapper titles.",
    },
    LightGunSystem {
        system_id: "snes",
        device: RETRO_DEVICE_LIGHTGUN,
        flagship_title: "Super Scope 6",
        default_core_dll: "snes9x_libretro.dll",
        status: WiringStatus::WiringShipped,
        notes: "snes9x polls LIGHTGUN for the Super Scope. bsnes \
                Super Scope path also goes through LIGHTGUN. AUX_A / \
                AUX_B (Cursor, Turbo, Pause) return 0 today — Phase 2 \
                Bindings UI work to wire them.",
    },
    LightGunSystem {
        system_id: "sms",
        device: RETRO_DEVICE_LIGHTGUN,
        flagship_title: "Operation Wolf",
        default_core_dll: "genesis_plus_gx_libretro.dll",
        status: WiringStatus::WiringShipped,
        notes: "Genesis Plus GX exposes the SMS Light Phaser through \
                LIGHTGUN. Phaser titles: Operation Wolf, Rambo III, \
                Shooting Gallery, Marksman Shooting / Trap Shooting.",
    },
    LightGunSystem {
        system_id: "saturn",
        device: RETRO_DEVICE_LIGHTGUN,
        flagship_title: "Virtua Cop",
        default_core_dll: "mednafen_saturn_libretro.dll",
        status: WiringStatus::WiringShipped,
        notes: "Beetle Saturn + Kronos both poll LIGHTGUN for the \
                Virtua Gun. Virtua Cop 1/2, House of the Dead, Death \
                Crimson 2, Crypt Killer.",
    },
    LightGunSystem {
        system_id: "psx",
        device: RETRO_DEVICE_LIGHTGUN,
        flagship_title: "Time Crisis",
        default_core_dll: "mednafen_psx_libretro.dll",
        status: WiringStatus::WiringShipped,
        notes: "Beetle PSX polls LIGHTGUN for the Namco GunCon (Time \
                Crisis 1/2, Point Blank trilogy) AND for the Konami \
                Justifier (Lethal Enforcers, Crypt Killer). Time \
                Crisis specifically uses off-screen aim for reload — \
                IS_OFFSCREEN plumbed 2026-05-27 (in_viewport flag on \
                InputState.pointer; off-viewport reports 1 to the core).",
    },
    LightGunSystem {
        system_id: "dreamcast",
        device: RETRO_DEVICE_LIGHTGUN,
        flagship_title: "House of the Dead 2",
        default_core_dll: "flycast_libretro.dll",
        status: WiringStatus::WiringShipped,
        notes: "Flycast polls LIGHTGUN for the DC light gun + the \
                arcade-cabinet ports (HotD 2, Confidential Mission, \
                Death Crimson OX, Maze of the Kings). Confidential \
                Mission's reload-by-aim now wired via IS_OFFSCREEN \
                (in_viewport plumbed 2026-05-27 alongside PSX Time \
                Crisis).",
    },
    LightGunSystem {
        system_id: "nds",
        device: RETRO_DEVICE_POINTER,
        flagship_title: "Phantom Hourglass (touch as gun)",
        default_core_dll: "melonds_libretro.dll",
        status: WiringStatus::WiringShipped,
        notes: "NDS uses POINTER (not LIGHTGUN) for stylus input. \
                The same coord-mapping path serves any touch-shaped \
                game including the handful of homebrew light-gun \
                titles. Included for completeness — the dispatch path \
                is shared with the gun systems above.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_has_a_known_device_type() {
        // Defensive: only LIGHTGUN (4) and POINTER (6) are valid
        // device types for this table. A typo would land here.
        for sys in LIGHT_GUN_SYSTEMS {
            assert!(
                sys.device == RETRO_DEVICE_LIGHTGUN || sys.device == RETRO_DEVICE_POINTER,
                "{} declared unknown device type {}", sys.system_id, sys.device,
            );
        }
    }

    #[test]
    fn no_duplicate_system_ids() {
        // Each system_id appears at most once. Two entries for the
        // same slug would silently shadow each other when the future
        // debug-UI Tauri command looks one up.
        let mut seen = std::collections::HashSet::new();
        for sys in LIGHT_GUN_SYSTEMS {
            assert!(
                seen.insert(sys.system_id),
                "duplicate system_id {}", sys.system_id,
            );
        }
    }

    #[test]
    fn all_required_fields_populated() {
        // Catch accidentally-empty-string fields. Operator running
        // playtest needs every field non-empty to know what to do.
        for sys in LIGHT_GUN_SYSTEMS {
            assert!(!sys.system_id.is_empty(), "empty system_id");
            assert!(!sys.flagship_title.is_empty(), "empty flagship_title for {}", sys.system_id);
            assert!(!sys.default_core_dll.is_empty(), "empty default_core_dll for {}", sys.system_id);
            assert!(!sys.notes.is_empty(), "empty notes for {}", sys.system_id);
        }
    }

    #[test]
    fn covers_systems_named_in_next_md() {
        // NEXT.md MEDIUM #5 enumerates dreamcast / saturn / nes / psx
        // as the operator-validation set. Table must contain entries
        // for at least those four — losing one would silently drop it
        // from the playtest matrix.
        let slugs: std::collections::HashSet<&'static str> =
            LIGHT_GUN_SYSTEMS.iter().map(|s| s.system_id).collect();
        for required in &["nes", "saturn", "psx", "dreamcast"] {
            assert!(slugs.contains(required), "missing required slug {required}");
        }
    }
}
