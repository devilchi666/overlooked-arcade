# Controller-Nav Coverage — Session Log

Newest first. Three lines per entry: **Shipped / Almost / Next**.

---

## 2026-06-13 — Slice 2: engine dialogs (2.0 / 2.1 / 2.2)

- **Shipped:** The lever — `Dialog.tsx` gains an opt-in `navigate` prop that
  mounts `useSettingsRowFocusGroup` over the dialog body (replacing the inert
  trap) + renders the select-overlay; default unset keeps the trap, so
  read-only dialogs are untouched. Added **radio** dispatch to the hook and
  bumped the select-overlay to `z-[80]` (above dialogs' `z-[70]`). Adopters:
  **2.0** GamePropertiesDialog (reference; one prop). **2.1** DebugLogDialog
  (filter/footer buttons + auto-scroll toggle marked; search + log body
  native/skipped), DiscPickerDialog (custom chrome → own vertical group +
  capture/restore + B-close; it's on the launch path), HelpDialogs (no change —
  read-only, inert trap is correct). **2.2** UnidentifiedGamesDialog (per-row
  "Show in folder" + footer), ScummvmDetectDialog (mode radios, pickers,
  per-row include/overwrite checkboxes, footer; descriptor text inputs stay
  native). OSK for text entry deferred ([OSK_PLAN.md](OSK_PLAN.md)). tsc clean;
  `vitest run src/platform/nav` green (44).
- **Almost:** Operator playtest pending for the Slice 2 adopters (Properties,
  Debug log, Disc picker, Unidentified, ScummVM detect) — verify walk + Confirm
  per control, select overlay opens above the dialog, B closes + restores
  focus. Branch `nav-coverage-slice2`, not yet merged.
- **Next:** Slice 3 — ImportWizard deep nav (table per-row pickers, step
  transitions, rule editor); then carousels + GameDetailPanel; OSK slice.
  Known v1 gap: GameProperties patch **Clear** + free-text fields aren't
  pad-reachable (keyboard escape hatch / OSK later).

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
  `vitest run src/platform/nav` green (44 tests). **Operator-verified working**
  in the real app. Follow-up fix this session: the focused-row highlight was
  invisible — `outline-none` (Tailwind `@layer utilities`) suppressed the
  shared `[data-oa-focus]` ring (`@layer components`); removed it and added an
  **unlayered** row highlight (accent background fill + ring, active/inactive
  variants) that beats the utility cascade. Operator confirms it now reads
  clearly.
- **Almost:** n/a — Slice 1 shipped + verified.
- **Next:** ~~Reset via Tertiary (Y)~~ **done** — taught `useDomQueryFocusGroup`
  to forward `onSecondary`/`onTertiary` (it silently dropped them), marked the
  SettingRow reset button `[data-setting-reset]`, and wired Y in the hook to
  click the focused row's reset (no-op mid-adjust / on rows with no override).
  The panel's `Tertiary: "Reset"` hint is now live. **Slice 1 merged to main
  (`--no-ff`).** Remaining: (b) **Metadata takeover** row-upgrade + game-pane
  `<select>` overlay (deferred this slice); (c) **Slice 2 — engine dialogs**,
  now scoped in [SLICE_2_PLAN.md](SLICE_2_PLAN.md): the lever is upgrading
  `Dialog.tsx` with an opt-in `navigate` prop that mounts
  `useSettingsRowFocusGroup` over the body (GameProperties as reference
  adopter), then easy/medium adopters; ImportWizard deep-nav splits to its own
  slice.

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
