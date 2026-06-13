# Controller-Nav Coverage — Decisions

Per-stream architectural choices + the *why*. Project-wide decisions live in
`docs/DECISIONS.md`; this file is scoped to the nav-coverage sweep.

---

## 2026-06-13 — Slice 1: how Settings bodies opt into controller nav

**Reuse the existing center focus group; don't stand up per-body groups.**
`SettingsPanel.tsx` already had `engine-settings-center` wrapping the whole
center `<section>`, with the sidebar hand-off (`neighbours.left → sidebar`, and
the sidebar's `right → center`) and a MutationObserver already wired. The only
reason it felt dead was its default `"button"` selector — the category bodies
are `SettingRow`s rendering `<select>`/`<input>` controls, not `<button>`s, so
it matched ~0 rows. Widening that one group's selector lights up every category
at once and reuses the proven hand-off + the observer's category-switch
re-query. Per-body groups would have meant N registrations all racing for the
active slot and duplicating the hand-off wiring.

**Explicit opt-in selector (`[data-setting-row], [data-setting-action]`), no
bare `button`.** Several categories embed whole sub-pages (Library →
LibraryManagerPage, Media → PlatformMediaDialog, System Health →
SystemHealthPage, Cores). A bare-`button` selector would have half-wired those
sub-pages' buttons into the Settings group, colliding with their own (current
or future) nav. Requiring an explicit marker means embedded sub-pages
contribute nothing here until they're deliberately wired in a later slice —
they're audit gap #2/#3, not Slice 1.

**Confirm (A) semantics per control type — operator-chosen "proper path".**
The operator's locked nav spec reserves DPad/stick LEFT-RIGHT for region
transfer (sidebar ⇄ center), so left/right can't double as value adjustment in
normal nav. Resolution:
- toggle → click the checkbox (flip)
- button → click
- select → open a controller-navigable **overlay picker** (the same
  vertical-list focus-group pattern as `CorePickerMenu` / context menus the
  audit cites as proven) — up/down + Confirm to pick
- slider → enter **adjust mode**: up/right increment, down/left decrement,
  Confirm/Back exit. Adjust mode is an explicitly-*entered* state, so
  repurposing left/right inside it does not violate the locked spec (which
  governs *normal* navigation).

**`pushBackHandler` added to the back stack (`back.ts`).** Adjust mode must
intercept B to exit *before* any outer Settings-close handler already on the
stack (`popBack` fires most-recently-pushed first). `useBackHandler` is
mount-scoped and couldn't model a transient enter/exit state, so a dynamic
`pushBackHandler(fn) → disposer` was added; `useBackHandler` now builds on it.

**Slider step snaps to the grid (`nextSliderValue`, pure + unit-tested).**
Repeated gamepad steps on a fractional-step slider (bloom, 0.05) would
accumulate float drift; the value is re-snapped to `min + round((v-min)/step)*
step` each step and clamped to `[min,max]`.

**Metadata takeover deferred.** `MetadataSettingsBody.tsx` is almost entirely
free-text fields (not pad-editable) plus buttons its existing first-pass
`metadata-takeover` group already walks. Converting it to the row hook would
mean marking every button just to avoid regressing today's working button-nav,
for little controller-edit gain. Its row-level upgrade (+ the game-pane
`<select>` overlay) is a clean follow-up, not Slice 1.
