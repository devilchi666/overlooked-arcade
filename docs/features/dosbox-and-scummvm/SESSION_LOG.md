# dosbox-and-scummvm Session Log

Cross-stream session log. Per-core ship details + commit shas live in the per-core SESSION_LOGs at `docs/cores/scummvm/SESSION_LOG.md` + `docs/cores/dosbox/SESSION_LOG.md`.

---

## 2026-05-24 (later×2) — ScummVM CLI mode added as a power-user option

- **Shipped:** Branch `feat/scummvm-cli-option`. Closes the "option A (CLI shell-out)" half of the original auto-detect parking-lot entry — both options now ship in the same dialog as a radio toggle. New backend module `apps/oa-shell/src/scummvm_cli.rs` (~350 lines including tests) handles: per-platform auto-discovery of `scummvm.exe` (Windows Program Files variants, macOS `.app` bundle, Unix `/usr/(local/)bin`, then `$PATH` walk); invocation via `scummvm --detect --recursive --path=<dir>` on tokio's blocking pool so the renderer doesn't stall; defensive line-by-line stdout parser that handles modern 2.x column-aligned format + variant widths (skips header, dashes, unparseable lines without aborting the batch); cross-platform path parsing (Unix `/foo/bar` + Windows `C:\foo\bar` + `D:/foo`). Two new Tauri commands: `find_scummvm_cli` (auto-discovery, called on dialog open to pre-fill) + `run_scummvm_cli_detect` (the actual scan). 10 new unit tests cover the parser shapes + path classification + edge cases.
- **Dialog updates:** Mode toggle radio at the top — "Built-in table" (default) vs "Standalone ScummVM CLI". CLI mode shows the resolved executable path inline with a "Change" button + amber "Not found" hint when no install was discovered. Scan dispatches based on selected mode: table mode runs only the curated walker; CLI mode runs the walker (for canonical subdir + existing-descriptor list) AND the CLI, then merges CLI matches into the directory rows by case-insensitive path key. Descriptor validator loosened to accept bare gameids (`monkey`) in addition to explicit `gameid:engine` (`monkey:scumm`) since CLI rows write bare ids that the libretro core resolves internally.
- **Almost:** Operator playtest with a standalone ScummVM install — drop a game directory it knows, switch to CLI mode, scan, verify it lands on the right gameid.
- **Next:** Both detection paths are shipped + tested in CI; operator validation is the only remaining gate for the per-core scummvm Phase 1 entries to flip ✅.

cargo test workspace green (469 oa-shell tests, +10 new scummvm_cli tests). Frontend tsc clean.

---

## 2026-05-24 (later) — ScummVM auto-detect follow-up

- **Shipped:** Branch `feat/scummvm-auto-detect`. Closes the PARKING_LOT entry "ScummVM `--detect` auto-generation of `.scummvm` files" via option B (curated sentinel-filename heuristic; no `scummvm.exe` dependency). New backend module `apps/oa-shell/src/scummvm_detect.rs` ships a table of ~18 well-known games (every SCUMM mainstay from Monkey Island through Curse of Monkey Island + the ScummVM freeware classics Beneath a Steel Sky / Flight of the Amazon Queen / Lure of the Temptress / Drascula / Soltys). Two new Tauri commands: `detect_scummvm_directories` (read-only scan) + `write_scummvm_descriptors` (operator-confirmed batch write). New frontend `ScummvmDetectDialog` opens from a banner in Import Wizard Step 2 (visible when a scummvm rule is active); operator picks the parent folder, sees per-subdir detection results, edits or fills in misses, clicks "Write N descriptors" to land the `.scummvm` files. After write the operator advances to Step 3 and the regular extension scan picks up the new descriptors.
- **Almost:** Same operator playtest gate as Phase 1 — drop `scummvm_libretro.dll` + a real game directory, run detect, launch.
- **Next:** With auto-detect shipped, the per-core ScummVM Phase 1 validation backlog is now significantly cheaper (operator points OA at a folder, gets descriptors generated automatically for the popular games). Phase 2 hardening items (per-game `scummvm_extra_path` core option, per-game core-options templates) remain ⬜.

cargo test workspace green (459 oa-shell tests, +9 new scummvm_detect tests). Frontend tsc clean.

---

## 2026-05-24 — Phase 1 + Phase 2 shipped, merged to main

The locked plan in this folder's `README.md` was implemented across two `--no-ff` merges, one per half. Operator opted to merge each phase as it landed rather than do a single end-of-branch merge (changed the original plan's "merge once after all 5 phases" to "merge per phase"). Final cleanup phase (this entry + the doc cross-references) follows as a tiny separate merge.

- **Shipped:**
  - **Phase 1 — scummvm onboarding** (branch `feat/dosbox-and-scummvm`, merged `0b56bd8`).
    Engine-launcher core wired into OA's stock system model. ScummVM ships as an ordinary OA system alongside consoles; a "game" is a tiny `.scummvm` descriptor file (`gameid:engineid`) that the libretro core opens to load game data from the same directory. New `is_descriptor_extension` helper + new `system_dir_for` helper (engine-launcher cores get per-core subdirectory under `<exe_dir>/system/`). 15 files, +461/-14.
  - **Phase 2 — dosbox onboarding** (branch `feat/dosbox-onboarding`, merged `b6fea2c`).
    DOSBox ships as an ordinary OA system alongside consoles; a "game" is a directory containing the game's executable + data files. New `is_directory_path_system` helper (parallels `is_descriptor_extension`). New `scan_service::run_dir_scan_blocking` directory-mode walker + `start_background_directory_scan` Tauri command. New `GameOverrides.dosbox_entry_point: Option<String>` per-game override. Import Wizard dual-mode scan dispatch + `systemHint`-aware classification. 17 files, +793/-57.
  - **Phase 3 / 4 / 5 — collapsed into per-phase cross-cutting** because most of the original Phase 3 work (art-pack mappings, rom_hashes empty refs, tests) naturally grouped with the per-system phases. Phase 4 (`VISION.md` / `NEXT.md` / `ACTIVE_WORK.md` updates) + Phase 5 (this SESSION_LOG + final merge) landed as a single tail-end `feat/dosbox-and-scummvm-cleanup` branch.

  Cross-system fixtures in `bindings.rs` extended for both new systems (default_keys_round_trip, dpad_lands, z_is_primary, to_libretro_bits_dispatches). Three new tests per system in `bindings.rs` + one in `art_pack_importer.rs` per system. `cargo test --workspace` ends green: 504 → 504 (Phase 1) → 507 (Phase 2; +3 dosbox tests). Frontend `tsc --noEmit` clean throughout.

- **Almost:** Operator playtest with real `scummvm_libretro.dll` + `dosbox_pure_libretro.dll` + actual game data on disk. Operator's playtest is gated on having game data — ScummVM playtest can use freeware (Beneath a Steel Sky from scummvm.org/games) for a zero-licensing-concerns end-to-end validation. DOS playtest defers until operator has DOS games on hand. Per-core ROADMAP Phase 1 entries flip ✅ when playtest validates each.

- **Next:** When playtest lands, flip per-core ROADMAP Phase 1 entries to ✅. Engine-launcher auto-detect UX (wrapping ScummVM's `--detect` CLI to auto-generate `.scummvm` descriptors so operators don't hand-create them) is a separate follow-up that lives in `docs/PARKING_LOT.md` — explicitly deferred from this scope so the libretro-core wiring landed first.
