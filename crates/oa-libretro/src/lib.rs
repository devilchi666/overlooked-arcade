//! oa-libretro — dynamic libretro core loader.
//!
//! Wraps a libretro-compatible `.dll` / `.so` / `.dylib` file as an
//! [`oa_core::Core`] so the rest of the workspace (renderer, audio sink,
//! input layer, save-state machinery) can drive it without caring whether the
//! emulation logic was statically linked or dynamically loaded.
//!
//! ## Usage
//!
//! ```no_run
//! use oa_libretro::{LibretroCore, RomSource};
//! use oa_core::{Core, SystemId};
//! use std::path::Path;
//!
//! let mut core = LibretroCore::load(
//!     Path::new("cores/mednafen_pce_fast_libretro.dll"),
//!     SystemId::PcEngine,
//!     Path::new("system"),
//!     Path::new("saves"),
//! ).unwrap();
//!
//! // HuCard / cart: bytes work fine.
//! let rom_bytes: Vec<u8> = std::fs::read("game.pce").unwrap();
//! core.load_rom(RomSource::Bytes(&rom_bytes), "pce", "game").unwrap();
//!
//! // CD images need a path because the core reads multiple track files
//! // relative to it.
//! core.load_rom(RomSource::Path(Path::new("game.cue")), "cue", "game").unwrap();
//!
//! loop {
//!     core.run_frame();
//!     // use core.framebuffer(), core.drain_audio(), etc.
//! }
//! ```
//!
//! ## Singleton constraint
//!
//! libretro cores are singletons (they keep mutable C globals). Only one
//! `LibretroCore` may exist at a time per process. Constructing a second one
//! while the first is alive returns [`LibretroError::AlreadyLoaded`]. The
//! singleton state is torn down on `Drop`.
//!
//! ## Callback model
//!
//! libretro's callback signatures are plain `extern "C"` function pointers
//! that can't carry closure state. The callbacks read shared state from a
//! process-global `Mutex<Option<State>>` (the singleton constraint above).
//! All callbacks fire from `retro_run`, which executes synchronously on the
//! emu thread, so the lock is theoretically uncontended.

#![deny(rust_2018_idioms)]

mod core;
pub mod ffi;
mod hw_vulkan;
mod keycode;
mod loader;
mod pixel;
mod state;

pub use crate::core::{probe, CoreInfo, LibretroCore, LibretroError, RomSource};
pub use crate::ffi::PixelFormat;
pub use crate::hw_vulkan::{HwVulkanFrame, LoadedHwVulkan};
pub use crate::keycode::{keycode_to_retro_key, modifiers_from_held};
pub use crate::state::{
    loaded_core_controller_devices, loaded_core_hw_frame, loaded_core_hw_vulkan,
    loaded_core_input_descriptors, loaded_core_min_audio_latency_ms, set_hw_import_mode,
};
