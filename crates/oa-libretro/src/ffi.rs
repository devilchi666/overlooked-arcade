//! libretro ABI types + constants.
//!
//! Hand-translated subset of `libretro.h` covering everything our shell needs
//! to host a core. Names mirror the C names so cross-referencing upstream docs
//! is straightforward. Layouts use `#[repr(C)]` and the field order matches the
//! C structs byte-for-byte.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]

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

pub const RETRO_DEVICE_NONE: u32     = 0;
pub const RETRO_DEVICE_JOYPAD: u32   = 1;
pub const RETRO_DEVICE_MOUSE: u32    = 2;
pub const RETRO_DEVICE_KEYBOARD: u32 = 3;
pub const RETRO_DEVICE_LIGHTGUN: u32 = 4;
pub const RETRO_DEVICE_ANALOG: u32   = 5;
// RETRO_DEVICE_POINTER = 6 (declared below alongside its ID constants).

/// Number of bits reserved for the base device id when subclassing. A
/// "subclassed" device is `base | (subclass << 8)` — e.g. a Super NES
/// Mouse is conceptually `RETRO_DEVICE_MOUSE` but cores can declare a
/// specific subclass so per-game device-type pickers surface the right
/// option name. Today our shell doesn't synthesize subclasses (we use
/// the bare device types); kept here for the libretro callers that
/// may receive subclassed values back from cores.
pub const RETRO_DEVICE_TYPE_SHIFT: u32 = 8;
pub const RETRO_DEVICE_MASK: u32 = (1 << RETRO_DEVICE_TYPE_SHIFT) - 1;

// Analog stick index — passed as the `index` arg to cb_input_state when
// device == RETRO_DEVICE_ANALOG.
pub const RETRO_DEVICE_INDEX_ANALOG_LEFT: u32   = 0;
pub const RETRO_DEVICE_INDEX_ANALOG_RIGHT: u32  = 1;
pub const RETRO_DEVICE_INDEX_ANALOG_BUTTON: u32 = 2;

// Analog axis — passed as the `id` arg to cb_input_state.
pub const RETRO_DEVICE_ID_ANALOG_X: u32 = 0;
pub const RETRO_DEVICE_ID_ANALOG_Y: u32 = 1;

// Pointer device — touch screen / light gun / mouse-as-touch input.
// Used by Nintendo DS (stylus), Saturn light-gun games, Dreamcast House
// of the Dead, etc. Cores poll `cb_input_state(port, RETRO_DEVICE_POINTER,
// index, id)` per pointer; `id` selects which axis or the press state.
pub const RETRO_DEVICE_POINTER: u32 = 6;
pub const RETRO_DEVICE_ID_POINTER_X: u32       = 0;
pub const RETRO_DEVICE_ID_POINTER_Y: u32       = 1;
pub const RETRO_DEVICE_ID_POINTER_PRESSED: u32 = 2;
pub const RETRO_DEVICE_ID_POINTER_COUNT: u32   = 3;

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

// ---------- core option declaration structs ----------
//
// libretro evolved three formats for cores to declare their configurable
// options. Modern cores use V2 (categories + intl); older cores use V1
// (no categories) or the legacy "variables" format where each option's
// `value` field is a formatted string `"description; opt1|opt2|opt3"`.
//
// All option arrays are sentinel-terminated:
//   - V1/V2 definitions terminate at the first entry with `key = NULL`.
//   - The embedded `values` array inside each definition terminates at
//     the first entry with `value = NULL`.
//   - V2 categories terminate at the first entry with `key = NULL`.
//
// Capacity 128 matches `RETRO_NUM_CORE_OPTION_VALUES_MAX` from libretro.h.

pub const RETRO_NUM_CORE_OPTION_VALUES_MAX: usize = 128;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct retro_core_option_value {
    pub value: *const c_char,
    pub label: *const c_char,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct retro_core_option_definition {
    pub key: *const c_char,
    pub desc: *const c_char,
    pub info: *const c_char,
    pub values: [retro_core_option_value; RETRO_NUM_CORE_OPTION_VALUES_MAX],
    pub default_value: *const c_char,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct retro_core_options_intl {
    pub us: *mut retro_core_option_definition,
    pub local: *mut retro_core_option_definition,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct retro_core_option_v2_category {
    pub key: *const c_char,
    pub desc: *const c_char,
    pub info: *const c_char,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct retro_core_option_v2_definition {
    pub key: *const c_char,
    pub desc: *const c_char,
    pub desc_categorized: *const c_char,
    pub info: *const c_char,
    pub info_categorized: *const c_char,
    pub category_key: *const c_char,
    pub values: [retro_core_option_value; RETRO_NUM_CORE_OPTION_VALUES_MAX],
    pub default_value: *const c_char,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct retro_core_options_v2 {
    pub categories: *mut retro_core_option_v2_category,
    pub definitions: *mut retro_core_option_v2_definition,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct retro_core_options_v2_intl {
    pub us: *mut retro_core_options_v2,
    pub local: *mut retro_core_options_v2,
}

/// Per-option visibility hint passed by cores via
/// `RETRO_ENVIRONMENT_SET_CORE_OPTIONS_DISPLAY`. Cores call this to hide
/// options that don't apply given the current values of other options
/// (e.g. "Lightgun crosshair color" is meaningless when "Lightgun" is off).
#[repr(C)]
#[derive(Copy, Clone)]
pub struct retro_core_option_display {
    pub key: *const c_char,
    pub visible: bool,
}

/// Core-registered callback the frontend invokes whenever a core option
/// value changes (via `RETRO_ENVIRONMENT_SET_CORE_OPTIONS_UPDATE_DISPLAY_CALLBACK`).
/// On invocation the core re-evaluates which options should be hidden and
/// pushes the new visibility set back through `SET_CORE_OPTIONS_DISPLAY`.
/// Returns true on the FIRST call if any visibility was updated, and on
/// later calls if anything changed since the previous invocation.
pub type retro_core_options_update_display_callback_t =
    unsafe extern "C" fn() -> bool;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct retro_core_options_update_display_callback {
    pub callback: Option<retro_core_options_update_display_callback_t>,
}

// ---------- rumble (env 23, GET_RUMBLE_INTERFACE) -------------------
//
// Cores call set_rumble_state(port, effect, strength) to drive controller
// vibration. `strength` is 0..=65535 (max amplitude). `effect` is the
// motor: 0 = strong/low-freq, 1 = weak/high-freq. Cores typically poke
// both motors for a single rumble pulse so the controller buzzes with
// both motors at once.

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetroRumbleEffect {
    Strong = 0,
    Weak = 1,
}

pub type retro_set_rumble_state_t = unsafe extern "C" fn(
    port: u32,
    effect: u32,
    strength: u16,
) -> bool;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct retro_rumble_interface {
    pub set_rumble_state: retro_set_rumble_state_t,
}

// ---------- sensor (env 25, GET_SENSOR_INTERFACE) -------------------
//
// Cores call set_sensor_state(port, action, rate) to enable/disable
// the accelerometer + gyroscope on the given port, then poll values via
// get_sensor_input(port, id). Used by GBA tilt games (Kirby Tilt 'n'
// Tumble, WarioWare Twisted!), GBA solar games (Boktai), NDS gyroscope
// (WarioWare D.I.Y. Showcase), 3DS-style motion. We ship a mock-zero
// implementation today + keyboard arrow-key tilt fallback so games are
// playable without OS-level accelerometer access.

pub const RETRO_SENSOR_ACCELEROMETER_ENABLE: u32 = 0;
pub const RETRO_SENSOR_ACCELEROMETER_DISABLE: u32 = 1;
pub const RETRO_SENSOR_GYROSCOPE_ENABLE: u32 = 2;
pub const RETRO_SENSOR_GYROSCOPE_DISABLE: u32 = 3;
pub const RETRO_SENSOR_ILLUMINANCE_ENABLE: u32 = 4;
pub const RETRO_SENSOR_ILLUMINANCE_DISABLE: u32 = 5;

pub const RETRO_SENSOR_ACCELEROMETER_X: u32 = 0;
pub const RETRO_SENSOR_ACCELEROMETER_Y: u32 = 1;
pub const RETRO_SENSOR_ACCELEROMETER_Z: u32 = 2;
pub const RETRO_SENSOR_GYROSCOPE_X: u32 = 3;
pub const RETRO_SENSOR_GYROSCOPE_Y: u32 = 4;
pub const RETRO_SENSOR_GYROSCOPE_Z: u32 = 5;
pub const RETRO_SENSOR_ILLUMINANCE: u32 = 6;

pub type retro_set_sensor_state_t = unsafe extern "C" fn(
    port: u32,
    action: u32,
    rate: u32,
) -> bool;

pub type retro_sensor_get_input_t = unsafe extern "C" fn(
    port: u32,
    id: u32,
) -> f32;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct retro_sensor_interface {
    pub set_sensor_state: retro_set_sensor_state_t,
    pub get_sensor_input: retro_sensor_get_input_t,
}

// ---------- disc control callback structs ----------
//
// Cores with multi-disc support (PCE-CD with `.m3u`, PSX, Saturn, etc.)
// register an interface via SET_DISK_CONTROL_INTERFACE (v1) or
// SET_DISK_CONTROL_EXT_INTERFACE (v2). The frontend stores the function
// pointers and calls them when the user wants to swap discs.
//
// Disc swap protocol:
//   1. set_eject_state(true)  — open the virtual tray
//   2. set_image_index(N)     — load disc N
//   3. set_eject_state(false) — close the tray
// The core resumes reading from the new disc on its next frame.

pub type retro_set_eject_state_t   = unsafe extern "C" fn(ejected: bool) -> bool;
pub type retro_get_eject_state_t   = unsafe extern "C" fn() -> bool;
pub type retro_get_image_index_t   = unsafe extern "C" fn() -> u32;
pub type retro_set_image_index_t   = unsafe extern "C" fn(index: u32) -> bool;
pub type retro_get_num_images_t    = unsafe extern "C" fn() -> u32;
pub type retro_replace_image_index_t =
    unsafe extern "C" fn(index: u32, info: *const retro_game_info) -> bool;
pub type retro_add_image_index_t   = unsafe extern "C" fn() -> bool;
pub type retro_set_initial_image_t =
    unsafe extern "C" fn(index: u32, path: *const c_char) -> bool;
pub type retro_get_image_path_t    =
    unsafe extern "C" fn(index: u32, path: *mut c_char, len: usize) -> bool;
pub type retro_get_image_label_t   =
    unsafe extern "C" fn(index: u32, label: *mut c_char, len: usize) -> bool;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct retro_disk_control_callback {
    pub set_eject_state: Option<retro_set_eject_state_t>,
    pub get_eject_state: Option<retro_get_eject_state_t>,
    pub get_image_index: Option<retro_get_image_index_t>,
    pub set_image_index: Option<retro_set_image_index_t>,
    pub get_num_images: Option<retro_get_num_images_t>,
    pub replace_image_index: Option<retro_replace_image_index_t>,
    pub add_image_index: Option<retro_add_image_index_t>,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct retro_disk_control_ext_callback {
    pub set_eject_state: Option<retro_set_eject_state_t>,
    pub get_eject_state: Option<retro_get_eject_state_t>,
    pub get_image_index: Option<retro_get_image_index_t>,
    pub set_image_index: Option<retro_set_image_index_t>,
    pub get_num_images: Option<retro_get_num_images_t>,
    pub replace_image_index: Option<retro_replace_image_index_t>,
    pub add_image_index: Option<retro_add_image_index_t>,
    pub set_initial_image: Option<retro_set_initial_image_t>,
    pub get_image_path: Option<retro_get_image_path_t>,
    pub get_image_label: Option<retro_get_image_label_t>,
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
pub type retro_cheat_reset_t                = unsafe extern "C" fn();
pub type retro_cheat_set_t                  = unsafe extern "C" fn(index: u32, enabled: bool, code: *const c_char);

pub type retro_set_environment_t        = unsafe extern "C" fn(cb: retro_environment_t);
pub type retro_set_video_refresh_t      = unsafe extern "C" fn(cb: retro_video_refresh_t);
pub type retro_set_audio_sample_t       = unsafe extern "C" fn(cb: retro_audio_sample_t);
pub type retro_set_audio_sample_batch_t = unsafe extern "C" fn(cb: retro_audio_sample_batch_t);
pub type retro_set_input_poll_t         = unsafe extern "C" fn(cb: retro_input_poll_t);
pub type retro_set_input_state_t        = unsafe extern "C" fn(cb: retro_input_state_t);

// ---------- keyboard ----------
//
// libretro keyboard support is the "computer-shaped systems" path — MAME's
// service / TAB menu, MSX BASIC, every future home-computer core. The core
// registers a `retro_keyboard_event_t` via `RETRO_ENVIRONMENT_SET_KEYBOARD_
// CALLBACK`; the frontend then calls that function pointer whenever a key
// transitions, passing the libretro `retro_key` code (NOT the OS scancode),
// a unicode character (or 0 if not a printable transition), and a bitmask
// of currently-held modifiers.
//
// Constants below mirror `libretro.h` enum `retro_key` byte-for-byte; the
// values are stable and many MAME/Mednafen cores reference them by number.

pub type retro_keyboard_event_t = unsafe extern "C" fn(
    down: bool,
    keycode: u32,
    character: u32,
    key_modifiers: u16,
);

#[repr(C)]
#[derive(Copy, Clone)]
pub struct retro_keyboard_callback {
    pub callback: Option<retro_keyboard_event_t>,
}

// retro_mod — bitmask of currently-held modifiers passed alongside each
// keyboard event. Mirrors `libretro.h` enum `retro_mod`.
pub const RETROKMOD_NONE:       u16 = 0x0000;
pub const RETROKMOD_SHIFT:      u16 = 0x0001;
pub const RETROKMOD_CTRL:       u16 = 0x0002;
pub const RETROKMOD_ALT:        u16 = 0x0004;
pub const RETROKMOD_META:       u16 = 0x0008;
pub const RETROKMOD_NUMLOCK:    u16 = 0x0010;
pub const RETROKMOD_CAPSLOCK:   u16 = 0x0020;
pub const RETROKMOD_SCROLLLOCK: u16 = 0x0040;

// retro_key — libretro's own keycode space. Roughly SDL1-shaped with a
// handful of additions; not the same as Windows VK or X11 keysyms.
// Values are stable across every core that uses keyboard input.
pub const RETROK_UNKNOWN:    u32 = 0;
pub const RETROK_BACKSPACE:  u32 = 8;
pub const RETROK_TAB:        u32 = 9;
pub const RETROK_CLEAR:      u32 = 12;
pub const RETROK_RETURN:     u32 = 13;
pub const RETROK_PAUSE:      u32 = 19;
pub const RETROK_ESCAPE:     u32 = 27;
pub const RETROK_SPACE:      u32 = 32;
pub const RETROK_EXCLAIM:    u32 = 33;
pub const RETROK_QUOTEDBL:   u32 = 34;
pub const RETROK_HASH:       u32 = 35;
pub const RETROK_DOLLAR:     u32 = 36;
pub const RETROK_AMPERSAND:  u32 = 38;
pub const RETROK_QUOTE:      u32 = 39;
pub const RETROK_LEFTPAREN:  u32 = 40;
pub const RETROK_RIGHTPAREN: u32 = 41;
pub const RETROK_ASTERISK:   u32 = 42;
pub const RETROK_PLUS:       u32 = 43;
pub const RETROK_COMMA:      u32 = 44;
pub const RETROK_MINUS:      u32 = 45;
pub const RETROK_PERIOD:     u32 = 46;
pub const RETROK_SLASH:      u32 = 47;
pub const RETROK_0: u32 = 48;
pub const RETROK_1: u32 = 49;
pub const RETROK_2: u32 = 50;
pub const RETROK_3: u32 = 51;
pub const RETROK_4: u32 = 52;
pub const RETROK_5: u32 = 53;
pub const RETROK_6: u32 = 54;
pub const RETROK_7: u32 = 55;
pub const RETROK_8: u32 = 56;
pub const RETROK_9: u32 = 57;
pub const RETROK_COLON:        u32 = 58;
pub const RETROK_SEMICOLON:    u32 = 59;
pub const RETROK_LESS:         u32 = 60;
pub const RETROK_EQUALS:       u32 = 61;
pub const RETROK_GREATER:      u32 = 62;
pub const RETROK_QUESTION:     u32 = 63;
pub const RETROK_AT:           u32 = 64;
pub const RETROK_LEFTBRACKET:  u32 = 91;
pub const RETROK_BACKSLASH:    u32 = 92;
pub const RETROK_RIGHTBRACKET: u32 = 93;
pub const RETROK_CARET:        u32 = 94;
pub const RETROK_UNDERSCORE:   u32 = 95;
pub const RETROK_BACKQUOTE:    u32 = 96;
pub const RETROK_a: u32 = 97;
pub const RETROK_b: u32 = 98;
pub const RETROK_c: u32 = 99;
pub const RETROK_d: u32 = 100;
pub const RETROK_e: u32 = 101;
pub const RETROK_f: u32 = 102;
pub const RETROK_g: u32 = 103;
pub const RETROK_h: u32 = 104;
pub const RETROK_i: u32 = 105;
pub const RETROK_j: u32 = 106;
pub const RETROK_k: u32 = 107;
pub const RETROK_l: u32 = 108;
pub const RETROK_m: u32 = 109;
pub const RETROK_n: u32 = 110;
pub const RETROK_o: u32 = 111;
pub const RETROK_p: u32 = 112;
pub const RETROK_q: u32 = 113;
pub const RETROK_r: u32 = 114;
pub const RETROK_s: u32 = 115;
pub const RETROK_t: u32 = 116;
pub const RETROK_u: u32 = 117;
pub const RETROK_v: u32 = 118;
pub const RETROK_w: u32 = 119;
pub const RETROK_x: u32 = 120;
pub const RETROK_y: u32 = 121;
pub const RETROK_z: u32 = 122;
pub const RETROK_LEFTBRACE:  u32 = 123;
pub const RETROK_BAR:        u32 = 124;
pub const RETROK_RIGHTBRACE: u32 = 125;
pub const RETROK_TILDE:      u32 = 126;
pub const RETROK_DELETE:     u32 = 127;

pub const RETROK_KP0: u32 = 256;
pub const RETROK_KP1: u32 = 257;
pub const RETROK_KP2: u32 = 258;
pub const RETROK_KP3: u32 = 259;
pub const RETROK_KP4: u32 = 260;
pub const RETROK_KP5: u32 = 261;
pub const RETROK_KP6: u32 = 262;
pub const RETROK_KP7: u32 = 263;
pub const RETROK_KP8: u32 = 264;
pub const RETROK_KP9: u32 = 265;
pub const RETROK_KP_PERIOD:   u32 = 266;
pub const RETROK_KP_DIVIDE:   u32 = 267;
pub const RETROK_KP_MULTIPLY: u32 = 268;
pub const RETROK_KP_MINUS:    u32 = 269;
pub const RETROK_KP_PLUS:     u32 = 270;
pub const RETROK_KP_ENTER:    u32 = 271;
pub const RETROK_KP_EQUALS:   u32 = 272;

pub const RETROK_UP:       u32 = 273;
pub const RETROK_DOWN:     u32 = 274;
pub const RETROK_RIGHT:    u32 = 275;
pub const RETROK_LEFT:     u32 = 276;
pub const RETROK_INSERT:   u32 = 277;
pub const RETROK_HOME:     u32 = 278;
pub const RETROK_END:      u32 = 279;
pub const RETROK_PAGEUP:   u32 = 280;
pub const RETROK_PAGEDOWN: u32 = 281;

pub const RETROK_F1:  u32 = 282;
pub const RETROK_F2:  u32 = 283;
pub const RETROK_F3:  u32 = 284;
pub const RETROK_F4:  u32 = 285;
pub const RETROK_F5:  u32 = 286;
pub const RETROK_F6:  u32 = 287;
pub const RETROK_F7:  u32 = 288;
pub const RETROK_F8:  u32 = 289;
pub const RETROK_F9:  u32 = 290;
pub const RETROK_F10: u32 = 291;
pub const RETROK_F11: u32 = 292;
pub const RETROK_F12: u32 = 293;
pub const RETROK_F13: u32 = 294;
pub const RETROK_F14: u32 = 295;
pub const RETROK_F15: u32 = 296;

pub const RETROK_NUMLOCK:    u32 = 300;
pub const RETROK_CAPSLOCK:   u32 = 301;
pub const RETROK_SCROLLOCK:  u32 = 302;
pub const RETROK_RSHIFT:     u32 = 303;
pub const RETROK_LSHIFT:     u32 = 304;
pub const RETROK_RCTRL:      u32 = 305;
pub const RETROK_LCTRL:      u32 = 306;
pub const RETROK_RALT:       u32 = 307;
pub const RETROK_LALT:       u32 = 308;
pub const RETROK_RMETA:      u32 = 309;
pub const RETROK_LMETA:      u32 = 310;
pub const RETROK_LSUPER:     u32 = 311;
pub const RETROK_RSUPER:     u32 = 312;
pub const RETROK_MODE:       u32 = 313;
pub const RETROK_COMPOSE:    u32 = 314;
pub const RETROK_HELP:       u32 = 315;
pub const RETROK_PRINT:      u32 = 316;
pub const RETROK_SYSREQ:     u32 = 317;
pub const RETROK_BREAK:      u32 = 318;
pub const RETROK_MENU:       u32 = 319;
pub const RETROK_POWER:      u32 = 320;
pub const RETROK_EURO:       u32 = 321;
pub const RETROK_UNDO:       u32 = 322;
pub const RETROK_OEM_102:    u32 = 323;
