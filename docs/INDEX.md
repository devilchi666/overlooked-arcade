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

## Cross-cutting features (active)

- [features/controller-identity/](features/controller-identity/) — Controller Identity & Auto-Config. **✅ Full arc shipped + merged 2026-06-13** (VID/PID identity, replug-stable ports, non-standard-pad normalization + SDL `gamecontrollerdb` import, label families, live test window). Parked follow-ups: Phase-3 wizard, glyph icons, data-file update mechanism. Plan archived at [_archive/PLANS/controller-identity-substrate.md](_archive/PLANS/controller-identity-substrate.md).
- [features/content-packs/](features/content-packs/) — **oa-packs — content-pack distribution infrastructure** (NEW arc, planned 2026-06-15). The single operator-initiated channel for optional content packs (emulator recipes · editorial DISCOVER · themes · per-system asset bundles · cheats · metadata) — one foundation, many pack types. Built **schema + verify + install first, hosting later** (CP1); schemas + on-disk layout are the early lock-in (CP2); pack `type` is additive (CP3); bundled-baseline is per-type (CP4); emulator recipes are the first non-editorial consumer (CP5). New `crates/oa-packs/` pure core. Arc plan at [PLANS/oa-packs-infrastructure.md](PLANS/oa-packs-infrastructure.md) (design: [PLANS/content-packs.md](PLANS/content-packs.md)); decisions CP1–CP5. Slice 1 queued.
- [features/dosbox-and-scummvm/](features/dosbox-and-scummvm/) — DOSBox + ScummVM onboarding plan (📐 planned, not yet implemented)
- [features/external-emulators/](features/external-emulators/) — **External Emulator Depth** (NEW arc, planned 2026-06-15). Deepens the shipped launcher abstraction (VL Phase C): recipe-format upgrade + **independent recipe updates** (changed CLI flags never force an OA rebuild) · **install pipeline** with a per-emulator legal gate (Green/Yellow, default Yellow; zero ROMs/BIOS/keys) · **OA-authored per-emulator control** toward the north star of in-window embedding. Plugin model = OA-authored adapters, NOT a third-party SDK (ED1). Slice 1 (schema accretion + ares/BizHawk profiles) queued. Plan at [PLANS/external-emulator-depth.md](PLANS/external-emulator-depth.md); decisions ED1–ED6.
- [features/guided-setup/](features/guided-setup/) — Guided Setup arc. Phase 1B closed 2026-06-01; Phase 2 (curated CPU-tier core selection) queued. Design at [PLANS/guided-setup.md](PLANS/guided-setup.md).
- [features/hw-render/](features/hw-render/) — HW-render pipeline. Hosts GPU-rendered libretro cores (Dolphin, paraLLEl-N64, Beetle PSX HW, …) via the libretro HW render interface on a Vulkan-first backend abstraction. **M1 proven, M2 merged (zero-copy 60fps), M3 Half 2 merged (status observability + software-peer fallback); M3 Half 1 (operator HW-core validation) + M4 future.** Design at [PLANS/hw-render-pipeline.md](PLANS/hw-render-pipeline.md).
- [features/kiosk-shell/](features/kiosk-shell/) — full-screen cabinet mode (design-only; substrate/Theme-Studio scope migrated to theming-substrate)
- [features/per-system-hub/](features/per-system-hub/) — **Per-System Settings Hub** (✅ COMPLETE + merged 2026-06-14). Consolidated all per-system settings into one card-based **Systems** hub (systems grid → domain cards → editor) — including the **Game/Platform Metadata** editors (which absorbed the former standalone Metadata Curation arc). Built on the spatial-nav engine; delivers the unified-nav Pillar-B panel primitives. Design at [PLANS/per-system-settings-hub.md](PLANS/per-system-settings-hub.md).
- [features/per-system-ui/](features/per-system-ui/) — Per-system custom UI. Stage 1 machinery (Slices 1–5) shipped (now in `platform/`); **architecture superseded/merged into Theming ARC 2 (2026-06-15)** — pilots + Stages 2–3 re-home there as Retroverse content. See [PLANS/theming-arc-2-per-system-layout.md](PLANS/theming-arc-2-per-system-layout.md).
- [features/settings-ia/](features/settings-ia/) — **Settings IA Redesign** (planned 2026-06-14, execution deferred). Re-cuts the engine Settings IA around user intent into new top-level **Themes/Appearance · Library · Organize My Collection · Import & Setup · External Emulators** groups, replacing the conflated Settings → Library 3-tab admin surface. Appearance = theme territory via a declarative per-theme settings schema (rides theming Phase 5); Library gains directory **re-point** (relink, no on-disk file ops). Slice 1 (IA re-skeleton + Library/Organize split) queued. Design at [PLANS/settings-ia-redesign.md](PLANS/settings-ia-redesign.md).
- [features/retroverse-ui/](features/retroverse-ui/) — Top-toolbar tab IA. 6/6 tabs operator-facing as of 2026-05-28. Design at [PLANS/retroverse-ui-rollout.md](PLANS/retroverse-ui-rollout.md).
- [features/theming-substrate/](features/theming-substrate/) — Theming substrate (BigBox-style themes, engine vs theme territory inside one binary). **ARC 1 + ARC 2 complete** (ARC 2 = Per-System Layout Substrate L1–L5 + the `.oatheme` loader P.1, merged 2026-06-16); **ARC 3 (Cinematic & Scripting) PLANNED 2026-06-16, M1 queued**. 4 arcs (D35): 1 MVS · 2 layout · 3 cinematic/scripting · 4 Theme Studio. Designs at [PLANS/theming-substrate.md](PLANS/theming-substrate.md) + [PLANS/theming-arc-2-per-system-layout.md](PLANS/theming-arc-2-per-system-layout.md) + [PLANS/theming-oatheme-loader.md](PLANS/theming-oatheme-loader.md) + [PLANS/theming-arc-3-cinematic.md](PLANS/theming-arc-3-cinematic.md).
- [features/unified-nav/](features/unified-nav/) — **Unified Navigation & Panel System** (planned 2026-06-14). Spatial-nav engine (universal focusable auto-discovery + geometry movement + layer scoping) + a unified input-agnostic panel structure/look (keyboard / controller / kiosk). Supersedes per-panel nav wiring. Design at [PLANS/unified-navigation-and-panels.md](PLANS/unified-navigation-and-panels.md). Predecessor: [features/nav-coverage/](features/nav-coverage/).

**Shipped (archived):** sidebar, ui-polish, library-import, portable-install, media-taxonomy, controller-nav, background-jobs, metadata-editing. See [_archive/INDEX.md](_archive/INDEX.md) for the manifest.

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
- [RESEARCH/external-emulators.md](RESEARCH/external-emulators.md) — **External / standalone emulator research need** (2026-06-14). Scopes the major research into command-line launching of a broad emulator roster — systems we can't run via cores (Cemu / RPCS3 / Ryujinx / Lime3DS / Vita3K / Xenia / xemu / …) AND ones we DO support but where users may want a standalone (PCSX2 / DuckStation / PPSSPP / Dolphin / …). Per-emulator research template + draft CLI roster + grow-over-time philosophy. Feeds `config/emulators/*.yaml` profiles + VL Phase D.
- [PLANS/](PLANS/) — design docs for in-flight or queued work
- [PLANS/external-emulator-depth.md](PLANS/external-emulator-depth.md) — **External Emulator Depth** (2026-06-15). 3 phases: recipe upgrade + independent updates · install pipeline (legal-gated) · OA-authored control toward window-wrapping. Builds on VL Phase C/D. Slice 1 queued in NEXT.md HIGH band.
- [PLANS/virtual-library-and-launcher-arc.md](PLANS/virtual-library-and-launcher-arc.md) — **major next arc** (2026-06-03, 8 phases). Phases 0 / A1 / A2 / E / C shipped + merged; **Phase B code-complete but stranded on branch**; Phases D / F / G future. Promotes virtual-library grouping to SQLite schema + launcher-abstraction for external standalone emulators. Reversed the 2026-05-16 libretro-only stance.
- [PLANS/hw-render-pipeline.md](PLANS/hw-render-pipeline.md) — **engine arc** (2026-06-07). Libretro HW render interface so GPU-rendered cores run in-process. Vulkan-first multi-backend abstraction; zero-copy shared-device. **M1 proven, M2 + M3 Half 2 merged; M3 Half 1 (operator HW-core validation) + M4 future.** Slotted before Theming ARC 3 (Cinematic & Scripting).
- [PLANS/per-system-descriptors.md](PLANS/per-system-descriptors.md) — Slices 1+2 shipped 2026-06-02; Slice 3 (L3 content packs + L4 SQLite + JSON Schema + CI lint) queued.
- [PLANS/content-packs.md](PLANS/content-packs.md) — content-pack distribution **design sketch** (the locked 2026-05-28 "what"). Execution arc below.
- [PLANS/oa-packs-infrastructure.md](PLANS/oa-packs-infrastructure.md) — **oa-packs infrastructure** (arc plan, 2026-06-15). The "how + order" on top of content-packs.md: new `crates/oa-packs/` pure core (sha256 verify + manifest validation + local-zip install) → registry fetch + Tauri → Settings panel + lifecycle → privacy panel → first consumers (emulator recipes, then editorial). Decisions CP1–CP5. Slice 1 queued in NEXT.md HIGH band.
- [PLANS/guided-setup.md](PLANS/guided-setup.md) — Phase 0/1B/2 shipped; later phases (folder mgmt, help-suppression, re-entry) partial/queued.
- [PLANS/per-system-ui.md](PLANS/per-system-ui.md) — **superseded/merged into Theming ARC 2** (Stage 1 machinery shipped; pilots + Stages 2–3 re-home as Retroverse content). Banner-reconciled 2026-06-15.
- [PLANS/retroverse-ui-rollout.md](PLANS/retroverse-ui-rollout.md) — Retroverse §10 open work tracker (6/6 tabs shipped; tracker lags).
- [PLANS/theming-substrate.md](PLANS/theming-substrate.md) — the ARC-1 theming substrate plan (engine/theme territory split + `.oatheme` distribution; absorbs Kiosk plan's 4-layer substrate). **ARC 1 complete** (Phases 1–6 shipped) bar the §6 Phase 5 `.oatheme` loader (absorbed into ARC 2). Arc table renumbered to 4 (D35).
- [PLANS/theming-arc-2-per-system-layout.md](PLANS/theming-arc-2-per-system-layout.md) — **Theming ARC 2 — Per-System Layout Substrate** (planned 2026-06-15). D32 per-system layout/view capability + persisted end-user override + D33 consumption opt-in + Per-System UI Stage 2/3 re-home + the `.oatheme` runtime loader. Fully declarative (no scripting/shaders — those are ARC 3). Slice order L1→P (D33 fix pulled forward); L1 queued in NEXT.md HIGH band. Decisions D34 (capability/content ownership) + D35 (arc split/renumber).
- [PLANS/theming-oatheme-loader.md](PLANS/theming-oatheme-loader.md) — **`.oatheme` runtime loader — declarative-first** (Theming ARC 2 "P", planned 2026-06-16; direction A locked). Load + switch + distribute themes that live on disk **without running author JS**: a built-in `DeclarativeShell` renders `theme.toml`+tokens+assets via the ARC 2 layout primitives; themes live at `<exe_dir>/themes/community/<id>/` and ship as the **`themes` pack type** on the content-pack channel (PD1–PD4). Custom-JS runtime loading + CSP allowlist deferred to P.2 (tees up ARC 3 Rhai). P.1 Slice 1 (Rust loader + discovery command) queued in NEXT.md HIGH band.
- [PLANS/unified-navigation-and-panels.md](PLANS/unified-navigation-and-panels.md) — **Unified Navigation & Panel System** (2026-06-14). Spatial-nav engine + a unified input-agnostic panel structure/look. **Phase 1 merged**; Phases 2-4 queued.

**Shipped plans (archived):** background-jobs-and-progress-bar, collections-tab-retroverse, controller-identity-substrate, disc-track-sha1-matching, discover-tab-retroverse, game-identities-schema, game-info-panel, launcher-abstraction, main-window, metadata-editing, play-now-tab-retroverse, retroverse-flag-deprecation, settings-declutter-system-health, settings-ia-redesign, settings-tab-retroverse, system-info-panel-v1, system-wiring-plan, theming-grabbag-drain, theming-platform-api-bridge. See [_archive/INDEX.md](_archive/INDEX.md).

## Setup reference (off-repo)

`C:\Users\Devilchi\.claude\plans\jazzy-spinning-blum.md` — project setup plan
(Cargo layout, Tauri+wgpu integration, license discussion, build/dev workflow).
