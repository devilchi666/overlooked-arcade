# Kiosk / Cabinet Mode — Design Plan

> **SUPERSESSION (2026-06-15, DECISIONS D7):** The **theme substrate + in-engine Theme Studio** scope in this plan (notably §2.2-2.4) **migrated to [../theming-substrate/](../theming-substrate/) (ARCs 1-4)**. What stays kiosk-specific = attract mode / multi-monitor surfaces / 5-bus audio mixer (the D20 deferred platform capabilities). Content is preserved below; read the theming/Theme-Studio sections as superseded-for-substrate.

> **STATUS: 📐 DESIGN-ONLY — NOT YET IMPLEMENTED.** Full design locked 2026-05-22 (this doc). Phase 0 (desktop polish) ran in parallel via `../../_archive/features/ui-polish/UI_POLISH_PLAN.md` (✅ shipped) + `../../_archive/features/ui-polish/UI_MENU_BAR_PLAN.md`. Phase 1+ (kiosk shell itself: `--kiosk` flag, theme substrate, attract mode, in-game menu, 5-bus mixer, multi-monitor surfaces) has not begun.

**Status:** Someday plan. Designed but not on the active roadmap. Pick this up after current polish + settings IA work lands.

**Date settled:** 2026-05-22 (planning conversation).

**Purpose:** Capture every design decision settled during the kiosk-mode planning conversation so a future session can pick this up without re-deriving anything. Companion documents that already exist on this topic: `../../_archive/features/ui-polish/UI_AUDIT.md` (today's UI surfaces — informs what changes for kiosk) and `../../_archive/features/ui-polish/UI_MENU_BAR_PLAN.md` (desktop-mode IA polish — runs in parallel to / before this).

This file is the source of truth for *what we decided*. When implementation starts, individual phases get their own working docs.

---

## 1. Center of gravity

- **Match or exceed BigBox**, with one acid test: the **moment of arrival** on a focused game tile (snap video preload, marquee swap, fanart crossfade, audio fade-in, all synchronized) is the design centerpiece. Everything else is mechanics around delivering that moment cleanly.
- **Blistering speed** — BigBox lags; we don't. Achieved through the architectural firewall between theme runtime and render thread (scripts never run per-frame), not through language choice.
- **Theme-able for the community** — substrate accessible enough that a competent web/CSS person can ship a usable theme in an afternoon, with shader hooks for power users.

---

## 2. Architecture decisions (settled)

### 2.1 Mode model: single binary, mode switch

- One `oa-shell.exe`. Launch normally → desktop UI. Launch with `--kiosk` or flip the toggle in settings → desktop chrome unmounts, kiosk chrome mounts. Renderer / audio / library / cores don't notice.
- **No separate kiosk-only build.** Cabinet operators who want lockdown get there via OS-level Windows kiosk account, not a separate binary.
- **Configuration depth lives in desktop mode**, play-time settings in kiosk. Cabinet operators boot to kiosk; when they need deep config, they switch to desktop mode in-app, configure, switch back. No separate launcher app.

### 2.2 Theming substrate: four layers

> **SUPERSEDED-FOR-SUBSTRATE (2026-06-15, D7):** §2.2 (substrate), §2.3 (Theme
> Studio), and §2.4 (theme distribution) below describe scope that **migrated to
> [../theming-substrate/](../theming-substrate/) (ARCs 1-4)**. Preserved here as
> the originating design record; the live home for this work is the
> theming-substrate feature folder.

| Layer | Format | Purpose | Touches render thread? |
|---|---|---|---|
| **Layout** | Declarative TOML/RON | Element tree, anchors, sizing, asset slot names | Parsed once at theme load → compiled render tree. No. |
| **Animations** | Declarative timelines | Keyframes, easing, triggers (`on_focus_enter`, `on_select`, `on_scroll_settled`) | Compiled to native animator structs at load. Animator state read per frame. No script execution. |
| **Behaviors** | Rhai scripts | Event-driven (`on_game_focus`, `on_platform_change`, `on_scroll_velocity_crossed`). Mutates a snapshot struct the renderer reads. | Never per-frame. Never blocks render path. |
| **Shaders** | WGSL slots | Transitions, CRT effects, glow, bezels. Picks from built-in library or ships custom. | Standard `oa-render` pipeline. |

**Why Rhai for scripts:** Rust-native, no GC pauses, sandboxable, integrates cleanly with our Rust types without a glue layer. Sandbox is non-negotiable (see §11).

### 2.3 Theme Studio: in-engine editor

- Built into `oa-shell` itself, accessible via `--theme-studio` or settings toggle. Not a separate companion app — separate apps mean separate renderers, and pixel-divergence becomes the dominant bug class forever (LaunchBox's "Big Box Theme Creator" failure mode).
- Webflow model: visual editor + code tab on the same underlying file. Drag elements visually, hop to code view to write Rhai, hop back. File on disk is the source of truth.
- Hot-reload runs both directions: edits in Theme Studio write to disk; edits in VS Code (or any editor) are picked up by Theme Studio. Both workflows usable simultaneously.
- Theme Studio layout: left dock = element tree, right dock = inspector, bottom dock = asset browser, floating = shader sliders, center = live theme with selection handles, plus "simulator" controls (pick any game, fire `on_focus`, scrub animations frame-by-frame).
- Undo policy: file is truth. Studio undo stops at the last external file save (rather than blindly clobbering external edits).

### 2.4 Theme distribution

- **Two formats**: folder of loose files during dev (hot-reloadable from VS Code), `.oatheme` zip archive for shipping (manifest + assets + scripts + shaders, signed by author, schema-versioned).
- **Federated, GitHub-anchored Index.** Anyone publishes `.oatheme` anywhere (own site, Itch, etc.). We maintain a lightweight "OA Theme Index" — JSON in a public GitHub repo listing known themes with canonical URL, screenshots, description, version, OA version requirement, optional author Patreon/Ko-fi link.
- **Index criteria** (inclusion): parses, screenshots real, URL live, target OA version declared. Not curated for quality. Submission via PR.
- **Index criteria** (removal): theme attempts sandbox escape (added to public blocklist), illegal/abusive content, URL dead. Not removed for being ugly. All Index changes via public PR — auditable, contestable.
- **Themes are NOT signed by OA.** We don't gatekeep. The Rhai sandbox enforces safety at runtime.
- **Theme manifest declares `min_oa_version` / `max_oa_version`.** Newer-than-max: warn but allow. Older-than-min: reject with clear message. Schema migration tool: `oa-theme migrate v1.0 v2.0 my-theme.oatheme`.
- **Updates**: pull-on-startup check + manual refresh in browser. Always require explicit user confirmation for install. Never silent auto-update of visual content.
- **Reviews/ratings**: GitHub Issues on the Index repo, tagging convention. Zero infrastructure cost.
- **Author links to Patreon / Ko-fi / personal site** appear in the in-app browser. We take no cut.

### 2.5 Reference themes (4 shipped)

| Theme | Identity | Pressure-tests |
|---|---|---|
| **Showcase** | BigBox-killer. Rich, video-heavy, multi-monitor. Cinematic transitions. | The moment-of-arrival ceiling. |
| **Minimalist** | Dim grid, box art + title, no video unless held. | Substrate's ability to do *less* gracefully. |
| **Cabinet** | Arcade aesthetic. CRT shader, marquees always lit, aggressive attract, arcade-flavored SFX. | "I built a real cab" use case. |
| **Kids** | Bright, colorful, simplified UI, larger tiles, attract off by default. | Pairs with kid mode (§9). |

If all four are tractable, the substrate is right.

---

## 3. Navigation

### 3.1 Five built-in patterns + escape hatch

Each pattern is a fully-featured layout/input/animation module with theme-configurable knobs (orientation, density, tile size, focus prominence, easing curves). Engine owns input semantics (D-pad → next-detent for wheel, next-cell for grid, etc.).

1. **Wheel** — iconic curving vertical list.
2. **Grid** — rows × columns of box art.
3. **Coverflow** — 3D depth-sorted Apple-style.
4. **Text-list** — compact, fast, for systems with 800+ games.
5. **Mosaic** — irregular, magazine-style.
6. **`custom-layout`** — theme implements own positioning function in Rhai. Escape hatch.

### 3.2 Jump layer (first-class, not menu chrome)

- **Letter-jump** — hold modifier → A-Z chooser → d-pad pick → scroll lands at section. Phone-contacts style.
- **Search overlay** — virtual keyboard with d-pad nav + **predictive results live below it from 2-3 characters**. The model is "type a few letters → pick from suggestions," not "type the whole word." Fuzzy match by default (`castlavania` finds Castlevania). Boost matches by recent-play history.
- **Filter sheet** — controller-pickable values for Genre / Year / Players / Rating / Region. Live-applied to current view, persists until cleared, shown in breadcrumb.
- **Phone companion** for text input: **architect the seam, defer the build.** Search accepts an external text-input event source from day one, but the QR-paired local-web-page implementation ships later.
- **Search index**: in-memory, sub-100ms response. nucleo-matcher scale (not Tantivy heavyweight). Rebuilt on library scan, incrementally updated.

### 3.3 Named views with arbitrary hierarchies

- **Default view**: BigBox-shaped, Platform → Game (2 levels). Floor.
- **User and theme can build any tree**: Decade → Year → Platform → Game, Genre → Platform → Game, Region → Manufacturer → Platform → Game, etc.
- **Nav tree is a *view*, not a folder structure.** A single game appears under multiple branches in different views simultaneously. We project the library through hierarchical filters; never copy game data.
- **Each level renders the same way** — list of children using whatever nav primitive the theme defines for that level, plus a context label. Themes don't need to know their depth.
- **Breadcrumbs are essential.** Top of screen shows `Platforms / PCE / Action`. Controller-B navigates one level up; Y opens a quick-jump menu to any ancestor.
- **Hierarchy authoring lives in desktop mode**, kiosk consumes. Theme Studio can preview hierarchies but doesn't author them.
- **Metadata fallback buckets**: a "Genre → Game" hierarchy needs "Unknown Genre" so undefined-metadata games don't vanish.
- **Soft depth ceiling**: warn at 5+ levels, don't hard-cap.
- **Per-system theming neutral state**: themes need a "neutral mode" for non-platform-scoped nodes (top of a Genre View has no platform context yet).
- **Saved hierarchies**: both theme-shipped (advisory, disappear when theme switches) and user-built (persistent across theme switches). User-built is the persistent layer.

### 3.4 Library organization

- **Platforms** — base axis, one per system.
- **Playlists** — user-curated, can span platforms. Castlevania (PCE) and Resident Evil (PSX) belong in the same "Spooky" list together.
- **Smart playlists** — rule-based DSL, e.g. `year >= 1985 AND year <= 1990 AND (genre CONTAINS "shmup")`. **Expression DSL with a generous library of pre-built starter playlists** ("All Multiplayer," "All 1986 Games," "All Highly-Rated Shmups," "All Games You Haven't Played"). Users copy-and-tweak. No visual editor — Theme Studio's visual-edits-the-file model doesn't earn its keep for low-frequency smart-playlist authoring.
- **Four hardcoded rails** (not user-curated, always exist): Recents (last ~20 played), Favorites (single-button heart), Most-Played (by accumulated playtime), Just-Added (last 30 days).

### 3.5 Top-level structure

- **Two-axis** (Switch-home model): top axis is "Platforms / Playlists / Recents" (themes can add more, e.g. Showcase adds "Just Added"). Second axis is the list within. Scales cleanly to 36+ systems because they live one level down inside the Platforms entry.
- **Cross-platform playlist behavior**:
  - Marquee shows focused game's marquee; playlist marquee is fallback when no game focused.
  - Inside cross-platform playlist, theme uses the playlist's theme override (or theme-default neutral). Tiles still show their platform's color accent as a smaller cue.
  - Background music: theme-defined per-playlist override.

---

## 4. Moment of arrival

The acid test of the whole design.

- **200ms hover threshold** before any expensive work fires. Below threshold = scrolling, no preload.
- After threshold: snap video preload + audio fade-in (400ms default, theme-controllable) + marquee push to second window + fanart crossfade. All synchronized.
- All interruptible — if user scrolls fast, the moment-of-arrival is cancelled and only the focused tile name renders (settle-then-transition).
- **Theme-controllable curves**: fade-in duration, audio EQ ramp, snap-start timing relative to visual. Minimalist might use 150ms hard cut; Showcase 700ms with EQ ramp; Cabinet zero fade.

---

## 5. Attract mode (three tiers)

The kiosk-idle state. Configurable: rotation source (all / favorites / playlist / per-platform), interval, **separate volume ceiling**, monitor dimming for burn-in, hard duration cap. Any input cancels within 50ms.

| Tier | What it is | Cost |
|---|---|---|
| **1 (baseline, ship first)** | Snap video cycle. ~10s per game. | Cheap, works for everything. |
| **2** | Pre-recorded attract videos. Background pass at first-launch runs each game for 30s, captures, saves as `<game>.attract.mp4`. | Background CPU cost once; runtime is just video playback. |
| **3 (the differentiator)** | **Live emulator attract.** Pre-baked save states at "BIOS done, attract waiting." Restore state → run N seconds → fade out → next. Restore is tens of ms; warm-rotate one core at a time. | Engineering cost real; demos in 10s with instant differentiation. |

Cabinet theme defaults to Tier 3 where supported; Showcase/Minimalist default to Tier 2. Tier 1 is the safety net.

---

## 6. Audio mixer

### 6.1 Five buses

| Bus | Source | Notes |
|---|---|---|
| Platform music | Theme-owned + user-overridable per system | Plays continuously, crossfades on platform switch. |
| Snap audio | Game preview videos | Gated by 200ms hover threshold. |
| Live game audio | Attract Tier 3, future preview-play | Distinct from snap (different post-processing — e.g. crunchy filter for Cabinet). |
| UI sounds | Theme-owned | Short, doesn't duck anything. |
| Announce / ceremony | Theme-owned | Game-launch chimes, "PLAYER 1 READY," station idents. |

### 6.2 Ducking matrix (theme-overridable)

- Platform music ducks 6dB when snap audio active.
- Platform music ducks 12dB, snap ducks 6dB, when announce/ceremony plays.
- Everything except UI sounds ducks 18dB during attract crossfades.
- UI sounds never duck anything.
- Cabinet theme will likely disable most ducking (layered noise *is* the aesthetic). Minimalist ducks aggressively. Showcase sits between.

### 6.3 Other audio decisions

- **Audio crossfades are slower than visual** — 1-2s audio crossfade during a 600ms visual transition. Audio cuts shorter than that read as harsh.
- **Layered audio** (game + music with ducking) is supported from day one.
- **Sample-rate handling** via rubato (or libsamplerate). Game audio comes out of cores at native rate (PCE 44.1kHz); snap videos usually 48kHz. Mixer resamples to output bus.
- **Theme-controllable audio fade curves** for moment-of-arrival, with sensible defaults per theme personality.
- **No engine-default audio assets.** Themes own all sonic content (UI sounds, platform music, system SFX). Engine ships zero audio.

---

## 7. Multi-monitor / surfaces

### 7.1 Three configurations

- **Single** — most home users. Everything on one display.
- **Main + Marquee** — classic cabinet. Marquee shows focused game's marquee art (and platform marquee when no game focused). Crossfades synced with moment-of-arrival.
- **Main + Marquee + Control panel** — high-end cab. Third surface shows game-specific control overlays ("□ = jump, △ = kick") or decorative fanart.

### 7.2 Architecture

- **Theme manifest declares which surfaces it supports** and provides a layout per surface. Cabinet ships all three; Minimalist might be main-only.
- **Fallback layouts**: when theme doesn't define a surface but user has the hardware, fall back to a default marquee layout (platform logo + game logo on themed background).
- **Sync between surfaces**: single coordinated state snapshot, surfaces are render targets of the same scene (not separate render contexts).
- **Manual monitor-to-surface assignment** with "show test pattern on this screen" button. Auto-detect is fragile (laptop external displays, USB monitors, hot-plug); we don't go there. Controller-driven, in kiosk settings.
- **Marquee asset pipeline**: per-game `marquee.png` / `marquee.mp4`. Extend existing media scraper; libretro-thumbnails has marquee folders for many systems.
- **Aspect tolerance**: cabinet marquees are weird (16:5, 16:6, USB-attached, sometimes CRT-via-transcoder). Theme layouts must be aspect-tolerant.

---

## 8. Launch ceremony

### 8.1 Four visual beats (theme-owned timeline)

1. **Acknowledgment** (~150ms) — tile zoom, focus reticle pulse, audio chime. Confirms input registered, gives core a head start.
2. **Ceremony** (variable, theme-defined) — marquee swap, fanart fill, snap audio swell, optional voice clip, optional overlay text.
3. **Fade-out** — kiosk UI dissolves to black or to theme's launch color.
4. **First frame** — cross-faded in, gated by **first non-trivial frame** heuristic (>5% non-black pixels). BigBox botches this; we don't.

### 8.2 Concurrent with core init

- **Ceremony and core init run in parallel**, not sequentially. Theme declares a `min_duration` and a "wait for ready" hook. When both ceremony minimum AND first non-trivial frame are ready, cross-fade in.
- Slow cores (PSP cold-init ~2s, some Saturn ~3s) hide behind the ceremony's natural runtime. Themes feel slightly different across systems (cabinet ceremony might run 4s on PCE but 5.2s on PSP — minimum vs cold-init dominated). Acceptable tradeoff for the responsiveness payoff.
- When core init exceeds ceremony duration by >500ms, surface a designed "Loading [Game Title]…" overlay with subtle progress.

### 8.3 Error handling

- Every failure mode has a designed error overlay with clear next-action: "PCE-CD requires `syscard3.pce`. Drop it in `<exe>/system/` and try again." Controller-dismissable.
- Overlay chrome is theme-styled. Error text and remediation steps are engine-owned (so they stay accurate as cores update).

---

## 9. Session persistence

### 9.1 Resume policy

- **Default: auto-resume from `last_session` slot**, with hold-B-at-launch bypass to start fresh. Bypass surfaced in launch ceremony overlay ("Resume — hold B for fresh start"). Per-system setting can switch to prompt mode for purists.
- **`last_session` is conceptually separate from numbered user slots.** Doesn't appear in slot picker; only powers Resume prompt. Numbered slots are intentional checkpoints.

### 9.2 Auto-save

- **Silent auto-save on exit** (no prompt). Auto-resume / auto-save loop only works if both ends are silent.
- **Auto-save every 30s during play** (for core crash safety — extends to "auto-save before any core call exits unexpectedly").
- Cores that can't save state safely mid-emulation: auto-save is best-effort and silently degrades to "last played slot or fresh start." Resume policy makes this visible: "Couldn't resume — start fresh?" overlay.

---

## 10. In-game menu

### 10.1 Behavior

- **Game freezes** (last frame held, audio muted). Same as Switch / Steam Big Picture / BigBox. The "need a moment to think" expectation matters.
- Semi-transparent overlay composes on top.

### 10.2 Contents (priority order)

1. **Resume** (default selection)
2. **Save state** — slot picker with thumbnails of captured frame
3. **Load state** — same picker
4. **Shader / preset** — cycle theme-defined shader presets
5. **Screenshot** — saves to configurable folder
6. **Game settings** — per-game overrides
7. **Exit to kiosk** — clean exit, triggers auto-save
8. **Exit to desktop mode** — drops into configuration shell

### 10.3 Save-state slot UI

- 9 named user slots + the `last_session` auto-slot.
- Each slot has thumbnail of captured frame + last-played timestamp + one-button overwrite.
- Steam Big Picture style; BigBox is confusing (numbered slots, no thumbnails).

### 10.4 Trigger

- **Dedicated hotkey OR combo**, user-configurable. Defaults: Select+Start held 500ms. Arcade-stick users with a real "Hotkey" button on the panel map a single-button trigger. The libretro input proxy reserves the combo from the game when held past threshold.

---

## 11. Controller mapping (shell-level)

### 11.1 Action vocabulary (~18 actions, three tiers)

- **Core nav (must be bound)**: Up, Down, Left, Right, Select/OK, Back.
- **Common (recommended)**: In-Game Menu, Search, Filter, Letter-Jump, Favorite-Toggle, Switch-View, Quick-Recents.
- **Advanced**: Screenshot, Save State, Load State, Theme-Switch, Exit Kiosk, Volume-Up, Volume-Down, Brightness.

Themes can declare theme-specific actions (Cabinet theme might add "Insert Coin" SFX trigger).

### 11.2 Binding shapes

All bindings configurable. Three trigger shapes for any action:
- **Single button**
- **Combo** (e.g. `combo("L1+R1")`)
- **Hold-modifier** (e.g. `hold("R1", 500ms)`)

### 11.3 Profiles

- **Named profiles**, switchable: "Home Gamepad," "Cabinet Build," "Fight Stick," "Coffee Table."
- **Default profiles shipped**: Generic Xbox / PS / Switch Pro, IPAC-style 2P6B arcade, minimal arcade (4-button 1P), Fight stick (1P 8B + start + select).
- **Per-controller-port "shell pilot"** — only player 1 (or designated pilot) drives the kiosk shell. Other controllers wake up when game launches. Pilot port configurable.

### 11.4 Minimum-button cab handling

- **6-input hard minimum** at binding time (warn below).
- **Function-Menu hold** as fallback: one designated button held >500ms opens a controller-navigable list of every unbound action. The "in case of emergency, break glass" path.

### 11.5 Authoring paths (all three coexist)

- **Hand-edit JSON/TOML** — source of truth on disk. Power-user-friendly.
- **Walkthrough wizard** — kiosk runs "Press the button you want for Up... now Down..." until required bindings are set. Primary user-facing flow at first-launch + after detected-new-controller.
- **Community `.oaprofile` files** — users drop into a folder, kiosk imports. Cabinet builders share their IPAC + Cabinet-theme config on r/cade. Real differentiator for the cab community.

---

## 12. Transitions

### 12.1 Six scales

| Scale | Trigger | Duration | Treatment |
|---|---|---|---|
| Micro | Focus a tile (moment-of-arrival) | 200–400ms | §4 |
| Mid | Enter/exit a hierarchy node | 300–500ms | Slide in/out |
| Macro | Switch platforms (per-system re-theme) | 500–1000ms | §12.2 |
| View switch | Hotkey named view | 400–600ms | Cross-blend |
| Mode switch | Desktop ↔ kiosk | ~500ms total | Fade-to-black + swap |
| Launch ceremony | Kiosk → game | Theme-defined | §8 |

### 12.2 Macro re-theme

- **Theme-configurable per platform.** A theme can declare different transition styles for different systems. PCE uses TurboGrafx hex-grid wipe; Lynx uses handheld-screen-power-on flicker; PSX does disc-tray-slide.
- Theme-overall defaults: Showcase → cinematic 800ms crossfades; Minimalist → 150ms snap; Cabinet → arcade-flavored CRT-degauss.
- **Audio crossfade timing differs from visual** (audio 1-2s, visual 400-800ms; see §6.3).

### 12.3 Settle-then-transition

- Rapidly scrolling past 5 platforms in 800ms cannot trigger 5 sequential 800ms re-themes. Detect scroll velocity, pause transitions, render minimal "currently scrolling" chrome (just focused name). When scroll stops (150ms idle), play full transition to current focus.

### 12.4 Cancellation: reverse animation

- User presses A to enter PCE, immediately presses B. Animation reverses, lands back at Platforms. Physically intuitive. Costs nothing on average case; feels right when used.

### 12.5 System-specific SFX

- **Theme-owned, no engine defaults.** Themes ship their own SFX in `.oatheme` archives if they want cartridge-insertion clicks, CD spin-up, arcade power-on hum.

---

## 13. Kid mode (single state, v1)

Multi-user profiles (Switch/PS5 model) deferred to v2 if demand surfaces. Single kid mode covers the actual common case.

### 13.1 What kid mode hides

- Library filtered by three stackable criteria: explicit per-game allowlist, rating filter (E / E10+ only), tag filter ("kid-safe" tag).
- Save states from non-kid sessions hidden — kid sees only their own saves, can't accidentally overwrite dad's Castlevania progress.
- Hidden games physically don't appear in any view, search, smart playlist, or attract. Not greyed-out — *invisible*. Greyed invites curiosity.

### 13.2 What kid mode locks

- No exit-to-desktop (menu item hidden, not greyed).
- No settings beyond play-time controls (volume, brightness).
- No theme switching.
- Optional: no in-game menu.
- Optional: **time limits** with auto-save-and-friendly-empty-state when up (uses §9.2 auto-save machinery).

### 13.3 Entry / exit

- **Enter from desktop**: Settings → Kid Mode → toggle, define filters, set PIN (or 4-button combo for cabinets). Confirm twice.
- **Exit while in kid mode**: hold 4-button combo 5 seconds (default `Select+Start+L1+R1`, configurable). Prompts for PIN. Wrong 3 times → 10-minute cooldown to prevent brute force.
- Cabinets without 4-button combo capability: kid mode is permanent until intervened-with from a different OS user. Documented limitation, not a bug.

### 13.4 "kid-safe" tag + Kids theme

- We ship a "kid-safe" tag the user applies manually or via bulk-apply ("apply to all E-rated games"). User curates their own kid library.
- **Kids reference theme** (the fourth shipped theme): bright, colorful, simplified UI, fewer hierarchy levels, larger tiles, attract off by default. Recommended when kid mode is on.

### 13.5 Kid attract

- **Theme-configurable** — Kids theme opts into curated kid attract with ambient-only audio; other themes suppress attract while kid mode is on; user can override either way.

### 13.6 Honest scope

Lockdown is at the kiosk shell, not the OS. OA can lock its own UI but can't prevent Alt-Tab. Real cab lockdown uses a Windows kiosk account. We document this; we don't pretend OA enforces it.

---

## 14. Accessibility

Seven first-class surfaces. The retro community we're serving needs this; nobody else in this space takes it seriously.

1. **Reduced motion** — clamps all easing durations to ≤120ms when on. Themes must honor this; enforced in animation runtime.
2. **UI scale** — 100% / 125% / 150% / 200% for text and chrome. Theme layouts must be responsive; Theme Studio gets a "preview at scale" toggle.
3. **High-contrast / color-blind variants** — flag on existing theme, not a separate theme. Boosts text contrast, uses color-blind-safe palette.
4. **Adjustable hold thresholds** — 0.5× / 1× / 2× / "disable holds, use sequential menus instead." With holds disabled, Function-Menu hold falls back to "tap to open." Surface at first-launch: "Use long-press combos?"
5. **Captions for snap audio** — best-effort descriptors ("music, mid-tempo guitar, light SFX"). Don't promise transcriptions we can't deliver.
6. **Single-switch / sequential-menu mode** — one input scans through choices sequentially at adjustable speed.
7. **TTS announcements for navigation** — OS-native (Windows SAPI / macOS Speech Synthesis). Announces focused game / platform / hierarchy level on focus change. Goes through the Announce audio bus. **Not a full screen reader** — wgpu canvas isn't an accessible surface.

### Defaults that affect everyone

- Never animate at <60fps regardless of monitor refresh (choppy = nausea for some).
- No camera-shake or parallax in default themes; opt-in, auto-disable when reduced-motion is on.
- Flash respects WCAG 2.1 threshold (<3 flashes/sec in any 1s window). Theme Studio linter flags violators.

### First-launch + persistent surface

- **Both**: skippable wizard during onboarding ("Do you need accessibility accommodations?") AND persistent `Settings → Accessibility` panel.

---

## 15. First-launch onboarding

**Setup-first**, no interactive tour. Earn the user's attention by getting their library working; if the kiosk doesn't sell itself afterward, the design failed.

1. **Welcome** — "Let's set up your library." Two buttons: Quick setup / Skip.
2. **ROM folders** — auto-detect common locations, user adds multiple.
3. **Cores check** — installer ships ~10 default cores (Beetle PCE Fast, Beetle Lynx, Stella, Genesis Plus GX, FCEUmm, Snes9x, etc.). For others, one-click "Download from buildbot?" with verify-signature.
4. **BIOS check** — per-system drop targets with explicit messaging ("PCE-CD needs `syscard3.pce`"). Skip per system. We never redistribute BIOS.
5. **Metadata scrape** — kicks off in background with progress bar (~643/1247, ~23 min remaining). User can advance and start using kiosk while it runs.
6. **Ready** — "1,247 games across 14 systems. Launch Kiosk Mode now? [Yes — set up controllers] [Not yet — stay in desktop]." Yes triggers binding walkthrough (§11.5).

**Default theme**: Showcase. **No theme choice at first launch** — choice paralysis. User switches later.

**Empty-state behavior** matters as much as the happy path. If user skips or has no ROMs detected, show empty state with three clear actions: Add ROM folder / Drop ROM file here / Read guide. Not a dead screen.

**Crash recovery on first launch**: state scrape pass to disk per-game so we resume gracefully, not restart.

---

## 16. Performance budgets

### 16.1 Frame budget

- **60Hz** fallback floor: 16.6ms/frame.
- **120Hz** standard target: 8.3ms/frame.
- **144Hz** stretch: 6.9ms/frame.
- Kiosk shell render must fit in 4ms at 144Hz, leaving ~3ms headroom for asset decode, audio mix, scroll physics.
- Emulator runs on its own thread to its own framebuffer; presentation decoupled (shell composites latest emulator frame).

### 16.2 Latency targets

- Button press → visual ack: **<50ms** (slides to 100ms and kiosk feels sluggish).
- Scroll → tile update: <16ms (one frame at 60Hz).
- Attract cancel → kiosk back: **<50ms**.
- UI sound onset: <5ms.
- Snap audio start: <20ms after hover threshold expires.
- Ducking onset: <50ms.
- Cold launch (cabinet boot) → first kiosk frame: **<2 seconds**.
- Mode switch (desktop ↔ kiosk): <500ms total (150ms out + swap + 250ms in).
- Sleep/resume: <200ms.
- Hover threshold → snap audio playing: **300ms total**.
- Search keystroke → results updated: **<100ms** across 5,000-game library.

### 16.3 Memory budget (kiosk shell + theme + render)

- Target: ≤500MB resident.
- Hard ceiling: 1GB → asset cache evicts aggressively.
- Theme can claim ~150MB.

### 16.4 Disk footprint

- Installer: <500MB for OA itself + ~100MB bundled cores → **~600MB fresh install**.
- Theme archive: target <50MB; "very heavy" tag in Index above that.

### 16.5 Asset prefetch policy

- Box art: 10 tiles ahead/behind in scroll direction in decoded memory.
- Snap video: 3 ahead/behind (expensive).
- Snap audio: 5 ahead/behind.
- Prefetch worker is low-priority thread; pauses during high-velocity scroll to free IO bandwidth.

### 16.6 Adaptive quality tiers (auto-detected at first launch, user-overridable)

- **"Everything On"** — Steam Deck-class and up (Zen 2 + RDNA2 + 16GB RAM).
- **"Polished"** — 2018-era mid-range (Intel UHD 620 / GTX 1050 / 8GB RAM).
- **"Lean"** — older integrated graphics; one snap video at a time, no shader effects on snap, simpler transitions.

We don't refuse to run below 2018-era — Lean catches even older. Honest about which polish ceiling each tier hits.

### 16.7 Pathologically slow cores

- "Loading [Game Title]…" overlay with progress when core init exceeds ceremony duration by >500ms. Not silent stall.

---

## 17. Robustness / appliance behavior

### 17.1 Three crash scopes

| Scope | Frequency | Treatment |
|---|---|---|
| **Asset / scrape failure** | Common, undramatic | Graceful degradation (snap missing → box art → generic platform tile). Logged WARN. Surfaced in desktop "Library Issues" panel for batch repair. No user-visible disruption in kiosk. |
| **Core crash** | Occasional | **Process isolation** — libretro core runs in child process. Memory-mapped frame buffers + IPC for input/audio. Recovery overlay: "Splatterhouse stopped responding. Return to library? [Yes] [Try again]." Auto-save every 30s means <1s of progress lost. |
| **Shell crash** | Rare | **Watchdog process** (`oa-watchdog.exe`) monitors heartbeat (1Hz IPC ping). Respawns shell in <2s. State journaled to disk; user returns to exactly where they were. Crash dump captured to `appData/crashes/<timestamp>/`. |

### 17.2 Watchdog notification

- **Mode-aware default**: silent restart in kiosk mode, notify in desktop mode. Same setting, default flips by mode. Settings → Diagnostics always shows crash history.

### 17.3 Controller hot-swap

- **In-game**: pause immediately, "Controller disconnected — reconnect to resume" overlay. Reconnect → auto-resume. After 60s without reconnection, auto-save and drop to kiosk.
- **In kiosk**: small banner, keep current focus/state. Reconnect → banner clears.

### 17.4 Long uptime safeguards

- Audio device re-validation, auto-rebind on disappearance.
- Display-wake hint on input, periodic "is this display alive" check.
- Hard ceilings on texture cache / log buffer / video decode pool. Periodic GC evicts cold entries.
- Log rotation: `oa-current.log` truncates each launch; long-running session also rotates at 50MB.

### 17.5 Crash-prone cores

- Community-curated incompatibility list (separate from theme Index, same federated model). "This core has known stability issues — recommended: [alternative]."

### 17.6 Telemetry stance

- **Zero telemetry by default.** No usage analytics, no opt-out checkboxes hidden in EULA, no fingerprinting. The "gift to the community" framing makes this load-bearing.
- **Opt-in crash report sharing**, fully transparent contents (no game library, no usernames, no paths beyond OA dirs). Primary path is "user zips folder and attaches to GitHub issue"; auto-send is convenience.
- **No background "check for updates" pings.** Updates checked when user opens updater, not on timer.

---

## 18. Rhai sandbox rules (non-negotiable)

If a theme can read `%USERPROFILE%\Documents\passwords.txt`, the federation model is broken. Hard rules:

- Filesystem scoped strictly to theme's own dir (read assets) + write-scoped scratchpad for runtime state.
- No network at all.
- No shelling out, no native FFI, no `eval`, no string-to-script.
- OS environment access limited to curated API (locale, DPI, theme version metadata — that's it).
- Security audit before community distribution opens.

---

## 19. Phased implementation

### Phase 0 — Polish current program (NOW)
- **Settings/IA polish** per `../../_archive/features/ui-polish/UI_MENU_BAR_PLAN.md`. Step-1 (trim placeholders) through Step-7 (Tools menu) are the critical pre-work.
- Get the desktop UI to a polished baseline; many of the surfaces stay in desktop mode forever.
- Mode switch foundation — even before kiosk is built, the shell architecture should anticipate a future mode switch (don't bake desktop-only assumptions into shared state).

### Phase 1 — MVP kiosk shell
- Single binary, `--kiosk` flag toggles chrome.
- Basic kiosk chrome: platforms wheel, game tile, basic search (letter-jump only).
- Simplified Showcase theme (no Theme Studio yet — manually authored).
- Moment of arrival v1: snap video on hover with 200ms threshold + audio fade.
- Launch ceremony v1: concurrent with core init.
- In-game menu v1: Resume / Save / Load / Exit.
- Auto-resume / silent auto-save.
- Controller binding wizard.
- First-launch onboarding (setup-first).
- Performance budgets enforced.

### Phase 2 — Theme substrate
- Four-layer model: Layout (TOML) + Animations + Behaviors (Rhai) + Shaders.
- Rhai sandbox with hard rules.
- Hot reload from external editors.
- Theme Studio in-engine editor.
- `.oatheme` archive format + signing.
- Three reference themes (Showcase, Minimalist, Cabinet).
- Per-system theming integration with reference themes.

### Phase 3 — Library depth + navigation
- Five built-in nav patterns + custom escape hatch.
- Predictive search overlay with virtual keyboard.
- Filter sheet.
- Playlists + smart playlists DSL with starter library.
- Named views with arbitrary hierarchies.
- Hardcoded rails (Recents/Favorites/Most-Played/Just-Added).
- Cross-platform playlist behaviors.
- Breadcrumbs.

### Phase 4 — Audio + multi-monitor + attract
- Five-bus audio mixer with ducking matrix.
- Theme-controllable audio curves.
- Multi-monitor surfaces in theme schema (main / marquee / control-panel).
- Marquee asset pipeline (scrape extension).
- Attract Tier 1 (snap cycle).
- Manual monitor-to-surface assignment with test pattern.

### Phase 5 — Kid mode + accessibility
- Single kid mode (locked-to-subset, PIN exit).
- Kids reference theme (the fourth).
- All seven accessibility surfaces.
- Accessibility wizard at first-launch + persistent settings.

### Phase 6 — Distribution + community
- Theme Index repo on GitHub.
- In-app theme browser (desktop full, kiosk simplified).
- Theme migration tool.
- Sandbox security audit.

### Phase 7 — Advanced (post-v1)
- Attract Tier 2 (pre-recorded MP4 background capture pass).
- Attract Tier 3 (live emulation via save-state rotation).
- Phone-companion search (the deferred-build seam).
- Multi-user profiles (if Phase 5 kid mode demand surfaces it).
- Single-switch accessibility mode.

### Cross-cutting (any phase)
- Robustness (watchdog, process isolation, telemetry stance).
- Performance budget enforcement.
- Adaptive quality tiers.

---

## 20. Prerequisites in current codebase

These connect the kiosk plan to today's code:

- **Engine crates are already standalone** — `oa-core`, `oa-render`, `oa-audio`, `oa-input`, `oa-libretro`, `oa-content`, `oa-savestate` don't care which shell sits on top. The architectural split the plan needs has already happened.
- **Per-system theming pillar already exists** — `frontend/src/platform/themes/systemPalettes.ts` (runtime `[data-system]` injection; `systems.css` was retired, D26) + `frontend/src/platform/themes/registry.ts`. Kiosk extends this rather than rebuilds it.
- **Save state infrastructure shipped** — Phase 1.5 + Phase 4 covers the in-game-menu and auto-resume needs.
- **Shader pipeline shipped** — `ShaderPreset` enum + `shaders/presets/*.preset.toml` + hot-reload + per-game/per-system overrides. Kiosk shaders slot into this.
- **Library scan + Import Wizard + watcher** — first-launch onboarding leans on this directly.
- **Media sync + scraper** — moment-of-arrival's video/marquee/fanart pipeline extends this.
- **Audio device picker shipped** — multi-bus mixer extends rather than replaces.
- **Multi-core CPU awareness shipped (2026-05-21)** — first-launch background scrape + attract Tier 2 capture both benefit directly.
- **Quick settings overlay shipped (slice 2.8.B)** — base for the in-game menu.

What's missing and needs to be built or planned:

- **Mode switch foundation** — single binary, `--kiosk` flag, shell chrome swap. Doesn't exist today.
- **Rhai sandbox + four-layer theme substrate** — entirely new subsystem.
- **`.oatheme` archive format + Theme Studio + Index repo** — new.
- **Watchdog process + process-isolated cores** — new (today cores run in-process).
- **Snap video playback in the shell** — partial (scraper pipeline exists; in-shell preview not wired).
- **Five-bus audio mixer** — today there's a single audio output; multi-bus + ducking is new.
- **Marquee / multi-surface rendering** — today there's two-window mode (Phase 2) but it's UI/emulator separation, not main+marquee. Different concept.

---

## 21. Open items (deliberately left for later)

- **Multi-user profiles** (Switch/PS5 model) — deferred to v2 if Phase 5's single kid mode surfaces demand.
- **Phone-companion search input** — deferred build, seam architected from the start.
- **Live-emulator attract (Tier 3)** — deferred to Phase 7; ship Tier 1 + Tier 2 first.
- **Schema version 2.0 migration tooling** — until we have schema version 2.0, the migration tool is hypothetical.
- **Cabinet build flag for kiosk-only binary** — deliberately *not* doing this; OS-level Windows kiosk account is the right answer.

---

## 22. Companion docs

- `../../_archive/features/ui-polish/UI_AUDIT.md` (2026-05-18) — today's UI surfaces. Informs what changes for kiosk.
- `../../_archive/features/ui-polish/UI_MENU_BAR_PLAN.md` (2026-05-18) — desktop-mode IA polish. Phase 0 of this plan.
- `docs/DECISIONS.md` — when implementation starts, append per-phase decisions here.
- `docs/PARKING_LOT.md` — kiosk-plan entry points here.
