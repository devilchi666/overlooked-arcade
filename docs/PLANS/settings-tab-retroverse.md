# SETTINGS tab — Retroverse default theme

**Status:** Design sketch. No code.

**Author:** Operator + Claude (LLM pair).

**Owner-of-decisions:** the operator. This document records what was sketched.

**Reference:** [features/per-system-ui/assets/default-theme-mockup.png](../features/per-system-ui/assets/default-theme-mockup.png) (HOME) +
[features/per-system-ui/assets/library-default-mockup.png](../features/per-system-ui/assets/library-default-mockup.png)
(LIBRARY). This doc is the SETTINGS-tab companion to those.

---

## 1. TL;DR

The SETTINGS tab in the Retroverse default theme mirrors the LIBRARY
view's **three-pane shape** (categories left / content middle /
live-preview right) so the shell stays coherent.

It is specifically the **OA-wide** tier of the existing three-tier
settings split (OA-wide / per-system / per-game). Per-system
surfaces as a collapsed group at the bottom of the same sidebar;
per-game stays in LIBRARY's right-side game-detail panel and is
never merged here. The three tiers keep their separate UI surfaces
and separate persistence keys.

The four existing modal `SettingsDialogs.tsx` panes (Display /
Audio / Gameplay / Shaders) become categories inside this
persistent tab. The live-preview pane is the differentiator vs
today's modal surface.

---

## 2. Layout shape

```
┌───────────────────────────────────────────────────────────────────────────────────┐
│ ◤ RETROVERSE      HOME  LIBRARY  COLLECTIONS  PLAY NOW  DISCOVER  ▣SETTINGS▣  ⌕  │
│   EMULATION FRONTEND                                              09:47 PM · 👤  │
├───────────────────────────────────────────────────────────────────────────────────┤
│ ┌─ CATEGORIES ─────┐ ┌─ ▣ DISPLAY ──────────────────────┐ ┌─ LIVE PREVIEW ─────┐ │
│ │                  │ │ How the shell + emulator render  │ │                    │ │
│ │ OA-WIDE          │ │ on your monitor.                 │ │   ┌──────────────┐ │ │
│ │ ▣ Display      ◀ │ │                                  │ │   │ ▓▓▓▓▓▓▓▓▓▓▓▓ │ │ │
│ │ ♪ Audio          │ │ ┌─ Scaling ────────────────────┐ │ │   │ ▓ scaling  ▓ │ │ │
│ │ ★ Shaders        │ │ │ Mode      [ Pixel-perfect ▾] │ │ │   │ ▓ preview  ▓ │ │ │
│ │ ⏵ Gameplay       │ │ │ Integer   [☑ Snap if poss.]  │ │ │   │ ▓▓▓▓▓▓▓▓▓▓▓▓ │ │ │
│ │ ◉ Controller nav │ │ └──────────────────────────────┘ │ │   └──────────────┘ │ │
│ │ ✦ Per-system UI  │ │                                  │ │                    │ │
│ │ ▦ Themes         │ │ ┌─ Window ─────────────────────┐ │ │   Pixel-perfect    │ │
│ │                  │ │ │ Mode      [ Windowed      ▾] │ │ │   keeps each emu   │ │
│ │ CONTENT          │ │ │ Monitor   [ DELL U2723QE  ▾] │ │ │   pixel a whole    │ │
│ │ ▤ Library        │ │ │ Shell     ○ Two-window       │ │ │   number of        │ │
│ │ ⊞ Media          │ │ │           ● Single-window    │ │ │   screen pixels.   │ │
│ │ ⊙ Cores          │ │ └──────────────────────────────┘ │ │                    │ │
│ │ ⊟ BIOS           │ │                                  │ │   ⓘ Changes save   │ │
│ │                  │ │ ┌─ Run-ahead ──────────────────┐ │ │     automatically. │ │
│ │ SYSTEM           │ │ │ Frames    ────●─────  2  33ms│ │ │                    │ │
│ │ ⌑ Storage        │ │ │ One frame of input-lag       │ │ │                    │ │
│ │ 👤 Profile       │ │ │ elimination per click; costs │ │ │                    │ │
│ │ ⓘ About          │ │ │ more CPU per added frame.    │ │ │                    │ │
│ │                  │ │ └──────────────────────────────┘ │ │                    │ │
│ │ ▾ PER-SYSTEM     │ │                                  │ │                    │ │
│ │   (37 systems)   │ │ ┌─ Per-system UI ──────────────┐ │ │                    │ │
│ │                  │ │ │ Master         [ ●ON   OFF ] │ │ │                    │ │
│ │                  │ │ │ Boot anims     [ ●ON   OFF ] │ │ │                    │ │
│ │                  │ │ │ Tile flourish  [ ●ON   OFF ] │ │ │                    │ │
│ │                  │ │ │ Per-system SFX [ ●ON   OFF ] │ │ │                    │ │
│ │                  │ │ │ Background art [ ●ON   OFF ] │ │ │                    │ │
│ │                  │ │ └──────────────────────────────┘ │ │                    │ │
│ └──────────────────┘ └──────────────────────────────────┘ └────────────────────┘ │
├───────────────────────────────────────────────────────────────────────────────────┤
│  Ⓐ SELECT   Ⓑ BACK   Ⓧ SEARCH   Ⓨ RESET TO DEFAULT          RS CHANGE CATEGORY  │
└───────────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Sidebar groups (left pane)

### OA-WIDE — the four current dialogs + the three newer surfaces

- **Display** — scaling mode, window mode, monitor, run-ahead
  frames, plus the per-system-UI master toggle's sub-surface
  (boot anims, tile flourishes, SFX, background art).
- **Audio** — output device, plus the four-bus mixer levels from
  the media-taxonomy Phase 4 work (platform-music / ui-sounds /
  ceremony / snap-audio).
- **Shaders** — preset picker, phosphor bloom amount.
- **Gameplay** — rewind enable / capture interval / buffer cap,
  fast-forward speed, autosave cadence.
- **Controller nav** — master toggle, navigation source, A↔B swap,
  animation budget (matches the existing controller-nav v2 polish
  Settings → Display sub-panel; promoted to its own category here
  because navigation feel is OA-wide enough to deserve top-level
  surface).
- **Per-system UI** — master toggle + the five consumer-side
  switches (currently nested under Display → "Per-system
  experiences"; promoted here because it's a top-level switch
  that ~37 systems all participate in).
- **Themes** — default OA theme picker. Today this is a single
  built-in theme; the category is reserved for when shells become
  swappable (Retroverse vs Heroic-style vs kiosk).

### CONTENT — what's in the library and how to keep it fresh

- **Library** — scan paths, scanner cadence, "ignore folders"
  list. Today reachable via `LibraryManagerPage`.
- **Media** — per-platform art slots (the nine PlatformMedia slots
  from media-taxonomy Phase 6 — banner, clear-logo, console,
  controller, fanart, marquee, photo, wheel, background).
  Replaces direct entry to `PlatformMediaDialog`.
- **Cores** — installed libretro `.dll`s with version, build date,
  update-from-buildbot action. New surface; today this is manual
  file management in `<exe_dir>/cores/`.
- **BIOS** — per-system BIOS status grid with green/amber/red dots
  and a "where does this go?" hint per row. New surface; today
  the BIOS check fires lazily at launch time.

### SYSTEM — about the OA install itself

- **Storage** — data dir path, portable install marker status,
  saves location, free space.
- **Profile** — avatar + display name. Drives the top-right
  profile chip.
- **About** — version, GPL notice, credits, "report a bug" link.

### PER-SYSTEM ▾ — collapsed group, separate tier

Expanding this group reveals one row per installed system
(matching the SYSTEMS sidebar from the LIBRARY mockup). Selecting
a system jumps to that system's `SystemSettingsDialog` content
rendered in the middle pane — but visually distinct (banner
header, accent border) to make the **"you're in per-system tier
now"** transition unmistakable.

Per-game settings tier is **not** present here. It lives in
LIBRARY's right-side detail panel + MORE → per-game settings
drawer. Three tiers, three surfaces; never merged.

---

## 4. Center pane (active category)

Glass-morphism cards matching the LIBRARY mockup's tile +
sidebar treatment:

- Rounded corners, dark navy fill, subtle border.
- Accent ring on focus (matches `data-oa-focus-active` from
  controller-nav v2 polish).
- Section header sits inside the card's top edge as a small
  caps label.
- Card title + inline form controls; help text below the
  primary control in a muted gray.

Categories with many sections (Display, Audio) stack 3-4 cards
vertically; categories with one purpose (Profile, About) get a
single richer card.

---

## 5. Right pane — live preview

This is the differentiator vs today's modal dialogs. Different
category, different preview:

- **Display** → scaled sample tile showing the chosen scaling
  mode + window mode.
- **Audio** → four-bus output meter showing live levels.
- **Shaders** → a phosphor-effect preview tile.
- **Gameplay** → diagram of rewind capture interval vs buffer
  cap (shows "how many seconds of history" you get).
- **Controller nav** → a tiny sample focus group the operator
  can DPad around in to feel the animation budget.
- **Per-system UI** → a sample tile + boot animation preview
  loop for the currently-highlighted system in the per-system
  group below.
- **Themes** → a sample LibraryTile + sidebar row rendered in
  the selected theme.
- **Library / Media** → status info instead of preview (last
  scan time, items found, errors).
- **Cores / BIOS** → status table (last update / file hash /
  size).
- **Storage / Profile / About** → just contextual help and
  hover-targets.

Auto-save reminder pinned at the bottom for the categories
where changes persist immediately (most of them — no Apply
button needed). Categories that DO require explicit apply
(rare — maybe data-dir migration) show an `APPLY` button at
the bottom of the right pane.

---

## 6. Footer hint bar

`Ⓐ SELECT` / `Ⓑ BACK` / `Ⓧ SEARCH` / `Ⓨ RESET TO DEFAULT` /
`RS CHANGE CATEGORY`. `RS` parallels the LIBRARY mock's
`RS CHANGE SYSTEM`. Reuses the existing controller-nav HintBar
primitive.

`Ⓨ RESET TO DEFAULT` resets only the currently-focused card's
fields, not the whole category. Whole-category reset lives
behind a confirm dialog reached from the `⋯` more menu on the
category header.

---

## 7. Notable deltas vs current OA settings

- The four `SettingsDialogs.tsx` modals become non-modal
  categories inside this persistent tab. Same surface area,
  just always-visible.
- The live-preview pane is net-new. Cheap for Display / Shaders
  (already a thing in the shader-preset work), more interesting
  for Themes once the theme system can render a sample tile in
  isolation. The pane stays present even when the preview is
  weak (contextual help fills the space instead).
- Per-system tier appears as a *collapsed group* in the same
  sidebar rather than its own top-level tab. Maintains tier
  separation without giving per-system equal weight to OA-wide.
- BIOS and Cores become first-class settings categories rather
  than file-management chores. Status-grid surface helps the
  operator see at-a-glance which systems are healthy.
- Themes is reserved-for-future. Today's theme list is
  effectively `[default]`; the category is plumbing for the
  eventual Heroic-style / Kiosk swap.

---

## 8. Implementation sketch (not committed)

Not a green-lit implementation plan — rough mapping in case it
ever ships:

- New `SettingsPage` route at `frontend/src/routes/Settings.tsx`,
  hosting the three-pane shell.
- Each category is a `SettingsCategory<T>` component receiving
  the OA-wide store slice it owns; today's `DisplayDialog` /
  `AudioDialog` / `GameplayDialog` / `ShadersDialog` bodies
  port over with minor restyling (cards instead of
  `DialogSection`).
- Right-pane preview is a polymorphic `LivePreview` component
  the active category renders into. Each category exports its
  own preview body.
- Per-system group reuses the existing `SystemSettingsDialog`
  body inside the middle pane when expanded — no new per-system
  category code.
- Mock data + a category registry would let this ship without
  rewriting Cores / BIOS internals (they'd render existing
  state read-only at first, gain actions later).

Status: idea, not in `ACTIVE_WORK.md`. Implementation will
follow once the operator green-lights the design and decides
where this lands relative to the existing per-system-ui Stage 2
arc.
