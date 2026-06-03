# Library Import — Session Log

Entries for library scanner / Import Wizard / media-sync / folder-watcher
work. Originally filed under whichever per-core SESSION_LOG was active at
the time (the 2026-05-21 SQLite folders entry was under
`docs/cores/nds/SESSION_LOG.md`) — re-filed here 2026-05-22 as part of the
docs reorg so cross-cutting work has a proper home.

---

## 2026-05-21 — Library folders: SQLite single source of truth (cross-system infra)

- **Diagnosis:** Operator reported "no folders tracked" in Settings →
  Library despite 5 folders + ~4500 games imported. SQLite `folders`
  table held all 5 paths correctly; the localStorage
  `oa.settings.v1.libraryFolders` mirror was empty. Two parallel stores
  had drifted (last log entries that would have showed the drift were
  already rotated out — the 5-archive cap loses ~3 days of history).
- **Shipped (Schema v12):** New `folders.display_order INTEGER NOT NULL`
  column, backfilled from `rowid`. `list_folders` orders by
  `display_order, rowid`. `add_folder` inserts at `MAX+1` so new rows
  go to the end of the user's order. New `reorder_folders(ordered_ids)`
  bulk-update for drag-reorder.
- **Shipped (Tauri):** `reorder_folders` + `migrate_folders_from_local_storage`
  commands. Migration is idempotent (paths already in `folders` are
  skipped) so the strip-and-save step is crash-safe.
- **Shipped (frontend settings store):** Removed `libraryFolders` from
  `Persisted`. Replaced with SQLite-backed `libraryFolderRows` signal
  populated via `list_folders`; `libraryFolders()` getter returns paths
  for backward compatibility with the watcher + Rescan-all. New
  `addLibraryFolderPath`, `removeLibraryFolderById`,
  `reorderLibraryFolderIds`, `refreshLibraryFolders` setters write
  through to SQLite then refresh. One-shot localStorage migration runs
  on init.
- **Shipped (App.tsx + SettingsPage + ImportWizard):** All `setLibraryFolders`
  callers migrated. SettingsPage drag-drop now uses folder ids as
  sortable keys (stable across reorder). ImportWizard drops the mirror
  line and calls `refreshLibraryFolders` after commit.
- **Tests:** `folders_display_order_persists_and_reorders` +
  `migrate_folders_from_local_storage_idempotent` alongside the
  existing `folders_crud_roundtrip`. `cargo test --workspace` green
  (333+ tests). Frontend `tsc --noEmit` clean.
- **Almost:** Operator validation. First launch after upgrade should
  auto-migrate any operator who has localStorage paths into SQLite +
  populate the Settings list from the now-authoritative store.
- **Next:** The operator's previously-imported 5 folders will appear
  in Settings on next launch (SQLite already has them; no migration
  needed for that case — the empty localStorage was the bug).
