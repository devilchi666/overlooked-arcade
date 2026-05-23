# Overlooked Arcade — Documentation Index

Routing table. Read this first; it points to everything else.

## How docs are organized

- **Cross-cutting work** lives under `docs/features/<name>/` — features that
  don't belong to a single core (sidebar, UI polish, library import, kiosk).
  Each carries its own README / ROADMAP / SESSION_LOG / DECISIONS.
- **Per-core work** lives under `docs/cores/<id>/`. Same file shape.
- **Project-wide** lives at `docs/` root.

## Project-wide

- [ACTIVE_WORK.md](ACTIVE_WORK.md) — what's in flight right now
- [NEXT.md](NEXT.md) — cross-system priority queue (HIGH / MEDIUM / LOWER / DEFERRED)
- [DECISIONS.md](DECISIONS.md) — append-only project decisions log
- [PARKING_LOT.md](PARKING_LOT.md) — out-of-scope ideas kept for reference
- [VISION.md](VISION.md) — project vision
- [ROADMAP.md](ROADMAP.md) — project-wide phase plan
- [SESSION_LOG.md](SESSION_LOG.md) — project-wide entries (cross-stream)

## Cross-cutting features

- [features/sidebar/](features/sidebar/) — library sidebar tier + view editor
- [features/ui-polish/](features/ui-polish/) — menu-bar IA + dialog reorganization
- [features/library-import/](features/library-import/) — import wizard, scanner, media sync
- [features/kiosk-shell/](features/kiosk-shell/) — full-screen cabinet mode (design-only)

## Per-core docs

All system-specific docs live under [docs/cores/&lt;id&gt;/](cores/) (38 systems
with docs as of 2026-05-22). See [ACTIVE_WORK.md](ACTIVE_WORK.md) for which
cores are currently being worked on.

## Research + plans (historical or pre-execution)

- [RESEARCH/](RESEARCH/) — competitor analysis, forum surveys
- [PLANS/](PLANS/) — design docs for in-flight work

## Setup reference (off-repo)

`C:\Users\Devilchi\.claude\plans\jazzy-spinning-blum.md` — project setup plan
(Cargo layout, Tauri+wgpu integration, license discussion, build/dev workflow).
