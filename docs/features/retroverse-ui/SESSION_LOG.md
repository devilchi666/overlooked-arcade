# Retroverse UI — Session Log

## 2026-05-29 — SETTINGS Per-system drill-in (last open category)

Closes the SETTINGS tab's last stub. Branch
`feat/retroverse-ui-settings-per-system`, 2 phase commits.

**Shipped:**

- **F1 — Section lift** (`c155dc9`): pulled the Display / Rewind /
  Shaders / Default-core section JSX out of `SystemDialogs.tsx` into
  a new shared file `frontend/src/components/perSystemSections.tsx`
  with a `usePerSystemOverrides` hook owning the fetch + patch
  cycle. Legacy `SystemSettingsDialog` becomes a thin wrapper that
  picks one section via the existing `section` prop. Bezel +
  Overscan editors + RewindLiveStats moved alongside. Same UX, no
  behavior change — the JSX moved verbatim. SystemDialogs.tsx
  re-exports helper types so existing import sites stay untouched.

- **F2 — Retroverse Per-system surface** (`7bb6fcf`):
  - Sidebar: replaces the "Per-system ▾" stub with an interactive
    expandable group. Click the header to toggle expand; when
    expanded, shows all 45 registered systems sorted alphabetically
    by display name, each with an accent dot + label. Clicking a
    system row activates the "per-system" category with that system
    picked.
  - Center pane: new `PerSystemSettingsBody` component renders all 4
    sections as inline cards plus two launcher buttons for the
    bigger editors (`SystemBindingsDialog`, `SystemCoreOptionsDialog`).
    Header card up top names the picked system + reminds the
    operator about the inheritance chain.
  - `CategoryId` gains `"per-system"`; the def lives outside the
    GROUP_ORDER iteration since the sidebar entry doesn't render as
    a flat button.
  - `usePerSystemOverrides` re-fetches when the systemId changes so
    swapping systems updates the cards in place. Monitors + cores
    lists eagerly fetched once the surface becomes active.

**Operator workflow now end-to-end:**

  1. SETTINGS tab → "Per-system ▾" in sidebar → click to expand.
  2. Pick any of the 45 systems.
  3. Edit Display / Rewind / Shaders / Default core inline; each row
     shows the inherited OA-wide value as a chip and flips to an
     override-color treatment when the operator sets a value.
  4. "Edit bindings…" + "Core options…" launchers open the existing
     focused-editor dialogs.

**Notes:**

- All 15 SETTINGS top-level categories now have real bodies, and the
  Per-system drill-in works for every registered system. The only
  remaining SETTINGS gap is content-driven (theme packs, etc.) which
  ships with Phase C6.
- The lifted section components are reusable. A future "per-game" tab
  could compose the same building blocks against a per-game override
  hook with minimal new UI.
- Bindings + Core options stay as dialogs by design — operator can
  always swap back to a different system from the sidebar without
  losing dialog state since the dialog mounts per-launch.

**Next:** Operator-chosen. Per the post-audit §10:
"Now playing" HintBar indicator (small, code-only),
DISCOVER tab body (depends on C6 packs),
Phase C6 content-packs infra (substantial),
RetroAchievements integration (external service).

— end of 2026-05-29 Per-system session.

## 2026-05-29 — Slice 12 custom collections (full feature)

Operator-built collection lists alongside the smart-list COLLECTIONS
surface shipped in Slice 11. Full feature in one branch
(`feat/retroverse-ui-slice-12-custom-collections`, 5 phase commits).

**Shipped:**

- **Slice 12A — Rust schema + Tauri commands** (`acb9877`). v14
  migration adds two SQLite tables: `custom_collections` (id PK, name,
  sort_order, created_at, updated_at) + `custom_collection_members`
  (collection_id FK ON DELETE CASCADE, rom_id, sort_order, added_at —
  composite PK). LibraryDb methods: list/create/rename/delete/add/
  remove + list_collection_members. INNER JOIN against games in the
  member-list query so dangling memberships from deleted roms don't
  surface. `delete_game` sweeps the junction. 8 unit tests cover the
  happy path + idempotent add + FK CASCADE + game-delete sweep +
  orphan filter via JOIN. 506 oa-shell tests green (was 497).

- **Slice 12B — frontend store** (`5bb60d2`).
  `createCustomCollectionsStore` in
  `frontend/src/library/customCollections.ts` mirrors the favorite /
  completed pattern: optimistic update + revert on failure. Member
  ids stored as `Map<collectionId, Set<romId>>` for O(1) "is X in
  collection Y?" lookups (TileContextMenu uses that per render).
  Plumbed through `RetroverseContext` so any future surface (HOME
  quick action, PLAY NOW filter) can read membership without prop-
  drilling.

- **Slice 12C — CollectionsPage MY COLLECTIONS sidebar + center pane**
  (`9136876`). Active-list state widens to a tagged union
  (`smart | custom`). MY COLLECTIONS sidebar group renders the
  custom list via `<For>`, with active highlight + member-count
  badge. Header card name / glyph / description / badge derive from
  a unified `headerInfo` memo — custom collections show "Custom ·
  editable" instead of "Built-in · read-only". Empty-state copy
  branches per active kind. v1 used `window.prompt` for the create
  flow — replaced in Slice D.

- **Slice 12D — NewCollectionDialog + TileContextMenu submenu**
  (`5baf42d`). New `NewCollectionDialog` component inside the Dialog
  primitive (inherits the menu-polish inert overlay + back-stack +
  focus restore). Auto-focuses the input on open. `seedRomId` prop
  lets the tile-menu path drop the right-clicked rom into the new
  collection on create — "make a list and add this game to it" in one
  dialog open. TileContextMenu gains an "Add to collection ▸"
  sub-view mirroring SystemContextMenu's main/move-category pattern:
  rows show ✓/○ for current membership; A toggles in place; tail
  entry "+ New collection…" opens the dialog seeded with this rom; B
  in sub-view returns to main (back-stack + onCancel both branch).
  Focus resets on view change; HintRegion adapts.

- **Slice 12E — Rename / Delete via sidebar right-click**
  (`dbdb5be`). NewCollectionDialog extends to a tagged `mode` prop
  (create | rename). Rename pre-fills + selects the current name.
  New `CollectionRowContextMenu` popover anchors on row right-click
  with Rename… and Delete… entries. Delete guards via
  `window.confirm`; if the deleted collection was the active list the
  view falls back to Favorites so the center pane keeps rendering.

**Operator workflow now end-to-end:**

  1. Right-click a tile in LIBRARY → Add to collection ▸ → + New
     collection… → name + Create. Game lands in the new list.
  2. Switch to COLLECTIONS tab → MY COLLECTIONS section shows the new
     list with a count badge → click to view members.
  3. Right-click the sidebar row → Rename… / Delete…
  4. Right-click any other tile → Add to collection ▸ → click an
     existing list to toggle membership in place.

**Notes:**

- Stale memberships are filtered in two places (delete_game sweep +
  INNER JOIN at list time) so the operator never sees a dangling row
  in the sidebar count or the center pane grid.
- Persisted member ordering follows add order via the `sort_order`
  column. Drag-reorder of memberships is a follow-up (the column is
  ready; the UI is not).
- The legacy (Retroverse OFF) shell doesn't surface the custom-
  collections UI — TileContextMenu hides the "Add to collection ▸"
  entry when the prop is absent. Slice 11's heart overlay + completed
  toggle stay legacy-visible.

**Next:** Operator-chosen. Open items in the rollout queue:
PLAY NOW placeholder moods (blocked on session-length tracking),
COLLECTIONS Hidden Gems + Last Played smart-lists (blocked on rating
data / play-order log), HOME carousel arrows / dot pagination,
SETTINGS Per-system category lift, Phase C6 content-packs infra.

— end of 2026-05-29 Slice 12 session.

## 2026-05-29 — Menu/dialog polish + sidebar DPad + dropped controllerNavSource

Follow-up to the same-day unified controller pipeline arc. Operator
queued three items as "polish where it still feels rough" and asked
to ship them in branch order. Branch
`feat/retroverse-ui-menu-polish`, 3 phase commits.

**Shipped:**

- **Phase 1 — Dialog primitive claims active** (`d619b7a`): the menu
  audit found that Dialog consumers (SettingsDialogs, GameDialogs,
  PlatformMediaDialog, GamePropertiesDialog, ScummvmDetectDialog,
  ImportArtPackDialog, SystemDialogs, WidgetCustomizerDialog,
  HelpDialogs, DebugLogDialog, ScreenshotGalleryDialog) used
  `captureFocusReturn` + `useBackHandler` but never activated a focus
  group, so while a Settings/Properties/Help dialog was up, A on the
  controller would route to whichever surface was last active behind
  the modal (typically the library grid → launching a tile). Added an
  inert focus group in `DialogBackHandler` that itemCount=0 / no-op
  handlers and an explicit `group.activate()` on mount; cleanup runs
  LIFO so the captured surface is restored. Monotonic id counter for
  stacked dialogs.

- **Phase 2 — LIBRARY sidebar DPad tree expand/collapse** (`ddd76a1`):
  with LibraryPage's page-level `LEFT_ID` claiming active in
  Retroverse mode, the legacy `left-sidebar` group's `onDirection`
  (DPad LEFT collapses / DPad RIGHT expands a container) no longer
  fired. Added `data-oa-sidebar-row` to the All Games / leaf-label /
  container-label / collapsed-leaf buttons + `data-oa-tree-node-id`
  + `data-oa-tree-node-kind` on tree rows; narrowed `LEFT_ID`'s
  selector to `[data-oa-sidebar-row]` so the walk skips twisty
  toggle buttons + the collapse-utility button. New `onDirection`
  on the page-level `LEFT_ID` reads the focused button's
  `data-oa-tree-node-kind` — on container rows, LEFT collapses
  expanded (consume) / RIGHT expands collapsed (consume); already-
  expanded RIGHT and already-collapsed LEFT fall through to the
  default DPad behaviour (RIGHT transfers to CENTER, LEFT no-ops).
  Leaf rows always fall through.

- **Phase 3 — Drop `controllerNavSource` setting** (`0931a41`):
  the legacy DPad / stick-left / both source filter was already a
  no-op after Phase 5 of the unified pipeline (each source carries
  different semantics now). Removed end-to-end: settings store
  type/persisted/signal/getter/setter/fallback/parser/save entry,
  SettingsDialogs + SettingsSections rows, App.tsx setNavSource
  round-trip, gamepad.ts NavSource type + setNavSource stub + two
  stale source-filter comments. Persisted `"stick-left"` / `"dpad"`
  values become inert on first save (key disappears from the
  serialized blob).

**Workspace tests:** 497 oa-shell + 19 + 20 + 24 + 16 + 1 across the
crate suites, all green.

**Notes for future polish:**

- The page-level `LEFT_ID` selector narrowing (skip the twisty
  buttons) doubles as a navigation ergonomics fix — operators now
  walk container rows as ONE item instead of two (twisty + label).
- `controllerNavSource` is gone but the broader
  `controllerNavEnabled` + `controllerNavSwapAB` +
  `controllerNavAnimationMs` settings stay — those still have meaningful
  effects.

**Next:** Operator-chosen from the Retroverse rollout queue —
Slice 12 custom collections, SETTINGS Per-system lift, Phase C6
content-packs infra, or content workstream.

— end of 2026-05-29 menu-polish session.

## 2026-05-29 — Unified controller pipeline (DPad transfers / stick walks)

Long-running arc fixing the controller-nav model end to end. Started
from operator complaint "DPad does nothing on LIBRARY", widened into
a full controller-pipeline audit, ended with an operator-approved
unified model where DPad transfers regions and the left stick walks
within regions. Merged as `1fcd522` (`--no-ff` from
`feat/retroverse-ui-unified-focus-spillover`, 16 phase commits, 23
files, +812 / -264 lines).

**Shipped (in merge order):**

- **Phase 1 framework** (`b972809`): `captureFocusReturn()` snapshot
  helper for menus/modals; manager activation-history stack so
  unmount picks last-known-good successor; `groupsVersion` reactive
  signal; `focusDomFor` drops the duplicate `scrollIntoView` that
  fought the virtualizer.
- **Phase 2 menu/modal migration** (`45e1809`): 8 hardcoded
  `activateFocusGroup("library-grid"/"left-sidebar")` cleanups
  migrated to `captureFocusReturn` (TileContextMenu / CorePickerMenu
  / RegionPicker / SaveSlotsModal / SystemContextMenu / QuickSettings
  / GameInfoModal / MenuBar).
- **Phase 3 per-page** (`85f394e`): LibraryPage initially dropped its
  page-level LEFT in favor of LeftSidebar's internal group; CENTER →
  library-grid delegating effect re-fires on `groupsVersion`;
  LeftSidebar consumes DPad-RIGHT on expanded containers; grid bind
  cleanup captures `flatIdx` at mount; CollectionsPage passes
  `gridFocusNeighbours`.
- **Phase 4 coverage** (`0587ac4`): DetailListView gains a focus
  group (shares "library-grid" id); ImportWizard gains a DOM-query
  focus group + `useBackHandler` + `captureFocusReturn`; Dialog
  primitive's `DialogBackHandler` adds save/restore so every dialog
  consumer inherits it.
- **DPad regression fix** (`9206890`): restored empty-group
  spillover, L1/R1 neighbours fallback, plumbed `autoClaim` through
  `useDomQueryFocusGroup` so `autoActivate: false` siblings don't
  win the first-registered race.
- **Phase 5 split** (`afbea1b`): rewrote `applyDirection` to branch
  on `event.source`. DPad activates `neighbours[direction]` (falls
  back to walking if no neighbour). Left stick walks within only —
  no spillover. L1/R1 explicit-opt-in (no neighbours fallback).
  Updated `HintBar` with new `dpad` + `stick` pseudo-glyphs.
  Controller-nav settings description updated.
- **Source filter drop** (`b50cddd`): the persisted
  `controllerNavSource = "stick-left"` was silencing every DPad press
  at the gamepad poller before events reached the focus framework.
  Removed the filter entirely under the new model — DPad and stick
  carry different semantics, so suppressing one is never correct.
  `setNavSource` becomes a no-op for settings round-trip.
- **HAT-axis DPad support** (`7cdfacd`): operator's controller
  (Faceoff Premiere Switch Pro, vendor 0e6f product 0184) advertises
  `mapping: ""`, 14 buttons (no DPad slots 12-15), 10 axes. DPad
  fires as a HID HAT switch on axis 9 with `(n-3.5)/3.5` encoding.
  Added `decodeHat`, `detectHatAxes`, `pollHat`. Detection is
  generic — any axis with idle value outside `[-1, 1]` at startup
  is tagged as a HAT axis. See
  [[reference_hid_hat_axis_decoding]].
- **Stale-active-group fix** (`0ae2600`): on route changes, the
  demote cascade left `activeGroupId` pointing at a stale
  (unregistered) sibling. `useFocusGroup.onMount` now auto-claims
  when `!currentIsRegistered` instead of `currentActive === null`.
  `manager.demote`'s fallback picks the MOST recent inserted group
  instead of the oldest. See
  [[reference_focus_framework_stale_active_group]].
- **LIBRARY unification** (`36fbbdb`): per operator request, LIBRARY
  now uses the same page-level LEFT_ID / CENTER_ID / RIGHT_ID
  pattern as HOME / COLLECTIONS / PLAY NOW / SETTINGS. LeftSidebar's
  internal `"left-sidebar"` group stays registered for legacy-shell
  use but doesn't compete because the page-level LEFT_ID claims
  active first. Grid `focusGroupNeighbours.left` updated to point
  at LEFT_ID. The delegating effect stays as the one LIBRARY-
  specific behaviour.
- **Diagnostic flag off** (`963a60e`): `FOCUS_DEBUG` defaults OFF
  for production. Instrumentation stays compiled in — flip
  `window.__oaFocusDebug = true` in DevTools to re-enable per-event
  logging.

**Key learnings (saved to memory):**

- [[retroverse-controller-nav-spec]] updated to reflect the new
  model. DPad = transfer, stick = walk. L1/R1 = tabs. B = back.
  A = enter. Same on every page; emulator mode the only exception.
- [[reference_hid_hat_axis_decoding]] documents the HAT axis pattern
  for future non-standard controllers.
- [[reference_focus_framework_stale_active_group]] documents the
  auto-claim-on-stale fix + the newest-first demote fallback.

**Almost / deferred to next session:**

- Operator hint at end of session: "we have to work on other menus
  down the road." Context menus (TileContextMenu / SystemContextMenu
  / RegionPicker / CorePickerMenu) and dialogs (GameDialogs,
  SettingsDialogs) work via the unified back-stack + `captureFocusReturn`
  now, but their internal navigation may still feel different from
  the page-level pattern. Polish pass needed: verify each menu's
  open/close + within-menu nav matches the unified model.
- LeftSidebar's tree expand/collapse via DPad on containers (was
  handled by its internal group's `onDirection`) is lost in
  Retroverse mode. Containers still expand via mouse click on the
  twisty or A-press navigation. Re-wiring this through the page-
  level LEFT_ID would need an `onDirection` that detects container
  rows and toggles expand/collapse.
- The `controllerNavSource` setting persists but is now a no-op.
  Could be removed in a follow-up cleanup once we're confident no
  operator wants the legacy behaviour.

**Next:** Operator-chosen. Most likely the menu/dialog polish pass
or returning to the deferred SETTINGS Per-system category lift +
Slice 12 custom collections from the Retroverse rollout queue.

— end of 2026-05-29 controller-pipeline session.

## 2026-05-28 — Full rollout to 6 operator-facing tabs + SETTINGS expansion

Massive session — designed + built the entire Retroverse UI from
zero. Pivoted through Phase A foundation, Phase B LIBRARY, Phase
C1-C4 per-tab implementations, HOME v2 operator-supplied mockup
redesign, three rounds of polish, and SETTINGS expansion.

**Shipped:**

- **Phase A foundation** (`1c4dee7`): experimentalRetroverseUi flag +
  `lib/retroverseFlag.ts` accessor; `play_time_secs` + `last_played_at`
  increment hooks via `close_active_session` helper; GameInfoModal →
  RightDetailPanel lift (later deleted in favor of GameDetailPanel);
  `currentRoute` signal + debug `__retroverse_debug` window globals.
- **Phase B LIBRARY** (`378863a`): RetroverseShell + top-tab strip +
  StubPage routing; LibraryPage 3-pane consuming existing LeftSidebar
  + LibraryView + RightDetailPanel via new RetroverseContext;
  HintRegion per page + shell-level L1/R1 cycle-tab via onNavEvent;
  fullBleed gate so launching a game lets the wgpu surface show.
- **Phase C1 SETTINGS** (`0671726`): 3-pane SettingsPage with 14
  category sidebar; 7 of 14 categories real
  (Display/Audio/Shaders/Gameplay/Controller-nav/Per-system-UI/Experimental).
- **Phase C3 COLLECTIONS** (`715f639`): favorite + completed
  library_db columns wired end-to-end; LibraryTile heart overlay;
  TileContextMenu add to favorites/completed; CollectionsPage with
  3 sidebar groups + 4 wired smart lists + 2 placeholders.
- **Phase C4 PLAY NOW** (`b2af79e`): PlayNowPage with hero +
  WHY-line generator + 3 rails + mood sidebar.
- **Phase C2 HOME v1** (`ca4ab04`): code-first skeleton (hero +
  Quick Launch + Recently Played + System Status gauges; right pane
  swapped on focus); controller-nav focus-group activation fix.
- **Controller-nav v2** (`71816bf`): operator-locked spec — DPad
  L/R = region transfer; L1/R1 = tab cycling. `useDomQueryFocusGroup`
  gained `onDirection` + `autoActivate`. System Status sysinfo
  persistent-handle fix + relocation to bottom-right pane as colored
  gauges.
- **LibraryPage focus-group port** (`6ea5e51`): aligned to operator
  spec via 3 page-level groups that override embedded sidebar/grid
  groups in Retroverse mode.
- **Right-pane redesign** (`6f24e4f`): GameDetailPanel +
  SystemInfoPanel ship as new components matching the operator-
  supplied library-default-mockup.png. RightDetailPanel.tsx deleted.
  SETTINGS dropped its live-preview right pane → 2-pane layout.
- **HOME v2** (`42da52f`): operator-supplied dense mockup redesign.
  Right pane: SYSTEM INFORMATION + TECHNICAL DETAILS + SUPPORTED
  PERIPHERALS + ACHIEVEMENTS cards. Center: massive hero + 6-card
  stats grid + popular-cover carousel + Recently Played panel. Left:
  systems list with era subline + Quick Launch panel at bottom.
  `systemMetadataStubs.ts` ships stub data (SNES verbatim, 6 priority
  systems hand-typed, rest "—").
- **Polish pass** (`0338501`): SETTINGS About / Storage / Themes
  categories filled in. Top toolbar wired (search → ctx.searchQuery
  + Enter → LIBRARY, live clock + date, profile chip routes to
  SETTINGS). LIBRARY header card (title + count) + opt-in per-tile
  system-label strip via `showSystemHeader` prop.
- **SETTINGS expansion** (`4a929bf`): Profile category (settings
  store gains profileDisplayName + profileAvatar; ProfileSettings UI
  with avatar preset row + freeform emoji input; toolbar chip reads
  the values). Cores category embeds CoresPage directly. BIOS
  category ships informational card surface. Library + Media remain
  informational placeholders pointing at legacy menu-bar surfaces.
- **Cleanup**: stale docstring refs to RightDetailPanel scrubbed
  across LibraryPage / CollectionsPage / HomePage / PlayNowPage /
  context / App.tsx.

**Almost (deferred — full list in
`docs/PLANS/retroverse-ui-rollout.md` §10 "Remaining work"):**

- Slice 12 — custom-manual collections (new SQLite tables + CRUD +
  sidebar dialog + TileContextMenu submenu). Code-only, well-scoped;
  best next-session pick.
- Phase C5 DISCOVER tab body (depends on C6).
- Phase C6 content-packs infrastructure (oa-packs Rust crate +
  Privacy panel + SETTINGS → Content panel + sha256 install/update
  flows + OA-curated GitHub registry).
- SETTINGS → Per-system / Library / Media full wraps (Per-system
  needs SystemSettingsDialog body lift; Library needs 5 store/callback
  props plumbed through RetroverseContext; Media needs variant="panel"
  lift on PlatformMediaDialog).
- BIOS live-presence grid (Rust get_bios_status command).
- PLAY NOW placeholder moods (Quick / Marathon / Challenge / Daily
  roulette) — need session-length tracking.
- COLLECTIONS Hidden Gems + Last Played smart-lists.
- HOME popular + recently-played carousel arrows + dot pagination.
- LibraryPage VirtualLibraryGrid 2D nav restoration in Retroverse
  mode.
- RetroAchievements integration OR local milestone tracking.
- "Now playing" audio indicator in HintBar.
- System Status panel — decide if/where to re-surface.

**Content workstream (operator-side):** per-system hero art files,
real metadata to replace systemMetadataStubs.ts approximations, real
per-system blurbs.

**Next:** Operator-chosen from the §10 list. Top picks by code-only
priority: Slice 12 → SETTINGS Per-system → Phase C6 content-packs.

— end of 2026-05-28 session, /clear scheduled.
