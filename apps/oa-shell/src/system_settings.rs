// Per-system settings persistence.
//
// Phase 2.8 slice C introduces the per-system settings page reached via the
// system page header `⚙` button. Each system gets its own JSON file under
// `appDataDir/systems/<system_id>.json` holding overrides that, when set,
// take precedence over the OA-wide preferences. Per-game overrides (slice D)
// will sit on top of these.
//
// Today's surface: scaling / window / monitor overrides. The per-system
// core override stays in `cores.json` (its own pre-existing store); the
// frontend bridges both stores transparently in the UI. Future overrides
// (audio device, region priority, shader preset) land as additional Option
// fields on `SystemSettings` — old files still parse because every field is
// already Option-wrapped and `#[serde(default)]`.
//
// Same file pattern as layout.rs / shell.rs / cores.rs: tolerant of missing
// or malformed files (returns defaults), pretty-print on write, one file per
// logical settings surface.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SystemSettings {
    /// One of the ScalingMode strings from settings/store.ts. None = inherit
    /// the OA-wide default. Stored as a String here (not an enum) so we
    /// don't have to keep the Rust enum in sync with the frontend's union
    /// type — the frontend is the source of truth for the valid set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scaling_override: Option<String>,
    /// One of "windowed" | "borderless" | "fullscreen". None = inherit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_mode_override: Option<String>,
    /// 0-based monitor index. None = inherit (== "Current monitor" in OA UI).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitor_index_override: Option<i32>,
    /// Phase 3 slice A — per-system shader preset name (looked up against
    /// the TOML registry; slice C). None = inherit the OA-wide default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shader_preset: Option<String>,
    /// Phase 3 slice C polish — per-system override for the Phosphor
    /// composite weight. None = use whatever the active preset's TOML
    /// `[params].bloom_amount` field carries (or whichever value the
    /// renderer currently holds if the preset doesn't specify). The
    /// override applies at launch time AFTER `set_shader_preset` so the
    /// override always wins over the TOML's default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bloom_amount: Option<f32>,
    /// Phase 4 slice A — per-system rewind enabled toggle. None = inherit
    /// the OA-wide default. Disabled by default since rewind has a non-zero
    /// RAM + CPU cost per frame.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rewind_enabled: Option<bool>,
    /// Phase 4 slice A — frames between snapshots. None = inherit.
    /// 1 = capture every frame; 6 = ~100 ms at 60 fps (default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rewind_capture_interval_frames: Option<u32>,
    /// Phase 4 slice A — rewind ring memory cap in MB. None = inherit.
    /// Cores with large states (SNES ~300 KB, Saturn ~3 MB) hold fewer
    /// seconds of history per MB; the UI surfaces seconds_held live.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rewind_buffer_megabytes: Option<u32>,
    /// Phase 6 Cross-system slice 2 — forward raw keyboard transitions
    /// to the core via `retro_set_keyboard_callback`. None = use the
    /// system's compiled-in default ([`default_keyboard_passthrough`]),
    /// which is `true` for computer-shaped systems (MAME, MSX/MSX2)
    /// where keyboard input is meaningful and `false` for everything
    /// else where pumping keys would cost work for no benefit.
    /// Use [`effective_keyboard_passthrough`] to resolve the override
    /// against the default in one call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyboard_passthrough: Option<bool>,
    /// Per-system override for the OA-wide
    /// [`crate::library_prefs::LibraryPrefs::region_priority`]. None =
    /// inherit. Useful for systems whose library skews region-different
    /// from the user's overall pref (e.g. a Japan-first PCE library
    /// inside an otherwise USA-first collection).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_priority_override: Option<Vec<String>>,
    /// Per-system override for
    /// [`crate::library_prefs::LibraryPrefs::revision_priority`]. None =
    /// inherit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision_priority_override: Option<crate::library_prefs::RevisionPriority>,
    /// Per-system override for the framebuffer's reported
    /// display_aspect. `None` = trust whichever value the libretro core
    /// reports via `retro_get_system_av_info` (the typical case —
    /// cores like Beetle PCE Fast correctly report PCE's non-square
    /// pixel aspect). `Some(x)` substitutes `x` at viewport-math time
    /// — set this when the core doesn't report aspect, or when the
    /// user prefers a different aspect (e.g. 4:3 fits-the-TV instead
    /// of the technically-correct PCE 256-mode ratio).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_aspect_override: Option<f32>,
    /// Per-edge overscan crop applied to the source framebuffer before
    /// the blit. Values are in source-framebuffer pixels. `None` = no
    /// crop. Crops larger than half the framebuffer clamp to 1 visible
    /// pixel per axis (defensive — an out-of-range setting can't blank
    /// the screen). Pairs with `display_aspect_override` for "make this
    /// system look right" tuning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overscan_crop_override: Option<OverscanCropPrefs>,
    /// Absolute path to a bezel image (PNG / JPEG / WebP) overlaid on
    /// top of the game pixels. `None` = use whatever the active shader
    /// preset's TOML `[bezel]` block carries (which may itself be none).
    /// Wins over the preset's default; cleared by setting to `None` +
    /// invoking `clear_bezel_image_override`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bezel_image_path: Option<String>,
    /// Phase 2.5 — per-system analog routing (deadzone / sensitivity /
    /// invert / keyboard-axis fallback / stick-swap). `None` = use the
    /// system's default identity routing (no tuning, no keyboard). Per-
    /// game overrides (`GameOverrides::analog_routing`) stack on top.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analog_routing: Option<AnalogRoutingPrefs>,
}

/// JSON-friendly mirror of `oa_input::AnalogRouting`. Lives here so the
/// settings file format isn't coupled to the input crate's runtime types
/// (`Keycode` etc.). The shell converts on push to InputPoller per port.
///
/// Per-port shape (Phase 2.5 — multi-controller analog routing):
/// `ports[i]` is the routing for port i (0..=4). Missing trailing
/// entries fall back to identity routing — operators only configure
/// the ports they actually use.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AnalogRoutingPrefs {
    /// Per-port routings. Index 0 = port 0. Length 0..=5. Missing
    /// trailing entries use identity routing at runtime.
    #[serde(default)]
    pub ports: Vec<AnalogPortRouting>,
}

/// Per-port analog routing — one of these per controller port.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AnalogPortRouting {
    pub left: AnalogStickPrefs,
    pub right: AnalogStickPrefs,
    pub stick_swap: bool,
}

impl AnalogPortRouting {
    /// Identity port routing — left=Left, right=Right, no swap.
    pub fn identity() -> Self {
        Self {
            left: AnalogStickPrefs::default(),
            right: AnalogStickPrefs::default_right(),
            stick_swap: false,
        }
    }

    pub fn to_runtime(&self) -> oa_input::AnalogRouting {
        oa_input::AnalogRouting {
            left: self.left.to_runtime(),
            right: self.right.to_runtime(),
            stick_swap: self.stick_swap,
        }
    }
}

/// JSON-friendly mirror of `oa_input::AnalogStickRouting`. Keys are
/// stored as Rust Keycode debug names (e.g. "Numpad8") — same shape as
/// `bindings.rs`'s keyboard entries — so the existing keymap converter
/// in the frontend (`systems/keymap.ts`) round-trips them cleanly.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnalogStickPrefs {
    /// "left" | "right" | "none"
    #[serde(default = "default_left_source")]
    pub gamepad_source: String,
    /// [up, down, left, right] keyboard bindings. Each `None` = unbound.
    #[serde(default)]
    pub keyboard: [Option<String>; 4],
    #[serde(default)]
    pub deadzone: f32,
    #[serde(default = "default_sensitivity")]
    pub sensitivity: f32,
    #[serde(default)]
    pub invert_x: bool,
    #[serde(default)]
    pub invert_y: bool,
}

fn default_left_source() -> String { "left".to_string() }
fn default_sensitivity() -> f32 { 1.0 }

impl Default for AnalogStickPrefs {
    fn default() -> Self {
        Self {
            gamepad_source: "left".into(),
            keyboard: [None, None, None, None],
            deadzone: 0.0,
            sensitivity: 1.0,
            invert_x: false,
            invert_y: false,
        }
    }
}

impl AnalogStickPrefs {
    pub fn default_right() -> Self {
        Self {
            gamepad_source: "right".into(),
            ..Self::default()
        }
    }

    /// Convert to runtime `oa_input::AnalogStickRouting`. Resolves the
    /// keyboard string slots via `crate::bindings::keycode_from_name` —
    /// unmapped or malformed names quietly drop to `None`.
    pub fn to_runtime(&self) -> oa_input::AnalogStickRouting {
        let keyboard = [
            self.keyboard[0].as_deref().and_then(crate::bindings::keycode_from_name),
            self.keyboard[1].as_deref().and_then(crate::bindings::keycode_from_name),
            self.keyboard[2].as_deref().and_then(crate::bindings::keycode_from_name),
            self.keyboard[3].as_deref().and_then(crate::bindings::keycode_from_name),
        ];
        oa_input::AnalogStickRouting {
            gamepad_source: oa_input::GamepadStick::parse(&self.gamepad_source),
            keyboard,
            deadzone: self.deadzone.clamp(0.0, 0.5),
            sensitivity: self.sensitivity.clamp(0.1, 4.0),
            invert_x: self.invert_x,
            invert_y: self.invert_y,
        }
    }
}

impl AnalogRoutingPrefs {
    /// Identity routing for every port. Used when persistence is
    /// absent / unset. Stored as an empty `ports` vec rather than
    /// 5 explicit default entries to keep the serialized form small.
    pub fn identity() -> Self {
        Self { ports: Vec::new() }
    }

    /// Resolve the routing for a specific port. Returns identity for
    /// ports beyond `ports.len()` or for any port whose entry is the
    /// struct's default.
    pub fn port_routing(&self, port: u32) -> AnalogPortRouting {
        let idx = port as usize;
        self.ports
            .get(idx)
            .cloned()
            .unwrap_or_else(AnalogPortRouting::identity)
    }

    /// Set the routing for a specific port. Extends `ports` if needed.
    /// Trims trailing identity entries so the serialized form stays
    /// minimal (5 ports with only port 2 configured stores 3 entries:
    /// identity, identity, port2 — but not the trailing 2 identities).
    pub fn set_port_routing(&mut self, port: u32, routing: AnalogPortRouting) {
        let idx = port as usize;
        if idx >= 5 {
            return;
        }
        // Pad with identity entries up to idx if needed.
        while self.ports.len() <= idx {
            self.ports.push(AnalogPortRouting::identity());
        }
        self.ports[idx] = routing;
        // Trim trailing identity entries to keep serialized form small.
        while let Some(last) = self.ports.last() {
            if *last == AnalogPortRouting::identity() {
                self.ports.pop();
            } else {
                break;
            }
        }
    }
}

/// Serialisable mirror of `oa_render::OverscanCrop` — broken out so the
/// settings file format isn't coupled to the renderer's type. All four
/// edges default to 0 so loading an old file with only some fields
/// present works.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct OverscanCropPrefs {
    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
}

impl OverscanCropPrefs {
    /// Convert to the renderer's [`oa_render::OverscanCrop`]. Today
    /// the launch path passes the four edges separately to the Tauri
    /// command so this is unused; kept for any future caller that
    /// wants to apply per-system overscan from Rust directly.
    #[allow(dead_code)]
    pub fn to_render(self) -> oa_render::OverscanCrop {
        oa_render::OverscanCrop {
            top: self.top,
            bottom: self.bottom,
            left: self.left,
            right: self.right,
        }
    }
    /// True when every edge is zero. Frontend cleans up zero-valued
    /// overscan-crop fields before persisting, so this is currently
    /// unused on the Rust side but mirrors the frontend's helper.
    #[allow(dead_code)]
    pub fn is_zero(self) -> bool {
        self.top == 0 && self.bottom == 0 && self.left == 0 && self.right == 0
    }
}

/// Compiled-in default for [`SystemSettings::keyboard_passthrough`] per
/// system. `true` for systems with meaningful keyboard input (MAME for
/// operator / TAB menu / per-driver keys; MSX + MSX2 for BASIC and
/// keyboard-driven games). `false` everywhere else — sending keyboard
/// events to a console core that never registered a keyboard callback
/// is harmless but wasted work, and could surprise debugging if a core
/// later adds an unsolicited keyboard listener.
pub fn default_keyboard_passthrough(system_id: &str) -> bool {
    matches!(
        system_id,
        "mame" | "msx" | "msx2"
        // Atari 5200 — the 12-key keypad (0-9, *, #) on the controller
        // doesn't fit RetroPad bits. Operators reach the keypad by
        // pressing the numeric/symbol keys directly; Atari800 libretro
        // wires them via the KEYBOARD device. Required for Missile
        // Command's screen-coord shooting, RealSports Football's play
        // selection, and a long tail of keypad-using titles.
        | "5200"
    )
}

/// Resolve a `SystemSettings` override against the system's compiled-in
/// default. Returns the effective passthrough value the emu thread should
/// use for this system + settings combination.
pub fn effective_keyboard_passthrough(system_id: &str, settings: &SystemSettings) -> bool {
    settings
        .keyboard_passthrough
        .unwrap_or_else(|| default_keyboard_passthrough(system_id))
}

/// Compiled-in default display aspect ratio per system, used when the
/// resolution chain (per-game override → per-system user override → core
/// reported value) doesn't already supply one. Returns `Some(x)` only
/// for systems whose libretro core's reported aspect disagrees with the
/// physical hardware — `None` everywhere else means "trust the core".
///
/// Today this is GBA-only: the Game Boy Advance's panel is 240×160 =
/// 3:2 (1.5), but mGBA reports 4:3 by libretro convention which
/// letterboxes the actual 3:2 panel. Set the default to 1.5 so a
/// fresh-install GBA library renders authentically without the user
/// having to discover the per-system aspect setting.
///
/// **New core onboarding checklist item:** if the core's reported
/// aspect via `retro_get_system_av_info` is canonically wrong for the
/// hardware (rare — most cores get this right), add an arm here.
pub fn default_display_aspect(system_id: &str) -> Option<f32> {
    match system_id {
        // GBA — 240×160 panel = 3:2 (1.5). mGBA reports 4:3 per the
        // libretro convention which adds a black bar above and below
        // the actual screen content. The hardware was 3:2; that's the
        // authentic default.
        "gba" => Some(1.5),
        _ => None,
    }
}

fn system_settings_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("systems")
}

fn system_settings_path(app_data_dir: &Path, system_id: &str) -> PathBuf {
    // System ids in the registry are short ASCII slugs (tg16, lynx, …). If
    // a future id ever contains path separators we'd want to sanitize; for
    // now the registry guarantees safe filenames.
    system_settings_dir(app_data_dir).join(format!("{system_id}.json"))
}

pub fn read_system_settings(app_data_dir: &Path, system_id: &str) -> SystemSettings {
    let path = system_settings_path(app_data_dir, system_id);
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return SystemSettings::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn write_system_settings(
    app_data_dir: &Path,
    system_id: &str,
    settings: &SystemSettings,
) -> std::io::Result<()> {
    let dir = system_settings_dir(app_data_dir);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{system_id}.json"));
    let body = serde_json::to_string_pretty(settings)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_all_none() {
        let d = SystemSettings::default();
        assert!(d.scaling_override.is_none());
        assert!(d.window_mode_override.is_none());
        assert!(d.monitor_index_override.is_none());
        assert!(d.shader_preset.is_none());
        assert!(d.bloom_amount.is_none());
        assert!(d.rewind_enabled.is_none());
        assert!(d.rewind_capture_interval_frames.is_none());
        assert!(d.rewind_buffer_megabytes.is_none());
        assert!(d.keyboard_passthrough.is_none());
        assert!(d.region_priority_override.is_none());
        assert!(d.revision_priority_override.is_none());
        assert!(d.display_aspect_override.is_none());
        assert!(d.overscan_crop_override.is_none());
        assert!(d.bezel_image_path.is_none());
    }

    #[test]
    fn keyboard_passthrough_default_per_system() {
        // Computer-shaped systems default to passthrough ON.
        assert!(default_keyboard_passthrough("mame"));
        assert!(default_keyboard_passthrough("msx"));
        assert!(default_keyboard_passthrough("msx2"));
        // Console / handheld systems default OFF.
        for sys in &["tg16", "pce-cd", "lynx", "nes", "snes", "sms", "gamegear", "atari7800"] {
            assert!(!default_keyboard_passthrough(sys), "{sys}: expected off");
        }
    }

    #[test]
    fn default_display_aspect_gba_is_three_halves() {
        // GBA panel is 240x160 = 3:2 (1.5). mGBA reports 4:3 by libretro
        // convention; the default overrides so the panel renders at its
        // physical aspect out of the box.
        assert_eq!(default_display_aspect("gba"), Some(1.5));
    }

    #[test]
    fn default_display_aspect_returns_none_for_systems_with_correct_core_reporting() {
        // Systems whose libretro core reports physical aspect correctly
        // (the typical case) → None = trust the core.
        for sys in &[
            "tg16", "pce-cd", "nes", "snes", "n64", "psx", "saturn",
            "dreamcast", "gb", "lynx", "atari7800", "2600", "5200",
            "vectrex", "virtualboy", "wonderswan", "mame",
        ] {
            assert_eq!(
                default_display_aspect(sys),
                None,
                "{sys}: expected None (core reports correctly), got {:?}",
                default_display_aspect(sys)
            );
        }
        assert_eq!(default_display_aspect("not-a-real-system"), None);
    }

    #[test]
    fn effective_keyboard_passthrough_uses_override_when_set() {
        let mut s = SystemSettings::default();
        // Unset → falls back to compiled-in default.
        assert!(effective_keyboard_passthrough("mame", &s));
        assert!(!effective_keyboard_passthrough("nes", &s));
        // Explicit override beats the default in either direction.
        s.keyboard_passthrough = Some(false);
        assert!(!effective_keyboard_passthrough("mame", &s));
        s.keyboard_passthrough = Some(true);
        assert!(effective_keyboard_passthrough("nes", &s));
    }

    #[test]
    fn missing_file_returns_default() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-syssettings-missing-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let s = read_system_settings(&tmp, "tg16");
        assert_eq!(s, SystemSettings::default());
    }

    #[test]
    fn round_trip_through_disk() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-syssettings-roundtrip-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let prefs = SystemSettings {
            scaling_override: Some("pixel-perfect".to_string()),
            window_mode_override: None,
            monitor_index_override: Some(1),
            shader_preset: Some("scanlines".to_string()),
            bloom_amount: Some(0.42),
            rewind_enabled: Some(true),
            rewind_capture_interval_frames: Some(6),
            rewind_buffer_megabytes: Some(32),
            keyboard_passthrough: Some(true),
            region_priority_override: Some(vec!["Japan".to_string(), "USA".to_string()]),
            revision_priority_override: Some(crate::library_prefs::RevisionPriority::Oldest),
            display_aspect_override: Some(1.333),
            overscan_crop_override: Some(OverscanCropPrefs { top: 8, bottom: 8, left: 0, right: 0 }),
            bezel_image_path: Some("C:/bezels/arcade.png".to_string()),
            analog_routing: Some(AnalogRoutingPrefs {
                ports: vec![AnalogPortRouting {
                    left: AnalogStickPrefs { deadzone: 0.1, ..AnalogStickPrefs::default() },
                    right: AnalogStickPrefs::default_right(),
                    stick_swap: false,
                }],
            }),
        };
        write_system_settings(&tmp, "tg16", &prefs).expect("write");
        let read = read_system_settings(&tmp, "tg16");
        assert_eq!(read, prefs);
        // Untouched system uses defaults.
        let untouched = read_system_settings(&tmp, "lynx");
        assert_eq!(untouched, SystemSettings::default());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn malformed_json_falls_back_to_default() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-syssettings-malformed-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("systems")).expect("mkdir");
        std::fs::write(tmp.join("systems").join("tg16.json"), "{not json").expect("write");
        assert_eq!(read_system_settings(&tmp, "tg16"), SystemSettings::default());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
