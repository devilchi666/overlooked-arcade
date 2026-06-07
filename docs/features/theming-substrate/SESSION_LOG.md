# Theming Substrate — Session Log

Each session that touches this feature appends a 3-line entry:
**Shipped / Almost / Next.** Older entries roll to
`SESSION_LOG_ARCHIVE.md` when this file passes ~150 lines.

---

## 2026-06-06 — Planning locked

- **Shipped:** Full plan + feature folder scaffold. Plan at
  [docs/PLANS/theming-substrate.md](../../PLANS/theming-substrate.md);
  feature folder this directory. Operator decisions: one unified
  premium frontend (no LaunchBox/BigBox split); engine vs theme
  territory inside one window; engine summon = fullscreen takeover,
  top-right corner, `F12` / `Select+Start`; manifest = TOML; Kiosk
  plan's 4-layer substrate absorbed (renamed). 3-arc structure
  (ARC 1 = Minimum Viable Substrate, ~22-26 weeks; ARCs 2-3 add
  Rhai + WGSL + Theme Studio).
- **Almost:** Nothing — pure planning session, no code touched.
- **Next:** Phase 1 of ARC 1 — engine/theme surface separation.
  Extract SETTINGS + Library Manager + Import Wizard + BIOS +
  Core installer + System Health + Background Jobs from
  Retroverse into engine-owned fullscreen takeover. Write
  SURFACES.md as part of the phase. See plan §6 Phase 1 for the
  full deliverable list + acceptance gate.

---

## 2026-06-06 — ARC 1 Phase 1 shipped

- **Shipped:** Engine/theme surface separation per plan §6 Phase 1.
  Branch `feat/theming-substrate-phase-1` cut from
  `5695adb`; snapshot tag `v0.x-pre-theming-substrate` + branch
  `pre-theming-substrate` both at the same commit as restore points.
  - **SURFACES.md** written first (scope-lock checkpoint) — surface-by-
    surface engine/theme/platform territory map + 5-dialog migration
    table + 3 summon affordances + residual ~12 dialog signals
    deferred to Phase 2.
  - **`platform/engineSurface.ts`** — engine surface visibility signal
    (`engineSurfaceOpen`, `openEngineSurface`, `closeEngineSurface`,
    `toggleEngineSurface`) + `wireEngineSummonChord()` for the
    Select+Start chord recognizer (600ms window, respects setNavEnabled).
  - **`platform/dialogs.ts`** — 5 dialog signals migrated out of
    App.tsx createSignals: `savesEntry`, `contextMenuFor`,
    `gameInfoFor`, `helpDialog`, `wizardOpen`. Each exports a Solid
    Accessor + a value-form setter. App.tsx destructures them so all
    existing call sites read + write through identical names.
    Per operator decision 2026-06-06: Platform owns open/close; themes
    pick where dialogs anchor. Phase 1 ships the state migration;
    theme-chosen anchors land in Phase 6 with Retroverse-as-theme.
  - **`engine/SettingsPanel.tsx`** lifted from
    `routes/retroverse/SettingsPage.tsx` — identical UX, same
    three-pane layout, same 14 category bodies + per-system drill-in
    picker. Still pulls `settings` via `useRetroverse()` for Phase 1
    (engine surface mounts inside RetroverseProvider while only one
    theme exists; Phase 2 splits to PlatformProvider).
  - **`engine/EngineManagerSurface.tsx`** — z-[60] fullscreen takeover
    rendered when `engineSurfaceOpen()`, with header bar (back button
    + "OA Settings" label) + body hosting `SettingsPanel`. Escape /
    back button / F12 close.
  - **`engine/EngineSummonIcon.tsx`** — gear-icon button themes mount
    in their top-right slot per D3.
  - **App.tsx** wired all three summon affordances: F12 hotkey in
    existing keydown handler (toggles when engine-open OR not gaming;
    falls through to emu-thread screenshot when game is running and
    surface is closed); Select+Start chord via
    `wireEngineSummonChord()` at mount; `<EngineManagerSurface />`
    mounted inside `<RetroverseProvider>` after the conditional Show so
    the surface stays summonable across gameplay.
  - **RetroverseShell** dropped SETTINGS tab (6 → 5 tabs:
    HOME / LIBRARY / COLLECTIONS / PLAY NOW / DISCOVER), mounted
    `<EngineSummonIcon />` in top-right cluster next to clock + quit +
    profile chip, gated L1/R1 tab-cycler on `!engineSurfaceOpen()` so
    L1/R1 inside Settings doesn't bleed through, profile chip click now
    opens engine surface (was: routed to SETTINGS tab).
  - **`routing/currentRoute.ts`** dropped the `"settings"` arm from
    `RetroverseRoute` + `RETROVERSE_ROUTES`. Header comment updated
    to "5 top-toolbar tabs."
  - `routes/retroverse/SettingsPage.tsx` deleted (orphaned after the
    lift).
  - Acceptance gate green: `cargo test -p oa-shell` 744 pass /
    0 fail; frontend `npm run typecheck` silent; SURFACES.md
    locked the boundary before refactor started.
- **Almost:** Operator playtest — F12 / chord / corner-icon round-trip
  + per-system drill-in equivalence + visual regression vs old SETTINGS
  tab is the pending validation step before merging to main.
- **Next:** Phase 2 of ARC 1 — Platform/Theme SDK foundation. Pull
  `frontend/src/platform/` out as a top-level dir with the
  `@oa/platform` Vite alias; move the stores + lib helpers + theme
  registry in; carve `ThemeContext` (rename of `RetroverseContext`);
  cleanup `HOTSPOT_SYSTEMS` triplicate + `customComponent` orphan;
  migrate residual ~12 dialog signals listed in SURFACES.md "Open
  boundary questions" section. Phase 1-2 run parallel with VL Phase A
  per plan §7; pause at end of Phase 2 for VL Phase E + C.
