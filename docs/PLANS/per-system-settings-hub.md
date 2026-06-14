# Per-System Settings Hub — card-based consolidation of the Settings "brain"

Planned + approved 2026-06-14. Builds on the merged Unified Navigation Phase 1
spatial engine. Off-tree origin: `~/.claude/plans/joyful-kindling-frog.md`.

## Context

Per-system settings are scattered across **five** Settings surfaces today, each
already keyed by `systemId` with stable persistence but reached from unrelated
places: the "Per-system" category (display/rewind/shaders/core/launcher +
bindings/core-options dialogs), per-system **Media** art slots, per-system
**Metadata** facts (a full-screen takeover), the **Library → Game-media** card
grid (Identify/Sync/Clear ops), and **BIOS** in System Health. The operator
loves the Game-media per-system **card** look and wants it to be the single,
logical home for everything about a system — Settings is "the entire brain."

**Goal:** one **Systems** hub. Grid of per-system cards → click a system → hub
of **domain cards** (Display & Video · Input · Core/Launcher · Media · Metadata ·
BIOS) → click a domain → its editor. **Replaces** the scattered surfaces; built
on the spatial-nav engine (zero per-control wiring); also delivers the
unified-nav **Pillar B** panel primitives as a byproduct.

**Locked decisions (operator, 2026-06-14):** (1) two-level card model;
(2) clean **replace** — remove the separate Per-system / Media / Metadata
categories + the Library→Game-media grid; (3) **library-first + Show-all**.

## Architecture

New dir `frontend/src/engine/systemsHub/`. The hub renders inside SettingsPanel's
`data-nav-region="settings-content"` (no new regions) — the spatial engine drives
it by geometry; the only nav wiring is a Back handler.

- **Nav stack** (in `SystemsHubRoot`): `grid → system → domain`, breadcrumb,
  one `pushBackHandler` registered while drilled in (pops a level before the
  takeover's onCancel), per-level initial focus.
- **Reuse map:** Display→`perSystemSections` (Display/Rewind/Shaders) +
  `usePerSystemOverrides`; Core/Launcher→`PerSystemDefaultCoreSection` + the
  launcher select; Input→existing `SystemBindingsDialog`/`SystemCoreOptionsDialog`;
  Media→extract `PlatformMediaSlots` from `PlatformMediaDialog` + the 5 ops from
  `GameMediaManagePanel`; Metadata→extract `SystemMetaForm` from
  `MetadataSettingsBody`; BIOS→per-system slice of `BiosSettings`.
- **Primitives (Pillar B byproduct):** `HubCard`, `HubGrid`, `PanelScaffold`+
  `HubSection`.
- **Stats:** `systemsHubStats.ts::useSystemsStats()` (self-contained over
  `usePlatform().library` + `useMedia()`); `LibraryManagerPage` keeps its local
  copy until its media tab is removed in S5.

## Slices (branch `feat/per-system-hub`; each shippable + playtestable)

- **S1 ✅ — nav-stack proof + Display & Core.** Systems grid + cards +
  library-first/Show-all + domain-card hub (Display & Core enabled, rest
  "Coming soon") + nav stack + Back + breadcrumb + `useSystemsStats`. Pure reuse
  of `perSystemSections`. Old "Per-system" sidebar entry left live in parallel.
- **S2 — Media domain.** Lift the 6 game-media op handlers from
  `LibraryManagerPage` into `useGameMediaOps()`; extract `PlatformMediaSlots`;
  `MediaEditor` = slots (scoped) + ops inline.
- **S3 — Metadata extraction.** Extract `SystemMetaForm({systemId})` from
  `MetadataSettingsBody`; per-game (`MetadataGamePane`) stays OUT.
- **S4 — BIOS + Input.** `BiosEditor` per-system slice + BIOS glyph; `InputEditor`
  (Bindings + Core-options launchers) + core glyph.
- **S5 — Remove old surfaces.** Delete the "Per-system" sidebar block, the
  `media`/`metadata` categories + Metadata takeover, the `media` tab from
  `LibraryManagerPage.TABS` (keep folders/views), `PerSystemSettingsBody.tsx`.
  Verify per-game metadata still reachable before deleting.

## Risks

Back-handler lifecycle (one balanced push/pop); `MetadataSettingsBody`
extraction (952 lines, re-key resource on systemId, keep provenance + autosave);
`PlatformMediaDialog` internal `<select>` → extract slots not `initialSystemId`;
game-media op handlers (700+ lines) move mechanically, keep `listenScoped`
cleanup; per-game metadata stays separate; Show-all renders zero-game systems;
**no backend edits** — reuse existing APIs + `usePerSystemOverrides.patch`
verbatim.

## Verification (per slice)

`cd frontend && npm run typecheck && npm run lint && npm run test && npm run
build` green; operator playtest in `cargo tauri dev` (drill grid→system→domain→
editor + Back up each level; UP/DOWN within a grid, LEFT/RIGHT to sidebar, Back
never closes the takeover prematurely); confirm each migrated domain persists
identically to the old surface (still live until S5).
