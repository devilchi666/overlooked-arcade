//! libloading wrapper: opens a libretro core .dll/.so/.dylib and resolves the
//! 22 entry points we need into a typed function-pointer table.

use std::path::Path;

use libloading::{Library, Symbol};

use crate::ffi::*;

/// All the libretro core entry points we use, pre-resolved. A few fields
/// (get_system_info, get_region) are reserved for the cores-folder UI that
/// will populate per-core info — not used yet.
#[allow(dead_code)]
pub struct LibretroFns {
    pub init: retro_init_t,
    pub deinit: retro_deinit_t,
    pub api_version: retro_api_version_t,
    pub get_system_info: retro_get_system_info_t,
    pub get_system_av_info: retro_get_system_av_info_t,
    pub set_controller_port_device: retro_set_controller_port_device_t,
    pub reset: retro_reset_t,
    pub run: retro_run_t,
    pub serialize_size: retro_serialize_size_t,
    pub serialize: retro_serialize_t,
    pub unserialize: retro_unserialize_t,
    pub load_game: retro_load_game_t,
    pub unload_game: retro_unload_game_t,
    pub get_region: retro_get_region_t,
    pub get_memory_data: retro_get_memory_data_t,
    pub get_memory_size: retro_get_memory_size_t,

    pub set_environment: retro_set_environment_t,
    pub set_video_refresh: retro_set_video_refresh_t,
    pub set_audio_sample: retro_set_audio_sample_t,
    pub set_audio_sample_batch: retro_set_audio_sample_batch_t,
    pub set_input_poll: retro_set_input_poll_t,
    pub set_input_state: retro_set_input_state_t,
}

/// Holds the `Library` so its lifetime exceeds the function pointers we
/// resolved out of it. Dropping this unloads the .dll.
pub struct LibretroLibrary {
    pub fns: LibretroFns,
    // Keep the Library alive; symbol pointers borrow from it.
    _lib: Library,
}

/// Helper to resolve a single symbol. Symbol is leaked into a transmuted fn
/// pointer — safe because we keep the parent Library alive in LibretroLibrary.
unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> Result<T, libloading::Error> {
    let symbol: Symbol<'_, T> = unsafe { lib.get(name)? };
    Ok(*symbol)
}

impl LibretroLibrary {
    /// Open `path`, resolve all libretro entry points. Fails if any required
    /// symbol is missing — libretro requires all of them, missing symbols mean
    /// the .dll isn't a valid libretro core.
    pub fn open(path: &Path) -> Result<Self, libloading::Error> {
        let lib = unsafe { Library::new(path)? };

        let fns = unsafe {
            LibretroFns {
                init: sym(&lib, b"retro_init\0")?,
                deinit: sym(&lib, b"retro_deinit\0")?,
                api_version: sym(&lib, b"retro_api_version\0")?,
                get_system_info: sym(&lib, b"retro_get_system_info\0")?,
                get_system_av_info: sym(&lib, b"retro_get_system_av_info\0")?,
                set_controller_port_device: sym(&lib, b"retro_set_controller_port_device\0")?,
                reset: sym(&lib, b"retro_reset\0")?,
                run: sym(&lib, b"retro_run\0")?,
                serialize_size: sym(&lib, b"retro_serialize_size\0")?,
                serialize: sym(&lib, b"retro_serialize\0")?,
                unserialize: sym(&lib, b"retro_unserialize\0")?,
                load_game: sym(&lib, b"retro_load_game\0")?,
                unload_game: sym(&lib, b"retro_unload_game\0")?,
                get_region: sym(&lib, b"retro_get_region\0")?,
                get_memory_data: sym(&lib, b"retro_get_memory_data\0")?,
                get_memory_size: sym(&lib, b"retro_get_memory_size\0")?,

                set_environment: sym(&lib, b"retro_set_environment\0")?,
                set_video_refresh: sym(&lib, b"retro_set_video_refresh\0")?,
                set_audio_sample: sym(&lib, b"retro_set_audio_sample\0")?,
                set_audio_sample_batch: sym(&lib, b"retro_set_audio_sample_batch\0")?,
                set_input_poll: sym(&lib, b"retro_set_input_poll\0")?,
                set_input_state: sym(&lib, b"retro_set_input_state\0")?,
            }
        };

        Ok(Self { fns, _lib: lib })
    }
}
