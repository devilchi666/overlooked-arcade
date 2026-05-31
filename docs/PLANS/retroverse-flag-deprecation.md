# Retroverse Flag Deprecation — Inventory & Plan

Audit pass to size the deletion PR that removes the legacy Shell once
the Retroverse flag flips default-on and runs one release cycle without
issues. Read-only audit at this stage — no code changes here.

Cross-references `docs/PLANS/retroverse-ui-rollout.md` §10 (the "what's
left" list, audited 2026-05-29 to be in sync with code reality). The
Library/Cores wraps that §10 calls out as SHIPPED are real — the
2026-05-30 question "what blocks killing the legacy Shell?" comes down
to dead-code removal mechanics rather than missing functionality.

---

## 1. The flag gate

- **Site:** `frontend/src/App.tsx:1824-2034`
  - `<Show when={isRetroverseUiEnabled()} fallback={<Shell …>…</Shell>}>`
  - Both branches sit inside the same set of providers (MediaProvider /
    PlatformMediaProvider / GameInfoBadgesProvider — lines 1816-1818).
  - Modals + dialogs after the closing `</Show>` (lines 2103-2289) work
    in both modes (ImportWizard, GameInfoModal, SaveSlotsModal,
    GamePropertiesDialog, the seven GameDialogs, AudioDialog +
    DisplayDialog + GameplayDialog + ShadersDialog,
    SystemBindingsDialog / SystemCoreOptionsDialog / SystemSettingsDialog,
    SettingsContextMenus, ToastStack, HintBar, PerformanceHud, etc.).
- **Accessor:** `frontend/src/lib/retroverseFlag.ts`
  - `isRetroverseUiEnabled()` + `setRetroverseUiEnabled(value)`.
- **Master toggle UI:** Settings → Display → Experimental →
  "Retroverse UI". Set via `ExperimentalSettings` in
  `frontend/src/components/SettingsSections.tsx`.

---

## 2. Files to delete entirely (legacy-only)

Confirmed by grepping for imports — every importer dies with the flag
removal.

| File | Lines | Importer count outside App.tsx |
|------|-------|--------------------------------|
| `frontend/src/layout/Shell.tsx`              | 85  | 0 |
| `frontend/src/layout/TopToolbar.tsx`         | 38  | 0 |
| `frontend/src/layout/RightSidebar.tsx`       | 246 | 0 |
| `frontend/src/layout/MenuBar.tsx`            | 604 | 0 |
| `frontend/src/layout/widgets/index.tsx`      | 120 | 1 (WidgetCustomizerDialog — see below) |
| `frontend/src/components/WidgetCustomizerDialog.tsx` | 175 | 0 |

Subtotal: **~1268 lines of strictly-legacy files**.

WidgetCustomizerDialog is opened only from the legacy `View → Customize
widgets…` MenuBar item (`App.tsx:1277`). Retroverse has no right
sidebar and no widgets — the dialog has no home in Retroverse and
should drop alongside the widgets registry.

---

## 3. Files that survive but lose legacy paths

### `frontend/src/App.tsx` (currently 2293 lines)

**Strip these blocks** (rough line counts; exact ranges shift after
each removal):

| Block | Lines | Notes |
|-------|-------|-------|
| `toolbarLeft` const + MenuBar JSX                       | 1201-1431 (~230) | 76 menu items total — Library / View / System / Settings / Help menus |
| `toolbarCenter` const                                   | 1433-1453 (~20)  | Search input is byte-identical to RetroverseShell's search input |
| `toolbarRight` const                                    | 1455-1517 (~63)  | See §5 migration items (Quit + Game-focus indicator) |
| Legacy `<Shell …>…</Shell>` fallback branch             | 1827-1990 (~165) | Wraps TopToolbar / LeftSidebar / RightSidebar + the Switch routing currentView to LibraryView / LibraryManagerPage / CoresPage |
| `openLibraryManager()` + `libraryManagerInitialTab` signal | 473-477 (~5)  | Only consumed by legacy LibraryManagerPage Match arm |
| `widgetCustomizerOpen` signal + dialog mount            | 272 + 2246-2250 (~6) | Goes with WidgetCustomizerDialog deletion |
| `overflowOpen` signal + handler                         | 470 + 1519-1526 (~10) | Legacy toolbar overflow menu |
| `TOOLBAR_BTN` const + 7 usages                          | (~10 cumulative) | Only used inside `toolbarRight` |
| Keyboard handler `library-manager` gates                | 718, 760 (~2)    | Legacy gates that prevent Quick Settings + Esc-handler in the library-manager page |
| Legacy HintRegion fallback hints                        | 2271-2286 (~16)  | Hardcoded `left-sidebar` / `library-grid` / `right-sidebar` focus-group names — Retroverse uses page-prefixed group names; switch simplifies to `default: {}` |
| Imports for `Shell` / `TopToolbar` / `LeftSidebar` legacy default / `RightSidebar` / `MenuBar` exports / `requestOpenFirstMenu` | 32-34, 40, 61, 101 (~8) | LeftSidebar's `type SidebarView` import stays; the default export of LeftSidebar ALSO stays (Retroverse LIBRARY page uses it) |
| `Show when={isRetroverseUiEnabled()} fallback={…}` wrapper | 1824, 1991-2034 | Becomes a plain `<Show when={!(isDirectLaunch() \|\| gameMode())}><RetroverseShell /></Show>` |

Subtotal: **~535 lines of legacy code paths** can be removed from App.tsx alone (bringing it from ~2293 to ~1760).

### `frontend/src/layout/LeftSidebar.tsx` (889 lines, stays — Retroverse reuses it)

- Drop the `library-manager` and `cores` variants from `SidebarView`
  (line 39). Verified no consumer outside legacy uses these kinds —
  `SidebarView` appears in 6 files:
  - `App.tsx` — only the legacy Match arms produce/consume these kinds.
  - `routes/retroverse/context.tsx` — re-exports the type; doesn't
    inspect kind.
  - `routes/retroverse/LibraryPage.tsx` — reads `cv.kind !== "view-node"`.
  - `library/filter.ts` — switches on kind (verify no `library-manager`/`cores` arm).
  - `routing/currentRoute.ts` — no kind inspection.
  - `components/LibraryView.tsx` — takes `currentView: SidebarView` as a
    prop (verify no kind inspection).

- Any internal handling of `library-manager` / `cores` kinds inside
  LeftSidebar itself (no current evidence — `Grep "library-manager"` in
  LeftSidebar.tsx returns no hits).

### Shared layout files that stay unchanged

| File | Why it survives |
|------|-----------------|
| `frontend/src/layout/LeftSidebar.tsx` | Retroverse LIBRARY page imports it (LibraryPage.tsx:25) |
| `frontend/src/layout/SidebarTreeNode.tsx` | Used by ViewEditorPane inside LibraryManagerPage (reachable from Retroverse SETTINGS → Library via the panel-mode wrap) |
| `frontend/src/layout/Dialog.tsx` | Dialog primitive — used by every dialog in `components/` |
| `frontend/src/layout/state.ts` | LayoutStore — used by every consumer of view mode / sort / group / sidebar visibility; Retroverse depends on it heavily |

### Components that have a legacy + Retroverse code path

| File | Legacy use | Retroverse use | Cleanup option |
|------|------------|----------------|----------------|
| `frontend/src/components/LibraryManagerPage.tsx` (1508 lines) | `variant="page"` rendered by legacy Match arm | `variant="panel"` rendered by `LibrarySettings` in SettingsSections.tsx | Optional: delete the `variant="page"` branch + remove the `variant` prop (~50 lines saved) |
| `frontend/src/components/CoresPage.tsx` (645 lines) | rendered with `onBack` from legacy Match arm | rendered with no-op `onBack` from `CoresCategorySettings` | The `onBack` handler is already a no-op in Retroverse mode; no parity work needed |

---

## 4. Files we may keep but flag for re-evaluation

- `frontend/src/components/SettingsSections.tsx::ExperimentalSettings`
  — currently hosts the Retroverse master toggle. The toggle UI
  obviously goes once the flag is permanently default-on. Whether the
  ExperimentalSettings category survives is operator preference (other
  experimental features may slot in here later).
- `frontend/src/components/SettingsSections.tsx` — the per-category
  `helpText` strings reference legacy access paths ("Reachable today
  via the menu bar's Library Manager…" at `SettingsPage.tsx:154`,
  "Theme picker…operator can switch presentation modes via the menu
  bar's Tools menu (legacy Shell only)" at `:144`). These need a sweep
  for stale prose.
- `frontend/src/lib/retroverseFlag.ts` — the accessor itself stays
  through the deprecation window. Becomes a permanent `true` after the
  legacy code is gone; delete after one more release cycle.

---

## 5. Migration items (must move to Retroverse before deletion)

Each of these is a small piece of functionality currently in legacy
chrome with no Retroverse equivalent. Pick a home for each, ship it,
THEN start the deletion PR.

| Status | Item | Currently in | Retroverse home | Cost |
|--------|------|--------------|-----------------|------|
| ✅ shipped 2026-05-30 | **Quit button** | `toolbarRight` (App.tsx:1505-1515) — fires `invoke("quit_app")` | `RetroverseShell` header `✕` button between clock and profile chip; new `onQuit` handler on `RetroverseContext`. Commit `494d1da`. | small |
| ✅ shipped 2026-05-30 | **Game-focus ON indicator** | `toolbarRight` (App.tsx:1457-1464) — pill that shows when keyboard passthrough is active | `RetroverseShell` header `<Show>`-gated accent pill between clock and Quit; new `gameFocus` accessor on `RetroverseContext`. Commit `494d1da`. | small |
| ⏳ verify (operator) | **Hide/Show library button (single-window mode)** | `toolbarRight` (App.tsx:1465-1478) — toggles `libraryVisible()` so wgpu pixels show through | Retroverse already has the equivalent gate at `App.tsx:2030` (`Show when={!(isDirectLaunch() \|\| gameMode())}`) — no explicit toggle button, the gate hides the entire shell on game-mode. Verify operator workflow during playtest cycle; if a toggle is needed, header button. | none-to-small |
| ⏳ verify (operator) | **Unload running ROM button** | `toolbarRight` (App.tsx:1479-1491) — fires `handleUnload()` | `Ctrl+W` keyboard shortcut works in both modes already (App.tsx:730-736). Optional: add a button to RetroverseShell header if playtest surfaces an actual operator need. | none-to-small |
| ✅ shipped 2026-05-30 | **Folder-drop overlay UI** | was rendered inside `<Shell>` at App.tsx:1973-1989 — only visible in legacy mode | Relocated to a sibling of the flag-gate `<Show>` so it overlays both shells; drop listener at App.tsx:1748-1769 was already window-global. Commit `c0bcacb`. | tiny |
| ✅ shipped 2026-05-30 | **Help → Debug log… + Keyboard shortcuts…** (discovered gap, not in original §5) | Legacy MenuBar Help menu at App.tsx:1424-1427 was the only entry point | Two buttons in `AboutSettings` → Report a bug card; new `onOpenDebugLog` + `onOpenKeyboardShortcuts` handlers on `RetroverseContext`. Commit `d8ce7b6`. | small |
| ✅ shipped 2026-05-30 | **Stale prose sweep** | `SettingsPage.tsx:144/154/455` + `LeftSidebar.tsx:568` referenced legacy access paths in misleading ways | Rephrased to drop "(legacy Shell only)" / "menu bar" tail references; fallback prose updated. Commit `d8ce7b6`. | tiny |
| ⏳ deletion PR | **WidgetCustomizerDialog** | `View → Customize widgets…` legacy menu | **Drop entirely** alongside the legacy `toolbarLeft` menu items that own it — deleting them together is cleaner than partial-state. ~175 lines (dialog) + ~120 lines (widgets registry, only used by `RightSidebar` which is also dying). | none |
| ⏳ deletion PR | **Show right sidebar button** | `toolbarRight` (App.tsx:1492-1504) — only relevant in legacy presentation modes | **Drop entirely** alongside the rest of `toolbarRight` — Retroverse has no right sidebar. | none |

---

## 6. Pre-conditions before the deletion PR starts

In order:

1. ✅ **Stale prose sweep** in `frontend/src/routes/retroverse/SettingsPage.tsx`
   (and SettingsSections.tsx category helpText) — shipped 2026-05-30
   in commit `d8ce7b6` on `feat/retroverse-migration-followups`.
2. ✅ **Migrate the small list in §5** (Quit / Game-focus / drop
   overlay; plus the discovered Help-dialog gap) — shipped 2026-05-30
   on `feat/retroverse-migration-followups` (commits `c0bcacb` +
   `494d1da` + `d8ce7b6`).
3. ✅ **Flip the flag default ON** — shipped 2026-05-31 on
   `feat/retroverse-flag-default-on` (single change at
   `frontend/src/settings/store.ts:139`,
   `DEFAULT_EXPERIMENTAL_RETROVERSE_UI: false → true`). The accessor
   shape at `lib/retroverseFlag.ts` was already reactive; only the
   store-side default needed to flip.
4. 🟨 **One release cycle of operator playtest** — in progress
   starting 2026-05-31. Operator can flip back OFF as the escape
   hatch via Settings → Display → Experimental → Retroverse UI for
   the duration of this cycle.
5. ⏳ **Confirm no remaining references to legacy `SidebarView` kinds**
   via grep — done as the first step of the deletion PR itself; the
   variants stay in place during the playtest cycle as cheap
   insurance for operators flipping back to legacy.

---

## 7. The deletion PR itself

Single focused branch. Order matters (some steps unblock others):

1. Drop the WidgetCustomizerDialog mount + signal + import + the
   `View → Customize widgets…` MenuItem.
2. Drop the legacy MenuBar items individually (each MenuItem references
   handlers / state; deleting them first lets the consts that own those
   handlers drop cleanly).
3. Drop `toolbarLeft` / `toolbarCenter` / `toolbarRight` consts +
   `TOOLBAR_BTN`.
4. Drop the `<Shell><TopToolbar/><LeftSidebar/><RightSidebar/>…</Shell>`
   fallback branch + the surrounding `<Show fallback={…}>` — replace
   with the unconditional Retroverse render block.
5. Drop `openLibraryManager` / `libraryManagerInitialTab` /
   `overflowOpen` / `widgetCustomizerOpen` signals.
6. Drop legacy imports from App.tsx (`Shell` / `TopToolbar` /
   `RightSidebar` / `MenuBar` + its sub-exports / `requestOpenFirstMenu`).
7. Drop legacy `SidebarView` kinds (`library-manager`, `cores`) from
   the type union in `LeftSidebar.tsx:36-40`.
8. Strip the keyboard handler's `currentView().kind === "library-manager"`
   gates (no-op now since the kind is gone).
9. Strip the legacy HintRegion fallback hints in App.tsx:2271-2286.
10. Delete the six legacy files in §2.
11. Optional cleanup: collapse `LibraryManagerPage.tsx` `variant` prop
    + the `variant="page"` branch (~50 lines).
12. Optional cleanup: same for any other `variant="page"|"panel"`
    component that no longer needs both modes.

---

## 8. Scope estimate

- App.tsx legacy branch + consts + signals: **~535 lines**
- Six legacy files (Shell / TopToolbar / RightSidebar / MenuBar /
  widgets / WidgetCustomizerDialog): **~1268 lines**
- LibraryManagerPage variant collapse + similar follow-up cleanups:
  **~100 lines**

**Total: ~1900 lines of dead code can be removed in the deletion PR**,
plus the surface-area shrink in `LeftSidebar.tsx` (a few kind variants
+ any branching on them).

---

## 9. Why this is safe

- The flag gate at App.tsx:1824-2034 is binary — no hybrid state, no
  shared rendering between modes. Either the legacy Shell renders or
  RetroverseShell renders, never both.
- Modals and dialogs live below the flag gate and work in both modes
  already.
- The Retroverse parity check at the top of this doc confirms every
  legacy feature has a Retroverse home except the small list in §5,
  which can be migrated in cheap follow-ups.
- §10 of `docs/PLANS/retroverse-ui-rollout.md` has been kept in sync
  with code reality through dated audits — it agrees with this
  inventory.

The risk is operator-side: someone has muscle memory for a menu-bar
path that goes away. The one-release cycle of flag-default-ON with the
escape hatch flip-back is the standard mitigation.
