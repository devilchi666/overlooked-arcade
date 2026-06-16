//! Network-call audit log for the content-pack manager (content-packs.md §9).
//!
//! Every network call the pack mechanism makes — registry fetches and pack
//! downloads — appends one entry here, so the Settings → Privacy panel can
//! show a per-call trail of exactly which URLs OA has hit and when. This is
//! the transparency half of the project's "operator-initiated, fully
//! disclosed, no telemetry" network posture (content-packs.md §3).
//!
//! A bounded ring of the last [`MAX_ENTRIES`], persisted as a JSON array at
//! `appDataDir/packs/network.log`. Best-effort: a write failure is logged,
//! never fatal — the audit log must not be able to break a real install.
//! Reads/writes are not serialized against concurrent pack operations; the
//! commands that write it are operator-initiated and rarely overlap, so the
//! occasional lost entry under true concurrency is acceptable for an audit
//! trail (it never affects install correctness).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// How many recent entries to keep. content-packs.md §12 says "last 100".
const MAX_ENTRIES: usize = 100;

/// One logged network call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetLogEntry {
    /// When the call completed, RFC3339. `None` only if the clock format
    /// failed (shouldn't happen).
    pub at: Option<String>,
    /// What triggered it: `registry`, `install:<id>`, `update:<id>`.
    pub action: String,
    /// The exact URL hit.
    pub url: String,
    /// `ok` | `error`.
    pub outcome: String,
    /// Error message on failure (omitted on success).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

fn log_path(data_dir: &Path) -> PathBuf {
    data_dir.join("packs").join("network.log")
}

fn now_rfc3339() -> Option<String> {
    use time::format_description::well_known::Rfc3339;
    time::OffsetDateTime::now_utc().format(&Rfc3339).ok()
}

/// Read the persisted log oldest-first (file order). Missing / malformed
/// file → empty.
pub fn read_log(data_dir: &Path) -> Vec<NetLogEntry> {
    let Ok(raw) = std::fs::read_to_string(log_path(data_dir)) else {
        return Vec::new();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

/// Append one entry, trimming to the last [`MAX_ENTRIES`]. Best-effort.
pub fn record(data_dir: &Path, action: &str, url: &str, outcome: &str, detail: Option<String>) {
    let mut entries = read_log(data_dir);
    entries.push(NetLogEntry {
        at: now_rfc3339(),
        action: action.to_string(),
        url: url.to_string(),
        outcome: outcome.to_string(),
        detail,
    });
    let len = entries.len();
    if len > MAX_ENTRIES {
        entries.drain(0..len - MAX_ENTRIES);
    }
    let path = log_path(data_dir);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(&entries) {
        Ok(body) => {
            if let Err(e) = std::fs::write(&path, body) {
                log::warn!("oa-packs: write network.log failed: {e}");
            }
        }
        Err(e) => log::warn!("oa-packs: serialize network.log failed: {e}"),
    }
}

/// Erase the log.
pub fn clear(data_dir: &Path) -> std::io::Result<()> {
    let path = log_path(data_dir);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oa-netlog-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn missing_log_is_empty() {
        assert!(read_log(&tmp("missing")).is_empty());
    }

    #[test]
    fn records_round_trip_and_clear() {
        let dir = tmp("rt");
        record(&dir, "registry", "https://example.invalid/r.json", "ok", None);
        record(&dir, "install:p", "https://example.invalid/p.zip", "error", Some("boom".into()));
        let log = read_log(&dir);
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].action, "registry");
        assert_eq!(log[1].outcome, "error");
        assert_eq!(log[1].detail.as_deref(), Some("boom"));
        clear(&dir).unwrap();
        assert!(read_log(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ring_trims_to_max() {
        let dir = tmp("ring");
        for i in 0..(MAX_ENTRIES + 25) {
            record(&dir, "registry", &format!("https://example.invalid/{i}"), "ok", None);
        }
        let log = read_log(&dir);
        assert_eq!(log.len(), MAX_ENTRIES);
        // Oldest 25 dropped — the first surviving entry is #25.
        assert_eq!(log[0].url, "https://example.invalid/25");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
