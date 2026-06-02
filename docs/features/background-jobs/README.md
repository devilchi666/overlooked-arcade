# Background jobs + persistent progress bar

Consolidates today's scattered per-operation toasts / modals / debug-log
surfaces (core downloads, dat sync, hash resolve, media sync, MAME imports,
folder scans, future per-track SHA-1) into ONE persistent bar across the
bottom of the Retroverse shell, backed by a SQLite-persisted
`background_jobs` table + per-kind resume handlers so operations survive
app restart.

**Source of truth:** [`docs/PLANS/background-jobs-and-progress-bar.md`](../../PLANS/background-jobs-and-progress-bar.md)
— 5-phase plan, ~5-6 weeks, planning locked 2026-06-02 across 6 rounds of
operator Q&A (24 design decisions). This folder records implementation
slices + decisions, not the design rationale.

## Phase plan

| Phase | Deliverable |
| --- | --- |
| **1** | Schema + JobRegistry + lock-file + heartbeat + `core_download` pilot wired end-to-end (cancel + crash-recovery mechanics in place; no auto-resume dispatch yet — interrupted rows sit and wait for operator retry). |
| 2 | `BackgroundJobsBar` Solid component — handle / collapsed / expanded states, max-3-rows + "+N more", per-row controls, header for 2+ jobs. |
| 3 | `JobResumer` trait + per-kind handlers for `core_download` / `artwork_sync` / `hash_resolve`. Auto-resume-on-launch flow + duplicate-trigger Wait/Restart/Cancel dialog. |
| 4 | Remaining 6 kinds wired (`folder_scan`, `metadata_sync`, `mame_listxml_import`, `dat_sync`, `bulk_core_install`, `disc_track_hash`) + dependency graph + per-kind retry policy. |
| 5 | "Download Settings" panel + Recent activity full panel + polish (per-kind opt-out, sound on completion, retry policy controls). |

## Scope (Phase 1)

| Slice | Deliverable |
| --- | --- |
| A | Migration v17→v18: `background_jobs` table + 3 indexes per plan §Schema. |
| B | New `apps/oa-shell/src/job_registry.rs` — JobRegistry / JobHandle / JobKind / JobState / JobSnapshot / JobEvent. 1 Hz SQLite write debounce, 10 Hz Tauri event broadcast cap, ~1s heartbeat. Rolling-buffer history cap 100. |
| C | `<data_dir>/oa.lock` lifecycle in main.rs. Lock present at startup → `promote_running_rows_to_interrupted()`. Delete on `RunEvent::Exit`. |
| D | `core_installer::download_core` wired through the registry: create_job at entry; cancel + pause AtomicBool checks in the chunk loop; mark_completed / failed / cancelled at exit; flush_resume_state on cancel; per-kind cancel-cleanup (delete `.partial`). Existing `oa://core-download-progress` emit kept intact (back-compat for Guided Setup's listener). |

## Out of scope for Phase 1

- Frontend bar UI — Phase 2.
- Auto-resume-on-launch dispatch loop — Phase 3.
- Other 8 kinds — Phase 4.
- Settings panel — Phase 5.
- Per-kind chime variants — PARKING_LOT.

## Open questions resolved before Phase 1

- **Resume scope:** Phase 1 ships the mechanics (cancel flush + crash
  detection promotes `running` → `interrupted`) but NOT the auto-resume
  dispatch loop. Interrupted rows sit until Phase 3 adds the resumer.
- **Per-kind inventory re-validation:** Deferred to Phase 4 (when the
  other kinds get wired). Phase 1 only touches `core_download`.
- **`oa://core-download-progress` keep-or-retire:** Kept intact in
  Phase 1; the registry calls `progress()` directly from the byte loop
  AND the existing emit fires (operator approved double-emit so Guided
  Setup's listener doesn't break).
