# Archive Index

**Read on explicit need only.** This directory holds shipped feature
folders + closed plan documents that no longer need to be in the
active surface. Files are preserved verbatim — Grep still reaches
them — but the active `docs/` tree stops carrying them.

When you might need to read into the archive:
- Investigating "why was X done that way?" — open the archived plan
  or the relevant feature's SESSION_LOG.
- Matching a past pattern — open the closest analogous feature's
  README or DECISIONS.
- Reconstructing context after a regression appears in code from a
  shipped arc.

When you should NOT read the archive:
- Picking up "what to work on next" — that's `docs/ACTIVE_WORK.md` +
  `docs/NEXT.md` only.
- Verifying current behavior — Grep the code, not the archive.
- General reading at session start — archive is dormant by design.

---

## Archived features (`docs/_archive/features/`)

- **`background-jobs/`** — Background jobs registry + persistent
  BackgroundJobsBar (`apps/oa-shell/src/job_registry.rs` +
  `frontend/src/components/BackgroundJobsBar.tsx`). Full 7-phase arc
  shipped 2026-06-02 / 2026-06-03 (Phases 1 / 2 / 3a / 3b / 4a / 4b /
  4c / 5 + polish). Tracks every long-running operation (core
  downloads, hash resolve, media sync, MAME listxml import, etc.)
  with pause / resume / cancel + crash-recovery via `oa.lock` +
  ResumePromptDialog. Plan archive:
  `_archive/PLANS/background-jobs-and-progress-bar.md`.

- **`controller-nav/`** — Phase 0 controller-nav primitives shipped
  2026-05-26. Focus manager (`frontend/src/nav/focus.ts`), gamepad →
  UI event layer (`frontend/src/nav/gamepad.ts`), focus-ring
  component pattern, on-screen HintBar, Controller-nav Settings
  section. Completion pass (2026-05-26) extended coverage to every
  remaining interactive surface (sidebar / dialogs / menu bar /
  chained popovers).

- **`library-import/`** — Library scanner + Import Wizard + media
  sync infrastructure. Shipped earlier 2026-05 series.

- **`media-taxonomy/`** — Full LaunchBox-shape art / audio / video /
  manual storage shipped 2026-05-24. Expanded MediaDb from 5 slots
  to ~26-slot LaunchBox shape. v1 legacy keys (boxart / snap /
  title / cart) remain readable for one-release fallback.

- **`metadata-editing/`** — Metadata Curation arc. Archived 2026-06-15:
  the override backend (`game_metadata_overrides`) shipped, and the
  editor UI was **absorbed into the Per-System Settings Hub**
  (`engine/systemsHub/domains/{Game,Platform}MetadataEditor.tsx`) — the
  standalone `metadata` Settings category was removed. Backend decisions
  (D1-D5) stay valid + reused by the Hub.

- **`portable-install/`** — `<exe_dir>/settings/` opt-in via
  `portable.txt` marker file next to oa-shell.exe.

- **`sidebar/`** — Library sidebar tier system + view editor +
  per-system theming integration. Multi-pass arc shipped 2026-05.

- **`ui-polish/`** — Menu-bar IA reorganization + dialog
  consolidation. Shipped during the 2026-05 Retroverse-rollout era.

---

## Archived plans (`docs/_archive/PLANS/`)

- **`background-jobs-and-progress-bar.md`** — Planning locked
  2026-06-02. Five-phase arc design (concurrency / pause / resume /
  persistence / Settings). Full execution shipped 2026-06-02 /
  2026-06-03 — see archived feature folder.

- **`collections-tab-retroverse.md`** — Retroverse COLLECTIONS tab
  design (6 smart-lists + Slice 12 custom collections). Shipped
  2026-05-29.

- **`discover-tab-retroverse.md`** — DISCOVER tab body design (4
  data-driven axes from MediaDb metadata + 5 editorial axes). Body
  shipped 2026-05-29; editorial axes await Phase C6 content-packs.

- **`game-info-panel.md`** — Game Info Panel v1 design (3-layer data
  model + 6 Tauri commands + 4-tab modal). v1 shipped 2026-05-30
  (`feat/game-info-panel-v1`). v2 (scraper + data repo + community
  pipeline) designed in §11; deferred.

- **`main-window.md`** — Earlier single-window vs two-window shell
  design. Both modes shipped; selectable via Settings → Display →
  Shell mode.

- **`play-now-tab-retroverse.md`** — PLAY NOW tab design (9-mood
  sidebar + 3 rails + WHY-line hero). Shipped earlier 2026-05.

- **`retroverse-flag-deprecation.md`** — Plan for deleting the
  legacy Shell after Retroverse rollout reached parity. Deletion
  shipped 2026-05-31 (-1860 lines, 13 files).

- **`settings-declutter-system-health.md`** — 5-phase arc that
  shipped System Health hub + Game-media status-first cards
  2026-06-03 (`feat/settings-declutter-system-health` →
  `dd430e4`).

- **`settings-tab-retroverse.md`** — Retroverse SETTINGS tab design
  (3-pane layout + 15 categories + Per-system drill-in). Shipped
  2026-05-28 / 2026-05-29.

- **`system-info-panel-v1.md`** — System Info Panel v1 design
  (4-table SQLite schema + per-system YAML curation + L3 overrides).
  Shipped 2026-06-01.

- **`system-wiring-plan.md`** — Original per-system Rust crate
  wiring plan. Superseded by the 2026-05-16 libretro pivot; kept
  for historical reference.

### Archived 2026-06-15 (doc audit — shipped/closed arcs)

- **`controller-identity-substrate.md`** — Controller Identity &
  Auto-Config. Full arc shipped + merged 2026-06-13 (`808fc0b`):
  VID/PID identity, replug-stable ports, non-standard-pad
  normalization + SDL `gamecontrollerdb` import, label families.
- **`disc-track-sha1-matching.md`** — VL Phase A1 sub-plan. Per-track
  SHA-1 shipped 2026-06-03 then pivoted to fuzzy-filename primary;
  only the parked Sub-phase 4 remains (tracked in the parent VL arc).
- **`game-identities-schema.md`** — VL Phase E sub-plan. Schema
  v22→v23 `game_identities` + read-path swap; Sub-phases 1-3 shipped
  2026-06-07 (Phase E complete).
- **`launcher-abstraction.md`** — VL Phase C sub-plan. `Launcher`
  trait + `ExternalProcessLauncher` + profile registry; C1-C3 merged
  2026-06-08 (Phase C complete).
- **`metadata-editing.md`** — Metadata Curation arc. Override backend
  shipped; editor UI absorbed into the Per-System Settings Hub
  (see archived feature folder). Closed 2026-06-15.
- **`settings-ia-redesign.md`** — Settings IA re-cut. Slices 1-4
  merged 2026-06-14/15 (Import & Setup · Library · Organize · Systems ·
  External Emulators · Themes/Appearance); Slice 5 deferred into
  guided-setup Phase 2.
- **`theming-grabbag-drain.md`** — Drained `src/components/` to zero
  (→ `engine/` + `platform/components/`); shipped 2026-06-09.
- **`theming-platform-api-bridge.md`** — Theming Phase 4. Corralled
  raw `invoke()`/`listen` behind typed `platform/api/*` + lint ban;
  all 6 slices + 4.5 merged 2026-06-10 (Phase 4 complete).

---

## How to add to the archive

When a feature folder or plan completes:

1. Confirm the work is fully shipped (or won't-fix) via the relevant
   SESSION_LOG entry.
2. `git mv docs/{features,PLANS}/<name> docs/_archive/{features,PLANS}/`.
3. Append a one-line entry to this INDEX with date + one-sentence
   summary + brief path/PR pointer.
4. Trim the matching entry out of `docs/ACTIVE_WORK.md` and
   `docs/NEXT.md` so the active surface stays small.

When `docs/SESSION_LOG.md` crosses ~150 lines, roll the oldest
entries into `docs/_archive/SESSION_LOG-<period>.md` per the existing
CLAUDE.md policy.
