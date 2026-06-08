//! Launcher implementations — virtual-library Phase C
//! (docs/PLANS/launcher-abstraction.md).
//!
//! [`LibretroLauncher`] wraps the in-process libretro pipeline behind
//! the `oa_core::Launcher` lifecycle contract: `launch` maps onto
//! `EmuCommand::LoadRom`, `terminate` onto `EmuCommand::UnloadRom` —
//! the exact dispatches `launch_rom` / `unload_rom` performed inline
//! before Phase C1, including the error strings.
//!
//! [`ExternalProcessLauncher`] (Phase C2) spawns a standalone emulator
//! binary from a `config/emulators/<id>.yaml` profile: argv comes from
//! the profile's `launch_args_template` with `{content}` expanded,
//! stdout/stderr land in the OA debug log, liveness is child-PID
//! based, and terminate tries a graceful close before the
//! kill-after-timeout fallback.

use std::io::BufRead;
use std::path::PathBuf;
use std::sync::{mpsc, Mutex};
use std::time::Duration;

use oa_core::{
    LaunchError, LaunchPrepared, LaunchRequest, LaunchedSession, Launcher, LauncherCapabilities,
};

use crate::emulator_profiles::{expand_args, EmulatorProfile};
use crate::EmuCommand;

/// The in-process libretro launcher. Holds its own clone of the
/// emulator thread's command sender; `Sender` is `!Sync`, so it sits
/// behind a `Mutex` exactly like `AppState::emu_tx`.
pub struct LibretroLauncher {
    emu_tx: Mutex<mpsc::Sender<EmuCommand>>,
}

impl LibretroLauncher {
    /// Build around a clone of the emulator thread's command sender.
    pub fn new(emu_tx: mpsc::Sender<EmuCommand>) -> Self {
        Self { emu_tx: Mutex::new(emu_tx) }
    }
}

impl Launcher for LibretroLauncher {
    fn id(&self) -> &str {
        "libretro"
    }

    fn capabilities(&self) -> LauncherCapabilities {
        // Today's behavior: the in-process pipeline supports the whole
        // QuickSettings surface.
        LauncherCapabilities::all()
    }

    fn prepare(&self, request: LaunchRequest) -> Result<LaunchPrepared, LaunchError> {
        // Nothing to resolve for the in-process path — content
        // resolution already ran in the shell, and the core .dll
        // precedence chain (per-game override → per-system pref →
        // hardcoded default) is resolved by the emulator thread at
        // LoadRom time, as before C1.
        Ok(LaunchPrepared { request })
    }

    fn launch(&self, prepared: LaunchPrepared) -> Result<LaunchedSession, LaunchError> {
        let req = prepared.request;
        let tx = self
            .emu_tx
            .lock()
            .map_err(|_| LaunchError::Launch("emu_tx poisoned".to_string()))?;
        tx.send(EmuCommand::LoadRom {
            path: req.content_path,
            bytes: req.content_bytes,
            restore_slot: req.restore_slot,
            restore_state_path: req.restore_state_path,
            core_override: req.core_override,
            system_id: req.system_id,
        })
        .map_err(|e| LaunchError::Launch(format!("emu thread closed: {e}")))?;
        Ok(LaunchedSession::InProcess)
    }

    fn is_alive(&self, session: &LaunchedSession) -> bool {
        // The emulator thread lives as long as the shell, so an
        // in-process session is alive until terminated. (Whether a ROM
        // is actually loaded is the emu thread's business — UnloadRom
        // on an empty core is already a graceful no-op.)
        matches!(session, LaunchedSession::InProcess)
    }

    fn terminate(
        &self,
        _session: &LaunchedSession,
        title: Option<String>,
    ) -> Result<(), LaunchError> {
        let tx = self
            .emu_tx
            .lock()
            .map_err(|_| LaunchError::Terminate("emu_tx poisoned".to_string()))?;
        tx.send(EmuCommand::UnloadRom { title })
            .map_err(|e| LaunchError::Terminate(format!("emu thread closed: {e}")))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ExternalProcessLauncher — VL Phase C2
// ---------------------------------------------------------------------------

/// How long terminate waits for a graceful close before force-killing.
const TERMINATE_GRACE: Duration = Duration::from_secs(5);

/// Spawns a standalone external emulator as a child process. One
/// instance per launch — `launch_rom` builds it from the per-system
/// default-launcher pref + the matching emulator profile, and the
/// instance owns its child handle until the session ends.
///
/// The launcher only covers the lifecycle contract; window
/// choreography (minimize-on-spawn / restore-on-exit, decision D3) and
/// play-session bookkeeping stay in the shell, which polls
/// [`Launcher::is_alive`] from a watcher thread.
pub struct ExternalProcessLauncher {
    profile_id: String,
    display_name: String,
    binary_path: PathBuf,
    args_template: Vec<String>,
    capabilities: LauncherCapabilities,
    /// The spawned child. Held so terminate can kill/reap and is_alive
    /// can `try_wait` (both need `&mut Child`).
    child: Mutex<Option<std::process::Child>>,
}

impl ExternalProcessLauncher {
    /// Build from a profile + the resolved binary path (appData pref →
    /// profile field, already decided by the caller via
    /// `emulator_profiles::effective_binary_path`).
    pub fn new(profile: &EmulatorProfile, binary_path: PathBuf) -> Self {
        Self {
            profile_id: profile.id.clone(),
            display_name: profile.display_name.clone(),
            binary_path,
            args_template: profile.launch_args_template.clone(),
            capabilities: profile.capabilities,
            child: Mutex::new(None),
        }
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Forward one std stream of the child to the OA debug log, one
    /// line at a time, on a dedicated reader thread. The thread ends
    /// when the child closes the stream (i.e. exits).
    fn pump_stream<R: std::io::Read + Send + 'static>(
        profile_id: String,
        stream_name: &'static str,
        stream: R,
    ) {
        std::thread::Builder::new()
            .name(format!("oa-external-{stream_name}"))
            .spawn(move || {
                let reader = std::io::BufReader::new(stream);
                for line in reader.lines() {
                    match line {
                        // Emulator stderr is routinely informational
                        // (Dolphin logs there), so both streams land at
                        // INFO; the stream name disambiguates.
                        Ok(l) if !l.is_empty() => log::info!(
                            target: "oa_shell::external",
                            "[{profile_id}:{stream_name}] {l}"
                        ),
                        Ok(_) => {}
                        Err(_) => break,
                    }
                }
            })
            .ok();
    }
}

/// Ask a process to close gracefully. On Windows `taskkill` (without
/// `/F`) posts WM_CLOSE to the process's windows — Dolphin treats that
/// as a normal quit and flushes its saves. Returns false when the
/// request couldn't be delivered (already dead, no windows, …); the
/// caller falls through to the hard kill.
#[cfg(windows)]
fn request_graceful_close(pid: u32) -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string()])
        .creation_flags(CREATE_NO_WINDOW)
        // taskkill chatters on both streams ("ERROR: ... /F option") —
        // its exit code is the only thing we act on.
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Non-Windows: no portable WM_CLOSE equivalent through std — skip
/// straight to the kill fallback. (A SIGTERM via libc can ride along
/// when an external-emulator pilot lands on those hosts.)
#[cfg(not(windows))]
fn request_graceful_close(_pid: u32) -> bool {
    false
}

impl Launcher for ExternalProcessLauncher {
    fn id(&self) -> &str {
        &self.profile_id
    }

    fn capabilities(&self) -> LauncherCapabilities {
        self.capabilities
    }

    fn prepare(&self, request: LaunchRequest) -> Result<LaunchPrepared, LaunchError> {
        if !self.binary_path.is_file() {
            return Err(LaunchError::Prepare(format!(
                "{} binary not found at {} — set the binary path in Settings → Cores → External emulators",
                self.display_name,
                self.binary_path.display()
            )));
        }
        if !self.args_template.iter().any(|a| a.contains("{content}")) {
            return Err(LaunchError::Prepare(format!(
                "emulator profile '{}' launch_args_template has no {{content}} placeholder",
                self.profile_id
            )));
        }
        // External emulators open the content themselves — the request
        // must be path-shaped and the path must exist on disk.
        let content = std::path::Path::new(&request.content_path);
        if !(content.is_file() || content.is_dir()) {
            return Err(LaunchError::Prepare(format!(
                "content not found: {}",
                request.content_path
            )));
        }
        Ok(LaunchPrepared { request })
    }

    fn launch(&self, prepared: LaunchPrepared) -> Result<LaunchedSession, LaunchError> {
        let req = prepared.request;
        let args = expand_args(&self.args_template, &req.content_path);
        log::info!(
            "oa-shell: external launch [{}] {} {}",
            self.profile_id,
            self.binary_path.display(),
            args.join(" ")
        );
        let mut child = std::process::Command::new(&self.binary_path)
            .args(&args)
            // Working dir = the binary's folder: portable emulator
            // builds (Dolphin among them) resolve their user/config
            // dirs relative to it.
            .current_dir(self.binary_path.parent().unwrap_or_else(|| std::path::Path::new(".")))
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                LaunchError::Launch(format!("spawn {}: {e}", self.binary_path.display()))
            })?;
        let pid = child.id();
        if let Some(out) = child.stdout.take() {
            Self::pump_stream(self.profile_id.clone(), "stdout", out);
        }
        if let Some(err) = child.stderr.take() {
            Self::pump_stream(self.profile_id.clone(), "stderr", err);
        }
        *self
            .child
            .lock()
            .map_err(|_| LaunchError::Launch("child slot poisoned".to_string()))? =
            Some(child);
        log::info!("oa-shell: external session started [{}] pid {pid}", self.profile_id);
        Ok(LaunchedSession::External { pid })
    }

    fn is_alive(&self, session: &LaunchedSession) -> bool {
        let LaunchedSession::External { pid } = session else {
            return false;
        };
        let mut guard = match self.child.lock() {
            Ok(g) => g,
            Err(_) => return false,
        };
        match guard.as_mut() {
            Some(child) if child.id() == *pid => match child.try_wait() {
                Ok(None) => true,
                // Exited (or wait failed) — reap and clear the slot so
                // repeated polls stay cheap.
                _ => {
                    *guard = None;
                    false
                }
            },
            _ => false,
        }
    }

    fn terminate(
        &self,
        session: &LaunchedSession,
        _title: Option<String>,
    ) -> Result<(), LaunchError> {
        let LaunchedSession::External { pid } = session else {
            return Err(LaunchError::Terminate(
                "external launcher asked to terminate a non-external session".to_string(),
            ));
        };
        let mut guard = self
            .child
            .lock()
            .map_err(|_| LaunchError::Terminate("child slot poisoned".to_string()))?;
        let Some(child) = guard.as_mut().filter(|c| c.id() == *pid) else {
            // Already gone (natural exit reaped it) — terminate on a
            // dead session is a graceful no-op, matching the libretro
            // launcher's unload-with-no-ROM behavior.
            return Ok(());
        };
        // Graceful first (flushes the emulator's own saves), then poll
        // for the grace window, then hard kill.
        if request_graceful_close(*pid) {
            let deadline = std::time::Instant::now() + TERMINATE_GRACE;
            while std::time::Instant::now() < deadline {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        *guard = None;
                        log::info!(
                            "oa-shell: external session [{}] pid {pid} closed gracefully",
                            self.profile_id
                        );
                        return Ok(());
                    }
                    Ok(None) => std::thread::sleep(Duration::from_millis(100)),
                    Err(e) => {
                        return Err(LaunchError::Terminate(format!("wait pid {pid}: {e}")))
                    }
                }
            }
            log::warn!(
                "oa-shell: external session [{}] pid {pid} ignored graceful close for {}s — force-killing",
                self.profile_id,
                TERMINATE_GRACE.as_secs()
            );
        }
        child
            .kill()
            .map_err(|e| LaunchError::Terminate(format!("kill pid {pid}: {e}")))?;
        let _ = child.wait(); // reap
        *guard = None;
        log::info!("oa-shell: external session [{}] pid {pid} force-killed", self.profile_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn request() -> LaunchRequest {
        LaunchRequest {
            content_path: "G:/roms/Bonk's Adventure (USA).pce".to_string(),
            content_bytes: vec![0xAA, 0xBB, 0xCC],
            system_id: "tg16".to_string(),
            restore_slot: Some(2),
            restore_state_path: Some(PathBuf::from("G:/states/replay.oastate")),
            core_override: Some("mednafen_pce_libretro.dll".to_string()),
        }
    }

    #[test]
    fn launch_maps_request_onto_load_rom_field_for_field() {
        let (tx, rx) = mpsc::channel();
        let launcher = LibretroLauncher::new(tx);

        let prepared = launcher.prepare(request()).expect("prepare");
        let session = launcher.launch(prepared).expect("launch");
        assert_eq!(session, LaunchedSession::InProcess);

        match rx.try_recv().expect("LoadRom dispatched") {
            EmuCommand::LoadRom {
                path,
                bytes,
                restore_slot,
                restore_state_path,
                core_override,
                system_id,
            } => {
                assert_eq!(path, "G:/roms/Bonk's Adventure (USA).pce");
                assert_eq!(bytes, vec![0xAA, 0xBB, 0xCC]);
                assert_eq!(restore_slot, Some(2));
                assert_eq!(
                    restore_state_path,
                    Some(PathBuf::from("G:/states/replay.oastate"))
                );
                assert_eq!(core_override.as_deref(), Some("mednafen_pce_libretro.dll"));
                assert_eq!(system_id, "tg16");
            }
            _ => panic!("expected EmuCommand::LoadRom"),
        }
    }

    #[test]
    fn terminate_maps_onto_unload_rom_with_title() {
        let (tx, rx) = mpsc::channel();
        let launcher = LibretroLauncher::new(tx);

        launcher
            .terminate(&LaunchedSession::InProcess, Some("Bonk's Adventure".to_string()))
            .expect("terminate");

        match rx.try_recv().expect("UnloadRom dispatched") {
            EmuCommand::UnloadRom { title } => {
                assert_eq!(title.as_deref(), Some("Bonk's Adventure"));
            }
            _ => panic!("expected EmuCommand::UnloadRom"),
        }
    }

    #[test]
    fn launch_error_string_matches_pre_c1_command_text() {
        // The pre-C1 launch_rom returned "emu thread closed: {e}" when
        // the emu thread was gone. The Launcher seam must surface the
        // same text through LaunchError's transparent Display.
        let (tx, rx) = mpsc::channel();
        drop(rx);
        let launcher = LibretroLauncher::new(tx);

        let prepared = launcher.prepare(request()).expect("prepare");
        let err = launcher.launch(prepared).expect_err("channel closed");
        assert!(
            err.to_string().starts_with("emu thread closed: "),
            "got: {err}"
        );

        let err = launcher
            .terminate(&LaunchedSession::InProcess, None)
            .expect_err("channel closed");
        assert!(
            err.to_string().starts_with("emu thread closed: "),
            "got: {err}"
        );
    }

    #[test]
    fn libretro_launcher_reports_full_capabilities() {
        let (tx, _rx) = mpsc::channel();
        let launcher = LibretroLauncher::new(tx);
        assert_eq!(launcher.id(), "libretro");
        assert_eq!(launcher.capabilities(), LauncherCapabilities::all());
        assert!(launcher.is_alive(&LaunchedSession::InProcess));
    }

    // ---- ExternalProcessLauncher (Phase C2) ----

    /// A profile pointing at a real host binary so prepare/launch
    /// shaped tests can run without an emulator install. On Windows
    /// that's cmd.exe (always present); the template runs `cmd /C type
    /// {content}` — opens the content path, exits immediately.
    #[cfg(windows)]
    fn host_echo_profile() -> (EmulatorProfile, PathBuf) {
        let cmd = std::env::var("ComSpec")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(r"C:\Windows\System32\cmd.exe"));
        let profile: EmulatorProfile = serde_yaml::from_str(
            r#"
id: hostecho
display_name: Host Echo
binary_name: cmd.exe
supported_systems: [gamecube]
launch_args_template: ["/C", "type", "{content}"]
"#,
        )
        .expect("parse test profile");
        (profile, cmd)
    }

    /// A content file that exists on disk for prepare's path check.
    fn tmp_content_file(label: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oa-extlaunch-{label}-{}.bin",
            std::process::id()
        ));
        std::fs::write(&p, b"content").expect("write tmp content");
        p
    }

    fn external_request(content: &std::path::Path) -> LaunchRequest {
        LaunchRequest {
            content_path: content.to_string_lossy().into_owned(),
            content_bytes: Vec::new(),
            system_id: "gamecube".to_string(),
            restore_slot: None,
            restore_state_path: None,
            core_override: None,
        }
    }

    #[cfg(windows)]
    #[test]
    fn external_prepare_rejects_missing_binary() {
        let (profile, _) = host_echo_profile();
        let launcher = ExternalProcessLauncher::new(
            &profile,
            PathBuf::from(r"C:\definitely\not\here\Dolphin.exe"),
        );
        let content = tmp_content_file("missing-binary");
        let err = launcher
            .prepare(external_request(&content))
            .expect_err("binary missing");
        assert!(err.to_string().contains("binary not found"), "got: {err}");
        let _ = std::fs::remove_file(&content);
    }

    #[cfg(windows)]
    #[test]
    fn external_prepare_rejects_missing_content() {
        let (profile, cmd) = host_echo_profile();
        let launcher = ExternalProcessLauncher::new(&profile, cmd);
        let err = launcher
            .prepare(external_request(std::path::Path::new(
                r"C:\definitely\not\here\game.iso",
            )))
            .expect_err("content missing");
        assert!(err.to_string().contains("content not found"), "got: {err}");
    }

    #[cfg(windows)]
    #[test]
    fn external_prepare_rejects_template_without_content_placeholder() {
        let (mut profile, cmd) = host_echo_profile();
        profile.launch_args_template = vec!["/C".to_string(), "exit".to_string()];
        let launcher = ExternalProcessLauncher::new(&profile, cmd);
        let content = tmp_content_file("no-placeholder");
        let err = launcher
            .prepare(external_request(&content))
            .expect_err("template lacks {content}");
        assert!(err.to_string().contains("{content}"), "got: {err}");
        let _ = std::fs::remove_file(&content);
    }

    #[cfg(windows)]
    #[test]
    fn external_launch_runs_and_session_dies_naturally() {
        let (profile, cmd) = host_echo_profile();
        let launcher = ExternalProcessLauncher::new(&profile, cmd);
        let content = tmp_content_file("natural-exit");
        let prepared = launcher.prepare(external_request(&content)).expect("prepare");
        let session = launcher.launch(prepared).expect("launch");
        assert!(matches!(session, LaunchedSession::External { .. }));
        // `cmd /C type` exits almost immediately; poll until is_alive
        // flips false (bounded so a wedged test can't hang the suite).
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while launcher.is_alive(&session) && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(!launcher.is_alive(&session), "child should have exited");
        // Terminate after natural exit is a graceful no-op.
        launcher.terminate(&session, None).expect("terminate on dead session");
        let _ = std::fs::remove_file(&content);
    }

    #[cfg(windows)]
    #[test]
    fn external_terminate_kills_long_running_child() {
        let (mut profile, cmd) = host_echo_profile();
        // 30 pings ≈ 30s — far beyond the test, so only terminate ends it.
        profile.launch_args_template = vec![
            "/C".to_string(),
            "ping -n 30 127.0.0.1 > nul & type {content}".to_string(),
        ];
        let launcher = ExternalProcessLauncher::new(&profile, cmd);
        let content = tmp_content_file("terminate");
        let prepared = launcher.prepare(external_request(&content)).expect("prepare");
        let session = launcher.launch(prepared).expect("launch");
        assert!(launcher.is_alive(&session), "child should be running");
        launcher.terminate(&session, None).expect("terminate");
        assert!(!launcher.is_alive(&session), "child should be gone after terminate");
        let _ = std::fs::remove_file(&content);
    }

    #[test]
    fn external_launcher_reports_profile_capabilities_and_id() {
        let profile: EmulatorProfile = serde_yaml::from_str(
            r#"
id: dolphin
display_name: Dolphin
binary_name: Dolphin.exe
supported_systems: [gamecube]
launch_args_template: ["--batch", "--exec={content}"]
"#,
        )
        .expect("parse");
        let launcher = ExternalProcessLauncher::new(&profile, PathBuf::from("Dolphin.exe"));
        assert_eq!(launcher.id(), "dolphin");
        assert_eq!(launcher.display_name(), "Dolphin");
        assert_eq!(launcher.capabilities(), LauncherCapabilities::none());
        // External launchers never own an in-process session.
        assert!(!launcher.is_alive(&LaunchedSession::InProcess));
    }
}
