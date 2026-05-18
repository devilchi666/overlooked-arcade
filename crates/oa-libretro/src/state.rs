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

use std::ffi::{c_void, CStr, CString};
use std::os::raw::c_char;
use std::sync::{LazyLock, Mutex};

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
    pub pending_info_ext: retro_game_info_ext,
    /// Whether the core has been initialised (retro_init called). Drop uses this
    /// to decide whether retro_deinit is needed.
    pub initialised: bool,
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
            display_aspect: 0.0,
            system_dir: CString::new(".").unwrap(),
            save_dir: CString::new(".").unwrap(),
            pending_rom_data: std::ptr::null(),
            pending_rom_size: 0,
            pending_name: CString::new("rom").unwrap(),
            pending_ext: CString::new("").unwrap(),
            pending_info_ext: zeroed_info_ext(),
            initialised: false,
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

pub(crate) unsafe extern "C" fn cb_input_state(
    port: u32,
    device: u32,
    _index: u32,
    id: u32,
) -> i16 {
    /// Special id passed by cores that use the bitmask API (we ack'd
    /// `RETRO_ENVIRONMENT_GET_INPUT_BITMASKS`). The core asks for the full
    /// joypad state in one call instead of polling each button individually.
    const RETRO_DEVICE_ID_JOYPAD_MASK: u32 = 256;

    if port >= 5 || device != RETRO_DEVICE_JOYPAD {
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
                    full_path: empty,
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
            // Reply "not set" for every option — core falls back to its compiled
            // defaults, which is what we want for first-cut parity.
            if data.is_null() {
                return false;
            }
            let var = unsafe { &mut *(data as *mut retro_variable) };
            var.value = std::ptr::null();
            false
        }
        // Core option declarations — accept all (we ignore the actual option
        // schemas; GET_VARIABLE returns "not set" so the core uses defaults).
        // Returning true keeps cores like Beetle from skipping their internal
        // option-init logic, which some versions tie to a "frontend supports
        // options" flag set by these acks.
        RETRO_ENVIRONMENT_SET_VARIABLES
        | RETRO_ENVIRONMENT_SET_CORE_OPTIONS
        | RETRO_ENVIRONMENT_SET_CORE_OPTIONS_INTL
        | RETRO_ENVIRONMENT_SET_CORE_OPTIONS_V2
        | RETRO_ENVIRONMENT_SET_CORE_OPTIONS_V2_INTL
        | RETRO_ENVIRONMENT_SET_CORE_OPTIONS_DISPLAY => true,

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

        // Decline these — we don't have rumble / sensors / camera / location
        // / keyboard / HW render interfaces, and lying about them risks the
        // core calling a null function pointer.
        RETRO_ENVIRONMENT_GET_RUMBLE_INTERFACE
        | RETRO_ENVIRONMENT_GET_SENSOR_INTERFACE
        | RETRO_ENVIRONMENT_GET_CAMERA_INTERFACE
        | RETRO_ENVIRONMENT_GET_LOCATION_INTERFACE
        | RETRO_ENVIRONMENT_SET_KEYBOARD_CALLBACK
        | RETRO_ENVIRONMENT_SET_HW_RENDER => false,
        RETRO_ENVIRONMENT_GET_VARIABLE_UPDATE => {
            if data.is_null() {
                return false;
            }
            unsafe {
                *(data as *mut bool) = false;
            }
            true
        }
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
