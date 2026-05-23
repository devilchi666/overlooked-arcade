# Portable install

Opt-in mode where all OA user state lives under `<exe_dir>/settings/`
next to `oa-shell.exe`, instead of `%APPDATA%\dev.overlookedarcade.shell\`.
Makes the install fully portable: the whole folder (binary + cores +
BIOS + settings) can be moved to a USB stick, copied between machines,
or backed up as a unit.

## How to enable

1. Drop a file named `portable.txt` next to `oa-shell.exe` (content
   ignored — the marker file's *presence* is the entire opt-in).
2. Launch. The startup log line confirms the mode:
   ```
   oa-shell: data dir = D:\OA-portable\settings (portable)
   ```
3. If you previously had AppData state, OA auto-copies it to the
   portable dir on first launch — saves, library DB, bindings,
   media cache, screenshots, all of it. The original AppData dir is
   left intact; delete it manually once you've verified the portable
   install works.

## What's portable, what isn't

- ✅ **User state** (`<exe_dir>/settings/`): audio.json, shell.json,
  cores.json, layout.json, presentation.json, views.json,
  bindings/, systems/, core-options/, library/games.sqlite + media
  caches, saves/, screenshots/, clips/, tas/, logs/, temp/.
- ✅ **Cores** (`<exe_dir>/cores/`): already portable, no change.
- ✅ **BIOS** (`<exe_dir>/system/`): already portable, no change.
- ❌ **OS-level state** (file associations, Windows registry, etc.):
  not touched.

## Without a marker file

Behavior is unchanged — OA uses Tauri's `app_data_dir()`
(`%APPDATA%\dev.overlookedarcade.shell\` on Windows). Existing
installs keep working with zero migration.

## Files in this folder

- `SESSION_LOG.md` — entries for portable-install work.

## Implementation notes

- Single resolver: `apps/oa-shell/src/data_dir.rs::resolve_data_dir`.
  Both GUI and headless-CLI paths route through it.
- Asset-protocol scope extension: in portable mode,
  `main.rs::setup` calls
  `app.asset_protocol_scope().allow_directory(&data_dir, true)` so
  the frontend's `convertFileSrc` URLs for cover art keep working.
- Frontend uses `frontend/src/lib/dataDir.ts::getDataDir()` (which
  invokes the `get_oa_data_dir` Tauri command) instead of
  `@tauri-apps/api/path::appDataDir()` — Tauri's resolver doesn't
  know about portable mode.
- Migration sentinel: `<settings>/.migrated-from-appdata` is written
  after a successful copy. Subsequent launches skip migration even
  if the portable dir is emptied manually.

## Related

- [docs/PARKING_LOT.md](../../PARKING_LOT.md) — installer / MSI
  work for non-portable distribution is its own stream.
- `apps/oa-shell/src/data_dir.rs` — the resolver source.
