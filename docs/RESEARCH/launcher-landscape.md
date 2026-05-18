# Launcher Landscape Research

**Date:** 2026-05-17
**Author:** Claude (synthesis of four parallel research agents)
**Status:** Planning document. No code yet. Source for prioritizing the next several months of OA work.

---

## How to read this doc

This is a synthesis, not a survey. Where two competitors disagreed, I picked a side and defended it. Where research conflicts with a load-bearing decision already captured in `docs/DECISIONS.md` or `docs/cores/tg16/DECISIONS.md`, that conflict is called out inline.

If you're skimming: read §1, then §3, then §8.

---

## 1. Executive summary

1. **The empty niche is real and load-bearing.** Commercial launchers (Steam, Heroic, GOG Galaxy) have the visual ceiling but poor scroll perf at scale. Emulator frontends (BigBox, ES-DE, Pegasus) have decent perf in narrow configurations but bad visual design. **Nobody owns "Heroic-tier polish + Steam-tier perf + per-system retro identity."** That's the OA wedge. Build that and we win.
2. **Performance is the highest-leverage product investment.** LaunchBox/BigBox users with 10K+ games report 5-10s scroll lag, 8-10 minute cold starts, and 1+ GB idle RAM ([troubleshooting guide](https://feedback.launchbox.gg/help/articles/9454889-troubleshooting-launchbox-and-big-box-performance), [large library thread](https://forums.launchbox-app.com/topic/29591-large-library-slow-performance/)). The dev publicly admits perf fixes are blocked on architectural risk. Steam itself ships a "Low Performance Mode" toggle because their library doesn't scale ([Steam Library Update](https://store.steampowered.com/libraryupdate)). **We can ship visibly smoother than any competitor through correct virtualization + Rust-side image pipeline + asset protocol delivery, on day one.**
3. **Visual ceiling is Heroic + Steam's tile hover.** Heroic for typography, spacing, and color discipline (MUI dark + custom palette + 8pt grid). Steam for the iconic 3D-tilt+scale+gradient hover and the Library Hero asset model (3840×1240 hero + 1280×720 logo + 600×900 capsule + 920×430 header). Per-game ambient theming derived from cover art is GOG Galaxy 2.0's idea; we can do it better in WGSL.
4. **The single biggest architectural decision is the theming model.** XAML themes (LaunchBox, Playnite) ship code-behind that can crash the host. QML themes (Pegasus) require theme authors to write a full UI app. CSS-only (Heroic) limits expressiveness. **Pick declarative-JSON-manifest + CSS variables + per-system override blocks + sandboxed WGSL fragment hooks for ambient effects.** This is a typed cousin of ES-DE's variants model. Code-behind never executes; bad themes fail validation at load with a line number.
5. **TanStack Virtual is the only credible 2D grid virtualization for Solid.** `@solid-primitives/virtual` does fixed-height vertical lists only ([Solid Primitives Virtual](https://primitives.solidjs.community/package/virtual/)). TanStack Virtual has a first-class Solid adapter and explicit 10K+ item demos ([TanStack docs](https://tanstack.com/virtual/latest/docs/introduction)). Heroic uses **no** virtualization — Steam Deck users with 200-game Heroic libraries hit 1GB RAM ([Heroic issue #1856](https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher/issues/1856)). We virtualize from day one and beat Heroic at any library size.
6. **ThumbHash beats BlurHash, LQIP, and color placeholders.** 13× faster decode than BlurHash; encodes aspect ratio + alpha at ~25 bytes per image; has a native Rust port ([ThumbHash homepage](https://evanw.github.io/thumbhash/), [decode benchmarks](https://github.com/swissspidy/media-experiments/issues/475)). Generate hashes at sync time on a Rayon thread pool, ship inline with metadata. **Zero per-tile cold fetch latency at scroll time.**
7. **Tauri asset protocol via `convertFileSrc` is the only correct local-file delivery path.** Already captured in `docs/DECISIONS.md` 2026-05-17 entry; this research independently arrives at the same conclusion. Base64-via-IPC explodes IPC bandwidth (Tauri Windows IPC hits ~5-10ms per call, ~200ms for 10MB on a community benchmark per [discussion #11915](https://github.com/tauri-apps/tauri/discussions/11915)). Custom URI schemes break in dev. Asset protocol gives WebView2 a normal HTTP-cacheable response and sidesteps IPC entirely.
8. **OpenEmu is the spiritual visual model for per-system identity. ES-DE is the technical model.** OpenEmu's "entering Saturn feels different from entering NES" via curated per-system tinted backgrounds is the design north star. ES-DE's `<variant>` + `<colorScheme>` + inheritance pattern is the implementation we should adapt (typed, not XML). Already partially aligned with our 2026-05-16 per-system-theming cascade decision.
9. **Cross-platform from day one is a real moat.** OpenEmu's Apple-Silicon-stall demonstrated the single-maintainer Mac-only bus factor. LaunchBox is Windows-only by structure. Mac and Linux retro communities have no premium frontend. **Tauri makes the second platform nearly free; we should ship Linux + macOS Day 1 of Phase 3.** This conflicts with `docs/DECISIONS.md` 2026-05-16 CI scope (excludes oa-shell on non-Windows). Worth revisiting on Phase 6+ but the strategic posture should be "cross-platform is a feature, not a future task."
10. **The single thing we must guarantee, above everything else, is that scrolling never stutters.** Every emulator-frontend Reddit thread has this complaint. Every demo on YouTube of a competing tool shows stutter. If OA scrolls 5,000 boxart tiles at 60fps with subsecond image fade-in, that single observable property will sell the product before any feature comparison runs.

---

## 2. Competitor table

| Tool | Target audience | Key strengths | Key weaknesses | Theming model | Performance | License | Verdict |
|---|---|---|---|---|---|---|---|
| **LaunchBox / BigBox** | Cabinet builders, large-library Windows retro fans | 30+ media categories, 3D box rendering, light-gun pre-configs, Premium ecosystem | XML-as-DB until 2025, RAM hog (1+ GB idle), 5-10s scroll lag, paid BigBox, Windows-only | XAML DLL themes (code-behind) | Bad at scale; SQLite migration 2025 helped | Proprietary, ~$30/yr | **Steal** media catalog, 3D box, light-gun support, save management. **Avoid** XAML themes, eager full-library load, sync image decode. |
| **RetroArch (XMB / Ozone / MaterialUI / RGUI)** | Power users who tolerate UX hostility | Engine ubiquity, one stack across desktop/console/handheld | Information architecture disaster, "Cores" jargon, XMB thumbnail crashes | Hardcoded C drivers + icon-pack swaps | Ozone fast, XMB synchronously decodes thumbs on nav thread | GPLv3 | **Steal** Ozone two-pane gamepad layout. **Avoid** synchronous thumbnail decode, jargon ("cores"), nested settings tunnels. |
| **ES-DE** | EmuDeck / RetroDECK / Batocera users wanting serious multi-system frontend | Per-system theming as first-class concept, ~150 systems preconfigured, in-app theme browser, XML+inheritance variants | RetroArch dependency, theme authoring is a multi-evening project, Android port lacks touch | XML v2: `<variant>`, `<colorScheme>`, `<aspectRatio>`, `<transition>` | Handles 10K+ libraries, video-heavy themes strain low-end | MIT | **Closest reference design.** Adapt the variants model with typed TOML schema. Build in-app theme browser. Pattern is exactly our pillar. |
| **Pegasus** | Theme tinkerers, Android boxes, cult favorites | Maximum expressiveness (theme = QtQuick app), cross-platform from one binary | Theme authoring requires QML+JS, no built-in scraper, small contributor pool | QML + JavaScript | GPU-accelerated on capable hardware | GPL-3.0 | **Reject** the "blank canvas" model — too much rope for authors, too much surface for us to maintain. **Steal** the exposed-data API shape (`collection.games.assets`). |
| **AttractMode / AttractMode Plus** | Arcade cabinet builders, BYOAC crowd | Boot-to-game speed (<5s), hardware-control friendly, HyperSpin asset compatibility | Squirrel scripting language is niche, default look austere, two forks split contributors | Squirrel scripts | Lean SFML, runs on Pi-class | GPL-3.0 | **Steal** boot-speed obsession, attract-mode idle-preview concept. Ignore scripting model. |
| **OpenEmu** | macOS users | Native macOS feel, three switchable views, zero-config controllers, auto-scrape on import | Stalled development (Intel-only through Apple Silicon transition), single-maintainer bus factor | None — native Cocoa | Smooth, Metal-backed | BSD-3 | **Visual benchmark for per-system identity.** Copy: source-list sidebar, tinted system pages, three view modes, auto-scrape. Lesson: cross-platform from day 1 mitigates bus factor. |
| **Playnite (emulator+general)** | Windows users wanting one launcher for everything | Unified library (Steam/GOG/Epic/emulator), CRC+serial ROM matching, rich plugin ecosystem, free Fullscreen mode | WPF VirtualizingStackPanel must be manually configured, dual Desktop+Fullscreen theme split, 2K+ games clunky | XAML (WPF) themes | OK with virtualization on; bad without | MIT | **Steal** CRC/serial matching, plugin architecture as model. **Avoid** dual-mode split (our Phase 2 single-window decision aligns). |
| **Steam (desktop)** | Everyone | Library Hero asset model, iconic tile hover, restrained palette letting art dominate, Discovery Queue | NOT actually smooth at 5K+ games (Low Performance Mode toggle is the smoking gun), 800KB placeholder images, slow CEF Chromium 85 baseline | CSS overrides on React, theme system removed in 2023 CEF rewrite | Mediocre at scale | Proprietary | **Visual ceiling for tile hover and asset spec.** Beat on perf with proper virtualization. |
| **Steam Big Picture / Deck UI** | Controller, 10-foot use | Gamescope microcompositor, color-tinted ambient backgrounds, focus ring + dim-others pattern | Same CEF base as desktop, 500ms-1s scroll latency on home, lags >720p | Same React/CEF, gamepad layout tree | Variable framerate 50-60fps in library | Proprietary | **Best controller-mode UX template.** Mirror: recent row + tabbed sub-content, color-tinted background, side drawer quick menu. |
| **Heroic Games Launcher** | Linux Epic/GOG/Amazon users wanting native Linux store | The visual ceiling we explicitly target in CLAUDE.md, MUI v5 dark + custom palette, cohesive sidebar, cards with title-below-image | NO virtualization, loads entire game list at startup, memory leaks documented, Electron | MUI CSS variables + community SCSS themes | Bad at scale — 200 games causes 1GB RAM on Deck | GPL-3.0 | **Bench against visually.** Beat on perf trivially via virtualization. Match typography, spacing, sidebar treatment. |
| **GOG Galaxy 2.0** | Multi-launcher aggregators | Per-game custom backgrounds, dominant-color ambient tinting, integrations plugin model | Electron bloat, ~15% CPU during sync, memory leaks across versions | Closed-source theme system, custom-cover plugin | Bad | Proprietary | **Steal** per-game custom backgrounds, dominant-color ambient theming, integrations metaphor. Avoid Electron. |
| **Epic Games Store** | (Mostly used grudgingly for exclusives) | (Free games once a week) | Backend round-trip on every click, 20s cold start, 2:30 full populate; VP publicly admits "the launcher sucks" ([VGC](https://www.videogameschronicle.com/news/epic-games-store-exec-admits-the-launcher-sucks-says-improvements-are-on-the-way/)) | Electron + skinned chrome | Catastrophic | Proprietary | **The cautionary tale.** Cache aggressively, never block UI on network. |
| **Itch.io app** | Itch.io users | Best-engineered launcher in the wild: Go daemon (Butler) for all I/O + thin Electron renderer | Tiny store-specific feature scope | Plain CSS + React | Lightning fast | MIT | **The architectural model.** Rust = Butler, Tauri WebView = thin renderer. Already aligned with our stack choice. |
| **Lutris** | Linux Wine/Proton + emulator users | Auto-installs runners on demand, community YAML install scripts | Controller setup tedious, not designed for couch | GTK + CSS | OK | GPL-3.0 | **Steal** the auto-download-runner-on-demand pattern for OA's `<exe_dir>/cores/` loader. Ignore the rest. |
| **RomM** | Self-hosters / homelab "Plex for ROMs" crowd | Beautiful web UI, multi-user, cloud save-state sync, browser EmulatorJS playback | Web-only, browser emulation ceiling ~SNES, DB issues with 10K+ libraries | Vue 3 + Vuetify | OK on small libraries, breaks at 10K+ | AGPL-3.0 | **Steal** the visual design language wholesale — best-looking emulator frontend in the world. |
| **HyperSpin** | (Legacy) | Invented per-system wheel + per-game scene pattern | Adobe Flash (dead), separate launch tooling (HyperLaunch/RocketLauncher) | Flash SWF + XML | Adequate when alive | Freeware (defunct) | **Spiritual ancestor.** Steal the philosophy (each system = a world). Never the tech. |
| **RocketLauncher** | (Legacy front-of-frontend) | Fade-to-loading screens, per-game bezels, unified pause overlay, per-game configs | Unmaintained, redundant with modern RetroArch | INI + AHK scripts | Adequate when alive | Freeware | **Steal the feature checklist** — bake natively into our launch sequence. |

---

## 3. Features to ship in OA (prioritized)

The goal is to **meet or exceed LaunchBox** while throwing out cruft. Tiers are ordered by leverage on the "visibly better than LaunchBox" demo, not by build complexity.

### MUST — table stakes to compete

| # | Feature | Inspired by | Implementation sketch | Conflicts? |
|---|---|---|---|---|
| M1 | **Virtualized library grid (2D)** | None of the emu frontends do this well; copy TanStack pattern | Solid wrapper around TanStack Virtual `createVirtualizer`. Compute `columnCount` from `ResizeObserver`, group items into row arrays, run one vertical virtualizer over rows. State snapshot/restore on route change copy from Virtuoso's `restoreStateFrom` pattern. ~300 LOC. | None |
| M2 | **ThumbHash placeholders, generated in Rust at sync time** | ThumbHash project | Add `thumbhash` Rust crate; during library sync, decode source → generate hash → store as bytes in SQLite. Frontend renders hash → ImageData on canvas, cross-fades to real `<img>` after `await img.decode()`. ~200 LOC Rust + ~100 LOC Solid component. | None |
| M3 | **WebP thumbnails at exact display sizes (300×400 @1x + 600×800 @2x)** | Steam pre-resized capsules | At sync time, `image` crate decodes source, resizes to two target sizes, encodes WebP, writes to `$APPDATA/media/<system>/<game>/{capsule.webp,capsule@2x.webp}`. Don't let WebView scale. | None |
| M4 | **Asset protocol delivery via `convertFileSrc`** | Tauri docs, our own 2026-05-17 decision | Already decided. Frontend calls `convertFileSrc(absPath)` to produce `http://asset.localhost/...` URLs. WebView caches as normal HTTP. | Aligns with `docs/DECISIONS.md` 2026-05-17 |
| M5 | **Library Hero / Logo / Capsule / Header asset spec** | Steam Library Update | Four-asset metadata schema per game: hero (3840×1240), logo (1280×720 transparent PNG), capsule (600×900), header (920×430). Source from SteamGridDB API + libretro-thumbnails. Store paths in SQLite. | None — extends our existing libretro-thumbnails work |
| M6 | **Per-system theming via cascade + CSS variables** | OpenEmu (visual) + ES-DE (technical) | Already decided. `data-system="tg16"` flips CSS-variable cascade. Extend to: per-system fonts, per-system marquee artwork, per-system ambient WGSL fragment hook. | Aligns with `docs/DECISIONS.md` 2026-05-16 |
| M7 | **Search-as-you-type via MiniSearch** | (Not in any emu frontend) | Build MiniSearch index at library-load. Solid `createMemo` returns filtered results, virtualizer rerenders. Sub-frame latency for 20K games. Fuzzy + prefix. ~50 LOC. | None |
| M8 | **Smart playlists with real expression engine** | LaunchBox (negative example) | Open feature request on LaunchBox for years: full AND/OR/NOT with nested groups, regex on title, datetime ops. Build a tiny expression parser in Rust, store as JSON in SQLite. UI is a chip-based query builder. | None |
| M9 | **Saved filter sets** | (No competitor has this well) | Any filter combination + sort + view-mode is named & pinnable as a sidebar entry. SQLite-stored. | None |
| M10 | **Auto-scrape on import, no separate "run scraper" step** | OpenEmu, Playnite | Add ROM → backend kicks off scraper pipeline → user sees fade-in metadata. Already partial via libretro-thumbnails + libretro-database. Generalize to "import event → enqueue scrape job → emit progress events." | Aligns with existing libretro-database work |
| M11 | **CRC + serial + DAT-aware ROM matching** | Playnite | At import, hash ROM, extract serial from CD ISO header, match against libretro-database DAT files. Beats filename matching by 10x accuracy. | Aligns with 2026-05-17 libretro-database decision |
| M12 | **Recently played row** | Steam | Time-sorted query on play-history table. Lazy-loaded row component. | None |
| M13 | **Game detail page with screenshots + save-state thumbs + play time + notes** | Steam + Heroic | Single route. Hero artwork at top with parallax, logo overlay, ambient color sampled from hero. Shelves below for screenshots, save states, related (same system / same series). | None |
| M14 | **Save state management UI + auto-backup on close** | LaunchBox (added 13.16) | Already on Phase 1.5; generalize backup retention slider, restore browser, RetroArch + standalone-emulator support. | Existing work |
| M15 | **Per-game core override (already-shipped per-system core picker extended)** | LaunchBox | UI on Game Detail page: "use core: [auto / X / Y / Z]". Stored in game row. | Listed as "Next" in `project_current_state` memory |
| M16 | **Zero-config controller setup** | OpenEmu | SDL2 gamepad DB at boot; if unknown controller detected, "press B to confirm" wizard. Test the Guide-button-refocus problem (Playnite ate it). | None |
| M17 | **Built-in scraper parallelism + retry + resume** | LaunchBox (negative) | LaunchBox's scraper is single-threaded with no resume. Ours uses `tokio::task::spawn` with semaphore for rate limits, persists `(game_id, source, status)` to SQLite so we resume after crash/restart. | Already partial in libretro-thumbnails work |

### SHOULD — premium differentiators

| # | Feature | Inspired by | Implementation sketch |
|---|---|---|---|
| S1 | **Dynamic ambient theming from cover art** | GOG Galaxy 2.0 + Steam | At import, extract dominant color via Rust port of Vibrant algorithm (`image` crate + median-cut quantize, ~50 LOC). Store as `(h, s, l)` in SQLite. On Game Detail page, tint the entire WebView via CSS variable; on system pages, blend the dominant color of the currently-focused game with the system accent. **The hook for the wgpu background to also tint comes free.** |
| S2 | **WGSL ambient background per system page** | (Nobody does this well) | Each system's `theme.toml` declares a WGSL fragment-shader hook. wgpu renders it behind the WebView in single-window mode. TG-16 = soft hex grid in lavender; Lynx = analog scanline grid; Vectrex = pure vector glow. Themers ship their own. ~30 LOC per shader. |
| S3 | **3D box rendering, beat LaunchBox's WPF Viewport3D** | LaunchBox | wgpu mesh + PBR materials + soft shadow + motion blur on rotate. Reuse existing 2D cover art as box face textures. Interactive shelf view: 5-7 boxes in 3D space, parallax scroll. Phase 4+. |
| S4 | **In-app theme creator with live preview** | LaunchBox's CTC is a *separate* Patreon tool | Owning the creator = owning the ecosystem. Live preview pane, view-tree inspector, asset picker, animation timeline, export as ZIP. Phase 5+. |
| S5 | **Federated metadata: libretro-database + ScreenScraper + IGDB + SteamGridDB** | (Nobody does this well) | Already have libretro-database; add ScreenScraper (with OAuth flow + rate limit), IGDB (for PC ports), SteamGridDB (for Library Hero assets). Aggregate at fetch time, dedupe by canonical ID. |
| S6 | **Steam-style tile hover** | Steam | `perspective: 1000px` container, `transform: rotateX(8deg) scale(1.03)` on hover, gradient overlay reveal, 250ms ease-out. Compositor-only. Already plan-of-record visually. |
| S7 | **"What to play next" row** | Steam Discovery Queue | Surface games unplayed in 90+ days OR games tagged "in progress" with no save state in 30+ days. Pure SQL query. Not a recommender — just rediscovery. |
| S8 | **Collections with custom hero artwork** | Steam Collections | User-defined groupings (manual or rule-based). Each collection picks a hero artwork or generates a montage. |
| S9 | **Activity feed / "What's new in your library"** | Steam | "You beat Bonk 30 mins ago"; "14 unfinished save states older than 6 months"; "new Beetle PCE Fast .dll available." Read-only timeline. |
| S10 | **Multi-monitor marquee window** | LaunchBox | Optional second wgpu Window outputting selected game's marquee art with crossfade. Tauri's two-window mode is already validated. |
| S11 | **Attract / screensaver mode** | LaunchBox + AttractMode | Idle → Ken Burns over key art with audio crossfades between snaps; "Now Playing" lower-third. wgpu does the motion. |
| S12 | **HTTP API for remotes** | LaunchBox's MarquesasServer plugin | Tauri Rust side exposes `GET /games`, `/selection`, `/media/<id>/<type>` plus optional auth'd POSTs. Enables Stream Deck, second-screen, mobile remote. |
| S13 | **Bezel art + shader presets per game (unified, replacing emulator-side overrides)** | RocketLauncher feature checklist | OA's wgpu pipeline owns the surface — we can route a bezel PNG as an overlay and apply a WGSL shader preset without touching the emulator's INI. Per-game, per-system, OA-wide tiers. Ship a small library of CRT presets. |
| S14 | **Auto-download libretro core on first system enable** | Lutris's auto-runner | User enables Genesis support → we fetch `genesis_plus_gx_libretro.dll` from buildbot.libretro.com (with consent prompt) → drop in `<exe_dir>/cores/`. Already aligned with the libretro pivot. |
| S15 | **Fade-to-loading-screen during emulator launch** | RocketLauncher | Pre-emulator black window with game's hero artwork while the core initializes. Hides the window-creation jank LaunchBox suffers from. |
| S16 | **In-emulator unified pause overlay** | RocketLauncher | One pause UI (game info, manual, save states) regardless of which core. Since we own the wgpu surface, we composite the overlay. |
| S17 | **Inline-editable custom fields** | LaunchBox (negative — 6 clicks per change) | Detail page right rail: click value → input → enter to save. Bulk-apply across selection. Keyboard shortcut per field. |
| S18 | **Plugin tier 1: declarative TS plugins, sandboxed** | LaunchBox C# plugins (negative) | Manifest declares capabilities (badge provider, metadata source, view widget). Runs in WebView sandbox. We expose typed APIs only. Phase 5+. |
| S19 | **Plugin tier 2: native Rust plugins via stable C ABI** | None | For performance work (custom shaders, custom audio FX). Phase 6+. |
| S20 | **DAT file support for ROM validation** | RomM (negative — they don't have it yet) | Match ROM hashes against No-Intro / Redump DATs. Surface match status on Game Detail (good dump / hack / corrupted). |

### COULD — nice-to-have polish

- C1. **RetroAchievements deep integration** (auto-login, badge filters, mastered/beaten/playtime cards). Mandatory parity for retro audience but not Phase 1 differentiator.
- C2. **Light-gun support** (Sinden, GUN4IR, AimTrak). LaunchBox's 600+ pre-baked profiles are a moat; we coexist for now.
- C3. **Custom backgrounds per game** (user upload override). Trivial once asset protocol is in.
- C4. **Manual / instruction card overlay** in pause UI. Source from libretro-database's `metadat/` manuals.
- C5. **Smart import preview** (Playnite's pattern — show CRC matches, let user confirm/reject before adding).
- C6. **Per-controller, per-profile mappings.** LaunchBox feature request for years.
- C7. **Cloud save sync (optional, self-hosted-friendly).** Don't want vendor lock-in; integrate with rclone targets or S3-compatible.
- C8. **Theme marketplace UI** (browse, preview, install community themes from a curated registry).
- C9. **Per-game shader live editor** (WGSL fragment + parameter sliders, write `.preset.toml`).
- C10. **OBS integration** for in-app recording. Truly niche; users can run OBS separately.

### WON'T — explicit anti-scope

- ❌ **Per-game scenes (HyperSpin-style).** Doesn't scale; community burnout. Each system = a room, not each game.
- ❌ **Browser-based emulation playback.** RomM owns this niche; web emulation is a different product.
- ❌ **Native social features.** No friends, no chat, no presence. Steam's own friend activity is shoved aside by most users.
- ❌ **Storefront integration (Steam/GOG/Epic import) for Phase 1-3.** Out of scope for a retro-first frontend. Maybe Phase 6+.
- ❌ **Built-in game DB curation.** Federation > re-curating. Use libretro-database + IGDB + ScreenScraper.
- ❌ **Theme code-behind (C#, Lua, QML, JS executing inside the host).** Declarative-only.
- ❌ **Multi-mode UI (Desktop + Fullscreen as separate theme trees).** One shell story; mode-switch is responsive CSS.
- ❌ **Native dual-binary (LaunchBox + BigBox).** One binary; single-window/two-window mode toggle in Settings.
- ❌ **Recommender algorithms / machine learning.** "What to play next" is a SQL query, not a model.

---

## 4. Design system principles

### Typography

Pick **Inter** for UI chrome (open, free, designed for screen reading at small sizes — used by GitHub, Mozilla, NPR), plus a display face for system pages and detail-page headlines. Recommend **Inter Display** (same family, optimized for >30px) for cohesion; alternatives like **Manrope** or a custom-fit Motiva-Sans-alike work but Inter Display avoids font-loading multiplication.

Size scale (8pt grid, modular):
- Captions: 12px / 16px line-height
- Body: 14px / 22px (desktop); 18px / 28px (10-foot)
- Buttons: 14px / 1 line
- H4 (card title): 16px / 22px
- H3 (section): 20px / 28px
- H2 (page): 28px / 36px
- H1 (display): 40px / 48px (desktop); 48px / 56px (10-foot)
- Hero: 64px / 72px (Game Detail name)

Weights: 400 (body), 500 (button + label), 600 (heading), 700 (hero only). No weight ≤300 (Heroic uses some light weights; legibility loss at small sizes in dark UIs).

Tracking: tighten display sizes (-0.01em at H1/H2, -0.02em at hero). Body stays at 0.

**Steam Deck floor:** 9px minimum at 1280×800 ([Steam Deck Verified compatibility](https://steamcommunity.com/discussions/forum/7/1291817208507967111/)). Our 12px caption is comfortably above.

### Spacing

Strict 8pt grid: 4 / 8 / 12 / 16 / 24 / 32 / 48 / 64 / 96. Use 4 only for inline element separation (icon + label, badge clusters). Card internal padding 16 desktop / 24 fullscreen. Grid gap 12 desktop / 20 fullscreen. Sidebar width 240 desktop / 320 fullscreen.

### Color

Near-black background (`#0E0E12` or `#0F1014`), NOT pure black. Pure black is an Itch-aesthetic move; it makes ambient color washes look muddy. Two surface elevations:

- `--surface-base`: `#0E0E12`
- `--surface-elevated`: `#16161D`
- `--surface-overlay`: `#1F1F28`

Text:
- `--text-primary`: `#F2F2F5` (off-white, not pure)
- `--text-secondary`: `#A4A4B0`
- `--text-tertiary`: `#6B6B7A`

System accent: per-system CSS variable `--color-system-accent`, neutral OA default. Already in `docs/DECISIONS.md` 2026-05-16.

Semantic:
- Success: `#3FB37F` (used sparingly — completed games, downloads finished)
- Warning: `#E6B450`
- Danger: `#E54B5C`

Reserve a single ambient accent derived from current game's cover art (`--color-ambient`). On Game Detail page, tints scroll bar, focus rings, secondary buttons. Never tints text or backgrounds (legibility).

References: [Steam brand colors](https://kulr.app/brands/steam) for restraint. Heroic's MUI dark palette for the surface elevation pattern.

### Motion system

Define globally as CSS variables (Riot Hextech pattern, [under-the-hood post](https://www.riotgames.com/en/news/under-hood-league-client%E2%80%99s-hextech-ui)):

```css
:root {
  --motion-instant: 80ms;
  --motion-fast: 150ms;
  --motion-medium: 250ms;
  --motion-slow: 400ms;
  --ease-out: cubic-bezier(0.16, 1, 0.3, 1);
  --ease-in-out: cubic-bezier(0.65, 0, 0.35, 1);
  --ease-snap: cubic-bezier(0.34, 1.56, 0.64, 1); /* slight overshoot for celebrations */
}
```

Use everywhere. Hover enter = `--motion-medium` `--ease-out`. Hover exit = `--motion-fast` `--ease-out` (snappier than entry). Modal open = `--motion-medium` `--ease-out`. Page transitions = `--motion-medium` `--ease-in-out`.

Never animate `box-shadow` ([Airbnb / Pinterest paint-perf docs](https://medium.com/airbnb-engineering/css-box-shadow-can-slow-down-scrolling-d8ea47ec6867)). Animate `opacity` + `transform` only. Static `filter: drop-shadow` on tiles, no hover-shadow-grow.

### Iconography

[Lucide icons](https://lucide.dev/) — open-licensed Feather successor, 1000+ icons, matches the launcher aesthetic. Strict 24px grid, 1.5-2px stroke.

### Density

**Two density modes**, not three. "Comfortable" (desktop default) and "Compact" (more rows visible, smaller titles). 10-foot mode is its own density tier, not a user-selectable comfort setting.

### Per-game ambient theming

When a game is focused (cursor on it in grid, or open on Detail page):
1. Sample dominant color from hero artwork on import → store as HSL.
2. CSS variable `--color-ambient` updates on selection change (debounced 150ms to avoid flash during scroll).
3. Game Detail: hero artwork backdrop ambient at 30% opacity over `--surface-base`, blur 40px.
4. Focus ring (controller mode) uses ambient hue with fixed S/L.
5. Optional WGSL background hook (Phase 4+) cross-fades to ambient hue.

GOG Galaxy 2.0 does this; we do it better via WGSL.

---

## 5. Performance architecture

The concrete techniques, with implementation paths and complexity ratings.

### Image pipeline

| Stage | Where | Technique | Why |
|---|---|---|---|
| Source ingest | Rust (Rayon) | `image` crate decodes JPEG/PNG/WebP source | Off-WebView decode |
| Resize | Rust (Rayon) | Lanczos3 or Catmull-Rom to 300×400 + 600×800 | WebView never scales |
| Re-encode | Rust (Rayon) | `webp` crate, quality 85 | Smallest decode at 300×400 |
| Hash | Rust (Rayon) | `thumbhash` crate, ~25 bytes per image | Instant placeholder |
| Persist | Rust → SQLite + filesystem | Hash bytes in SQLite, WebP files in `$APPDATA/media/.../` | Recovery + cache eviction |
| Serve | Tauri asset protocol | `convertFileSrc(absPath)` → `http://asset.localhost/...` | Browser-cacheable, no IPC |
| Placeholder render | Solid + canvas | `thumbHashToRGBA(bytes)` → blit to canvas behind `<img>` | 0.5ms decode, no visible lag |
| Image load | Solid Tile component | `<img src decoding="async" fetchpriority="high|low">` + `await img.decode()` | Off-main-thread decode, atomic swap |
| Cancellation | Solid onCleanup | `img.src = ''` aborts in-flight request | Free up bandwidth during fast scroll |

**Complexity:** M (Rust sync job ~400 LOC, Solid Tile ~150 LOC, asset protocol already configured).

### Virtualization

| Element | Choice | Why |
|---|---|---|
| Library | **TanStack Virtual** Solid adapter, 2D-grid via row grouping | Only credible Solid option; explicit 10K+ demos |
| Resize | `ResizeObserver` on container + reactive `columnCount` signal | Standard |
| Scroll-to-item | `virtualizer.scrollToIndex(i, {align: 'center'})` | Built-in |
| State snapshot | Custom: capture `scrollOffset` + visible range, restore on mount | Copy Virtuoso's `restoreStateFrom` pattern |
| Focus restoration | DIY: remember focused game ID, restore on return | TanStack doesn't ship this |
| Overscan | Default 5 rows | Tunable; matches industry default |

Combine with `content-visibility: auto` + `contain-intrinsic-size: auto 320px 420px` on every tile for free defense-in-depth.

**Complexity:** M (Solid wrapper around TanStack ~300 LOC + grid resize hook).

### Scroll-perf CSS rules

```css
.library-scroll {
  will-change: scroll-position;
  overflow-y: auto;
}

.tile {
  content-visibility: auto;
  contain-intrinsic-size: auto 320px 420px;
  contain: layout paint;
  /* static shadow, no animation */
  filter: drop-shadow(0 2px 6px rgba(0, 0, 0, 0.35));
  transition: transform var(--motion-medium) var(--ease-out);
}

.tile:hover {
  transform: perspective(1000px) rotateX(8deg) scale(1.03);
  /* don't add will-change here; let the GPU promotion happen on first hover */
}

.tile__overlay {
  opacity: 0;
  transition: opacity var(--motion-medium) var(--ease-out);
}

.tile:hover .tile__overlay {
  opacity: 1;
}
```

**Forbidden:**
- `box-shadow` transitions (paint killer at scroll, [Airbnb post](https://medium.com/airbnb-engineering/css-box-shadow-can-slow-down-scrolling-d8ea47ec6867))
- `filter: blur` on scrolling container (invalidates compositor)
- `backdrop-filter` on tile overlays (use a static semi-transparent gradient instead)
- `will-change: transform` on every tile (layer explosion, [will-change docs](https://developer.mozilla.org/en-US/docs/Web/CSS/will-change))
- Animated gradient backgrounds

**Complexity:** S (~50 LOC CSS + rAF scroll coalescing pattern).

### IPC pattern

| When | Pattern |
|---|---|
| Initial library load | One Tauri command returns serialized bytes (`bincode` or `bitcode`) for all games. Frontend deserializes in ~10ms. |
| Per-image | None — asset protocol bypasses IPC. |
| Search | None — MiniSearch in-memory in WebView. |
| Filter / sort | None — in-memory operation on the already-loaded games array. |
| Mutation (favorite toggle, custom field edit) | Single Tauri command, optimistic UI update, rollback on error. |
| Subscription (progress events during scrape) | Tauri event listener with throttled emits (10Hz max). |

**Avoid:** any per-tile IPC. Any large IPC payload over JSON. Pre-emptive base64.

**Complexity:** M (one binary serialization library choice + Rust serializer + JS deserializer; ~150 LOC).

### Startup sequence

1. **Tauri `setup()`** (synchronous, runs before WebView shows):
   - Open SQLite, query game count → if <100ms, load all into memory; else defer.
   - Spawn worker thread: verify boxart files exist, re-derive missing ThumbHashes.
2. **WebView shows** at ~400ms post-launch:
   - Inline-styled `<head>` with the OA palette (no FOUC).
   - Static `splash.html` shows logo + skeleton library shell (no spinner).
3. **Solid mounts** (~50ms after WebView):
   - Calls `get_initial_library()` Tauri command → bincode bytes → 20K games in ~10ms.
   - Builds MiniSearch index in background (~50ms for 20K).
   - First grid paint begins; placeholder ThumbHashes render immediately on visible tiles.
4. **Image loading streams in** at ~600ms:
   - Visible tiles fetch via asset protocol, decode async, swap in.
   - Off-screen tiles wait for IntersectionObserver.
5. **Library is fully interactive** at ~800ms cold, ~300ms warm.

**Budget:** First paint <500ms cold. Interactive <2s with 20K games. Beats Epic's 20s by 25×; matches Itch.

**Complexity:** M (~300 LOC across `setup()`, the Tauri command, the Solid splash component).

### Search responsiveness

- **MiniSearch** in-memory, index built once at library load (~50ms for 20K games).
- `createMemo` over `query` signal; filter results immediately on every keystroke.
- Virtualizer rerenders affected rows; only visible tiles paint.
- No debounce — search is sub-frame fast.
- Escalate to **Tantivy** in Rust only if library exceeds ~100K or full-text-over-descriptions becomes a requirement.

**Complexity:** S (~50 LOC integration).

### Memory budget

| Slice | Target |
|---|---|
| Idle (library loaded, no scrolling) | <200 MB |
| Active scroll | <500 MB |
| Game running (core + audio + UI) | <800 MB |
| Absolute ceiling | <1 GB |

Compare LaunchBox idle 500MB-1.5GB; Heroic 1GB at 200 games. We can dominate this by:
- Bounded LRU image cache (max 200 decoded tiles, ~100MB)
- No eager full-library media load
- Drop libretro core on user-unload (per `reference_libretro_mednafen_unload_then_load_no_gap` memory)
- Single-process Tauri vs multi-process Electron

---

## 6. Anti-patterns / things to never do

A consolidated frustration catalog from user complaints across emulator frontends. Every entry is sourced.

1. **Don't block scroll on image load.** The single most-cited LaunchBox/BigBox complaint ([scrolling lag thread](https://forums.launchbox-app.com/topic/70088-big-box-scrolling-lag/), [excruciatingly slow thread](https://forums.launchbox-app.com/topic/44298-bigbox-is-excruciatingly-slow/), [60K library thread](https://forums.launchbox-app.com/topic/35368-bigbox-performance-and-how-to-make-it-better/)). RetroArch XMB's "Thumbnail Delay" knob is a hack admission. Decode off-thread, ship placeholders synchronously.
2. **Don't crash on thumbnail load.** XMB has crash issues with thumbnails on multiple platforms ([RA #4745](https://github.com/libretro/RetroArch/issues/4745), [#17890](https://github.com/libretro/RetroArch/issues/17890)). Time-box decode operations, fail safely.
3. **Don't load the entire library at startup.** Heroic's #1 perf bug ([#1856](https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher/issues/1856)). Stream from Rust as user scrolls.
4. **Don't ship without virtualization.** Playnite needs WPF virtualization manually enabled ([WPF perf docs](https://learn.microsoft.com/en-us/dotnet/desktop/wpf/advanced/optimizing-performance-controls)). Heroic doesn't virtualize at all. Both fall over at 2-3K games.
5. **Don't keep media in RAM after navigation.** LaunchBox media is loaded into RAM and not released ([RAM thread](https://forums.launchbox-app.com/topic/61433-launchbox-using-way-too-much-ram/)). Bounded LRU.
6. **Don't ship XML/JSON-as-database.** LaunchBox only just migrated to SQLite in Feb 2025 ([13.19 beta thread](https://forums.launchbox-app.com/topic/89019-launchbox-1319-beta-thread/page/5/)). Our SQLite-from-day-1 already aligns.
7. **Don't network-on-every-click.** Epic's `It's really slow` admission ([Tom's Hardware](https://www.tomshardware.com/video-games/pc-gaming/epic-knows-its-launcher-sucks-and-is-fully-rebuilding-it-a-faster-more-stable-client-is-on-the-way-with-player-profiles-and-extended-social-features)). Cache aggressively, never block UI on network.
8. **Don't make scraping a separate ritual.** Pegasus has no scraper, users bounce to Skraper/ES-DE first. Auto-scrape on import (OpenEmu, Playnite).
9. **Don't use jargon.** RetroArch's "Cores" cost them a decade of new-user confusion ([#12995](https://github.com/libretro/RetroArch/issues/12995)). Call emulators "emulators." Call themes "themes."
10. **Don't ship two UI shells.** Playnite's Desktop+Fullscreen split doubles theme-author burden and creates focus-recovery bugs ([#2876](https://github.com/JosefNemec/Playnite/issues/2876)). Our single-shell Phase 2 decision aligns.
11. **Don't ship a beautiful empty.** AttractMode's default is austere; the "Zestful" community build is gorgeous. Default-out-of-the-box must showcase the system, not be a blank template.
12. **Don't depend on a dead format.** HyperSpin's Flash dependency killed it. Use formats viable in 10 years (PNG, MP4, WGSL, TOML, WebP).
13. **Don't make controller setup a chore.** Lutris's per-build controller flakiness ([#6146](https://github.com/lutris/lutris/issues/6146)). Ship SDL2 gamepad DB + press-buttons wizard.
14. **Don't lose window focus on emulator exit.** Playnite's bug ([#2876](https://github.com/JosefNemec/Playnite/issues/2876), [#4032](https://github.com/JosefNemec/Playnite/issues/4032)). We own the window stack — no excuses.
15. **Don't tie the binary to a dying runtime.** OpenEmu's Intel-only-through-Apple-Silicon-transition ([#5123](https://github.com/OpenEmu/OpenEmu/issues/5123)). Cross-platform from day 1; Rust + Tauri makes this nearly free.
16. **Don't ship code-behind themes.** LaunchBox's XAML DLL themes can crash the host. Sandboxed declarative themes only.
17. **Don't bundle Electron.** Heroic + GOG Galaxy memory leak streams ([Heroic #2627](https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher/issues/2627), [Heroic #4203](https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher/issues/4203)). Tauri WebView2 + Rust dominates this.
18. **Don't make custom fields require 6 clicks.** LaunchBox QuickFieldToggle plugin exists *because* the core UX is broken. Inline-editable, bulk-apply.
19. **Don't make smart playlists single-field-OR.** LaunchBox feature-request-open-for-years ([thread](https://forums.launchbox-app.com/topic/63026-we-can-use-and-when-creating-a-custom-autopopulated-playlist-query-but-can-we-use-or/)). Real expression engine.
20. **Don't ship a tool that requires a separate tool.** LaunchBox themes require CTC (separate Patreon-funded tool). RocketLauncher was front-of-frontend. Build a complete product.
21. **Don't `box-shadow` transitions on scroll.** Pinterest's "Gone in 60 Frames" case study + [Airbnb post](https://medium.com/airbnb-engineering/css-box-shadow-can-slow-down-scrolling-d8ea47ec6867). Static shadows; animate opacity of a pre-rendered shadow if needed.
22. **Don't `will-change: transform` on every tile.** Layer explosion → OOM on low-end devices ([MDN will-change](https://developer.mozilla.org/en-US/docs/Web/CSS/will-change)).
23. **Don't use the `data:` URL scheme for thousands of images.** Defeats HTTP cache, blows up IPC size. Asset protocol only.
24. **Don't synchronously read filesystem from the WebView.** Always Rust-side via Tauri command or asset protocol.
25. **Don't show a spinner during startup.** Splash should be skeleton chrome (sidebar populated, grid empty with shimmer). Heroic/Steam pattern.

---

## 7. Open questions

Research didn't conclusively point one way on these. Flagging for human review.

**Q1. Custom UI primitives vs Kobalte.**
Heroic uses Material UI. There's no MUI for Solid; the best headless library is [Kobalte](https://kobalte.dev/) (Radix-style accessible primitives). Question: do we adopt Kobalte for menus/dialogs/popovers/focus management, or build our own minimal a11y primitives layer? Kobalte buys accessibility for free; building our own keeps the dep tree smaller (we already have Solid + Tailwind v4 only). **My recommendation:** Kobalte. Accessibility is hard to retrofit, and our shell will need menus and dialogs eventually.

**Q2. Font choice — Inter vs custom display face.**
Inter is the safe, free, screen-tuned default. But Heroic uses MUI's font stack (Roboto-derived). Steam uses Motiva Sans. Per-system fonts (like ES-DE's Details theme with 57 system-specific fonts) is a possible future. Question: ship one default font family across all systems, or allow themes to specify per-system fonts? **My recommendation:** Inter as OA-default, allow themes to override per system. Don't ship a "custom display face" until a designer is in the loop.

**Q3. SteamGridDB API for hero/logo assets.**
The Steam Library Hero / Logo asset model is best-in-class. [SteamGridDB](https://www.steamgriddb.com/) ships these assets for retro games too. Question: do we hit SteamGridDB as a metadata source? It requires API key + has rate limits. **My recommendation:** Yes, but Phase 3+. Phase 1-2 can use libretro-thumbnails for boxart only.

**Q4. Cross-platform timing.**
`docs/DECISIONS.md` 2026-05-16 CI scope excludes `oa-shell` on non-Windows. Research strongly suggests cross-platform-from-day-1 is a strategic moat. Question: do we bring CI for Linux/macOS Tauri builds forward to Phase 3, or hold until Phase 6+? **My recommendation:** Phase 3 if validation cycle on Mac/Linux is cheap to add to your testing rhythm; otherwise hold but reserve the option. The Tauri crates already build cross-platform; CI is the gating cost.

**Q5. Binary serialization library for the initial library payload.**
`bincode` is the obvious default. `bitcode` is faster and smaller for similar use cases but newer. `postcard` is small-binary-optimized for embedded but works for desktop too. Question: pick one. **My recommendation:** `bincode` for now (most boring choice, broadest ecosystem). Reevaluate if perf is bound.

**Q6. WGSL ambient background priority.**
The S2 "WGSL ambient background per system" feature is gorgeous but expensive design work. Question: ship a single placeholder ambient effect per system (CSS gradient + subtle WGSL noise) in Phase 3, then iterate per-system over Phase 4-6? Or punt entirely to Phase 5+? **My recommendation:** placeholder per system in Phase 3 (CSS gradient with system accent), real WGSL fragments in Phase 5+. Per-system rooms need to *exist* visually before we polish them.

**Q7. ThumbHash vs CSS dominant-color placeholder.**
ThumbHash is gorgeous (you can almost recognize the game from the blur). A pure CSS background-color from the dominant color is cheaper and instantaneous. Question: do we ship both (CSS color as immediate, ThumbHash canvas as enhancement)? **My recommendation:** ThumbHash only. The 0.5ms decode is invisible; the visual upside is real; the implementation complexity is the same once Rust generation is wired up.

**Q8. Theme creator timing.**
LaunchBox's CTC is a separate Patreon-funded Windows app. We could ship an in-app theme creator early (low-fi: live preview + JSON edit) to attract theme authors before the audience exists. Or we could ship a great default theme + docs, build the creator in Phase 5+ once the audience is real. Question: which path? **My recommendation:** Docs-and-default-theme first. Creator in Phase 5+. Building the creator pre-audience is solving the wrong problem.

**Q9. Light gun support timing.**
LaunchBox's moat is 600+ pre-baked profiles. Replicating this is content work, not engineering. Question: skip entirely, ship a basic Sinden/GUN4IR routing in Phase 4, or fully match in Phase 6+? **My recommendation:** skip Phase 1-3. Phase 4 spike basic Sinden mouse-axis routing. Phase 6+ if cabinet-builder feedback warrants.

**Q10. RetroAchievements integration.**
Mandatory parity for retro audience. Question: how deep does Phase 1-2 need to go? Just auto-login passthrough to RA-aware cores, or full UI integration (badge filters, mastered/beaten cards)? **My recommendation:** Auto-login passthrough Phase 2; UI integration Phase 4.

---

## 8. Suggested next 3-5 sessions

Based on this research, in order, with reasoning.

### Session 1 — **Library virtualization + ThumbHash placeholder pipeline**

The single highest-leverage decision is correct virtualization + Rust-side placeholder generation. Demonstrates the "smoother than Steam" pitch in one observable artifact.

Tasks:
1. Add `thumbhash` Rust crate to a new `crates/oa-media` or extend existing media work.
2. In the metadata sync pipeline, generate ThumbHash for every game's primary boxart, store as bytes in SQLite.
3. Add Solid wrapper around TanStack Virtual; implement 2D grid via row-grouping pattern.
4. Build a Tile component that renders ThumbHash → canvas first, swaps in real `<img>` via asset protocol after `await img.decode()`.
5. Apply `content-visibility: auto` + `contain-intrinsic-size` to tiles.
6. Add `loading="lazy"` + `decoding="async"` + dynamic `fetchpriority` based on viewport intersection.
7. Confirm cold-start <2s with 1K games, profile to find the actual bottleneck.

**Why first:** This is the foundational UX promise. Until this is rock-solid, everything else is decoration.

**Estimated complexity:** M (~1000 LOC across Rust + Solid).

### Session 2 — **Game Detail page with hero artwork + ambient theming + per-game core override UI**

Now that scrolling demos beautifully, the Game Detail page is the second visible product surface. Combines several Must features in one route.

Tasks:
1. Game Detail route in Solid with hero parallax, logo overlay, ambient color sampled from hero.
2. Dominant color extraction in Rust at media-sync time (median-cut or similar, ~50 LOC).
3. Shelves: screenshots, save states, related (same system). All virtualized.
4. Per-game core override UI on the detail page (already on the Next list per the project_current_state memory).
5. Inline-editable custom fields (right-rail).

**Why second:** Game Detail is where users decide to play. The polish ceiling lives here. Also unlocks the per-game core override that's already on the "next" list.

**Estimated complexity:** L (~1500 LOC + Vibrant-port Rust crate).

### Session 3 — **Smart playlists + saved filter sets + search-as-you-type via MiniSearch**

Discovery features. Now that the library scrolls smoothly and Game Detail is polished, library navigation gets its glow-up.

Tasks:
1. MiniSearch integration; build index at library load.
2. Smart playlist expression engine in Rust (parser + evaluator). UI is a chip-based query builder.
3. Saved filter sets — any combination of filters + sort + view becomes a pinnable sidebar entry.
4. Solid `createMemo` over query+filters → virtualizer rerenders.

**Why third:** With M7 + M8 + M9 done, OA already has features LaunchBox has never shipped. The "exceed LaunchBox" pitch becomes demonstrable.

**Estimated complexity:** M (~800 LOC).

### Session 4 — **Per-system theming evolution: WGSL ambient backgrounds + theme manifest schema**

Now that the product is feature-competitive with LaunchBox on library UX, invest in the differentiator: per-system identity.

Tasks:
1. Define `theme.toml` schema with per-system overrides, asset folder layout, WGSL fragment hook points.
2. Wire system-page WGSL ambient: each system gets a fragment shader registered against the system page. TG-16 = hex grid in lavender; Lynx = analog scanline; Vectrex = vector glow.
3. Build in-app theme browser (list installed themes, switch active).
4. Schema-validate themes at load with helpful errors.

**Why fourth:** Per-system theming is the pillar that makes OA visually distinct from every competitor. Builds on the foundation laid in earlier sessions.

**Estimated complexity:** L (~1500 LOC across Rust + Solid + WGSL).

### Session 5 — **Performance pass + cross-platform CI bring-up**

Don't ship features faster than you measure performance. Now that the product is shipping recognizable surfaces, set up the performance discipline that distinguishes OA at 1.0.

Tasks:
1. Add a benchmark harness: cold-start time, time-to-first-paint, scroll FPS at 1K/5K/20K games, RAM idle / RAM under scroll.
2. Profile with Chrome DevTools (Edge DevTools — WebView2 is Edge), Tauri's built-in profiling, and a Rust-side allocation tracker.
3. Fix whatever the bottleneck actually is (probably image decode pool sizing, possibly bincode payload size, possibly excessive Solid re-renders).
4. Bring Linux + macOS CI online for `oa-shell` per Q4. Validates the cross-platform moat.

**Why fifth:** "Smoother than Steam" is the product promise. Reserve a session for measuring it. Cross-platform CI here unlocks the Linux/macOS audience without slowing Phase 4-6 feature work.

**Estimated complexity:** M (instrumentation + fixes + CI ~800 LOC + CI YAML).

---

## Conflicts with existing decisions

Called out inline above; collected here for one-glance review.

| Conflict | Existing decision | This research suggests | Resolution |
|---|---|---|---|
| Cross-platform timing | `docs/DECISIONS.md` 2026-05-16 CI excludes `oa-shell` on non-Windows | Linux/macOS Day 1 of Phase 3 — moat is real | Session 5 brings CI online; strategic posture flips earlier. |
| Tauri 2 + wgpu single-window | Two-window default, single-window opt-in per `docs/DECISIONS.md` 2026-05-16 | No conflict — both modes selectable per existing decision; this research reinforces the choice | None. |
| Per-system theming as cascade | `docs/DECISIONS.md` 2026-05-16 (CSS variables + `data-system` attribute) | Exactly what we should do; ES-DE pattern aligns | None. Extend with per-system fonts + WGSL hooks. |
| Tauri asset protocol | `docs/DECISIONS.md` 2026-05-17 | Same conclusion independently | None. |
| Libretro dynamic loading | `docs/DECISIONS.md` 2026-05-16 architecture pivot | Same; Lutris's auto-runner-on-demand pattern is the next move | None — already aligned. |
| libretro-database for metadata | `docs/cores/tg16/DECISIONS.md` 2026-05-17 | Same; federation with IGDB / SteamGridDB / ScreenScraper as Phase 3+ | None — extends existing path. |
| No per-core ARCHITECTURE.md | `docs/DECISIONS.md` 2026-05-15 | No conflict — this is launcher UX research, not chip docs | None. |

No silent overrides. Where research suggested adjusting timing (cross-platform), it's called out.

---

## Sources

All inline. Reproduced here for convenient grep:

### LaunchBox / BigBox
- [LaunchBox 13.19 SQLite migration](https://forums.launchbox-app.com/topic/89019-launchbox-1319-beta-thread/page/5/)
- [Troubleshooting performance](https://feedback.launchbox.gg/help/articles/9454889-troubleshooting-launchbox-and-big-box-performance)
- [BigBox scrolling lag](https://forums.launchbox-app.com/topic/70088-big-box-scrolling-lag/)
- [BigBox excruciatingly slow](https://forums.launchbox-app.com/topic/44298-bigbox-is-excruciatingly-slow/)
- [60K library scroll](https://forums.launchbox-app.com/topic/35368-bigbox-performance-and-how-to-make-it-better/)
- [RAM thread](https://forums.launchbox-app.com/topic/61433-launchbox-using-way-too-much-ram/)
- [Smart playlist AND/OR](https://forums.launchbox-app.com/topic/63026-we-can-use-and-when-creating-a-custom-autopopulated-playlist-query-but-can-we-use-or/)
- [Media file naming](https://forums.launchbox-app.com/topic/33272-fyi-media-file-naming/)
- [BigBox Views docs](https://feedback.launchbox.gg/help/articles/9450321-big-box-views)
- [3D Box Models](https://launchbox.featurebase.app/en/help/articles/6846068-3d-box-models)
- [Plugin API](https://pluginapi.launchbox-app.com/)
- [Crash thread 13.23](https://forums.launchbox-app.com/topic/91598-big-box-crashing-in-version-1323-here%E2%80%99s-what-you-need-to-know)

### RetroArch + ES-DE + Pegasus + AttractMode
- [Libretro GUI docs](https://docs.libretro.com/guides/gui/)
- [Ozone default announcement](https://www.libretro.com/index.php/retroarch-ozone-becomes-the-default-menu-ui-plus-touchscreen-and-scaling-updates/)
- [RetroArch UX meta-issue](https://github.com/libretro/RetroArch/issues/12995)
- [XMB thumbnail crash #4745](https://github.com/libretro/RetroArch/issues/4745)
- [XMB thumbnail crash #17890](https://github.com/libretro/RetroArch/issues/17890)
- [ES-DE](https://es-de.org/)
- [ES-DE THEMES-DEV](https://gitlab.com/es-de/emulationstation-de/-/blob/master/THEMES-DEV.md)
- [Pegasus theme overview](https://pegasus-frontend.org/docs/themes/overview/)
- [AttractMode Plus](https://github.com/oomek/attractplus)

### OpenEmu / Playnite / Lutris / RomM
- [OpenEmu](https://openemu.org/)
- [OpenEmu Apple Silicon #5123](https://github.com/OpenEmu/OpenEmu/issues/5123)
- [Playnite emulation docs](https://api.playnite.link/docs/manual/features/emulationSupport/emulationSupportOverview.html)
- [Playnite Fullscreen docs](https://api.playnite.link/docs/manual/gettingStarted/playniteFullscreenMode.html)
- [WPF perf docs](https://learn.microsoft.com/en-us/dotnet/desktop/wpf/advanced/optimizing-performance-controls)
- [Lutris controller setup #6146](https://github.com/lutris/lutris/issues/6146)
- [RomM](https://romm.app/)
- [RomM HN thread](https://news.ycombinator.com/item?id=44247964)

### Steam / Heroic / GOG Galaxy / Epic / Itch
- [Steam Library Update](https://store.steampowered.com/libraryupdate)
- [Steamworks Library Assets](https://partner.steamgames.com/doc/store/assets/libraryassets)
- [Steam CEF migration](https://steamcommunity.com/groups/SteamClientBeta/discussions/0/3365901765276206401/)
- [Steam library laggy thread](https://steamcommunity.com/discussions/forum/0/1644304412662820556/)
- [Steam library issue #6500](https://github.com/ValveSoftware/steam-for-linux/issues/6500)
- [Heroic repo](https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher)
- [Heroic 2.4 redesign](https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher/discussions/1478)
- [Heroic frontend perf #1856](https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher/issues/1856)
- [GOG Galaxy Electron HN](https://news.ycombinator.com/item?id=26752352)
- [Epic launcher sucks — VGC](https://www.videogameschronicle.com/news/epic-games-store-exec-admits-the-launcher-sucks-says-improvements-are-on-the-way/)
- [Epic launcher rebuild — Tom's Hardware](https://www.tomshardware.com/video-games/pc-gaming/epic-knows-its-launcher-sucks-and-is-fully-rebuilding-it-a-faster-more-stable-client-is-on-the-way-with-player-profiles-and-extended-social-features)
- [Itch app rewrites](https://fasterthanli.me/articles/itch-app-rewrites)
- [Riot Hextech UI](https://www.riotgames.com/en/news/under-hood-league-client%E2%80%99s-hextech-ui)

### Performance & web platform
- [TanStack Virtual docs](https://tanstack.com/virtual/latest/docs/introduction)
- [Solid Primitives Virtual](https://primitives.solidjs.community/package/virtual/)
- [VirtuosoGrid API](https://virtuoso.dev/react-virtuoso/api-reference/virtuoso-grid/)
- [ThumbHash homepage](https://evanw.github.io/thumbhash/)
- [ThumbHash vs BlurHash benchmarks](https://github.com/swissspidy/media-experiments/issues/475)
- [web.dev: lazy image loading](https://web.dev/articles/browser-level-image-lazy-loading)
- [web.dev: fetch priority](https://web.dev/articles/fetch-priority)
- [web.dev: content-visibility](https://web.dev/articles/content-visibility)
- [web.dev: high-perf animations](https://web.dev/articles/animations-guide)
- [MDN: HTMLImageElement.decoding](https://developer.mozilla.org/en-US/docs/Web/API/HTMLImageElement/decoding)
- [MDN: HTMLImageElement.decode](https://developer.mozilla.org/en-US/docs/Web/API/HTMLImageElement/decode)
- [MDN: contain CSS](https://developer.mozilla.org/en-US/docs/Web/CSS/contain)
- [MDN: will-change](https://developer.mozilla.org/en-US/docs/Web/CSS/will-change)
- [Tauri 2 asset protocol](https://v2.tauri.app/security/asset-protocol/)
- [Tauri 2 splashscreen](https://v2.tauri.app/learn/splashscreen/)
- [Tauri discussion #7145 (display image)](https://github.com/tauri-apps/tauri/discussions/7145)
- [Tauri discussion #11915 (IPC perf)](https://github.com/tauri-apps/tauri/discussions/11915)
- [WebView2 perf best practices](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/performance)
- [Airbnb: box-shadow scroll perf](https://medium.com/airbnb-engineering/css-box-shadow-can-slow-down-scrolling-d8ea47ec6867)
- [Pinterest paint case study](https://www.smashingmagazine.com/2013/06/pinterest-paint-performance-case-study/)
- [MiniSearch blog post](https://lucaongaro.eu/blog/2019/01/30/minisearch-client-side-fulltext-search-engine.html)
- [Tantivy GitHub](https://github.com/quickwit-oss/tantivy)
- [Kobalte (Solid headless)](https://kobalte.dev/)
- [Lucide icons](https://lucide.dev/)
