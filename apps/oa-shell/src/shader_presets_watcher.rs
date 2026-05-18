//! Phase 3 slice D — filesystem watcher for shader presets.
//!
//! Watches `<exe_dir>/shaders/presets/` for changes. On every modify /
//! create / remove event the watcher does two things:
//!
//! 1. Emits `oa://shader-presets-changed` with the freshly-loaded registry
//!    summary so the frontend's `shaderPresets()` signal refreshes (any
//!    open dropdowns immediately show new / renamed / removed presets).
//! 2. If the user has an active preset selected (set by the most recent
//!    `set_shader_preset` Tauri call), re-resolves it through the registry
//!    and sends `EmuCommand::ApplyShaderPreset` so the emu thread picks
//!    up the new params (e.g. tweaking `bloom_amount` in
//!    `phosphor.preset.toml` while the game is running takes effect on
//!    the next frame, no relaunch needed).
//!
//! No debouncing today — notify on Windows coalesces most write+close+
//! attr triplets and a handful of double-applies is harmless. If a user
//! reports flicker, add `notify-debouncer-mini` here.

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use notify::{Event, EventKind, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter};

use crate::shader_presets::{self, ResolvedPreset};

/// Owns the underlying OS watcher handle. Dropping this stops the watcher.
/// Stored on AppState so it lives for the duration of the process. The
/// `Mutex<Option<...>>` wrap matches the pattern in `watcher.rs` — the
/// trait-object `dyn Watcher + Send` isn't Sync on its own, and AppState
/// needs to be Sync to satisfy Tauri's `State<'_, T>` bound.
pub struct ShaderPresetsWatcher {
    #[allow(dead_code)] // held to keep the OS watcher alive
    inner: Mutex<Option<Box<dyn Watcher + Send>>>,
}

/// State shared with the notify callback. Cheap to clone — every field
/// is either Arc'd, Clone-able, or owned by the watcher itself.
#[derive(Clone)]
struct CallbackState {
    handle: AppHandle,
    exe_dir: PathBuf,
    emu_tx: Sender<crate::EmuCommand>,
    active_preset: Arc<Mutex<Option<String>>>,
}

/// Spawn the watcher. Creates `<exe_dir>/shaders/presets/` if missing so
/// users can drop a `.preset.toml` file in a freshly-installed exe without
/// having to mkdir first. Returns an error if the directory creation or
/// watcher registration fails.
pub fn spawn(
    handle: AppHandle,
    exe_dir: PathBuf,
    emu_tx: Sender<crate::EmuCommand>,
    active_preset: Arc<Mutex<Option<String>>>,
) -> Result<ShaderPresetsWatcher, String> {
    let presets_dir = exe_dir.join("shaders").join("presets");
    std::fs::create_dir_all(&presets_dir)
        .map_err(|e| format!("mkdir {}: {e}", presets_dir.display()))?;

    let state = CallbackState {
        handle,
        exe_dir,
        emu_tx,
        active_preset,
    };

    let cb_state = state.clone();
    let mut watcher: notify::RecommendedWatcher = notify::Watcher::new(
        move |res: Result<Event, notify::Error>| {
            let Ok(event) = res else {
                return;
            };
            if !is_relevant(&event) {
                return;
            }
            handle_event(&cb_state);
        },
        notify::Config::default().with_poll_interval(Duration::from_secs(2)),
    )
    .map_err(|e| format!("notify::Watcher::new: {e}"))?;

    watcher
        .watch(&presets_dir, RecursiveMode::NonRecursive)
        .map_err(|e| format!("watch {}: {e}", presets_dir.display()))?;

    log::info!(
        "oa-shell: shader presets watcher started on {}",
        presets_dir.display()
    );

    Ok(ShaderPresetsWatcher {
        inner: Mutex::new(Some(Box::new(watcher))),
    })
}

/// Filter out events that can't change the registry (e.g. file-access
/// time bumps). We care about content / creation / removal / rename.
fn is_relevant(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any
    ) && event.paths.iter().any(|p| {
        p.file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.ends_with(".preset.toml"))
            .unwrap_or(false)
    })
}

fn handle_event(state: &CallbackState) {
    let defs = shader_presets::load_all(&state.exe_dir);
    let summary = shader_presets::summarize(&defs);
    if let Err(e) = state.handle.emit("oa://shader-presets-changed", &summary) {
        log::warn!("oa-shell: emit shader-presets-changed failed: {e}");
    }

    // Re-apply the active preset if one is selected. Reads the latest
    // name out of the mutex each time — a preset rename mid-watch could
    // leave the stored name pointing at nothing, in which case
    // `set_shader_preset` falls back to plain (same as the Tauri command
    // path).
    let name = match state.active_preset.lock() {
        Ok(guard) => guard.clone(),
        Err(e) => {
            log::warn!("oa-shell: active_preset lock poisoned: {e}");
            return;
        }
    };
    let Some(name) = name else { return };

    let def = match defs.iter().find(|d| d.name == name) {
        Some(d) => d.clone(),
        None => {
            log::warn!(
                "oa-shell: active preset `{name}` not in updated registry; leaving renderer as-is"
            );
            return;
        }
    };
    let resolved: ResolvedPreset = shader_presets::apply(&def, &state.exe_dir);
    if let Err(e) = state.emu_tx.send(crate::EmuCommand::ApplyShaderPreset(resolved)) {
        log::warn!("oa-shell: re-apply shader preset failed (emu thread closed?): {e}");
    } else {
        log::info!("oa-shell: shader preset `{name}` re-applied after TOML change");
    }
}

/// Re-export of [`is_relevant`] so external tests can verify the filter
/// without spinning up a real watcher.
#[cfg(test)]
pub(crate) fn is_relevant_for_test(event: &Event) -> bool {
    is_relevant(event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{CreateKind, ModifyKind, RemoveKind};
    use std::path::Path;

    fn evt_with(kind: EventKind, paths: &[&str]) -> Event {
        Event {
            kind,
            paths: paths.iter().map(|p| Path::new(p).to_path_buf()).collect(),
            attrs: notify::event::EventAttributes::default(),
        }
    }

    #[test]
    fn relevant_for_preset_toml_modify() {
        let e = evt_with(
            EventKind::Modify(ModifyKind::Any),
            &["G:/foo/shaders/presets/phosphor.preset.toml"],
        );
        assert!(is_relevant_for_test(&e));
    }

    #[test]
    fn relevant_for_preset_toml_create() {
        let e = evt_with(
            EventKind::Create(CreateKind::File),
            &["/tmp/shaders/presets/new.preset.toml"],
        );
        assert!(is_relevant_for_test(&e));
    }

    #[test]
    fn relevant_for_preset_toml_remove() {
        let e = evt_with(
            EventKind::Remove(RemoveKind::File),
            &["/tmp/shaders/presets/gone.preset.toml"],
        );
        assert!(is_relevant_for_test(&e));
    }

    #[test]
    fn ignores_non_preset_files() {
        // Plain .toml file in the dir (not .preset.toml) → ignored.
        let e = evt_with(
            EventKind::Modify(ModifyKind::Any),
            &["/tmp/shaders/presets/notes.toml"],
        );
        assert!(!is_relevant_for_test(&e));
        // .wgsl file → also ignored (we don't auto-reload shaders today).
        let e = evt_with(
            EventKind::Modify(ModifyKind::Any),
            &["/tmp/shaders/blit.wgsl"],
        );
        assert!(!is_relevant_for_test(&e));
    }
}
