# Main Window Plan

**Date:** 2026-05-17
**Sibling of:** `docs/RESEARCH/launcher-landscape.md` (research, opinions) — this doc is the actionable plan that translates the research into geometry + delivery order.
**Status:** Draft for human review. Decisions flagged inline; open questions consolidated in §12.

---

## Where we are today

The current shell is functional but flat: one top header with system buttons inline, one big LibraryGrid (no virtualization), one tabbed modal for all OA-wide settings, and per-system pages reached by clicking a system button in the header. Modes that exist:
- Two-window vs single-window shell (decided 2026-05-16).
- Game-mode idle timer hides the header + cursor after 3s during single-window gameplay.
- Settings modal already does 5 tabs (Display / Audio / Cores / Library / Game media) with the Steam-style left rail.

What we don't have yet, which this plan addresses:
- Persistent **left sidebar** with reorderable system list, playlists, "Recently played" rail, custom views.
- Persistent or pinnable **right sidebar** with focused game metadata, customizable widgets.
- **Top toolbar** that lives as more than a header — search, navigation breadcrumbs, mode toggle, profile, idle-hide rules.
- **Library grid view modes** (capsule grid / horizontal hero / list / details / wall).
- **Import flow** with per-folder rules, scan preview, background progress, scheduling.
- **Cabinet mode** — BigBox-style controller-first 10-foot UI with attract mode and marquee output.
- **Settings as a page** for the three-tier split (OA-wide / per-system / per-game), with one consolidated entry point.

---

## 1. Three target presentations

OA must render the same library state in three distinct visual modes. Picking the right one per-context is the single highest-leverage UX decision in this plan.

| Mode | Input | Window | Audience | Use case |
|---|---|---|---|---|
| **Desktop** | Mouse + keyboard primary | Windowed or borderless on desktop OS | Everyday PC user at a desk | Browsing, organizing library, tweaking settings |
| **Theater** | Controller primary, keyboard fallback | Borderless fullscreen, one window | Couch/TV user with a wireless gamepad | Picking a game to play tonight |
| **Cabinet** | Controller / arcade panel only | Exclusive fullscreen, optional marquee output | Cabinet builders, kiosk operators | Attract loop, no-mouse navigation, optional locked settings |

These are not separate apps. They share the same library data, the same Solid components, and the same Tauri shell. The mode dictates which regions render, which interactions are bound, font sizes, default view, and visibility of administrative actions (settings, quit, import).

**Switching:** Settings → Presentation → Mode. Persists in `appData/presentation.json` (Rust-side, like shell-mode), reads at startup. Hotkey to cycle (F10) in Desktop and Theater. Cabinet mode requires a deliberate exit gesture (long-press Start+Select, or a numeric code) — see §9.

**Decision A (load-bearing):** Theater is its own mode, not "Desktop with a bigger zoom slider." Different layouts, different focus management, different default view. Cabinet builds on Theater by removing administrative chrome and adding attract mode. **My call: yes, three distinct modes.** Two modes (Desktop + Cabinet) would force Cabinet to also be the couch UX, which is the wrong target — couch users still want settings discoverability that cabinet users don't.

---

## 2. Layout architecture

The window divides into seven regions. Each can be visible, collapsed, or hidden depending on mode + user preference.

```
┌──────────────────────────────────────────────────────────────────────┐
│  TOP TOOLBAR (T)                                                     │
├────────────┬────────────────────────────────────────┬────────────────┤
│            │                                        │                │
│            │                                        │                │
│  LEFT      │        MAIN CONTENT                    │   RIGHT        │
│  SIDEBAR   │        (grid / detail / system /       │   SIDEBAR      │
│  (L)       │         game-running-overlay)          │   (R)          │
│            │                                        │                │
│            │        (M)                             │                │
│            │                                        │                │
│            │                                        │                │
├────────────┴────────────────────────────────────────┴────────────────┤
│  STATUS BAR (S) — optional, off by default                           │
└──────────────────────────────────────────────────────────────────────┘
   (Modals open as drawer-from-right or center-overlay depending on type)
```

**Region visibility rules by mode:**

| Region | Desktop | Theater | Cabinet | Game running |
|---|---|---|---|---|
| Top toolbar (T) | always | collapsible / on-focus | hidden by default | idle-hidden after 3s |
| Left sidebar (L) | resizable, default 240px | collapsible to icons, default expanded | hidden by default | hidden |
| Main content (M) | always | always | always | game surface |
| Right sidebar (R) | toggleable, default expanded 320px | toggleable, default hidden | hidden | hidden |
| Status bar (S) | off by default | off | off | off |
| Modals | drawer or center | drawer or center | center only, larger | center only |

**Geometry tokens** (defined as CSS variables, overridable by theme):
- `--layout-top-toolbar-height`: 56px (Desktop) / 72px (Theater) / 88px (Cabinet)
- `--layout-left-sidebar-width`: 240px (Desktop default) / 280px (Theater) / hidden (Cabinet)
- `--layout-left-sidebar-collapsed-width`: 64px (icon rail)
- `--layout-right-sidebar-width`: 320px (Desktop) / 360px (Theater) / hidden (Cabinet)
- `--layout-content-padding-x`: 24px (Desktop) / 40px (Theater) / 64px (Cabinet)
- `--layout-grid-gap`: 12px (Desktop) / 20px (Theater) / 32px (Cabinet)

**Persistence:** sidebar widths, collapse state, right-sidebar widget order, and last-active view are stored per-mode in `appData/layout.json`. Cabinet mode layout is read-only at runtime — admin changes happen in Desktop mode.

---

## 3. Top toolbar

The current header is overloaded. The new toolbar splits responsibility across three zones.

```
┌──────────────────────────────────────────────────────────────────────┐
│  ◐ OA   [Library]  ›  [TG-16]  ›  [Bonk]      🔍 ⎵search⎵   ◄ ➤ ⟲   │
│         ── Breadcrumbs ──                     ── Center ──   ── ── ─ │
│                                                                       │
│                                          ⏵ Play  ⋯  ⚙ Settings  ✕   │
│                                          ── Right actions ──         │
└──────────────────────────────────────────────────────────────────────┘
```

### Left zone — identity + breadcrumb
- OA logomark (clickable → Home).
- Breadcrumb path showing the current view (e.g. `Library › TG-16 › Bonk's Adventure`). Each crumb clickable, with a `‹‹` quick-back to root.
- During gameplay this zone shows the running game's logo + system instead.

### Center zone — search + navigation
- Global search (`/` focuses). Searches game titles + developer + publisher + genre + tags + custom-field values. MiniSearch under the hood (per research doc §5).
- Forward/back/refresh affordances for views with history (Game Detail, System pages).

### Right zone — actions
- Primary action: `▶ Play` if a game is focused with a pinned selection; **Resume** if a game is paused; hidden when nothing's focused. Big colored button — visually the most important thing in the toolbar.
- Overflow menu (`⋯`): View mode picker, Sort, Import folder, Refresh library, Toggle right sidebar, Toggle presentation mode.
- Settings (`⚙`) — opens the Settings page (not a modal; §8).
- Window controls (`✕` only — minimize/maximize live in the Tauri title bar in Desktop mode; both hidden in Theater/Cabinet).

### Visibility rules
- **Desktop:** always visible.
- **Theater:** auto-hide after 3s idle (mouse OR controller). Reveals on activity. Same `oa-header-fade` pattern we already have.
- **Cabinet:** hidden by default. A specific controller gesture (e.g. long-press Select+Start) reveals a minimal toolbar that contains only `Settings (PIN-locked)`, `Quit (PIN-locked)`. No mouse hover reveal in cabinet.
- **Game running (any mode):** idle-hidden after 3s; reveal on activity.

### What moves OUT of the toolbar (from current state)
- System buttons (TG-16 / Lynx / …) → into the **left sidebar** under "Systems."
- "Pick folder" → into the **Library** settings page; the toolbar gets a generic "Import" affordance via the overflow menu that opens the Import wizard.
- "Unload" → into the Quick Actions overlay (Escape during gameplay) or right-sidebar Now-Playing widget.
- "Hide library" → into a keyboard/controller shortcut + a visible toggle on the right side of the toolbar during gameplay.

**Decision B:** The toolbar is action-thin, search-and-breadcrumb-heavy. We resist the LaunchBox pattern of N system buttons in the top bar. **Why:** LaunchBox doesn't scale past ~6 systems in the bar; OA targets 10+ at launch and 30+ long-term.

---

## 4. Left sidebar

The single most important navigational surface. Replaces the current header system buttons + adds playlists, custom views, recent.

```
┌──────────────────────────┐
│  🏠 Home                  │
│  📚 All Games   (4,213)   │
│  ⭐ Favorites    (88)     │
│  ⏱ Recent       (12)     │
│  ▶ Continue     (3)       │
│                           │
│  ─ Systems ─        ⊕  ⇅  │
│  ► TurboGrafx-16  (842)   │
│    PC Engine CD   (203)   │
│    SuperGrafx     (5)     │
│  ▼ Atari Lynx     (76)    │
│     [Lynx logo bg]        │
│  ► Atari 7800     (54)    │
│  ► SMS / Game Gear (412)  │
│  ► …                      │
│                           │
│  ─ Playlists ─      ⊕     │
│  🎮 Pickup-and-play       │
│  🏆 Beaten         (40)   │
│  📖 To Play        (210)  │
│  + New playlist           │
│                           │
│  ─ Smart Views ─    ⊕     │
│  🔥 Top 100 (rated)       │
│  🆕 Added this week       │
│  + New smart view         │
└──────────────────────────┘
```

### Sections
1. **Quick destinations** (Home, All Games, Favorites, Recent, Continue Playing). Pinned to the top, not reorderable; users can hide individual entries via right-click → Hide.
2. **Systems** group. Reorderable via drag-and-drop. Each entry shows: system icon (small marquee logo) + display name + count. Hover/focus shows a tiny system-themed background pulse (a CSS gradient using the system accent). Expansion arrow shows sub-systems (PCE → CD, SGX).
3. **Playlists** group. User-created, manually-curated. Drag games onto a playlist to add. Reorderable.
4. **Smart Views** group. Rule-based saved filter sets (the M9 feature in research doc §3). Each is a stored query that re-evaluates on the live library.

### Behaviors
- **Drag-to-reorder** within a group (uses HTML5 drag-and-drop with Solid's `createSortable` — cheap to write).
- **Drag a system onto another system** = nested sub-group (e.g. user can group `PCE` + `PCE CD` + `SGX` under "PC Engine family"). Persisted as a tree, not flat.
- **Right-click on a system:**
  - Hide from sidebar
  - Pin to top of group
  - Edit system display (override name, marquee, accent color override)
  - Open system page
  - Open system settings (Phase 3+, alongside per-system settings work)
- **Width-resize** via drag on the right edge. Snap points at 200px / 240px / 280px / 320px.
- **Collapse to icon-only rail** (64px wide) via toolbar overflow toggle or `Ctrl+B`. Hover an icon → flyout shows the full group.
- **Empty state:** if no folders imported yet, the sidebar shows a single big "Import games" CTA in place of the Systems section.

### Customization storage
```jsonc
// appData/layout.json (per-mode)
{
  "desktop": {
    "leftSidebar": {
      "width": 280,
      "collapsed": false,
      "quickDestinations": ["home", "all", "favorites", "recent"],   // ordered, hidden ones omitted
      "systemTree": [
        { "id": "tg16", "children": ["pce-cd", "sgx"] },
        { "id": "lynx" },
        { "id": "atari7800" }
      ],
      "hiddenSystems": ["wonderswan-color"],
      "playlistOrder": ["pickup", "beaten", "to-play"],
      "smartViewOrder": ["top100", "added-this-week"]
    },
    "rightSidebar": { ... },
    "lastView": { "kind": "system", "id": "tg16" }
  },
  "theater": { ... },
  "cabinet": { ... }
}
```

**Decision C:** Systems live in the **left sidebar**, not the top toolbar. Direct conflict with the current header. **Why:** scale (10+ systems unmanageable in a horizontal bar), per-system theming demands a dedicated surface, and the BigBox-style "wheel" navigation users expect maps naturally to a vertical list with system art.

---

## 5. Right sidebar — customizable game detail pane

The right sidebar shows information about the **focused game** (hover/click on a tile, or last-launched). Three states:

| State | Width | Trigger |
|---|---|---|
| Hidden | 0px | User toggle, or Cabinet mode |
| Collapsed | 64px (just a "show details" affordance) | User toggle |
| Expanded | 320px (Desktop) / 360px (Theater) | Default in Desktop |

### Widget system

The right sidebar is **dashboard-style**: an ordered list of widgets the user toggles on/off. Each widget is a self-contained Solid component bound to the focused game.

Available widgets (Phase 3 ship-list):

```
┌──────────────────────────┐
│ ┌──────────────────────┐ │
│ │ HERO ARTWORK         │ │  <-- Cover/Hero widget (default on)
│ │   [box art]          │ │
│ │                      │ │
│ └──────────────────────┘ │
│                          │
│ Bonk's Adventure         │  <-- Title + system + year
│ TurboGrafx-16 · 1990     │
│ ──────────────────────── │
│                          │
│ ⏱ 3h 42m played          │  <-- Play stats
│ ★ ★ ★ ★ ☆  4.2          │
│ 🏆 12 / 24 achievements  │  <-- RetroAchievements
│                          │
│ ▶ Play     ⏱ 12 saves    │  <-- Action row
│ ⋯ More                   │
│                          │
│ ── Recent activity ──    │  <-- Activity log widget (optional)
│ • Save state #4 yesterday│
│ • Achievement: Cherry…   │
│ • Played 28 min ago      │
│                          │
│ ── Metadata ──           │  <-- Metadata widget (default on)
│ Developer  Red Company   │
│ Publisher  Hudson Soft   │
│ Genre      Platformer    │
│ Players    1             │
│                          │
│ ── Description ──        │  <-- Description widget (default on)
│ A prehistoric platformer │
│ where you headbutt …     │
│                          │
│ ── Custom fields ──      │  <-- Custom fields widget (default off)
│ Tags        retro, easy  │
│ My note     beat lvl 4   │
└──────────────────────────┘
```

Widgets shipping Phase 3:
1. **Hero / Cover** — primary cover art, fallback to placeholder + system tint.
2. **Title block** — title, system, year (and region if non-default).
3. **Play stats** — play time, rating, last-played, RA progress (when available).
4. **Action row** — Play / Save states / Edit / Open file location / More menu.
5. **Recent activity** — last 5 events for this game (Phase 4 once activity log exists).
6. **Metadata** — dev/publisher/genre/players from libretro-database.
7. **Description** — text block, expandable. From libretro-database RDB once shipped; placeholder copy if absent.
8. **Custom fields** — inline-editable user-defined fields (S17 in research doc).
9. **Snapshots / screenshots strip** — 3-5 thumbnails from libretro-thumbnails snap/title sync.
10. **Save states preview** — 4-cell grid of recent save thumbnails with timestamps.
11. **Achievements panel** — RA badges, locked/unlocked grid (Phase 4).
12. **Compatibility / notes** — community per-game notes (much later, Phase 5+).

### Widget management
Right-click on a widget header → Move up / Move down / Hide widget / Settings. A "+ Add widget" affordance at the bottom opens a picker. Widget order + visibility persisted per-mode in `layout.json`.

### "Pin focused game" vs "follow focus"
A small pin icon at the top of the sidebar. When unpinned (default), the sidebar tracks the focused tile in the grid — hover or arrow-key onto a tile, the right sidebar updates. When pinned, the sidebar stays on whatever game the user pinned regardless of grid focus. Useful for: looking at one game while scrolling for similar ones.

**Decision D:** Right sidebar follows hover/focus by default; pin is opt-in. **Why:** Steam/Heroic both auto-update on grid focus; users grow to expect it. Pinning is a power feature.

### Where Game Detail Page fits

Click a tile → Game Detail full-screen page (existing `SystemPage` pattern extended for games). The right sidebar in the grid view is a **compact, always-visible** version of what the Game Detail page shows in larger form. Users who configure the right sidebar to show everything can avoid the Game Detail page entirely; users who hide the sidebar still get full detail via the page.

---

## 6. Library grid changes

The current grid (`grid grid-cols-2 sm:grid-cols-3 …`) needs three changes:

### 6.1 View modes

Add a view-mode picker (toolbar overflow → View, or `V` keyboard shortcut). Five views:

| View | Use case | Tile layout |
|---|---|---|
| **Capsule grid** (default) | Most users | 2:3 vertical tiles (the Steam Library Capsule shape) |
| **Horizontal hero** | Movie-poster aesthetic, fewer titles visible | Wide cards using the 920×430 header asset |
| **Detail list** | Long lists, metadata-heavy | One row per game, large boxart + columns for year/genre/play time |
| **Wall** | Maximum density | Smaller capsules, more per row |
| **Coverflow** | Cabinet/Theater pizzazz | Horizontal scroll with depth, focused tile larger |

Coverflow is the BigBox-style "wheel." It's Theater/Cabinet default. Desktop's default is Capsule grid.

### 6.2 Virtualization

Replace the current `<ul class="grid">` with TanStack Virtual + 2D-row-grouping per research doc §5. Critical for the 5K+ library promise. Phase 3 ship-blocker for the main grid; views like Detail-list are simpler vertical virtualizers.

### 6.3 Sort / filter / group bar

Above the grid (sticky, below toolbar):

```
┌──────────────────────────────────────────────────────────────────────┐
│  TurboGrafx-16  ·  842 games        Sort: A→Z ▾   Filter ▾   Group ▾ │
└──────────────────────────────────────────────────────────────────────┘
```

- **Sort:** Name, Date Added, Year, Last Played, Play Time, Rating, Random.
- **Filter:** Tag chips (Favorites, Beaten, In Progress) + dropdowns (Genre, Year range, Developer, etc.). Each filter is a removable chip below the bar.
- **Group:** None / Letter / Year / Genre / Developer / Rating. Group headings are sticky during scroll, like iOS Photos sections.

### 6.4 Empty state

The current "Library is empty" prompt is fine. Extend to: large CTA card with three actions — "Import a folder," "Drop a folder here" (whole-window drop target), "Try with sample ROM" (loads a homebrew ROM bundled in the installer for demo).

---

## 7. Import flow

The current "Pick folder" → instant ingest is too thin. Replace with a wizard + per-folder rules system + background scanner.

### 7.1 Import wizard

`Settings → Library → Import` or `Toolbar → ⋯ → Import folder`. Multi-step:

```
┌──────────────────────────────────────────────────────────────────────┐
│  Import games                                                Step 1/4 │
├──────────────────────────────────────────────────────────────────────┤
│                                                                       │
│  Pick a folder to scan.                                               │
│                                                                       │
│  [ G:\ROMs\TurboGrafx-16                            ] [ Browse… ]    │
│                                                                       │
│  ☑ Scan subfolders                                                    │
│  ☐ Treat each subfolder as a system (folder name → system)            │
│                                                                       │
│                                            [ Cancel ]  [ Next › ]     │
└──────────────────────────────────────────────────────────────────────┘
```

**Step 2 — System mapping:**
```
For this folder, ROMs of type:
  *.pce → TurboGrafx-16             ▾
  *.sgx → SuperGrafx                ▾
  *.cue/.chd → PC Engine CD-ROM²    ▾
  *.zip → Auto-detect from contents ▾   ← unzips in memory, matches inside

[ + Add rule ]
```

**Step 3 — Scan preview:**
```
Scanning… 1,234 of 4,500 files
[████████████░░░░░░░] 27%

Currently:  Castlevania - Rondo of Blood (Japan).chd → PC Engine CD-ROM²

Found so far:
  ✓ TurboGrafx-16        642 games        12 duplicates skipped
  ✓ PC Engine CD-ROM²    87 games         3 unmatched
  ✓ SuperGrafx           4 games          0 unmatched
  ⚠ Atari Lynx           12 games (folder structure suggests wrong system?)

[ Pause ]  [ Cancel ]
```

**Step 4 — Confirm:**
```
Add 745 games to your library?

Preview the unmatched 18 → expandable list, user can manually assign system
or skip.

Run media + metadata sync after import?
  ☑ Cover art   ☑ Snapshots   ☑ Title screens
  ☑ Year/genre/developer/publisher/players metadata

[ Back ]  [ Skip and add ]  [ Import + sync ]
```

After import: background sync runs with progress in the status bar (or a small toast that pins until done).

### 7.2 Per-folder rules persisted

Each tracked folder gets a record:

```jsonc
// appData/library/folders.json
{
  "folders": [
    {
      "id": "folder-abc123",
      "path": "G:\\ROMs\\TurboGrafx-16",
      "scanSubfolders": true,
      "subfoldersAreSystems": false,
      "rules": [
        { "match": "*.pce", "systemId": "tg16" },
        { "match": "*.sgx", "systemId": "sgx" },
        { "match": "*.cue", "systemId": "pce-cd" },
        { "match": "*.chd", "systemId": "pce-cd" }
      ],
      "lastScannedAt": "2026-05-17T10:30:00Z",
      "watchEnabled": true,
      "schedule": "on-startup"
    }
  ]
}
```

Rules let the user fix edge cases (mixed-system folders, weird extensions, archives). `subfoldersAreSystems` covers the common BigBox-style layout `ROMs/<System>/<files>`.

### 7.3 Background scanner

Move the ingestion logic from the WebView (`pickFolderAndIngest`) into a Rust-side **background scan service**:

- A Tauri-managed `Mutex<ScanQueue>` with pending jobs.
- Scanner runs on `tokio::task::spawn_blocking` (filesystem) → emits progress events at `oa://library-scan-progress`.
- Frontend subscribes and renders progress in the toolbar status indicator + a Toasts.
- Scans can run in parallel across folders (one worker per CPU core, max 4).
- Cancellable via `cancel_scan` command.

This also unblocks **watch mode**: a folder marked `watchEnabled: true` is registered with a filesystem watcher (`notify` crate) so newly-dropped ROMs auto-appear without an explicit rescan.

### 7.4 Media import vs ROM import

Today these are coupled in the user's mental model ("import" = both at once). Make them explicitly separate workflows:
1. **ROM import** (this wizard) — adds games to the library.
2. **Media sync** — already exists in Settings → Game media; can run unprompted after import.
3. **Manual media override** — Game Detail page → Replace cover (file picker). User drops their own art for one game.

**Decision E:** Library is the source-of-truth; media is derived. Importing a folder never mutates media without user opt-in. **Why:** users may have curated their own art and don't want auto-fetch to clobber it.

---

## 8. Settings architecture

Today: one big modal with a left rail and 5 tabs. The three-tier split memory says don't merge OA-wide / per-system / per-game. The current modal is OA-wide-only, which is correct, but the modal-as-container limits us as we grow.

### 8.1 Convert Settings from modal → page

Settings becomes a route (`/settings`) like the system pages. Keeps the same left-rail tab layout we already built, but gets:
- More vertical space (no fixed-height modal).
- URL deep-linking (Settings → Cores can be opened directly).
- No focus-trap conflicts with other modals.
- The Quick Settings overlay can still pop a subset on top of game running (see below).

**Backward compat:** the Settings *modal* can stay as a "Quick Settings" inline picker for the most-used OA-wide adjustments (audio device, scaling mode, shell mode) reachable from the toolbar `⚙` overflow. Full settings = page.

### 8.2 Three settings surfaces

| Surface | Reach | Persistence | Contents |
|---|---|---|---|
| **OA-wide settings page** | Toolbar `⚙` | `appData/*.json` + `localStorage` | Presentation mode, display, audio, input, cores, library folders, media providers, theme, hotkeys |
| **Per-system settings page** | System page header → `⚙` | `appData/systems/<id>.json` | Default core, default scaling, default shaders, default region priority, button mapping, system theme override |
| **Per-game settings drawer** | Game Detail or right-sidebar `⋯ → Properties` | SQLite row | Core override, scaling override, shader override, region override, custom button map, custom fields, notes |

Each per-game override falls back to per-system, which falls back to OA-wide. The Settings UI on each surface displays the inherited value greyed out alongside the override input — same pattern Visual Studio Code uses for workspace-vs-user settings.

### 8.3 OA-wide settings page structure

```
Settings
├── Presentation
│   ├── Mode (Desktop / Theater / Cabinet)
│   ├── Theme (active theme, browse community themes, install from file)
│   ├── Per-system themes (override accent / marquee / font per system)
│   └── Animations (motion-reduce option, scroll smoothness)
├── Display (existing)
│   ├── Scaling mode (default for all systems)
│   ├── Window mode + monitor
│   └── Shell mode (single/two-window)
├── Audio (existing)
│   ├── Output device
│   └── Latency profile
├── Input
│   ├── Keyboard mapping
│   ├── Controller mappings (one per detected controller)
│   ├── Hotkeys (F1-F12 + Ctrl combos for shell actions)
│   └── Light gun (when supported)
├── Library
│   ├── Folders (existing) + per-folder rules (new)
│   ├── Auto-scan schedule
│   ├── Background watcher
│   └── Database tools (rebuild index, vacuum, export, import)
├── Cores (existing)
│   ├── Detected cores
│   ├── Per-system default (in OA Cores tab today)
│   └── Auto-download (libretro nightly opt-in)
├── Game media (existing)
│   ├── Kinds to fetch
│   ├── Per-system sync
│   ├── Region priority
│   └── Storage
├── Metadata providers
│   ├── libretro-database (always on)
│   ├── ScreenScraper (optional, credentials)
│   ├── IGDB (optional, API key)
│   └── SteamGridDB (optional, API key) — for Library Hero assets
├── Cabinet
│   ├── Enable cabinet mode (PIN)
│   ├── Marquee output (which monitor)
│   ├── Attract mode (idle timer, transition style)
│   ├── Boot directly into Cabinet
│   └── Lock administrative actions
├── Updates
│   ├── Check for OA updates
│   └── Check for core updates (libretro nightlies)
├── Advanced
│   ├── Hardware (GPU info, GPU selector)
│   ├── Logging (level, open log folder)
│   └── Reset (per-category reset to defaults)
└── About
```

### 8.4 Per-system settings page

Reached from system page header → `⚙`. Same left-rail layout, sections:
- Display (scaling override)
- Audio (system-specific output if needed, e.g. mono for handhelds)
- Input (system button labels, default button map for new controllers)
- Cores (override default for this system)
- Shaders (Phase 3+)
- Theme (override OA-wide system theme: custom accent, custom marquee, custom font)

### 8.5 Per-game settings drawer

Slides in from right when triggered. Tabs along top:
- Overview (metadata, custom fields)
- Core (per-game core override + per-game core options)
- Display (per-game scaling, per-game window mode override)
- Audio (per-game audio adjustments)
- Input (per-game button remap)
- Shaders (per-game preset)
- Region (per-game region priority override)

### 8.6 Quick Settings overlay (during gameplay)

Triggered by `Escape` or controller Start during gameplay. A center-aligned card with the most useful in-game toggles:

```
┌────────────────────────────────────┐
│   Bonk's Adventure                 │
│                                    │
│   ▶ Resume                         │
│   ⟲ Restart                        │
│   ⏱ Save / Load states             │
│   📊 Scaling (Pixel Perfect)        │
│   🎨 Shader (CRT)                  │
│   ⚙ Game settings                  │
│   ─────────────────                │
│   🚪 Exit to library                │
└────────────────────────────────────┘
```

This is small and fast. Full settings reachable via the "Game settings" entry.

**Decision F:** Settings is a page, not a modal. Quick Settings overlay during gameplay is the only modal form. **Why:** content keeps growing; modal pixel ceiling is restrictive; URL deep-linking is useful for diagnostics and "open Settings to this section" links from elsewhere.

---

## 9. Cabinet (arcade) mode

What "BigBox-equivalent" means for OA, concretely:

### 9.1 The shape of Cabinet mode

- **Boot directly** into Cabinet (optional — Settings → Cabinet → Boot into Cabinet on launch).
- **Exclusive fullscreen, primary monitor.** No window chrome.
- **Default view: Coverflow (system wheel + game wheel).** Vertical scroll = systems, horizontal scroll inside = games. Or grid-wheel hybrid configurable.
- **Controller-first.** Mouse hidden by default; touch supported for touchscreens. All shell-level actions reachable from controller.
- **Marquee output.** If a second monitor is detected, optionally use it for the focused game's marquee art (large, animated, system-themed). Phase 4 ship.
- **Attract mode.** After N seconds of idle (default 90), fade into an attract loop: random game previews with snap videos, gameplay snippets, system fades. Tap any control to exit.
- **Administrative lockdown.** Settings access requires PIN (entered with controller D-pad as a 4-digit code). Quit requires PIN. The exit-to-OS gesture is intentionally not discoverable.
- **No "delete" / "remove" actions** surfaced. Cabinet is read-only of the library state.

### 9.2 Cabinet layout

```
┌──────────────────────────────────────────────────────────────────────┐
│  (Optional marquee on second monitor — system marquee + game name)   │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│                                                                      │
│            ◀  ◀     [BONK'S ADVENTURE]      ▶  ▶                   │
│         (game cover) (3D box, highlighted)  (next cover)             │
│                                                                      │
│            ★★★★☆       1990 · Hudson Soft · Platformer              │
│                                                                      │
│                  [ TG-16 SYSTEM LOGO + AMBIENT BG ]                 │
│                                                                      │
│                                                                      │
│                  ⓐ Play   ⓧ Save states   ⓨ Info                   │
│                                                                      │
├──────────────────────────────────────────────────────────────────────┤
│  TurboGrafx-16     Atari Lynx    Atari 7800    SMS/Game Gear   …    │
└──────────────────────────────────────────────────────────────────────┘
```

- Top: optional marquee strip.
- Center: focused game art, 3D box rotation, system-themed ambient background (per the WGSL hook in research doc S2).
- Bottom: horizontal system selector (the "wheel").
- D-pad up/down: navigate within a system. Left/right: switch system. A/Start: launch. B: back/cancel.

### 9.3 Attract mode

After idle:
1. Fade out chrome.
2. Cross-fade between random game snaps (5s per game), each with system marquee bar at top.
3. Audio: low-volume system theme or game audio sample.
4. Any input cancels and returns to the previous focused game.

Implementation: a Solid component that takes a random `RomEntry`, renders the snap video as `<video>`, advances on a timer. No emulator runs during attract.

### 9.4 Exit / unlock gesture

- Hold Select + Start for 3 seconds → reveals admin overlay → asks for PIN.
- Or: Esc-Esc-Esc on connected keyboard (cabinet operators sometimes hide a keyboard inside the cab for service).

**Decision G:** Cabinet is opt-in, not a default. Most OA users will run Desktop or Theater. **Why:** the cabinet feature set (PIN locking, attract mode, marquee output) is significant engineering, but the cabinet market is small and willing to wait — Phase 4-5 territory.

---

## 10. Storage architecture for customization

All persisted in `appDataDir/`:

```
appData/
├── presentation.json          # Active presentation mode + per-mode toggle history
├── layout.json                # Sidebar widths, widget order/visibility per mode
├── shell.json                 # Existing — single/two-window pref
├── audio.json                 # Existing — audio device pref
├── cores.json                 # Existing — per-system default core
├── input/
│   ├── keyboard.json          # Keyboard map
│   └── controllers/<guid>.json # Per-controller button maps
├── library/
│   ├── games.sqlite           # Master library DB (replaces localStorage v1)
│   ├── folders.json           # Tracked folders + per-folder rules
│   └── playlists.json         # User playlists + smart views
├── systems/
│   ├── tg16.json              # Per-system overrides
│   ├── lynx.json
│   └── …
├── themes/
│   ├── active.json            # Which theme is currently active
│   └── installed/<id>/        # Theme bundles
├── cabinet/
│   ├── config.json            # Cabinet mode prefs (PIN hash, attract timer, marquee monitor)
│   └── boot.flag              # If present, boot into cabinet
└── media/                     # Existing — boxart, snaps, titles, thumbhashes
```

**Library migration:** the current `oa.library.v1` localStorage entry migrates to `library/games.sqlite` at first launch after the upgrade. SQLite gets FTS5 for search (research doc §5) + indexes on `(system_id, title)`, `(last_played_at)`, `(custom_tags)`.

**localStorage retained for:** WebView-only ephemeral state (last active settings tab, last view-mode picker selection, last scroll position per route). Anything that needs to survive WebView-clear-cache or be readable from Rust before WebView boots goes to `appData/*.json` / SQLite.

---

## 11. Phased delivery

Translates this plan into deliverable chunks. Each chunk should be shippable on its own.

### Phase 2.5 — Layout shell + SQLite migration (now, before Phase 3 shaders)

The "main window plan" foundation. Unblocks every visible thing afterward.

1. **Layout primitives.** Solid components for `<Shell>`, `<TopToolbar>`, `<LeftSidebar>`, `<RightSidebar>`, `<MainContent>`. CSS-variable-driven geometry. ResizeObserver + drag handles for resizable sidebars.
2. **Move existing chrome.** System buttons → left sidebar (with nested-tree placeholder); "Pick folder" → toolbar overflow; status text → toolbar; "Hide library / Unload" relocated per §3.
3. **Right sidebar v1.** Three default widgets: Hero/Cover, Title block, Metadata. **Expanded by default in Desktop (320px)**, hidden in Theater/Cabinet.
4. **Layout persistence.** `appData/layout.json` (sidebar widths, collapse state, widget order, last view) per-mode; `appData/presentation.json` (active mode). Rust IO on the same pattern as existing `shell.json` / `audio.json` / `cores.json`.
5. **Presentation mode toggle** in Settings — Desktop / Theater / Cabinet. Cabinet ships as **layout stub only** (no attract, no marquee, no PIN — those land in Phase 4).
6. **SQLite library migration.** Add `oa-library` crate (or extend existing Rust shell with a new module) wrapping `rusqlite` + `r2d2`. Schema: `games` (id, system_id, file_path, title, normalized_title, year, genre, developer, publisher, players, rating, play_time_secs, last_played_at, added_at, region, custom_fields_json) + `folders` + `playlists` + `play_history` + FTS5 virtual table on `(title, normalized_title, developer, publisher)`. One-shot migration on first launch reads `localStorage[oa.library.v1]` via the WebView, sends through a `migrate_library_from_local_storage` Tauri command, populates SQLite, clears the localStorage key. Existing media DB stays separate (covers, snapshots, thumbhashes already on disk).
7. **Library store rewrite.** `createLibraryStore` queries SQLite via Tauri commands (`list_games`, `add_games`, `update_game`, `delete_game`, `search_games`). Solid store stays in-memory but is now the read-through cache, not the source of truth.

**Acceptance:**
- App opens with new layout: toolbar + left sidebar + grid + right sidebar all visible.
- Resizing sidebars persists across restarts; presentation mode toggle works.
- Switching to Theater collapses sidebars and bumps font sizes; Cabinet hides chrome entirely (stub only — no attract mode yet).
- First launch after upgrade migrates the existing localStorage library to SQLite with zero data loss; user sees their existing games unchanged.
- Right sidebar tracks focused tile by default; pin toggle keeps it locked to one game.

**Estimated complexity:** XL (~2500 LOC: ~1500 frontend layout + ~700 Rust SQLite + ~300 migration).

### Phase 2.6 — Library polish

1. **TanStack Virtual** for the main grid (per research doc §5).
2. **View mode picker** (Capsule grid + Detail list initially; Wall + Hero later).
3. **Sort / filter / group bar** above the grid.
4. **Drag-to-reorder** systems in left sidebar.
5. **Empty state CTA** with whole-window drag-drop import target.

**Acceptance:** Library scrolls 5K games smoothly. Sidebar reorder persists. Drag a folder onto the empty state → import wizard opens.

**Estimated complexity:** L (~1200 LOC).

### Phase 2.7 — Import flow

1. **Import wizard** (4 steps: folder → mapping → preview → confirm).
2. **Background scan service** in Rust with progress events.
3. **Per-folder rules** in `library/folders.json`.
4. **Filesystem watcher** for `watchEnabled` folders.
5. **Migrate library state from localStorage → SQLite.**

**Acceptance:** User imports a 5K-ROM folder with mixed systems. Wizard correctly maps extensions. Background scan completes with progress visible. New ROMs dropped into a watched folder auto-appear within 5s.

**Estimated complexity:** L (~1500 LOC across Rust + Solid + SQLite migrations).

### Phase 2.8 — Settings as a page

1. **Convert SettingsModal → Settings route.** Keep the existing 5 tabs. Add: Presentation, Input, Cabinet stubs (visible but mostly empty).
2. **Quick Settings overlay** for gameplay (Escape → small card).
3. **Per-system settings page** scaffolding (linked from system page header).
4. **Per-game settings drawer** scaffolding (linked from Game Detail / right-sidebar `⋯`).

**Acceptance:** Settings reachable as a page with URL deep-link. Per-system page exists for at least TG-16. Per-game drawer exists for at least one game. Three-tier inheritance UI (greyed-out inherited values) works.

**Estimated complexity:** M (~1000 LOC, mostly reorganization).

### Phase 3 — Shaders + per-game overrides (existing roadmap)

Folds in: per-game shader settings live in the per-game drawer built in 2.8.

### Phase 4 — Cabinet mode + RetroAchievements

1. **Attract mode** (idle timer, snap-video loop).
2. **Marquee output** on second monitor.
3. **PIN-locked admin gestures.**
4. **Boot-directly-into-cabinet** flag.
5. **RetroAchievements** auto-login passthrough + badge widget in right sidebar.

**Acceptance:** Cabinet builders can install OA on a kiosk and never see settings unless they enter the PIN. Attract mode runs through the library after 90s idle.

**Estimated complexity:** L.

### Phase 5+ — Polish, plugin tiers, theme creator, federated metadata

Per research doc §3 SHOULD-tier features, ordered by user value.

---

## 12. Locked decisions

Resolved 2026-05-17 with the human:

- **L-1.** Three presentation modes: **Desktop / Theater / Cabinet**. All three first-class. Theater is its own mode, not a Desktop zoom or a Cabinet-without-PIN.
- **L-2.** **Settings is a route**, not a modal. Quick Settings overlay (small center card on Escape) covers in-game adjustments. Existing SettingsModal gets retired during Phase 2.8.
- **L-3.** Left sidebar systems are **nested via drag-and-drop**. User can drag a system onto another to create a named sub-group ("PC Engine family"). Tree persisted in `layout.json`.
- **L-4.** **Phase 2.5 (Layout shell) is the next session**, including the SQLite library migration (see L-7).
- **L-5.** Right sidebar **expanded by default (320px)** in Desktop mode with prominent toggle. Hero / Title / Metadata widgets visible on first launch. Theater hides it by default; Cabinet always hides.
- **L-6.** **Cabinet mode at Phase 4**, alongside RetroAchievements. Phase 2.5 ships only the layout stub for the Cabinet mode toggle — no attract loop, marquee output, or PIN until Phase 4.
- **L-7.** **SQLite library migration folds into Phase 2.5.** Builds `games.sqlite` from day one. One-shot `localStorage` → SQLite migration runs at first launch after the upgrade. Sidebar tree, layout state, presentation mode go to `appData/*.json`.
- **L-8.** **Top toolbar is sidebar-only for system nav** — three zones (breadcrumb / search / actions). No "favorite systems" pin row. Quick destinations (Home / All / Favorites / Recent / Continue) live pinned at the top of the left sidebar as the closest analog to "tabs in the header."

No remaining open decisions for the next session.

---

## 13. What this plan deliberately does NOT include

To keep scope honest:

- **WGSL ambient backgrounds per system** — research doc S2; defer to Phase 3+ alongside shader work.
- **3D box rendering** — research doc S3; defer to Phase 4-5.
- **In-app theme creator** — research doc S4; defer to Phase 5+. Phase 2.5 lets users edit theme TOML/CSS by hand, which is enough until a creator is justified.
- **Plugin system (any tier)** — research doc S18-S19; defer to Phase 5+.
- **Steam-style hero artwork download** — research doc M5; depends on SteamGridDB integration, defer to Phase 3.
- **Smart-playlist expression engine** — research doc M8; Phase 3. The sidebar exposes the *concept* of Smart Views in 2.5, but creation UI is later.
- **Light gun support** — research doc C2; Phase 5+.

If something on this "deferred" list becomes urgent, we move it forward explicitly via a new entry in `docs/DECISIONS.md`.

---

## 14. References

- Research doc: `docs/RESEARCH/launcher-landscape.md` (especially §3 features, §4 design system, §5 perf, §6 anti-patterns).
- Existing decisions: `docs/DECISIONS.md` 2026-05-15 stack, 2026-05-16 per-system theming cascade, 2026-05-17 asset protocol.
- Memory: `feedback_settings_three_tier_split.md` (three-tier split is settled).
- Memory: `feedback_multi_core_architecture_ready.md` (8-step recipe; this plan adds the per-system-theme step).
- Current Roadmap Phase 2 settings work in `docs/ROADMAP.md`.
