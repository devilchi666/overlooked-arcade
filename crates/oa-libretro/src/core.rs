//! `LibretroCore` — `oa_core::Core` impl over a dynamically-loaded libretro
//! core. The shell loads a .dll/.so/.dylib once at app start (or per ROM if
//! we go multi-core), and operates on this handle as if it were a statically-
//! linked core.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;

use oa_core::{
    ControllerDeviceDescriptor, Core, CoreError, Framebuffer, InputDescriptor, InputState,
    MemoryRegionId, PortIndex, SystemId, Timing,
};

use crate::ffi::*;
use crate::loader::LibretroLibrary;
use crate::state::{self, HwRenderStatus, State};

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

/// Call a libretro disc-control getter that writes a NUL-terminated
/// C string into a caller-supplied buffer (the `get_image_label` and
/// `get_image_path` shape — both have signature
/// `fn(index, *mut c_char, usize) -> bool`). Returns the empty String
/// on either a `false` return or a malformed payload. `cap` is the
/// buffer size to allocate; 256 covers labels comfortably, 1024 covers
/// paths.
fn read_disc_string_field(
    index: u32,
    f: unsafe extern "C" fn(u32, *mut c_char, usize) -> bool,
    cap: usize,
) -> String {
    let mut buf = vec![0i8; cap];
    let ok = unsafe { f(index, buf.as_mut_ptr() as *mut c_char, buf.len()) };
    if !ok {
        return String::new();
    }
    let cstr = unsafe { CStr::from_ptr(buf.as_ptr() as *const c_char) };
    cstr.to_string_lossy().into_owned()
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

        // Reset rotation to 0 BEFORE the new core's retro_load_game runs.
        // Cores that need rotation set it via RETRO_ENVIRONMENT_SET_ROTATION
        // during load; cores that don't never call the env, so the previous
        // game's rotation would leak through without this reset.
        //
        // Same reasoning for memory descriptors: SET_MEMORY_MAPS fires from
        // inside retro_load_game on cores that publish a map. Clearing here
        // ensures a non-publishing core doesn't inherit the previous game's
        // descriptors after a back-to-back swap.
        state::with_state(|s| {
            s.rotation = 0;
            s.memory_descriptors.clear();
            s.memory_map_ptrs.clear();
        });

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
                // Synthesize a logical "<name>.<ext>" path. Genesis
                // Plus GX (and any libretro core that reads the last
                // 3 bytes of full_path to detect the system) needs a
                // string that ends with the real extension — an empty
                // full_path makes its extension-extraction memcpy
                // read out of bounds and SMS/GG/MD detection fails.
                let synthetic_path = format!("{}.{}", name, extension);
                let full_path_cstr = CString::new(synthetic_path.as_bytes())
                    .map_err(|_| CoreError::InvalidRom("synthesized path contains NUL".into()))?;
                state::with_state(|s| {
                    s.pending_rom_data = data.as_ptr();
                    s.pending_rom_size = data.len();
                    s.pending_ext = ext_cstr;
                    s.pending_name = name_cstr;
                    s.pending_full_path = full_path_cstr;
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
                // Mirror the real path into pending_full_path so any
                // core that prefers info_ext->full_path over info->path
                // gets a sensible value (same shape as the Bytes
                // branch's synthetic path).
                let full_path_cstr = CString::new(path_str.as_bytes())
                    .map_err(|_| CoreError::InvalidRom("ROM path contains NUL".into()))?;
                // Clear data path so GET_GAME_INFO_EXT returns nothing — the
                // core picks up the path from info.path and opens files itself.
                state::with_state(|s| {
                    s.pending_rom_data = std::ptr::null();
                    s.pending_rom_size = 0;
                    s.pending_ext = ext_cstr;
                    s.pending_name = name_cstr;
                    s.pending_full_path = full_path_cstr;
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
        self.finish_load();
        Ok(())
    }

    /// Boot the core without any content — the libretro spec calls
    /// this `retro_load_game(NULL)`. Only valid against cores that
    /// advertised [`supports_no_game`](Self::supports_no_game) =
    /// `true` during init; everything else returns
    /// `CoreError::InvalidRom`.
    ///
    /// Targeted use case: DOSBox-Pure boots into its own DOS-game
    /// browser when launched bootless; ScummVM boots into the engine
    /// launcher with its built-in game list. Lets the operator browse
    /// content from inside the core rather than picking a game from
    /// OA's library first.
    ///
    /// Does the same post-load work as [`load_rom`](Self::load_rom):
    /// wires controller port 0 to JOYPAD, snapshots the av_info, and
    /// pushes the aspect ratio to the renderer.
    pub fn load_no_rom(&mut self) -> Result<(), CoreError> {
        if self.rom_loaded {
            unsafe { (self.lib.fns.unload_game)() };
            self.rom_loaded = false;
        }
        // Same per-load reset as load_rom — clear rotation + memory
        // descriptors so a back-to-back swap doesn't inherit state from
        // the previous game.
        state::with_state(|s| {
            s.rotation = 0;
            s.memory_descriptors.clear();
            s.memory_map_ptrs.clear();
        });

        let ok = unsafe { (self.lib.fns.load_game)(std::ptr::null()) };
        if !ok {
            return Err(CoreError::InvalidRom(
                "retro_load_game(NULL) returned false — core may not support no-game launch"
                    .into(),
            ));
        }
        self.rom_loaded = true;
        self.finish_load();
        Ok(())
    }

    /// Post-load common work shared by `load_rom` + `load_no_rom`. Wires
    /// controller port 0 to RETRO_DEVICE_JOYPAD (Mednafen-derived cores
    /// clobber data_ptr[] during MDFNI_LoadGame so this MUST run AFTER
    /// retro_load_game) and snapshots real av_info into `self.timing`.
    fn finish_load(&mut self) {
        // Wire EVERY port (not just port 0) to RETRO_DEVICE_JOYPAD, matching
        // RetroArch's default (all ports init to JOYPAD). Must run AFTER
        // retro_load_game — Mednafen-derived cores clobber data_ptr[] during
        // load, so a port left un-wired post-load can read as disconnected,
        // which silently kills players 2-5 on multitap titles (PCE 5-player,
        // SNES/Saturn/PSX multitap). The operator's per-game device dialog can
        // still re-wire any port to a peripheral via `set_port_device`.
        for port in 0..5u32 {
            unsafe { (self.lib.fns.set_controller_port_device)(port, RETRO_DEVICE_JOYPAD) };
        }

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

        // HW-render cores (Dolphin et al.): now that the game is loaded and
        // our standalone Vulkan device + interface are ready, drive the
        // core's context_reset so it (re)creates its GPU resources. No-op
        // for software cores. See docs/PLANS/hw-render-pipeline.md (M1/D6).
        state::hw_after_load();
    }

    /// True if the core advertised `SET_SUPPORT_NO_GAME = true` during
    /// init — see [`load_no_rom`](Self::load_no_rom) for the targeted
    /// use case. Captured once during `retro_set_environment`; cores
    /// can't change their mind later.
    pub fn supports_no_game(&self) -> bool {
        state::with_state(|s| s.supports_no_game).unwrap_or(false)
    }

    /// Outcome of this core's hardware-render negotiation (Vulkan
    /// accepted / declined non-Vulkan / declined on instance error /
    /// not requested). Set once during `retro_set_environment`; the
    /// shell logs it per launch and, on a decline, can suggest the
    /// system's software-peer core. See [`HwRenderStatus`].
    pub fn hw_render_status(&self) -> HwRenderStatus {
        state::with_state(|s| s.hw_render_status).unwrap_or(HwRenderStatus::NotRequested)
    }

    /// Append an empty disc slot to the core's image list. The newly-
    /// allocated index needs to be filled with
    /// [`replace_disc_image`](Self::replace_disc_image) before it can
    /// be inserted into the tray. Supported by both v1 and v2 disc-
    /// control interfaces — `add_image_index` is part of the v1 struct.
    ///
    /// Use case: the operator forgot a disc when launching an `.m3u`
    /// playlist and wants to add it after the core is already running.
    /// Returns false if the core declined or if no disc-control
    /// interface was ever registered.
    pub fn add_disc_image(&mut self) -> bool {
        state::with_state(|s| {
            let cb = s
                .disk_v2
                .as_ref()
                .and_then(|c| c.add_image_index)
                .or_else(|| s.disk_v1.as_ref().and_then(|c| c.add_image_index));
            match cb {
                Some(f) => unsafe { f() },
                None => false,
            }
        })
        .unwrap_or(false)
    }

    /// Point disc slot `index` at the image file at `path`. Used to
    /// fill a slot just created by [`add_disc_image`](Self::add_disc_image),
    /// or to swap content for an existing index (the operator points
    /// disc 2 at a different rip of the same game). Both v1 and v2
    /// cores support `replace_image_index`.
    ///
    /// Returns false if path encoding fails (interior NUL), the core
    /// declined, or no disc-control interface was registered.
    pub fn replace_disc_image(&mut self, index: u32, path: &Path) -> bool {
        let Ok(path_cstr) = CString::new(path.to_string_lossy().as_bytes()) else {
            log::warn!("oa-libretro: replace_disc_image rejected — path contained interior NUL");
            return false;
        };
        let info = retro_game_info {
            path: path_cstr.as_ptr(),
            data: std::ptr::null(),
            size: 0,
            meta: std::ptr::null(),
        };
        state::with_state(|s| {
            let cb = s
                .disk_v2
                .as_ref()
                .and_then(|c| c.replace_image_index)
                .or_else(|| s.disk_v1.as_ref().and_then(|c| c.replace_image_index));
            match cb {
                Some(f) => unsafe { f(index, &info) },
                None => false,
            }
        })
        .unwrap_or(false)
    }

    /// Tell the core which disc to load first when retro_load_game
    /// runs — multi-disc resume support. v2 cores only (v1 cores can't
    /// resume mid-playlist).
    ///
    /// Per libretro spec this MUST run BEFORE `load_rom`: the core
    /// reads the value during its load path. Cores typically register
    /// their disc-control interface during `retro_set_environment`
    /// (which runs inside [`LibretroCore::load`]), so calling this
    /// between `load` and `load_rom` is the intended window. Cores
    /// that register late (inside retro_load_game itself) won't have
    /// the callback available yet — in that case this returns false
    /// and the operator silently starts on disc 0.
    ///
    /// `path` should match the disc image's path inside the `.m3u`
    /// playlist (the core matches by string).
    pub fn set_initial_disc_image(&mut self, index: u32, path: &Path) -> bool {
        let Ok(path_cstr) = CString::new(path.to_string_lossy().as_bytes()) else {
            log::warn!("oa-libretro: set_initial_disc_image rejected — path contained interior NUL");
            return false;
        };
        state::with_state(|s| {
            let cb = s.disk_v2.as_ref().and_then(|c| c.set_initial_image);
            match cb {
                Some(f) => unsafe { f(index, path_cstr.as_ptr()) },
                None => false,
            }
        })
        .unwrap_or(false)
    }

    /// Look up the filesystem path the core has recorded for disc
    /// `index`. v2 cores only. Returns None for v1 cores, when the
    /// core declined to fill the buffer, or when the path string was
    /// empty / malformed.
    ///
    /// This is a single-disc convenience around the bulk `paths`
    /// vector already populated by `disc_state()`; useful when the
    /// caller only needs one entry.
    pub fn disc_image_path(&self, index: u32) -> Option<String> {
        state::with_state(|s| {
            let cb = s.disk_v2.as_ref().and_then(|c| c.get_image_path)?;
            let s = read_disc_string_field(index, cb, 1024);
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        })
        .flatten()
    }

    /// True once `load_rom` has returned Ok.
    pub fn has_rom(&self) -> bool {
        self.rom_loaded
    }

    /// Display rotation reported by the core via
    /// `RETRO_ENVIRONMENT_SET_ROTATION`. Units of 90° clockwise (0..=3).
    /// Vertical arcade boards (Pac-Man, Galaxian, Donkey Kong, Galaga,
    /// Donkey Kong Jr.) set 1 or 3; non-rotated systems leave this at 0.
    /// The shell reads this after `load_rom` returns and pushes to the
    /// renderer's viewport math.
    pub fn rotation(&self) -> u32 {
        state::with_state(|s| s.rotation).unwrap_or(0)
    }

    /// Re-wire a controller port to the given libretro device type. Used to
    /// switch a port between `RETRO_DEVICE_JOYPAD` (default arcade / console
    /// behavior) and `RETRO_DEVICE_KEYBOARD` (for MAME's operator inputs,
    /// MSX, future computer-shaped systems). Must run AFTER `retro_load_game`
    /// — Mednafen-derived cores clobber `data_ptr[]` during load, so a
    /// pre-load wiring silently disconnects. See
    /// `reference_libretro_controller_after_load_game`.
    pub fn set_port_device(&mut self, port: u32, device: u32) {
        if !self.rom_loaded {
            log::warn!(
                "oa-libretro: set_port_device({port}, {device}) called before load_rom — ignored"
            );
            return;
        }
        unsafe { (self.lib.fns.set_controller_port_device)(port, device) };
    }

    /// Supported-device list this core advertised for `port` via the
    /// libretro `SET_CONTROLLER_INFO` env call (parsed in
    /// `state.rs::parse_controller_info`). Empty Vec means either:
    ///
    /// - the core never called `SET_CONTROLLER_INFO` (rare in modern
    ///   cores; falls back to "Standard Pad + Disconnected" in the
    ///   frontend);
    /// - the core advertised fewer ports than `port + 1`;
    /// - the core explicitly declared the port supports nothing.
    ///
    /// Frontend uses this to render the per-game Input dialog's device
    /// dropdown — each entry's `id` is dispatched verbatim via
    /// `set_port_device` so cores that use subclass-encoded peripheral
    /// ids (FCEUmm Zapper = 258, snes9x Super Scope = 260, …) work
    /// without per-system hardcoded tables in the frontend.
    pub fn controller_devices(&self, port: u32) -> Vec<ControllerDeviceDescriptor> {
        state::with_state(|s| {
            s.controller_devices
                .get(port as usize)
                .cloned()
                .unwrap_or_default()
        })
        .unwrap_or_default()
    }

    /// Per-game button-label list this core advertised via
    /// `RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS`. Empty when the core
    /// never published or no game is loaded. Frontend uses this to
    /// render the in-game semantic label ("Whip", "Jump") next to
    /// the operator-chosen physical mapping in the bindings UI.
    /// Updated on every env-11 call; the latest publish wins (cores
    /// may re-publish across a game's lifetime when modes change).
    pub fn input_descriptors(&self) -> Vec<InputDescriptor> {
        state::with_state(|s| s.input_descriptors.clone()).unwrap_or_default()
    }

    /// Minimum audio latency (ms) this core requested via
    /// `RETRO_ENVIRONMENT_SET_MINIMUM_AUDIO_LATENCY` (env 63). Returns
    /// 0 when the core didn't call the env (spec: 0 means "use default
    /// frontend latency"). Capped at 512 ms in the parser per the spec
    /// ceiling against buggy cores. Frontend (oa-shell) calls this
    /// post-load and forwards to `oa_audio::AudioSink::ensure_min
    /// _latency_ms` so the ring buffer grows to cover any declared
    /// headroom — prevents crackling on CPU-heavy frames in cores like
    /// Genesis Plus GX / Beetle PSX / Flycast that declare 64-100 ms.
    pub fn min_audio_latency_ms(&self) -> u32 {
        state::with_state(|s| s.min_audio_latency_ms).unwrap_or(0)
    }

    /// Forward a keyboard transition to the core via the callback the core
    /// registered through `RETRO_ENVIRONMENT_SET_KEYBOARD_CALLBACK`. No-op
    /// when the core never registered (the typical console / handheld case).
    ///
    /// - `down`: `true` on key press, `false` on key release.
    /// - `keycode`: libretro `retro_key` value (NOT an OS scancode). See the
    ///   `RETROK_*` constants in [`crate::ffi`]; translate from
    ///   `device_query::Keycode` via [`crate::keycode_to_retro_key`].
    /// - `character`: unicode codepoint for printable transitions, `0`
    ///   otherwise. Most cores ignore this and key off `keycode` alone.
    /// - `modifiers`: bitmask of currently-held modifiers (combine the
    ///   `RETROKMOD_*` constants).
    pub fn send_keyboard_event(
        &mut self,
        down: bool,
        keycode: u32,
        character: u32,
        modifiers: u16,
    ) {
        // Copy the function pointer out under the mutex, then release the
        // lock before calling the core. The core may call back into our env
        // handler from inside the keyboard callback (e.g. to log via
        // GET_LOG_INTERFACE); holding the State mutex across the call would
        // deadlock that path.
        let cb = state::with_state(|s| s.keyboard_cb).flatten();
        if let Some(f) = cb {
            unsafe { f(down, keycode, character, modifiers) };
        }
    }

    /// True if the active core has registered a keyboard-event callback.
    /// Used by the shell to short-circuit the keyboard-event pump when the
    /// core has declined keyboard input — pumping events to a core that
    /// hasn't registered wastes work and the callback would be no-op anyway.
    pub fn has_keyboard_callback(&self) -> bool {
        state::with_state(|s| s.keyboard_cb.is_some()).unwrap_or(false)
    }

    /// Phase F — snapshot the rumble strength the core has requested
    /// per (port × effect). `[port][0]` = strong/low-freq motor,
    /// `[port][1]` = weak/high-freq motor. Range 0..=65535. The shell
    /// reads this once per frame and forwards to
    /// `oa-input::InputPoller::dispatch_rumble` which pokes gilrs's
    /// force-feedback API. Returns all-zeros for cores that don't use
    /// the rumble interface.
    pub fn rumble_snapshot(&self) -> [[u16; 2]; 5] {
        state::with_state(|s| s.rumble).unwrap_or([[0; 2]; 5])
    }

    /// Phase G — push per-port-per-axis sensor values (accelerometer,
    /// gyroscope, illuminance) so the core's next
    /// `cb_get_sensor_input` poll reads fresh data. The shell's input
    /// poll loop pumps these every frame; today the values come from
    /// keyboard arrow-keys-as-tilt so GBA Boktai / Kirby Tilt 'n'
    /// Tumble / WarioWare Twisted! are playable without OS-level
    /// accelerometer access.
    pub fn set_sensor_values(&self, values: [[f32; 7]; 5]) {
        state::with_state(|s| s.sensor_values = values);
    }

    /// Phase G — true if the core has enabled at least one sensor on
    /// any port via `cb_set_sensor_state`. The shell skips pushing
    /// sensor values when nothing's enabled (saves a per-frame
    /// keyboard poll for the 95% of cores that don't use sensors).
    pub fn sensors_enabled(&self) -> bool {
        state::with_state(|s| {
            s.sensor_enabled
                .iter()
                .any(|p| p.iter().any(|&e| e))
        })
        .unwrap_or(false)
    }

    /// Push polled keyboard held-state for `RETRO_DEVICE_KEYBOARD` cores
    /// (MAME, blueMSX/fMSX, DOSBox-Pure, Atari 5200, Odyssey²). These cores
    /// read held keys every frame via `cb_input_state(.., RETRO_DEVICE_
    /// KEYBOARD, 0, retro_key)` and do NOT register the keyboard-event
    /// callback, so the event path (`send_keyboard_event`) never reaches
    /// them — this is the parallel state path. `keys` is indexed by
    /// `retro_key` id; the shell builds it from the host's currently-held
    /// keys (via `keycode_to_retro_key`) for keyboard-passthrough systems.
    /// Indices ≥ the array length are ignored.
    pub fn set_keyboard_state(&self, keys: &[(u32, bool)]) {
        state::with_state(|s| {
            // Clear then set — the shell pushes the full held set each frame.
            for k in s.keyboard_state.iter_mut() {
                *k = false;
            }
            for &(id, held) in keys {
                let idx = id as usize;
                if idx < s.keyboard_state.len() {
                    s.keyboard_state[idx] = held;
                }
            }
        });
    }

    /// Push per-port mouse state for `RETRO_DEVICE_MOUSE` cores (arcade
    /// trackball / spinner / dial games, SNES/Saturn/PSX Mouse). `dx`/`dy`
    /// are per-poll RELATIVE deltas; `buttons` is a bitmask whose bit
    /// position matches the `RETRO_DEVICE_ID_MOUSE_*` button id (bit 2 =
    /// LEFT, 3 = RIGHT, 6 = MIDDLE). The shell computes deltas frame-over-
    /// frame from the host cursor and resets them after each poll.
    pub fn set_mouse_state(&self, port: usize, dx: i16, dy: i16, buttons: u32) {
        if port >= 5 {
            return;
        }
        state::with_state(|s| s.input_mouse[port] = (dx, dy, buttons));
    }

    /// Push the frontend's current throttle mode + target fps for
    /// `GET_THROTTLE_STATE` (env 71). `mode` is a `RETRO_THROTTLE_*` constant.
    pub fn set_throttle_state(&self, mode: u32, rate: f32) {
        state::with_state(|s| s.throttle_state = (mode, rate));
    }

    /// Set the serialization context reported via `GET_SAVESTATE_CONTEXT`
    /// (env 72). The shell sets `RUNAHEAD_SAME_INSTANCE` around the per-frame
    /// run-ahead save/restore and `NORMAL` otherwise so cores can drop
    /// reconstructable data for transient snapshots.
    pub fn set_savestate_context(&self, context: i32) {
        state::with_state(|s| s.savestate_context = context);
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
        }
        // HW-render teardown BEFORE deinit + uninstall: calls the core's
        // context_destroy, then drops our standalone Vulkan device (waits
        // idle + destroys all Vk objects). No-op for software cores.
        state::hw_teardown();
        unsafe {
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
        // HW-render cores produce no CPU framebuffer — read the core's
        // rendered VkImage back into fb_rgba so the existing present path
        // shows it. No-op for software cores. See M1/D6.
        state::hw_after_run();
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
        state::with_state(|s| {
            s.input_bits[port_idx] = input.buttons as u16;
            // Analog axes flow through the same set_input call. Cores
            // that don't poll RETRO_DEVICE_ANALOG (most pre-N64 systems)
            // simply ignore the stored values; cores that do (N64,
            // GameCube, DualShock, Saturn 3D Pad, etc.) get the i16
            // values back via cb_input_state's RETRO_DEVICE_ANALOG arm.
            s.input_axes[port_idx] = input.axes;
            // Pointer device state flows through the same path. Cores
            // that don't poll RETRO_DEVICE_POINTER (most systems) ignore
            // the stored values; NDS / light-gun games / DC HoTD get the
            // (x, y, pressed) tuple back via cb_input_state's
            // RETRO_DEVICE_POINTER arm. `pointer_secondary` is the
            // multi-touch slot: cores polling POINTER with `index = 1`
            // read it; defaults to `(0, 0, false, false)` until a
            // second-finger input source is wired.
            s.input_pointer[port_idx] = input.pointer;
            s.input_pointer_secondary[port_idx] = input.pointer_secondary;
            // LIGHTGUN gun-side button bitmask. Cores polling
            // RETRO_DEVICE_LIGHTGUN with AUX_A/B/C, START, SELECT,
            // DPAD_{UP,DOWN,LEFT,RIGHT}, or RELOAD read the
            // matching bit from this u32 via lightgun_field_value.
            // TRIGGER + SCREEN_X/Y + IS_OFFSCREEN keep their existing
            // sources (pointer.pressed + pointer coords + in_viewport).
            s.input_lightgun_buttons[port_idx] = input.lightgun_buttons;
            // Per-button analog pressure flows through the same path.
            // Cores that don't poll RETRO_DEVICE_INDEX_ANALOG_BUTTON
            // (most systems) ignore the stored values; PSX/Saturn/GC/DC
            // pressure-sensitive trigger or face-button reads get the
            // per-button i16 values back via cb_input_state's
            // ANALOG_BUTTON arm.
            s.input_analog_buttons[port_idx] = input.analog_buttons;
        });
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

    fn hidden_option_keys(&self) -> Vec<String> {
        state::with_state(|s| s.hidden_options.iter().cloned().collect())
            .unwrap_or_default()
    }

    fn refresh_option_visibility(&mut self) {
        // Lift the callback pointer out of State so we don't hold the
        // singleton mutex while the core re-enters cb_environment via
        // SET_CORE_OPTIONS_DISPLAY (which would deadlock).
        let cb = state::with_state(|s| s.update_display_cb).flatten();
        if let Some(f) = cb {
            unsafe { f() };
        }
    }

    fn disc_state(&self) -> Option<oa_core::DiscInfo> {
        state::with_state(|s| {
            // Prefer v2 (carries labels + paths). Fall back to v1.
            if let Some(cb) = s.disk_v2.as_ref() {
                let num_discs = cb.get_num_images.map(|f| unsafe { f() }).unwrap_or(0);
                if num_discs == 0 {
                    return None;
                }
                let current_index = cb.get_image_index.map(|f| unsafe { f() }).unwrap_or(0);
                let ejected = cb.get_eject_state.map(|f| unsafe { f() }).unwrap_or(false);
                let labels: Vec<String> = if let Some(get_label) = cb.get_image_label {
                    (0..num_discs)
                        .map(|i| read_disc_string_field(i, get_label, 256))
                        .collect()
                } else {
                    Vec::new()
                };
                // get_image_path — v2 extra. 1024 cap matches PATH_MAX
                // on common platforms; cores typically write short
                // relative paths (the m3u entry).
                let paths: Vec<String> = if let Some(get_path) = cb.get_image_path {
                    (0..num_discs)
                        .map(|i| read_disc_string_field(i, get_path, 1024))
                        .collect()
                } else {
                    Vec::new()
                };
                return Some(oa_core::DiscInfo {
                    num_discs,
                    current_index,
                    ejected,
                    labels,
                    paths,
                });
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
                paths: Vec::new(),
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

    fn memory_map(&self) -> Vec<oa_core::MemoryDescriptor> {
        state::with_state(|s| s.memory_descriptors.clone()).unwrap_or_default()
    }

    fn drain_messages(&mut self) -> Vec<oa_core::CoreMessage> {
        state::with_state(|s| std::mem::take(&mut s.pending_messages)).unwrap_or_default()
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
