# Overlooked Arcade — Documentation Index

Routing table. Read this first; it points to everything else.

## How docs are organized

- **Cross-cutting work** lives under `docs/features/<name>/` — features that
  don't belong to a single core (sidebar, UI polish, library import, kiosk).
  Each carries its own README / ROADMAP / SESSION_LOG / DECISIONS.
- **Per-core work** lives under `docs/cores/<id>/`. Same file shape.
- **Project-wide** lives at `docs/` root.
- **Old SESSION_LOG entries** spill to `SESSION_LOG_ARCHIVE.md` next to the
  live one when the live file grows past ~150 lines. Read the archive only
  when you need history older than the last ~5 entries.

## Project-wide

- [ACTIVE_WORK.md](ACTIVE_WORK.md) — what's in flight right now
- [NEXT.md](NEXT.md) — cross-system priority queue (HIGH / MEDIUM / LOWER / DEFERRED)
- [DECISIONS.md](DECISIONS.md) — append-only project decisions log
- [PARKING_LOT.md](PARKING_LOT.md) — out-of-scope ideas kept for reference
- [VISION.md](VISION.md) — project vision
- [ROADMAP.md](ROADMAP.md) — project-wide phase plan
- [SESSION_LOG.md](SESSION_LOG.md) — project-wide entries (cross-stream)
- [CHATGPT_BRIEFING.md](CHATGPT_BRIEFING.md) — self-contained briefing for ChatGPT / external LLM collaborators. Paste into a fresh session to get gap-spotting + feature-ideation help.

## Cross-cutting features

- [features/sidebar/](features/sidebar/) — library sidebar tier + view editor
- [features/ui-polish/](features/ui-polish/) — menu-bar IA + dialog reorganization
- [features/library-import/](features/library-import/) — import wizard, scanner, media sync
- [features/kiosk-shell/](features/kiosk-shell/) — full-screen cabinet mode (design-only)
- [features/portable-install/](features/portable-install/) — `<exe_dir>/settings/` opt-in via portable.txt marker
- [features/media-taxonomy/](features/media-taxonomy/) — full LaunchBox-shape art/audio/video/manual storage (✅ shipped 2026-05-24)
- [features/dosbox-and-scummvm/](features/dosbox-and-scummvm/) — DOSBox + ScummVM onboarding plan (📐 planned, not yet implemented)
- [features/controller-nav/](features/controller-nav/) — Phase 0 controller-nav primitives (focus manager + gamepad event layer + hint bar); shared foundation for guided-setup + per-system-UI
- [features/guided-setup/](features/guided-setup/) — Guided Setup arc (wizard upgrade + smart-scan + readiness checklist + curated cores). Phase 1B Slice 1 shipped 2026-06-01; design lives at [docs/PLANS/guided-setup.md](PLANS/guided-setup.md).
- [features/per-system-ui/](features/per-system-ui/) — Per-system custom UI (audio / boot animations / tile flourishes / backgrounds, eventually layout + in-game overlays); Stage 1 in flight
- [features/retroverse-ui/](features/retroverse-ui/) — Experimental top-toolbar IA replacing the legacy sidebar Shell; 6 of 6 tabs operator-facing as of 2026-05-28. Design + planning docs in `docs/PLANS/retroverse-ui-rollout.md` + per-tab + content-packs docs. Deletion plan for the legacy Shell in `docs/PLANS/retroverse-flag-deprecation.md` (2026-05-30 audit; no code changes yet). System info panel v1 shipped 2026-06-01 (`feat/system-info-panel-v1`, six phase commits); design + retro at `docs/PLANS/system-info-panel-v1.md`. Schema reference appended to `docs/cores/SCHEMA.md`. Cross-system inventory entry in `docs/NEXT.md`.

## Per-core docs

All system-specific docs live under [docs/cores/&lt;id&gt;/](cores/) (41 systems
with docs as of 2026-05-27 — jagcd / sega32xcd / stv added). See
[ACTIVE_WORK.md](ACTIVE_WORK.md) for which
cores are currently being worked on.

- [cores/SCHEMA.md](cores/SCHEMA.md) — YAML schema reference for the
  per-system `games-info.md` files that drive the Game Info Panel
  (factual + narrative reference data per game). Authoritative.

## Research + plans (historical or pre-execution)

- [RESEARCH/](RESEARCH/) — competitor analysis, forum surveys
- [PLANS/](PLANS/) — design docs for in-flight work

## Setup reference (off-repo)

`C:\Users\Devilchi\.claude\plans\jazzy-spinning-blum.md` — project setup plan
(Cargo layout, Tauri+wgpu integration, license discussion, build/dev workflow).
