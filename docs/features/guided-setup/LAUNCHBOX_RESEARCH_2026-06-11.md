# LaunchBox Desktop — Competitive Research: Settings, Import, Library & Emulator Management

**Date:** 2026-06-11
**Author:** Claude (4 primary + 6 sub research agents → synthesis)
**Purpose:** Deep competitive analysis of the **LaunchBox desktop app** (NOT
BigBox theming — that's covered in `features/theming-substrate/BIGBOX_RESEARCH_2026-06-11.md`)
across import, library setup, emulator/controller management, metadata
viewing/editing, the settings surface, and the overall feature set — so OA
can "do better in every way" on setup, import, and library/emulator
management.

Scope owners this feeds: **guided-setup** (this folder), **virtual-library
+ launcher arc** (`PLANS/virtual-library-and-launcher-arc.md`), **launcher
abstraction** (`PLANS/launcher-abstraction.md`), **game-identities schema**
(`PLANS/game-identities-schema.md`), and a not-yet-planned **library/metadata
editing** surface (see §4 — the biggest greenfield).

Citations live in the raw agent outputs (session transcript). Load-bearing
ones are inline. `[INFERENCE]` / confidence caveats preserved from agents.

---

## 0. TL;DR — OA already wins the structural fights; the work is UX depth

**The headline:** LaunchBox's four most-damaging, most-cited weaknesses are
all *architectural* and all things **OA already solved or is mid-building**:

| LaunchBox's structural weakness (10-yr, founder-acknowledged) | OA's status |
| --- | --- |
| **Name/substring matching, not hash** — its #1 import-accuracy complaint | ✅ OA ships **5-tier Hash/Header/Extension/Hint** ID (`scan_service.rs`) |
| **Entire 108k-game `Metadata.xml` loaded into RAM at startup** — the named root cause of slowness at scale ("a full rewrite might be necessary") | ✅ OA is **SQLite-backed**, indexed, lazy |
| **WPF + Windows-only** — can't do Linux/Steam Deck without a ground-up rewrite (dev declined) | ✅ OA is **Rust + wgpu**, cross-platform by construction |
| **Per-platform import wizard run once per system; no auto-detect of installed emulators; cores are manual** | ✅ OA ships **smart-scan results table** (guided-setup Phase 1B) + **ships its own cores** in `cores/` — no RetroArch-core-association trap at all |

So this report is **not** "catch up to LaunchBox." It's: (a) confirm those
wins, (b) harvest LaunchBox's genuinely good UX patterns and exact field
vocabularies so OA reaches parity-or-better on *depth*, and (c) catalogue
LaunchBox's pain points so OA's still-unbuilt surfaces (esp. **metadata
editing**, §4) don't reproduce them.

**The five things to actually act on (detail in §9):**
1. **Build a first-class metadata-editing surface** — LaunchBox's single
   biggest *interaction* weakness (modal-only, no inline edit, no undo,
   15–20s open lag, overwrite-only bulk edit). OA has **no editing UI yet** →
   pure greenfield to win.
2. **One-pass multi-system import** ("point at my whole ROM tree, detect
   every platform at once") — beats LaunchBox's run-the-wizard-per-system
   *and* the real ease-of-setup benchmark (RetroBat/ES-DE/Batocera).
3. **A real "fix the wrong match" flow** — LaunchBox's years-old open sore
   (clearing a DB ID and re-searching often returns the same wrong result).
4. **Proper boolean smart-collection logic (AND/OR/NOT + groups)** —
   LaunchBox explicitly punted ("would get tremendously complicated").
5. **A "set as default image" picker + download-only-missing media** — the
   two most-cited media-management frustrations.

---

## 1. The settings architecture — LaunchBox's two-surface split vs OA's tiers

**LaunchBox has TWO separate settings surfaces** (a real structural fact):
- **Desktop `Tools > Options`** = library/curation tree. Recent 13.x reorg
  into a shallow nested left-tree: **General** (Video Playback, System Tray,
  Save Management, Updates, Automated Imports, Frame Rate), **Visuals**
  (Game Details, Filters Side Bar, Boxes, Dialog Theme), **Data** (Search,
  Backups, Game Progress + Priorities), **Gameplay** (Game Startup, Pause
  Screen), **Related Games**.
- **BigBox `System Menu > Options`** = kiosk/presentation tree: **Sound**
  (music) + **Sounds** (nav SFX sound packs), **Videos** (playback engine),
  **Attract Mode**, **Screensaver**, **Views**, **Transitions**, **Images**,
  **Image Cache**, **Controller Mapping**.

Notable: **audio, image-cache, and most presentation settings exist ONLY in
BigBox, not on desktop.** Play-tracking is called **Game Progress** (under
Data), not "Scrobbling." Plugins are managed at `Tools > Manage > Plugins`,
*outside* Options.

**OA mapping:** OA's **three-tier settings split** (OA-wide / per-system /
per-game — memory `feedback_settings_three_tier_split`) is a *cleaner* model
than LaunchBox's accidental desktop-vs-BigBox bifurcation, because OA's split
is by *scope* not by *which executable*. With the theming substrate's
engine-vs-theme territory split, OA already routes "kiosk/presentation"
settings to theme territory and "library/curation" to engine territory —
mirroring LaunchBox's two trees but on a principled boundary. **No action;
validate the model is right.** One harvest: LaunchBox's **Game Progress
Categories→Values** taxonomy (Not Started: Unplayed/Want to Play/Won't Play ·
Active: In Progress/Continuous/Paused · Done: Beaten/Completed/Mastered/
Dropped) is a well-designed, user-customizable progress model worth copying
wholesale if/when OA adds progress tracking.

---

## 2. Import & library setup

### 2.1 The import wizard flow (the exact 10 screens — parity checklist)
LaunchBox `Tools > Import > ROM Files > Single Platform`, **one platform per
run**:
1. **Choose Your Files** (folder recommended)
2. **Select a Platform** (auto-prefilled from folder name; custom names get a
   **"Scrape As"** mapping)
3. **Set Up Your Emulator** (auto-setup supports exactly: **RetroArch,
   Dolphin, PCSX2, MAME, ScummVM, BigPEmu, Xemu**; else manual)
4. **Missing Dependencies** (BIOS/system files, red/yellow/green status dots)
5. **File Handling** — three radios: **Copy into LaunchBox / Move into
   LaunchBox / Use files in their current location**
6. **Choose Media to Download** (+ a **Media Limit** cap per type)
7. **EmuMovies Integration** (paid for videos/music/manuals)
8. **Download Bezels** (The Bezel Project, 4 fallback modes)
9. **Advanced Settings** — checkboxes incl. **Force import duplicates,
   Ignore subfolders, Use folder names as game titles, Detect PDFs as
   manuals, Combine ROMs into one game, Force MAME metadata for MAME-named
   ROMs**
10. **Review & Finish** — games playable instantly; **media downloads
    continue in the background and resume across restarts**

A separate **MAME Arcade Full Set** wizard is DAT/listxml-driven (clone→parent
resolution, region priority, "skip mahjong/casino/mature/non-working"
filters). **Automated ROM Imports**: dropping ROMs into `Games\<Platform>\`
auto-adds + scrapes with no wizard.

**OA mapping & wins:**
- OA's **guided-setup Phase 1B** already ships the *better* version of screens
  1–5: a **per-ROM smart-scan results table with confidence tiers**, a
  **per-system readiness checklist**, **bulk missing-core download** (OA ships
  its own cores → no "associate the right RetroArch core" step at all), and
  **guided BIOS resolution with a per-file picker**. This is ahead of
  LaunchBox already.
- **The one big gap to close (recommendation #2):** LaunchBox is *one platform
  per wizard run*; even its own users complain ("import a folder of 10 systems
  → run the wizard 10×"). The real ease-of-setup benchmark is RetroBat/ES-DE/
  Batocera's **drop-folder, auto-reveal-systems** model. OA should make
  **one-pass multi-system detection** the default (scan a whole tree, classify
  every platform at once), with the wizard as the beginner on-ramp, not the
  only path. *This is the single highest-leverage import win and it maps onto
  guided-setup Phase 2C (folder management) + the virtual-library scan path.*
- **Keep the good defaults:** background/resumable media download, "media
  limit" cap, and the copy/move/in-place choice are all worth matching.
- **Harvest the Advanced-Settings checklist** as a parity list (esp.
  "Combine ROMs into one game" = OA's variant grouping; "Detect PDFs as
  manuals").

### 2.2 Matching — OA's biggest structural advantage, already built
LaunchBox matching is **filename/title regex-substring against an in-RAM XML
blob — NOT CRC/hash** (official: import proceeds LBDatabaseID → filename →
title). This is permissive enough that "Alien 2"/"Aliens" cross-match, and
it's the root of its #1 import complaint. ROMs named by CRC32 *fail* to match.

**OA mapping:** OA's **5-tier Hash/Header/Extension/Hint** chain + disc-track
SHA-1 matching (`PLANS/disc-track-sha1-matching.md`) is exactly the hash-first
model LaunchBox can't do. **Validated bet — no action.** The competitor lesson
(ScreenScraper, used by ES-DE/Batocera/RetroBat/Skraper/ARRM) confirms
**hash-first with filename fallback** is the winning pattern; OA already has it.

### 2.3 Multi-disc / M3U / archives / BIOS edge cases (harvest the specifics)
- **Multi-disc:** LaunchBox auto-combines via an **Additional Apps** model;
  recognizes `Disc 1`/`Disk 1`/`Disc 1 of N` but **NOT `Side 1`/`Load 1`**.
  Manual `Combine Selected Games` / `Expand Selected Games`; launch via
  right-click **Play Version**. → OA's variant grouping should recognize the
  superset of these patterns (incl. `Side`/`Load` for computers).
- **M3U:** a per-emulator toggle **"Use M3U Playlists for Multiple Discs"**;
  LaunchBox **generates the `.m3u` at launch as a temp file** (doesn't litter
  ROM folders). Good pattern to copy.
- **Archive extraction is a per-EMULATOR flag, not per-core** — a real,
  long-standing pain point (Flycast wants extract-ON, DeSmuME wants OFF →
  users duplicate the emulator entry). **OA should make extraction
  per-system/per-core**, not per-emulator. (The community's
  `Archive Cache Manager` plugin — extract+cache + per-platform extension
  priority preferring `.cue` over `.bin`/`.iso` — is the pattern to beat.)
- **BIOS:** LaunchBox has **NO standing BIOS auditor** — only a first-import
  prompt. **OA already beats this** (BIOS resolution + grouped Issues/Ready +
  System Health Overview are shipped). Validated win.

### 2.4 Library organization primitives
- **Platforms** (one XML + media folders each), **Platform Categories**
  (nestable; a platform can be in multiple; a **"Root"** checkbox controls
  top-level visibility; stored in `Parents.xml`), **Playlists** (manual +
  auto-populate), **Favorites** (boolean), **Star Rating** (0–5). **Custom
  collections are implemented AS playlists** — no separate collection object.
- **On-disk layout** is a fully-portable self-contained `LaunchBox\` folder:
  `Data\Platforms\<Platform>.xml`, `Data\Playlists\*.xml`, `Images\<Platform>\
  <MediaType>\` (exact folder names like `Box - Front`, per-game files
  `<Game>-01.png`), `Videos\`, `Manuals\`, `Music\`, `Games\` (ROMs only if
  copied/moved in). **Media tree is always LaunchBox-managed but remappable**
  via a per-platform **Folders** tab.
- **Auto-populate ("smart") playlists** — rules are `Field / Comparison /
  Value` triples; fields incl. Platform, Genre, Play Count, Star Rating,
  Title, Publisher, Developer, Source, Region, Favorite, Progress. **CRITICAL
  LIMIT (verbatim from Jason Carr): no OR-vs-AND toggle — multiple rules are
  AND-only.** The only OR is "has at least one of the values" *within a single
  multi-value field*. No nested groups, no "top N" count limits. Playlists are
  **computed dynamically at runtime** (a perf tax at scale).

**OA mapping:**
- OA's variant grouping (`library_groups.rs` + `game_group_defaults`) +
  SQLite already gives a *canonical-game* model LaunchBox lacks (it groups via
  Additional Apps hacks). The **virtual-library arc's schema promotion**
  (`game_identities`) is the right home for collections-as-first-class.
- **Recommendation #4: ship real boolean logic** (AND/OR/NOT + nested groups +
  count limits) for smart collections — LaunchBox explicitly won't, and Pegasus/
  ARRM users prove the demand. With SQLite this is a query-builder, not a perf
  problem.
- Consider whether OA imposes a managed media tree (LaunchBox-style) or adopts
  the **Pegasus convention** (`media/<type>/<game>` beside ROMs + extensible
  `x-` keys) which buys free interop with Skraper/ARRM and portability. *Open
  question for the virtual-library schema work (§10).*

### 2.5 Maintenance utilities (and their bugs to avoid)
- **Audit window** — spreadsheet of the library, one column per media/metadata
  type, sort-by-count to surface zeros. The de-facto find-missing-media tool.
  **Limits:** filename-only (never reads ROM contents or DAT-compares), can't
  tell you which games you're *missing* from a full set.
- **Scan for Added ROMs** — only looks in `Games\<Platform>` by default.
- **Scan for Removed ROMs** — **known-buggy** (reports "no missing ROMs" even
  after deletions; bug tickets #7095/#6753).
- **Download Metadata and Media** — bulk, but **no fill-missing-only toggle**
  (re-scans everything, appends `-01/-02` rather than overwriting); and the
  bulk path is *stricter* than per-game "Search for Metadata" (finds less).

**OA mapping:** OA's System Health Overview + readiness checklists already
cover the "what's not ready" axis better. **Recommendation #5 in part:** give
OA a **"download only missing media"** mode (LaunchBox's explicit gap) and a
DAT-aware **"what am I missing from the full set"** audit (nobody does this
well; OA's hash identity makes it tractable).

---

## 3. Emulator & controller management

### 3.1 Manage Emulators (the field vocabulary)
`Tools > Manage Emulators > Add/Edit`. **Details tab:** Emulator Name,
Application Path, **Default Command-Line Parameters**, checkboxes **"use file
name only without file extension or folder path"** / **"Don't use quotes"** /
**"Extract ROM Archives Before Running"**, plus **Running / Exiting AutoHotkey
script** tabs (the "with code" escape hatch; e.g. `WinClose, ahk_exe
{{{StartupEXE}}}` for soft-close).
**Command-line variables:** `%romfile%`, `%romlocation%`,
`%romlocation_noquotes%`, `%romfilename%`, `%romextension%`, `%platform%`,
`%gameid%`, `%launchboxorbigboxexepath%`, `%noromfile%`. LaunchBox
**auto-appends the ROM path** unless `%romfile%`/`%noromfile%` is present —
and **the UI preview lies about this** (still shows the appended file).
**Associated Platforms tab:** per-platform Core (RetroArch only), Extra
Command-Line Parameters, Extract ROMs Before Playing, Use M3U Playlists,
Status column. **Command-line priority:** Game > Associated-Platform >
Emulator-Default (first non-empty wins, not concatenated).

### 3.2 RetroArch handling — the special case (and OA sidesteps it entirely)
The **Core** field only appears for RetroArch. LaunchBox **scans
`RetroArch/cores`** to populate the dropdown — only installed cores show, and
internal-vs-display naming (Beetle filed under `mednafen`) confuses users
into thinking cores are missing. The official **RetroArch Integration Plugin**
auto-downloads RetroArch + matching cores, builds a ready-to-use profile,
validates BIOS via MD5, syncs RetroAchievements creds, applies bezels — the
single best part of LaunchBox's setup story, but **RetroArch-only**.

**OA mapping — a genuine structural advantage:** because OA **ships its own
forked-core `.dll`s in `cores/`** and loads them via `libloading`, the entire
"associate the right core / core not installed / internal-name mismatch / go
download RetroArch" trap — LaunchBox's #1 *emulator-setup* pain cluster —
**does not exist in OA.** Lean on this hard in onboarding copy: *"emulators
just work; nothing to download or associate."* The **Launcher trait** arc
(external standalone emulators: Cemu/RPCS3/etc.) is where OA should adopt the
*good* half of the plugin model — **emulator-definitions-as-data** (Playnite's
per-emulator YAML / OA's `config/emulators/<id>.yaml`) so external-emulator
launch flags are curated, never hand-authored.

### 3.3 Per-game / per-platform overrides (discoverability gap to beat)
Per-game override lives in **Edit Game > Emulation tab** ("Use Custom
Command-Line Parameters" + emulator selector) — **buried**, and **bulk
overrides require a third-party plugin** (Bulk Custom Command-Line Editor).
Recurring complaint. **OA action:** surface per-game emulator/option overrides
prominently (per-game settings tier already exists in OA's model) and support
**native bulk** override. Maps to OA's per-game settings tier + the launcher
arc's per-game settings model.

### 3.4 Controller / input setup (LaunchBox's weakest input story)
- LaunchBox's controller mappings drive **LaunchBox/BigBox navigation + the
  Pause Screen ONLY** — **it does not configure controllers for the
  emulators** (the one exception: the RetroArch plugin can set "basic
  controls"). In-emulator input is the emulator's job.
- **Mappings are GLOBAL across all controllers, not per-device** — a
  long-standing top feature request (button-1 means different physical buttons
  across Xbox/Switch/DualShock; users resort to reWASD/X360CE).
- **Exit-combo confusion** is a recurring support sink ("Exit" closes BigBox,
  not the game; you need Pause Screen → Exit Emulator or an AHK soft-close).
- Steam's controller layer re-enumerating XInput devices breaks controllers
  "only when launched via LaunchBox."

**OA mapping — a structural win OA is uniquely positioned for:** because OA
**loads libretro cores directly**, it can **map shell navigation AND in-game
input from one config** — something *none* of LaunchBox/ES-DE/Playnite do
cleanly (they all only map the frontend UI; in-game is the emulator's
problem). OA already has: HID HAT-axis decoding for oddball pads (memory
`reference_hid_hat_axis_decoding`), the controller-nav verb/binding
indirection (theming S1 `navBindings.ts`), and the **NavRemapCard** gamepad
rebinding UI (theming S5 D30). The dynamic-controller-info +
dynamic-input-descriptors plans (`PLANS/dynamic-controller-info.md`,
`PLANS/dynamic-input-descriptors.md`) are the home for **per-device** mapping
(LaunchBox's #1 unmet input request) and unified shell+gameplay binding.
*Recommendation: make per-device controller profiles + auto-config (SDL
`gamecontrollerdb`-style) a first-class goal there.*

---

## 4. Library viewing & metadata editing — OA's biggest greenfield to win

This is where LaunchBox is most *interaction*-weak and where **OA currently
has no editing UI at all** → the cleanest opportunity in the whole report.

### 4.1 The Edit Game dialog (the field model to match — high-confidence)
Backed by the official `IGame` plugin interface. Tabs (field groups
high-confidence; tab groupings medium): **Details** (Title, Sort Title,
Release Date/Year, Release Type [Released/DLC/Homebrew/ROM Hack/Unlicensed/
Unreleased], Developer(s), Publisher(s), Genre(s), Series, Region, Play
Mode(s) [Single/Multiplayer/Cooperative], Version, Source, Max Players,
ESRB/PEGI Rating, Star Rating [personal] vs **Community Star Rating**, Notes/
Overview, Wikipedia URL+ID, Video URL); status booleans (Completed/Favorite/
**Hidden**/Broken/Installed) + **Game Progress**; auto-tracked read-only stats
(Play Count, Play Time, Last Played, Date Added/Modified). **Emulation tab**
(Application Path, per-game Emulator override, Use Custom Command-Line Params,
DOSBox/ScummVM sub-fields, per-game startup-screen overrides). **Additional
Apps** (variants, each with Launch/Metadata/Saves sub-tabs + Make Default +
runtime Play Version). **Media** tabs (Images/Videos/Manuals/Music).
**Custom Fields** (§4.3). **Previous/Next** buttons cycle the list.

**OA mapping:** use this as the **field checklist** for OA's editing surface.
OA's metadata already lives in **MediaDb** (memory
`reference_metadata_lives_in_mediadb_not_games_table` — year/genre/developer/
publisher come from `useMedia().media(romId)?.metadata`, *not* the games
table). An OA Edit surface writes to MediaDb; the per-game emulator override
goes to the per-game settings tier.

### 4.2 The interaction pain points to engineer AROUND (ranked by echo)
These are LaunchBox's loudest editing complaints — OA's editing surface should
be designed as their inverse:
1. **No inline/spreadsheet editing** — editing is modal, per-game. The #1
   request for years. → **OA: ship inline list-view editing.**
2. **Edit dialog opens with 15–20s lag** (staff-localized to GameDetails view
   clearing) + selecting a game fires *hundreds* of redundant
   `Directory.GetFiles` scans. → **OA: instant selection feedback** (SQLite +
   no per-select disk scans).
3. **Bulk edit OVERWRITES, doesn't merge** (apply genres → all selected get the
   *same* set; no per-game merge; no find-and-replace). → **OA: merge-mode
   bulk edit (add/remove) + find-and-replace across the full schema.**
4. **No undo stack** — recovery from a bad edit is backup-restore only; XML
   corruption threads are common. → **OA: a real per-edit undo stack** (SQLite
   transactions make this cheap).
5. **Wrong matches are sticky** — clearing the DB ID and re-searching often
   returns the same wrong result; Steam remakes mis-match for 20+ months. →
   **OA recommendation #3: a first-class "fix wrong match" flow** — search/
   select the correct identity directly (OA's hash identity + variant model
   makes "pick the right canonical game" natural).

### 4.3 Custom fields (all string, all Premium-gated, can't be columns)
Name+Value pairs, **but: Premium-only to save, string-only (no typed/bool/
numeric/dropdown), can't be shown as sortable List View columns, no
platform-level custom fields, multi-value broken (semicolons treated as one
literal).** → **OA: typed, sortable, filterable, free custom fields as
first-class columns** is an easy, clear win.

### 4.4 Views / sorting / filtering / search (harvest the search syntax)
- Desktop has only **Images View** + **List View** + a Game Details panel.
  **List View columns can't be hidden, reordered persistently, or inline-
  edited; custom fields can't be columns; layout resets on restart** (dev-
  confirmed). View mode historically doesn't persist across restart.
- **Sorting:** column-click OR **View > Arrange By** (one field at a time, no
  multi-field, no sub-sort within groups).
- **Sidebar:** a dropdown switches the facet (Platform/Playlist/Region/Star
  Rating/Completed/Favorite/Genre/Series/Developer/Publisher/Play Mode); **not
  user-customizable** (open request).
- **Advanced Search syntax** (well-designed — worth copying): stackable
  switches, `-` excludes — `all: broken: dev: fav: genre: hide: installed:
  notes: plat: play: portable: progress: pub: rating: region: series: source:
  status: store: title:`. Default search hits titles + alternate titles only.

**OA mapping:** OA's Retroverse already has richer browsing (custom
collections shipped — memory `project_current_state`). **Harvest the search-
switch grammar** as a model for OA's search. **Win on the column system**
(configurable, persistent, custom-field-aware, inline-editable) since that's
LaunchBox's explicit dev-acknowledged weakness.

### 4.5 Media management UX (the two cited frustrations)
- **47 image types** (the same ImageTypes taxonomy as BigBox — Box Front/Back/
  Spine/Full/3D, Cart, Disc, ClearLogo, Banner, 5 screenshot sub-slots, arcade
  set, fanart family, storefront art). Multiple same-type images kept as
  `-01/-02`.
- **Biggest UX weakness:** **there is NO picker to choose which same-type
  image is the default/displayed one** — official guidance is literally
  "delete the others." → **OA recommendation #5: a "set as default image"
  picker per type/region.**
- **Region priority** (`Options > Data > Region Priorities`, "World" always
  included) governs which regional art shows — but the game's own Region is
  force-prioritized even when unchecked, so **users rename files** to force
  PAL art. → OA: make the default-image choice explicit and per-game, not an
  implicit region-priority side effect.
- **Manuals: no easy post-import bulk download.** **Music** per-game.

**OA mapping:** OA's media URL helpers already return converted asset URLs
(memory `reference_media_url_helpers_already_converted`); the slot taxonomy
maps onto OA's MediaDb. The default-image picker is a small, high-praise
feature.

---

## 5. General features & gaps (the non-import/edit surface)

LaunchBox has aggressively pulled community features native over ~2 years
(RetroAchievements, Save Management, Game Progress, cloud stat sync, the
RetroArch plugin). Inventory + OA-relevant gaps:

- **Backups:** library XML auto-backed-up on every start/shutdown (up to 25
  retained); **Save Management** vault (per-game save/state versioning,
  backup-on-close, periodic backup, max-per-game cap; folder saves archived as
  .7z) for RetroArch/Dolphin/PCSX2. → OA has `oa-savestate`; the **save-vault
  versioning UX** is worth copying.
- **RetroAchievements:** native, multi-emulator (RetroArch/Dolphin/PCSX2),
  Retro Score widget, per-game achievement list, Time-to-Beat/Master,
  auto-progress. → strong; OA has RA "actually-open" per
  `project_current_state`. Match it.
- **Plugins:** official .NET Plugin API (`ISystemMenuItemPlugin`,
  `IGameMenuItemPlugin`, `IGameLaunchingPlugin`, `IBigBoxThemeElementPlugin`,
  etc.). → OA's posture is sandboxed (Rhai/WGSL), not native DLLs — better for
  a public ecosystem.
- **Netplay:** a RetroArch-backed "Netplay Lobby," widely reported **fragile**
  (ROM-hash mismatch failures) and RetroArch-only. → parked for OA (PARKING_LOT
  territory); a soft target if ever pursued.
- **CLI / external launch:** **no native CLI** to launch a game (open request;
  only a community "CLI Launcher" plugin). → OA could offer a first-class
  launch IPC/CLI as a differentiator (Stream Deck / automation).
- **Cloud sync:** **only thin stat sync** (play time/count/favorites/progress
  via a LBGDB account); **metadata/playlists/saves/settings are NOT synced** —
  both are open requests. → genuine gap; OA could win here but it's a big
  feature (park unless prioritized).
- **Multi-user / profiles:** **none** — repeatedly requested. → gap; per-profile
  stats/progress is a differentiator (note OA's `user` memory motivation is a
  community gift, so multi-user-per-household has thematic fit).
- **Portable:** LaunchBox is inherently portable (everything beside the exe).
  → OA has the `portable.txt` marker model (`features/portable-install/`).
- **Stats:** inline per-game only; **no analytics/dashboard view**. → small
  differentiator opportunity.

---

## 6. Free vs paid — positioning OA against the model

- LaunchBox **desktop is free forever** (full library management, unlimited
  games, scraping, emulator launching). **BigBox (the big-screen/couch/cabinet
  experience) + theming + startup/pause/shutdown screens + bezels + custom
  categories + controller automation are Premium.**
- **Premium pricing (Windows, USD, one-time NOT subscription):** Regular **$30**
  (1 yr of updates, then optional **$15/yr** to keep getting *new* features —
  software keeps working if you lapse) / **Forever-Updates "lifetime" $75**.
  Frequent 50%-off sales ($15 / $45). DRM-free, multi-PC. Big Box is **included
  in Premium, not a separate SKU**. (Android is a separate cheaper license.)
- **Community sentiment:** the model is broadly seen as **fair**; the one
  recurring friction is that **BigBox — the headline reason people want
  LaunchBox — is the paywalled part**, so the free tier "feels like just a
  manager."

**OA positioning:** OA's **non-commercial GPLv2 gift** model (memory
`user_project_context`) is strictly more generous and is a clean marketing
wedge — *the polished couch/per-system-themed experience that LaunchBox
charges for is free in OA.* Competitor research confirms "free + open" is the
**#1 cited reason people leave LaunchBox** (for Playnite/ES-DE/Batocera). No
action — just a positioning note for whenever OA has public-facing copy.

---

## 7. Community pain points → OA's structural answers (the strategic core)

Ranked by recurrence across the LaunchBox forums + feedback board (Reddit was
not directly fetchable; forums carry the weight and are where paying users
vent):

| LaunchBox pain (STRONG RECURRING) | OA's structural answer |
| --- | --- |
| **Performance at 10k+ games** — the #1 reputation wound, 2015→2026, founder-acknowledged, architectural (WPF + monolithic XML in RAM). 22k games → 15s menus; 49k → 1-min startup; RTX 3080 + i9 still lags; memory leaks to 13GB | **Rust + wgpu + SQLite, indexed/lazy** — instant at 50k+ is category-defining and they can't fix it fast |
| **Windows-only** (66-vote Linux req; "runs like dogshit" on Deck-via-Windows; dev declined the rewrite) | **wgpu → Vulkan/Metal/GL**, cross-platform by construction; **Steam Deck is the single highest-leverage platform to court** |
| **Import matching wrong / can't fix** | **5-tier hash ID** + a planned **fix-wrong-match flow** (§4.2) |
| **Setup = too much manual work** (the real benchmark is RetroBat/ES-DE/Batocera, NOT LaunchBox) | **Ship own cores** (no core-association trap) + **guided-setup smart-scan** + **one-pass multi-system import** (§2.1) |
| **Bulk edit weak / UI clunky / no undo** | **Greenfield editing surface** designed as the inverse (§4.2) |
| **Smart-playlist logic too weak (AND-only)** | **Real boolean query builder** on SQLite (§2.4) |
| **No cross-machine sync, no multi-user** | open gaps OA *can* win (park unless prioritized) |

**The cross-cutting read:** LaunchBox's deepest weaknesses are structural and
slow-to-fix (one dev, WPF, monolithic XML). OA's stack neutralizes the top
two (performance, cross-platform) *for free*, and OA's existing foundation
(hash ID, SQLite, guided setup, own cores) neutralizes the next two. The
remaining work is **UX depth on surfaces OA hasn't built yet** (editing,
one-pass import, boolean collections, media picker).

---

## 8. The real ease-of-setup benchmark — don't beat LaunchBox, beat these

A critical reframing from the competitor agent: **LaunchBox is the HARD
option; RetroBat / ES-DE / Batocera are the "easy setup" benchmarks.** To win
on setup, OA must beat *those*, not LaunchBox:
- **RetroBat:** one installer, **RetroArch + cores bundled**, other emulators
  auto-download on first need, **all config from inside the frontend**, 4
  built-in scrapers (ScreenScraper/TheGamesDB/HFSDB/ArcadeDB), drop-folder ROM
  detection. This is the entire "easier than LaunchBox" reputation.
- **ES-DE / Batocera:** drop ROMs into `roms/<system>/` → **system tab
  auto-appears**; hash-based ScreenScraper; SDL `gamecontrollerdb` auto-config;
  Batocera is a **whole bootable OS** (plug-and-play floor OA can't match, and
  needn't).
- **Playnite:** **metadata-as-provider-plugins** (per-field: cover from
  SteamGridDB, description from IGDB, in one pass) + **emulator-definitions-as-
  YAML** auto-fill. Both patterns worth adopting.
- **Skraper / ARRM:** dedicated **hash-first multi-source** scrapers with
  **"scrape only games missing X"** batch filters + local media cache to dodge
  quota throttling. ARRM aggregates 10 sources. OA should adopt: hash-first,
  multi-source per-field, scrape-only-missing, local cache/queue.

**OA's winning combo (already half-built):** own cores (RetroBat's bundled-
RetroArch win, but for *every* system and no licensing constraint) + drop-
folder one-pass detection + hash-first multi-source scraping +
in-frontend everything + cross-platform. That's a setup story better than any
single competitor.

---

## 9. Consolidated recommendations (mapped to OA arcs, by leverage)

1. **[guided-setup Phase 2C + virtual-library scan] One-pass multi-system
   import** — scan a whole ROM tree, classify every platform at once, auto-
   reveal systems; wizard becomes the beginner on-ramp, not the only path.
   Beats LaunchBox *and* the RetroBat/ES-DE/Batocera benchmark.
2. **[NEW surface — biggest greenfield] A first-class metadata-editing UI** —
   inline list-view editing, instant selection, merge-mode bulk edit +
   find-and-replace, a real undo stack, and a **"fix wrong match" flow**. Use
   the §4.1 `IGame` field list as the checklist; write to MediaDb. Designed as
   the inverse of LaunchBox's §4.2 pain points.
3. **[virtual-library schema] Real boolean smart-collections** (AND/OR/NOT +
   nested groups + count limits) — LaunchBox explicitly won't; trivial on
   SQLite.
4. **[dynamic-input-descriptors / dynamic-controller-info] Per-device
   controller profiles + unified shell+gameplay binding + auto-config** —
   LaunchBox's #1 unmet input request, and OA is uniquely able (direct core
   loading) to map shell AND in-game from one config.
5. **[MediaDb / library] Media polish:** a **"set as default image" picker**
   per type/region + a **"download only missing media"** mode + DAT-aware
   "what am I missing from the full set" audit. Three cited LaunchBox gaps.
6. **[launcher-abstraction] Emulator-definitions-as-data** (`config/emulators/
   <id>.yaml`, Playnite-shaped) for external standalone emulators, with
   per-system (not per-emulator) archive-extraction and prominent, bulk-capable
   per-game overrides.
7. **[settings] Harvest LaunchBox's good vocabularies:** the Advanced Search
   switch grammar (§4.4), the Game Progress Categories→Values taxonomy (§1),
   the command-line variable set (§3.1), the save-vault versioning UX (§5).
8. **[positioning, later] Lead with cross-platform (Steam Deck) + free/open +
   "cores just work"** — the three places LaunchBox structurally can't follow.

---

## 10. Open questions for the operator

1. ~~**Managed media tree vs. beside-the-ROM convention?**~~ **RESOLVED
   2026-06-11 — Option B (beside-the-ROM convention) + relative-path
   portability.** Operator chose the convention layout for free Skraper/ARRM
   interop + cross-OS/drive portability (desktop ↔ cabinet); MediaDb becomes
   the resolution/override/canonical layer over a convention-discovered base
   (hybrid: convention first, managed cache second). Forces **relative paths
   everywhere** in the SQLite library. Full decision recorded as
   `PLANS/virtual-library-and-launcher-arc.md` **S9** (+ open sub-decisions
   S9a: exact dialect, read-vs-write, casing).
2. ~~**Is a metadata-editing surface in scope soon, or parked?**~~
   **RESOLVED 2026-06-11 — its own arc, soon.** Gets a dedicated arc at the
   next pause point (not folded into another arc); it's OA's biggest
   interaction win over LaunchBox and deserves focused design. Field
   checklist = §4.1; design as the inverse of the §4.2 pain points; writes to
   MediaDb / `game_identities`.
3. ~~**How far into "general features" does OA want to go?**~~ **RESOLVED
   2026-06-11.** Priority 1 = **external-emulator integration + auto-config**
   (launch-and-return baseline + deep per-emulator profiles that install /
   auto-configure / take over settings / manage saves — NOT window-embedding;
   recorded as arc **S10**). Priority 2 = **multi-user profiles** (expand OA's
   existing profile support). **Parked:** cross-machine sync, stats/analytics
   dashboard, netplay (RetroArch-fragile). *(The original option I'd framed as
   "CLI/launch-IPC" — launching OA from outside — was set aside in favor of
   the external-emulator control substance.)*
4. ~~**Scraping sources / provider model?**~~ **DEFERRED 2026-06-11 — leaning
   curated/self-hosted.** Operator undecided; leaning toward **curated lists +
   a custom external scraper tool, self-hosted on a git repo** (the
   libretro-database / libretro-thumbnails model OA already consumes — memory
   `reference_libretro_thumbnails_uses_git_symlinks`) rather than a live
   multi-source scraper. Pairs cleanly with the Option-B beside-the-ROM layout
   (a curated dataset drops into the convention). Decision left open; see arc
   **S9a**.

---

## Appendix — research provenance & caveats

Four primary agents + six sub-agents (2026-06-11), each URL-cited in the
session transcript:
- Import & library setup · Emulator & controller setup · Library viewing/
  editing · Settings tree · Free-vs-paid · Onboarding · Notable features ·
  Community pain points · Competitor setup/management.

Sourcing caveats (carried from agents): **Reddit was not directly fetchable**
(Anthropic UA blocked) — the LaunchBox official forums + feedback board carry
the weight, which is arguably *better* signal (actual paying users). Several
official pages and a few tutorial/changelog pages **403'd to automated
fetch** — affected claims are corroborated via search snippets and flagged
in the raw outputs. **Edit-dialog tab groupings and the full Options left-tree
ordering are medium-confidence** (LaunchBox publishes no screenshot-level
reference); the *field model* is high-confidence (official `IGame` /
`ImageTypes` plugin API). Pricing rests on search snippets + deal-tracker
corroboration (direct pricing-page fetch was blocked) — verify exact current
figures before any public use.
