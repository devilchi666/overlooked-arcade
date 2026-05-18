//! `LibretroCore` — `oa_core::Core` impl over a dynamically-loaded libretro
//! core. The shell loads a .dll/.so/.dylib once at app start (or per ROM if
//! we go multi-core), and operates on this handle as if it were a statically-
//! linked core.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;

use oa_core::{Core, CoreError, Framebuffer, InputState, MemoryRegionId, PortIndex, SystemId, Timing};

use crate::ffi::*;
use crate::loader::LibretroLibrary;
use crate::state::{self, State};

/// How the ROM gets handed to the libretro core. HuCard / cart formats
/// (`.pce`, `.sgx`, `.nes`, `.smc`, etc.) load fine from in-memory bytes; CD
/// images (`.cue` + multiple `.bin` tracks, `.chd`, `.toc`, `.m3u` playlists)
/// MUST go via a filesystem path because the core opens additional files
/// relative to the path. Cores that set `need_fullpath = true` in
/// `retro_system_info` always need the Path variant.
pub enum RomSource<'a> {
    /// In-memory bytes — typical for cart / HuCard images.
    Bytes(&'a [u8]),
    /// Filesystem path — required for multi-file CD images.
    Path(&'a Path),
}

/// Summary of a libretro core as reported by `retro_get_system_info`. Used by
/// the cores-folder scanner UI — opens the .dll briefly, reads the static
/// info strings, drops the library handle. Doesn't call `retro_init` so the
/// core's globals stay untouched.
#[derive(Debug, Clone)]
pub struct CoreInfo {
    /// Filename of the .dll/.so/.dylib (no path).
    pub file_name: String,
    /// Library name reported by the core (e.g. "Beetle PCE Fast").
    pub library_name: String,
    /// Version string reported by the core (e.g. "0.9.48 e22b706").
    pub library_version: String,
    /// `|`-separated list of file extensions the core handles (e.g. "pce|cue|chd|toc|m3u").
    pub valid_extensions: String,
    /// Core requires a real filesystem path (multi-file CD images etc.); the
    /// `RomSource::Bytes` path will refuse to load ROMs into this core.
    pub need_fullpath: bool,
    /// Core handles its own archive extraction internally; the frontend
    /// should leave archive contents intact when handing the file off.
    pub block_extract: bool,
}

/// Open a libretro .dll, read `retro_get_system_info`, drop the library.
/// Used to enumerate cores for the picker UI without paying for a full init.
///
/// Safe to call against a .dll that's also currently loaded as an active
/// `LibretroCore` — the OS refcounts the load, and we never call `retro_init`
/// here, so the core's runtime state isn't touched.
pub fn probe(path: &Path) -> Result<CoreInfo, LibretroError> {
    let lib = LibretroLibrary::open(path)?;
    let mut sys = retro_system_info {
        library_name: std::ptr::null(),
        library_version: std::ptr::null(),
        valid_extensions: std::ptr::null(),
        need_fullpath: false,
        block_extract: false,
    };
    unsafe { (lib.fns.get_system_info)(&mut sys) };
    let library_name = unsafe { c_str_to_owned(sys.library_name) };
    let library_version = unsafe { c_str_to_owned(sys.library_version) };
    let valid_extensions = unsafe { c_str_to_owned(sys.valid_extensions) };
    let need_fullpath = sys.need_fullpath;
    let block_extract = sys.block_extract;
    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(CoreInfo {
        file_name,
        library_name,
        library_version,
        valid_extensions,
        need_fullpath,
        block_extract,
    })
}

unsafe fn c_str_to_owned(p: *const c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

/// Errors from loading or driving a libretro core.
#[derive(Debug, thiserror::Error)]
pub enum LibretroError {
    /// Library couldn't be opened or a required symbol was missing.
    #[error("libretro library load failed: {0}")]
    Load(#[from] libloading::Error),
    /// The core reported an unsupported ABI version.
    #[error("unsupported libretro API version: {0} (expected 1)")]
    UnsupportedApi(u32),
    /// Trying to load two cores at once. The global callback state is a
    /// singleton, so only one LibretroCore can exist at a time per process.
    #[error("libretro core already loaded (singleton)")]
    AlreadyLoaded,
    /// retro_load_game returned false.
    #[error("retro_load_game returned false (ROM may be unsupported by this core)")]
    LoadGameFailed,
}

/// A loaded libretro core, holding both the .dll handle and the per-core
/// state. Drop runs retro_unload_game + retro_deinit + dlclose.
pub struct LibretroCore {
    lib: LibretroLibrary,
    system_id: SystemId,
    timing: Timing,
    rom_loaded: bool,
    audio_buf: Vec<i16>,
}

impl LibretroCore {
    /// Load a libretro core from a .dll/.so/.dylib at `path` and wire up our
    /// callbacks. `system_id` lets the shell tag this instance — libretro
    /// itself doesn't report which system it emulates in a structured way.
    /// `system_dir` is what the core sees via GET_SYSTEM_DIRECTORY (BIOS dir
    /// for CD cores etc.).
    pub fn load(
        path: &Path,
        system_id: SystemId,
        system_dir: &Path,
        save_dir: &Path,
    ) -> Result<Self, LibretroError> {
        if state::is_loaded() {
            return Err(LibretroError::AlreadyLoaded);
        }
        let lib = LibretroLibrary::open(path)?;

        let api = unsafe { (lib.fns.api_version)() };
        if api != 1 {
            return Err(LibretroError::UnsupportedApi(api));
        }

        // Build the State BEFORE wiring callbacks — set_environment can fire
        // immediately and our callback expects state present.
        let mut state = State::new();
        state.system_dir = CString::new(system_dir.to_string_lossy().as_bytes())
            .unwrap_or_else(|_| CString::new(".").unwrap());
        state.save_dir = CString::new(save_dir.to_string_lossy().as_bytes())
            .unwrap_or_else(|_| CString::new(".").unwrap());
        state::install(state);

        // Register every callback before retro_init: a core may consult its
        // environment during init to negotiate pixel format / variables.
        unsafe {
            (lib.fns.set_environment)(state::cb_environment);
            (lib.fns.set_video_refresh)(state::cb_video_refresh);
            (lib.fns.set_audio_sample)(state::cb_audio_sample);
            (lib.fns.set_audio_sample_batch)(state::cb_audio_sample_batch);
            (lib.fns.set_input_poll)(state::cb_input_poll);
            (lib.fns.set_input_state)(state::cb_input_state);
            (lib.fns.init)();
        }
        state::with_state(|s| s.initialised = true);

        // Initial timing is bogus until a game is loaded (most cores need a
        // ROM to report real values), but we set placeholders so the shell
        // can size things. PCE-ish defaults; will be overwritten on load_rom.
        let timing = Timing {
            width: 256,
            height: 240,
            fps: 60.0,
            sample_rate: 44_100,
        };

        Ok(Self {
            lib,
            system_id,
            timing,
            rom_loaded: false,
            audio_buf: Vec::with_capacity(16384),
        })
    }

    /// Load a ROM from either in-memory bytes or a filesystem path.
    ///
    /// `extension` is the lowercase file extension without the dot (e.g. "pce",
    /// "cue", "chd") — the core uses it via GET_GAME_INFO_EXT to route the load
    /// (HuCard vs CD vs etc.).
    ///
    /// `name` is the ROM stem with no path / no extension (e.g. "Bonk's
    /// Adventure (USA)"). Cores read it via `info_ext->name` for save filenames
    /// + display. The empty string is accepted but discouraged — FCEUmm uses
    /// it to derive .sav paths, so an empty name produces "<save_dir>/.sav".
    ///
    /// CD images (multi-track `.cue`, `.chd`, `.toc`, `.m3u`) must use
    /// [`RomSource::Path`] because the core opens additional files relative to
    /// the path. HuCard / cart formats can use either, but [`RomSource::Bytes`]
    /// avoids a temp file when the ROM came from a download or archive.
    pub fn load_rom(
        &mut self,
        source: RomSource<'_>,
        extension: &str,
        name: &str,
    ) -> Result<(), CoreError> {
        // Unload any previous game first — back-to-back swaps are clean if
        // the core respects retro_unload_game before retro_load_game.
        if self.rom_loaded {
            unsafe { (self.lib.fns.unload_game)() };
            self.rom_loaded = false;
        }

        let ext_cstr = CString::new(extension.as_bytes())
            .map_err(|_| CoreError::InvalidRom("extension contains NUL".into()))?;
        let name_cstr = CString::new(name.as_bytes())
            .map_err(|_| CoreError::InvalidRom("name contains NUL".into()))?;

        // path_cstr must outlive the load_game call below — CString owns its
        // bytes via a Vec, so the pointer is valid until the CString drops.
        let path_cstr: Option<CString>;
        let info = match source {
            RomSource::Bytes(data) => {
                if data.is_empty() {
                    return Err(CoreError::InvalidRom("empty ROM data".into()));
                }
                state::with_state(|s| {
                    s.pending_rom_data = data.as_ptr();
                    s.pending_rom_size = data.len();
                    s.pending_ext = ext_cstr;
                    s.pending_name = name_cstr;
                });
                path_cstr = None;
                retro_game_info {
                    path: std::ptr::null(),
                    data: data.as_ptr() as *const std::ffi::c_void,
                    size: data.len(),
                    meta: std::ptr::null(),
                }
            }
            RomSource::Path(p) => {
                let path_str = p.to_string_lossy();
                log::info!(
                    "oa-libretro: path-based load — path='{}' ({} bytes utf8), exists={}",
                    path_str,
                    path_str.len(),
                    p.is_file(),
                );
                path_cstr = Some(
                    CString::new(path_str.as_bytes())
                        .map_err(|_| CoreError::InvalidRom("ROM path contains NUL".into()))?,
                );
                // Clear data path so GET_GAME_INFO_EXT returns nothing — the
                // core picks up the path from info.path and opens files itself.
                state::with_state(|s| {
                    s.pending_rom_data = std::ptr::null();
                    s.pending_rom_size = 0;
                    s.pending_ext = ext_cstr;
                    s.pending_name = name_cstr;
                });
                retro_game_info {
                    path: path_cstr.as_ref().unwrap().as_ptr(),
                    data: std::ptr::null(),
                    size: 0,
                    meta: std::ptr::null(),
                }
            }
        };

        let ok = unsafe { (self.lib.fns.load_game)(&info) };

        // Clear staging — the core has either copied/refcounted what it needs,
        // or rejected the load. `path_cstr` drops here, freeing its bytes.
        state::with_state(|s| s.clear_pending());
        drop(path_cstr);

        if !ok {
            return Err(CoreError::InvalidRom(
                "retro_load_game returned false".into(),
            ));
        }
        self.rom_loaded = true;

        // Wire controller port 0 AFTER load — Mednafen-derived cores clobber
        // their input data_ptr table during MDFNI_LoadGame, so pre-load
        // configuration silently disconnects.
        // See: reference_libretro_controller_after_load_game memory.
        unsafe { (self.lib.fns.set_controller_port_device)(0, RETRO_DEVICE_JOYPAD) };

        // Snapshot real timing now that the core knows what it's running.
        let mut av = retro_system_av_info {
            geometry: retro_game_geometry {
                base_width: 0,
                base_height: 0,
                max_width: 0,
                max_height: 0,
                aspect_ratio: 0.0,
            },
            timing: retro_system_timing {
                fps: 60.0,
                sample_rate: 44_100.0,
            },
        };
        unsafe { (self.lib.fns.get_system_av_info)(&mut av) };
        self.timing = Timing {
            width: av.geometry.base_width,
            height: av.geometry.base_height,
            fps: av.timing.fps,
            sample_rate: av.timing.sample_rate.round() as u32,
        };
        state::with_state(|s| s.display_aspect = av.geometry.aspect_ratio);

        Ok(())
    }

    /// True once `load_rom` has returned Ok.
    pub fn has_rom(&self) -> bool {
        self.rom_loaded
    }

    /// Release the currently-loaded ROM via `retro_unload_game` but keep the
    /// core initialised so a subsequent `load_rom` can re-use the singleton.
    /// No-op if no ROM is loaded.
    pub fn unload_rom(&mut self) {
        if !self.rom_loaded {
            return;
        }
        unsafe { (self.lib.fns.unload_game)() };
        self.rom_loaded = false;
    }
}

// SAFETY: the underlying core is singleton and we hold the only handle for
// its lifetime; all FFI calls happen on the emu thread.
unsafe impl Send for LibretroCore {}

impl Drop for LibretroCore {
    fn drop(&mut self) {
        unsafe {
            if self.rom_loaded {
                (self.lib.fns.unload_game)();
            }
            // initialised flag is in State; check before deinit.
            let was_init = state::with_state(|s| s.initialised).unwrap_or(false);
            if was_init {
                (self.lib.fns.deinit)();
            }
        }
        // Tear down the State singleton so a subsequent load() can succeed.
        state::uninstall();
    }
}

impl Core for LibretroCore {
    fn system(&self) -> SystemId {
        self.system_id
    }

    fn timing(&self) -> Timing {
        self.timing
    }

    fn reset(&mut self) {
        if self.rom_loaded {
            unsafe { (self.lib.fns.reset)() };
        }
    }

    fn run_frame(&mut self) {
        if !self.rom_loaded {
            return;
        }
        state::with_state(|s| s.audio.clear());
        unsafe { (self.lib.fns.run)() };
    }

    fn framebuffer(&self) -> Framebuffer<'_> {
        // We hold the State Mutex briefly to capture the pixels + aspect,
        // then build a Framebuffer borrowing a slice of the pre-sized buffer.
        // The Framebuffer's lifetime is tied to &self, but the underlying
        // buffer lives in State — we return a slice that aliases through.
        //
        // Implementation: copy the slice handle out under the lock, then
        // return it. The Mutex guard's drop runs after the borrow is taken
        // but Rust's borrow checker can't see through the Mutex, so we route
        // through a static reference to the State's buffer.
        //
        // The simplest correct approach here is to memoize the last-frame
        // bytes into a buffer owned by self. We do that with a small dance.
        // (For first cut we accept a single copy per frame; a future revision
        // can swap the buffer through and skip the copy.)
        //
        // NOTE: we cheat slightly — borrow checker insists we tie the slice
        // to &self, but the bytes physically live in the State singleton.
        // Since the State's buffer is only mutated inside cb_video_refresh
        // (which fires during run_frame), and Framebuffer's lifetime forbids
        // calling run_frame between framebuffer() and the slice being used,
        // the aliasing is sound.
        let (w, h, aspect, ptr, len) = state::with_state(|s| {
            (
                s.fb_width,
                s.fb_height,
                s.display_aspect,
                s.fb_rgba.as_ptr(),
                (s.fb_width as usize)
                    .saturating_mul(s.fb_height as usize)
                    .saturating_mul(4),
            )
        })
        .unwrap_or((0, 0, 0.0, std::ptr::null(), 0));

        let pixels: &[u8] = if ptr.is_null() || len == 0 {
            &[]
        } else {
            // SAFETY: ptr lives in the State singleton's Vec<u8> which is
            // never reallocated after construction (we pre-size in State::new)
            // and never freed until LibretroCore::drop. The slice is valid for
            // the lifetime of &self.
            unsafe { std::slice::from_raw_parts(ptr, len) }
        };

        Framebuffer {
            width: w,
            height: h,
            pixels,
            display_aspect: aspect,
        }
    }

    fn drain_audio(&mut self) -> &[i16] {
        self.audio_buf.clear();
        state::with_state(|s| {
            self.audio_buf.extend_from_slice(&s.audio);
            s.audio.clear();
        });
        &self.audio_buf
    }

    fn set_input(&mut self, port: PortIndex, input: InputState) {
        let port_idx = port as usize;
        if port_idx >= 5 {
            return;
        }
        state::with_state(|s| s.input_bits[port_idx] = input.buttons as u16);
    }

    fn save_state(&self, writer: &mut dyn std::io::Write) -> Result<(), CoreError> {
        if !self.rom_loaded {
            return Err(CoreError::Internal("save_state before load_rom".into()));
        }
        let size = unsafe { (self.lib.fns.serialize_size)() };
        if size == 0 {
            return Err(CoreError::Internal(
                "core reported zero serialize size".into(),
            ));
        }
        let mut buf = vec![0u8; size];
        let ok = unsafe { (self.lib.fns.serialize)(buf.as_mut_ptr() as *mut _, buf.len()) };
        if !ok {
            return Err(CoreError::Internal("retro_serialize returned false".into()));
        }
        writer.write_all(&buf)?;
        Ok(())
    }

    fn load_state(&mut self, reader: &mut dyn std::io::Read) -> Result<(), CoreError> {
        if !self.rom_loaded {
            return Err(CoreError::Internal("load_state before load_rom".into()));
        }
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        if buf.is_empty() {
            return Err(CoreError::SaveStateMalformed);
        }
        let ok = unsafe { (self.lib.fns.unserialize)(buf.as_ptr() as *const _, buf.len()) };
        if !ok {
            return Err(CoreError::SaveStateMalformed);
        }
        Ok(())
    }

    fn options(&self) -> Vec<oa_core::CoreOption> {
        state::with_state(|s| s.core_options.clone()).unwrap_or_default()
    }

    fn option_categories(&self) -> Vec<oa_core::CoreOptionCategory> {
        state::with_state(|s| s.option_categories.clone()).unwrap_or_default()
    }

    fn set_option(&mut self, key: &str, value: &str) {
        state::with_state(|s| s.set_option_value(key, value));
    }

    fn disc_state(&self) -> Option<oa_core::DiscInfo> {
        state::with_state(|s| {
            // Prefer v2 (carries labels). Fall back to v1.
            if let Some(cb) = s.disk_v2.as_ref() {
                let num_discs = cb.get_num_images.map(|f| unsafe { f() }).unwrap_or(0);
                if num_discs == 0 {
                    return None;
                }
                let current_index = cb.get_image_index.map(|f| unsafe { f() }).unwrap_or(0);
                let ejected = cb.get_eject_state.map(|f| unsafe { f() }).unwrap_or(false);
                let labels: Vec<String> = if let Some(get_label) = cb.get_image_label {
                    (0..num_discs)
                        .map(|i| {
                            let mut buf = [0i8; 256];
                            let ok = unsafe { get_label(i, buf.as_mut_ptr() as *mut _, buf.len()) };
                            if !ok {
                                return String::new();
                            }
                            let cstr = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr() as *const _) };
                            cstr.to_string_lossy().into_owned()
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                return Some(oa_core::DiscInfo { num_discs, current_index, ejected, labels });
            }
            let cb = s.disk_v1.as_ref()?;
            let num_discs = cb.get_num_images.map(|f| unsafe { f() }).unwrap_or(0);
            if num_discs == 0 {
                return None;
            }
            let current_index = cb.get_image_index.map(|f| unsafe { f() }).unwrap_or(0);
            let ejected = cb.get_eject_state.map(|f| unsafe { f() }).unwrap_or(false);
            Some(oa_core::DiscInfo {
                num_discs,
                current_index,
                ejected,
                labels: Vec::new(),
            })
        })
        .flatten()
    }

    fn set_disc_eject(&mut self, ejected: bool) {
        state::with_state(|s| {
            let cb = s
                .disk_v2
                .as_ref()
                .and_then(|c| c.set_eject_state)
                .or_else(|| s.disk_v1.as_ref().and_then(|c| c.set_eject_state));
            if let Some(f) = cb {
                let ok = unsafe { f(ejected) };
                if !ok {
                    log::warn!("oa-libretro: set_eject_state({ejected}) returned false");
                }
            }
        });
    }

    fn set_disc_image(&mut self, index: u32) {
        state::with_state(|s| {
            let cb = s
                .disk_v2
                .as_ref()
                .and_then(|c| c.set_image_index)
                .or_else(|| s.disk_v1.as_ref().and_then(|c| c.set_image_index));
            if let Some(f) = cb {
                let ok = unsafe { f(index) };
                if !ok {
                    log::warn!("oa-libretro: set_image_index({index}) returned false");
                }
            }
        });
    }

    fn cheat_reset(&mut self) {
        if !self.rom_loaded {
            return;
        }
        unsafe { (self.lib.fns.cheat_reset)() };
    }

    fn cheat_set(&mut self, index: u32, enabled: bool, code: &str) {
        if !self.rom_loaded {
            return;
        }
        // libretro requires the code as a NUL-terminated C string. Bail
        // silently on interior NULs (no core would accept those anyway).
        let Ok(c) = std::ffi::CString::new(code) else {
            log::warn!("oa-libretro: cheat_set rejected — code contained interior NUL");
            return;
        };
        unsafe { (self.lib.fns.cheat_set)(index, enabled, c.as_ptr()) };
    }

    fn memory_region_mut(&mut self, id: MemoryRegionId) -> Option<&mut [u8]> {
        if !self.rom_loaded {
            return None;
        }
        let retro_id = id as u32;
        let size = unsafe { (self.lib.fns.get_memory_size)(retro_id) };
        if size == 0 {
            return None;
        }
        let ptr = unsafe { (self.lib.fns.get_memory_data)(retro_id) };
        if ptr.is_null() {
            return None;
        }
        // SAFETY: same guarantees as `memory_region` — libretro keeps
        // the pointer stable for `size` bytes between
        // retro_load_game/retro_unload_game, and the borrow is tied to
        // `&mut self` so the caller drops it before any other method
        // can invalidate it. Mutable access is the whole point — the
        // cheat runtime writes into this region.
        let slice = unsafe { std::slice::from_raw_parts_mut(ptr as *mut u8, size) };
        Some(slice)
    }

    fn memory_region(&self, id: MemoryRegionId) -> Option<&[u8]> {
        if !self.rom_loaded {
            return None;
        }
        let retro_id = id as u32;
        let size = unsafe { (self.lib.fns.get_memory_size)(retro_id) };
        if size == 0 {
            return None;
        }
        let ptr = unsafe { (self.lib.fns.get_memory_data)(retro_id) };
        if ptr.is_null() {
            return None;
        }
        // SAFETY: libretro guarantees the pointer is valid for `size`
        // bytes until the next retro_load_game / retro_unload_game.
        // The slice lifetime is tied to `&self`, and the core's
        // `&mut self` methods would invalidate any borrow we hand out
        // by potentially calling load_game / unload_game (which only
        // we control). The shell holds the borrow briefly to read
        // bytes out, then drops it before resuming run_frame.
        let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, size) };
        Some(slice)
    }
}
