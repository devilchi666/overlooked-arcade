//! Core installer — buildbot.libretro.com nightly catalog browser + downloader.
//!
//! Power users can drop libretro `.dll` / `.so` / `.dylib` files into
//! `<exe_dir>/cores/` by hand; this module turns that into a click. We
//! ship a curated catalog of ~30 popular cores covering the first-wave
//! Overlooked Arcade systems plus a handful of common Genesis / SNES /
//! NES alternates, fetch them on demand from the libretro nightly
//! buildbot, and write the resulting `.dll` (or `.so` / `.dylib` on
//! other hosts) straight into the cores folder.
//!
//! ## URL shape
//!
//! ```text
//! https://buildbot.libretro.com/nightly/<host>/<arch>/latest/<base>_libretro.<ext>.zip
//! ```
//!
//! - **windows / x86_64** → `windows/x86_64/`
//! - **linux / x86_64**   → `linux/x86_64/`
//! - **macos / x86_64**   → `apple/osx/x86_64/`
//! - **macos / arm64**    → `apple/osx/arm64/`
//!
//! Each zip contains exactly one `.dll`/`.so`/`.dylib` at the top level.
//! After extraction we run `oa_libretro::probe` against the new file to
//! validate the libretro ABI before reporting success — if it doesn't
//! probe, we delete the half-installed file and surface the error.
//!
//! ## Progress events
//!
//! Long-running downloads emit `oa://core-download-progress` with the
//! payload below so the UI can render a progress bar:
//!
//! ```text
//! { fileName, downloadedBytes, totalBytes, phase: "downloading" | "extracting" | "done" | "error" }
//! ```
//!
//! `totalBytes` may be `null` if the server didn't send a Content-Length.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// One curated catalog entry. Cross-platform: `base` is the buildbot
/// basename (no extension) and `systems` lists the Overlooked Arcade
/// `SystemId`s the core is appropriate for. The host-specific filename
/// is derived at runtime via [`core_filename_for_host`].
pub struct CatalogEntry {
    pub base: &'static str,
    pub display_name: &'static str,
    /// Short blurb shown under the title — what the core is, what's
    /// special about it.
    pub blurb: &'static str,
    /// OA system slugs this core can drive. Empty = no first-wave OA
    /// system matches today (still installable; useful for power users
    /// who add a new SystemId in their fork).
    pub systems: &'static [&'static str],
}

/// Curated catalog of cores we expose in the installer. Includes the
/// first-wave OA systems plus a handful of mainstream alternates that
/// libretro power users typically expect to see (NES / SNES / Genesis /
/// PSX / N64). Adding a core here is a one-line edit; the runtime
/// derives `<base>_libretro.<ext>` for the host's platform automatically.
pub const CATALOG: &[CatalogEntry] = &[
    // -------- TG-16 / PC Engine family --------
    CatalogEntry {
        base: "mednafen_pce_fast_libretro",
        display_name: "Beetle PCE Fast",
        blurb: "Mednafen-derived. Fast, plays HuCard + CD. OA default.",
        systems: &["tg16", "pce-cd"],
    },
    CatalogEntry {
        base: "mednafen_pce_libretro",
        display_name: "Beetle PCE (full Mednafen)",
        blurb: "Higher-accuracy alternative. Heavier, same library coverage.",
        systems: &["tg16", "pce-cd"],
    },
    CatalogEntry {
        base: "mednafen_supergrafx_libretro",
        display_name: "Beetle SuperGrafx",
        blurb: "SGX-only — the four SuperGrafx-enhanced HuCards.",
        systems: &["tg16"],
    },
    // -------- Lynx --------
    CatalogEntry {
        base: "mednafen_lynx_libretro",
        display_name: "Beetle Lynx",
        blurb: "Atari Lynx. Needs lynxboot.img in <exe_dir>/system/.",
        systems: &["lynx"],
    },
    CatalogEntry {
        base: "handy_libretro",
        display_name: "Handy",
        blurb: "Older Lynx core, slightly different audio character.",
        systems: &["lynx"],
    },
    // -------- NES / Famicom --------
    CatalogEntry {
        base: "fceumm_libretro",
        display_name: "FCEUmm",
        blurb: "FCEU + community mapper updates. Wide mapper coverage.",
        systems: &["nes"],
    },
    CatalogEntry {
        base: "mesen_libretro",
        display_name: "Mesen",
        blurb: "Higher-accuracy NES. Cycle-stepped PPU.",
        systems: &["nes"],
    },
    CatalogEntry {
        base: "nestopia_libretro",
        display_name: "Nestopia UE",
        blurb: "Long-running cycle-accurate NES core.",
        systems: &["nes"],
    },
    // -------- SNES --------
    CatalogEntry {
        base: "snes9x_libretro",
        display_name: "Snes9x",
        blurb: "Standard SNES core. Fast, broad compatibility.",
        systems: &["snes"],
    },
    CatalogEntry {
        base: "bsnes_libretro",
        display_name: "bsnes",
        blurb: "Higher accuracy SNES (byuu-derived).",
        systems: &["snes"],
    },
    CatalogEntry {
        base: "mesen-s_libretro",
        display_name: "Mesen-S",
        blurb: "Mesen author's SNES core, accuracy-focused.",
        systems: &["snes"],
    },
    // -------- Sega Master System / Game Gear / Genesis (queued systems) --------
    CatalogEntry {
        base: "genesis_plus_gx_libretro",
        display_name: "Genesis Plus GX",
        blurb: "SMS / GG / Mega Drive / Sega CD — the standard pick.",
        systems: &[],
    },
    CatalogEntry {
        base: "picodrive_libretro",
        display_name: "PicoDrive",
        blurb: "Lighter Mega Drive / 32X / Sega CD.",
        systems: &[],
    },
    CatalogEntry {
        base: "smsplus_libretro",
        display_name: "SMS Plus GX",
        blurb: "SMS / Game Gear-only alternative.",
        systems: &[],
    },
    // -------- Atari 7800 --------
    CatalogEntry {
        base: "prosystem_libretro",
        display_name: "ProSystem",
        blurb: "Atari 7800 core. ProSystem-derived.",
        systems: &[],
    },
    // -------- MSX / MSX2 --------
    CatalogEntry {
        base: "bluemsx_libretro",
        display_name: "blueMSX",
        blurb: "MSX / MSX2 / MSX Turbo R. Broad coverage.",
        systems: &[],
    },
    CatalogEntry {
        base: "fmsx_libretro",
        display_name: "fMSX",
        blurb: "Lighter MSX alternative.",
        systems: &[],
    },
    // -------- ColecoVision --------
    CatalogEntry {
        base: "gearcoleco_libretro",
        display_name: "GearColeco",
        blurb: "ColecoVision. Needs colecovision.col BIOS.",
        systems: &[],
    },
    // -------- Vectrex --------
    CatalogEntry {
        base: "vecx_libretro",
        display_name: "Vecx",
        blurb: "Vectrex with optional overlay support.",
        systems: &[],
    },
    // -------- Virtual Boy --------
    CatalogEntry {
        base: "mednafen_vb_libretro",
        display_name: "Beetle VB",
        blurb: "Mednafen Virtual Boy. Anaglyph or split-screen 3D.",
        systems: &[],
    },
    // -------- WonderSwan --------
    CatalogEntry {
        base: "mednafen_wswan_libretro",
        display_name: "Beetle WonderSwan",
        blurb: "WonderSwan + WonderSwan Color.",
        systems: &[],
    },
    // -------- Mainstream alternates (broad libretro audience) --------
    CatalogEntry {
        base: "mgba_libretro",
        display_name: "mGBA",
        blurb: "Game Boy / GB Color / GB Advance — the standard pick.",
        systems: &[],
    },
    CatalogEntry {
        base: "gambatte_libretro",
        display_name: "Gambatte",
        blurb: "GB / GBC-only, very high accuracy.",
        systems: &[],
    },
    CatalogEntry {
        base: "mednafen_psx_hw_libretro",
        display_name: "Beetle PSX HW",
        blurb: "Sony PlayStation, HW renderer. Needs PSX BIOS.",
        systems: &[],
    },
    CatalogEntry {
        base: "swanstation_libretro",
        display_name: "SwanStation",
        blurb: "Faster PSX, fork of DuckStation. Needs PSX BIOS.",
        systems: &[],
    },
    CatalogEntry {
        base: "mupen64plus_next_libretro",
        display_name: "Mupen64Plus-Next",
        blurb: "N64 — GLideN64-based renderer.",
        systems: &[],
    },
    CatalogEntry {
        base: "stella_libretro",
        display_name: "Stella",
        blurb: "Atari 2600.",
        systems: &[],
    },
    CatalogEntry {
        base: "neocd_libretro",
        display_name: "NeoCD",
        blurb: "Neo Geo CD. Needs Neo Geo CD BIOS.",
        systems: &[],
    },
    CatalogEntry {
        base: "fbneo_libretro",
        display_name: "FinalBurn Neo",
        blurb: "Arcade — Neo Geo, CPS1/2/3, Cave, more.",
        systems: &[],
    },
];

/// The platform-specific dynamic-library extension for the running host.
/// Mirrors `core_extension_for_host` in main.rs — kept duplicated here so
/// the installer module doesn't import shell-private helpers.
fn dylib_ext() -> &'static str {
    if cfg!(windows)          { "dll"   }
    else if cfg!(target_os = "macos") { "dylib" }
    else                              { "so"    }
}

/// Render the on-disk filename for a catalog entry on the current host
/// (e.g. `mednafen_pce_fast_libretro.dll` on Windows).
fn core_filename_for_host(base: &str) -> String {
    format!("{base}.{}", dylib_ext())
}

/// Resolve the buildbot URL prefix for the running host + arch. Returns
/// `None` for unsupported combinations (the catalog still renders; the
/// install button just refuses with a clean error).
fn buildbot_path_segment() -> Option<&'static str> {
    let arch = std::env::consts::ARCH;
    if cfg!(target_os = "windows") && arch == "x86_64" {
        Some("windows/x86_64")
    } else if cfg!(target_os = "linux") && arch == "x86_64" {
        Some("linux/x86_64")
    } else if cfg!(target_os = "macos") && arch == "x86_64" {
        Some("apple/osx/x86_64")
    } else if cfg!(target_os = "macos") && (arch == "aarch64" || arch == "arm64") {
        Some("apple/osx/arm64")
    } else {
        None
    }
}

/// Final buildbot URL for a catalog `base`. None when the platform isn't
/// in [`buildbot_path_segment`].
fn buildbot_url_for(base: &str) -> Option<String> {
    let segment = buildbot_path_segment()?;
    let ext = dylib_ext();
    Some(format!(
        "https://buildbot.libretro.com/nightly/{segment}/latest/{base}_libretro.{ext}.zip",
    ))
}

/// Frontend-visible catalog entry. Combines a [`CatalogEntry`] with
/// installed-state info so the UI can render "Install" vs
/// "Update (current v1.2.3)" in one pass.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AvailableCore {
    pub base: String,
    pub file_name: String,
    pub display_name: String,
    pub blurb: String,
    pub systems: Vec<String>,
    pub installed: bool,
    /// Installed library version (from `retro_get_system_info`), if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_version: Option<String>,
    /// Whether the buildbot supports the current OS+ARCH at all. When
    /// false, the UI greys out the install button.
    pub supported_on_host: bool,
    /// The URL the installer will hit. Surfaced for diagnostics only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buildbot_url: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct CoreDownloadProgress<'a> {
    file_name: &'a str,
    downloaded_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_bytes: Option<u64>,
    /// `"downloading"`, `"extracting"`, `"done"`, `"error"`.
    phase: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<&'a str>,
}

fn emit_progress(
    app: &AppHandle,
    file_name: &str,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    phase: &str,
    message: Option<&str>,
) {
    let payload = CoreDownloadProgress {
        file_name,
        downloaded_bytes,
        total_bytes,
        phase,
        message,
    };
    if let Err(e) = app.emit("oa://core-download-progress", payload) {
        log::warn!("core_installer: emit progress failed: {e:?}");
    }
}

/// Walk `<exe_dir>/cores/` and probe every dylib. Returns a filename →
/// (version, valid_extensions) map so [`available_cores`] can stamp
/// "installed (v…)" chips.
fn installed_index(cores_dir: &Path) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    let Ok(rd) = std::fs::read_dir(cores_dir) else { return out };
    let ext = dylib_ext();
    for entry in rd.flatten() {
        let p = entry.path();
        if !p.is_file() { continue; }
        if p.extension().and_then(|s| s.to_str()) != Some(ext) { continue; }
        let Some(name) = p.file_name().and_then(|s| s.to_str()) else { continue };
        match oa_libretro::probe(&p) {
            Ok(info) => { out.insert(name.to_string(), info.library_version); }
            Err(_) => { out.insert(name.to_string(), String::new()); }
        }
    }
    out
}

/// Catalog merged with installed-state. The frontend renders this
/// directly — one row per catalog entry, with Install / Update buttons
/// driven by `installed` + `installed_version`.
#[tauri::command]
pub fn available_cores(cores_dir: tauri::State<'_, CoresDir>) -> Vec<AvailableCore> {
    let installed = installed_index(&cores_dir.0);
    let host_supported = buildbot_path_segment().is_some();
    CATALOG
        .iter()
        .map(|c| {
            let file_name = core_filename_for_host(c.base);
            let installed_version = installed.get(&file_name).cloned();
            AvailableCore {
                base: c.base.to_string(),
                file_name: file_name.clone(),
                display_name: c.display_name.to_string(),
                blurb: c.blurb.to_string(),
                systems: c.systems.iter().map(|s| s.to_string()).collect(),
                installed: installed.contains_key(&file_name),
                installed_version: installed_version.filter(|v| !v.is_empty()),
                supported_on_host: host_supported,
                buildbot_url: buildbot_url_for(c.base),
            }
        })
        .collect()
}

/// Tauri-managed state holder so the commands don't have to call
/// `resolve_cores_dir` on every invoke. Set up once at app start.
pub struct CoresDir(pub PathBuf);

/// Download the buildbot nightly zip for `base`, extract the single
/// dylib inside, validate it via libretro probe, and place it at
/// `<exe_dir>/cores/<base>.<ext>`. Emits `oa://core-download-progress`
/// throughout. If a file with the same name already exists, it gets
/// overwritten (Update path).
#[tauri::command]
pub async fn download_core(
    base: String,
    cores_dir: tauri::State<'_, CoresDir>,
    app: AppHandle,
) -> Result<String, String> {
    let dest_dir = cores_dir.0.clone();
    let url = buildbot_url_for(&base)
        .ok_or_else(|| format!(
            "buildbot has no build for this OS/ARCH ({}/{})",
            std::env::consts::OS, std::env::consts::ARCH,
        ))?;
    let file_name = core_filename_for_host(&base);
    let final_path = dest_dir.join(&file_name);

    if let Err(e) = std::fs::create_dir_all(&dest_dir) {
        return Err(format!("create cores dir {}: {e}", dest_dir.display()));
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|e| format!("reqwest client: {e}"))?;

    emit_progress(&app, &file_name, 0, None, "downloading", None);

    let resp = client
        .get(&url)
        .header("User-Agent", "OverlookedArcade")
        .send()
        .await
        .map_err(|e| {
            emit_progress(&app, &file_name, 0, None, "error", Some(&format!("{e}")));
            format!("GET {url}: {e}")
        })?;
    if !resp.status().is_success() {
        let status = resp.status();
        emit_progress(&app, &file_name, 0, None, "error", Some(&format!("HTTP {status}")));
        return Err(format!("buildbot HTTP {status} for {url}"));
    }
    let total = resp.content_length();

    // Stream the body to memory — the zips are small (typically <10 MB)
    // so we avoid the tempfile dance. If we ever ship something where
    // 10s of MB matters, switch to a `tokio::fs::File` writer.
    use futures::StreamExt;
    let mut downloaded: u64 = 0;
    let mut zip_bytes: Vec<u8> = Vec::with_capacity(total.unwrap_or(0) as usize);
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| {
            emit_progress(&app, &file_name, downloaded, total, "error", Some(&format!("{e}")));
            format!("download chunk: {e}")
        })?;
        zip_bytes.extend_from_slice(&chunk);
        downloaded += chunk.len() as u64;
        emit_progress(&app, &file_name, downloaded, total, "downloading", None);
    }

    emit_progress(&app, &file_name, downloaded, total, "extracting", None);

    // Extract: every libretro buildbot zip contains exactly one dylib
    // at the top level. We pick the first entry whose extension matches
    // our host's dylib extension and write it through to a `.partial`
    // file so a failed validation doesn't clobber an existing good copy.
    let ext = dylib_ext();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&zip_bytes))
        .map_err(|e| {
            emit_progress(&app, &file_name, downloaded, total, "error", Some(&format!("{e}")));
            format!("open zip: {e}")
        })?;
    let mut payload: Option<Vec<u8>> = None;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)
            .map_err(|e| format!("read zip entry {i}: {e}"))?;
        if !entry.is_file() { continue; }
        let name = entry.name().to_string();
        if !name.to_ascii_lowercase().ends_with(&format!(".{ext}")) { continue; }
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf)
            .map_err(|e| format!("read zip body {name}: {e}"))?;
        payload = Some(buf);
        break;
    }
    let Some(payload) = payload else {
        emit_progress(&app, &file_name, downloaded, total, "error", Some("zip had no .{ext} entry"));
        return Err(format!("zip {url} contained no .{ext} entry"));
    };

    let partial_path = final_path.with_extension(format!("{ext}.partial"));
    std::fs::write(&partial_path, &payload).map_err(|e| {
        emit_progress(&app, &file_name, downloaded, total, "error", Some(&format!("{e}")));
        format!("write {}: {e}", partial_path.display())
    })?;

    // Validate the just-written file before renaming over the live one.
    // A bogus .dll surfaces as a probe error, the .partial gets cleaned
    // up, and the previous-good install is left intact.
    if let Err(e) = oa_libretro::probe(&partial_path) {
        let _ = std::fs::remove_file(&partial_path);
        emit_progress(&app, &file_name, downloaded, total, "error", Some(&format!("probe: {e}")));
        return Err(format!("downloaded file failed libretro probe: {e}"));
    }

    // Replace the existing core, if any. On Windows a currently-loaded
    // DLL can't be overwritten — surface that cleanly. On any error,
    // keep the .partial around so the user can retry post-restart.
    if final_path.exists() {
        if let Err(e) = std::fs::remove_file(&final_path) {
            emit_progress(
                &app, &file_name, downloaded, total, "error",
                Some("existing core in use; restart Overlooked Arcade and retry"),
            );
            return Err(format!(
                "remove existing {}: {e} (likely still loaded by the running process; restart and retry)",
                final_path.display(),
            ));
        }
    }
    if let Err(e) = std::fs::rename(&partial_path, &final_path) {
        let _ = std::fs::remove_file(&partial_path);
        emit_progress(&app, &file_name, downloaded, total, "error", Some(&format!("{e}")));
        return Err(format!("rename to {}: {e}", final_path.display()));
    }

    emit_progress(&app, &file_name, downloaded, total, "done", None);
    log::info!("core_installer: installed {} from {url}", final_path.display());
    Ok(final_path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buildbot_path_segment_for_current_host() {
        // Just ensure the resolver returns something on the platforms we
        // actually develop on. CI matrix already runs Windows + Linux +
        // macOS (both archs).
        let seg = buildbot_path_segment();
        if cfg!(any(target_os = "windows", target_os = "linux", target_os = "macos")) {
            assert!(seg.is_some(), "no buildbot mapping for {} {}", std::env::consts::OS, std::env::consts::ARCH);
        } else {
            // Other OSes are allowed to be unsupported.
            let _ = seg;
        }
    }

    #[test]
    fn url_for_known_core_contains_buildbot_host() {
        // Skip when the host isn't on the buildbot at all.
        if buildbot_path_segment().is_none() { return; }
        let url = buildbot_url_for("mednafen_pce_fast_libretro").unwrap();
        assert!(url.starts_with("https://buildbot.libretro.com/"), "{url}");
        assert!(url.contains("mednafen_pce_fast_libretro"), "{url}");
        assert!(url.ends_with(".zip"), "{url}");
    }

    #[test]
    fn catalog_has_no_duplicate_bases() {
        let mut seen = std::collections::HashSet::new();
        for entry in CATALOG {
            assert!(seen.insert(entry.base), "duplicate base {}", entry.base);
        }
    }
}
