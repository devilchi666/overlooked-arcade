# Portable Install — Session Log

Entries for the portable-install work. Three lines per entry:
**Shipped / Almost / Next**.

---

## 2026-05-23 — Portable install MVP (4-phase ship)

Operator asked to make the program portable: settings next to the
.exe, same model as the already-portable cores/ and system/ folders.
Marker-file opt-in (`portable.txt`); same binary works in both
modes; AppData → portable auto-migration on first portable launch.

Audit confirmed the change was small: 13 of 14 subsystems already
take `app_data_dir: PathBuf` as a parameter, so no subsystem code
needed touching. Single Tauri entry point (`main.rs::setup`) +
single CLI entry point (`cli.rs::run_clear_metadata_headless`)
+ one asset-protocol scope extension + 3 frontend asset-URL
sites = total change surface.

- **Shipped:** 4 phase commits on `feat/portable-install`:
  - **Phase 1 (`f8f50b7`)** — `apps/oa-shell/src/data_dir.rs`
    resolver. Detects `portable.txt` next to `current_exe()`,
    returns `<exe_dir>/settings/` if present, else falls back to
    Tauri's `app_data_dir()` (GUI) or per-OS env-var resolution
    (CLI). `DataDir { path, portable }` carries the mode bit
    forward. Replaces the duplicated `resolve_app_data_dir` in
    cli.rs and the inline `app.path().app_data_dir()` in
    main.rs::setup. Loud error + AppData fallback if
    `<exe_dir>/settings/` isn't writable (e.g. Program Files).
    3 unit tests.

  - **Phase 2 (`beb7399`)** — `app.asset_protocol_scope().allow_directory(&data_dir, true)`
    runtime call in portable mode (Tauri 2.11.1 API,
    verified in `tauri-2.11.1/src/scope/fs.rs:278`). Compile-time
    `$APPDATA/**` scope in tauri.conf.json stays as-is. New
    `get_oa_data_dir` Tauri command + `frontend/src/lib/dataDir.ts`
    helper with a cached Promise. Swapped 3 frontend asset-URL
    sites (media.tsx, GameInfoModal, RegionPicker) off
    `appDataDir()` to `getDataDir()`. `appDataDir()` always
    returns the OS AppData path regardless of mode, which would
    404 cover art in portable mode.

  - **Phase 3 (`2edbe17`)** — `migrate_from_appdata_if_needed`.
    Recursive std::fs copy (no walkdir dep — one-shot operation
    with small file counts). Sentinel `.migrated-from-appdata`
    written after success so the operator can empty the portable
    dir later without re-overwriting. Errors logged but never
    propagated (migration failure must not block app launch).
    AppData left intact; operator deletes manually once verified.
    3 more unit tests.

  - **Phase 4** — this entry + `docs/features/portable-install/`
    folder + CLAUDE.md "Debugging" section updated to note the
    log path varies by mode (with the startup line
    `oa-shell: data dir = … (portable|appdata)` as the
    confirmation surface).

- **Almost:** N/A — `cargo test --workspace` is 355/355 (was
  349; +6 from this feature). `cargo check -p oa-shell` clean.
  `npm run typecheck` (frontend) clean. End-to-end runtime
  verification (launching portable vs non-portable, watching for
  cover art rendering + log path + save-state writes) is the
  next step operator runs locally.

- **Next:** Operator runtime test of the three scenarios in the
  plan's verification section (`plans/groovy-enchanting-candle.md`):
  (1) non-portable regression, (2) portable fresh install,
  (3) portable migration from existing AppData. On green-light:
  merge `feat/portable-install` `--no-ff` to main, delete branch
  both sides.

  Followup items (not blocking this PR):
  - Decide whether to ship a `portable.txt` in the GitHub release
    ZIP by default (makes the ZIP "portable by default", the
    installer "non-portable by default").
  - If/when an MSI installer is built, ensure it doesn't ship
    `portable.txt` (so installer-installs default to AppData).
  - Sidebar / Settings UI affordance to surface the current mode
    (parking-lot candidate — the marker-file IS the affordance
    today).
