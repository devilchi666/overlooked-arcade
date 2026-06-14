# Per-System Settings Hub — Session Log

Newest first. Three lines per entry: **Shipped / Almost / Next**.

---

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
