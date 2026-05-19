//! Three-output logger — stderr + per-session file + bounded in-memory ring.
//!
//! Replaces the bare `env_logger::init()` so we get:
//!
//! - **stderr**: preserves the dev workflow (cargo tauri dev sees the same
//!   log stream as before).
//! - **File**: every session writes to
//!   `appData/logs/oa-current.log` (truncated on launch) PLUS a permanent
//!   archive copy at `appData/logs/oa-<ISO-timestamp>.log`. The "current"
//!   file is the stable path a debugger can always Read; the archives are
//!   for post-mortem of prior runs. Keeps the most recent N archives;
//!   older ones get pruned on startup.
//! - **Ring**: a bounded `VecDeque<LogEntry>` the `get_recent_logs` Tauri
//!   command copies from. Lives entirely in RAM (~500 KB worst-case at
//!   2000 entries), so the `Help → Debug log…` dialog reads from it at
//!   1 Hz without involving the file.
//!
//! Init is two-phase because `app_data_dir` isn't resolved until Tauri
//! `setup()` runs:
//!
//! 1. [`init_early`] installs the logger at `main()` start. Only stderr
//!    + ring are active; the file output is None.
//! 2. [`configure_file_output`] is called from `setup()` once we know
//!    the path. From that point on the file is also written.
//!
//! Logs emitted between (1) and (2) — the very early `main()` diagnostics
//! — land in stderr + ring but NOT in the file. That window is normally
//! a handful of lines (cli arg parse, shell_mode resolution); acceptable.

use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;

/// Single log record surfaced to the frontend's `get_recent_logs` view.
/// Both the file and the ring serialize from this shape; the format on
/// disk is human-readable plain text, while the ring vends JSON via
/// Tauri commands.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    /// Milliseconds since the Unix epoch.
    pub timestamp_ms: i64,
    /// `"error"` | `"warn"` | `"info"` | `"debug"` | `"trace"`.
    pub level: &'static str,
    /// Module path the record came from (e.g. `oa_shell::media`).
    pub target: String,
    /// The formatted message body.
    pub message: String,
}

/// Bounded ring buffer of recent log entries. Eviction policy: oldest
/// first. `push` is O(1).
pub struct LogRing {
    capacity: usize,
    entries: VecDeque<LogEntry>,
}

impl LogRing {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: VecDeque::with_capacity(capacity),
        }
    }

    fn push(&mut self, entry: LogEntry) {
        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// Copy the last `limit` entries (or all of them if `limit > len`).
    /// Returns oldest-first; the dialog auto-scrolls to the bottom.
    pub fn snapshot(&self, limit: Option<usize>) -> Vec<LogEntry> {
        let n = limit.unwrap_or(self.entries.len()).min(self.entries.len());
        self.entries
            .iter()
            .skip(self.entries.len() - n)
            .cloned()
            .collect()
    }
}

const RING_CAPACITY: usize = 2_000;
/// How many archived session log files to retain before pruning. The
/// `oa-current.log` is separate (always exists, always the latest).
const ARCHIVE_RETAIN: usize = 5;

/// The global logger instance. `log::set_logger` requires a `&'static`
/// reference, so we park the logger in a `OnceLock`.
static LOGGER: OnceLock<MultiLogger> = OnceLock::new();

/// Type-erased file-side writer. We swap a `TeeWriter` (current + archive)
/// in via `configure_file_output`; before that the slot is `None`.
type FileSink = Box<dyn Write + Send>;

pub struct MultiLogger {
    ring: Arc<Mutex<LogRing>>,
    file: Arc<Mutex<Option<FileSink>>>,
    file_path: Arc<Mutex<Option<PathBuf>>>,
    min_level: log::LevelFilter,
}

impl log::Log for MultiLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.min_level
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
        let timestamp_ms = now.unix_timestamp_nanos() as i64 / 1_000_000;

        let level: &'static str = match record.level() {
            log::Level::Error => "error",
            log::Level::Warn => "warn",
            log::Level::Info => "info",
            log::Level::Debug => "debug",
            log::Level::Trace => "trace",
        };
        let target = record.target().to_string();
        let message = format!("{}", record.args());

        // Stderr — preserve the dev workflow. Same format env_logger
        // produced before, roughly.
        let time_str = now.format(&Iso8601::DEFAULT).unwrap_or_else(|_| String::new());
        eprintln!("{} {:<5} [{}] {}", time_str, level.to_uppercase(), target, message);

        // File — same format as stderr.
        if let Ok(mut file_guard) = self.file.lock() {
            if let Some(writer) = file_guard.as_mut() {
                let _ = writeln!(
                    writer,
                    "{} {:<5} [{}] {}",
                    time_str,
                    level.to_uppercase(),
                    target,
                    message
                );
                // Flush on WARN/ERROR so a crash mid-line still leaves a
                // useful tail. INFO/DEBUG ride the BufWriter and flush
                // on drop or buffer fill.
                if matches!(record.level(), log::Level::Warn | log::Level::Error) {
                    let _ = writer.flush();
                }
            }
        }

        // Ring — Tauri command reads from this.
        if let Ok(mut ring) = self.ring.lock() {
            ring.push(LogEntry {
                timestamp_ms,
                level,
                target,
                message,
            });
        }
    }

    fn flush(&self) {
        if let Ok(mut file_guard) = self.file.lock() {
            if let Some(writer) = file_guard.as_mut() {
                let _ = writer.flush();
            }
        }
    }
}

/// Logger handle — owns the ring + file slots the Tauri commands need
/// to reach. Held by `AppState`.
#[derive(Clone)]
pub struct LoggerHandle {
    pub ring: Arc<Mutex<LogRing>>,
    pub file_path: Arc<Mutex<Option<PathBuf>>>,
}

/// Install the logger at `main()` start. Stderr + ring are active; the
/// file output is None until [`configure_file_output`] runs. Returns a
/// handle that AppState can use to drive Tauri commands.
pub fn init_early() -> LoggerHandle {
    let ring = Arc::new(Mutex::new(LogRing::new(RING_CAPACITY)));
    let file: Arc<Mutex<Option<FileSink>>> = Arc::new(Mutex::new(None));
    let file_path: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));

    let min_level = std::env::var("RUST_LOG")
        .ok()
        .and_then(|v| v.parse::<log::LevelFilter>().ok())
        .unwrap_or(log::LevelFilter::Info);

    let logger = MultiLogger {
        ring: ring.clone(),
        file: file.clone(),
        file_path: file_path.clone(),
        min_level,
    };
    let _ = LOGGER.set(logger);
    let static_ref: &'static MultiLogger = LOGGER.get().expect("logger just installed");
    let _ = log::set_logger(static_ref).map(|()| log::set_max_level(min_level));

    LoggerHandle { ring, file_path }
}

/// Open the per-session log files. Truncates `oa-current.log` and
/// creates a fresh archive copy under `oa-<ISO-timestamp>.log`. Both
/// receive the same lines. Older archives beyond `ARCHIVE_RETAIN` are
/// pruned in oldest-first order.
///
/// Returns the path of the current session's archive file. The
/// `oa-current.log` path is always `<logs>/oa-current.log` — stable
/// across runs so a debugger can always Read it.
pub fn configure_file_output(app_data_dir: &Path) -> std::io::Result<PathBuf> {
    let logs_dir = app_data_dir.join("logs");
    std::fs::create_dir_all(&logs_dir)?;

    // Open the stable "current" file — truncated on every launch.
    let current_path = logs_dir.join("oa-current.log");
    let current_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&current_path)?;

    // Open a timestamped archive copy that survives this session. Same
    // bytes get written to both via the Tee writer below.
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    let stamp = now
        .format(&time::macros::format_description!(
            "[year][month][day]-[hour][minute][second]"
        ))
        .unwrap_or_else(|_| "unknown".into());
    let archive_path = logs_dir.join(format!("oa-{stamp}.log"));
    let archive_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&archive_path)?;

    let tee = TeeWriter {
        a: BufWriter::new(current_file),
        b: BufWriter::new(archive_file),
    };

    if let Some(logger) = LOGGER.get() {
        if let Ok(mut slot) = logger.file.lock() {
            *slot = Some(Box::new(tee) as FileSink);
        }
        if let Ok(mut slot) = logger.file_path.lock() {
            *slot = Some(current_path.clone());
        }
    }

    // Prune older archives. List `oa-*.log` (excluding "current"), sort
    // by modified time, retain the newest N.
    prune_old_archives(&logs_dir);

    Ok(current_path)
}

/// File writer that fans bytes to both a per-session "current" file and
/// a timestamped archive. Same bytes, two destinations.
struct TeeWriter {
    a: BufWriter<File>,
    b: BufWriter<File>,
}
impl Write for TeeWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let _ = self.a.write_all(buf);
        let _ = self.b.write_all(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        let _ = self.a.flush();
        self.b.flush()
    }
}

fn prune_old_archives(logs_dir: &Path) {
    let Ok(entries) = std::fs::read_dir(logs_dir) else { return };
    let mut archives: Vec<(PathBuf, std::time::SystemTime)> = entries
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let name = p.file_name()?.to_string_lossy().into_owned();
            if !name.starts_with("oa-") || !name.ends_with(".log") {
                return None;
            }
            if name == "oa-current.log" {
                return None;
            }
            let modified = e.metadata().ok()?.modified().ok()?;
            Some((p, modified))
        })
        .collect();
    if archives.len() <= ARCHIVE_RETAIN {
        return;
    }
    archives.sort_by_key(|(_, t)| std::cmp::Reverse(*t));
    for (path, _) in archives.into_iter().skip(ARCHIVE_RETAIN) {
        let _ = std::fs::remove_file(&path);
    }
}
