//! Static callback state + extern "C" trampolines.
//!
//! libretro callbacks are plain `extern "C"` function pointers that can't
//! carry closure state, so the data lives in a process-global Mutex. Cores
//! are singletons anyway (every libretro core we've seen keeps module-globals
//! in C land), so a Mutex<Option<State>> matches reality.
//!
//! All callbacks fire from `retro_run`, which is driven by the emu thread, so
//! the Mutex is theoretically uncontended — it's a safety net, not a hot path
//! contention point.

use std::collections::{HashMap, HashSet};
use std::ffi::{c_void, CStr, CString};
use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::{LazyLock, Mutex};

use oa_core::{
    ControllerDeviceDescriptor, CoreOption, CoreOptionCategory, CoreOptionValue,
    InputDescriptor,
};

use crate::ffi::*;
use crate::pixel;

// ---- log_interface bridge ----
//
// Cores call libretro's variadic `retro_log_printf_t` (`void(level, fmt, ...)`)
// to print messages. Defining a variadic Rust function requires the unstable
// `c_variadic` feature, so the variadic frame lives in C (see
// `log_trampoline.c`) which vsnprintfs into a stack buffer then calls back
// into `oa_libretro_log_forward` here with a finished string.

extern "C" {
    /// Defined in `log_trampoline.c`. Variadic in C; declared with `...` in
    /// Rust so the function pointer's type is `retro_log_printf_t`-compatible
    /// when handed to a core via `RETRO_ENVIRONMENT_GET_LOG_INTERFACE`.
    fn oa_libretro_log_trampoline(level: u32, fmt: *const c_char, ...);
}

/// Called by the C trampoline with the already-formatted message. Routes
/// through the `log` crate so core output interleaves with our regular logging.
#[no_mangle]
pub extern "C" fn oa_libretro_log_forward(level: u32, msg: *const c_char) {
    if msg.is_null() {
        return;
    }
    let s = unsafe { CStr::from_ptr(msg) }.to_string_lossy();
    let trimmed = s.trim_end_matches(['\n', '\r']);
    if trimmed.is_empty() {
        return;
    }
    match level {
        0 => log::debug!("core: {trimmed}"),
        1 => log::info!("core: {trimmed}"),
        2 => log::warn!("core: {trimmed}"),
        _ => log::error!("core: {trimmed}"),
    }
}

/// Per-core mutable state owned by our callbacks.
pub(crate) struct State {
    pub pix_fmt: PixelFormat,
    pub fb_rgba: Vec<u8>,
    pub fb_width: u32,
    pub fb_height: u32,
    pub audio: Vec<i16>,
    pub input_bits: [u16; 5],
    /// Per-port analog axes. Layout: `[port][axis]` where axis is
    /// `[Left X, Left Y, Right X, Right Y]` (matches the `InputState::axes`
    /// field on the oa_core side). Values are i16 in the libretro range
    /// (-32768 .. 32767). cb_input_state returns these for
    /// RETRO_DEVICE_ANALOG queries. Set per frame via
    /// LibretroCore::set_input.
    pub input_axes: [[i16; 4]; 5],
    /// Per-port pointer device state. Layout:
    /// `(x, y, pressed, in_viewport)` per port. Used by Nintendo DS
    /// stylus, Saturn / Dreamcast / arcade light-gun games. Set per
    /// frame via LibretroCore::set_input from `InputState.pointer`;
    /// cb_input_state returns the right component for
    /// RETRO_DEVICE_POINTER queries (x/y/pressed) AND
    /// RETRO_DEVICE_LIGHTGUN queries (the `in_viewport` flag drives
    /// `RETRO_DEVICE_ID_LIGHTGUN_IS_OFFSCREEN` so cores can detect
    /// the reload-by-aim-off-screen gesture).
    pub input_pointer: [(i16, i16, bool, bool); 5],
    /// Per-port secondary pointer for multi-touch dispatch. Cores
    /// polling `RETRO_DEVICE_POINTER` with `index = 1` receive
    /// fields from this tuple; `index = 0` reads `input_pointer`;
    /// `index ≥ 2` returns zero. `RETRO_DEVICE_ID_POINTER_COUNT`
    /// reports how many pointers are currently pressed across both
    /// slots (0 / 1 / 2). Mirrors `InputState.pointer_secondary`
    /// via LibretroCore::set_input.
    pub input_pointer_secondary: [(i16, i16, bool, bool); 5],
    /// Per-port LIGHTGUN gun-side button bitmask. Bit position ==
    /// `RETRO_DEVICE_ID_LIGHTGUN_*`. Mirrors `InputState.lightgun_buttons`
    /// via LibretroCore::set_input. `lightgun_field_value` reads the
    /// matching bit for AUX_A/B/C, START, SELECT, DPAD_{UP,DOWN,LEFT,RIGHT},
    /// and RELOAD; TRIGGER stays driven by `input_pointer.pressed`.
    pub input_lightgun_buttons: [u32; 5],
    /// Per-port analog-button pressure. Layout: `[port][button_id]`
    /// where `button_id` matches the RETRO_DEVICE_ID_JOYPAD_* bit
    /// position (0-15). Range 0..32767. Cores polling
    /// `cb_input_state(port, RETRO_DEVICE_ANALOG,
    /// RETRO_DEVICE_INDEX_ANALOG_BUTTON, button_id)` get the pressure
    /// for that button — used by Gran Turismo brake-pressure,
    /// Metal Gear Solid prone-walk, GameCube L/R triggers, PS2/DC
    /// pressure-sensitive face buttons. Set per frame via
    /// LibretroCore::set_input from `InputState.analog_buttons`.
    pub input_analog_buttons: [[i16; 16]; 5],
    /// Snapshotted display aspect (final image W:H, 0.0 = caller falls back to width:height).
    pub display_aspect: f32,
    /// Path/dir/ext strings the core may request via environment callbacks.
    /// CStrings own the bytes; the raw pointers we hand back must stay valid
    /// until the next env call. We hold them in State for that lifetime.
    pub system_dir: CString,
    pub save_dir: CString,
    /// In-flight ROM pointer for GET_GAME_INFO_EXT.
    pub pending_rom_data: *const u8,
    pub pending_rom_size: usize,
    pub pending_name: CString,
    pub pending_ext: CString,
    /// Synthetic file path handed to the core via
    /// `retro_game_info_ext.full_path`. Must end with `.<ext>` because
    /// Genesis Plus GX (and any other libretro core that detects the
    /// system from the filename's trailing characters) reads
    /// `&filename[strlen(filename) - 3]` directly — an empty string
    /// makes that out-of-bounds. For Bytes-source loads we synthesize
    /// `"<name>.<ext>"`; for Path-source loads we use the real path.
    pub pending_full_path: CString,
    pub pending_info_ext: retro_game_info_ext,
    /// Whether the core has been initialised (retro_init called). Drop uses this
    /// to decide whether retro_deinit is needed.
    pub initialised: bool,
    /// Parsed option definitions registered by the core via one of the
    /// libretro core-option environment callbacks (SET_VARIABLES /
    /// SET_CORE_OPTIONS / SET_CORE_OPTIONS_V2 + their _INTL variants).
    /// Populated during `retro_set_environment` or `retro_load_game`.
    pub core_options: Vec<CoreOption>,
    /// V2 categories. Empty for cores using V1 / legacy variables.
    pub option_categories: Vec<CoreOptionCategory>,
    /// User-overridden option values. Keyed by `CoreOption::key`. The
    /// `CString` is held here so the pointer we hand back via
    /// `GET_VARIABLE` stays valid until the next write — cores typically
    /// re-read at the top of each frame, so the pointer's lifetime only
    /// needs to span one frame, but holding it indefinitely costs ~nothing.
    pub option_values: HashMap<String, CString>,
    /// Set whenever `option_values` is mutated. Cores poll
    /// `GET_VARIABLE_UPDATE`; returning `true` once + clearing tells the
    /// core "values changed, re-read all your variables." Initial value
    /// `true` is intentional — on the first poll cores expect to be told
    /// "yes, fetch your initial values now."
    pub variables_updated: bool,
    /// Option keys the core has marked NOT visible via
    /// `RETRO_ENVIRONMENT_SET_CORE_OPTIONS_DISPLAY`. Cores call this to
    /// hide options that don't apply given the current values of other
    /// options (e.g. "Lightgun crosshair color" is hidden when "Lightgun"
    /// is off). Cleared whenever a fresh schema arrives; mutated each
    /// time the core invokes the SET_CORE_OPTIONS_DISPLAY env (either
    /// during init or from inside the update-display callback after a
    /// value change).
    pub hidden_options: HashSet<String>,
    /// Callback the core registered via
    /// `RETRO_ENVIRONMENT_SET_CORE_OPTIONS_UPDATE_DISPLAY_CALLBACK`. The
    /// frontend invokes this after writing a new option value so the
    /// core can re-evaluate which options should be hidden and push the
    /// updated set back through `SET_CORE_OPTIONS_DISPLAY`. `None` for
    /// cores that don't support dynamic visibility (most do not — they
    /// only push initial visibility once during load).
    pub update_display_cb: Option<retro_core_options_update_display_callback_t>,
    /// Disc control v1 callbacks, registered via SET_DISK_CONTROL_INTERFACE.
    /// `None` until the core registers; cores without multi-disc support
    /// never call this env, in which case all the disc-control methods on
    /// LibretroCore return as-if-empty.
    pub disk_v1: Option<retro_disk_control_callback>,
    /// Disc control v2 callbacks. When `Some`, the frontend prefers this
    /// over `disk_v1` (it carries label + path getters too). Cores typically
    /// register one or the other — modern Beetle cores use v2.
    pub disk_v2: Option<retro_disk_control_ext_callback>,
    /// Keyboard-event callback registered by the core via
    /// `RETRO_ENVIRONMENT_SET_KEYBOARD_CALLBACK`. Cores that want raw key
    /// events (MAME for its operator-test / TAB menu, MSX for BASIC, every
    /// future computer-shaped core) register this; the frontend then calls
    /// it on every key transition for ports wired up as RETRO_DEVICE_KEYBOARD.
    /// `None` until the core registers — cores that never call the
    /// SET_KEYBOARD_CALLBACK env leave this unset and the frontend's
    /// `send_keyboard_event` becomes a no-op.
    pub keyboard_cb: Option<retro_keyboard_event_t>,
    /// Latest rumble strength the core requested, per (port × effect).
    /// `rumble[port][0]` is strong/low-freq, `rumble[port][1]` is weak/
    /// high-freq. Range 0..=65535. Updated by `cb_set_rumble_state`;
    /// the shell polls `LibretroCore::rumble_snapshot()` once per frame
    /// and forwards to `oa-input::InputPoller::dispatch_rumble` which
    /// pokes gilrs's force-feedback API.
    pub rumble: [[u16; 2]; 5],
    /// Per-port-per-sensor enable bits set by the core via
    /// `cb_set_sensor_state`. Index by `[port][sensor_class]` where
    /// class 0 = accelerometer, 1 = gyroscope, 2 = illuminance. The
    /// frontend only writes non-zero into `cb_get_sensor_input` when
    /// the matching class is enabled, so disabled sensors read 0.
    pub sensor_enabled: [[bool; 3]; 5],
    /// Per-port-per-axis sensor value populated by the shell each
    /// frame for whatever sensor classes are enabled. Index by
    /// `[port][RETRO_SENSOR_* axis id]` (0..=6). The Phase G
    /// implementation feeds these from keyboard arrow keys as a
    /// tilt fallback so GBA Boktai / Kirby Tilt 'n' Tumble /
    /// WarioWare Twisted! are playable without an OS-level
    /// accelerometer. Real motion is a separate later phase.
    pub sensor_values: [[f32; 7]; 5],
    /// Whether the core advertised `SET_SUPPORT_NO_GAME = true` during
    /// init. Cores like DOSBox-Pure (built-in DOS-game browser) and
    /// ScummVM (engine launcher with built-in game list) set this to
    /// signal "you may call `retro_load_game(NULL)` and I'll boot into
    /// my own menu / browser." Cores that need content always leave
    /// this `false`; calling `load_no_rom()` against one of those
    /// returns an error from `retro_load_game`.
    ///
    /// The flag is captured once during `retro_set_environment` (the
    /// only time SET_SUPPORT_NO_GAME is allowed per spec); it does
    /// not reset between loads.
    pub supports_no_game: bool,
    /// OSD-style status messages the core has pushed via
    /// `SET_MESSAGE` / `SET_MESSAGE_EXT`. Drained per frame by the
    /// emu thread, which fires each entry as a `oa://toast` event to
    /// the frontend. Stays small — most cores send <1 message/sec on
    /// state changes; we cap implicitly at whatever a frame's worth
    /// of bursts could produce.
    pub pending_messages: Vec<oa_core::CoreMessage>,
    /// Per-port supported-device list, captured from the core's
    /// `RETRO_ENVIRONMENT_SET_CONTROLLER_INFO` env call. Index by port
    /// (typically 0..=4 — cores can advertise more or fewer). An empty
    /// outer Vec means the core never published; an empty inner Vec at
    /// a given port means the core declared that port supports nothing.
    /// Cleared when the core .dll is reloaded (State is freshly
    /// constructed per `reference_libretro_mednafen_unload_then_load_no_gap`).
    pub controller_devices: Vec<Vec<ControllerDeviceDescriptor>>,
    /// Per-(port, device, index, id) → "human label" tuples captured
    /// from the core's `RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS` env
    /// call. Cores publish these PER-GAME (different label table for
    /// each loaded ROM), and may re-publish across a game's lifetime
    /// when modes change (fighting-game character-select vs in-match);
    /// the parser overwrites on each call so the latest publish wins.
    /// Cleared on core reload alongside `controller_devices`.
    pub input_descriptors: Vec<InputDescriptor>,
    /// Minimum audio latency in milliseconds the core requested via
    /// `RETRO_ENVIRONMENT_SET_MINIMUM_AUDIO_LATENCY` (env 63). Cores
    /// set this when they expect CPU-heavy frames that briefly stall
    /// audio production — Genesis Plus GX during heavy FM-synth scenes,
    /// Beetle PSX during GPU-heavy moments, Flycast during 3D-heavy
    /// areas. The shell uses this on every core load to size oa-audio's
    /// ring buffer to at least this many ms of headroom, preventing
    /// crackling under those conditions.
    ///
    /// Per spec (libretro.h:1369): 0 = "use default frontend latency"
    /// (no override). Values larger than current frontend latency take
    /// effect; smaller values have no effect. Spec asks frontends to
    /// honor requests up to 512 ms. We cap parser output at the spec's
    /// 512 ms ceiling to defend against a buggy core declaring an
    /// absurd value that would balloon the ring buffer.
    pub min_audio_latency_ms: u32,
    /// Parsed memory descriptors from the core's `SET_MEMORY_MAPS`
    /// call. Empty for cores that never publish one. Host pointers
    /// stay private to oa-libretro (stored in `memory_map_ptrs`); only
    /// the metadata copies cross to oa-core via `Core::memory_map()`.
    /// Cleared on `load_rom` so stale descriptors from a previous game
    /// don't leak through.
    pub memory_descriptors: Vec<oa_core::MemoryDescriptor>,
    /// Host base pointers for each descriptor in `memory_descriptors`,
    /// indexed in parallel. Stored as `usize` so the State stays `Send`
    /// (raw pointers aren't auto-Send). Reserved for future rcheevos
    /// or scripting integration that wants to translate a guest
    /// address → host byte; the current shell doesn't read these.
    pub memory_map_ptrs: Vec<usize>,
    /// Display rotation set by the core via
    /// `RETRO_ENVIRONMENT_SET_ROTATION`. Units of 90° clockwise:
    ///   0 = no rotation (default)
    ///   1 = 90° clockwise
    ///   2 = 180°
    ///   3 = 270° clockwise (== 90° counter-clockwise)
    /// Vertical arcade boards like Pac-Man / Galaxian / Donkey Kong set
    /// `1` or `3`; non-rotated systems leave this at `0`. The shell
    /// reads `LibretroCore::rotation()` after `retro_load_game` returns
    /// and pushes the value to the renderer for viewport math.
    pub rotation: u32,
    /// HW-render callback the core registered via `SET_HW_RENDER` (env 14)
    /// — carries the requested context type (Vulkan/GL/D3D), API version,
    /// and the core's `context_reset` / `context_destroy` pointers. `None`
    /// for the software cores (the common case). Captured in the M1 probe
    /// phase; the device-build commit fills its frontend-provided fields
    /// (`get_proc_address` / `get_current_framebuffer`) and drives
    /// `context_reset`. See docs/PLANS/hw-render-pipeline.md.
    #[allow(dead_code)] // read by the M1 device-build commit (next).
    pub hw_render_callback: Option<retro_hw_render_callback>,
    /// Vulkan context-negotiation interface the core handed us via env 43.
    /// The M1 standalone-device build calls its `create_device` to stand up
    /// a `VkDevice` with exactly the extensions/features the core needs.
    /// `None` until a Vulkan HW core registers one.
    #[allow(dead_code)] // read by the M1 device-build commit (next).
    pub hw_negotiation_vulkan: Option<retro_hw_render_context_negotiation_interface_vulkan>,
}

// SAFETY: raw pointers stored in `pending_rom_data` are only dereferenced
// inside `retro_load_game` (immediately after we set them in load_rom) and
// nulled out right after. Mutex serializes access; the pointer is never read
// across threads even though Send is technically a lie about *const u8.
unsafe impl Send for State {}

impl State {
    pub fn new() -> Self {
        // PCE's widest mode is 512×242 — pre-size for 1024×512 RGBA8 (≈2 MB)
        // so resizes are rare. Same constants as shim.cpp.
        const FB_MAX_W: usize = 1024;
        const FB_MAX_H: usize = 512;
        Self {
            pix_fmt: PixelFormat::Xrgb8888,
            fb_rgba: vec![0; FB_MAX_W * FB_MAX_H * 4],
            fb_width: 256,
            fb_height: 240,
            audio: Vec::with_capacity(16384),
            input_bits: [0; 5],
            input_axes: [[0; 4]; 5],
            input_pointer: [(0, 0, false, false); 5],
            input_pointer_secondary: [(0, 0, false, false); 5],
            input_lightgun_buttons: [0; 5],
            input_analog_buttons: [[0; 16]; 5],
            display_aspect: 0.0,
            system_dir: CString::new(".").unwrap(),
            save_dir: CString::new(".").unwrap(),
            pending_rom_data: std::ptr::null(),
            pending_rom_size: 0,
            pending_name: CString::new("rom").unwrap(),
            pending_ext: CString::new("").unwrap(),
            pending_full_path: CString::new("").unwrap(),
            pending_info_ext: zeroed_info_ext(),
            initialised: false,
            core_options: Vec::new(),
            option_categories: Vec::new(),
            option_values: HashMap::new(),
            variables_updated: true,
            hidden_options: HashSet::new(),
            update_display_cb: None,
            rumble: [[0; 2]; 5],
            sensor_enabled: [[false; 3]; 5],
            sensor_values: [[0.0; 7]; 5],
            disk_v1: None,
            disk_v2: None,
            keyboard_cb: None,
            supports_no_game: false,
            pending_messages: Vec::new(),
            controller_devices: Vec::new(),
            input_descriptors: Vec::new(),
            min_audio_latency_ms: 0,
            memory_descriptors: Vec::new(),
            memory_map_ptrs: Vec::new(),
            rotation: 0,
            hw_render_callback: None,
            hw_negotiation_vulkan: None,
        }
    }

    /// Set a single user-overridden option value + raise the update flag.
    /// Called from `LibretroCore::set_option`. `value` must be valid UTF-8;
    /// the wrapper converts it to a `CString` for stable C pointer life.
    pub fn set_option_value(&mut self, key: &str, value: &str) {
        if let Ok(cstr) = CString::new(value) {
            self.option_values.insert(key.to_string(), cstr);
            self.variables_updated = true;
        }
    }

    pub fn clear_pending(&mut self) {
        self.pending_rom_data = std::ptr::null();
        self.pending_rom_size = 0;
    }
}

fn zeroed_info_ext() -> retro_game_info_ext {
    retro_game_info_ext {
        full_path: std::ptr::null(),
        archive_path: std::ptr::null(),
        archive_file: std::ptr::null(),
        dir: std::ptr::null(),
        name: std::ptr::null(),
        ext: std::ptr::null(),
        meta: std::ptr::null(),
        data: std::ptr::null(),
        size: 0,
        file_in_archive: false,
        persistent_data: true,
    }
}

/// Stable empty C-string used to fill `retro_game_info_ext` fields we don't
/// have real values for (e.g. `archive_path` for a non-archived ROM). The
/// libretro spec allows NULL for these, but at least one widely-used core
/// (FCEUmm 2026-05-18) dereferences them without null-checking and crashes.
/// Pointing them at a valid "" instead of NULL costs ~nothing and dodges
/// the issue across every core that has the same bug class.
static EMPTY_CSTR: LazyLock<CString> = LazyLock::new(|| CString::new("").unwrap());

// ---- core-option declaration parsers --------------------------------
//
// Cores register their option schemas via one of three libretro callbacks:
//
//   SET_VARIABLES (legacy, all cores still ship)  — array of retro_variable,
//     each `value` formatted as `"description; opt1|opt2|opt3"`. First
//     option is the default.
//   SET_CORE_OPTIONS / SET_CORE_OPTIONS_INTL (V1) — array of
//     retro_core_option_definition with split desc/info + values array.
//   SET_CORE_OPTIONS_V2 / SET_CORE_OPTIONS_V2_INTL — adds categories.
//
// All arrays terminate at the sentinel (first entry whose key is NULL).
//
// SAFETY: callers must ensure the array pointer remains valid for the
// duration of the call. libretro cores typically declare options from
// static memory so this is fine, but we copy out all strings into owned
// Rust types so the State's snapshot doesn't alias core memory.

unsafe fn cstr_to_string(p: *const c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    Some(unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
}

unsafe fn cstr_to_string_default(p: *const c_char) -> String {
    unsafe { cstr_to_string(p) }.unwrap_or_default()
}

/// Walk the `values` array embedded in a V1 / V2 option definition. Stops
/// at the first entry with a NULL `value` pointer.
unsafe fn parse_option_values(
    values: &[retro_core_option_value; RETRO_NUM_CORE_OPTION_VALUES_MAX],
) -> Vec<CoreOptionValue> {
    let mut out = Vec::new();
    for v in values.iter() {
        if v.value.is_null() {
            break;
        }
        out.push(CoreOptionValue {
            value: unsafe { cstr_to_string_default(v.value) },
            label: unsafe { cstr_to_string(v.label) },
        });
    }
    out
}

/// Parse the legacy `retro_variable` array. The `value` field is a
/// formatted string: `"Display description; option1|option2|option3"`.
/// First option in the pipe-separated list is treated as the default.
unsafe fn parse_legacy_variables(arr: *const retro_variable) -> Vec<CoreOption> {
    let mut out = Vec::new();
    if arr.is_null() {
        return out;
    }
    let mut i = 0isize;
    loop {
        let entry = unsafe { &*arr.offset(i) };
        if entry.key.is_null() {
            break;
        }
        let key = unsafe { cstr_to_string_default(entry.key) };
        let raw_value = unsafe { cstr_to_string(entry.value) }.unwrap_or_default();
        let (desc, options_str) = match raw_value.split_once(';') {
            Some((d, o)) => (d.trim().to_string(), o.trim()),
            None => (raw_value.clone(), raw_value.as_str()),
        };
        let values: Vec<CoreOptionValue> = options_str
            .split('|')
            .filter(|s| !s.is_empty())
            .map(|s| CoreOptionValue { value: s.trim().to_string(), label: None })
            .collect();
        if values.is_empty() {
            i += 1;
            continue;
        }
        let default_value = values[0].value.clone();
        out.push(CoreOption { key, desc, info: None, category_key: None, default_value, values });
        i += 1;
    }
    out
}

/// Parse the V1 `retro_core_option_definition` array.
unsafe fn parse_core_options_v1(arr: *const retro_core_option_definition) -> Vec<CoreOption> {
    let mut out = Vec::new();
    if arr.is_null() {
        return out;
    }
    let mut i = 0isize;
    loop {
        let entry = unsafe { &*arr.offset(i) };
        if entry.key.is_null() {
            break;
        }
        let values = unsafe { parse_option_values(&entry.values) };
        if values.is_empty() {
            i += 1;
            continue;
        }
        let default_value = unsafe { cstr_to_string(entry.default_value) }
            .unwrap_or_else(|| values[0].value.clone());
        out.push(CoreOption {
            key: unsafe { cstr_to_string_default(entry.key) },
            desc: unsafe { cstr_to_string_default(entry.desc) },
            info: unsafe { cstr_to_string(entry.info) },
            category_key: None,
            default_value,
            values,
        });
        i += 1;
    }
    out
}

/// Parse the V2 `retro_core_option_v2_definition` array (with categories).
unsafe fn parse_core_options_v2(arr: *const retro_core_option_v2_definition) -> Vec<CoreOption> {
    let mut out = Vec::new();
    if arr.is_null() {
        return out;
    }
    let mut i = 0isize;
    loop {
        let entry = unsafe { &*arr.offset(i) };
        if entry.key.is_null() {
            break;
        }
        let values = unsafe { parse_option_values(&entry.values) };
        if values.is_empty() {
            i += 1;
            continue;
        }
        let default_value = unsafe { cstr_to_string(entry.default_value) }
            .unwrap_or_else(|| values[0].value.clone());
        out.push(CoreOption {
            key: unsafe { cstr_to_string_default(entry.key) },
            desc: unsafe { cstr_to_string_default(entry.desc) },
            info: unsafe { cstr_to_string(entry.info) },
            category_key: unsafe { cstr_to_string(entry.category_key) },
            default_value,
            values,
        });
        i += 1;
    }
    out
}

/// Walk the `retro_memory_descriptor` array and copy out the metadata
/// fields into owned [`oa_core::MemoryDescriptor`] values. The host
/// base pointers are returned in a parallel `Vec<usize>` so the State
/// stays `Send` (raw pointers aren't auto-Send) and so consumers that
/// don't need them never see them.
///
/// SAFETY: caller must ensure `arr` points at `n` valid
/// `retro_memory_descriptor` entries. The pointed-to `addrspace`
/// strings are read with `CStr::from_ptr` so they must be NUL-
/// terminated; `addrspace = NULL` is allowed and yields `None`.
unsafe fn parse_memory_descriptors(
    arr: *const retro_memory_descriptor,
    n: u32,
) -> (Vec<oa_core::MemoryDescriptor>, Vec<usize>) {
    let mut descs = Vec::with_capacity(n as usize);
    let mut ptrs = Vec::with_capacity(n as usize);
    if arr.is_null() || n == 0 {
        return (descs, ptrs);
    }
    for i in 0..n as isize {
        let d = unsafe { &*arr.offset(i) };
        descs.push(oa_core::MemoryDescriptor {
            flags: d.flags,
            offset: d.offset as u64,
            start: d.start as u64,
            select: d.select as u64,
            disconnect: d.disconnect as u64,
            len: d.len as u64,
            addrspace: unsafe { cstr_to_string(d.addrspace) },
        });
        ptrs.push(d.ptr as usize);
    }
    (descs, ptrs)
}

unsafe fn parse_v2_categories(arr: *const retro_core_option_v2_category) -> Vec<CoreOptionCategory> {
    let mut out = Vec::new();
    if arr.is_null() {
        return out;
    }
    let mut i = 0isize;
    loop {
        let entry = unsafe { &*arr.offset(i) };
        if entry.key.is_null() {
            break;
        }
        out.push(CoreOptionCategory {
            key: unsafe { cstr_to_string_default(entry.key) },
            desc: unsafe { cstr_to_string_default(entry.desc) },
            info: unsafe { cstr_to_string(entry.info) },
        });
        i += 1;
    }
    out
}

pub(crate) static STATE: Mutex<Option<State>> = Mutex::new(None);

/// True when the singleton holds a State. Used by LibretroCore to refuse
/// double-init.
pub(crate) fn is_loaded() -> bool {
    STATE.lock().map(|g| g.is_some()).unwrap_or(false)
}

pub(crate) fn install(state: State) {
    *STATE.lock().expect("state mutex poisoned") = Some(state);
}

pub(crate) fn uninstall() -> Option<State> {
    STATE.lock().expect("state mutex poisoned").take()
}

/// Run a closure with mutable access to the State. Returns None if the State
/// was never installed.
pub(crate) fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    let mut g = STATE.lock().expect("state mutex poisoned");
    g.as_mut().map(f)
}

/// Enumerate + log the host's Vulkan physical devices exactly once per
/// process. Called from the HW-render env handlers (SET_HW_RENDER /
/// negotiation interface) — both can fire and we only want one device
/// dump. Process-global (not per-load) is fine: it's a diagnostic, and the
/// device set doesn't change across core reloads. See `hw_vulkan`.
fn hw_probe_once() {
    static HW_PROBED: AtomicBool = AtomicBool::new(false);
    if !HW_PROBED.swap(true, AtomicOrdering::Relaxed) {
        crate::hw_vulkan::probe_physical_devices();
    }
}

/// Supported-device list the loaded core advertised for `port` via
/// `RETRO_ENVIRONMENT_SET_CONTROLLER_INFO`. Reads the singleton State
/// directly so Tauri commands (which don't hold a `LibretroCore` ref —
/// the core lives inside the emu thread) can answer
/// `get_controller_devices` without dispatching through `EmuCommand`.
///
/// Returns an empty Vec when:
/// - no core is currently loaded (singleton never installed)
/// - the loaded core never called `SET_CONTROLLER_INFO`
/// - the core advertised fewer ports than `port + 1`
/// - the core declared this port supports nothing
pub fn loaded_core_controller_devices(port: u32) -> Vec<ControllerDeviceDescriptor> {
    with_state(|s| {
        s.controller_devices
            .get(port as usize)
            .cloned()
            .unwrap_or_default()
    })
    .unwrap_or_default()
}

/// Per-(port, device, index, id) → description list the loaded core
/// advertised via `RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS`. Same
/// singleton-direct read as `loaded_core_controller_devices`; same
/// caller — Tauri commands answering the bindings UI's "what does
/// THIS button DO in THIS game" lookup.
///
/// Returns an empty Vec when no core is loaded or the loaded core
/// never published descriptors. Updated on every successful
/// `cb_environment` call to env 11 (which may fire during init,
/// during retro_load_game, or any time during a game's lifetime per
/// spec; latest publish wins).
pub fn loaded_core_input_descriptors() -> Vec<InputDescriptor> {
    with_state(|s| s.input_descriptors.clone()).unwrap_or_default()
}

/// Minimum audio latency (ms) the loaded core requested via
/// `RETRO_ENVIRONMENT_SET_MINIMUM_AUDIO_LATENCY` (env 63). Returns 0
/// when no core is loaded OR the core didn't call the env (spec: 0
/// means "use default frontend latency" — same effect either way).
/// Capped at 512 ms in the parser per the spec ceiling.
///
/// Caller (oa-shell post-load) passes this to
/// `oa_audio::AudioSink::ensure_min_latency_ms` so the ring buffer
/// grows to cover the requested headroom. No-op when the request is
/// smaller than the existing buffer (per spec: smaller-than-current
/// has no effect).
pub fn loaded_core_min_audio_latency_ms() -> u32 {
    with_state(|s| s.min_audio_latency_ms).unwrap_or(0)
}

// ---------- extern "C" callback trampolines ----------

pub(crate) unsafe extern "C" fn cb_video_refresh(
    data: *const c_void,
    width: u32,
    height: u32,
    pitch: usize,
) {
    // HW-rendered-frame sentinel. A Vulkan/GL core signals "the frame is
    // already on the GPU, not in this pointer" by passing
    // `data == RETRO_HW_FRAME_BUFFER_VALID` (the `(void*)-1` value). It is
    // NOT a CPU buffer — running `pixel::convert` on it would dereference
    // `0xFFFF…` and crash (a latent bug even before HW support: the
    // null-check below lets `-1` through). The M1 device-build commit reads
    // the core's `VkImage` back into `fb_rgba` here; in this probe phase we
    // have no GPU readback yet, so we simply drop the HW frame. Either way:
    // never treat the sentinel as a pixel pointer.
    if data as usize == RETRO_HW_FRAME_BUFFER_VALID {
        return;
    }
    if data.is_null() || width == 0 || height == 0 {
        return;
    }
    let _ = with_state(|s| {
        let pix_fmt = s.pix_fmt;
        // Cap to the preallocated buffer dimensions.
        const FB_MAX_W: u32 = 1024;
        const FB_MAX_H: u32 = 512;
        let w = width.min(FB_MAX_W);
        let h = height.min(FB_MAX_H);
        s.fb_width = w;
        s.fb_height = h;
        // SAFETY: caller (core) guarantees `data` points to at least
        // `pitch * h` valid bytes; we only read within those bounds.
        unsafe {
            pixel::convert(pix_fmt, data as *const u8, pitch, w, h, &mut s.fb_rgba);
        }
    });
}

pub(crate) unsafe extern "C" fn cb_audio_sample(left: i16, right: i16) {
    let _ = with_state(|s| {
        s.audio.push(left);
        s.audio.push(right);
    });
}

pub(crate) unsafe extern "C" fn cb_audio_sample_batch(data: *const i16, frames: usize) -> usize {
    if data.is_null() || frames == 0 {
        return 0;
    }
    let samples = frames * 2;
    let _ = with_state(|s| {
        // SAFETY: core guarantees `data` points to `frames * 2` interleaved i16s.
        let src = unsafe { std::slice::from_raw_parts(data, samples) };
        s.audio.extend_from_slice(src);
    });
    frames
}

pub(crate) unsafe extern "C" fn cb_input_poll() {
    // We push input via LibretroCore::set_input; no fetch needed here.
}

/// Phase F — `retro_rumble_interface::set_rumble_state` trampoline.
/// Cores call this whenever they want to drive controller vibration
/// (per (port, effect-kind, strength) tuple, libretro spec env 23).
/// We stash the strength into `State.rumble[port][effect]`; the shell
/// reads the snapshot once per frame and dispatches via gilrs.
pub(crate) unsafe extern "C" fn cb_set_rumble_state(
    port: u32,
    effect: u32,
    strength: u16,
) -> bool {
    if port >= 5 || effect >= 2 {
        return false;
    }
    with_state(|s| {
        s.rumble[port as usize][effect as usize] = strength;
    });
    true
}

/// Phase G — `retro_sensor_interface::set_sensor_state` trampoline.
/// Cores call this to enable/disable per-port accelerometer / gyro /
/// illuminance polling at a target sample `rate`. We honor the
/// enable/disable bit but ignore `rate` (the shell pumps sensor values
/// every frame regardless; cores get freshness without polling cadence
/// negotiation).
pub(crate) unsafe extern "C" fn cb_set_sensor_state(
    port: u32,
    action: u32,
    _rate: u32,
) -> bool {
    if port >= 5 {
        return false;
    }
    let (class_idx, enable) = match action {
        RETRO_SENSOR_ACCELEROMETER_ENABLE => (0usize, true),
        RETRO_SENSOR_ACCELEROMETER_DISABLE => (0usize, false),
        RETRO_SENSOR_GYROSCOPE_ENABLE => (1usize, true),
        RETRO_SENSOR_GYROSCOPE_DISABLE => (1usize, false),
        RETRO_SENSOR_ILLUMINANCE_ENABLE => (2usize, true),
        RETRO_SENSOR_ILLUMINANCE_DISABLE => (2usize, false),
        _ => return false,
    };
    with_state(|s| {
        s.sensor_enabled[port as usize][class_idx] = enable;
        // Zero the matching value slots on disable so a re-enable
        // doesn't read stale data from a previous session.
        if !enable {
            let axis_range: &[usize] = match class_idx {
                0 => &[RETRO_SENSOR_ACCELEROMETER_X as usize,
                       RETRO_SENSOR_ACCELEROMETER_Y as usize,
                       RETRO_SENSOR_ACCELEROMETER_Z as usize],
                1 => &[RETRO_SENSOR_GYROSCOPE_X as usize,
                       RETRO_SENSOR_GYROSCOPE_Y as usize,
                       RETRO_SENSOR_GYROSCOPE_Z as usize],
                2 => &[RETRO_SENSOR_ILLUMINANCE as usize],
                _ => &[],
            };
            for axis in axis_range {
                s.sensor_values[port as usize][*axis] = 0.0;
            }
        }
    });
    true
}

/// Phase G — `retro_sensor_interface::get_sensor_input` trampoline.
/// Cores call this every frame for each sensor axis they care about.
/// We return `State.sensor_values[port][id]` for the enabled classes;
/// disabled classes always read 0 (the disable side cleared the
/// matching slots).
pub(crate) unsafe extern "C" fn cb_get_sensor_input(port: u32, id: u32) -> f32 {
    if port >= 5 || id > RETRO_SENSOR_ILLUMINANCE {
        return 0.0;
    }
    with_state(|s| s.sensor_values[port as usize][id as usize])
        .unwrap_or(0.0)
}

/// Pure helper: resolve a `RETRO_DEVICE_ID_POINTER_*` query against
/// the per-port primary + secondary pointer tuples.
///
/// Multi-touch dispatch: cores poll
/// `cb_input_state(port, RETRO_DEVICE_POINTER, index, id)` where
/// `index` selects which finger (0 = primary, 1 = secondary,
/// ≥ 2 = unused/zero). `COUNT` is independent of `index` and
/// reports the total number of pressed pointers (0 / 1 / 2).
///
/// Extracted from cb_input_state so the dispatch table is
/// unit-testable without manipulating the libretro singleton state.
pub(crate) fn pointer_field_value(
    primary: (i16, i16, bool, bool),
    secondary: (i16, i16, bool, bool),
    index: u32,
    id: u32,
) -> i16 {
    // COUNT is touched-finger total — the same answer regardless of
    // which `index` the core queried. Compute it before the
    // index dispatch so `index = 1` `COUNT` returns 2-when-both-down
    // (rather than dropping to whatever the index-1 PRESSED happens
    // to be).
    if id == RETRO_DEVICE_ID_POINTER_COUNT {
        return primary.2 as i16 + secondary.2 as i16;
    }
    let (x, y, pressed, _in_viewport) = match index {
        0 => primary,
        1 => secondary,
        _ => return 0,
    };
    match id {
        RETRO_DEVICE_ID_POINTER_X       => x,
        RETRO_DEVICE_ID_POINTER_Y       => y,
        RETRO_DEVICE_ID_POINTER_PRESSED => if pressed { 1 } else { 0 },
        _ => 0,
    }
}

/// Walk the null-terminated `retro_controller_info` array a core
/// passes via `RETRO_ENVIRONMENT_SET_CONTROLLER_INFO` and clone every
/// `(label, id)` pair into owned Rust strings.
///
/// The outer array sentinel is the first entry whose `types == NULL`
/// AND `num_types == 0` — there's no explicit count in the spec. An
/// inner per-port `num_types == 0` with a non-null `types` pointer
/// means "port has no advertised devices" (rare but spec-permitted) —
/// distinct from the sentinel.
///
/// `desc` strings live in the core's `.dll` text segment per spec;
/// cloning to owned `String` here decouples the frontend hop from the
/// core's lifetime. Empty `String` substituted when `desc` is NULL.
///
/// Defensive cap at 32 ports — libretro doesn't define a max, but a
/// buggy core that forgets the sentinel could walk forever into
/// unmapped memory. 32 is well above any real per-system port count
/// (PCE multitap is 5; nothing ships more).
///
/// # Safety
///
/// `data` must either be NULL or point to a null-terminated array of
/// `retro_controller_info` as documented in libretro.h. The core owns
/// the memory; we read it during the env call only.
unsafe fn parse_controller_info(
    data: *const retro_controller_info,
) -> Vec<Vec<ControllerDeviceDescriptor>> {
    let mut out: Vec<Vec<ControllerDeviceDescriptor>> = Vec::new();
    if data.is_null() {
        return out;
    }
    let mut port = 0usize;
    const MAX_PORTS: usize = 32;
    loop {
        let entry = unsafe { &*data.add(port) };
        if entry.types.is_null() && entry.num_types == 0 {
            break;
        }
        let mut devices: Vec<ControllerDeviceDescriptor> =
            Vec::with_capacity(entry.num_types as usize);
        if !entry.types.is_null() {
            for i in 0..entry.num_types as usize {
                let desc_entry = unsafe { &*entry.types.add(i) };
                let label = if desc_entry.desc.is_null() {
                    String::new()
                } else {
                    unsafe { CStr::from_ptr(desc_entry.desc) }
                        .to_string_lossy()
                        .into_owned()
                };
                devices.push(ControllerDeviceDescriptor {
                    label,
                    id: desc_entry.id as u32,
                });
            }
        }
        out.push(devices);
        port += 1;
        if port > MAX_PORTS {
            log::warn!(
                "oa-libretro: SET_CONTROLLER_INFO exceeded {} ports without sentinel — truncating (likely a malformed core array)",
                MAX_PORTS,
            );
            break;
        }
    }
    out
}

/// Walk the null-terminated `retro_input_descriptor` array a core
/// passes via `RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS` and clone
/// every `(port, device, index, id, description)` tuple into owned
/// Rust values.
///
/// Sentinel: the first entry whose `description == NULL` (no count
/// field). Per-spec, the string lifetime is "until retro_unload_game()
/// is called" — we clone to owned `String` so the frontend hop is
/// decoupled from the core's load lifetime.
///
/// Defensive cap at 256 descriptors — libretro doesn't define a max,
/// but Street Fighter (one of the heaviest in-tree examples) declares
/// ~30. 256 is comfortable headroom against a buggy core that forgets
/// the sentinel.
///
/// # Safety
///
/// `data` must either be NULL or point to a null-terminated array of
/// `retro_input_descriptor` as documented in libretro.h. The core
/// owns the memory; we read it during the env call only.
unsafe fn parse_input_descriptors(
    data: *const retro_input_descriptor,
) -> Vec<InputDescriptor> {
    let mut out: Vec<InputDescriptor> = Vec::new();
    if data.is_null() {
        return out;
    }
    const MAX_DESCRIPTORS: usize = 256;
    let mut i = 0usize;
    loop {
        let entry = unsafe { &*data.add(i) };
        if entry.description.is_null() {
            break;
        }
        let description = unsafe { CStr::from_ptr(entry.description) }
            .to_string_lossy()
            .into_owned();
        out.push(InputDescriptor {
            port: entry.port as u32,
            device: entry.device as u32,
            index: entry.index as u32,
            id: entry.id as u32,
            description,
        });
        i += 1;
        if i > MAX_DESCRIPTORS {
            log::warn!(
                "oa-libretro: SET_INPUT_DESCRIPTORS exceeded {} entries without sentinel — truncating (likely a malformed core array)",
                MAX_DESCRIPTORS,
            );
            break;
        }
    }
    out
}

/// Parse + clamp the `RETRO_ENVIRONMENT_SET_MINIMUM_AUDIO_LATENCY`
/// payload to a milliseconds value. Returns 0 when `data` is NULL
/// (spec-permitted edge case — same effect as "use default" per the
/// spec docstring). Clamps to the spec's 512 ms ceiling so a buggy
/// core that publishes a pathological value can't balloon oa-audio's
/// ring buffer.
///
/// # Safety
///
/// `data` must either be NULL or point to a valid `unsigned` (== u32)
/// value owned by the core. The core retains ownership; we read and
/// drop the reference.
unsafe fn parse_min_audio_latency_ms(data: *const c_void) -> u32 {
    if data.is_null() {
        log::warn!(
            "oa-libretro: SET_MINIMUM_AUDIO_LATENCY called with NULL data — ignored"
        );
        return 0;
    }
    let raw_ms = unsafe { *(data as *const u32) };
    const SPEC_CEILING_MS: u32 = 512;
    let capped = raw_ms.min(SPEC_CEILING_MS);
    if raw_ms > SPEC_CEILING_MS {
        log::warn!(
            "oa-libretro: SET_MINIMUM_AUDIO_LATENCY requested {raw_ms} ms — capping at spec ceiling {SPEC_CEILING_MS} ms"
        );
    } else {
        log::info!("oa-libretro: SET_MINIMUM_AUDIO_LATENCY = {capped} ms");
    }
    capped
}

/// Render the parsed descriptor list as a single log line for the
/// operator-visible startup trace. Shows the first ~6 entries with a
/// "+N more" tail so a Street-Fighter-sized advertisement stays
/// readable; full set is in the State for the bindings UI to consume.
fn format_input_descriptors_for_log(descriptors: &[InputDescriptor]) -> String {
    if descriptors.is_empty() {
        return String::from("[]");
    }
    let preview = descriptors
        .iter()
        .take(6)
        .map(|d| {
            format!(
                "p{}/d{}/i{}/id{}={}",
                d.port, d.device, d.index, d.id, d.description
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let suffix = if descriptors.len() > 6 {
        format!(", +{} more", descriptors.len() - 6)
    } else {
        String::new()
    };
    format!("[{preview}{suffix}]")
}

/// Render the parsed per-port device list as a single log line for
/// the operator-visible startup trace. Truncates long lists so the
/// log stays readable when a core advertises dozens of devices
/// per port (FCEUmm's Famicom expansion port advertises ~12).
fn format_controller_devices_for_log(
    devices: &[Vec<ControllerDeviceDescriptor>],
) -> String {
    devices
        .iter()
        .enumerate()
        .map(|(port, devs)| {
            if devs.is_empty() {
                format!("port {port} = []")
            } else {
                let preview = devs
                    .iter()
                    .take(8)
                    .map(|d| format!("{}={}", d.label, d.id))
                    .collect::<Vec<_>>()
                    .join(", ");
                let suffix = if devs.len() > 8 {
                    format!(", +{} more", devs.len() - 8)
                } else {
                    String::new()
                };
                format!("port {port} = [{preview}{suffix}]")
            }
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// Pure helper: resolve a RETRO_DEVICE_ID_LIGHTGUN_* query against a
/// stored `(x, y, pressed, in_viewport)` tuple + the gun-side button
/// bitmask. Wires the screen-coordinate + trigger + deprecated
/// relative aliases + the IS_OFFSCREEN reload-by-aim flag + the
/// AUX/START/SELECT/DPAD/RELOAD bits sourced from
/// `InputState.lightgun_buttons`. See `RETRO_DEVICE_ID_LIGHTGUN_*`
/// in `ffi.rs` for the full id table.
///
/// `buttons` bit positions match the libretro LIGHTGUN id directly
/// (bit 3 = AUX_A, bit 16 = RELOAD, etc.).
pub(crate) fn lightgun_field_value(
    pointer: (i16, i16, bool, bool),
    buttons: u32,
    id: u32,
) -> i16 {
    let (x, y, pressed, in_viewport) = pointer;
    match id {
        // Absolute screen coordinates — the modern way (post-2013).
        // All currently-maintained light-gun cores poll these.
        RETRO_DEVICE_ID_LIGHTGUN_SCREEN_X      => x,
        RETRO_DEVICE_ID_LIGHTGUN_SCREEN_Y      => y,
        // Deprecated relative coordinates — legacy cores still query
        // these. Same source as absolute; the spec treats the
        // distinction as historical.
        RETRO_DEVICE_ID_LIGHTGUN_X             => x,
        RETRO_DEVICE_ID_LIGHTGUN_Y             => y,
        // Primary fire button — stays driven by mouse-button state
        // (the pointer.pressed flag) so left-click fires the trigger
        // even when an operator has no per-system gun-side bindings
        // configured. Bit 2 of `buttons` is reserved for parity with
        // the libretro id space; we deliberately don't OR it in here.
        RETRO_DEVICE_ID_LIGHTGUN_TRIGGER       => if pressed { 1 } else { 0 },
        // Reload-by-aim-off-screen gesture. `in_viewport` is sourced
        // from oa-input's poll_pointer — flips to false when the
        // cursor is outside the game-output rectangle (sidebar /
        // letterbox / chrome). Light-gun cores read this as "shoot
        // off-screen to reload" which is the canonical UX for House
        // of the Dead 2, Time Crisis series, Lethal Enforcers, etc.
        // Without a viewport rectangle set (Phase 0 fallback path
        // in poll_pointer) `in_viewport` is unconditionally true,
        // matching the pre-2026-05-27 always-on-screen behaviour.
        RETRO_DEVICE_ID_LIGHTGUN_IS_OFFSCREEN  => if in_viewport { 0 } else { 1 },
        // Gun-side buttons (AUX_A/B/C, START, SELECT, DPAD_*, RELOAD).
        // Bit position in `buttons` matches the libretro id. Operator
        // binds physical inputs → these bits via the existing
        // per-system RetroPad bindings (mirrored from JOYPAD bits by
        // oa-input::lightgun_buttons_from_joypad_bits).
        RETRO_DEVICE_ID_LIGHTGUN_AUX_A
        | RETRO_DEVICE_ID_LIGHTGUN_AUX_B
        | RETRO_DEVICE_ID_LIGHTGUN_AUX_C
        | RETRO_DEVICE_ID_LIGHTGUN_START
        | RETRO_DEVICE_ID_LIGHTGUN_SELECT
        | RETRO_DEVICE_ID_LIGHTGUN_DPAD_UP
        | RETRO_DEVICE_ID_LIGHTGUN_DPAD_DOWN
        | RETRO_DEVICE_ID_LIGHTGUN_DPAD_LEFT
        | RETRO_DEVICE_ID_LIGHTGUN_DPAD_RIGHT
        | RETRO_DEVICE_ID_LIGHTGUN_RELOAD      => ((buttons >> id) & 1) as i16,
        // Future libretro additions land here.
        _ => 0,
    }
}

pub(crate) unsafe extern "C" fn cb_input_state(
    port: u32,
    device: u32,
    index: u32,
    id: u32,
) -> i16 {
    /// Special id passed by cores that use the bitmask API (we ack'd
    /// `RETRO_ENVIRONMENT_GET_INPUT_BITMASKS`). The core asks for the full
    /// joypad state in one call instead of polling each button individually.
    const RETRO_DEVICE_ID_JOYPAD_MASK: u32 = 256;

    if port >= 5 {
        return 0;
    }

    // Analog stick / button queries — RETRO_DEVICE_ANALOG. Routed to the
    // per-port `input_axes` array. Layout: `[Left X, Left Y, Right X, Right Y]`.
    // Cores that emulate analog hardware (N64 stick, GameCube C-stick,
    // DualShock, Saturn 3D Pad, VB right D-pad, Intv 16-way disc) poll
    // this device with `index = LEFT/RIGHT` and `id = X/Y`. Values are
    // i16 in the libretro range -32768..32767.
    if device == RETRO_DEVICE_ANALOG {
        // RETRO_DEVICE_INDEX_ANALOG_BUTTON queries pressure for a
        // specific joypad button — DualShock face / trigger pressure,
        // GC L/R analog triggers, DC L/R triggers, PS2 pressure-
        // sensitive face buttons. Returns the per-port analog-button
        // value for the queried `id` (which matches the corresponding
        // RETRO_DEVICE_ID_JOYPAD_* bit position). 0 for unmapped slots.
        if index == RETRO_DEVICE_INDEX_ANALOG_BUTTON {
            if id > 15 {
                return 0;
            }
            return with_state(|s| s.input_analog_buttons[port as usize][id as usize]).unwrap_or(0);
        }
        let axis_idx = match (index, id) {
            (RETRO_DEVICE_INDEX_ANALOG_LEFT, RETRO_DEVICE_ID_ANALOG_X)  => 0,
            (RETRO_DEVICE_INDEX_ANALOG_LEFT, RETRO_DEVICE_ID_ANALOG_Y)  => 1,
            (RETRO_DEVICE_INDEX_ANALOG_RIGHT, RETRO_DEVICE_ID_ANALOG_X) => 2,
            (RETRO_DEVICE_INDEX_ANALOG_RIGHT, RETRO_DEVICE_ID_ANALOG_Y) => 3,
            _ => return 0,
        };
        return with_state(|s| s.input_axes[port as usize][axis_idx]).unwrap_or(0);
    }

    // Pointer device — touch screen / mouse-as-touch / stylus. Cores
    // polling this get the per-port (x, y, pressed) tuple back per
    // `index`: 0 → primary, 1 → secondary, ≥2 → zero. POINTER_COUNT
    // reports total pressed across both slots regardless of index.
    if device == RETRO_DEVICE_POINTER {
        return with_state(|s| pointer_field_value(
            s.input_pointer[port as usize],
            s.input_pointer_secondary[port as usize],
            index,
            id,
        )).unwrap_or(0);
    }

    // Light gun device — classical arcade-style point-and-shoot. Shares
    // the per-port input_pointer state with RETRO_DEVICE_POINTER but
    // exposes the libretro LIGHTGUN id space, which most light-gun
    // cores (FCEUmm Zapper, Beetle PSX GunCon, Flycast HotD, Beetle
    // Saturn Virtua Gun, Genesis Plus GX Light Phaser, snes9x Super
    // Scope) poll instead of POINTER. See lightgun_field_value for
    // the id → coord/state mapping.
    if device == RETRO_DEVICE_LIGHTGUN {
        // One-shot diagnostic: log the FIRST LIGHTGUN poll per process
        // so we can confirm the core actually queries this device. If
        // this never fires, the core isn't asking for LIGHTGUN — usually
        // a core-option mismatch (FCEUmm's `fceumm_zapper_mode` set to
        // mouse / touchscreen instead of clightgun).
        static LIGHTGUN_FIRST_POLL: AtomicBool = AtomicBool::new(false);
        if !LIGHTGUN_FIRST_POLL.swap(true, AtomicOrdering::Relaxed) {
            let snapshot = with_state(|s| (
                s.input_pointer[port as usize],
                s.input_lightgun_buttons[port as usize],
            ));
            log::info!(
                "oa-libretro: first LIGHTGUN poll port={port} id={id} pointer={:?} buttons={:08x}",
                snapshot.as_ref().map(|t| t.0),
                snapshot.as_ref().map(|t| t.1).unwrap_or(0),
            );
        }
        return with_state(|s| lightgun_field_value(
            s.input_pointer[port as usize],
            s.input_lightgun_buttons[port as usize],
            id,
        )).unwrap_or(0);
    }

    if device != RETRO_DEVICE_JOYPAD {
        return 0;
    }
    if id == RETRO_DEVICE_ID_JOYPAD_MASK {
        return with_state(|s| s.input_bits[port as usize] as i16).unwrap_or(0);
    }
    if id > 15 {
        return 0;
    }
    with_state(|s| ((s.input_bits[port as usize] >> id) & 1) as i16).unwrap_or(0)
}

pub(crate) unsafe extern "C" fn cb_environment(cmd: u32, data: *mut c_void) -> bool {
    // Temporary diagnostic while bringing up the CD path — every env cmd is
    // logged at info so a crash during init can be correlated to the last
    // command before death. Downgrade to debug once CD playback is verified.
    log::info!(
        "oa-libretro: env cmd {} (raw 0x{:x}), data={}",
        cmd & !RETRO_ENVIRONMENT_EXPERIMENTAL,
        cmd,
        if data.is_null() { "null" } else { "ptr" }
    );
    match cmd {
        RETRO_ENVIRONMENT_SET_PIXEL_FORMAT => {
            if data.is_null() {
                return false;
            }
            // SAFETY: core hands us a *retro_pixel_format which we read as u32.
            let v = unsafe { *(data as *const u32) };
            let Some(fmt) = PixelFormat::from_u32(v) else {
                log::warn!("oa-libretro: unsupported pixel format {v}");
                return false;
            };
            with_state(|s| s.pix_fmt = fmt);
            true
        }
        RETRO_ENVIRONMENT_GET_LOG_INTERFACE => {
            // Hand the core our C trampoline. Mednafen-derived cores log their
            // CD bringup, BIOS lookup, CHD parsing, etc. through this; without
            // it those messages fall through to fprintf(stderr) which is
            // invisible on a Windows GUI subsystem app.
            if data.is_null() {
                return false;
            }
            unsafe {
                let cb = data as *mut retro_log_callback;
                (*cb).log = oa_libretro_log_trampoline;
            }
            true
        }
        RETRO_ENVIRONMENT_GET_GAME_INFO_EXT => {
            // Return a fully-populated info_ext with NON-NULL string
            // pointers for every field. Two iterations got us here:
            //   v1 (NULL pointers for fields we didn't have): FCEUmm
            //       crashed dereferencing one of the NULL strings.
            //   v2 (return false / "unsupported"): FCEUmm declined the
            //       load instead of falling back to info.data + info.size
            //       from retro_load_game. Spec-noncompliant but matches
            //       the actual FCEUmm code path in mid-2025 builds.
            //   v3 (this): valid pointers everywhere. Fields we don't
            //       have real values for point at a stable empty C-
            //       string ("\0"). Cores doing unchecked strlen/strstr
            //       get harmless behavior; cores that DO null-check get
            //       a benign empty-string result. Works across both.
            if data.is_null() {
                return false;
            }
            with_state(|s| {
                if s.pending_rom_data.is_null() || s.pending_rom_size == 0 {
                    return false;
                }
                let empty = EMPTY_CSTR.as_ptr();
                s.pending_info_ext = retro_game_info_ext {
                    // full_path MUST end with the real extension —
                    // some cores (Genesis Plus GX) read the last 3
                    // bytes of this string to decide SMS vs GG vs MD.
                    // Set by load_rom to "<name>.<ext>" for Bytes
                    // source or the real path for Path source.
                    full_path: s.pending_full_path.as_ptr(),
                    archive_path: empty,
                    archive_file: empty,
                    dir: empty,
                    name: s.pending_name.as_ptr(),
                    ext: s.pending_ext.as_ptr(),
                    meta: empty,
                    data: s.pending_rom_data as *const c_void,
                    size: s.pending_rom_size,
                    file_in_archive: false,
                    persistent_data: true,
                };
                unsafe {
                    *(data as *mut *const retro_game_info_ext) = &s.pending_info_ext;
                }
                true
            })
            .unwrap_or(false)
        }
        // SET_INPUT_DESCRIPTORS (11) — per-(port, device, index, id)
        // "human label" tuples. Cores publish these per-game with the
        // in-game semantics ("B = Run" in Mario; "B = Whip" in
        // Castlevania) so the bindings UI can show what each physical
        // button DOES in the current game alongside its operator-
        // chosen physical mapping. Sibling of SET_CONTROLLER_INFO
        // (see parse_controller_info above for the same shape).
        // Plan: docs/PLANS/dynamic-input-descriptors.md.
        RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS => {
            let parsed = unsafe {
                parse_input_descriptors(data as *const retro_input_descriptor)
            };
            log::info!(
                "oa-libretro: SET_INPUT_DESCRIPTORS — {} descriptor(s) received: {}",
                parsed.len(),
                format_input_descriptors_for_log(&parsed),
            );
            with_state(|s| s.input_descriptors = parsed);
            true
        }
        RETRO_ENVIRONMENT_SET_ROTATION => {
            // Core reports its preferred display rotation. Units of 90°
            // clockwise; valid values 0..=3. Vertical arcade boards
            // (Pac-Man / Galaxian / DK) set 1 or 3 here. We store the
            // value and let the shell pull it via LibretroCore::rotation()
            // after retro_load_game returns.
            if data.is_null() {
                return false;
            }
            let rot = unsafe { *(data as *const u32) };
            if rot > 3 {
                log::warn!("oa-libretro: SET_ROTATION got out-of-range {} (expected 0..=3); ignoring", rot);
                return false;
            }
            with_state(|s| { s.rotation = rot; });
            true
        }
        RETRO_ENVIRONMENT_SET_GEOMETRY => {
            if data.is_null() {
                return false;
            }
            // SAFETY: core hands us a *const retro_game_geometry.
            let geom = unsafe { &*(data as *const retro_game_geometry) };
            with_state(|s| s.display_aspect = geom.aspect_ratio);
            true
        }
        RETRO_ENVIRONMENT_SET_SYSTEM_AV_INFO => {
            if data.is_null() {
                return false;
            }
            let av = unsafe { &*(data as *const retro_system_av_info) };
            with_state(|s| s.display_aspect = av.geometry.aspect_ratio);
            true
        }
        RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY => {
            if data.is_null() {
                return false;
            }
            with_state(|s| {
                unsafe {
                    *(data as *mut *const std::os::raw::c_char) = s.system_dir.as_ptr();
                }
                true
            })
            .unwrap_or(false)
        }
        RETRO_ENVIRONMENT_GET_SAVE_DIRECTORY => {
            if data.is_null() {
                return false;
            }
            with_state(|s| {
                unsafe {
                    *(data as *mut *const std::os::raw::c_char) = s.save_dir.as_ptr();
                }
                true
            })
            .unwrap_or(false)
        }
        RETRO_ENVIRONMENT_GET_VARIABLE => {
            // Look up the user-overridden value if any; otherwise fall back
            // to the default the core itself registered. Returning NULL would
            // make the core use its hardcoded defaults — that worked for
            // first-cut parity but defeats the per-system / per-game option
            // override flow now that the schema is captured + persisted.
            if data.is_null() {
                return false;
            }
            let var = unsafe { &mut *(data as *mut retro_variable) };
            let key = unsafe { cstr_to_string(var.key) }.unwrap_or_default();
            with_state(|s| {
                if let Some(cstr) = s.option_values.get(&key) {
                    var.value = cstr.as_ptr();
                    return true;
                }
                // No override — fall back to the core's declared default
                // (synthesized as a CString stored on the State so the
                // pointer outlives the env call).
                if let Some(opt) = s.core_options.iter().find(|o| o.key == key) {
                    let default = opt.default_value.clone();
                    let cstr = s
                        .option_values
                        .entry(key.clone())
                        .or_insert_with(|| CString::new(default).unwrap_or_default());
                    var.value = cstr.as_ptr();
                    return true;
                }
                var.value = std::ptr::null();
                false
            })
            .unwrap_or(false)
        }
        RETRO_ENVIRONMENT_GET_VARIABLE_UPDATE => {
            // Return + clear the dirty flag. Initial value is `true` so the
            // core's first poll says "yes, fetch your variables."
            if data.is_null() { return false; }
            with_state(|s| {
                let updated = s.variables_updated;
                s.variables_updated = false;
                unsafe { *(data as *mut bool) = updated; }
                true
            })
            .unwrap_or(false)
        }
        RETRO_ENVIRONMENT_SET_VARIABLES => {
            if data.is_null() { return true; }
            let parsed = unsafe { parse_legacy_variables(data as *const retro_variable) };
            with_state(|s| {
                s.core_options = parsed;
                s.option_categories.clear();
                s.hidden_options.clear();
                s.variables_updated = true;
            });
            true
        }
        RETRO_ENVIRONMENT_SET_CORE_OPTIONS => {
            if data.is_null() { return true; }
            let parsed = unsafe {
                parse_core_options_v1(data as *const retro_core_option_definition)
            };
            with_state(|s| {
                s.core_options = parsed;
                s.option_categories.clear();
                s.hidden_options.clear();
                s.variables_updated = true;
            });
            true
        }
        RETRO_ENVIRONMENT_SET_CORE_OPTIONS_INTL => {
            // INTL struct has `us` (English fallback) + `local` (current
            // locale, may be NULL). Prefer `local` when present; we don't
            // do locale resolution today.
            if data.is_null() { return true; }
            let intl = unsafe { &*(data as *const retro_core_options_intl) };
            let src = if !intl.local.is_null() { intl.local } else { intl.us };
            let parsed = unsafe { parse_core_options_v1(src) };
            with_state(|s| {
                s.core_options = parsed;
                s.option_categories.clear();
                s.hidden_options.clear();
                s.variables_updated = true;
            });
            true
        }
        RETRO_ENVIRONMENT_SET_CORE_OPTIONS_V2 => {
            if data.is_null() { return true; }
            let v2 = unsafe { &*(data as *const retro_core_options_v2) };
            let categories = unsafe { parse_v2_categories(v2.categories) };
            let definitions = unsafe { parse_core_options_v2(v2.definitions) };
            with_state(|s| {
                s.core_options = definitions;
                s.option_categories = categories;
                s.hidden_options.clear();
                s.variables_updated = true;
            });
            true
        }
        RETRO_ENVIRONMENT_SET_CORE_OPTIONS_V2_INTL => {
            if data.is_null() { return true; }
            let intl = unsafe { &*(data as *const retro_core_options_v2_intl) };
            let src = if !intl.local.is_null() { intl.local } else { intl.us };
            if src.is_null() { return true; }
            let v2 = unsafe { &*src };
            let categories = unsafe { parse_v2_categories(v2.categories) };
            let definitions = unsafe { parse_core_options_v2(v2.definitions) };
            with_state(|s| {
                s.core_options = definitions;
                s.option_categories = categories;
                s.hidden_options.clear();
                s.variables_updated = true;
            });
            true
        }
        // SET_CORE_OPTIONS_DISPLAY toggles individual option visibility.
        // The core calls this once per (key, visible) pair — either during
        // init when it first computes which options apply, or re-entrantly
        // from inside the update-display callback after a value change. We
        // mirror the visible bit into State.hidden_options so the per-system
        // / per-game settings panel can filter accordingly. Unknown keys
        // are silently inserted into the hidden set so a future schema-add
        // doesn't briefly expose a flag the core already wanted hidden.
        RETRO_ENVIRONMENT_SET_CORE_OPTIONS_DISPLAY => {
            if data.is_null() { return true; }
            let disp = unsafe { &*(data as *const retro_core_option_display) };
            let Some(key) = (unsafe { cstr_to_string(disp.key) }) else {
                return true;
            };
            with_state(|s| {
                if disp.visible {
                    s.hidden_options.remove(&key);
                } else {
                    s.hidden_options.insert(key);
                }
            });
            true
        }
        // SET_CORE_OPTIONS_UPDATE_DISPLAY_CALLBACK lets the core register a
        // "I want to re-evaluate visibility now" hook. The frontend calls
        // it after writing a new option value; the core's implementation
        // synchronously fires SET_CORE_OPTIONS_DISPLAY for each option
        // whose visibility changed, then returns. Cores without dynamic
        // visibility never call this env; their initial SET_CORE_OPTIONS_
        // DISPLAY pass remains in effect for the session.
        RETRO_ENVIRONMENT_SET_CORE_OPTIONS_UPDATE_DISPLAY_CALLBACK => {
            if data.is_null() {
                // Spec: passing NULL un-registers any prior callback.
                with_state(|s| s.update_display_cb = None);
                return true;
            }
            let reg = unsafe {
                &*(data as *const retro_core_options_update_display_callback)
            };
            with_state(|s| s.update_display_cb = reg.callback);
            true
        }

        // Frontend "I support core option API version N". Report v2 so modern
        // cores use SET_CORE_OPTIONS_V2 instead of the legacy path.
        RETRO_ENVIRONMENT_GET_CORE_OPTIONS_VERSION => {
            if data.is_null() { return false; }
            unsafe { *(data as *mut u32) = 2; }
            true
        }

        // SET_CONTROLLER_INFO (35) — per-port supported-device list.
        // Parsed + stored so the frontend can render a dropdown of the
        // exact (label, id) entries the core declared, instead of a
        // hardcoded "Light Gun = 4 / Mouse = 2" list that's wrong for
        // every core using `RETRO_DEVICE_SUBCLASS`-extended peripherals
        // (FCEUmm Zapper = 258, snes9x Super Scope = 260, Genesis Plus
        // GX Light Phaser = 260, Beetle PSX GunCon = 260, …). See
        // docs/PLANS/dynamic-controller-info.md.
        RETRO_ENVIRONMENT_SET_CONTROLLER_INFO => {
            let parsed = unsafe {
                parse_controller_info(data as *const retro_controller_info)
            };
            log::info!(
                "oa-libretro: SET_CONTROLLER_INFO — {} port(s) advertised: {}",
                parsed.len(),
                format_controller_devices_for_log(&parsed),
            );
            with_state(|s| s.controller_devices = parsed);
            true
        }

        // Setters where accepting is harmless — we don't store the declared
        // data, but acknowledging it keeps cores from bailing on init when
        // their setup path checks for frontend acks.
        RETRO_ENVIRONMENT_SET_PERFORMANCE_LEVEL
        | RETRO_ENVIRONMENT_SET_SUBSYSTEM_INFO
        | RETRO_ENVIRONMENT_SET_SERIALIZATION_QUIRKS
        | RETRO_ENVIRONMENT_SET_FASTFORWARDING_OVERRIDE => true,

        // SET_MINIMUM_AUDIO_LATENCY (63) — core requests minimum audio
        // buffer size in milliseconds (`const unsigned *` payload).
        // Cores set this when they know they'll have CPU-heavy frames
        // that briefly stall audio production. The shell consults the
        // stored value on every core load via
        // `loaded_core_min_audio_latency_ms()` and grows oa-audio's
        // ring buffer to at least that many ms of headroom — prevents
        // crackling on heavy frames in cores like Genesis Plus GX,
        // Beetle PSX, and Flycast that declare 64-100 ms requests.
        // Per spec (libretro.h:1369): 0 = "use default" (no override);
        // capped at the spec's 512 ms ceiling against buggy cores.
        RETRO_ENVIRONMENT_SET_MINIMUM_AUDIO_LATENCY => {
            let capped = unsafe { parse_min_audio_latency_ms(data) };
            with_state(|s| s.min_audio_latency_ms = capped);
            true
        }

        // SET_SUPPORT_NO_GAME (env 18) — core declares it can boot
        // without content. DOSBox-Pure (built-in DOS-game browser),
        // ScummVM (engine launcher with game list), 2048-libretro and
        // similar standalone-as-a-core titles all set this true. The
        // shell consults `LibretroCore::supports_no_game()` to decide
        // whether `load_no_rom()` is a valid action.
        RETRO_ENVIRONMENT_SET_SUPPORT_NO_GAME => {
            if data.is_null() {
                return false;
            }
            let flag = unsafe { *(data as *const bool) };
            log::info!("oa-libretro: SET_SUPPORT_NO_GAME = {flag}");
            with_state(|s| s.supports_no_game = flag);
            true
        }

        // SET_MESSAGE (env 6) — legacy OSD text + duration-in-frames.
        // Most pre-2020 cores still use this form. We parse it as an
        // Info-level message and ignore the frames field; the toast
        // stack has its own dismiss schedule. Log the text at info
        // level too so it's visible in the log file even if the user
        // missed the toast.
        RETRO_ENVIRONMENT_SET_MESSAGE => {
            if data.is_null() {
                return false;
            }
            let msg = unsafe { &*(data as *const retro_message) };
            let Some(text) = (unsafe { cstr_to_string(msg.msg) }) else {
                return false;
            };
            if text.is_empty() {
                return false;
            }
            log::info!("core OSD: {text}");
            with_state(|s| {
                s.pending_messages.push(oa_core::CoreMessage {
                    text,
                    level: oa_core::CoreMessageLevel::Info,
                    log_only: false,
                });
            });
            true
        }

        // GET_MESSAGE_INTERFACE_VERSION (env 59) — cores ask which OSD
        // API version we support before choosing between SET_MESSAGE
        // (v0) and SET_MESSAGE_EXT (v1). Report v1 so modern cores
        // route through the richer path.
        RETRO_ENVIRONMENT_GET_MESSAGE_INTERFACE_VERSION => {
            if data.is_null() {
                return false;
            }
            unsafe {
                *(data as *mut u32) = RETRO_MESSAGE_INTERFACE_VERSION;
            }
            true
        }

        // SET_MESSAGE_EXT (env 60) — modern OSD with duration in ms,
        // priority, log level, target (OSD vs LOG vs ALL), and message
        // type. We honor: text + log-level → toast tint, target = LOG
        // → log-only (no toast), target = OSD/ALL → toast. We ignore
        // duration (toast stack has its own schedule), priority (we
        // don't preempt — first-in shows), and type (NOTIFICATION vs
        // STATUS vs PROGRESS — all surface as the same toast shape;
        // future polish could route PROGRESS to a progress-bar widget).
        RETRO_ENVIRONMENT_SET_MESSAGE_EXT => {
            if data.is_null() {
                return false;
            }
            let m = unsafe { &*(data as *const retro_message_ext) };
            let Some(text) = (unsafe { cstr_to_string(m.msg) }) else {
                return false;
            };
            if text.is_empty() {
                return false;
            }
            // Mirror to log at the requested level so the message is
            // visible in the log file alongside the toast.
            match m.level {
                0 => log::debug!("core OSD: {text}"),
                1 => log::info!("core OSD: {text}"),
                2 => log::warn!("core OSD: {text}"),
                _ => log::error!("core OSD: {text}"),
            }
            // LOG-target messages skip the toast surface — they're
            // intended only for the log file (cores sometimes use this
            // for verbose status that would spam the OSD).
            let log_only = m.target == RETRO_MESSAGE_TARGET_LOG;
            let level = match m.level {
                2 => oa_core::CoreMessageLevel::Warn,
                3 => oa_core::CoreMessageLevel::Error,
                _ => oa_core::CoreMessageLevel::Info, // Debug + Info both surface as Info
            };
            with_state(|s| {
                s.pending_messages.push(oa_core::CoreMessage { text, level, log_only });
            });
            true
        }

        // SET_MEMORY_MAPS — parse + store the descriptor array. Cores
        // publish their guest-address-space layout here so future
        // RetroAchievements / cheat-search consumers can translate
        // guest addresses → host bytes. We store only the metadata in
        // oa-core types + the host base pointers separately as `usize`
        // (kept inside oa-libretro). Most pre-NES systems never call
        // this; most modern cores do. Returning true regardless keeps
        // cores happy whether they call it or not.
        RETRO_ENVIRONMENT_SET_MEMORY_MAPS => {
            if data.is_null() {
                return true;
            }
            let map = unsafe { &*(data as *const retro_memory_map) };
            let (descs, ptrs) =
                unsafe { parse_memory_descriptors(map.descriptors, map.num_descriptors) };
            log::info!(
                "oa-libretro: SET_MEMORY_MAPS captured {} descriptor(s)",
                descs.len()
            );
            with_state(|s| {
                s.memory_descriptors = descs;
                s.memory_map_ptrs = ptrs;
            });
            true
        }

        // DECLINE `SET_CONTENT_INFO_OVERRIDE`. We don't actually store the
        // declared override array — accepting it would be lying about
        // frontend behavior. Note: declining this does NOT prevent the
        // core from calling `GET_GAME_INFO_EXT` (FCEUmm probes that
        // unconditionally — verified 2026-05-18). The actual fix for the
        // FCEUmm NES crash lives in the `GET_GAME_INFO_EXT` arm: we
        // decline it too. Both decisions are spec-correct and shouldn't
        // affect any core that uses the standard `retro_load_game(info)`
        // path (PCE Fast, Mednafen Lynx, FCEUmm post-fallback, Snes9x).
        RETRO_ENVIRONMENT_SET_CONTENT_INFO_OVERRIDE => false,

        // Audio-buffer-status callback registration. We accept but never call
        // the callback back — the core will treat audio as always-ready, which
        // matches our simple ringbuf consumer. Without acceptance some cores
        // store the rejection state and the CD audio path can NPE.
        RETRO_ENVIRONMENT_SET_AUDIO_BUFFER_STATUS_CALLBACK => true,

        // We support input bitmasks (id == RETRO_DEVICE_ID_JOYPAD_MASK in
        // cb_input_state returns the full button bitmap for the port).
        RETRO_ENVIRONMENT_GET_INPUT_BITMASKS => true,

        // Frontend supports N concurrent users. We wire 5 input ports.
        RETRO_ENVIRONMENT_GET_INPUT_MAX_USERS => {
            if data.is_null() { return false; }
            unsafe { *(data as *mut u32) = 5; }
            true
        }

        // Disc control callbacks — register the function-pointer struct
        // so LibretroCore::disc_* can call through. Cores typically only
        // register one of v1 or v2.
        RETRO_ENVIRONMENT_SET_DISK_CONTROL_INTERFACE => {
            if data.is_null() { return true; }
            let cb = unsafe { *(data as *const retro_disk_control_callback) };
            with_state(|s| s.disk_v1 = Some(cb));
            true
        }
        RETRO_ENVIRONMENT_SET_DISK_CONTROL_EXT_INTERFACE => {
            if data.is_null() { return true; }
            let cb = unsafe { *(data as *const retro_disk_control_ext_callback) };
            with_state(|s| s.disk_v2 = Some(cb));
            true
        }
        RETRO_ENVIRONMENT_GET_DISK_CONTROL_INTERFACE_VERSION => {
            // Report v1 — that's enough to cover the get/set_eject_state +
            // set_image_index + get_num_images surface we expose. We DO
            // accept v2 (handled above) and read its label getter when
            // available; reporting v2 here would have cores assume we
            // call the ext-only callbacks too, which we don't yet.
            if data.is_null() { return false; }
            unsafe { *(data as *mut u32) = 1; }
            true
        }

        // Both A and V are enabled. Bit 0 = enable video, bit 1 = enable audio.
        // (Bit 2 = fast-forward, bit 3 = hardcore mode — we leave these off.)
        RETRO_ENVIRONMENT_GET_AUDIO_VIDEO_ENABLE => {
            if data.is_null() { return false; }
            unsafe { *(data as *mut i32) = 0b11; }
            true
        }

        // Not fast-forwarding.
        RETRO_ENVIRONMENT_GET_FASTFORWARDING => {
            if data.is_null() { return false; }
            unsafe { *(data as *mut bool) = false; }
            true
        }

        // Phase F (2026-05-21) — hand the core a rumble interface
        // pointing at our cb_set_rumble_state trampoline. The trampoline
        // writes per-port-per-effect strength into State.rumble; the
        // shell polls `LibretroCore::rumble_snapshot()` each frame and
        // dispatches via gilrs's force-feedback API.
        RETRO_ENVIRONMENT_GET_RUMBLE_INTERFACE => {
            if data.is_null() { return false; }
            let iface = retro_rumble_interface {
                set_rumble_state: cb_set_rumble_state,
            };
            unsafe { *(data as *mut retro_rumble_interface) = iface; }
            true
        }
        // Phase G (2026-05-21) — hand the core a sensor interface so
        // GBA tilt / solar / NDS gyroscope games can read sensor input.
        // Today the shell pumps keyboard arrow-keys-as-tilt into
        // State.sensor_values so games are playable without OS-level
        // accelerometer access; real motion is a separate later phase.
        RETRO_ENVIRONMENT_GET_SENSOR_INTERFACE => {
            if data.is_null() { return false; }
            let iface = retro_sensor_interface {
                set_sensor_state: cb_set_sensor_state,
                get_sensor_input: cb_get_sensor_input,
            };
            unsafe { *(data as *mut retro_sensor_interface) = iface; }
            true
        }
        // Decline these — we don't have camera / location interfaces, and
        // lying about them risks the core calling a null function pointer.
        RETRO_ENVIRONMENT_GET_CAMERA_INTERFACE
        | RETRO_ENVIRONMENT_GET_LOCATION_INTERFACE => false,

        // ---- HW render (M1 probe phase) ----
        //
        // docs/PLANS/hw-render-pipeline.md + DECISIONS D6. These three
        // env commands form the libretro HW-render handshake. In this
        // PROBE commit we capture + log what the core requests but do NOT
        // yet stand up the standalone Vulkan device — so `SET_HW_RENDER`
        // still returns false (today's behavior; the core falls back / the
        // crash is unchanged). The point is to read the log afterward and
        // see exactly which negotiation version + apiVersion + GPU the
        // operator's Dolphin .dll wants, then write the device code to
        // match. The device-build commit flips `SET_HW_RENDER` to accept.
        RETRO_ENVIRONMENT_GET_PREFERRED_HW_RENDER => {
            // Answer Vulkan so the core selects its Vulkan path and hands
            // us the Vulkan negotiation interface (env 43). Without this,
            // Dolphin defaults to OpenGL (observed in the 2026-06-07 log)
            // and we never see the Vulkan negotiation struct we need to
            // probe. Harmless as a hint even while we still decline
            // SET_HW_RENDER below.
            if data.is_null() {
                return false;
            }
            unsafe {
                *(data as *mut u32) = RETRO_HW_CONTEXT_VULKAN;
            }
            log::info!("oa-libretro HW: GET_PREFERRED_HW_RENDER -> Vulkan");
            true
        }
        RETRO_ENVIRONMENT_SET_HW_RENDER => {
            if data.is_null() {
                return false;
            }
            // SAFETY: core hands us a *retro_hw_render_callback it filled in.
            let cb = unsafe { *(data as *const retro_hw_render_callback) };
            log::info!(
                "oa-libretro HW: SET_HW_RENDER — context_type={} version={}.{} depth={} \
                 stencil={} bottom_left_origin={} cache_context={} debug_context={}",
                cb.context_type,
                cb.version_major,
                cb.version_minor,
                cb.depth,
                cb.stencil,
                cb.bottom_left_origin,
                cb.cache_context,
                cb.debug_context,
            );
            with_state(|s| s.hw_render_callback = Some(cb));
            if cb.context_type == RETRO_HW_CONTEXT_VULKAN {
                hw_probe_once();
            } else {
                log::warn!(
                    "oa-libretro HW: core requested non-Vulkan context_type {} — M1 targets Vulkan only",
                    cb.context_type,
                );
            }
            // PROBE phase: decline. The device-build commit returns true
            // here after standing up the standalone device + filling
            // cb.get_proc_address / cb.get_current_framebuffer.
            false
        }
        RETRO_ENVIRONMENT_SET_HW_RENDER_CONTEXT_NEGOTIATION_INTERFACE => {
            if data.is_null() {
                return false;
            }
            // The interface starts with `{ interface_type, interface_version }`;
            // confirm it's the Vulkan variant (type 0) before reinterpreting
            // the rest of the struct.
            let base_type = unsafe { *(data as *const u32) };
            if base_type != RETRO_HW_RENDER_CONTEXT_NEGOTIATION_INTERFACE_VULKAN {
                log::warn!(
                    "oa-libretro HW: negotiation interface type {base_type} is not Vulkan (0) — ignoring"
                );
                return false;
            }
            // SAFETY: type-checked above; the core owns the struct for the
            // duration of the call. We copy it out (it's Copy — fn pointers
            // + scalars only).
            let iface =
                unsafe { *(data as *const retro_hw_render_context_negotiation_interface_vulkan) };
            crate::hw_vulkan::log_negotiation_interface(&iface);
            with_state(|s| s.hw_negotiation_vulkan = Some(iface));
            hw_probe_once();
            true
        }

        // Keyboard callback registration. Cores that want raw key events
        // (MAME for operator-test / TAB menu, MSX for BASIC, every future
        // computer-shaped core) call this with a `retro_keyboard_callback`
        // pointing at the function we should invoke on every key transition.
        // The shell side decides per-system whether to actually pump events
        // (`SystemSettings::keyboard_passthrough` flag) and whether to wire
        // the port as `RETRO_DEVICE_KEYBOARD`. Storing the pointer is cheap
        // and harmless even for systems that never enable passthrough.
        RETRO_ENVIRONMENT_SET_KEYBOARD_CALLBACK => {
            if data.is_null() { return false; }
            let cb = unsafe { &*(data as *const retro_keyboard_callback) };
            with_state(|s| s.keyboard_cb = cb.callback);
            true
        }
        // GET_VARIABLE_UPDATE — handled above in the core-options block
        // alongside GET_VARIABLE / SET_VARIABLES so the state mutation
        // lives next to the rest of the option machinery.
        RETRO_ENVIRONMENT_GET_CAN_DUPE => {
            if data.is_null() {
                return false;
            }
            unsafe {
                *(data as *mut bool) = true;
            }
            true
        }
        RETRO_ENVIRONMENT_GET_LANGUAGE => {
            if data.is_null() {
                return false;
            }
            unsafe {
                *(data as *mut u32) = RETRO_LANGUAGE_ENGLISH;
            }
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- parse_controller_info ----------------------------------------

    /// Helper: build a leak-resilient table of `retro_controller_description`
    /// from a list of (label, id) pairs. Holds the `CString`s alongside
    /// the descriptor array so the C strings outlive the test's call to
    /// the parser. Returns (cstrings, descriptors) — keep both bound to
    /// stack locals until after the parse call.
    fn build_descs(
        entries: &[(&str, u32)],
    ) -> (Vec<CString>, Vec<retro_controller_description>) {
        let cstrings: Vec<CString> = entries
            .iter()
            .map(|(label, _)| CString::new(*label).unwrap())
            .collect();
        let descs: Vec<retro_controller_description> = entries
            .iter()
            .zip(cstrings.iter())
            .map(|((_, id), cs)| retro_controller_description {
                desc: cs.as_ptr(),
                id: *id as std::os::raw::c_uint,
            })
            .collect();
        (cstrings, descs)
    }

    #[test]
    fn parse_controller_info_handles_null_top_pointer() {
        let parsed = unsafe { parse_controller_info(std::ptr::null()) };
        assert!(parsed.is_empty());
    }

    #[test]
    fn parse_controller_info_two_ports_with_sentinel() {
        // Reproduces a realistic FCEUmm-shaped advertisement: port 0
        // and port 1 each advertise [Joypad, Zapper], sentinel at
        // port 2. Asserts both the (label, id) preservation AND that
        // walking stops at the sentinel rather than reading past it.
        let (cs0, d0) = build_descs(&[("Joypad", 1), ("Zapper", 258)]);
        let (cs1, d1) = build_descs(&[("Joypad", 1), ("Zapper", 258)]);
        let _keep = (cs0, cs1); // keep CStrings alive past the parse

        let info = [
            retro_controller_info {
                types: d0.as_ptr(),
                num_types: d0.len() as std::os::raw::c_uint,
            },
            retro_controller_info {
                types: d1.as_ptr(),
                num_types: d1.len() as std::os::raw::c_uint,
            },
            retro_controller_info {
                types: std::ptr::null(),
                num_types: 0,
            },
        ];

        let parsed = unsafe { parse_controller_info(info.as_ptr()) };
        assert_eq!(parsed.len(), 2, "should stop at sentinel, not over-read");
        assert_eq!(parsed[0].len(), 2);
        assert_eq!(parsed[0][0].label, "Joypad");
        assert_eq!(parsed[0][0].id, 1);
        assert_eq!(parsed[0][1].label, "Zapper");
        assert_eq!(parsed[0][1].id, 258);
        assert_eq!(parsed[1][1].label, "Zapper");
        assert_eq!(parsed[1][1].id, 258);
    }

    #[test]
    fn parse_controller_info_clones_strings_owned_not_aliased() {
        // After parse, the source CStrings get dropped — the parsed
        // strings must still be valid. (Catches a future regression
        // where someone refactors the parser to store borrowed `&str`
        // or `&CStr` into the State.)
        let parsed_then_dropped = {
            let (cs, descs) = build_descs(&[("Zapper", 258)]);
            let info = [
                retro_controller_info {
                    types: descs.as_ptr(),
                    num_types: 1,
                },
                retro_controller_info {
                    types: std::ptr::null(),
                    num_types: 0,
                },
            ];
            let parsed = unsafe { parse_controller_info(info.as_ptr()) };
            drop(cs);
            drop(descs);
            parsed
        };
        assert_eq!(parsed_then_dropped[0][0].label, "Zapper");
        assert_eq!(parsed_then_dropped[0][0].id, 258);
    }

    #[test]
    fn parse_controller_info_null_desc_becomes_empty_label() {
        // Spec allows NULL desc; substitute empty label rather than
        // panicking. Operator-visible: the dropdown row would render
        // its id as the label fallback (handled at the frontend).
        let bogus = retro_controller_description {
            desc: std::ptr::null(),
            id: 99,
        };
        let descs = [bogus];
        let info = [
            retro_controller_info {
                types: descs.as_ptr(),
                num_types: 1,
            },
            retro_controller_info {
                types: std::ptr::null(),
                num_types: 0,
            },
        ];
        let parsed = unsafe { parse_controller_info(info.as_ptr()) };
        assert_eq!(parsed[0][0].label, "");
        assert_eq!(parsed[0][0].id, 99);
    }

    #[test]
    fn parse_controller_info_empty_per_port_is_kept_not_sentinel() {
        // A port with num_types == 0 but a non-NULL types pointer is
        // NOT the sentinel — it's a port that declares zero devices.
        // (Rare but spec-permitted. Sentinel requires BOTH null + zero.)
        // Use a zero-length array's as_ptr() — guaranteed non-null in
        // Rust for empty slices.
        let empty: [retro_controller_description; 0] = [];
        let (_cs1, d1) = build_descs(&[("Joypad", 1)]);
        let info = [
            retro_controller_info {
                types: empty.as_ptr(),
                num_types: 0,
            },
            retro_controller_info {
                types: d1.as_ptr(),
                num_types: 1,
            },
            retro_controller_info {
                types: std::ptr::null(),
                num_types: 0,
            },
        ];
        let parsed = unsafe { parse_controller_info(info.as_ptr()) };
        assert_eq!(parsed.len(), 2, "empty-but-present port should not terminate the walk");
        assert!(parsed[0].is_empty());
        assert_eq!(parsed[1].len(), 1);
        assert_eq!(parsed[1][0].label, "Joypad");
    }

    #[test]
    fn parse_controller_info_subclass_ids_preserved_verbatim() {
        // The whole point of this parser: subclass-encoded ids
        // round-trip without rewriting. Exercises every base device id
        // that appears in any of the cores we ship today.
        // - FCEUmm Zapper       = SUBCLASS(MOUSE,    0) = 258
        // - FCEUmm Arkanoid     = SUBCLASS(MOUSE,    1) = 514
        // - FCEUmm Power Pad A  = SUBCLASS(KEYBOARD, 0) = 259
        // - snes9x Super Scope  = (1 << 8) | LIGHTGUN   = 260
        // - snes9x Super Multi  = (1 << 8) | JOYPAD     = 257
        // - Dolphin Wii+Nunchuk = (2 << 8) | JOYPAD     = 769
        let (_cs, descs) = build_descs(&[
            ("Joypad",          1),
            ("Super Multitap",  257),
            ("Zapper",          258),
            ("Power Pad A",     259),
            ("Super Scope",     260),
            ("Arkanoid",        514),
            ("Wii + Nunchuk",   769),
        ]);
        let info = [
            retro_controller_info {
                types: descs.as_ptr(),
                num_types: descs.len() as std::os::raw::c_uint,
            },
            retro_controller_info {
                types: std::ptr::null(),
                num_types: 0,
            },
        ];
        let parsed = unsafe { parse_controller_info(info.as_ptr()) };
        let ids: Vec<u32> = parsed[0].iter().map(|d| d.id).collect();
        assert_eq!(ids, vec![1, 257, 258, 259, 260, 514, 769]);
    }

    // ---- parse_input_descriptors -------------------------------------

    /// Helper: build a leak-resilient table of `retro_input_descriptor`
    /// from `(port, device, index, id, description)` tuples. Holds the
    /// CStrings alongside the array so the C strings outlive the test's
    /// parser call.
    fn build_input_descs(
        entries: &[(u32, u32, u32, u32, &str)],
    ) -> (Vec<CString>, Vec<retro_input_descriptor>) {
        let cstrings: Vec<CString> = entries
            .iter()
            .map(|(_, _, _, _, desc)| CString::new(*desc).unwrap())
            .collect();
        let descs: Vec<retro_input_descriptor> = entries
            .iter()
            .zip(cstrings.iter())
            .map(|((port, device, index, id, _), cs)| retro_input_descriptor {
                port: *port as std::os::raw::c_uint,
                device: *device as std::os::raw::c_uint,
                index: *index as std::os::raw::c_uint,
                id: *id as std::os::raw::c_uint,
                description: cs.as_ptr(),
            })
            .collect();
        (cstrings, descs)
    }

    /// Sentinel entry — `description = NULL`, everything else 0.
    fn input_desc_sentinel() -> retro_input_descriptor {
        retro_input_descriptor {
            port: 0,
            device: 0,
            index: 0,
            id: 0,
            description: std::ptr::null(),
        }
    }

    #[test]
    fn parse_input_descriptors_handles_null_top_pointer() {
        let parsed = unsafe { parse_input_descriptors(std::ptr::null()) };
        assert!(parsed.is_empty());
    }

    #[test]
    fn parse_input_descriptors_stops_at_sentinel() {
        // Reproduces a realistic FCEUmm-shaped publish: 4 face-button
        // descriptors at port 0 / JOYPAD / index 0, with the sentinel
        // immediately after. Asserts walk stops at sentinel, not over-
        // reads the buffer.
        let (_cs, mut descs) = build_input_descs(&[
            (0, 1, 0,  0, "Up"),
            (0, 1, 0,  8, "A"),
            (0, 1, 0,  9, "B"),
            (0, 1, 0,  3, "Start"),
        ]);
        descs.push(input_desc_sentinel());

        let parsed = unsafe { parse_input_descriptors(descs.as_ptr()) };
        assert_eq!(parsed.len(), 4, "should stop at sentinel, not over-read");
        assert_eq!(parsed[0].description, "Up");
        assert_eq!(parsed[1].id, 8);
        assert_eq!(parsed[1].description, "A");
        assert_eq!(parsed[2].description, "B");
        assert_eq!(parsed[3].description, "Start");
    }

    #[test]
    fn parse_input_descriptors_preserves_per_game_semantics() {
        // The whole point: different games get different labels for
        // the same physical button. Simulates loading Castlevania
        // followed by Super Mario Bros — second publish overwrites
        // first via with_state, but the parser itself must faithfully
        // reproduce whatever the core hands it.
        //
        // FCEUmm RetroPad button id mapping: A = 8, B = 0 (per
        // libretro.h RETRO_DEVICE_ID_JOYPAD_A / _B), so this test
        // uses those.
        let (_cs1, mut castlevania) =
            build_input_descs(&[(0, 1, 0, 8, "Whip"), (0, 1, 0, 0, "Jump")]);
        castlevania.push(input_desc_sentinel());

        let (_cs2, mut mario) =
            build_input_descs(&[(0, 1, 0, 8, "Run"), (0, 1, 0, 0, "Jump")]);
        mario.push(input_desc_sentinel());

        let p1 = unsafe { parse_input_descriptors(castlevania.as_ptr()) };
        assert_eq!(p1[0].description, "Whip");
        let p2 = unsafe { parse_input_descriptors(mario.as_ptr()) };
        assert_eq!(p2[0].description, "Run");
        // Same physical button (port 0 / JOYPAD / id 8) → different
        // semantic label per game. This is the whole reason the data
        // is per-game in the spec.
        assert_eq!(p1[0].id, p2[0].id);
        assert_ne!(p1[0].description, p2[0].description);
    }

    #[test]
    fn parse_input_descriptors_clones_strings_owned_not_aliased() {
        // After parse, the source CStrings get dropped — the parsed
        // strings must still be valid. Mirrors the controller-info
        // ownership test; would catch a future refactor that stored
        // borrowed `&str` instead of owned `String`.
        let parsed_then_dropped = {
            let (cs, mut descs) = build_input_descs(&[(0, 1, 0, 8, "Whip")]);
            descs.push(input_desc_sentinel());
            let parsed = unsafe { parse_input_descriptors(descs.as_ptr()) };
            drop(cs);
            drop(descs);
            parsed
        };
        assert_eq!(parsed_then_dropped[0].description, "Whip");
    }

    #[test]
    fn parse_input_descriptors_handles_multiple_ports_devices_axes() {
        // Multi-port + multi-device + multi-index publish: port 0
        // JOYPAD + port 0 ANALOG (both sticks) + port 1 LIGHTGUN.
        // Asserts the (port, device, index, id) tuple is preserved
        // verbatim — frontend's lookup keys depend on all four.
        let (_cs, mut descs) = build_input_descs(&[
            (0, 1, 0,  0, "D-Pad Up"),            // port 0, JOYPAD, id Up
            (0, 5, 0,  0, "Left stick X"),        // port 0, ANALOG, index 0=LEFT, id 0=X
            (0, 5, 0,  1, "Left stick Y"),        // port 0, ANALOG, index 0=LEFT, id 1=Y
            (0, 5, 1,  0, "Right stick X"),       // port 0, ANALOG, index 1=RIGHT, id 0=X
            (0, 5, 1,  1, "Right stick Y"),       // port 0, ANALOG, index 1=RIGHT, id 1=Y
            (1, 4, 0, 13, "Lightgun trigger"),    // port 1, LIGHTGUN, id 13
        ]);
        descs.push(input_desc_sentinel());

        let parsed = unsafe { parse_input_descriptors(descs.as_ptr()) };
        assert_eq!(parsed.len(), 6);
        assert_eq!((parsed[1].port, parsed[1].device, parsed[1].index, parsed[1].id),
                   (0, 5, 0, 0));
        assert_eq!(parsed[1].description, "Left stick X");
        assert_eq!((parsed[3].port, parsed[3].device, parsed[3].index, parsed[3].id),
                   (0, 5, 1, 0));
        assert_eq!(parsed[3].description, "Right stick X");
        assert_eq!((parsed[5].port, parsed[5].device),
                   (1, 4));
        assert_eq!(parsed[5].description, "Lightgun trigger");
    }

    #[test]
    fn format_input_descriptors_for_log_truncates_long_lists() {
        // Street Fighter II–shaped advertisement (~30 descriptors).
        // Formatter caps at 6 with a "+N more" tail so the operator
        // log line stays a one-liner.
        let many: Vec<InputDescriptor> = (0..30u32)
            .map(|i| InputDescriptor {
                port: 0,
                device: 1,
                index: 0,
                id: i,
                description: format!("Btn{i}"),
            })
            .collect();
        let formatted = format_input_descriptors_for_log(&many);
        assert!(formatted.starts_with("[p0/d1/i0/id0=Btn0"));
        assert!(formatted.contains("Btn5"));
        assert!(!formatted.contains("Btn6"));
        assert!(formatted.contains("+24 more"));
    }

    #[test]
    fn format_input_descriptors_for_log_empty_renders_brackets() {
        assert_eq!(format_input_descriptors_for_log(&[]), "[]");
    }

    // ---- parse_min_audio_latency_ms (env 63 payload parser) -----------
    //
    // The env handler is a thin wrapper around this parser + a
    // singleton write — exercising the parser in isolation keeps the
    // tests free of singleton-mutex parallelism races. Singleton
    // integration is covered indirectly by the production load path
    // (operator validation in Slice C).

    #[test]
    fn parse_min_audio_latency_typical_value_passes_through() {
        // Genesis Plus GX / Beetle PSX / Flycast typically request
        // 64-100 ms during heavy frames. Under the spec ceiling →
        // passes through verbatim.
        let payload: u32 = 64;
        let data = &payload as *const u32 as *const c_void;
        assert_eq!(unsafe { parse_min_audio_latency_ms(data) }, 64);
    }

    #[test]
    fn parse_min_audio_latency_zero_is_explicit_default() {
        // Per spec docstring (libretro.h:1378): 0 means "use default
        // frontend audio latency." Parser preserves it; downstream
        // ensure_min_latency_ms(0) is a no-op since any current
        // capacity already covers 0 ms.
        let payload: u32 = 0;
        let data = &payload as *const u32 as *const c_void;
        assert_eq!(unsafe { parse_min_audio_latency_ms(data) }, 0);
    }

    #[test]
    fn parse_min_audio_latency_caps_at_spec_ceiling() {
        // Spec asks frontends to honor up to 512 ms; cap defends
        // against a buggy core declaring something pathological that
        // would balloon oa-audio's ring buffer to GBs.
        let payload: u32 = 99999;
        let data = &payload as *const u32 as *const c_void;
        assert_eq!(
            unsafe { parse_min_audio_latency_ms(data) },
            512,
            "values above 512 ms must clamp to the spec ceiling",
        );
    }

    #[test]
    fn parse_min_audio_latency_at_exact_ceiling_passes() {
        // Boundary: exactly 512 ms is the max per spec, not above it,
        // so passes through.
        let payload: u32 = 512;
        let data = &payload as *const u32 as *const c_void;
        assert_eq!(unsafe { parse_min_audio_latency_ms(data) }, 512);
    }

    #[test]
    fn parse_min_audio_latency_null_payload_returns_zero() {
        // Spec doesn't explicitly allow NULL data here, but defensively
        // handle it the same as "use default" (0 ms) rather than
        // dereferencing a null pointer.
        assert_eq!(unsafe { parse_min_audio_latency_ms(std::ptr::null()) }, 0);
    }

    #[test]
    fn format_controller_devices_for_log_truncates_long_lists() {
        // Sanity-check the log helper. FCEUmm's Famicom expansion port
        // (port 4) advertises ~12 entries; the formatter caps at 8 with
        // a "+N more" suffix so the operator-visible startup line stays
        // a one-liner.
        let port_devs: Vec<ControllerDeviceDescriptor> = (0..12u32)
            .map(|i| ControllerDeviceDescriptor {
                label: format!("Dev{i}"),
                id: 100 + i,
            })
            .collect();
        let formatted = format_controller_devices_for_log(&[port_devs]);
        assert!(formatted.contains("port 0 = ["));
        assert!(formatted.contains("Dev0=100"));
        assert!(formatted.contains("Dev7=107"));
        assert!(!formatted.contains("Dev8=108"));
        assert!(formatted.contains("+4 more"));
    }

    // ---- pointer_field_value ------------------------------------------

    /// Released secondary slot used in single-pointer tests so the
    /// pre-Phase-3 semantics keep reading clearly.
    const NO_SECONDARY: (i16, i16, bool, bool) = (0, 0, false, false);

    #[test]
    fn pointer_field_value_returns_stored_coords() {
        // Mid-screen click, pressed, inside viewport, no second finger.
        // Index 0 = primary slot — pre-Phase-3 single-pointer behaviour.
        let p = (1000i16, -2000i16, true, true);
        let s = NO_SECONDARY;
        assert_eq!(pointer_field_value(p, s, 0, RETRO_DEVICE_ID_POINTER_X),       1000);
        assert_eq!(pointer_field_value(p, s, 0, RETRO_DEVICE_ID_POINTER_Y),       -2000);
        assert_eq!(pointer_field_value(p, s, 0, RETRO_DEVICE_ID_POINTER_PRESSED), 1);
        assert_eq!(pointer_field_value(p, s, 0, RETRO_DEVICE_ID_POINTER_COUNT),   1);
    }

    #[test]
    fn pointer_field_value_unpressed_reports_zero_count() {
        // Pointer at min coords, released, no second finger. COUNT
        // must follow PRESSED total — 0 here.
        let p = (-32768i16, -32768i16, false, true);
        let s = NO_SECONDARY;
        assert_eq!(pointer_field_value(p, s, 0, RETRO_DEVICE_ID_POINTER_X),       -32768);
        assert_eq!(pointer_field_value(p, s, 0, RETRO_DEVICE_ID_POINTER_Y),       -32768);
        assert_eq!(pointer_field_value(p, s, 0, RETRO_DEVICE_ID_POINTER_PRESSED), 0);
        assert_eq!(pointer_field_value(p, s, 0, RETRO_DEVICE_ID_POINTER_COUNT),   0);
    }

    #[test]
    fn pointer_field_value_unknown_id_returns_zero() {
        // Anything outside the 4 documented IDs returns 0 — defensive
        // against future libretro spec additions.
        let p = (100i16, 200i16, true, true);
        let s = NO_SECONDARY;
        for id in [4u32, 7, 42, 9999] {
            assert_eq!(pointer_field_value(p, s, 0, id), 0, "id {id} must return 0");
        }
    }

    #[test]
    fn pointer_field_value_ignores_in_viewport_flag() {
        // POINTER device polls don't read the in_viewport flag — that's
        // the LIGHTGUN device's IS_OFFSCREEN territory. POINTER results
        // must be identical whether the flag is true or false (so NDS
        // stylus dispatch doesn't accidentally suppress when the cursor
        // leaves the viewport).
        let p_in  = (1000i16, -2000i16, true, true);
        let p_out = (1000i16, -2000i16, true, false);
        let s = NO_SECONDARY;
        assert_eq!(
            pointer_field_value(p_in,  s, 0, RETRO_DEVICE_ID_POINTER_X),
            pointer_field_value(p_out, s, 0, RETRO_DEVICE_ID_POINTER_X),
        );
        assert_eq!(
            pointer_field_value(p_in,  s, 0, RETRO_DEVICE_ID_POINTER_PRESSED),
            pointer_field_value(p_out, s, 0, RETRO_DEVICE_ID_POINTER_PRESSED),
        );
    }

    // ---- pointer_field_value — multi-touch (index 1+) -----------------

    #[test]
    fn pointer_field_value_index_1_returns_secondary_coords() {
        // Two distinct pointers down. Index 0 reads primary; index 1
        // reads secondary. Coords + PRESSED differ between slots so
        // a swap regression would surface immediately.
        let primary   = (1000i16, -2000i16, true, true);
        let secondary = (-5000i16, 7777i16, true, true);
        assert_eq!(pointer_field_value(primary, secondary, 0, RETRO_DEVICE_ID_POINTER_X), 1000);
        assert_eq!(pointer_field_value(primary, secondary, 0, RETRO_DEVICE_ID_POINTER_Y), -2000);
        assert_eq!(pointer_field_value(primary, secondary, 1, RETRO_DEVICE_ID_POINTER_X), -5000);
        assert_eq!(pointer_field_value(primary, secondary, 1, RETRO_DEVICE_ID_POINTER_Y), 7777);
        assert_eq!(pointer_field_value(primary, secondary, 0, RETRO_DEVICE_ID_POINTER_PRESSED), 1);
        assert_eq!(pointer_field_value(primary, secondary, 1, RETRO_DEVICE_ID_POINTER_PRESSED), 1);
    }

    #[test]
    fn pointer_field_value_index_out_of_range_returns_zero() {
        // libretro's `index` parameter is u32. Anything past index 1
        // is undefined territory — return zero so cores polling
        // (NDS homebrew that loops poll until COUNT) don't read stale
        // primary data when they intended to ask about a third finger.
        let primary   = (1000i16, -2000i16, true, true);
        let secondary = (-5000i16, 7777i16, true, true);
        for idx in [2u32, 3, 7, 99, u32::MAX] {
            assert_eq!(
                pointer_field_value(primary, secondary, idx, RETRO_DEVICE_ID_POINTER_X),
                0, "index {idx} POINTER_X must return 0",
            );
            assert_eq!(
                pointer_field_value(primary, secondary, idx, RETRO_DEVICE_ID_POINTER_PRESSED),
                0, "index {idx} POINTER_PRESSED must return 0",
            );
        }
    }

    #[test]
    fn pointer_field_value_count_sums_pressed_slots() {
        // COUNT semantics: total pressed across both slots,
        // independent of `index`. 0 / 1 / 2 covers the full domain.
        let down  = (0i16, 0i16, true,  true);
        let up    = (0i16, 0i16, false, true);
        // Both up → 0
        for idx in [0u32, 1] {
            assert_eq!(pointer_field_value(up, up, idx, RETRO_DEVICE_ID_POINTER_COUNT), 0);
        }
        // Only primary → 1 (regardless of index)
        for idx in [0u32, 1] {
            assert_eq!(pointer_field_value(down, up, idx, RETRO_DEVICE_ID_POINTER_COUNT), 1);
        }
        // Only secondary → 1
        for idx in [0u32, 1] {
            assert_eq!(pointer_field_value(up, down, idx, RETRO_DEVICE_ID_POINTER_COUNT), 1);
        }
        // Both down → 2
        for idx in [0u32, 1] {
            assert_eq!(pointer_field_value(down, down, idx, RETRO_DEVICE_ID_POINTER_COUNT), 2);
        }
    }

    #[test]
    fn pointer_field_value_count_unaffected_by_out_of_range_index() {
        // Even with index ≥ 2 (where X/Y/PRESSED return 0), COUNT
        // still answers the total-pressed question — cores that
        // pre-poll COUNT before iterating fingers shouldn't see 0
        // just because they probed an arbitrarily-high index.
        let down = (0i16, 0i16, true, true);
        assert_eq!(pointer_field_value(down, down, 2, RETRO_DEVICE_ID_POINTER_COUNT), 2);
        assert_eq!(pointer_field_value(down, down, 99, RETRO_DEVICE_ID_POINTER_COUNT), 2);
    }

    // ---- lightgun_field_value -----------------------------------------

    /// No gun-side buttons pressed — passed when only the geometry +
    /// trigger ids are under test.
    const NO_GUN_BUTTONS: u32 = 0;

    #[test]
    fn lightgun_field_value_screen_coords_match_pointer_state() {
        // The screen-coordinate IDs (modern) must return the same
        // coords as the underlying input_pointer state.
        let p = (12345i16, -7890i16, true, true);
        assert_eq!(lightgun_field_value(p, NO_GUN_BUTTONS, RETRO_DEVICE_ID_LIGHTGUN_SCREEN_X), 12345);
        assert_eq!(lightgun_field_value(p, NO_GUN_BUTTONS, RETRO_DEVICE_ID_LIGHTGUN_SCREEN_Y), -7890);
    }

    #[test]
    fn lightgun_field_value_deprecated_x_y_alias_screen_coords() {
        // Legacy LIGHTGUN_X / LIGHTGUN_Y aliases (id 0/1) must report
        // the same absolute coords. Some older cores (Genesis Plus GX
        // Light Phaser path) still poll the relative aliases.
        let p = (-100i16, 200i16, false, true);
        assert_eq!(lightgun_field_value(p, NO_GUN_BUTTONS, RETRO_DEVICE_ID_LIGHTGUN_X), -100);
        assert_eq!(lightgun_field_value(p, NO_GUN_BUTTONS, RETRO_DEVICE_ID_LIGHTGUN_Y), 200);
        // Confirm the alias matches the modern SCREEN_X result for
        // the same input — a regression that desyncs them would silently
        // break older cores.
        assert_eq!(
            lightgun_field_value(p, NO_GUN_BUTTONS, RETRO_DEVICE_ID_LIGHTGUN_X),
            lightgun_field_value(p, NO_GUN_BUTTONS, RETRO_DEVICE_ID_LIGHTGUN_SCREEN_X),
        );
    }

    #[test]
    fn lightgun_field_value_trigger_follows_pressed() {
        let pressed  = (0i16, 0i16, true,  true);
        let released = (0i16, 0i16, false, true);
        assert_eq!(lightgun_field_value(pressed,  NO_GUN_BUTTONS, RETRO_DEVICE_ID_LIGHTGUN_TRIGGER), 1);
        assert_eq!(lightgun_field_value(released, NO_GUN_BUTTONS, RETRO_DEVICE_ID_LIGHTGUN_TRIGGER), 0);
    }

    #[test]
    fn lightgun_field_value_trigger_ignores_buttons_bitmask() {
        // TRIGGER (id 2) stays driven by pointer.pressed only, not by
        // bit 2 of the buttons bitmask. Setting bit 2 in `buttons`
        // shouldn't fire the trigger if the mouse isn't held.
        let released = (0i16, 0i16, false, true);
        let bit_2_set = 1u32 << RETRO_DEVICE_ID_LIGHTGUN_TRIGGER;
        assert_eq!(
            lightgun_field_value(released, bit_2_set, RETRO_DEVICE_ID_LIGHTGUN_TRIGGER),
            0, "bit 2 in buttons must NOT activate TRIGGER — TRIGGER follows pointer.pressed",
        );
    }

    #[test]
    fn lightgun_field_value_is_offscreen_inverts_in_viewport() {
        // 2026-05-27: the in_viewport plumbing landed end-to-end
        // (poll_pointer → InputState.pointer → input_pointer tuple
        // → lightgun_field_value). Cursor inside the viewport reports
        // IS_OFFSCREEN=0; cursor outside reports IS_OFFSCREEN=1 so
        // light-gun cores see the reload-by-aim-off-screen gesture.
        let p_inside  = (100i16, 100i16, true,  true);
        let p_outside = (0i16,   0i16,   false, false);
        assert_eq!(lightgun_field_value(p_inside,  NO_GUN_BUTTONS, RETRO_DEVICE_ID_LIGHTGUN_IS_OFFSCREEN), 0);
        assert_eq!(lightgun_field_value(p_outside, NO_GUN_BUTTONS, RETRO_DEVICE_ID_LIGHTGUN_IS_OFFSCREEN), 1);
    }

    #[test]
    fn lightgun_field_value_is_offscreen_independent_of_press_state() {
        // The reload gesture must fire whether or not the operator is
        // holding the trigger — some games accept "aim off-screen,
        // release fire to reload" while others want "aim off-screen,
        // press fire to reload." IS_OFFSCREEN reflects geometry only,
        // not press state.
        let off_unpressed = (0i16, 0i16, false, false);
        let off_pressed   = (0i16, 0i16, true,  false);
        assert_eq!(
            lightgun_field_value(off_unpressed, NO_GUN_BUTTONS, RETRO_DEVICE_ID_LIGHTGUN_IS_OFFSCREEN),
            lightgun_field_value(off_pressed,   NO_GUN_BUTTONS, RETRO_DEVICE_ID_LIGHTGUN_IS_OFFSCREEN),
        );
    }

    #[test]
    fn lightgun_field_value_gun_side_buttons_unpressed_when_bitmask_zero() {
        // With `buttons == 0`, every gun-side id reports unpressed.
        // Preserves the pre-Phase-4 fresh-setup default — cores polling
        // AUX / START / SELECT / DPAD / RELOAD see neutral unless the
        // operator has actively pressed a bound input.
        let p = (500i16, -500i16, true, true);
        for id in [
            RETRO_DEVICE_ID_LIGHTGUN_AUX_A,
            RETRO_DEVICE_ID_LIGHTGUN_AUX_B,
            RETRO_DEVICE_ID_LIGHTGUN_AUX_C,
            RETRO_DEVICE_ID_LIGHTGUN_START,
            RETRO_DEVICE_ID_LIGHTGUN_SELECT,
            RETRO_DEVICE_ID_LIGHTGUN_DPAD_UP,
            RETRO_DEVICE_ID_LIGHTGUN_DPAD_DOWN,
            RETRO_DEVICE_ID_LIGHTGUN_DPAD_LEFT,
            RETRO_DEVICE_ID_LIGHTGUN_DPAD_RIGHT,
            RETRO_DEVICE_ID_LIGHTGUN_RELOAD,
        ] {
            assert_eq!(
                lightgun_field_value(p, NO_GUN_BUTTONS, id), 0,
                "id {id} with buttons=0 must read unpressed",
            );
        }
    }

    #[test]
    fn lightgun_field_value_gun_side_buttons_follow_matching_bit() {
        // Each gun-side id reads bit `id` of `buttons`. Setting the
        // matching bit must produce 1; all other ids stay 0. This
        // catches a bit-position regression (e.g. accidentally reading
        // `id - 1` or some shifted offset).
        let p = (0i16, 0i16, false, true);
        let gun_ids = [
            RETRO_DEVICE_ID_LIGHTGUN_AUX_A,
            RETRO_DEVICE_ID_LIGHTGUN_AUX_B,
            RETRO_DEVICE_ID_LIGHTGUN_AUX_C,
            RETRO_DEVICE_ID_LIGHTGUN_START,
            RETRO_DEVICE_ID_LIGHTGUN_SELECT,
            RETRO_DEVICE_ID_LIGHTGUN_DPAD_UP,
            RETRO_DEVICE_ID_LIGHTGUN_DPAD_DOWN,
            RETRO_DEVICE_ID_LIGHTGUN_DPAD_LEFT,
            RETRO_DEVICE_ID_LIGHTGUN_DPAD_RIGHT,
            RETRO_DEVICE_ID_LIGHTGUN_RELOAD,
        ];
        for &target in &gun_ids {
            let buttons = 1u32 << target;
            for &poll_id in &gun_ids {
                let want = if poll_id == target { 1 } else { 0 };
                assert_eq!(
                    lightgun_field_value(p, buttons, poll_id), want,
                    "buttons bit {target} set → polling id {poll_id} must be {want}",
                );
            }
        }
    }

    #[test]
    fn lightgun_field_value_reload_bit_at_id_16() {
        // RELOAD is libretro id 16 — the highest gun-side id and the
        // reason `lightgun_buttons` is u32 rather than u16. Verify
        // bit 16 specifically (a u16 truncation would silently drop
        // this one to bit 0).
        let p = (0i16, 0i16, false, true);
        let buttons = 1u32 << RETRO_DEVICE_ID_LIGHTGUN_RELOAD;
        assert_eq!(
            lightgun_field_value(p, buttons, RETRO_DEVICE_ID_LIGHTGUN_RELOAD), 1,
            "bit 16 (RELOAD) must register as pressed when set",
        );
        // Off-screen reload-by-aim gesture remains independent —
        // polling IS_OFFSCREEN with the RELOAD bit set still reports
        // 0 when the pointer is on-screen.
        assert_eq!(
            lightgun_field_value(p, buttons, RETRO_DEVICE_ID_LIGHTGUN_IS_OFFSCREEN), 0,
        );
    }

    #[test]
    fn lightgun_field_value_unknown_id_returns_zero() {
        // Spec compliance: any id outside the documented LIGHTGUN
        // table returns 0 rather than leaking stack / uninit.
        let p = (1i16, 2i16, true, true);
        // Pass an all-bits-set buttons to confirm unknown ids don't
        // accidentally read whichever bit happens to share their
        // value.
        for id in [17u32, 50, 100, 9999, u32::MAX] {
            assert_eq!(lightgun_field_value(p, u32::MAX, id), 0, "unknown id {id} must return 0");
        }
    }

    // ---- parse_memory_descriptors ------------------------------------

    #[test]
    fn parse_memory_descriptors_handles_null_array() {
        // libretro spec allows cores to call SET_MEMORY_MAPS with a NULL
        // descriptor pointer to clear an earlier map. The parser must
        // return empty Vecs without dereferencing.
        let (descs, ptrs) = unsafe { parse_memory_descriptors(std::ptr::null(), 5) };
        assert!(descs.is_empty(), "null array must produce empty descriptors");
        assert!(ptrs.is_empty(), "null array must produce empty pointers");
    }

    #[test]
    fn parse_memory_descriptors_handles_zero_count() {
        // n=0 short-circuits regardless of pointer — a non-null pointer
        // with zero entries shouldn't read any memory.
        let dummy: retro_memory_descriptor = retro_memory_descriptor {
            flags: 0,
            ptr: std::ptr::null_mut(),
            offset: 0,
            start: 0,
            select: 0,
            disconnect: 0,
            len: 0,
            addrspace: std::ptr::null(),
        };
        let (descs, ptrs) = unsafe { parse_memory_descriptors(&dummy, 0) };
        assert!(descs.is_empty());
        assert!(ptrs.is_empty());
    }

    #[test]
    fn parse_memory_descriptors_copies_metadata_and_keeps_pointer_separately() {
        // Simulate a 2-region NES-shape map: WRAM at $0000-$07FF and
        // PRG-ROM at $8000-$FFFF. Verify the metadata copies in cleanly
        // and the host pointer comes back in the parallel Vec.
        let mut buf1 = [0u8; 0x800];
        let mut buf2 = [0u8; 0x8000];
        let s_tag = CString::new("S").unwrap();
        let descs_in = [
            retro_memory_descriptor {
                flags: 0x4, // RETRO_MEMDESC_SYSTEM_RAM
                ptr: buf1.as_mut_ptr() as *mut c_void,
                offset: 0,
                start: 0x0000,
                select: 0,
                disconnect: 0,
                len: 0x800,
                addrspace: s_tag.as_ptr(),
            },
            retro_memory_descriptor {
                flags: 0x1, // RETRO_MEMDESC_CONST
                ptr: buf2.as_mut_ptr() as *mut c_void,
                offset: 0,
                start: 0x8000,
                select: 0,
                disconnect: 0,
                len: 0x8000,
                addrspace: std::ptr::null(),
            },
        ];
        let (descs, ptrs) =
            unsafe { parse_memory_descriptors(descs_in.as_ptr(), descs_in.len() as u32) };

        assert_eq!(descs.len(), 2);
        assert_eq!(ptrs.len(), 2);

        assert_eq!(descs[0].flags, 0x4);
        assert_eq!(descs[0].start, 0x0000);
        assert_eq!(descs[0].len, 0x800);
        assert_eq!(descs[0].addrspace.as_deref(), Some("S"));

        assert_eq!(descs[1].flags, 0x1);
        assert_eq!(descs[1].start, 0x8000);
        assert_eq!(descs[1].len, 0x8000);
        assert_eq!(descs[1].addrspace, None, "null addrspace must yield None");

        // Host pointers come back through the parallel ptrs Vec — not
        // exposed in oa_core::MemoryDescriptor. The values match what
        // we passed in (raw usize transmute), so the storage roundtrips
        // cleanly even though oa-core sees no pointers.
        assert_eq!(ptrs[0], buf1.as_ptr() as usize);
        assert_eq!(ptrs[1], buf2.as_ptr() as usize);
    }
}
