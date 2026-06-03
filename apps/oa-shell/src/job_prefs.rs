//! Per-kind preferences for background-jobs behavior. Lives in a
//! standalone JSON file at `<data_dir>/library/job-prefs.json` so the
//! schema is independent of the library_prefs aggregation tunables.
//!
//! Phase 3b ships the data plumbing only. The Settings UI for these
//! toggles lands in Phase 5; for now operators can edit the JSON
//! directly if they need to override the auto-resume default for a
//! specific kind.
//!
//! Plan reference: docs/PLANS/background-jobs-and-progress-bar.md
//! §"Resume on app launch" — auto-resume by default, per-kind opt-out
//! via this prefs file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JobPrefs {
    /// Per-kind opt-out: when the kind's entry is `true`, the
    /// resume_interrupted_jobs dispatcher SKIPS its registered
    /// resumer and instead emits an `oa://job-event` ResumePrompt
    /// variant carrying the snapshot. The frontend surfaces a
    /// prompt dialog at launch (Slice D of the polish branch).
    ///
    /// Empty map = auto-resume every kind (the plan's default).
    /// Insert `kind => true` to opt out of auto-resume for that kind.
    /// Insert `kind => false` (or omit) for the default behavior.
    #[serde(default)]
    pub prompt_before_resume_on_launch: HashMap<String, bool>,

    /// Polish: play a subtle completion chime through the
    /// `ui-sounds` audio bus when a job finalizes successfully.
    /// Default ON. The frontend silently no-ops if the chime asset
    /// isn't bundled — see docs/features/background-jobs/ASSETS.md
    /// for where to drop the file.
    #[serde(default = "default_true")]
    pub sound_on_completion: bool,

    /// Polish: keep the bar's collapsed handle visible even when
    /// no jobs are active. Default OFF — the handle normally
    /// auto-hides per plan §"Bar UI shape." Operators who want a
    /// persistent "I can always see the bar" affordance flip
    /// this ON.
    #[serde(default)]
    pub always_show_bar: bool,
}

impl Default for JobPrefs {
    fn default() -> Self {
        Self {
            prompt_before_resume_on_launch: HashMap::new(),
            sound_on_completion: true,
            always_show_bar: false,
        }
    }
}

fn default_true() -> bool {
    true
}

impl JobPrefs {
    /// Closure-friendly accessor used by
    /// `JobRegistry::resume_interrupted_jobs`. Returns true when the
    /// kind has an explicit opt-out; false otherwise (including the
    /// "no entry" common case).
    pub fn should_prompt(&self, kind: &str) -> bool {
        self.prompt_before_resume_on_launch
            .get(kind)
            .copied()
            .unwrap_or(false)
    }
}

fn prefs_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("library").join("job-prefs.json")
}

pub fn read_job_prefs(app_data_dir: &Path) -> JobPrefs {
    let path = prefs_path(app_data_dir);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn write_job_prefs(app_data_dir: &Path, prefs: &JobPrefs) -> std::io::Result<()> {
    let path = prefs_path(app_data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(prefs).unwrap_or_else(|_| "{}".into());
    std::fs::write(&path, body)
}

/// Read the current prefs.
#[tauri::command]
pub fn get_job_prefs(state: tauri::State<'_, crate::AppDataDir>) -> JobPrefs {
    read_job_prefs(&state.0)
}

/// Flip the per-kind prompt-before-resume bit. Pass `prompt=true` to
/// opt out of auto-resume for `kind`; `prompt=false` (or just don't
/// invoke) for the default behavior.
#[tauri::command]
pub fn set_job_resume_prompt(
    kind: String,
    prompt: bool,
    state: tauri::State<'_, crate::AppDataDir>,
) -> Result<JobPrefs, String> {
    let mut prefs = read_job_prefs(&state.0);
    if prompt {
        prefs.prompt_before_resume_on_launch.insert(kind, true);
    } else {
        prefs.prompt_before_resume_on_launch.remove(&kind);
    }
    write_job_prefs(&state.0, &prefs)
        .map_err(|e| format!("write job-prefs.json: {e}"))?;
    Ok(prefs)
}

/// Toggle the completion chime. Returns the updated prefs.
#[tauri::command]
pub fn set_job_sound_on_completion(
    enabled: bool,
    state: tauri::State<'_, crate::AppDataDir>,
) -> Result<JobPrefs, String> {
    let mut prefs = read_job_prefs(&state.0);
    prefs.sound_on_completion = enabled;
    write_job_prefs(&state.0, &prefs)
        .map_err(|e| format!("write job-prefs.json: {e}"))?;
    Ok(prefs)
}

/// Toggle the always-show-bar handle. Returns the updated prefs.
#[tauri::command]
pub fn set_job_always_show_bar(
    enabled: bool,
    state: tauri::State<'_, crate::AppDataDir>,
) -> Result<JobPrefs, String> {
    let mut prefs = read_job_prefs(&state.0);
    prefs.always_show_bar = enabled;
    write_job_prefs(&state.0, &prefs)
        .map_err(|e| format!("write job-prefs.json: {e}"))?;
    Ok(prefs)
}
