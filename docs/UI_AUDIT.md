# Overlooked Arcade — UI Surface Audit

**Status as of 2026-05-22:** Several specifics in this audit have shipped or drifted (PerSystemSettingsPage no longer exists, SettingsPage is down to 2 tabs and has been renamed LibraryManagerPage, PerGameSettingsDrawer is 10 tabs not 11, etc.). See `docs/UI_POLISH_PLAN.md` §0 for the current state. The structural-IA findings here (overlap matrix, orphaned features) remain valid as design context.

**Date:** 2026-05-18
**Purpose:** Catalog every window, tab, modal, drawer, menu, and setting in the shell so we can find gaps, redundancies, and orphaned features before redesigning IA.
**Scope:** UI surfaces only. Rust commands cross-referenced to find features without a home.

---

## 1. Shell chrome (always-on layout)

### 1.1 Top toolbar (`TopToolbar.tsx`)

Three-zone header, full width, idle-fades during single-window gameplay.

| Zone | Items |
|---|---|
| Left | `◐` home glyph button · breadcrumb (e.g. `Library › TG-16 › Settings`) |
| Center | Search input (`Search games…`, in-memory title filter, Esc to clear) · truncated status line |
| Right | Shell-mode `Show/Hide` library button · `Unload` (when game running) · `‹` right-sidebar reveal (when hidden) · `⋯` overflow menu · `⚙` settings · `✕` quit |

**Overflow menu (`⋯`) items:** Import folder… · Rescan tracked folders · Mode: {Desktop/Theater/Cabinet} · Expand/Collapse sidebar.

### 1.2 Left sidebar (`LeftSidebar.tsx`)

Width 200–360px, collapsible (Ctrl+B), hidden in Cabinet presentation mode.

- **Quick destinations** (5 entries; 4 disabled placeholders): Home · All Games · Favorites · Recent · Continue.
- **Systems**: one entry per registered system, with game count badge, drag-to-reorder, right-click → `SystemContextMenu`. Filtered by `hiddenSystems` and `autoHideEmptySystems`.
- **Playlists** (placeholder section): "No playlists yet".
- **Smart Views** (placeholder section): "No smart views yet".
- **Bottom**: `Cores` button · `Settings` button · Collapse/Expand toggle.

### 1.3 Right sidebar (`RightSidebar.tsx`)

Width 240–440px, hideable, hidden in Cabinet. Driven by **pinned** entry (sticky) or **focused** entry (hover/keyboard focus on a tile).

- Header: "Game details" label · Pin/Unpin toggle · `›` hide.
- Widgets (ordered, hideable per `layout.widgetOrder` / `widgetHidden` — but no UI to toggle order/visibility yet):
  - `hero` — full-width boxart with fallback gradient.
  - `title` — game title + system + year.
  - `metadata` — Developer · Publisher · Genre · Players.
- Action row: `▶ Play` · `Saves` · `Game info`.

### 1.4 System landing header (`SystemHeader.tsx`)

Shown above GridControls when viewing a system-filtered library. Quick-action chips: `⌨ Bindings` · `⚡ Cores` · `🎞 Shaders` · `⚙ Settings`. Each deep-links into the matching tab of `PerSystemSettingsPage`.

### 1.5 Game-mode chrome (single-window only)

When `gameMode()` (single-window + ROM running + library hidden):

- Toolbar auto-hides after 3s idle (mouse/keyboard activity reveals).
- `Esc · library` hint pill bottom-right when hidden.
- Esc → `QuickSettings` overlay.
- Ctrl+W → unload.
- Library can be re-shown with `Show` toolbar button.

---

## 2. Library views

### 2.1 GridControls (`GridControls.tsx`)

Sticky bar above the grid:

- Left: view title (e.g. "All Games", "Sega Genesis") + count badge.
- Right: per-system `⚙ Settings` (only when system-filtered) · view-mode toggle (`▦` capsule / `≡` list) · sort dropdown (Name · Date added · Year) · group dropdown (None · Letter · System).

### 2.2 Capsule grid (`VirtualLibraryGrid.tsx`, `LibraryTile.tsx`)

Virtualized 220px tiles. Each tile: cover art (system aspect ratio) + system short-name chip. Loading shimmer. System-themed accent + glow. Left-click launches; right-click → `TileContextMenu`; hover/focus → updates right sidebar.

### 2.3 Detail list (`DetailListView.tsx`)

Virtualized rows, 76px tall: thumbnail · title · system · year. Group headers between sections. Same context menu behavior.

### 2.4 Empty / null states

- **No matches** (search active): `🔍 No matches — Try a different search.`
- **Empty library**: `◐ Library is empty` + Import folder button.
- **Seed-only**: `◐ Library is full of placeholders` + Import folder button.
- **Drag-drop hover**: full-screen `⇩ Drop folder to import` overlay.

---

## 3. Modals & drawers

### 3.1 `QuickSettings` overlay (in-game)

Trigger: Esc during gameplay (or backend `oa://request-quick-settings` event in two-window mode). Modal, centered. Header shows system + game title + paused/rewinding state.

**Actions menu (default view):**
1. `▶ Resume` (Esc)
2. `⏪ Rewind…` (with `6.3s · 24 snaps` hint)
3. `⏺ TAS recording…` (idle / recording / replaying state)
4. `🎥 Video capture…` (live frame count + drops)
5. `🧠 Memory inspector…` ("dev / power user")
6. `💿 Disc control…` (multi-disc only)
7. `⏱ Save / Load states`
8. `ⓘ Game info`
9. `📐 Scaling: {mode}` (opens Settings)
10. `🎨 Shader` (**disabled, marked "Phase 3"**)
11. `⚙ All settings`
12. `🚪 Exit to library` (Ctrl+W; destructive style)

**Nested views (drill-in):**
- **Rewind** — scrub timeline · seconds offset · oldest/live labels · Cancel / Resume here.
- **TAS** — recording name input · list (name, duration, frame count, timestamp, Replay/Delete) · recording/replay progress.
- **Video** — capture name input · clip list (duration, frame count, dimensions, dropped, timestamp, Convert WebM / Open folder / Delete).
- **Memory inspector** — region selector (System RAM / Save RAM / Video RAM / RTC) · offset input · 8-col hex dump · Prev/Next page.
- **Disc control** — list of discs with current index · Insert per disc.

### 3.2 `SaveSlotsModal`

Triggered from tile context, right sidebar `Saves`, QuickSettings, or `GameInfoModal`. 2–5-column slot grid. Per slot: 4:3 thumbnail · slot number · size · modified date · Delete. Click slot to launch+restore. Footer text reminds slots 0–9, F5 save / F8 load.

### 3.3 `GameInfoModal`

Triggered from tile context or right sidebar `Game info`.
- Left ~40%: hero boxart + regional variant strip (region + manual/synced source tag).
- Right: title, close · metadata grid (Year, Genre, Developer, Publisher, Players) · description (collapsible) · tabs: **Screenshots** · **Title screens** · **Save states**.
- Footer: `Resume from slot N` (if save exists) + `Launch`.

### 3.4 `ImportWizard`

4-step modal:
1. **Folder** — picker · recent-tracked dropdown · Scan subfolders · Treat subfolders as systems · Watch for new ROMs.
2. **Mapping** — extension→system rules table; Add rule · Reset.
3. **Preview** — live scan progress + per-system tally + unmatched count.
4. **Confirm** — post-import sync toggles: Cover art · Snapshots · Title screens · Metadata.

### 3.5 `PerGameSettingsDrawer`

Right-side drawer (max 480px). Tabs across top.

| Tab | Contents |
|---|---|
| **Overview** | Read-only: Title, System, ROM path, In archive, Added date |
| **Core** | Core override select · ROM patch (IPS/UPS/BPS) picker |
| **Core options** | Embedded `CoreOptionsPanel` (per-game) |
| **Display** | Scaling · Window · Monitor (scaffold; doesn't take effect yet) |
| **Audio** | Placeholder |
| **Input** | Placeholder (bindings are system-scope only) |
| **Rewind** | Enable · Capture interval · Buffer cap |
| **Shaders** | Shader preset · Bloom amount slider with live preview |
| **Region** | Regional boxart variant picker |
| **Milestones** | List + Add/Edit/Delete; per row: name, description, region, offset, width, operator, target, edge-only, triggered-at |
| **Cheats** | Placeholder (despite cheats being fully backed; see §6) |

### 3.6 `CoreOptionsPanel`

Embedded in PerSystemSettingsPage and PerGameSettingsDrawer. Filter input + per-option row (description · dropdown · Reset · inheritance chip). Empty message when schema hasn't loaded.

### 3.7 `RegionPicker`

Modal listing all boxart variants side-by-side (~200px each). Region badge + source tag. Click to pin variant.

### 3.8 `CorePickerMenu`

Floating menu at cursor (anchored to tile context "Change core…"). Header: game title. Items: `(Default — auto-detect)` · compatible cores list (name, version, filename, active badge). Empty state for unsupported extension.

### 3.9 `TileContextMenu`

Right-click on a library tile. Sections:
- Launch
- Cover: Pick cover file… · Pick region… (if ≥2 variants) · Clear cover
- Game metadata: Save states… · Game info… · Change core… · Game properties…
- Removal: Remove from library

### 3.10 `SystemContextMenu`

Right-click on a sidebar system. Items: Show library · Edit bindings… · System settings… · Hide from sidebar.

### 3.11 `ToastStack`

Bottom-right, max 4 visible. Levels info/success/warn/error. Driven by `oa://toast` events: file ops, sync completions, launch results, errors, milestone unlocks.

---

## 4. Full-page surfaces (routed in App, not modals)

### 4.1 `SettingsPage` — OA-wide (7 tabs)

| Tab | Fields |
|---|---|
| **Presentation** | Mode (Desktop/Theater/Cabinet) · Left sidebar expanded · Right sidebar visible · Auto-hide empty systems |
| **Display** | Scaling mode · Window mode · Monitor · Shell mode · Run-ahead frames (0–5) · Phosphor bloom amount |
| **Audio** | Output device |
| **Gameplay** | Enable rewind · Capture interval (1/2/3/6/10/15/30) · Buffer cap (8–512 MB) |
| **Cores** | Detected cores list (display-only) · Per-system default core dropdowns |
| **Library** | Library folders list · System visibility checkboxes · Auto-remove on file delete · Clear games for system · Reset entire library |
| **Game media** | Only sync identified ROMs · Kinds to fetch · Per-system sync rows (Sync media / metadata / hashes / Identify ROMs) · Region priority list · Disk usage |

### 4.2 `PerSystemSettingsPage` (8 tabs)

| Tab | Fields |
|---|---|
| **Display** | Scaling · Window · Monitor (each with inherited chip) — *scaffold* |
| **Audio** | Placeholder |
| **Input** | `SystemBindingsEditor`: button×{keyboard,gamepad} table, right-click clear, capture on click, Reset to defaults |
| **Rewind** | Enable · Capture interval · Buffer cap |
| **Cores** | Default core for this system |
| **Core options** | Embedded `CoreOptionsPanel` |
| **Shaders** | Shader preset · Bloom amount (live preview) |
| **Theme** | Placeholder ("Phase 4+") |

### 4.3 `CoresPage`

- Header: Back · Refresh · `Add core…`
- Installed cores: per-core row with library name, file name, version, size, modified, supported extensions, update/remove buttons, "Set as default for…" dropdown, default-system chips.
- Browse cores (buildbot catalog): per-core blurb, supported systems badges, Install button, download progress bar.
- Status line for operation messages.

---

## 5. Settings tier matrix (overlap map)

| Setting | OA-wide | Per-system | Per-game | QuickSettings | Notes |
|---|:-:|:-:|:-:|:-:|---|
| Scaling mode | ✓ | scaffold | scaffold | shortcut→Settings | Per-system/per-game don't yet take effect |
| Window mode | ✓ | scaffold | scaffold | | |
| Monitor index | ✓ | scaffold | scaffold | | |
| Run-ahead | ✓ | — | — | | OA only — fine |
| Shader preset | ✓ | ✓ | ✓ | disabled | Quick Settings entry exists but is disabled "Phase 3" |
| Bloom amount | ✓ | ✓ (live) | ✓ (live) | | Three full-fledged sliders; clean inheritance |
| Rewind enable | ✓ | ✓ | ✓ | indirect | Per-game/system inherit chain |
| Rewind interval / cap | ✓ | ✓ | ✓ | | |
| Default core | ✓ (per-system) | ✓ (single) | ✓ (override) | tile menu | Three surfaces for the same notion, but each scope is justified |
| Core options | — | ✓ | ✓ | | OA-wide doesn't exist (cores are per-system; correct) |
| Input bindings | — | ✓ | — | — | Hardware-scope only. Correct. |
| ROM patch | — | — | ✓ | — | |
| Region priority (media) | ✓ | — | — | — | Global only |
| Region (game) | — | — | tile "Pick region" + drawer Region tab | — | Two surfaces for picking boxart variant — debatable |
| Milestones | — | — | ✓ | — | Per-game only |
| Cheats | — | — | drawer placeholder | — | **Backend ships; drawer tab is a placeholder. Orphaned.** |
| Audio device | ✓ | placeholder | placeholder | — | |
| Presentation mode | ✓ | — | — | — | |
| Library folders | ✓ | — | — | — | + Import Wizard step |

---

## 6. Orphaned / under-surfaced features

Cross-referenced from 127 Tauri commands → frontend `invoke()` sites.

### 6.1 Backend ships, no UI yet

- **Cheats** — full CRUD + `arm_cheats` wired; `PerGameSettingsDrawer.tsx` declares a Cheats tab but it's a placeholder. **Highest-impact orphan.**
- **Cheat search state machine** (`start/filter/peek/end_cheat_search`) — no UI; four-stage protocol unreachable.
- **ROM patching** (`pick_patch_file`) — `PerGameSettingsDrawer` → Core tab has a "ROM patch (IPS/UPS/BPS)" field but the picker plumbing isn't bound to the Tauri command. Half-built.
- **Core auto-download** (`available_cores`, `download_core`) — `CoresPage` shows a "Browse cores" section, but install path uses `probe_core_file` (manual files). Auto-download endpoints are dead.
- **Media storage stats** (`media_storage_stats`) — backend computes per-kind disk usage; UI shows it inline on the Game media tab (✓ partially surfaced, but no top-level "Storage" panel for cache/saves/states/exports).
- **Hash lookup** (`lookup_rom_hash`) — public command, never invoked. Could power a "What is this ROM?" tooltip / debug action.
- **Drop seed games** (`drop_seed_games`) — no "Clear placeholders" affordance.

### 6.2 Keyboard-only, no visible affordance

- F1 reset · F2 pause · F3 frame-advance · F5 save · F6 fast-forward · F7 slow-motion · F8 load · F12 screenshot — none are visible in QuickSettings.
- Ctrl+W unload — only hinted in Unload button tooltip.
- Ctrl+B sidebar toggle — only hinted in toolbar.

### 6.3 Disabled / "soon" placeholders cluttering the UI

- Left sidebar: Home · Favorites · Recent · Continue (4 of 5 destinations disabled).
- QuickSettings: `🎨 Shader` (Phase 3).
- PerSystemSettingsPage: Audio tab · Theme tab (both placeholders).
- PerGameSettingsDrawer: Audio · Input · Cheats (placeholders).
- Right sidebar widgets: no UI to reorder or hide widgets (the store fields exist).

### 6.4 Confusing duplication

- **Cores live in three places**: Settings → Cores tab (per-system defaults + installed list), Per-System → Cores tab (single dropdown), Per-Game → Core tab (override), plus the standalone `CoresPage` (install/update/remove). Four surfaces, three different intents — the Settings → Cores tab feels redundant with CoresPage.
- **Sidebar `Play` button** duplicates tile left-click.
- **Right sidebar `Saves` button** and tile context menu `Save states…` both open `SaveSlotsModal` — fine, but `GameInfoModal` *also* has an inline save-slot tab. Three doors to save states.
- **Region** picker appears in: tile context "Pick region…", PerGameSettingsDrawer → Region tab, and `GameInfoModal` regional variant strip. Three surfaces for the same action.
- **System per-system settings** are reachable five ways: GridControls ⚙ · SystemHeader buttons (4 buttons → 4 tabs) · SystemContextMenu (Edit bindings… + System settings…) · Sidebar `Cores` button (sort of) · `Settings → Cores`. Discoverability is great, but the surface area is high.

### 6.5 Missing UI for things that probably want a home

- A **"Now Playing"** persistent strip (cover, title, elapsed time) when a game is running and the library is shown.
- A **performance HUD** toggle (FPS, frame time, audio buffer) — backend has the data via emu thread.
- A **shortcut cheat-sheet** — every Fn key is invisible to a new user.
- A **screenshot gallery** — F12 produces screenshots; nowhere to view them.
- **Phosphor / shader live editor** — sliders for shader uniforms beyond bloom_amount.
- **Continue playing** / **Recently played** lists — backend tracks `addedAt`; no `lastPlayedAt` column surfaced.
- **Run-ahead per-system / per-game** — only OA-wide.
- **Theme overrides** at the per-system level (the tab exists but is a placeholder; theming registry already supports it).

---

## 7. Open-question observations

1. **`QuickSettings` is doing too much.** It's the in-game pause menu *and* the rewind UI *and* the TAS UI *and* the video capture UI *and* the memory inspector *and* the disc switcher. Each is a deep, specialized surface jammed into a single overlay tree. The drill-in views are essentially full pages.
2. **`PerGameSettingsDrawer` has 11 tabs, 3 of which are placeholders.** The drawer is 480px wide; some tabs (Milestones, Core options) really want more room than a side drawer affords.
3. **Settings tier-2 (per-system) "scaffold" fields** — Display tab settings exist in the UI but the comment explicitly says they don't take effect. Either wire them or hide them.
4. **There's no global activity surface.** Sync jobs run in the background and only surface as toasts + an embedded progress bar deep inside Settings → Game media. A multi-job state is invisible.
5. **Left-sidebar Quick Destinations is 80% aspirational.** Either implement Favorites/Recent/Continue or trim the section.
6. **Right-sidebar widget hide/reorder backend exists with no UI.** `widgetOrder` and `widgetHidden` are persisted but nothing lets the user change them.
7. **Two-window vs single-window** is exposed via Settings → Display → Shell mode but the difference isn't explained anywhere. New users won't know what they're choosing.

---

## 8. File index

- `frontend/src/App.tsx` — root composition, routing, keyboard handlers, drag-drop, idle-fade.
- `frontend/src/layout/{Shell,TopToolbar,LeftSidebar,RightSidebar,state}.tsx` — chrome.
- `frontend/src/layout/widgets/index.tsx` — hero · title · metadata.
- `frontend/src/components/LibraryView.tsx` · `GridControls.tsx` · `VirtualLibraryGrid.tsx` · `LibraryTile.tsx` · `DetailListView.tsx` — library views.
- `frontend/src/components/SettingsPage.tsx` · `PerSystemSettingsPage.tsx` · `PerGameSettingsDrawer.tsx` · `CoreOptionsPanel.tsx` — settings.
- `frontend/src/components/QuickSettings.tsx` — in-game overlay.
- `frontend/src/components/SaveSlotsModal.tsx` · `GameInfoModal.tsx` · `ImportWizard.tsx` · `RegionPicker.tsx` · `CorePickerMenu.tsx` · `CoresPage.tsx` · `SystemBindingsEditor.tsx` · `SystemHeader.tsx` · `ToastStack.tsx` · `TileContextMenu.tsx` · `SystemContextMenu.tsx`.
- `frontend/src/settings/store.ts` — OA-wide settings store.
- `frontend/src/library/{store,launch,ingest,filter,media,types}.ts` — library state.
- `frontend/src/themes/registry.ts` — per-system metadata.
