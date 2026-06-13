# Controller-Nav Coverage — Slice 2 scope: engine dialogs

Scoped 2026-06-13 from a read-only audit of the dialog surfaces (the audit
findings are baked into the per-dialog table below). Slice 1 (Settings bodies)
shipped + merged; this is audit gap #2.

## Goal

Make the engine's modal/form dialogs controller-navigable — today most are
keyboard/mouse-only inside. Reuse Slice 1's machinery; don't reinvent.

## The lever — upgrade `Dialog.tsx` once (like SettingRow was for Slice 1)

`platform/components/Dialog.tsx` already gives every `<Dialog>`-based modal an
**inert** focus group (`DialogBackHandler`): it traps controller input so
nothing leaks to the background, B/Esc closes, focus restores on close — but
`itemCount: 0` + no-op handlers, so it does **not** navigate the body.

**Slice 2 core:** add an opt-in `navigate` prop to `Dialog`. When set, Dialog
mounts a `useSettingsRowFocusGroup` (from Slice 1) over its body container (the
`max-h-[70vh] overflow-y-auto` div — needs a ref) **instead of** the inert
0-item group, and renders the select-overlay. B=close stays (back stack).
Default (prop unset) keeps today's inert behavior verbatim, so read-only
dialogs (Help/About) need zero change. This is the SettingRow-style
"fix-the-substrate" move: once Dialog can navigate, per-dialog work collapses to
"add `navigate` + mark the buttons."

Because the body group reuses `useSettingsRowFocusGroup`, dialog bodies get the
exact Slice-1 vocabulary for free: `SettingRow`s already carry `data-setting-row`
(toggle-flip / select-overlay / slider-adjust / Y-reset); stray buttons opt in
with `data-setting-action`.

Small dispatch extension needed: `useSettingsRowFocusGroup` currently handles
checkbox / select / range / button on Confirm. Add **radio** (`input[type=radio]`
→ click to select) for ScummvmDetectDialog's mode picker. ~3 lines.

## Recurring limitation (state it, don't fight it)

Free-text inputs — search boxes, folder paths, rule patterns, ScummVM
descriptors — can't be driven by a pad. They stay native: nav can focus them
(operator switches to keyboard) or skips them. This is the same keyboard escape
hatch the framework already assumes; call it out in each affected dialog rather
than pretending coverage.

**Decision 2026-06-13:** the proper fix — a controller-driven on-screen
keyboard — is **deferred to its own slice after Slice 2** (option A). See
[OSK_PLAN.md](OSK_PLAN.md). It hooks centrally into `useSettingsRowFocusGroup`'s
dispatch, so it retro-covers every text field once built; deferring it costs no
rework. Slice 2 ships text fields keyboard-only.

## Per-dialog work

| Dialog | Chrome | Today | Slice-2 work | Effort |
| --- | --- | --- | --- | --- |
| **GamePropertiesDialog** | `<Dialog>` | inert only; body is 2 SettingRows (core select + patch pick/clear) | add `navigate`; mark Pick/Clear buttons `data-setting-action` | **Easy** |
| **DebugLogDialog** | `<Dialog>` | inert only | add `navigate`; mark 6 filter buttons + 2 footer buttons + auto-scroll checkbox; search input stays native; read-only log body skipped | **Easy** |
| **DiscPickerDialog** | custom chrome (own backdrop) | raw Esc only, no group | doesn't get the Dialog upgrade — give it its own vertical `useDomQueryFocusGroup` over the disc-button list + Cancel (or migrate it onto `<Dialog navigate>`) | **Easy** |
| **HelpDialogs** (KeyboardShortcuts + About) | `<Dialog>` | inert only; read-only table/text | **none** — inert trap is correct; confirm B closes | **Trivial** |
| **LightGunHelp** | not a dialog (help component) | n/a | **none** | **N/A** |
| **UnidentifiedGamesDialog** | `<Dialog>` | inert only; vertical list of rows, per-row "Show in folder" + 2 footer buttons (list capped ~500) | add `navigate`; mark per-row button + footer `data-setting-action`; vertical walk handles the rest | **Medium** |
| **ScummvmDetectDialog** | `<Dialog>` | inert only; radios + folder picker + per-row results (checkbox + free-text descriptor) + footer | add `navigate` + radio dispatch; mark radios/folder/footer + per-row checkboxes; descriptors stay native | **Medium** |
| **ImportWizard** | custom chrome | **already** has `useDomQueryFocusGroup` over its buttons + back stack + capture | extend: the Step-2 results table's per-row system `<select>` → overlay picker (mark rows `data-setting-row`); sanity-check focus across step transitions + the ScummVM sub-dialog; rule-editor add/remove list | **Hard / its own slice** |

## Recommended sequencing

1. **Slice 2.0 — the Dialog upgrade** (the lever) + radio dispatch. Land it
   against **GamePropertiesDialog** as the reference adopter (smallest real
   form: SettingRows + 2 buttons). Verify the trap→navigate handoff (only the
   body group active; B still closes; focus restores).
2. **Slice 2.1 — easy adopters:** DebugLogDialog, DiscPickerDialog,
   HelpDialogs (confirm-only). Each is marker work once 2.0 is proven.
3. **Slice 2.2 — medium:** UnidentifiedGamesDialog, ScummvmDetectDialog.
4. **Slice 3 (split out) — ImportWizard deep nav:** table per-row pickers,
   step-transition focus, rule editor. High traffic but dense enough to
   deserve its own slice rather than riding Slice 2 — don't let it block the
   easy wins.

## Verification (each adopter)

Real app + the PDP Faceoff (`0e6f:0184`, DPad on HAT axis 9): open the dialog,
walk rows/buttons, Confirm per control type, B closes and **returns focus to
where it was** (the trap→restore is the part most likely to regress). Keep
`cd frontend && npx vitest run src/platform/nav` green; add a unit test if the
radio dispatch or any new pure helper warrants one.

## Out of scope (Slice 2)

- Carousels (Home/Play Now rails) — audit gap #3, later slice.
- GameDetailPanel panel-mode buttons — audit gap #4, later slice.
- The deferred **Metadata takeover** row-upgrade (tracked in SESSION_LOG).
- Driving free-text fields with a pad (keyboard escape hatch, by design).
