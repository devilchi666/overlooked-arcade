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
use std::sync::{LazyLock, Mutex};

use oa_core::{CoreOption, CoreOptionCategory, CoreOptionValue};

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
    /// Per-port pointer device state. Layout: `(x, y, pressed)` per
    /// port. Used by Nintendo DS stylus, Saturn / Dreamcast / arcade
    /// light-gun games. Set per frame via LibretroCore::set_input
    /// from `InputState.pointer`; cb_input_state returns the right
    /// component for RETRO_DEVICE_POINTER queries.
    pub input_pointer: [(i16, i16, bool); 5],
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
            input_pointer: [(0, 0, false); 5],
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
            rotation: 0,
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

// ---------- extern "C" callback trampolines ----------

pub(crate) unsafe extern "C" fn cb_video_refresh(
    data: *const c_void,
    width: u32,
    height: u32,
    pitch: usize,
) {
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

    // Pointer device — touch screen / mouse-as-touch / light gun.
    // Cores polling this get the per-port (x, y, pressed) tuple back.
    // OA's `index` is ignored at Phase 0 (multi-touch is Phase 2.5);
    // single-pointer support handles NDS stylus + light-gun titles.
    if device == RETRO_DEVICE_POINTER {
        return with_state(|s| {
            let (x, y, pressed) = s.input_pointer[port as usize];
            match id {
                RETRO_DEVICE_ID_POINTER_X       => x,
                RETRO_DEVICE_ID_POINTER_Y       => y,
                RETRO_DEVICE_ID_POINTER_PRESSED => if pressed { 1 } else { 0 },
                // POINTER_COUNT — returns the number of active pointers.
                // Phase 0 supports single-pointer only; report 1 if
                // pressed, 0 if not.
                RETRO_DEVICE_ID_POINTER_COUNT   => if pressed { 1 } else { 0 },
                _ => 0,
            }
        }).unwrap_or(0);
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
        RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS => true,
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

        // Setters where accepting is harmless — we don't store the declared
        // data, but acknowledging it keeps cores from bailing on init when
        // their setup path checks for frontend acks.
        RETRO_ENVIRONMENT_SET_PERFORMANCE_LEVEL
        | RETRO_ENVIRONMENT_SET_SUBSYSTEM_INFO
        | RETRO_ENVIRONMENT_SET_CONTROLLER_INFO
        | RETRO_ENVIRONMENT_SET_MEMORY_MAPS
        | RETRO_ENVIRONMENT_SET_SERIALIZATION_QUIRKS
        | RETRO_ENVIRONMENT_SET_MINIMUM_AUDIO_LATENCY
        | RETRO_ENVIRONMENT_SET_FASTFORWARDING_OVERRIDE => true,

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
        // Decline these — we don't have camera / location / HW render
        // interfaces, and lying about them risks the core calling a
        // null function pointer.
        RETRO_ENVIRONMENT_GET_CAMERA_INTERFACE
        | RETRO_ENVIRONMENT_GET_LOCATION_INTERFACE
        | RETRO_ENVIRONMENT_SET_HW_RENDER => false,

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
