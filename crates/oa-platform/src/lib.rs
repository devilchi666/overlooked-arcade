//! oa-platform — OS paths, settings persistence, ROM scanning.
//!
//! Phase 1 scaffold.

#![deny(rust_2018_idioms)]

use std::path::PathBuf;

/// Resolve the OS-standard user data directory for the app. Stub for now.
pub fn user_data_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("dev", "OverlookedArcade", "OverlookedArcade")
        .map(|p| p.data_dir().to_path_buf())
}

/// Resolve the OS-standard user config directory for the app. Stub for now.
pub fn user_config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("dev", "OverlookedArcade", "OverlookedArcade")
        .map(|p| p.config_dir().to_path_buf())
}
