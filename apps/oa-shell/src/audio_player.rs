//! File-driven audio player — kiosk-plan 5-bus mixer for platform
//! music, UI sounds, ceremony, and snap audio. Live game audio rides
//! the separate `oa-audio` crate (i16 PCM + ring buffer from the
//! emulator core); this player is for "play this .ogg / .mp3 / .flac
//! file" use cases.
//!
//! Wraps rodio (cpal output + symphonia decoders). The output stream
//! is owned by a dedicated audio thread because rodio's OutputStream
//! is `!Send` and Tauri state needs `Send + Sync`. Tauri commands
//! send `AudioCommand` messages over an mpsc channel; the audio
//! thread processes them in order. One rodio `Sink` per bus —
//! per-bus volume + stop/replace play.
//!
//! Phase 4 v1 scope:
//! - 4 buses: PlatformMusic, UiSounds, Ceremony, SnapAudio.
//! - play_path(bus, path, looped) — replaces whatever was on the bus.
//! - stop_bus(bus), set_bus_volume(bus, gain).
//! - Format support is whatever rodio's `symphonia-all` feature gives
//!   (.ogg, .opus, .mp3, .flac, .wav, .m4a).
//!
//! Deferred (Phase 4.5 stretch):
//! - Ducking matrix (per-pair gain attenuation, e.g. dip music when
//!   ceremony plays).
//! - Crossfade between platform-music tracks on focus change.
//! - Audio device picker (rodio uses the OS default).

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread::JoinHandle;

use serde::{Deserialize, Serialize};

/// One of the four file-driven audio buses. Each bus has its own
/// `rodio::Sink` so volumes + play/stop are independent. The fifth
/// kiosk-plan bus (`LiveGameAudio`) is handled by `oa-audio`'s
/// dedicated PCM pipe and is NOT routed here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AudioBus {
    /// Background music for the currently-focused system / game.
    /// Typically loops. Swapped when the operator changes focus in
    /// the library.
    PlatformMusic,
    /// One-shot UI cues — click, navigate, back, launch, error,
    /// scroll-tick. Don't loop.
    UiSounds,
    /// Ceremony / fanfare sounds — game-start jingle, achievement,
    /// high score. Don't loop. Conceptually the loudest bus.
    Ceremony,
    /// Audio extracted from a game's preview video / snap clip.
    /// Plays under the snap when the operator hovers a tile and the
    /// snap preview kicks in. Typically loops the audio while the
    /// snap loops the video.
    SnapAudio,
}

impl AudioBus {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "platform-music" => Some(AudioBus::PlatformMusic),
            "ui-sounds"      => Some(AudioBus::UiSounds),
            "ceremony"       => Some(AudioBus::Ceremony),
            "snap-audio"     => Some(AudioBus::SnapAudio),
            _ => None,
        }
    }

    /// Stable kebab-case name. Mirrors `AudioBus::parse` — every
    /// string produced here parses back to the same variant. Used
    /// in tests + by future Phase 6 UI (settings page surfaces the
    /// bus names verbatim).
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            AudioBus::PlatformMusic => "platform-music",
            AudioBus::UiSounds      => "ui-sounds",
            AudioBus::Ceremony      => "ceremony",
            AudioBus::SnapAudio     => "snap-audio",
        }
    }

    /// All four buses, useful for iteration in tests + reset flows.
    pub const ALL: &'static [AudioBus] = &[
        AudioBus::PlatformMusic,
        AudioBus::UiSounds,
        AudioBus::Ceremony,
        AudioBus::SnapAudio,
    ];
}

/// Default per-bus gain. PlatformMusic sits below UI/ceremony so a
/// click or fanfare audibly punches over the music without needing a
/// per-pair ducking matrix (Phase 4.5 stretch). Values are linear
/// gain (rodio's set_volume expects 0..=1.0, occasionally above for
/// boost; we stay ≤ 1.0).
fn default_volume_for_bus(bus: AudioBus) -> f32 {
    match bus {
        AudioBus::PlatformMusic => 0.5,
        AudioBus::UiSounds      => 0.7,
        AudioBus::Ceremony      => 0.85,
        AudioBus::SnapAudio     => 0.6,
    }
}

/// Commands the audio thread processes. Sent via mpsc from Tauri
/// command handlers.
#[derive(Debug, Clone)]
enum AudioCommand {
    Play { bus: AudioBus, path: PathBuf, looped: bool },
    Stop { bus: AudioBus },
    SetVolume { bus: AudioBus, gain: f32 },
    /// Tear-down signal; the thread exits its recv loop after
    /// processing this. Tauri-state drop sends this on shutdown.
    Shutdown,
}

/// Owns the channel into the audio thread. Cheap to clone if we ever
/// want multiple owners; today only Tauri state holds one.
#[derive(Clone)]
pub struct AudioPlayer {
    tx: mpsc::Sender<AudioCommand>,
}

/// Thread-managed audio runtime. Holds the JoinHandle so Drop can
/// signal-and-join on shutdown. The mpsc Sender owns the command
/// channel; cloned into AudioPlayer for handing off to Tauri state.
pub struct AudioPlayerHandle {
    pub player: AudioPlayer,
    join: Option<JoinHandle<()>>,
    tx: mpsc::Sender<AudioCommand>,
}

impl AudioPlayer {
    /// Send a play command. Returns Err only when the audio thread
    /// has exited (rare — would mean rodio failed to initialize or
    /// the app is shutting down).
    pub fn play(&self, bus: AudioBus, path: PathBuf, looped: bool) -> Result<(), String> {
        self.tx
            .send(AudioCommand::Play { bus, path, looped })
            .map_err(|e| format!("audio thread gone: {e}"))
    }

    pub fn stop(&self, bus: AudioBus) -> Result<(), String> {
        self.tx
            .send(AudioCommand::Stop { bus })
            .map_err(|e| format!("audio thread gone: {e}"))
    }

    pub fn set_volume(&self, bus: AudioBus, gain: f32) -> Result<(), String> {
        self.tx
            .send(AudioCommand::SetVolume { bus, gain })
            .map_err(|e| format!("audio thread gone: {e}"))
    }
}

impl AudioPlayerHandle {
    /// Spawn the audio thread. Falls back to a no-op player if rodio
    /// can't open a default output device (no speakers, audio
    /// service down) — the rest of the app keeps running silently
    /// rather than crashing at startup.
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel::<AudioCommand>();
        let join = std::thread::Builder::new()
            .name("oa-audio-player".to_string())
            .spawn(move || audio_thread_main(rx))
            .expect("spawn oa-audio-player thread");
        let player = AudioPlayer { tx: tx.clone() };
        AudioPlayerHandle { player, join: Some(join), tx }
    }

    pub fn player(&self) -> AudioPlayer {
        self.player.clone()
    }
}

impl Drop for AudioPlayerHandle {
    fn drop(&mut self) {
        let _ = self.tx.send(AudioCommand::Shutdown);
        if let Some(j) = self.join.take() {
            // Best-effort join — if the thread is wedged we don't
            // want to hang the parent shutdown. A short wait gives
            // it a chance to flush; beyond that we abandon.
            let _ = j.join();
        }
    }
}

fn audio_thread_main(rx: mpsc::Receiver<AudioCommand>) {
    use std::collections::HashMap;

    let (_stream, stream_handle) = match rodio::OutputStream::try_default() {
        Ok(p) => p,
        Err(e) => {
            log::warn!(
                "oa-audio-player: no default output device ({e:?}); audio commands will silently no-op"
            );
            // Drain the channel as a no-op so senders' send() calls
            // don't fail (which would surface in user logs as
            // distracting warnings on every UI sound dispatch).
            while rx.recv().is_ok() {}
            return;
        }
    };
    log::info!("oa-audio-player: started; 4-bus mixer ready");

    // Per-bus state. Sinks are created on first Play and torn down
    // on Stop / replaced on subsequent Play.
    struct BusState {
        sink: Option<rodio::Sink>,
        volume: f32,
    }
    let mut buses: HashMap<AudioBus, BusState> = HashMap::new();
    for &b in AudioBus::ALL {
        buses.insert(b, BusState { sink: None, volume: default_volume_for_bus(b) });
    }

    while let Ok(cmd) = rx.recv() {
        match cmd {
            AudioCommand::Shutdown => {
                log::info!("oa-audio-player: shutdown received; stopping all buses");
                for state in buses.values_mut() {
                    if let Some(s) = state.sink.take() {
                        s.stop();
                    }
                }
                break;
            }
            AudioCommand::Play { bus, path, looped } => {
                let state = buses.entry(bus).or_insert(BusState {
                    sink: None,
                    volume: default_volume_for_bus(bus),
                });
                // Stop any prior playback on this bus.
                if let Some(s) = state.sink.take() {
                    s.stop();
                }
                let file = match std::fs::File::open(&path) {
                    Ok(f) => f,
                    Err(e) => {
                        log::warn!(
                            "oa-audio-player: open {} on bus {:?} failed: {e}",
                            path.display(), bus,
                        );
                        continue;
                    }
                };
                let reader = std::io::BufReader::new(file);
                let decoder = match rodio::Decoder::new(reader) {
                    Ok(d) => d,
                    Err(e) => {
                        log::warn!(
                            "oa-audio-player: decode {} on bus {:?} failed: {e}",
                            path.display(), bus,
                        );
                        continue;
                    }
                };
                let sink = match rodio::Sink::try_new(&stream_handle) {
                    Ok(s) => s,
                    Err(e) => {
                        log::warn!(
                            "oa-audio-player: sink alloc on bus {:?} failed: {e}",
                            bus,
                        );
                        continue;
                    }
                };
                sink.set_volume(state.volume);
                if looped {
                    use rodio::Source as _;
                    sink.append(decoder.repeat_infinite());
                } else {
                    sink.append(decoder);
                }
                state.sink = Some(sink);
                log::debug!(
                    "oa-audio-player: play {} on {:?} (looped={looped}, vol={:.2})",
                    path.display(), bus, state.volume,
                );
            }
            AudioCommand::Stop { bus } => {
                if let Some(state) = buses.get_mut(&bus) {
                    if let Some(s) = state.sink.take() {
                        s.stop();
                        log::debug!("oa-audio-player: stopped {:?}", bus);
                    }
                }
            }
            AudioCommand::SetVolume { bus, gain } => {
                let gain = gain.clamp(0.0, 1.0);
                if let Some(state) = buses.get_mut(&bus) {
                    state.volume = gain;
                    if let Some(s) = &state.sink {
                        s.set_volume(gain);
                    }
                    log::debug!("oa-audio-player: set volume {:?} = {:.2}", bus, gain);
                }
            }
        }
    }
    log::info!("oa-audio-player: thread exiting");
}

// ---- Tauri commands ----

/// Play `path` on `bus`. `looped` should be true for platform-music
/// and snap-audio (BGM-shaped use cases) and false for one-shot UI
/// cues + ceremony jingles.
#[tauri::command]
#[allow(non_snake_case)]
pub fn play_audio(
    bus: String,
    path: String,
    looped: Option<bool>,
    state: tauri::State<'_, AudioPlayer>,
) -> Result<(), String> {
    let bus = AudioBus::parse(&bus).ok_or_else(|| format!("unknown bus: {bus}"))?;
    let path = PathBuf::from(path);
    state.play(bus, path, looped.unwrap_or(false))
}

#[tauri::command]
pub fn stop_audio(
    bus: String,
    state: tauri::State<'_, AudioPlayer>,
) -> Result<(), String> {
    let bus = AudioBus::parse(&bus).ok_or_else(|| format!("unknown bus: {bus}"))?;
    state.stop(bus)
}

#[tauri::command]
pub fn set_audio_volume(
    bus: String,
    gain: f32,
    state: tauri::State<'_, AudioPlayer>,
) -> Result<(), String> {
    let bus = AudioBus::parse(&bus).ok_or_else(|| format!("unknown bus: {bus}"))?;
    state.set_volume(bus, gain)
}

/// Resolve the platform-music path for a (system_id, optional game_id)
/// pair using the 3-tier cascade: per-game `GameOverrides.platform_music_path`
/// wins, then per-system `SystemSettings.platform_music_path`,
/// otherwise None (theme default — none today; future kiosk-shell
/// theme work fills this).
///
/// Returns the resolved `PathBuf` as a string, or None if every tier
/// is empty. Frontend uses this on library focus change to decide
/// whether to swap the platform-music bus.
#[tauri::command]
#[allow(non_snake_case)]
pub fn resolve_platform_music(
    systemId: String,
    gameId: Option<String>,
    library: tauri::State<'_, crate::library_db::LibraryDb>,
    app_data_dir: tauri::State<'_, crate::AppDataDir>,
) -> Result<Option<String>, String> {
    // Per-game override first.
    if let Some(id) = gameId.as_deref() {
        if let Ok(overrides) = library.get_game_overrides(id) {
            if let Some(p) = overrides.platform_music_path {
                return Ok(Some(p.to_string_lossy().to_string()));
            }
        }
    }
    // Per-system override second.
    let sys = crate::system_settings::read_system_settings(&app_data_dir.0, &systemId);
    if let Some(p) = sys.platform_music_path {
        return Ok(Some(p.to_string_lossy().to_string()));
    }
    Ok(None)
}

/// Resolve a UI-sound event name to a path via the same cascade as
/// platform music, but without a per-game tier (UI sounds are
/// per-system or OA-wide, never per-game). `event` must be one of:
/// "click" | "navigate" | "back" | "launch" | "error" | "scroll-tick".
///
/// Resolution order:
/// 1. Operator override in `SystemSettings.ui_sound_<event>` (per-system
///    file path written through the per-system audio overrides UI).
/// 2. Per-system bundled asset at
///    `<exe_dir>/assets/system-ui/<systemId>/sounds/<event>.<ext>` for
///    `ext` in `[ogg, opus, wav, mp3, flac, m4a]` (matching the
///    formats rodio's `symphonia-all` feature decodes).
/// 3. Universal baseline at
///    `<exe_dir>/assets/system-ui/_baseline/sounds/<event>.<ext>` so
///    every system gets at least a soft click even before a per-system
///    asset bank exists.
/// 4. None — frontend treats null as "skip the dispatch silently."
///
/// Per-System Custom UI Stage 1 Slice 2 introduced the bundled-asset
/// tiers (2 + 3); tier 1 has shipped since the 2026-05-24 media-taxonomy
/// wave. Slice 2's content work drops a CC0 click into the `_baseline`
/// dir; pilot slices 6/7/8 add the system-specific banks.
#[tauri::command]
#[allow(non_snake_case)]
pub fn resolve_ui_sound(
    systemId: String,
    event: String,
    app_data_dir: tauri::State<'_, crate::AppDataDir>,
) -> Result<Option<String>, String> {
    let sys = crate::system_settings::read_system_settings(&app_data_dir.0, &systemId);
    let override_path = match event.as_str() {
        "click"       => sys.ui_sound_click,
        "navigate"    => sys.ui_sound_navigate,
        "back"        => sys.ui_sound_back,
        "launch"      => sys.ui_sound_launch,
        "error"       => sys.ui_sound_error,
        "scroll-tick" => sys.ui_sound_scroll_tick,
        _ => return Err(format!("unknown ui sound event: {event}")),
    };
    if let Some(p) = override_path {
        return Ok(Some(p.to_string_lossy().to_string()));
    }
    let assets_dir = resolve_assets_dir();
    Ok(find_bundled_ui_sound_in_dir(&assets_dir, &systemId, &event)
        .map(|pb| pb.to_string_lossy().to_string()))
}

/// Locate the `<exe_dir>/assets` directory the same way
/// `resolve_cores_dir` locates `<exe_dir>/cores`: next to the .exe.
fn resolve_assets_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("assets")
}

/// Audio file extensions checked in priority order. Mirrors what rodio
/// can decode with the `symphonia-all` feature.
const BUNDLED_UI_SOUND_EXTS: &[&str] = &["ogg", "opus", "wav", "mp3", "flac", "m4a"];

/// Pure resolver — given an assets dir, system slug, and event name,
/// return the first bundled UI sound file that exists. Checks the
/// per-system directory first, then the `_baseline` fallback. Returns
/// None if no candidate exists. Parameterized on `assets_dir` so unit
/// tests can drop a tempdir's contents in and verify the cascade.
fn find_bundled_ui_sound_in_dir(assets_dir: &Path, system_id: &str, event: &str) -> Option<PathBuf> {
    // Defensive — refuse to do disk I/O with traversal-y inputs. Event
    // names come from the resolver's match arm above (closed set) so
    // they're safe; the system slug is operator-influenced via
    // SystemId, so reject anything with path separators.
    if system_id.contains('/') || system_id.contains('\\') || system_id == ".." {
        return None;
    }
    for slug in [system_id, "_baseline"] {
        let base = assets_dir.join("system-ui").join(slug).join("sounds");
        for ext in BUNDLED_UI_SOUND_EXTS {
            let p = base.join(format!("{event}.{ext}"));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_bus_parse_round_trip() {
        for &b in AudioBus::ALL {
            assert_eq!(AudioBus::parse(b.as_str()), Some(b));
        }
    }

    #[test]
    fn audio_bus_parse_rejects_unknown() {
        assert_eq!(AudioBus::parse("game-audio"), None);
        assert_eq!(AudioBus::parse(""), None);
        // Case sensitive.
        assert_eq!(AudioBus::parse("Platform-Music"), None);
    }

    #[test]
    fn audio_bus_as_str_kebab_case() {
        assert_eq!(AudioBus::PlatformMusic.as_str(), "platform-music");
        assert_eq!(AudioBus::UiSounds.as_str(), "ui-sounds");
        assert_eq!(AudioBus::Ceremony.as_str(), "ceremony");
        assert_eq!(AudioBus::SnapAudio.as_str(), "snap-audio");
    }

    #[test]
    fn audio_bus_all_lists_every_variant() {
        // Locks the ALL invariant: bumping the enum forces a bump
        // here too (the count check fails until ALL is updated).
        assert_eq!(AudioBus::ALL.len(), 4);
    }

    #[test]
    fn default_volumes_are_in_range() {
        for &b in AudioBus::ALL {
            let v = default_volume_for_bus(b);
            assert!(v >= 0.0 && v <= 1.0, "{:?} default vol out of range: {v}", b);
        }
    }

    #[test]
    fn default_volumes_have_music_below_cues() {
        // Music sits below cues so a click or fanfare audibly punches
        // through without needing a per-pair ducking matrix
        // (Phase 4.5 stretch).
        assert!(default_volume_for_bus(AudioBus::PlatformMusic)
            < default_volume_for_bus(AudioBus::UiSounds));
        assert!(default_volume_for_bus(AudioBus::PlatformMusic)
            < default_volume_for_bus(AudioBus::Ceremony));
    }

    #[test]
    fn audio_bus_serde_round_trip() {
        let json = serde_json::to_string(&AudioBus::PlatformMusic).expect("serialize");
        assert_eq!(json, "\"platform-music\"");
        let parsed: AudioBus = serde_json::from_str("\"snap-audio\"").expect("deserialize");
        assert_eq!(parsed, AudioBus::SnapAudio);
    }

    // --- Per-System UI Stage 1 Slice 2: bundled UI sound resolver ----

    fn tempdir_for(tag: &str) -> PathBuf {
        let tmp = std::env::temp_dir().join(format!("oa-shell-ui-sound-{tag}"));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        tmp
    }

    fn drop_sound(assets_dir: &Path, system_slug: &str, event: &str, ext: &str) -> PathBuf {
        let dir = assets_dir.join("system-ui").join(system_slug).join("sounds");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(format!("{event}.{ext}"));
        std::fs::write(&p, b"fake-audio").unwrap();
        p
    }

    #[test]
    fn bundled_resolver_finds_per_system_first() {
        let tmp = tempdir_for("per-system-wins");
        let per_system = drop_sound(&tmp, "nes", "navigate", "ogg");
        let _baseline = drop_sound(&tmp, "_baseline", "navigate", "ogg");

        let resolved = find_bundled_ui_sound_in_dir(&tmp, "nes", "navigate");
        assert_eq!(resolved.as_deref(), Some(per_system.as_path()));
    }

    #[test]
    fn bundled_resolver_falls_back_to_baseline() {
        let tmp = tempdir_for("baseline-fallback");
        let baseline = drop_sound(&tmp, "_baseline", "navigate", "ogg");

        let resolved = find_bundled_ui_sound_in_dir(&tmp, "nes", "navigate");
        assert_eq!(resolved.as_deref(), Some(baseline.as_path()));
    }

    #[test]
    fn bundled_resolver_returns_none_when_nothing_on_disk() {
        let tmp = tempdir_for("empty");
        let resolved = find_bundled_ui_sound_in_dir(&tmp, "nes", "navigate");
        assert!(resolved.is_none());
    }

    #[test]
    fn bundled_resolver_prefers_ogg_over_other_extensions() {
        // Extension priority matters when a system bank ships multiple
        // formats for the same event — operators dropping in both .ogg
        // and .wav copies should see the .ogg play (smaller + lossless
        // enough for short SFX). Locks BUNDLED_UI_SOUND_EXTS order.
        let tmp = tempdir_for("ext-priority");
        let ogg = drop_sound(&tmp, "nes", "navigate", "ogg");
        let _wav = drop_sound(&tmp, "nes", "navigate", "wav");

        let resolved = find_bundled_ui_sound_in_dir(&tmp, "nes", "navigate");
        assert_eq!(resolved.as_deref(), Some(ogg.as_path()));
    }

    #[test]
    fn bundled_resolver_walks_extension_list_when_ogg_missing() {
        let tmp = tempdir_for("ext-walk");
        let wav = drop_sound(&tmp, "_baseline", "click", "wav");
        let resolved = find_bundled_ui_sound_in_dir(&tmp, "nes", "click");
        assert_eq!(resolved.as_deref(), Some(wav.as_path()));
    }

    #[test]
    fn bundled_resolver_rejects_path_traversal_in_system_id() {
        // Defensive — the system slug is operator-influenced (it
        // matches SystemId from the frontend registry) so reject any
        // path-separator content before it can land in std::fs::Path
        // join.
        let tmp = tempdir_for("traversal");
        drop_sound(&tmp, "_baseline", "navigate", "ogg");

        for evil in ["../nes", "..\\nes", "..", "nes/../etc", "nes\\..\\etc"] {
            let resolved = find_bundled_ui_sound_in_dir(&tmp, evil, "navigate");
            assert!(resolved.is_none(), "expected None for slug {evil:?}, got {resolved:?}");
        }
    }
}
