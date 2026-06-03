# Library import — Import Wizard, scanner, media sync

Placeholder folder for the cross-system library import stream: folder
scanning, ROM hash identification, Import Wizard flow, media (cover art /
thumbnails) sync against libretro-thumbnails repos, folder watch, and
the SQLite library DB.

**Status as of 2026-05-22:** Shipped (Phase 2.5 + Phase 2.7); no active
work. This folder exists so future dedicated work has a home — today the
relevant docs are scattered across `docs/DECISIONS.md` entries and per-PR
commit messages.

## Files in this folder

Empty today. If/when a dedicated import-stream effort kicks off (e.g.
folder-as-game scanner for scummvm + dosbox, or arcade ROM-set resolution
work), this folder gets ROADMAP.md / SESSION_LOG.md / DECISIONS.md added.

## What lives elsewhere today

- Multi-repo media sync — `crates/oa-content/src/media.rs`,
  `apps/oa-shell/src/media.rs::sync_media_for_system`.
- Library DB — `apps/oa-shell/src/library_db.rs`.
- Import Wizard frontend — `frontend/src/components/ImportWizard*.tsx`.
- Cross-system infra inventory — `docs/NEXT.md` "Cross-system infrastructure
  inventory" section.
