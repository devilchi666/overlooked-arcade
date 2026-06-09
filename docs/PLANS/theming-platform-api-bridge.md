# Theming Phase 4 — typed `platform/api/` Tauri bridge

**Status:** queued (planned 2026-06-09). Slice 1 is in `docs/NEXT.md` HIGH
band. This is the **last** platform/theme decoupling step — after it the
layers are isolated at both the file level (six enforced lint zones, done) AND
the API level (no component/theme binds to a backend command name).

Owning feature: [features/theming-substrate/](../features/theming-substrate/).
Predecessor (done + merged): the grab-bag drain
([PLANS/theming-grabbag-drain.md](theming-grabbag-drain.md)) — `src/components/`
is gone; every file is in `platform/`, `engine/`, or `routes/`.

## The coupling this closes

Files everywhere call Tauri backend commands directly by string name:

```ts
const games = await invoke<RomEntry[]>("list_games");
await invoke("set_window_mode", { mode });
```

Fresh census (2026-06-09, `frontend/src/`):

- **351 `invoke()` call sites** across **54 files**, hitting **222 distinct
  command names**.
- Top offenders: `App.tsx` (38), `platform/components/QuickSettings.tsx` (37),
  `platform/components/GameDialogs.tsx` (22), `engine/SettingsSections.tsx`
  (21), `platform/library/store.ts` (18), `platform/settings/store.ts` (16),
  `engine/ImportWizard.tsx` (16), `platform/library/media.tsx` (14).
- `platform/api/` does not exist yet.

> The earlier estimate in SURFACES.md / the substrate plan ("157 calls / 37
> files") was stale — the real surface is ~2× bigger. This plan supersedes
> that number.

The problem: a theme's QuickSettings overlay (37 raw invokes) is hard-wired to
the exact backend command names + payload shapes. Rename a command or change a
payload and every caller silently breaks at runtime; a theme author has no
typed, discoverable surface to build against. The directory boundary can't
catch this — `invoke("set_window_mode", …)` is a string, not an import.

## The goal

1. A typed wrapper layer `platform/api/<domain>Api.ts` — one named, typed
   function per command. Every `invoke()` call moves behind it.
2. An ESLint rule banning raw `invoke()` (and the `@tauri-apps/api/core`
   `invoke` import) **outside `platform/api/`** — the ratchet that keeps it
   closed. Turned on in the final slice, when the count hits zero.

After this, the platform/engine/theme separation is complete: a theme reaches
the backend only through `@oa/platform/api/*`, a typed surface the backend
contract is pinned to in one place.

## The wrapper convention (lock this in Slice 1)

Wrappers are **thin typed pass-throughs. No behavior change.** Phase 4 is
decoupling, not an error-handling refactor.

```ts
// platform/api/settingsApi.ts
import { invoke } from "@tauri-apps/api/core";
import type { VideoState, WindowMode } from "@oa/platform/settings/store";

/// GET the current renderer AV state.
export function getVideoState(): Promise<VideoState> {
  return invoke<VideoState>("get_video_state");
}

/// Set the shell window mode (windowed / borderless / fullscreen).
export function setWindowMode(mode: WindowMode): Promise<void> {
  return invoke("set_window_mode", { mode });
}
```

Rules:

- **One named export per command**, camelCase of the command name
  (`set_window_mode` → `setWindowMode`). Named exports, not a default object —
  tree-shakeable + greppable.
- **Typed args + return.** Reuse existing types from `platform/settings/store`,
  `platform/library/types`, etc. Where a payload type doesn't exist yet, define
  a local `…Args` type in the api module. No `any`.
- **No error handling inside the wrapper.** Call sites keep their existing
  `try/catch` + `reportInvokeError(...)`. The wrapper only adds the type + the
  command-name indirection. (A future, separate pass could standardize error
  handling — explicitly out of scope here to keep each slice mechanical and
  behavior-preserving.)
- **No new logic.** If a call site builds its payload inline, the wrapper takes
  those same fields as typed params and forwards them unchanged.
- Command name string lives in exactly one place (the wrapper). Grep for the
  string across `src/` after a slice → only the api module should match.

## Module map (13 domains)

Assign by concern, not by which file calls it. Commands grouped (representative
— full assignment happens per slice):

| Module | Commands (concern) |
| --- | --- |
| `settingsApi` | video/display (get_video_state, set_window_mode, set_scaling_mode, set_bloom_amount, set_shader_preset, list_shader_presets, set_display_aspect_override, set_overscan_crop, set_presentation_mode/get, list_monitors, set_run_ahead), audio (list/get/set_audio_device_pref, set_audio_volume, list_audio_devices, play/stop_audio), system settings (get/set_system_settings), per-game overrides (get/set_game_overrides), shell mode (get_shell_mode, get/set_shell_mode_pref), kiosk (get_kiosk_mode) |
| `libraryApi` | games (list_games, add_games, get_game, delete_game/all/for_system, drop_seed_games, find_game_id_by_path, update_game_favorite/completed/core_override, set_game_*), folders (list/add/remove/update/reorder_folders, set_folder_rules, set_watched_folders, directory_is_empty), groups (list_game_groups, set/clear_game_group_default), migration (migrate_*_from_local_storage) |
| `collectionsApi` | list/create/delete/rename_custom_collection, add_to/remove_from_custom_collection, list_collection_members |
| `viewsApi` | get_views, set_views, get_layout, set_layout |
| `mediaApi` | get_media_index, get/set/clear_platform_media, set_manual_cover, clear_media, get/set_media_kinds_to_fetch, get/set_only_sync_identified, sync_media/metadata_for_system, import_art_pack, media_storage_stats, open_media_folder, resolve_background_asset, game-info (get/set/delete_game_info_override, list_game_info_overridden/badges, get_game_info), mame (lookup_mame_title/game, refresh_mame_system_info), hashes (resolve/sync_rom_hashes_for_system) |
| `coresApi` | list_cores, available_cores, download_core, install_core_from_path, remove_installed_core, start_bulk_core_install, recommended_core_for_system, core options (list_core_options, has_core_options_schema, get/set_core_pref, set_system_core_option, set_game_core_option, apply_game_core_options), bios (get_bios_status, install_bios_file, open_bios_folder) |
| `inputApi` | get_bindings, set_binding, reset_bindings, get_input_descriptors, arm_libretro_device, get_controller_devices, analog (set_analog_routing/_for_game, arm_analog_routing, analog_sticks_for_system), system_has_light_gun |
| `emulatorApi` | launch_rom, unload_rom, boot_without_game, system_supports_bootless, external launchers (list_emulator_profiles, set_emulator_binary_path, get/set_launcher_pref, get_active_launcher_capabilities), disc (set_disc_image/eject, get_disc_state, list_disc_set_members, set_selected_variant), region (get/set_region_priority) |
| `rewindTasApi` | rewind (get_rewind_state, set_rewind_config, start/end_rewind_scrub, set_rewind_scrub_position), tas (get_tas_state, start/stop_tas_recording, start/stop_tas_replay, list/delete_tas_recordings), save slots (list_save_slots, delete_save_slot) |
| `cheatsApi` | list/add/update/delete_cheat, arm_cheats, list_cheat_formats, start/peek/filter/end_cheat_search, read_memory_region, pick_patch_file |
| `milestonesApi` | list/add/update/delete_milestone, arm_milestones, reset_milestone_progress |
| `captureApi` | screenshots (list/delete_screenshot, open_screenshot_folder), video (list/delete_video_clip, start/stop_video_capture, convert_video_clip_to_webm, open_video_clip_folder) |
| `jobsApi` | list_active/recent_jobs, cancel/pause/resume_job, cancel/pause_all_jobs, clear_job_history, check_duplicate_job, get_job_prefs, set_job_resume_prompt, resume_one_interrupted_job, spawn_test_job, start_background_scan/_directory_scan, cancel_background_scan |
| `systemApi` | get_system_status, get/set/delete_system_info_override, reset_system_info_to_default, get_system_info/_curated, set_system_info, detect_cpu_tier, get_perf_stats |
| `shellApi` | quit_app, get_oa_data_dir, set_ui_intercepting, log_from_frontend, get_recent_logs, get_log_file_path, reveal_logs_folder, reveal_game_file_in_folder, scummvm (find_scummvm_cli, detect_scummvm_directories, run_scummvm_cli_detect, write_scummvm_descriptors), sounds (resolve_ui_sound, resolve_platform_music, resolve_completion_chime) |

(14 modules listed — `collectionsApi` + `viewsApi` split out of `libraryApi`
for cohesion; fold back if either is trivially small in practice.)

## Slice order (6 PRs, verify between each)

Each slice: create the module(s) → repoint every caller → `npm run typecheck`
+ `npm run lint` green. The lint rule turns on only in Slice 6.

1. **`settingsApi` + the convention** — establishes `platform/api/` + the
   wrapper shape on the highest-value theme surface (QuickSettings 37,
   GameDialogs display/shader/audio, SettingsSections, SystemDialogs, App.tsx
   AV paths, settings/store.ts). ~50 sites. **This is the queued Slice 1.**
2. **`libraryApi` + `collectionsApi` + `viewsApi`** — the store-heavy core
   (library/store.ts, customCollections.ts, ingest.ts, views/store.ts,
   App.tsx). ~55 sites.
3. **`mediaApi`** — art/metadata sync + game-info + mame + hashes
   (media.tsx, platformMedia.tsx, gameInfo, MediaSettings, ImportWizard art
   paths). ~45 sites.
4. **`coresApi` + `inputApi`** — cores/bios/core-options + bindings/analog
   (CoresPage, CoreOptionsPanel, SystemBindingsEditor, AnalogBindingsSection,
   ImportWizard core paths). ~45 sites.
5. **`emulatorApi` + `rewindTasApi` + `cheatsApi` + `milestonesApi` +
   `captureApi`** — the in-game / gameplay cluster (launch.ts, QuickSettings
   gameplay controls, GameDialogs cheats/milestones, SaveSlotsModal,
   ScreenshotGalleryDialog). ~70 sites.
6. **`jobsApi` + `systemApi` + `shellApi` + TURN ON THE LINT RULE** — jobs
   (backgroundJobs.ts, background-jobs/*), system health (SystemHealthPage,
   systemInfo.ts), app/shell (App.tsx, logbridge.ts, scummvm, sounds). ~85
   sites. Then add the `no raw invoke() outside platform/api/` ESLint rule and
   confirm green — the ratchet closes.

Slice sizes are estimates; split any slice that runs long (e.g. Slice 5's five
modules can land as two PRs). Don't merge a half-migrated module — a module is
done when grepping its command strings across `src/` matches only its api file.

## The lint rule (Slice 6)

`import/no-restricted-paths` is path-based; banning a named import needs
`no-restricted-imports` (or `no-restricted-syntax` for the bare call). Add to
`frontend/eslint.config.mjs`, scoped to everything except `src/platform/api/`:

```js
// Applied to files NOT under src/platform/api/:
"no-restricted-imports": ["error", {
  paths: [{
    name: "@tauri-apps/api/core",
    importNames: ["invoke"],
    message:
      "Raw invoke() is corralled into platform/api/. Import a typed wrapper " +
      "from @oa/platform/api/<domain>Api instead, or add one there.",
  }],
}],
```

(`convertFileSrc` from the same module stays allowed — only `invoke` is
restricted. Use an `ignores`/per-file override so `src/platform/api/**` keeps
raw `invoke`.) Verify it fires with a probe (as the grab-bag drain did), then
confirm the whole tree is green.

## Verification (every slice)

```
cd frontend && npm run typecheck && npm run lint
```

Per-slice grep check: after migrating module X, every command string it owns
should match ONLY `platform/api/XApi.ts` across `src/`. No behavior change
intended — operator smoke-test of the touched surface (Slice 1: open
QuickSettings during a game, change window mode / scaling / shader / audio
device / volume; per-game Display + Shaders + Audio dialogs; Settings → Display
/ Audio / Shaders) before merge.

## After this

Platform/engine/theme are fully isolated — file boundary (six lint zones) +
API boundary (typed `platform/api/` + invoke ban). The decoupling track is
**done**. Remaining theming work is the *enable-other-themes* track, which
builds on the clean boundary rather than decoupling further: ARC 1 Phase 3
(shared nav primitives), Phase 5 (`.oatheme` packaging), Phase 6 (rebuild
Retroverse as a theme on the SDK), then ARCs 2-3 (Rhai behaviors + WGSL
shaders + Theme Studio). See [PLANS/theming-substrate.md](theming-substrate.md).
