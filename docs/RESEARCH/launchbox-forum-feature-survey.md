# LaunchBox Forum Feature Survey

**Date:** 2026-05-21
**Source:** https://forums.launchbox-app.com/forum/63-features/ (top ~70 threads across pages 1–3, sorted by views).
**Scope:** Forum-driven feature ideas worth considering for OA that are **not already** shipped, in `docs/NEXT.md`, in `docs/PARKING_LOT.md`, or covered by the `MUST` / `SHOULD` / `COULD` tiers of `docs/RESEARCH/launcher-landscape.md`.

Companion to `launcher-landscape.md`. That doc derives features from competitor analysis (what their products do); this doc derives features from competitor users' unfilled requests (what their products **don't** do but users keep asking for). The two together cover both ends of the gap.

Items are tagged with rough fit:
- **★** = strong fit, premium-shell flavour matches OA pillars.
- **○** = decent fit, worth queueing once HIGH backlog clears.
- **△** = borderline; could go either way depending on operator interest.

---

## Items not yet captured anywhere in OA docs

### Launch & in-game UX

- **★ Pre-launch controller-mapping splash card.**
  A short, skip-with-any-key card shown while the core is initialising, displaying: (a) the original system's labelled controller diagram, and (b) the user's current key/gamepad bindings for that system. Lifted from the "Show Controller Mappings Before Launching Game" thread (137 replies, 63.9k views — one of the highest-engagement feature threads on the forum) and the related "Gearbox: community input on designing a game controls viewer." Sits cleanly under the existing `fade-to-loading-screen` slot (`launcher-landscape.md` §S15). Per-system controller diagrams are an art-pass dependency — see `docs/PARKING_LOT.md` 2026-05-19 region/publisher logo entry for the same per-system asset pipeline.

- **★ Random-game / "wheel spin" launcher.**
  Single hotkey + visible UI affordance to launch a random game from the *currently filtered* set (filter chips, system page, playlist, favourites). LaunchBox forum thread "BigBox - Select Random Game Feature Fixed by Wheel Spin" (167 replies, 46.2k views). Pure SQL with an `ORDER BY RANDOM() LIMIT 1` against the current filter predicate. Visible UI flourish ("wheel" animation or marquee cycle) elevates this from gimmick to discovery feature. Pairs naturally with M12 "Recently played row" — it's the inverse of "what to play next."

- **★ Picture-in-picture manual viewer while playing.**
  C4 in `launcher-landscape.md` covers a pause-menu manual viewer. The forum thread "Read a pdf file (or other format) while playing" extends this — users want the manual visible **alongside** active gameplay, not as a pause-blocking overlay. Translucent floating window or side panel reading from libretro-database's `metadat/` PDFs. Since OA owns the wgpu surface, composite the PDF render into a corner without pausing the core. Phase 4+.

- **★ In-game pause overlay quick-jump to manual + cheats.**
  Extension of S16 (unified pause overlay). Forum threads consistently surface "I want the manual + the cheats + the controls all in one place." Currently planned individually; explicit UX of "Pause → tabs along the top: Save States | Manual | Cheats | Controls | Settings" is the asked-for shape.

### Library & metadata

- **★ Tags as first-class user metadata.**
  Forum thread "Tags support, import from Steam." Distinct from playlists (membership lists) and genres (scraper-derived). User-assignable, multi-value per game, filterable as AND/OR/NOT chips in the search bar, persistable as saved filter sets (M9). Storage: `game_tags(game_id, tag)` join table. UI: chip-style multi-select on Game Detail right rail. Surfaces "Best Co-Op", "Christmas-themed", "Won a Game-of-the-Year award", and other axes the scraper can't infer.

- **★ Sort / filter by screen orientation (TATE vs YOKO).**
  Forum thread "Arrange game by Screen type?" — important for arcade collections (MAME, Neo Geo) where vertical-shooter cabinets (e.g. DonPachi, Raiden) sit awkwardly in horizontal grids. Needs an `orientation` column inferred from MAME's listxml `<display rotate="…">` attribute on import. Surfaces in the filter chips: `Orientation: Vertical | Horizontal`. Pairs with eventual per-game aspect override + screen-rotation render path.

- **○ ROM-filename vs database-title display preference.**
  Forum thread "Is there a way to display the rom name instead of the DB title?" Some collectors want to see the verifiable No-Intro/Redump filename (`Sonic the Hedgehog 2 (USA, Europe).bin`) instead of the cleaned title (`Sonic the Hedgehog 2`). Per-user toggle in Settings, or per-system override for serious archivists. One-line CSS / data-binding flip; trivial to ship.

- **○ Region-preferred boxart selection.**
  Forum thread "Is there a region preference box art selection?" Multi-region games (e.g. Castlevania released as `Akumajou Dracula` in JP, `Castlevania` in US, `Vampire Killer` in EU with different cover art) should resolve to the user's region-preferred scan. Chains with the existing region priority list (`SystemSettings.region_priority`) so the user's "USA → Europe → Japan" priority drives both ROM selection **and** cover selection. Multi-repo media sync (gb DMG+CGB precedent) already supplies the asset side.

- **★ Game-as-container hierarchy (master + child versions).**
  Forum thread "Is it possible to get LaunchBox to combine duplicate games?" and "Games as containers for subfolders." OA's existing 2026-05-19 multi-region grouping work (Versions submenu) covers the regional-variants case. The forum ask extends this to **all** version relationships — ROM hacks, fan translations, prototypes, romhack-of-romhack chains. Master tile shows the canonical cover; expanding reveals children with badges (`[Hack]`, `[Translation FR]`, `[Proto v0.3]`). Per-child overrides cascade off the master. The schema is already half-there (`master_id` column on games, NULL for master rows). Phase 3+.

- **○ Notes field per game.**
  Forum thread "LaunchBoxAnnotator Tool" (92 replies, 35.5k views — a third-party tool sprang up to fill the gap, which is the strongest "feature is missing" signal there is). Free-text multi-line notes attached to each game, visible on the Game Detail right rail, searchable. Distinct from the S17 inline-editable custom fields — those are short single-line values; this is a journaling surface ("got stuck on the boss in level 5, try magic attack pattern next time"). One `notes TEXT` column on `games`, one Solid textarea, debounced save.

- **○ Hide platform / playlist toggle.**
  Forum threads "Suggestion: Hide a platform/playlist" and "Option to hide an additional application in BigBox." Per-user visibility flag that hides a system/playlist from the sidebar without deleting it or unscanning its games. Useful for shared installs ("the kids' household profile shouldn't see the arcade cabinet shoot-em-ups") and for archive-but-don't-delete workflows. `hidden BOOLEAN` on systems + playlists tables; sidebar query filters by `hidden=0`.

### Per-system polish

- **★ Per-system enter/exit audio cues.**
  Forum thread "Per System Sounds on Enter/Exit" (16 replies, 2.8k views). Distinctive short audio sting played when navigating into a system home — TG-16 boot chime entering the PCE page, Sega-screams entering Genesis, etc. Strong fit because it dovetails with OA's per-system theming pillar — *each system is a room* extends naturally to *each room has an arrival sound*. Asset folder under `themes/<system>/audio/{enter,exit}.{ogg,flac}`, ~10 LOC Web Audio API hook on route change. Volume tied to the new menu-audio bus (below). Mutable globally + per-system.

- **★ Per-bus audio volume mixer (UI sounds / preview videos / in-game / system stings).**
  Forum thread "Separate audio volumes for videos when viewed in menus versus viewed in fullscreen?" OA today has a single device picker and a single global mute. Premium frontends mix at least four buses independently:
    1. **Core audio** (the game itself, full fidelity)
    2. **Preview videos in tile grids** (typically -12 to -18 dB by default — they're decoration, not focus)
    3. **Preview videos in fullscreen detail page** (closer to 0 dB)
    4. **UI sounds** (system enter/exit stings, tile hover clicks if any)
  Web Audio API `GainNode` per bus, single sliders panel under Settings → Audio. Persisted globally; per-system overrides for the stings.

### Workflow & power-user

- **★ Customisable global hotkeys (UI binding editor).**
  Forum thread "Request.... Change hotkeys." OA today has a hard-coded list (`HelpDialogs.tsx` SHORTCUTS array — Ctrl+G, F11, etc.). The forum ask is a Settings → Hotkeys page where every shortcut is rebindable, including Quick Save, Quick Load, slot cycling, fullscreen toggle, Game Focus, pause overlay, and the new random-game-spin hotkey. Storage: `hotkeys` JSON map in `Settings`. Per-system overrides (already feasible via three-tier cascade) cover the "MAME wants Ctrl+S as Save State, but my MAME ROMs use Ctrl+S in-game" case.

- **★ Bulk-edit wizard for library rows.**
  Forum thread "Feature Request: Bulk Edit Wizard - Retroarch Core Selector" and S17's "bulk-apply across selection" line. The explicit *wizard* UX (vs. multi-select + right-click) is what users keep asking for: step 1 select rows by filter, step 2 pick fields to change (core / shader / region preference / tag / aspect / overscan), step 3 preview the diff, step 4 commit with undo. Sits in front of the existing per-game settings stack — every field already exists, this is just a batch-apply UI on top.

- **○ Import / scan rule editor (allow / deny patterns).**
  Forum thread "Automatic Import Restrictions." OA's existing `folder_rules` JSON column on the folders table already covers the data; the ask is a UI for it. Allow + deny glob patterns ("`**/proto/**` skip", "`*.bak` skip", "files < 4 KB skip"), per-folder. Surfaces inside the existing Library Folders page when a folder row is expanded.

- **○ Display ROM-set canonical name alongside scraped title for MAME.**
  Forum context: MAME users live half in romset-name space (`sf2ce`) and half in title space ("Street Fighter II' - Champion Edition"). Both should be searchable and both should be visible on the tile. Already in `docs/NEXT.md` DOC / DATA / TRIAGE section as "MAME ROM-set name resolution"; forum thread "Is there a way to display the rom name instead of the DB title?" generalises the demand beyond MAME.

### Integrations & extensibility

- **○ Multi-disc resume-on-last-inserted.**
  Forum thread "Enable select disc for last played game." For multi-disc PSX / Saturn / Sega CD titles (FFVII three discs, Lunar two discs, Policenauts, etc.), default the next launch of that game to the disc the user last had in. Today OA presumably starts every multi-disc launch on Disc 1. Trivial: add a `last_disc_index` column on the games row, prefill the `.m3u` index on next launch. Operator can override via the disc-swap quick menu.

- **△ MAME high-score persistence + leaderboard surface.**
  Forum thread "New MAME High Score Feature" (117 replies, 33.2k views). MAME's upstream `hiscore.dat` system persists arcade high scores. UI surface: Game Detail page shows top-5 high scores with date stamps, system-page leaderboard column ("you've topped the Pac-Man cab 4 times this month"). Out-of-scope-feeling for non-MAME systems; in-scope-feeling once MAME is a real first-class citizen and OA is competing for cabinet builders.

- **△ HTTP API for external launchers (Kodi addon, Stream Deck, mobile remote).**
  Forum threads "Kodi Addon for Launching Big Box" (132 replies, 61k views) and the various Stream-Deck-style asks. `launcher-landscape.md` §S12 captures this; the forum data is *evidence of demand* rather than a new idea. Worth surfacing because the cumulative reply count makes it the second-most-discussed integration thread.

- **△ Native multi-monitor marquee window.**
  Forum threads "HyperMarquee Support" (72 replies, 26.7k views) and various pincab/marquee threads. Covered as S10 in `launcher-landscape.md`. Mentioning here because the forum volume is higher than I expected — cabinet builders are the most vocal LaunchBox cohort and would migrate to OA wholesale if the marquee window shipped early.

### Operational

- **★ Translation / i18n infrastructure.**
  Forum thread "Translating LaunchBox" — 1.6k replies, 490.6k views, by far the highest-engagement thread on the entire forum. Community-driven string catalogues are *the* lever for global retro-community adoption. Stack: `i18next` or `solid-i18n`; string keys in TS files; PR-based locale contributions in `frontend/src/locales/<lang>.json`. Ship English + Japanese + Spanish + Brazilian Portuguese on day-1 of the i18n migration since those communities self-identified as wanting in.

- **○ Auto-import & catalogue-update from buildbot for cores.**
  Already in `launcher-landscape.md` §S14, but the forum thread "Launchbox auto installs RetroArch cores" reveals a sub-feature: **silent-update channel** for cores the user has already opted into. Mirror RetroArch's "download core info files in background" + "notify when a newer build of an enabled core is available." Sits on top of the existing core installer + buildbot catalog UI.

---

## Items already covered (forum confirms demand; OA already plans them)

For PR-author convenience, here's the cross-reference. If a forum thread maps directly to an existing tracked item, that's recorded here rather than spawning duplicate work.

| Forum thread | Already tracked |
|---|---|
| Game manual viewing | `launcher-landscape.md` §C4 |
| Achievement icon in Roms group | §C1 |
| Possible to re-theme attract mode? | §S11 |
| BigBox screensaver - unmute video? | §S11 |
| Improved Controller Profiles and Auto-Switching | Per-system bindings cascade (shipped) + Q9 |
| Cloud Sync for Metadata and Playlists | §C7 |
| Playlist auto-populate 'And' 'Or' support | §M8 |
| Macbook M1 Support | Q4 cross-platform timing |
| Show controller mappings before launching game | This doc (new) + §S15 fade-to-loading-screen slot |
| Game Controls viewer? / Gearbox | This doc (new) |
| Platform Image Collection | `docs/PARKING_LOT.md` 2026-05-19 region/publisher logos |
| Lightspeed MAME Importer | §M11 (DAT-aware matching), §M10 (auto-scrape on import) |
| New LaunchBox Platform Image Collection | `docs/PARKING_LOT.md` 2026-05-19 |
| Kiosk Mode? Immutable? | `docs/PARKING_LOT.md` 2026-05-20 kiosk profile + kiosk flags |
| Vertical orientation for pincab | `docs/PARKING_LOT.md` 2026-05-21 kiosk/cabinet flags |
| RocketLauncher (feature checklist) | §S13 / §S15 / §S16 |
| Exit Games and Emulators | Shipped (`drop core on user-unload` pattern) |

---

## Explicit non-scope (forum-driven but rejected for OA)

These came up on the forum and are listed here so future audits don't re-add them.

- **ExoDOS / DOS games platform.** Out of scope per OA's "console-first" framing. DOSBox-libretro could still ship as a core, but ExoDOS's curated collection is a different product.
- **TeknoParrot auto-import.** Arcade PC platform — out of OA's "overlooked consoles" focus. Re-evaluate if OA picks up an arcade-PC cohort.
- **Non-installed Steam/GOG/Origin game import.** Explicit `launcher-landscape.md` "WON'T" item.
- **DVD player platform category.** Out of scope; OA is not a media center.
- **LEDBlinky cabinet hardware control.** Re-evaluate only if the cabinet-builder cohort actively migrates to OA. Probably better as a community plugin (Phase 5+ S18 declarative TS plugins) than core feature.
- **JoyToKey command line integration.** OA's input layer is native and key-binding-aware; JoyToKey as middleware shouldn't be necessary.
- **Daemon Tools / virtual drive mounting.** OA reads BIN/CUE/CHD/M3U natively. Virtual drive layer is redundant.
- **Per-game system requirements (PC).** N/A for console emulation.
- **Browser-based playback (RomM-style web emulation).** Explicit `launcher-landscape.md` "WON'T" item.

---

## Recommended next bites (initial pass)

If the operator wants to chase this list, suggested ordering by leverage:

1. **Pre-launch controller-mapping splash card** — high visible polish for low effort once the per-system controller asset gap is closed; pairs with the parking-lot region/publisher logo art pass.
2. **Random-game wheel-spin** — one SQL query, one hotkey, one animation. Hours of work, instantly demo-able.
3. **Per-bus audio mixer + per-system enter/exit stings** — two features that share the audio-routing refactor; ship together.
4. **Customisable global hotkeys UI** — directly unblocks the long tail of "but I want Ctrl+S for something else" forum complaints; trivial in volume but high in user-felt control.
5. **Tags as first-class metadata** — biggest library-UX delta against LaunchBox per actual user value; chained nicely with the existing smart-playlist (§M8) work.
6. **i18n infrastructure** — strategic, not tactical. Open the door for the global retro community to contribute before the audience is fully formed.

---

# Deeper survey — pages 4–30

Second pass picked up another ~60 distinct asks across forum pages 4–30. Grouped by theme, deduped against the first pass, and again only listing items not already shipped, in `NEXT.md`, in `PARKING_LOT.md`, or in `launcher-landscape.md`.

Same tagging convention: **★** strong fit, **○** decent fit, **△** borderline.

## Launch lifecycle & startup polish

These all relate to *what happens between "click play" and "game on screen"* — one of the highest-frequency request clusters on the forum.

- **★ Per-system + per-game startup video (the "boot animation" between click and game).**
  Forum: "Splashscreen Video when start a game" (13 replies, 3.4k views), "Customizing Startup/Pause/Shutdown screens for emulators and games", "Can you turn on the game startup screen for only one platform?", "Big box platform startup video", "Platform loading videos". Pattern: short MP4 plays full-screen during core init, fades into the running game. Per-platform default, per-game override. Sits in the same `fade-to-loading-screen` slot as the controller-mapping splash — share the implementation.

- **★ Per-game custom pre-launch text card / disclaimer.**
  Forum: "Display a message before launching a game?". Single text field on the game row, modal shown with skip-with-any-key. Use cases: tip cards ("Press Select+Start to skip the opening crawl"), historical context ("This is the Japan-only revision; English patch applied"), warnings ("This game's framerate fluctuates intentionally — that's not a bug"). One `pre_launch_message TEXT` column; ~30 LOC.

- **★ Pre-launch / post-exit script hooks.**
  Forum: "Is there a feature to run a script before emulator executes?", "Run Python script before rom launch", "Request - Execute script on launch/close game", "Apps on Exit", "Option to launch app before or after game", "[Request] Option to Close the 'Additional App' that was Set to Run Before Game", "Launchbox ability to startup script/app on launch and kill on close", "add prestart app when starting game" — *a dozen separate threads, easily the most-recurring power-user ask*. Pattern: per-system or per-game shell command run before launch (e.g. set monitor rotation, mute Discord, change ambient lighting), and a paired command run on exit. Storage: two `TEXT` columns on system + game. Safety: opt-in by user; surface as Advanced section in per-game / per-system settings. ~80 LOC.

## Library navigation & sorting

- **★ A–Z alphabet jump ribbon / letter-press quick scroll.**
  Forum: "Alphabet letter listing - select letter", "A-Z Vertical Navigation Ribbon in LB mode?", "Launchbox scrollbar Letter indicator?", "An option From A-Z choose?", "Control F - Find". For libraries with 5000+ games, scrolling is hostile; the standard mobile-pattern alphabet ribbon (A B C…Z 0–9 ★) jumps the virtualizer to the first game starting with that letter. Already affordable via TanStack `scrollToIndex`. ~80 LOC.

- **★ Comprehensive sort options with persisted ascending/descending toggle.**
  Forum: "Sort by Release Date (Year) Descending", "BIGBOX _Sort by release date", "Add the feature to sort games by release date per platform/playlist in BigBox" (8 replies, "please vote"), "Can you reverse the 'Arrange by - Date added' to descending?" (21 replies, 8k views), "[Feature Request] Sort by Playlist", "Sorting games in Bigbox by star rating?", "Add a decimal point of granularity to Sort by Star Rating", "Sort by Notes field", "Separate 'Arrange By' for Platforms, Categories, Playlists". Single feature: every visible game list has Title / Release Date / Last Played / Date Added / Date Modified / Star Rating / Play Time / Orientation sort options, each toggleable ASC/DESC, with the sort choice persisted per-view (so the Favourites view can sort by Last Played while the MAME view sorts by Title). ~150 LOC SQL + UI dropdown.

- **○ Mark Played / Unplayed manually + completion-status tracking.**
  Forum: "Mark as Played/Unplayed", "Beta Progress Feature", "Automatic Progress Setting for Paused", "Add 'Games with Saved Progress' feature to LB and BB", "# 3 Suggested/Recommended Games", "Feature Request: Last Played Game improvement". Distinct from RetroAchievements (which is per-game machine-detected) — this is a user-controlled status: `Unplayed | In Progress | Beaten | Mastered | Abandoned`. Surfaces as a chip on the tile and as a filterable column. The "Beaten" forum-feature in RA partly inspires this, but the user wants manual control as well.

- **○ Wishlist / "to play later" pile.**
  Forum: "Wishlist". A first-class list separate from owned/favourites/playlists. Useful when a user has scanned ROMs they don't yet own legitimately but want to remember to acquire, or for "play this when I beat the current one" queueing.

## Filtering & search

- **★ Multi-axis filter chips + saved filter sets.**
  Forum: "Filters greatly needed", "Feature Request - Multiple filtering support", "Search titles in OR statement (multiple search)", "Search by Notes field", "Search in Launchbox not searching 'Series'", "Big Box filters", "Deactivate some filters", "Toggle on/off some filters". Already partially in `launcher-landscape.md` §M9 saved filter sets and §M8 smart playlists. The forum elaboration adds: (a) **filter chips at the top of every list** as the primary interaction (not a buried menu), (b) **search-across-all-fields default**, with explicit field qualifiers (`series:Castlevania`, `tag:co-op`) as an opt-in, (c) **deactivate-but-remember** chips so users can fine-tune without rebuilding from scratch.

- **★ Hide BIOS / non-playable entries.**
  Forum: "Only show specific regions and hide BIOS roms?" (14 replies, 5.7k views). One toggle: hide entries flagged `is_bios=1` from library lists by default. Also hides `[BIOS]`-tagged MAME romsets. Surfaces only inside the Cores / BIOS settings panel.

- **★ Mature / adult content filter with PIN gating.**
  Forum: "Hide Mature Games", "Adult filter? Parental stuff?", "Pin-Locked Games" (13 replies, "final request for developers"), "inappropriate game art". Family installations need a way to hide adult-rated games behind a PIN or just hide them outright. Standard: ESRB / PEGI mature flag on the game row (scraper-populated), per-user toggle in Settings → Content. PIN gating is the more involved version — a four-digit code unlocks the hidden section. ~120 LOC for the basic toggle; +80 for PIN.

## Visibility / hierarchy

- **○ Playlist subcategories / nested playlists.**
  Forum: "Question: Is there a way to add playlist subcategories?", "Subcategories BigBox", "Can I create sub-genres?", "Platform Sub-Categories?", "Subcategories" (18 replies, 5.9k views). Playlists today are flat. The ask: nest them — Genre → Action → Beat-em-Up → Final Fight. Storage: `parent_playlist_id` nullable column. Sidebar tree-view.

- **○ Per-platform "hide from sidebar" toggle.**
  Forum: "Hide Nested games from parent platform", "Hide Game from Platform List But Show In Nested Playlist?", "possible? hierarchal menus and easy hide of platforms?", "Ability to choose which Side Bar categories are visible?". Already noted as "Hide platform / playlist" in v1 of this survey; the deeper-dive variant adds: a game can be **hidden from its native platform list but still visible in a playlist it's a member of** — useful for "I only want my favourite arcade games visible in MAME, but my Capcom playlist still pulls them in."

- **○ "Show only games missing X asset" view.**
  Forum: "Is there any way to group by, or to only show games that are missing the asset that is to be shown on the menu?", "List of missing games per system", "Request - Missing Rom feature", "Auditing Tool" (5 replies, 2.3k views), "Allow audit tool to work on playlists, not just platforms". A library curation surface: filter to games missing box art, missing screenshots, missing video previews, missing release year, missing developer. Inverse of the standard "show all" — surfaces curation work that needs doing. Pure SQL `WHERE field IS NULL` query, dropdown chooses the field.

## Media & assets

- **★ System-page background music with per-system tracks + shuffle.**
  Forum: "no option for background music in Launchbox", "Shuffling BigBox Background Music" (13 replies, 11.1k views), "Theme music?" (3 replies, 4.2k views), "Big Box background music" (16 replies, 14.7k views), "How To Add Music To ROM Select?", "How to create a jukebox with categories 70s 80s 90s?", "Random Startup videos for bigbox". One of the *highest-engagement clusters* on the forum. Pattern: each system home has a music folder (`themes/<system>/music/*.mp3|ogg|flac`), tracks shuffle by default, volume mixes against the menu-audio bus (already proposed in v1). Cross-fades on game-tile focus change *don't* swap the track — only entering a different system does. Add: respect `--no-music` CLI flag for kiosk installs.

- **○ Per-game preview video volume normalization.**
  Forum: "Bigbox - video sound taking priority over background music". When a game tile is focused and its preview video starts playing, the BG music ducks (sidechain compress 6 dB) rather than fights for headroom. Single `GainNode` automation; ~20 LOC on top of the per-bus mixer.

- **★ Auto M3U generation for multi-disc CD games.**
  Forum: "Auto M3U Generation?", "Multi Disk Support (Seperate from additinal apps)", "Need help with automatic multi-disc M3U creation" (6.2k views), "Suggested feature for multi disc games", "Multi-Disk Games/Duplicates", "Problem with Versions and Multi-Disc Games" (5.6k views), "[Tutorial] Dealing with games that have playable secondary discs". Pattern: on scan, detect `Final Fantasy VII (USA) (Disc 1).chd` + `(Disc 2)` + `(Disc 3)` sharing a stem → generate `Final Fantasy VII (USA).m3u` containing all three. Single library row, three disc children. Disc-swap inside the existing quick-menu disc-cycle UX. Sits on top of OA's existing `extract_to_temp` + disc-id extraction.

- **★ Custom badges for fields and user metadata.**
  Forum: "Custom Badges for Custom Fields and Existing Metadata Fields", "Input Badges request". Extends §M5 hero/logo/capsule. Pattern: user defines a badge (small PNG/SVG + label + colour) and binds it to a field condition — "show this badge when `tags` contains 'co-op'", "show this when `play_time` > 50 hours", "show this when `completion_status = Mastered`". Surfaces as overlay chips on the library tile. Theme-overridable. ~200 LOC; pairs with Tags + Mark Played.

- **○ Series field as first-class metadata.**
  Forum: "Search in Launchbox not searching 'Series'". Already exists as a scraper field; the ask is that it appears in search, in the sort dropdown, and as a sidebar grouping (Final Fantasy → I, II, III, IV…). Trivial promotion of existing data. ~50 LOC.

- **○ Multiple manuals per game.**
  Forum: "Games with multiple manuals", "Is there a way to add a second manual that can be accessed on the pause screen, so you can add a guide?". Tech manual + strategy guide + map sheet — multiple PDFs per game. Pause overlay shows a tab strip per attached manual. Schema: `game_manuals(game_id, label, path)` join table.

- **△ APNG / animated badge support.**
  Forum: "APNG support?". Mostly cosmetic; modern WebView2 supports APNG natively, so this is more about *encouraging* animated badges (a library of pre-made animated genre badges, completion-tier badges, etc.) than about format support per se.

## Controller / input

- **★ Per-game controller mapping (beyond per-system).**
  Forum: "Is there a way to set up different controls for different games on the same platform?" (2.5k views), "Configuring Two XBox360 Controllers for two player alternating games", "Individual controller mapping/autoconfigs" (4.5k views), "Improved Controller Profiles and Auto-Switching Between Systems" (page 3, v1 survey noted but underplayed). OA's existing bindings cascade is per-system; the ask is one more level — per-*game*. Use case: MAME's CPS-2 games want the 6-button SF2 layout but Pac-Man wants a 1-button layout, and per-system MAME bindings can't satisfy both. Storage already exists (`GameOverrides` is the right shelf). ~100 LOC UI.

- **★ Battery indicator for connected gamepads.**
  Forum: "Battery Indicator for Game Controllers". Small but felt — DualSense / Xbox Series controllers report battery; surface as a chip in the status bar. SDL2 gamepad query returns this directly. ~30 LOC.

- **○ NFC / RFID tag launch.**
  Forum: "NFC - Loading games from BigBox". Cabinet builders use cheap NFC readers + tagged "cartridges" as physical launchers — wave a tagged card at the reader, the game launches. OA's direct-launch CLI already supports this *programmatically* (any external NFC reader can shell-out to `oa-shell.exe --rom …`); the forum ask is for **first-class NFC support in-app**. △ rather than ★ because USB NFC is niche; better as a community plugin.

- **○ Stream Deck / Elgato hardware key launching.**
  Forum: "Launch a game with a key of StreamDeck", "Using Elgato Stream Deck to access Bigbox games/Platform/Genres etc.", "Incorporating a Stream Deck for Commodore 64 games", "Bigbox won't register streamdeck input". The HTTP API §S12 from `launcher-landscape.md` is the architectural answer; the Stream Deck plugin lives outside OA and just hits that API. Worth marking explicit because the forum demand is concrete.

- **△ Map controller Home/Guide button as pause hotkey.**
  Forum: "Map Xbox/PS Home button as pause menu button?" (4.4k views). The Home button is usually swallowed by the OS overlay (Xbox Game Bar, Steam Overlay). Possible with low-level XInput hook but anti-pattern in many Windows configs. Defer until operator demand.

## Multi-monitor / cabinet

- **○ 3+ monitor support (marquee + cabinet + main).**
  Forum: "BigBox - Second Monitor Question" (57 replies, 17.2k views), "Launchbox / Big Box 3 Screen (Multi) Setup", "BigBox with 3 screens?" (24 replies), "2-Monitor Help", "Dual screen - possibilities?". S10 covers a 2nd marquee window; the deeper ask is N windows: main + marquee + LCD ticker + control-panel reference. Tauri supports multi-window natively. Cabinet build feature.

- **○ Cocktail-cab flipped P2 display.**
  Forum: "Cocktail cabs anyone? Please vote!", "linking two PC's with Big Box four player", "Dictated P1 P2 screen?". For cocktail cabinets where two players sit opposite each other and the display flips between turns. Per-player rotation hook on the renderer.

## Achievements & progress (deeper than v1)

- **○ RetroAchievements UI: badge filters, beaten/mastered cards, score in themes.**
  Forum: "Support for Retroachievements new beaten functionality", "RetroAchievements Badge Question / Potential Feature Request", "Show achievements score in big box themes", "Achievement icon in Roms group", "Achievements List not showing for certain games", "Retroachievements - can you remove the menu item by platform?", "Retroachievements File Compatibility Check". §C1 in `launcher-landscape.md` lists "RetroAchievements deep integration"; the forum data sharpens the asks: (a) **per-platform RA toggle** (some users want RA *off* for systems where the achievement set is junky); (b) **beaten-status filter** alongside the per-game completion status; (c) **badge grid** on Game Detail and a leaderboard surface.

## Migration & multi-install

- **○ Library / settings / playlist transfer between installs.**
  Forum: "Windows to Windows Migration?", "Transfer playlist and roms feature" (page 7), "Transferring a Purchase and Settings", "copying favorites to another instance of Launchbox?", "Image Cache Portability Between Installs", "Easier way to add media to external drive". One-step export-to-ZIP of library DB + media + settings → import on another install. Distinct from cloud sync (§C7) — this is *manual* portable transfer.

- **○ Multiple libraries / split data files by platform.**
  Forum: "Discussion: multiple libraries", "alternate master list?", and the perf thread's "split systems could be added to LaunchBox natively." OA's SQLite-from-day-1 architecture already wins on raw perf; the deeper ask is a *workflow* feature: a single OA install hosts multiple named library "profiles" (Dad's, Kids', Public installation, Speedrun-only). Add a `library_id` discriminator across rows. Phase 5+ project.

## Cross-system / cross-game linking

- **○ "Play this on a different platform" jump from Game Detail.**
  Forum: "Feature Request - Same game across multiple platforms", "Possible Ports: Game with same LB-ID, but different platform", "From ROM details screen want to be able to jump to any other platform version of the same game". Castlevania exists on NES + Famicom Disk + Game Boy + N64 — Game Detail should surface a "Also on:" affordance to jump between releases. Different from the master/child versioning (which is for *the same release across regions*) — this is *the same series across hardware*. Schema: ship the libretro-database `Series` field + a same-series query on the game detail page. ~80 LOC.

## Accessibility & UX

- **○ Vision-impaired accessibility pass.**
  Forum: "Legally Blind Accessibility For Vision Impaired Gamers", "Accessibility". A11y is mostly a cumulative discipline — high-contrast theme, full keyboard navigation (already broadly true), screen-reader-friendly ARIA labels on icons. Worth a dedicated audit pass once UI surfaces stabilise.

- **○ Per-platform / per-game custom title font + colour.**
  Forum: "Customizable game title font colors (bitbucket #7543)", "BigBox wheel fallback text font". Per-system already has theme-variable hooks; per-game would let users *visually flag* a special title in the grid ("this is a fan-translation"). Risks visual cacophony; default off, opt-in per-game.

- **○ Restart shell / "reload app" menu item.**
  Forum: "Minor QOL feature request - 'Restart Bigbox' menu option", "auto cycle through installed games". Settings change that requires a relaunch should offer a one-click restart rather than "please close and reopen the app." ~5 LOC via `tauri::process::restart`.

## Performance / runtime

- **○ "Lite mode" preset that disables animations, video previews, ambient effects.**
  Forum: "Performance presets" from the BigBox perf thread; "Reduced resource consumption during emulation"; "Big Box background blur option?". The launcher-landscape.md already commits to outperforming LaunchBox; this is a one-toggle "give me everything off" mode for low-end hardware / Steam Deck. Aliases existing settings (animation: off, preview videos: off, ambient WGSL: off, background music: off). ~30 LOC, one bool, mostly UX work.

- **○ Auto-suspend OA UI while emulator is running.**
  Forum: "BigBox to knock itself down to some kind of bare bones form temporarily while an emulator is running". OA already drops the libretro core on user-unload (reference memory). The forum ask extends to *the UI itself* — pause animations, throttle render, defer background tasks while a game is in foreground. ~50 LOC if the existing two-window mode is the model.

- **○ Save state browser / restore manager UI.**
  Forum: "Emulator Save Backup manager". Phase 1.5 / Phase 4 ship save-states + thumbnails; the forum data emphasises a *standalone management UI* — browse all save states across all games, sort by date, prune old ones, restore from a backup folder. Mostly a route + list view; the data is already there.

## Netplay / capture / streaming

- **△ Netplay (RetroArch-style).**
  Forum: "Include Netplay in BigBox", "How does the Netplay Lobby Feature for Retroarch works?", "Suggestion: Netplay system with video streaming, similar to Parsec". libretro cores ship netplay support; OA's libretro frontend could expose it. Borderline because the moderation / lobby surface is a substantial product on top.

- **△ Parsec / Chiaki / remote-play integrations.**
  Forum: "Parsec Integration - Remote Game-Sharing", "Adding Chiaki to Launchbox". The HTTP API §S12 + external apps is the architectural answer rather than first-class integration.

- **△ Built-in OBS replay-buffer-style capture.**
  Forum: "OBS Studio Replay Buffer", "Alternative Game Capture Method", "Screenshot animated gif". OA already has video capture infra (`video_capture.rs`). The forum ask layers a *replay buffer* (last 60s rolling) hotkey-saved on demand. ~150 LOC if extending existing capture; defer until Phase 4+.

## Specifics worth noting (small but recurring)

These are tiny features whose forum cumulative reply count says people care more than the size suggests:

- **○ Mouse-wheel auto-scroll** (middle-click drag-scroll). Forum: "Mouse-Wheel Click / Auto-Scroll Feature". Two lines of JS.
- **○ Star ratings with decimal granularity** (4.5 stars, not just 1–5). Forum: "Add a decimal point of granularity to Sort by Star Rating".
- **○ "Apply" button in Options dialog** (don't autosave on every change). Forum: "Please Add 'Apply' to Options Menu", "'Apply' Changes in Options". A11y + accident-prevention concern.
- **○ System tray + tray hotkeys** (minimize to tray, global mute hotkey). Forum: "Minimize to system tray" (30 replies, 14.7k views), "Any way to bind mute system through BigBox?". Tauri supports tray icons natively.
- **○ Inline rename of platforms / playlists.** Forum: "How to rename platform title without change folder name and structure?", "Can't rename predefined emulators".
- **○ Custom emulator labels.** Forum: "Can't rename predefined emulators". Users want to display "RetroArch (Beetle PCE Fast)" instead of just "RetroArch" when multiple cores are wired.

## Updated explicit non-scope (expanded)

Adding to the v1 list from forum pages 4–30:

- **Java / J2ME / feature-phone games.** Niche; defer.
- **Pinball cabinet (VPX) support.** Visual Pinball X is a distinct product; cabinet builders typically run it parallel to OA, not inside.
- **3D box rendering models (multiple variants, EU/JP/Long Box variants).** `launcher-landscape.md` §S3 covers 3D box rendering as a generic feature; per-region 3D model variants are a content asset problem, not a feature.
- **Steam Family Share / Xbox Game Pass / PS Now integration.** Out-of-scope storefront work.
- **Online play (PSN / Xbox Live).** Not an OA concern — those services are platform-specific and not approachable without official SDK access.
- **Pinball Future Pinball Hiscore.** Out of scope (cabinet pinball).
- **Game Magazine viewer (scanned magazines).** Distinct product; out of OA's "play the games" framing.

---

## Recommended next bites (revised — full list)

Combining the v1 list with the deeper-dive findings, ordered by impact-per-effort:

### Tier A — small effort, high felt-impact
1. **Pre-launch controller-mapping splash card** (v1).
2. **A–Z alphabet jump ribbon** for library lists.
3. **Random-game wheel-spin** (v1).
4. **Comprehensive sort options + ASC/DESC toggle** persisted per-view.
5. **Custom global hotkeys UI** (v1).
6. **Battery indicator chip** for connected gamepads.
7. **Restart shell menu item.**
8. **"Apply" button in Options dialog** (and accident-prevention pass).

### Tier B — medium effort, premium-shell differentiators
9. **Tags as first-class metadata** (v1).
10. **Per-bus audio mixer + per-system enter/exit stings** (v1).
11. **Pre-launch / post-exit script hooks** — one of the most-requested power-user features on the whole forum.
12. **Per-system startup video slot** (sharing the loading-screen art pipeline).
13. **System-page background music with shuffle + ducking when previews play.**
14. **Mark Played / Unplayed manually + completion status field** with badge.
15. **Auto M3U generation for multi-disc games on scan.**
16. **Per-game pre-launch text card.**
17. **Mature / parental filter with optional PIN gating.**
18. **Hide BIOS roms toggle.**

### Tier C — strategic / larger investments
19. **i18n infrastructure** (v1).
20. **Per-game controller mapping (one level beyond per-system).**
21. **Game-as-container master/child hierarchy** (v1).
22. **Custom badges bound to field conditions.**
23. **PiP manual viewer while playing** (v1).
24. **Library / settings / playlist export-import migration tool.**
25. **Lite mode preset toggle.**

### Tier D — cabinet-builder cohort (only if that audience materialises)
26. **3+ monitor / cabinet support** beyond marquee.
27. **Cocktail-cab flipped P2 mode.**
28. **NFC / RFID tag launch** (or expose HTTP API for an external NFC service).
29. **Stream Deck integration** (via HTTP API §S12).
30. **Auto-restart on close / kiosk persistent profile** (already in `PARKING_LOT.md`).

Items not in any tier above are queueable but not load-bearing; they're listed in the body of this doc for reference.
