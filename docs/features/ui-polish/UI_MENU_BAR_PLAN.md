# UI Redesign — Menu-Bar Architecture

> **STATUS: 🚧 SUBSTANTIALLY SHIPPED via `UI_POLISH_PLAN.md` (2026-05-22).** The intent architecture (Library / View / System / Game / Tools / Settings menus organized by tier) is live via the dialog refactor: SettingsPage → LibraryManagerPage rename, sidebar Cores/Settings buttons removed, SystemDialogs.tsx organized by intent, PerGameSettingsDrawer shrunk. Visual top-of-window menu-bar component itself is not the canonical home today — the menu-organized dialogs do the work. Kept as historical reference for the IA reasoning.

**Date:** 2026-05-18
**Status:** Proposal · planning only · no code (original; see banner above)
**Companion:** sibling `UI_AUDIT.md` (inventory of today's UI surfaces)

## Why this shape

The audit identified three problems:

1. Too many parallel doors to the same room (per-system settings reachable 5 ways; cores in 4 places; region picker in 3).
2. `QuickSettings` overlay tries to be both a pause menu and a power-user tool console.
3. Aspirational UI leaking into production — disabled "soon" buttons, placeholder tabs, "scaffold" fields that don't take effect.

Research into the three reference frontends informed the choice:

- **LaunchBox (desktop)** — classic Windows menu bar (File · Edit · View · Tools · Help). Tools menu is the heavy one. Settings spread across menus by intent.
- **BigBox (LaunchBox big-screen)** — sectioned wheel/carousel, controller-first. Relevant for our Cabinet mode only.
- **RetroArch Ozone** — three-column sidebar with 22-category Settings tree.
- **RetroArch XMB** — horizontal top tab row + vertical drill-downs. 11 top tabs.
- **RetroArch Quick Menu** (in-game) — one flat ~17-item menu, no sub-tabs.

We adopt **LaunchBox's menu-bar discipline** (named menus by intent, opening focused dialogs) with the visual ceiling of Spotify/Heroic, not Win32. Each setting tier (OA-wide / per-system / per-game) gets its own menu so the tier is **obvious by which menu the user is in**.

## Visual treatment

Stylized text. No icons in the bar itself.

```
◐  Library   View   System ▾   Game ▾   Tools   Settings   Help        [ search games… ]              ⋯  ✕
```

- **Type:** 0.75rem, medium weight, sentence case (not ALL-CAPS — that's our chip/label register).
- **Default:** `--color-oa-ink-dim`.
- **Hover:** `--color-oa-ink` + subtle `white/[0.04]` background tint, `rounded-md`.
- **Open (menu pulled down):** `--color-system-accent` text + persistent tint.
- **Disabled** (System/Game with no context): `--color-oa-ink-dim` at 40%, `cursor: default`, tooltip explains why.
- **No pipes or borders** between menus. Spacing alone separates — 1rem gap.
- **Caret `▾`** on contextual menus (System, Game) only, at 0.6rem / 70% opacity, flush to the label.

The bar stays visible during gameplay (idle-fades like today). It becomes the always-available pause-menu surface.

## Surface taxonomy

Following the "wide things stay full-page" rule:

| Class | Used for | Surfaces |
|---|---|---|
| **Full page** (route) | Wide, list-heavy, multi-section content with grids and progress bars | Library Manager · Cores Manager |
| **Modal dialog** | Configurable forms with 3–10 fields, or specialized tools (memory inspector, TAS, video) | Display · Audio · Gameplay · Shaders · Bindings · Core options · Theme · Memory inspector · TAS recorder · Video capture · Cheats · Milestones · Properties · Save slots · Game info · Region picker · Import wizard · Screenshot gallery |
| **Inline** (popover / toggle in the menu itself) | Single-radio / single-toggle state | View mode · Sort · Group · Mode · Sidebar toggles · Hide system · Auto-hide empty · Auto-remove on delete · Performance HUD · Shell mode · Core override · Default core |

Two full pages total. The current 7-tab `SettingsPage` + 8-tab `PerSystemSettingsPage` + 11-tab `PerGameSettingsDrawer` collapse into menu-launched modals.

---

## Menu contents

### `Library`

| Item | Today's home | New target |
|---|---|---|
| Import folder… | Toolbar `⋯` | Opens `ImportWizard` |
| Rescan tracked folders | Toolbar `⋯` | Action |
| Library Manager… | Settings → Library | Opens full page |
| Cores Manager… | Settings → Cores + `CoresPage` | Opens full page (one canonical home) |
| Sync media… | Settings → Game media | Jumps to Library Manager → Media |
| Region priority… | Settings → Game media → Region priority | Jumps to Library Manager → Region priority |
| Hide system → submenu | Settings → Library checkboxes | Submenu of per-system checkboxes |
| Auto-hide empty systems | Settings → Library / Presentation | Checkbox toggle |
| Auto-remove on file delete | Settings → Library | Checkbox toggle |
| Clear games for system → submenu | Settings → Library | Submenu of systems (destructive, confirm) |
| Reset entire library… | Settings → Library | Action (confirm dialog) |

### `View`

| Item | Today's home | New target |
|---|---|---|
| View mode → Capsule / Detail | `GridControls` toggle | Radio submenu |
| Sort by → Name / Date added / Year | `GridControls` dropdown | Radio submenu |
| Group by → None / Letter / System | `GridControls` dropdown | Radio submenu |
| Left sidebar (Ctrl+B) | Settings → Presentation | Checkbox toggle |
| Right sidebar | Settings → Presentation | Checkbox toggle |
| Customize widgets… | (orphan) | Opens dialog |
| Mode → Desktop / Theater / Cabinet | Settings → Presentation + `⋯` | Radio submenu |

`GridControls` loses its sort/group/view dropdowns; the bar above the library shrinks to title + count.

### `System ▾`  *(disabled when no system context)*

Source of context: sidebar selection, or system-filtered library view.

| Item | Today's home | New target |
|---|---|---|
| (label: system display name) | — | Header row, dimmed |
| Show library | Left-click sidebar system | Action |
| Bindings… | PerSystem → Input | Opens `SystemBindingsEditor` modal |
| Default core → submenu | PerSystem → Cores | Submenu (cores + Auto-detect) |
| Shaders… | PerSystem → Shaders | Opens Shaders modal |
| Core options… | PerSystem → Core options | Opens `CoreOptionsPanel` modal |
| Display overrides… | PerSystem → Display (scaffold) | Opens dialog **OR hide until wired** |
| Audio overrides… | PerSystem → Audio (placeholder) | **Hidden until wired** |
| Rewind overrides… | PerSystem → Rewind | Opens dialog |
| Theme… | PerSystem → Theme (placeholder) | **Hidden until wired** |
| Hide from sidebar | `SystemContextMenu` | Action |

Placeholder tabs disappear from the UI entirely. `SystemContextMenu` (right-click) mirrors this list so there's one canonical action set.

### `Game ▾`  *(disabled when no game context)*

Source of context: right-sidebar focus, currently-running ROM, or last-right-clicked tile.

| Item | Today's home | New target |
|---|---|---|
| (label: game title) | — | Header row, dimmed |
| ▶ Launch / 🚪 Exit to library | Tile click / Unload / QuickSettings | Action (context-aware) |
| Save states… | Tile menu / sidebar / GameInfo / QuickSettings | Opens `SaveSlotsModal` |
| Game info… | Tile menu / sidebar | Opens `GameInfoModal` |
| Properties… | Tile menu "Game properties" | Opens `PerGameSettingsDrawer` |
| Core override → submenu | Tile menu "Change core" | Submenu (replaces floating `CorePickerMenu`) |
| Cheats… | (placeholder tab) | Opens Cheats dialog ★ surfaced |
| Milestones… | PerGame → Milestones | Opens Milestones dialog |
| ROM patch… | PerGame → Core (half-wired) | Opens patch picker ★ surfaced |
| Shaders… | PerGame → Shaders | Opens dialog |
| Pick region… | Tile menu / PerGame Region / GameInfo strip | Opens `RegionPicker` (one canonical door) |
| Pick cover file… | Tile menu | File picker |
| Clear cover | Tile menu | Action |
| Remove from library | Tile menu | Action (destructive, confirm) |

`TileContextMenu` survives — mirrors this list so right-click works the same as the menu.

### `Tools`

In-game power tools, always visible.

| Item | Today's home | New target |
|---|---|---|
| Memory inspector | QuickSettings drill-in | Modal (more room than overlay) |
| TAS recorder | QuickSettings drill-in | Modal |
| Video capture | QuickSettings drill-in | Modal |
| Screenshot gallery | (orphan — F12 saves nowhere visible) | Modal ★ surfaced |
| Performance HUD (toggle) | (orphan) | Checkbox toggle ★ surfaced |
| Disc control… | QuickSettings drill-in (when multi-disc) | Dialog (enabled only when applicable) |
| Rewind config… | QuickSettings inline / Settings → Gameplay | Dialog |

### `Settings`  (OA-wide only)

| Item | Today's home | New target |
|---|---|---|
| Display… | Settings → Display | Dialog (scaling, window, monitor, run-ahead, bloom) |
| Audio… | Settings → Audio | Dialog |
| Gameplay… | Settings → Gameplay | Dialog (rewind enable / interval / buffer) |
| Shaders… | Settings → Display (shader fields) | Dialog (preset + bloom) |
| Shell mode → submenu | Settings → Display → Shell mode | Submenu (Two-window / Single-window) |
| Open settings file… | — | Reveals `appDataDir` in explorer |

### `Help`

| Item | Today's home | New target |
|---|---|---|
| Keyboard shortcuts… | (orphan — invisible) | Cheatsheet modal ★ surfaced |
| About | — | About modal |
| Open logs folder | — | Reveals folder |

---

## What gets deleted or simplified

- **`SettingsPage` route deleted.** Today's 7-tab page no longer exists.
- **`PerSystemSettingsPage` route deleted.** 8-tab page becomes 4–5 modals reached from `System ▾`. `SystemHeader`'s quick-action chips disappear.
- **`PerGameSettingsDrawer` simplifies.** Either fully retires (each tab becomes a Game-menu modal) or stays as one "everything about this game" surface reachable only from `Game → Properties` — and its 11 tabs shrink to 4–5 since standalone modals handle the rest.
- **`QuickSettings` overlay simplifies to verbs.** Resume · Save state · Load state · Game info · Exit. Drill-ins move to the always-visible Tools menu.
- **`GridControls`** loses sort/group/view dropdowns; bar above the library shrinks to title + count.
- **Toolbar `⋯` overflow** loses Import / Rescan / Mode / Sidebar (all in menus). May not need an overflow at all.
- **Sidebar bottom buttons** (`Cores`, `Settings`) — gone.

## Orphans surfaced as a side-effect

| # | Orphan | New home |
|---|---|---|
| 1 | Cheats CRUD + Cheat Search | `Game → Cheats…` |
| 2 | ROM patch picker (`pick_patch_file`) | `Game → ROM patch…` |
| 3 | Screenshot gallery (F12 output) | `Tools → Screenshot gallery` |
| 4 | Performance HUD | `Tools → Performance HUD` (toggle) |
| 5 | Keyboard shortcuts cheatsheet | `Help → Keyboard shortcuts…` |
| 6 | Right-sidebar widget customization | `View → Customize widgets…` |

## Mapping of every current settings field

For audit traceability — every field listed in sibling `UI_AUDIT.md` §5 has a new home above. Summary by source surface:

- **`SettingsPage` (OA-wide, 7 tabs)** → `Settings ▾` (Display / Audio / Gameplay / Shaders / Shell mode) + `Library ▾` (Library Manager + Cores Manager + Game media routes) + `View ▾` (Presentation toggles).
- **`PerSystemSettingsPage` (8 tabs)** → `System ▾` items, each opening a focused modal.
- **`PerGameSettingsDrawer` (11 tabs)** → `Game ▾` items, each opening a focused modal. `Properties…` retains the consolidated drawer for power users.
- **`QuickSettings` (overlay)** → drill-ins relocated to `Tools ▾`; the overlay keeps only verbs.

---

## Caveats / open questions

1. **Disabled menus need to teach.** A dimmed `System ▾` with no explanation is a dead end. Minimum: hover tooltip ("Pick a system in the sidebar"). Better: a one-time onboarding hint.
2. **Long submenus.** "Hide system → 9 platforms" is workable; "Default core → many cores" could get long once we ship more. Search-as-you-type inside submenus is a follow-up.
3. **In-game discoverability.** New users may not realize the menu bar works during gameplay. Consider a one-time hint on first ROM launch.
4. **Cabinet mode** has no menu bar (controller-first). The menus need to mirror to a controller-driven equivalent. Out of scope for v1; the existing Cabinet skin keeps working.
5. **`Properties…` redundancy.** If every PerGame tab also gets a top-level Game-menu modal, the consolidated drawer becomes a backup surface. Decide whether to retire the drawer entirely or keep it as a "everything in one place" view.
6. **Display overrides scaffold.** Per-system Display fields don't take effect today. Decide before menu items ship: wire them, or hide them.

## Roll-out order (for when planning ends)

1. **Trim placeholders + disabled items** in today's UI. Removes "half-finished" noise. Pure deletion.
2. **Build the menu bar shell** (visual treatment, popover/dropdown primitive, disabled state, keyboard accessibility). No content yet.
3. **Move `View` items** into the menu (presentation, sort, group, view mode). Lowest risk — pure relocation, no new dialogs.
4. **Move `Settings`** into menu-launched dialogs. Delete the `SettingsPage` route.
5. **Move `System`** into menu-launched dialogs. Delete the `PerSystemSettingsPage` route. Update `SystemContextMenu` to mirror.
6. **Move `Game`** into menu-launched dialogs. Decide drawer fate. Update `TileContextMenu` to mirror.
7. **Move `Tools`** drill-ins out of `QuickSettings`. Simplify `QuickSettings` to verbs.
8. **Build `Library`** and **`Cores`** Manager full pages. Largest single piece of work — consolidates several existing surfaces.
9. **Wire surfaced orphans** (Cheats, ROM patch, screenshot gallery, performance HUD, shortcuts).
10. **Visual polish pass** — type ramp, accent usage, icon set replacement.

## References

- [RetroArch Ozone Interface — Libretro Docs](https://docs.libretro.com/guides/ozone/)
- [RetroArch XMB Menu Map — Libretro Docs](https://docs.libretro.com/guides/xmb-menu-map/)
- [RetroArch Menu Styles — Libretro Docs](https://docs.libretro.com/guides/gui/)
- [BigBox Themes and Where They Apply — LaunchBox](https://feedback.launchbox.gg/en/help/articles/9915075-big-box-themes-and-where-they-apply)
- [LaunchBox Customizing menus thread](https://forums.launchbox-app.com/topic/74243-customizing-menus/)
