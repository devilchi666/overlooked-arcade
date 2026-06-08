//! Launcher implementations — virtual-library Phase C
//! (docs/PLANS/launcher-abstraction.md).
//!
//! [`LibretroLauncher`] wraps the in-process libretro pipeline behind
//! the `oa_core::Launcher` lifecycle contract: `launch` maps onto
//! `EmuCommand::LoadRom`, `terminate` onto `EmuCommand::UnloadRom` —
//! the exact dispatches `launch_rom` / `unload_rom` performed inline
//! before Phase C1, including the error strings. Phase C2 adds
//! `ExternalProcessLauncher` beside it.

use std::sync::{mpsc, Mutex};

use oa_core::{
    LaunchError, LaunchPrepared, LaunchRequest, LaunchedSession, Launcher, LauncherCapabilities,
};

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
}
