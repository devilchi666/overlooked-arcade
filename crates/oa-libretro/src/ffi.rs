//! libretro ABI types + constants.
//!
//! Hand-translated subset of `libretro.h` covering everything our shell needs
//! to host a core. Names mirror the C names so cross-referencing upstream docs
//! is straightforward. Layouts use `#[repr(C)]` and the field order matches the
//! C structs byte-for-byte.

#![allow(non_camel_case_types, non_snake_case, dead_code)]

use std::ffi::c_void;
use std::os::raw::c_char;

// ---------- pixel format ----------

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// `0RRRRRGGGGGBBBBB` — top bit unused.
    Rgb1555 = 0,
    /// `XXXXXXXXRRRRRRRRGGGGGGGGBBBBBBBB` (BGRA in little-endian byte order).
    Xrgb8888 = 1,
    /// `RRRRRGGGGGGBBBBB`.
    Rgb565 = 2,
}

impl PixelFormat {
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(Self::Rgb1555),
            1 => Some(Self::Xrgb8888),
            2 => Some(Self::Rgb565),
            _ => None,
        }
    }
}

// ---------- device IDs ----------

pub const RETRO_DEVICE_JOYPAD: u32 = 1;

pub const RETRO_DEVICE_ID_JOYPAD_B: u32       = 0;
pub const RETRO_DEVICE_ID_JOYPAD_Y: u32       = 1;
pub const RETRO_DEVICE_ID_JOYPAD_SELECT: u32  = 2;
pub const RETRO_DEVICE_ID_JOYPAD_START: u32   = 3;
pub const RETRO_DEVICE_ID_JOYPAD_UP: u32      = 4;
pub const RETRO_DEVICE_ID_JOYPAD_DOWN: u32    = 5;
pub const RETRO_DEVICE_ID_JOYPAD_LEFT: u32    = 6;
pub const RETRO_DEVICE_ID_JOYPAD_RIGHT: u32   = 7;
pub const RETRO_DEVICE_ID_JOYPAD_A: u32       = 8;
pub const RETRO_DEVICE_ID_JOYPAD_X: u32       = 9;
pub const RETRO_DEVICE_ID_JOYPAD_L: u32       = 10;
pub const RETRO_DEVICE_ID_JOYPAD_R: u32       = 11;
pub const RETRO_DEVICE_ID_JOYPAD_L2: u32      = 12;
pub const RETRO_DEVICE_ID_JOYPAD_R2: u32      = 13;
pub const RETRO_DEVICE_ID_JOYPAD_L3: u32      = 14;
pub const RETRO_DEVICE_ID_JOYPAD_R3: u32      = 15;

// ---------- environment commands ----------

pub const RETRO_ENVIRONMENT_EXPERIMENTAL: u32 = 0x10000;

pub const RETRO_ENVIRONMENT_SET_ROTATION: u32              = 1;
pub const RETRO_ENVIRONMENT_GET_OVERSCAN: u32              = 2;
pub const RETRO_ENVIRONMENT_GET_CAN_DUPE: u32              = 3;
pub const RETRO_ENVIRONMENT_SET_MESSAGE: u32               = 6;
pub const RETRO_ENVIRONMENT_SHUTDOWN: u32                  = 7;
pub const RETRO_ENVIRONMENT_SET_PERFORMANCE_LEVEL: u32     = 8;
pub const RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY: u32      = 9;
pub const RETRO_ENVIRONMENT_SET_PIXEL_FORMAT: u32          = 10;
pub const RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS: u32     = 11;
pub const RETRO_ENVIRONMENT_SET_KEYBOARD_CALLBACK: u32     = 12;
pub const RETRO_ENVIRONMENT_SET_DISK_CONTROL_INTERFACE: u32 = 13;
pub const RETRO_ENVIRONMENT_SET_HW_RENDER: u32             = 14;
pub const RETRO_ENVIRONMENT_GET_VARIABLE: u32              = 15;
pub const RETRO_ENVIRONMENT_SET_VARIABLES: u32             = 16;
pub const RETRO_ENVIRONMENT_GET_VARIABLE_UPDATE: u32       = 17;
pub const RETRO_ENVIRONMENT_SET_SUPPORT_NO_GAME: u32       = 18;
pub const RETRO_ENVIRONMENT_GET_LIBRETRO_PATH: u32         = 19;
pub const RETRO_ENVIRONMENT_SET_FRAME_TIME_CALLBACK: u32   = 21;
pub const RETRO_ENVIRONMENT_SET_AUDIO_CALLBACK: u32        = 22;
pub const RETRO_ENVIRONMENT_GET_RUMBLE_INTERFACE: u32      = 23;
pub const RETRO_ENVIRONMENT_GET_INPUT_DEVICE_CAPABILITIES: u32 = 24;
pub const RETRO_ENVIRONMENT_GET_SENSOR_INTERFACE: u32      = 25 | RETRO_ENVIRONMENT_EXPERIMENTAL;
pub const RETRO_ENVIRONMENT_GET_CAMERA_INTERFACE: u32      = 26 | RETRO_ENVIRONMENT_EXPERIMENTAL;
pub const RETRO_ENVIRONMENT_GET_LOG_INTERFACE: u32         = 27;
pub const RETRO_ENVIRONMENT_GET_PERF_INTERFACE: u32        = 28;
pub const RETRO_ENVIRONMENT_GET_LOCATION_INTERFACE: u32    = 29;
pub const RETRO_ENVIRONMENT_GET_CORE_ASSETS_DIRECTORY: u32 = 30;
pub const RETRO_ENVIRONMENT_GET_SAVE_DIRECTORY: u32        = 31;
pub const RETRO_ENVIRONMENT_SET_SYSTEM_AV_INFO: u32        = 32;
pub const RETRO_ENVIRONMENT_SET_PROC_ADDRESS_CALLBACK: u32 = 33;
pub const RETRO_ENVIRONMENT_SET_SUBSYSTEM_INFO: u32        = 34;
pub const RETRO_ENVIRONMENT_SET_CONTROLLER_INFO: u32       = 35;
pub const RETRO_ENVIRONMENT_SET_MEMORY_MAPS: u32           = 36 | RETRO_ENVIRONMENT_EXPERIMENTAL;
pub const RETRO_ENVIRONMENT_SET_GEOMETRY: u32              = 37;
pub const RETRO_ENVIRONMENT_GET_USERNAME: u32              = 38;
pub const RETRO_ENVIRONMENT_GET_LANGUAGE: u32              = 39;
pub const RETRO_ENVIRONMENT_GET_CURRENT_SOFTWARE_FRAMEBUFFER: u32 = 40 | RETRO_ENVIRONMENT_EXPERIMENTAL;
pub const RETRO_ENVIRONMENT_GET_HW_RENDER_INTERFACE: u32   = 41 | RETRO_ENVIRONMENT_EXPERIMENTAL;
pub const RETRO_ENVIRONMENT_SET_SUPPORT_ACHIEVEMENTS: u32  = 42 | RETRO_ENVIRONMENT_EXPERIMENTAL;
pub const RETRO_ENVIRONMENT_SET_HW_RENDER_CONTEXT_NEGOTIATION_INTERFACE: u32 = 43 | RETRO_ENVIRONMENT_EXPERIMENTAL;
pub const RETRO_ENVIRONMENT_SET_SERIALIZATION_QUIRKS: u32  = 44;
pub const RETRO_ENVIRONMENT_SET_HW_SHARED_CONTEXT: u32     = 44 | RETRO_ENVIRONMENT_EXPERIMENTAL;
pub const RETRO_ENVIRONMENT_GET_VFS_INTERFACE: u32         = 45 | RETRO_ENVIRONMENT_EXPERIMENTAL;
pub const RETRO_ENVIRONMENT_GET_LED_INTERFACE: u32         = 46 | RETRO_ENVIRONMENT_EXPERIMENTAL;
pub const RETRO_ENVIRONMENT_GET_AUDIO_VIDEO_ENABLE: u32    = 47 | RETRO_ENVIRONMENT_EXPERIMENTAL;
pub const RETRO_ENVIRONMENT_GET_MIDI_INTERFACE: u32        = 48 | RETRO_ENVIRONMENT_EXPERIMENTAL;
pub const RETRO_ENVIRONMENT_GET_FASTFORWARDING: u32        = 49 | RETRO_ENVIRONMENT_EXPERIMENTAL;
pub const RETRO_ENVIRONMENT_GET_TARGET_REFRESH_RATE: u32   = 50 | RETRO_ENVIRONMENT_EXPERIMENTAL;
pub const RETRO_ENVIRONMENT_GET_INPUT_BITMASKS: u32        = 51 | RETRO_ENVIRONMENT_EXPERIMENTAL;
pub const RETRO_ENVIRONMENT_GET_CORE_OPTIONS_VERSION: u32  = 52;
pub const RETRO_ENVIRONMENT_SET_CORE_OPTIONS: u32          = 53;
pub const RETRO_ENVIRONMENT_SET_CORE_OPTIONS_INTL: u32     = 54;
pub const RETRO_ENVIRONMENT_SET_CORE_OPTIONS_DISPLAY: u32  = 55;
pub const RETRO_ENVIRONMENT_GET_PREFERRED_HW_RENDER: u32   = 56;
pub const RETRO_ENVIRONMENT_GET_DISK_CONTROL_INTERFACE_VERSION: u32 = 57;
pub const RETRO_ENVIRONMENT_SET_DISK_CONTROL_EXT_INTERFACE: u32     = 58;
pub const RETRO_ENVIRONMENT_GET_MESSAGE_INTERFACE_VERSION: u32      = 59;
pub const RETRO_ENVIRONMENT_SET_MESSAGE_EXT: u32                    = 60;
pub const RETRO_ENVIRONMENT_GET_INPUT_MAX_USERS: u32                = 61;
pub const RETRO_ENVIRONMENT_SET_AUDIO_BUFFER_STATUS_CALLBACK: u32   = 62;
pub const RETRO_ENVIRONMENT_SET_MINIMUM_AUDIO_LATENCY: u32          = 63;
pub const RETRO_ENVIRONMENT_SET_FASTFORWARDING_OVERRIDE: u32        = 64;
pub const RETRO_ENVIRONMENT_SET_CONTENT_INFO_OVERRIDE: u32          = 65;
pub const RETRO_ENVIRONMENT_GET_GAME_INFO_EXT: u32                  = 66;
pub const RETRO_ENVIRONMENT_SET_CORE_OPTIONS_V2: u32                = 67;
pub const RETRO_ENVIRONMENT_SET_CORE_OPTIONS_V2_INTL: u32           = 68;
pub const RETRO_ENVIRONMENT_SET_CORE_OPTIONS_UPDATE_DISPLAY_CALLBACK: u32 = 69;
pub const RETRO_ENVIRONMENT_SET_VARIABLE: u32                       = 70;

// ---------- log levels ----------

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Debug = 0,
    Info  = 1,
    Warn  = 2,
    Error = 3,
}

// ---------- language ----------

pub const RETRO_LANGUAGE_ENGLISH: u32 = 0;

// ---------- struct definitions ----------

#[repr(C)]
#[derive(Debug)]
pub struct retro_game_info {
    pub path: *const c_char,
    pub data: *const c_void,
    pub size: usize,
    pub meta: *const c_char,
}

#[repr(C)]
#[derive(Debug)]
pub struct retro_game_info_ext {
    pub full_path: *const c_char,
    pub archive_path: *const c_char,
    pub archive_file: *const c_char,
    pub dir: *const c_char,
    pub name: *const c_char,
    pub ext: *const c_char,
    pub meta: *const c_char,
    pub data: *const c_void,
    pub size: usize,
    pub file_in_archive: bool,
    pub persistent_data: bool,
}

#[repr(C)]
#[derive(Debug)]
pub struct retro_system_info {
    pub library_name: *const c_char,
    pub library_version: *const c_char,
    pub valid_extensions: *const c_char,
    pub need_fullpath: bool,
    pub block_extract: bool,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct retro_game_geometry {
    pub base_width: u32,
    pub base_height: u32,
    pub max_width: u32,
    pub max_height: u32,
    pub aspect_ratio: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct retro_system_timing {
    pub fps: f64,
    pub sample_rate: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct retro_system_av_info {
    pub geometry: retro_game_geometry,
    pub timing: retro_system_timing,
}

#[repr(C)]
#[derive(Debug)]
pub struct retro_variable {
    pub key: *const c_char,
    pub value: *const c_char,
}

pub type retro_log_printf_t = unsafe extern "C" fn(level: u32, fmt: *const c_char, ...);

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct retro_log_callback {
    pub log: retro_log_printf_t,
}

// ---------- callback function-pointer types ----------

pub type retro_environment_t       = unsafe extern "C" fn(cmd: u32, data: *mut c_void) -> bool;
pub type retro_video_refresh_t     = unsafe extern "C" fn(data: *const c_void, width: u32, height: u32, pitch: usize);
pub type retro_audio_sample_t      = unsafe extern "C" fn(left: i16, right: i16);
pub type retro_audio_sample_batch_t = unsafe extern "C" fn(data: *const i16, frames: usize) -> usize;
pub type retro_input_poll_t        = unsafe extern "C" fn();
pub type retro_input_state_t       = unsafe extern "C" fn(port: u32, device: u32, index: u32, id: u32) -> i16;

// ---------- core entry-point typedefs (what we resolve via libloading) ----------

pub type retro_init_t                       = unsafe extern "C" fn();
pub type retro_deinit_t                     = unsafe extern "C" fn();
pub type retro_api_version_t                = unsafe extern "C" fn() -> u32;
pub type retro_get_system_info_t            = unsafe extern "C" fn(info: *mut retro_system_info);
pub type retro_get_system_av_info_t         = unsafe extern "C" fn(info: *mut retro_system_av_info);
pub type retro_set_controller_port_device_t = unsafe extern "C" fn(port: u32, device: u32);
pub type retro_reset_t                      = unsafe extern "C" fn();
pub type retro_run_t                        = unsafe extern "C" fn();
pub type retro_serialize_size_t             = unsafe extern "C" fn() -> usize;
pub type retro_serialize_t                  = unsafe extern "C" fn(data: *mut c_void, size: usize) -> bool;
pub type retro_unserialize_t                = unsafe extern "C" fn(data: *const c_void, size: usize) -> bool;
pub type retro_load_game_t                  = unsafe extern "C" fn(game: *const retro_game_info) -> bool;
pub type retro_unload_game_t                = unsafe extern "C" fn();
pub type retro_get_region_t                 = unsafe extern "C" fn() -> u32;
pub type retro_get_memory_data_t            = unsafe extern "C" fn(id: u32) -> *mut c_void;
pub type retro_get_memory_size_t            = unsafe extern "C" fn(id: u32) -> usize;

pub type retro_set_environment_t        = unsafe extern "C" fn(cb: retro_environment_t);
pub type retro_set_video_refresh_t      = unsafe extern "C" fn(cb: retro_video_refresh_t);
pub type retro_set_audio_sample_t       = unsafe extern "C" fn(cb: retro_audio_sample_t);
pub type retro_set_audio_sample_batch_t = unsafe extern "C" fn(cb: retro_audio_sample_batch_t);
pub type retro_set_input_poll_t         = unsafe extern "C" fn(cb: retro_input_poll_t);
pub type retro_set_input_state_t        = unsafe extern "C" fn(cb: retro_input_state_t);
