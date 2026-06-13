# Controller-Nav Coverage — Session Log

Newest first. Three lines per entry: **Shipped / Almost / Next**.

---

## 2026-06-13 — Slice 1: Settings category bodies (OA-wide + per-system)

- **Shipped:** Controller row-nav for the Settings center pane. New reusable
  `useSettingsRowFocusGroup` (`platform/nav/settingsRowNav.tsx`) widens the
  already-wired `engine-settings-center` group from selector `"button"` to
  `[data-setting-row], [data-setting-action]` and teaches Confirm (A) per
  control type: toggle→flip, button→click, **select→overlay picker**
  (`SettingSelectOverlay`, the proven context-menu vertical-list pattern),
  **slider→adjust mode** (up/right +, down/left −, Confirm/Back exit). Marked
  `SettingRow`'s root with `data-setting-row` + `tabindex=-1` (one lever lights
  up Display/Audio/Shaders/Gameplay/Performance/Controller-nav/Per-system-UI/
  Experimental **and** the Per-system body via `perSystemSections`); marked the
  stray action buttons in those bodies + the per-system launcher select. Added
  `pushBackHandler` to `back.ts` (adjust-mode B-intercept), an adjust-mode CSS
  ring (`[data-setting-adjusting]`), and a unit test for the pure step math
  (`nextSliderValue`). Decisions in [DECISIONS.md](DECISIONS.md). tsc clean;
  `vitest run src/platform/nav` green (44 tests).
- **Almost:** Operator real-app verification still pending — open Settings →
  Controllers (test window) + drive the category bodies with the PDP Faceoff
  (`0e6f:0184`, DPad on HAT axis 9): row stepping, select overlay, slider
  adjust-mode, sidebar ⇄ center hand-off, B-to-exit.
- **Next:** After playtest sign-off — (a) **Reset via Tertiary (Y)**: the panel
  HintRegion already advertises `Tertiary: "Reset"` but it's dead (the
  `useDomQueryFocusGroup` wrapper doesn't forward `onTertiary`); wire Y to click
  the focused row's Reset. (b) **Metadata takeover** row-upgrade + game-pane
  `<select>` overlay (deferred this slice). (c) Slice 2 — engine dialogs
  (import wizard, per-game properties, Debug/Help).

## 2026-06-13 — Stream queued

- **Shipped:** Nothing in code yet — stream created. Scope + prioritized gap
  table + standard recipe captured in [README.md](README.md), backed by the
  [nav-coverage audit](../controller-identity/NAV_COVERAGE_AUDIT_2026-06-12.md)
  run during the controller-identity arc. Queued in NEXT.md HIGH band.
- **Almost:** n/a — paperwork only.
- **Next:** **Slice 1 — Settings category bodies.** Wire row-by-row controller
  nav into the per-category Settings forms (`SettingsSections.tsx`,
  `PerSystemSettingsBody.tsx`, `MetadataSettingsBody.tsx`) — the sidebar already
  navigates; the bodies don't. Use `useDomQueryFocusGroup` with a row selector;
  verify with the controller test window (Settings → Controllers) + a real pad.
