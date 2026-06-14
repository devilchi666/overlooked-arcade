# Overlooked Arcade — Documentation Index

Routing table. Read this first; it points to everything else.

## How docs are organized

- **Active cross-cutting work** lives under `docs/features/<name>/` — features
  currently in flight or queued. Each carries its own README / ROADMAP /
  SESSION_LOG / DECISIONS.
- **Per-core work** lives under `docs/cores/<id>/`. Same file shape.
- **Project-wide** lives at `docs/` root.
- **Shipped / closed work** lives under `docs/_archive/` (features + plans).
  Read-on-need only — see [docs/_archive/INDEX.md](_archive/INDEX.md) for the
  manifest. Loading discipline rules in CLAUDE.md "How to start a session".
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

## Cross-cutting features (active)

- [features/controller-identity/](features/controller-identity/) — Controller Identity & Auto-Config. Stable per-controller identity (VID/PID) + auto-config of shell-nav AND per-system gameplay bindings from shared data files (`controllers.json` / `systems-input.json` / `default-maps.json`) + a press-the-buttons wizard. Fixes non-standard pads (wired Switch Pro) + replug port-shuffle. Planned 2026-06-12; Phase 0 (identity spike) queued. Design at [PLANS/controller-identity-substrate.md](PLANS/controller-identity-substrate.md).
- [features/dosbox-and-scummvm/](features/dosbox-and-scummvm/) — DOSBox + ScummVM onboarding plan (📐 planned, not yet implemented)
- [features/guided-setup/](features/guided-setup/) — Guided Setup arc. Phase 1B closed 2026-06-01; Phase 2 (curated CPU-tier core selection) queued. Design at [PLANS/guided-setup.md](PLANS/guided-setup.md).
- [features/hw-render/](features/hw-render/) — HW-render pipeline. Hosts GPU-rendered libretro cores (Dolphin, paraLLEl-N64, Beetle PSX HW, …) via the libretro HW render interface on a Vulkan-first backend abstraction. Planning locked 2026-06-07; slotted after VL Phase C3, before Theming ARC 2. Design at [PLANS/hw-render-pipeline.md](PLANS/hw-render-pipeline.md).
- [features/kiosk-shell/](features/kiosk-shell/) — full-screen cabinet mode (design-only)
- [features/metadata-editing/](features/metadata-editing/) — Metadata Curation. Premium engine **Settings → "Metadata"** editor for game + system metadata on an override layer (per-field reset + provenance). Planned 2026-06-11; Wave 1 / S1 queued. Design at [PLANS/metadata-editing.md](PLANS/metadata-editing.md).
- [features/per-system-ui/](features/per-system-ui/) — Per-system custom UI. Stage 1 code arc complete; Stages 2+3 + content pilots pending.
- [features/retroverse-ui/](features/retroverse-ui/) — Top-toolbar tab IA. 6/6 tabs operator-facing as of 2026-05-28. Design at [PLANS/retroverse-ui-rollout.md](PLANS/retroverse-ui-rollout.md).
- [features/theming-substrate/](features/theming-substrate/) — Theming substrate (BigBox-style themes, engine vs theme territory inside one binary). 3-arc structure planned 2026-06-06; ARC 1 queued. Design at [PLANS/theming-substrate.md](PLANS/theming-substrate.md).
- [features/unified-nav/](features/unified-nav/) — **Unified Navigation & Panel System** (planned 2026-06-14). Spatial-nav engine (universal focusable auto-discovery + geometry movement + layer scoping) + a unified input-agnostic panel structure/look (keyboard / controller / kiosk). Supersedes per-panel nav wiring. Design at [PLANS/unified-navigation-and-panels.md](PLANS/unified-navigation-and-panels.md). Predecessor: [features/nav-coverage/](features/nav-coverage/).

**Shipped (archived):** sidebar, ui-polish, library-import, portable-install, media-taxonomy, controller-nav, background-jobs. See [_archive/INDEX.md](_archive/INDEX.md) for the manifest.

## Per-core docs

All system-specific docs live under [docs/cores/&lt;id&gt;/](cores/) (41 systems
with docs as of 2026-05-27 — jagcd / sega32xcd / stv added). See
[ACTIVE_WORK.md](ACTIVE_WORK.md) for which
cores are currently being worked on.

- [cores/SCHEMA.md](cores/SCHEMA.md) — YAML schema reference for the
  per-system `games-info.md` files that drive the Game Info Panel
  (factual + narrative reference data per game). Authoritative.

## Research + plans (in flight / pre-execution)

- [RESEARCH/](RESEARCH/) — competitor analysis, forum surveys
- [PLANS/](PLANS/) — design docs for in-flight or queued work
- [PLANS/controller-identity-substrate.md](PLANS/controller-identity-substrate.md) — **input-infrastructure arc** (2026-06-12). Stable per-controller identity (VID/PID) + auto-config of shell-nav AND per-system gameplay bindings from three shared data files + a press-the-buttons wizard; fixes non-standard pads (wired Switch Pro) + the replug port-shuffle (the RetroArch gap). Two pollers stay separate, the config unifies. Composes with dynamic-controller-info / dynamic-input-descriptors. Foundation-first; Phase 0 (identity spike) in NEXT.md HIGH band.
- [PLANS/virtual-library-and-launcher-arc.md](PLANS/virtual-library-and-launcher-arc.md) — **major next arc** (2026-06-03, 8 phases, ~14–22 weeks). Promotes virtual-library grouping to SQLite schema + launcher-abstraction for external standalone emulators (Cemu / RPCS3 / Lime3DS). Reverses the 2026-05-16 libretro-only stance.
- [PLANS/hw-render-pipeline.md](PLANS/hw-render-pipeline.md) — **engine arc** (2026-06-07). Implements the libretro HW render interface so GPU-rendered cores (Dolphin, paraLLEl-N64, Beetle PSX HW, Flycast, PPSSPP, Beetle Saturn HW) run in-process instead of crashing. Vulkan-first multi-backend abstraction (RetroArch video-driver model on wgpu); zero-copy shared-device end state. Slotted after VL Phase C3, before Theming ARC 2.
- [PLANS/per-system-descriptors.md](PLANS/per-system-descriptors.md) — Slices 1+2 shipped 2026-06-02; Slice 3 (L3 content packs + L4 SQLite + JSON Schema + CI lint) queued.
- [PLANS/disc-track-sha1-matching.md](PLANS/disc-track-sha1-matching.md) — per-track SHA-1 for disc-shape systems. **Folds into the virtual-library arc as Slice A1.**
- [PLANS/content-packs.md](PLANS/content-packs.md) — content-pack distribution design (unbuilt).
- [PLANS/guided-setup.md](PLANS/guided-setup.md) — Phase 2 (curated CPU-tier core selection) queued.
- [PLANS/metadata-editing.md](PLANS/metadata-editing.md) — **Metadata Curation arc** (2026-06-11). Premium Settings → "Metadata" editor for game + system metadata via an override layer; OA's biggest greenfield interaction win over LaunchBox (research §4). System override backend already shipped; builds the game-factual half + the editor. Wave 1 / S1 in NEXT.md HIGH band.
- [PLANS/per-system-ui.md](PLANS/per-system-ui.md) — Stages 2+3 of the per-system UI arc.
- [PLANS/retroverse-ui-rollout.md](PLANS/retroverse-ui-rollout.md) — Retroverse §10 open work tracker.
- [PLANS/theming-substrate.md](PLANS/theming-substrate.md) — 3-arc theming substrate plan (engine/theme territory split + `.oatheme` distribution; absorbs Kiosk plan's 4-layer substrate). Phases 1-2 + boundary-enforcement track + grab-bag drain shipped; Phase 4 next, then the enable-other-themes track (Phase 3/5/6 + ARC 2-3).
- [PLANS/theming-grabbag-drain.md](PLANS/theming-grabbag-drain.md) — ✅ SHIPPED 2026-06-09. Drained `src/components/` to zero (→ `engine/` + `platform/components/`); six boundary lint zones enforced.
- [PLANS/theming-platform-api-bridge.md](PLANS/theming-platform-api-bridge.md) — **Theming Phase 4** (queued 2026-06-09). Last platform/theme decoupling step: corral 351 raw `invoke()` calls (54 files / 222 commands) behind typed `platform/api/*` wrappers + an invoke-ban lint rule. Slice 1 (`settingsApi`) in NEXT.md HIGH band.
- [PLANS/unified-navigation-and-panels.md](PLANS/unified-navigation-and-panels.md) — **Unified Navigation & Panel System** (2026-06-14). Spatial-nav engine (universal auto-discovery + geometry movement + layer scoping, reusing the Controller-Nav-Coverage activate layer) + a unified input-agnostic panel structure/look. Phase 1 (engine + prove on Settings) in NEXT.md HIGH band.

**Shipped plans (archived):** background-jobs-and-progress-bar, collections-tab-retroverse, discover-tab-retroverse, game-info-panel, main-window, play-now-tab-retroverse, retroverse-flag-deprecation, settings-declutter-system-health, settings-tab-retroverse, system-info-panel-v1, system-wiring-plan. See [_archive/INDEX.md](_archive/INDEX.md).

## Setup reference (off-repo)

`C:\Users\Devilchi\.claude\plans\jazzy-spinning-blum.md` — project setup plan
(Cargo layout, Tauri+wgpu integration, license discussion, build/dev workflow).
