# Per-System Settings Hub — Session Log

Newest first. Three lines per entry: **Shipped / Almost / Next**.

---

## 2026-06-14 — S3: Metadata domain (SystemMetaForm extraction)

- **Shipped (branch `feat/per-system-hub`):** the Metadata domain card is live.
  - `engine/SystemMetaForm.tsx` — the per-system metadata editor extracted from
    the 952-line `MetadataSettingsBody`: keyed by a single `systemId` accessor,
    owns the merged/curated/override resources + optimistic debounced autosave
    (mid-switch guard preserved) + the grouped `MetaField` form + `MetaField` /
    `LivePreviewHero` / `PeripheralEditor` sub-components + `FIELD_GROUPS` +
    provenance (`inheritedFor`/`effective`). Props: `systemId`, optional
    `showPreview`, `onSaved`. Persistence unchanged (`*_system_info_override`).
  - `MetadataSettingsBody` slimmed to the takeover shell (back · Systems/Games
    switch · preview toggle · `SystemList` rail) + `<SystemMetaForm>` (onSaved →
    refetch the rail's "edited" dots). Per-GAME (`MetadataGamePane`) untouched +
    OUT of the hub (DECISIONS D5).
  - `systemsHub/domains/MetadataEditor.tsx` = `SystemMetaForm` + live preview in
    a `PanelScaffold`. Metadata domain card enabled + wired in `SystemsHubRoot`.
  - typecheck + lint + vitest(97) + build green.
- **Almost:** the takeover's Games half still exists (per-game metadata) — stays;
  only the System half migrates. Takeover removed in S5 after parity.
- **Next:** operator playtest (edit a system's facts from its Metadata card →
  autosave; confirm the old Metadata takeover shows the same values). Then
  **S4 — BIOS + Input** domains.

## 2026-06-14 — S2: Media domain (artwork + game-data ops)

- **Shipped (branch `feat/per-system-hub`):** the Media domain card is live.
  - `systemsHub/gameMediaOps.ts` — `useGameMediaOps()` hook lifting the 6
    per-system ops (Identify / Sync covers / Sync metadata / Clear metadata /
    Refresh hash DB / Freshen) + busy state + the background-completion event
    listeners out of LibraryManagerPage. Self-contained (reads
    `usePlatform().library` + `useMedia()` + `useSystemsStats()`). Lives under
    `engine/` because it depends on the engine stats hook (platform↛engine
    boundary). Persistence + Tauri commands unchanged.
  - `engine/PlatformMediaSlots.tsx` — extracted the per-system art-slot grid
    (9 slots) from `PlatformMediaDialog`, controlled by a `systemId` accessor
    (no internal `<select>`). `PlatformMediaDialog` refactored to consume it
    (now just owns its system picker + Dialog chrome) — DRY, no behavior change.
  - `systemsHub/domains/MediaEditor.tsx` — Artwork (`PlatformMediaSlots`) +
    Game-data ops inline (5 op rows, accent/rose styling, busy-gated, count
    summary; progress via the global BackgroundJobsBar). Media domain card
    enabled + wired in `SystemsHubRoot`.
  - typecheck + lint + vitest(97) + build green.
- **Almost:** the "View N unidentified" affordance from the old Manage drawer
  isn't surfaced in the hub yet (deferred — needs `UnidentifiedGamesDialog`
  wiring); LibraryManagerPage still has its own op copy until S5.
- **Next:** operator playtest (run an op from a system's Media card; set/clear
  an art slot; confirm parity with the old Library→Game-media grid). Then
  **S3 — Metadata extraction** (`SystemMetaForm` from `MetadataSettingsBody`).

## 2026-06-14 — Arc planned + S1 shipped (nav-stack proof + Display & Core)

- **Shipped (branch `feat/per-system-hub`):**
  - **Plan + IA** locked via AskUserQuestion (DECISIONS D1–D5): two-level card
    model, clean replace, library-first + Show-all. Plan at
    [../../PLANS/per-system-settings-hub.md](../../PLANS/per-system-settings-hub.md).
  - **S1 code** under `frontend/src/engine/systemsHub/`: `useSystemsStats`
    (self-contained per-system stats hook); Pillar-B primitives `HubCard` /
    `HubGrid` / `PanelScaffold`+`HubSection`; `SystemsHub` level-1 grid
    (library-first + Show-all toggle) with `SystemCard` (identified/covers/
    metadata glyphs + accent stripe); `SystemHubDetail` level-2 grid of 6
    `DomainCard`s (Display & Core enabled, rest "Coming soon"); `SystemsHubRoot`
    owning the in-pane nav stack (grid→system→domain) + breadcrumb + the
    `pushBackHandler` Back integration + per-level initial focus. Two domain
    editors reuse `perSystemSections` verbatim: `DisplayVideoEditor`
    (Display/Rewind/Shaders + `usePerSystemOverrides`) and `CoreLauncherEditor`
    (`PerSystemDefaultCoreSection` + launcher select).
  - **SettingsPanel:** new `"systems"` category (CONTENT group) + `<Match>` arm
    → `SystemsHubRoot`. Old "Per-system"/"Media"/"Metadata" surfaces left LIVE
    in parallel (removed in S5).
  - typecheck + lint + vitest(97) + build all green. No backend changes.
- **Almost:** the SettingsPanel category header + the hub breadcrumb both show
  "Systems" at the grid level (minor redundancy; polish later).
- **Next:** operator playtest of the nav stack (drill grid→system→domain→editor,
  Back up each level, persistence parity vs the old Per-system surface). Then
  **S2 — Media domain** (lift game-media ops into `useGameMediaOps()` + extract
  `PlatformMediaSlots`).
