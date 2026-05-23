# Media taxonomy — full LaunchBox-shaped art + audio + video + manual storage

> **Status: 📐 PLANNED, not implemented.** Design locked 2026-05-23 in
> conversation with operator. Implementation deferred — listed under
> `docs/NEXT.md` MEDIUM band. Pick up by reading this doc + the original
> approved plan at `C:\Users\Devilchi\.claude\plans\groovy-enchanting-candle.md`.

## Context

OA today supports five game-art slots (`boxart`, `snap`, `title`, `cart`,
`disc`) stored at `<data_dir>/media/covers/<systemId>/<romId>.<ext>` and
`<data_dir>/media/thumbs/<systemId>/<sha1[..16]>.webp`. The `romId` is a
djb2 path-hash like `rom-1k3jq9z` — totally opaque to a human browsing
the folder, which makes:

1. **Drag-drop of art packs** (LaunchBox / EmuMovies) impossible —
   their files arrive as `Sonic the Hedgehog (USA).png`, mismatched
   against our `rom-1k3jq9z.png` convention.
2. **Pruning unwanted art** require querying the JSON DB to figure out
   which file is which.
3. **Spot-checking art coverage** via Explorer impossible without
   round-tripping through the app.

In parallel, the [kiosk-shell plan](../kiosk-shell/KIOSK_PLAN.md)
commits to a 5-bus audio mixer with platform music, snap audio, live
game audio, UI sounds, and announce/ceremony — but the storage layout
for any of that wasn't yet designed.

**Trigger:** operator surfaced the naming problem while reviewing the
portable-install storage tour, and we used the opportunity to design
the full media taxonomy in one pass (game art + audio + video + manuals
+ per-system theme assets) so all of it fits one consistent model.

**Outcome:** A LaunchBox-shape folder tree under `<data_dir>/media/`
that operators can populate by drag-dropping existing art packs or by
the in-app importer / libretro-thumbnails sync. Naming convention is
rom-filename-stem (matches LaunchBox / EmuMovies / EmulationStation).
The full ~25-slot taxonomy ships at the data-model + folder level from
day one; UI catches up incrementally. Audio overrides ride the
existing 3-tier settings cascade (no new folder tree); theme content
stays inside `.oatheme` packages per the kiosk plan.

## Locked design decisions

(All decided in conversation with operator 2026-05-23.)

| # | Decision |
|---|---|
| 1 | **System-first** folder: `media/<systemId>/<kind>/<rom_stem>.ext` (matches LaunchBox; "manage one system at a time" workflow) |
| 2 | **Full ~25-slot LaunchBox taxonomy** in data model + importer from day one. UI renders the 5 used slots; the other ~20 sit on disk until UI catches up |
| 3 | **`-NN` suffix** for multi-variant art per rom (primary = `Sonic.png`, variant 2 = `Sonic-02.png`). `MediaVariant.region` carries the metadata; suffix is just an ordering hint |
| 4 | **Audio overrides ride the existing settings cascade.** New `Option<PathBuf>` fields on `SystemSettings` + `GameOverrides`. No new folder tree. Per-game override is free |
| 5 | **Desktop UI silent by default**; opt-in via per-sound settings paths (same cascade pattern as #4). Kiosk stays theme-driven |
| 6 | **Audio**: universal via Symphonia (.ogg, .opus, .mp3, .flac, .wav, .m4a). **Video**: HTML5-native (MP4 H.264, WebM VP9) — no new deps. **Images**: PNG + JPG + WebP (current) |
| 7 | **Manuals**: PDF rendered inline in WebView2; accept `.epub`, `.cbz`, `.cbr`, `.txt`, `.md` and open via OS shell |
| 8 | **libretro-thumbnails sync mapping**: `Named_Boxarts → box-front`, `Named_Snaps → screenshot-gameplay`, `Named_Titles → screenshot-title`. **Operator's manual art always wins** — sync only fills empty slots |

## Folder layout

```
<data_dir>/
  media/                                          ← OPERATOR-OWNED (theme-independent)
    <systemId>/                                   ← e.g. "genesis", "snes", "tg16"
      box-front/<rom_stem>.{png,jpg,webp}         ← LaunchBox "Box - Front"
      box-back/<rom_stem>.{png,jpg,webp}          ← "Box - Back"
      box-3d/<rom_stem>.{png,jpg,webp}            ← "Box - 3D"
      box-spine/<rom_stem>.{png,jpg,webp}         ← "Box - Spine"
      box-full/<rom_stem>.{png,jpg,webp}          ← "Box - Full" (unfolded)
      cart-front/<rom_stem>.{png,jpg,webp}        ← "Cart - Front"
      cart-back/<rom_stem>.{png,jpg,webp}         ← "Cart - Back"
      cart-3d/<rom_stem>.{png,jpg,webp}           ← "Cart - 3D"
      disc/<rom_stem>.{png,jpg,webp}              ← "Disc"
      screenshot-gameplay/<rom_stem>.{png,jpg,webp} ← "Screenshot - Gameplay"
      screenshot-title/<rom_stem>.{png,jpg,webp}    ← "Screenshot - Game Title"
      screenshot-select/<rom_stem>.{png,jpg,webp}   ← "Screenshot - Game Select"
      banner/<rom_stem>.{png,jpg,webp}            ← "Banner"
      clear-logo/<rom_stem>.png                   ← "Clear Logo" (transparency)
      fanart-background/<rom_stem>.{jpg,png}      ← "Fanart - Background"
      fanart-disc/<rom_stem>.png                  ← "Fanart - Disc"
      advert-front/<rom_stem>.{png,jpg}           ← "Advertisement Flyer - Front"
      advert-back/<rom_stem>.{png,jpg}            ← "Advertisement Flyer - Back"
      arcade-cabinet/<rom_stem>.{png,jpg}         ← arcade-only systems
      arcade-marquee/<rom_stem>.{png,jpg}
      arcade-controlpanel/<rom_stem>.{png,jpg}
      arcade-controlsinfo/<rom_stem>.{png,jpg}
      arcade-playerselect/<rom_stem>.{png,jpg}
      arcade-flyer/<rom_stem>.{png,jpg}
      video/<rom_stem>.{mp4,webm}                 ← per-game preview video
      music/<rom_stem>.{ogg,opus,mp3,flac,wav,m4a} ← per-game music preview
      manual/<rom_stem>.{pdf,epub,cbz,cbr,txt,md}

    platform/                                     ← per-SYSTEM, not per-rom
      banner/<systemId>.{png,jpg}
      clear-logo/<systemId>.png
      console/<systemId>.{png,jpg}                ← hardware photo
      controller/<systemId>.{png,jpg}
      fanart/<systemId>.{jpg,png}
      marquee/<systemId>.{png,jpg}
      photo/<systemId>.{jpg,png}                  ← real-world system photo
      wheel/<systemId>.png                        ← transparent, for wheel UI
      background/<systemId>.{jpg,png}

    thumbs/<systemId>/<sha1[..16]>.webp           ← derived/cache; unchanged
    cache/libretro-thumbnails/<systemId>/<Named_Foo>/<original-filename>.png
                                                  ← upstream sync cache; unchanged
    cache/index-<systemId>.v2.json                ← per-system tree cache; unchanged

  themes/                                         ← .oatheme content (kiosk plan)
    <theme-id>/...                                ← future, not built here

  # Audio overrides do NOT get a folder tree — they're path-references
  # stored in SystemSettings + GameOverrides JSON files.
  # Example: SystemSettings("genesis").platform_music_path = Some("D:/Music/Genesis Theme.ogg")
```

## Data model changes

### `MediaVariant` (in `apps/oa-shell/src/media.rs`)

No shape change — `MediaVariant { source, region, path, thumb_path, width, height, sha1, bytes }`
already carries everything we need. The `path` field starts containing
the new layout: `media/genesis/box-front/Sonic the Hedgehog (USA).png`.

### `GameMedia` (in `apps/oa-shell/src/media.rs`)

Extend the existing 5-field struct (`boxart`, `snap`, `title`, `cart`,
`disc`) with the full ~25-field taxonomy. Every new field is
`Vec<MediaVariant>` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`
so old `media.json` files parse forward with new fields defaulting to
empty.

```rust
pub struct GameMedia {
    // Existing v1 fields — renamed semantically to map to LaunchBox slots
    pub box_front: Vec<MediaVariant>,             // was: boxart
    pub screenshot_gameplay: Vec<MediaVariant>,   // was: snap
    pub screenshot_title: Vec<MediaVariant>,      // was: title
    pub cart_front: Vec<MediaVariant>,            // was: cart
    pub disc: Vec<MediaVariant>,                  // unchanged
    // New slots (all default-empty)
    pub box_back: Vec<MediaVariant>,
    pub box_3d: Vec<MediaVariant>,
    pub box_spine: Vec<MediaVariant>,
    pub box_full: Vec<MediaVariant>,
    pub cart_back: Vec<MediaVariant>,
    pub cart_3d: Vec<MediaVariant>,
    pub screenshot_select: Vec<MediaVariant>,
    pub banner: Vec<MediaVariant>,
    pub clear_logo: Vec<MediaVariant>,
    pub fanart_background: Vec<MediaVariant>,
    pub fanart_disc: Vec<MediaVariant>,
    pub advert_front: Vec<MediaVariant>,
    pub advert_back: Vec<MediaVariant>,
    pub arcade_cabinet: Vec<MediaVariant>,
    pub arcade_marquee: Vec<MediaVariant>,
    pub arcade_controlpanel: Vec<MediaVariant>,
    pub arcade_controlsinfo: Vec<MediaVariant>,
    pub arcade_playerselect: Vec<MediaVariant>,
    pub arcade_flyer: Vec<MediaVariant>,
    pub video: Vec<MediaVariant>,
    pub music: Vec<MediaVariant>,
    pub manual: Vec<MediaVariant>,
    // Existing
    pub selected: Option<SelectedMedia>,
    pub metadata: Option<GameMetadata>,
}
```

`SelectedMedia` extends from 3 to 25+ optional `usize` indexes (one
per slot), all `#[serde(default, skip_serializing_if = "Option::is_none")]`.

### `MediaKind` enum

Bump from the current 5-variant enum to the full ~25-variant set. Keep
the serde rename so JSON uses kebab-case (`"box-front"`, not
`"BoxFront"`) — matches the folder names exactly.

### Platform (per-system) media — new struct

```rust
pub struct PlatformMedia {
    pub banner: Option<MediaVariant>,
    pub clear_logo: Option<MediaVariant>,
    pub console: Option<MediaVariant>,
    pub controller: Option<MediaVariant>,
    pub fanart: Option<MediaVariant>,
    pub marquee: Option<MediaVariant>,
    pub photo: Option<MediaVariant>,
    pub wheel: Option<MediaVariant>,
    pub background: Option<MediaVariant>,
}
```

`Option<MediaVariant>` not `Vec<>` because there's only one cover per
system (no region variants for the hardware itself). Stored in a new
`<data_dir>/library/platform-media.json` (sibling of `media.json`).

### `SystemSettings` audio override fields (in `apps/oa-shell/src/system_settings.rs`)

Add `Option<PathBuf>`-shaped fields following the existing pattern
(every field already optional + serde-default):

```rust
// Per-system platform music — the BGM that plays when this system is selected
// in kiosk. None = inherit theme default. Some(path) = play this file instead.
#[serde(skip_serializing_if = "Option::is_none")]
pub platform_music_path: Option<PathBuf>,

// Desktop UI sounds (silent by default; opt-in per-event)
#[serde(skip_serializing_if = "Option::is_none")] pub ui_sound_click: Option<PathBuf>,
#[serde(skip_serializing_if = "Option::is_none")] pub ui_sound_navigate: Option<PathBuf>,
#[serde(skip_serializing_if = "Option::is_none")] pub ui_sound_back: Option<PathBuf>,
#[serde(skip_serializing_if = "Option::is_none")] pub ui_sound_launch: Option<PathBuf>,
#[serde(skip_serializing_if = "Option::is_none")] pub ui_sound_error: Option<PathBuf>,
#[serde(skip_serializing_if = "Option::is_none")] pub ui_sound_scroll_tick: Option<PathBuf>,
```

The UI sound fields might be more naturally on a top-level
`UiSoundPrefs` struct (since they're not really per-system) — TBD
during implementation. The per-system version makes sense if operators
want "different UI sounds for the Game Boy than for the Saturn" which
might be cute but probably overkill.

### `GameOverrides` audio override fields (in `apps/oa-shell/src/library_db.rs`)

```rust
// Per-game platform music — plays when this game is highlighted in the
// library, overriding the per-system default. Killer BigBox feature.
#[serde(skip_serializing_if = "Option::is_none")]
pub platform_music_path: Option<PathBuf>,
```

## File-by-file change list

### Existing files to modify

- **`apps/oa-shell/src/media.rs`** (~600 lines of edits)
  - Rename `MediaKind` 5 variants → 25-variant enum with kebab-case serde
  - Extend `GameMedia` with new slot fields (default-empty)
  - Extend `SelectedMedia` with per-slot indices
  - Rewrite `ingest_manual_cover` → kind-aware version
    `ingest_manual_for_slot(..., kind: MediaKind)` that writes to
    `media/<systemId>/<kind>/<rom_stem>.<ext>`
  - Rewrite libretro-thumbnails sync's destination-path builder:
    `media/<systemId>/<kind>/<filename>` where `kind` is
    `box-front` / `screenshot-gameplay` / `screenshot-title`
  - Add "operator art wins" guard to the sync: check if `db.<slot>` is
    non-empty for this rom *with `source = Manual`* — if so, skip
  - Add `-NN` variant-suffix logic to the writer (when slot already
    has a primary, append `-02` / `-03` etc)
  - Add new commands: `set_manual_for_slot(rom_id, slot, source_path)`,
    `clear_media_slot(rom_id, slot)`, `set_platform_media(system_id, slot, source_path)`
  - Keep existing `set_manual_cover` as a thin wrapper that calls
    `set_manual_for_slot(rom_id, MediaKind::BoxFront, source_path)`

- **`apps/oa-shell/src/system_settings.rs`** (~30 lines added)
  - New optional fields for `platform_music_path` + 6 `ui_sound_*` paths
  - Existing tests + readers cover migration via `#[serde(default)]`

- **`apps/oa-shell/src/library_db.rs`** (~10 lines added)
  - New `platform_music_path: Option<PathBuf>` on `GameOverrides`
  - SQL schema unchanged (lives in the `overrides_json` blob column)

- **`apps/oa-shell/src/main.rs`** (~80 lines of edits)
  - Register new media commands in `tauri::generate_handler!`
  - Add new audio-playback service (Phase 4 — see phase plan): a
    cpal-backed player that takes a path + sound type, decodes via
    Symphonia, mixes into the right kiosk-plan bus
  - Listen for `oa://library-game-focused` events from the frontend
    to trigger `platform_music_path` swap (Phase 4)

- **`frontend/src/library/media.tsx`** (~150 lines of edits)
  - Extend `GameMedia` TypeScript type to mirror the new Rust shape
  - Extend `MediaKind` union type
  - `coverUrl()` helper learns to resolve any kind, not just `boxart`
  - Add new helpers: `boxartUrl()`, `snapUrl()`, `titleUrl()`,
    `clearLogoUrl()` (the v1 visible ones), all building on `coverUrl()`

- **`frontend/src/components/GameInfoModal.tsx`** (~40 lines)
  - Add tabs for the slots we surface: Box Front / Back / 3D, Cart,
    Disc, Screenshots (gameplay/title/select), Clear Logo, Manual,
    Video, Music
  - "Set art for this slot…" / "Clear" buttons per slot
  - v1 only wires box-front + screenshots + cart + disc (the 5 visible
    today); the other tabs can be stubs reading "Not yet rendered in
    UI — file accepted but won't display"

- **`frontend/src/components/SystemDialogs.tsx`** (~30 lines)
  - Add per-system platform-media slots (banner, clear-logo, console,
    controller, fanart, marquee, photo, wheel, background) — same
    "Set" / "Clear" affordances
  - Add per-system audio override fields (platform music path picker)

### New files

- **`apps/oa-shell/src/audio_player.rs`** (~250 lines, Phase 4)
  - cpal output stream + Symphonia decoder + small mixer
  - One stream per bus (platform-music, ui-sounds, ceremony, snap-audio)
  - Public API: `play_path(bus, path, loop_)`, `stop_bus(bus)`,
    `set_bus_volume(bus, gain)`
  - Hooks into existing `oa-audio` crate where possible (the
    `cpal`-backed game-audio stream pattern is already there)

- **`apps/oa-shell/src/art_pack_importer.rs`** (~200 lines, Phase 3)
  - Recursive importer for LaunchBox / EmuMovies folder structure
  - Maps `<source>/Sega Genesis/Box - Front/<game>.png` →
    `<data_dir>/media/genesis/box-front/<rom_stem>.png` for every rom
    that matches the source filename
  - LaunchBox platform-name → OA system_id mapping table (data file)
  - Fuzzy-match the import filename against library titles (reuses
    existing fuzzy match in `media.rs`)
  - Dry-run mode: report what WOULD be imported, no writes

- **`apps/oa-shell/src/data_dir.rs::migrate_media_naming`**
  (added to existing module, ~150 lines, Phase 5)
  - One-shot pass on launch: walk old `media/covers/<systemId>/`,
    look up each `rom-<hash>.ext` in library DB to find its filename
    stem, rename to `media/<systemId>/box-front/<rom_stem>.ext`
  - Same for thumbs (no rename — thumbs stay content-addressed)
  - Walk `media.json` and rewrite `path` strings to new layout
  - Sentinel `.media-taxonomy-migrated` in `<data_dir>` so it only
    runs once

## Phase plan

1. **Phase 1 — Data model + folder layout (~400 lines)**
   - Extend `MediaKind` enum + `GameMedia` struct + `MediaVariant.path`
     interpretation
   - Update writers to use new `<systemId>/<kind>/<rom_stem>.<ext>` shape
   - Backfill the v1 5-slot semantic rename
     (`boxart→box_front`, `snap→screenshot_gameplay`,
     `title→screenshot_title`, `cart→cart_front`)
   - Migration of in-memory `MediaDb` is transparent — `#[serde(default)]`
     handles missing fields, old serialized-name fallback parses legacy
     keys for one release cycle then can be deleted
   - Unit tests for path computation + variant suffix logic

2. **Phase 2 — libretro-thumbnails sync update (~150 lines)**
   - Switch destination path builder to new layout
   - "Operator art wins" guard (check `source == Manual` before
     overwriting)
   - Update `MediaKind` mappings: `Named_Boxarts → BoxFront`,
     `Named_Snaps → ScreenshotGameplay`,
     `Named_Titles → ScreenshotTitle`
   - Existing per-kind batch flush logic stays as-is
   - Tests: existing-manual-not-clobbered, new-sync-fills-empty,
     `-NN` suffix appended when both manual + sync present

3. **Phase 3 — Art-pack importer (~200 lines)**
   - New `art_pack_importer.rs` + Tauri command
   - LaunchBox platform-name → system_id static mapping
   - Recursive scan of source dir, fuzzy filename→library-title match
   - Dry-run mode + progress reporting via existing scan-progress event
   - Library Manager UI button: "Import art pack from folder…"
     (`ImportArtPackDialog.tsx`)

4. **Phase 4 — Audio playback service (~300 lines)**
   - New `audio_player.rs` — bus-shaped mixer over cpal + Symphonia
   - Hook into kiosk plan's 5-bus model (platform-music, ui-sounds,
     ceremony, snap-audio; live-game-audio stays on the existing
     `oa-audio` pipe)
   - Tauri commands: `play_audio(bus, path, loop_)`, `stop_audio(bus)`,
     `set_audio_volume(bus, gain)`
   - Frontend: `platform_music_path` resolution on library focus,
     UI sound dispatch on event (click/navigate/back/launch/error)
   - Defer ducking matrix (Phase 4.5 stretch)

5. **Phase 5 — Existing-install migration (~150 lines)**
   - `data_dir::migrate_media_naming` one-shot pass on launch
   - Sentinel guard like portable-install's `.migrated-from-appdata`
   - Rewrites `media.json` paths + renames files on disk
   - Tests: missing-old-layout no-ops, mixed-old-and-new tolerated,
     re-run-after-success no-ops

6. **Phase 6 — Platform-media + per-system UI (~250 lines)**
   - `platform-media.json` + `PlatformMedia` struct
   - System page surfaces the 9 platform slots in `SystemDialogs.tsx`
   - System wheel UI (kiosk-flavored, prep work for kiosk shell) can
     consume `wheel/<systemId>.png` if present, fall back to current
     gradient logo otherwise

7. **Phase 7 — Docs + SESSION_LOG (~30 lines)**
   - This README becomes the historical reference
   - `SESSION_LOG.md` records the ship per phase
   - CLAUDE.md no change — folder layout doc lives under this feature

## Critical files to reference

- **`apps/oa-shell/src/media.rs:1062`** — `ingest_manual_cover` (the
  function to refactor into kind-aware shape)
- **`apps/oa-shell/src/media.rs:1631`** — `sync_single_rom`
  destination-path builder (libretro-thumbnails sync)
- **`apps/oa-shell/src/media.rs:1083`** — thumbnail path builder
  (unchanged but referenced by new writers)
- **`apps/oa-shell/src/system_settings.rs:24`** — `SystemSettings`
  struct (the audio-override extension point)
- **`apps/oa-shell/src/library_db.rs:35`** — `GameOverrides` struct
  (per-game audio override point)
- **`frontend/src/library/media.tsx:37`** — `MediaVariant` /
  `GameMedia` TypeScript types (mirror Rust changes)
- **`frontend/src/components/GameInfoModal.tsx`** — UI surface for
  per-game art slots (extend tabs)
- **`frontend/src/components/SystemDialogs.tsx`** — UI surface for
  per-system platform media + audio overrides

## Reuse / existing patterns

- **3-tier settings cascade** — `SystemSettings` + `GameOverrides`
  are the OA-wide → per-system → per-game pattern; audio overrides
  slot in identically. No new persistence to design.
- **`MediaVariant.region` + RegionPicker** — already shipped; the
  `-NN` variant suffix is just a filename convention, RegionPicker
  drives the actual display choice.
- **Fuzzy match for libretro-thumbnails sync** — reuse the same
  scoring for art-pack importer's filename → library-title match.
- **Per-kind batch flush** in libretro-thumbnails sync — already
  groups `media.json` writes per (repo × kind); extends cleanly to
  the new ~25 kinds.
- **Content-addressed thumbs** — keep as-is. Derived/regeneratable,
  dedup across region clones, users don't browse them.
- **Settings UI per-system page** — already exists from slice 2.8.C;
  add a new "Media" section with the platform slots + audio override
  fields.

## Verification (when implementing)

End-to-end on Windows:

1. **Pure data-model migration (no operator action)**
   - Old `media.json` with `boxart` / `snap` / `title` / `cart` /
     `disc` keys deserializes into the new struct via serde alias /
     compat layer.
   - Existing thumbnail rendering keeps working (paths haven't moved).
   - `cargo test -p oa-shell media` clean.

2. **Phase 5 file-on-disk migration**
   - Pre-condition: an install with `media/covers/<systemId>/rom-1k3jq9z.png` files.
   - First launch after upgrade: log shows "migrating media naming",
     file count, success.
   - Post-condition: files now at
     `media/<systemId>/box-front/<rom_stem>.png`; `media.json` paths
     rewritten; sentinel `.media-taxonomy-migrated` present.
   - Second launch: no-op, sentinel guards.

3. **Art-pack importer**
   - Operator points at a LaunchBox/EmuMovies "Sega Genesis" art
     pack folder.
   - Dry-run reports: "Would import 47 box-front, 32 clear-logo,
     12 fanart-background, 8 manual"; no skipped.
   - Real run lands files at correct paths; library refreshes; cover
     tiles update without restart.

4. **libretro-thumbnails sync with operator art present**
   - Operator manually sets box-front for one game.
   - Sync runs for that system.
   - Operator's manual file untouched; sync fills other empty slots
     for that game.
   - Sync metadata: 1 game skipped with reason "manual present"
     visible in the sync result toast.

5. **Per-game music override end-to-end**
   - Operator sets `GameOverrides.platform_music_path` on
     "Sonic the Hedgehog" pointing at a custom OGG.
   - In the library, focus moves to Sonic → custom track plays.
   - Focus moves to another Genesis game → falls back to
     `SystemSettings("genesis").platform_music_path`, or silence.

6. **Per-system UI sound override**
   - Operator sets `SystemSettings("snes").ui_sound_click =
     Some("snes-click.ogg")`.
   - Clicking in the SNES library page plays the custom sound.
   - Other system pages: silence (no override set).

7. **`cargo test --workspace`** — target ≥ 30 new unit tests across
   media path computation, variant suffix logic, importer fuzzy
   match, migration idempotence, audio bus mixing math.

## Out of scope (deferred)

- **MAME romset variant handling** — MAME ROMs are zipped sets, not
  single files; their "rom_stem" is the zip basename (`smk.zip` →
  `smk`). The naming convention works; no special case needed.
- **CHD-based CD games** — same as MAME; the .chd's basename is the
  rom_stem. Multi-disc games with `.m3u` use the .m3u basename.
- **`.cue+.bin` multi-file sets** — the .cue is canonical; rom_stem
  is the .cue's basename.
- **Theme `.oatheme` package support** — kiosk plan, separate work
  stream.
- **Audio ducking matrix** — implementing the kiosk plan's per-bus
  ducking is Phase 4.5 stretch; Phase 4 ships unducked.
- **Phone-companion search input** — kiosk plan; unrelated.
- **Game manual reader UI** — Phase 6+ if PDF-inline UX needs work
  beyond the default WebView2 viewer.
- **Image format upconversion** — operator dropping a 4K boxart
  should not be auto-downscaled; we keep their file. Thumbnails
  remain 300px WebP from the source.
- **PARKING_LOT v3.4 art-slot integration** — the sidebar v3.4
  per-container art slot work parked earlier could consume the new
  platform-media slots (e.g. point a "Handhelds" container at the
  GB controller image). Worth noting but doesn't change v1 scope.

## Branch + commit plan (when implementing)

1. Pre-feature push (main clean).
2. `git checkout -b feat/media-taxonomy`.
3. Phase commits in order. Each phase is independent enough that
   operator can test + thumbs-up incrementally.
4. Push after each phase for visibility; final merge `--no-ff` after
   all 7 phases land + operator approves end-to-end.
5. SESSION_LOG entry per phase under
   `docs/features/media-taxonomy/SESSION_LOG.md`.

## Risk register

- **`MediaKind` serde rename collision** — old `media.json` files
  serialize as `"boxart"` / `"snap"` etc. New code expects
  `"box-front"` / `"screenshot-gameplay"`. Need `#[serde(alias = "boxart")]`
  on the new variants, OR a one-shot migration in Phase 5 that
  rewrites the JSON. Plan picks both for safety — alias for grace
  period, migration to clean up.
- **Filename sanitization** — Windows forbids `<>:"/\|?*` in
  filenames; some ROM titles have `:` (`X: Beyond Frontier`) or `?`
  (`Whoa?!`). Sanitize aggressively (replace forbidden chars with
  `_`) at write time. Document the sanitization rule.
- **Filename collisions** — two ROMs that share a stem post-sanitize
  (rare). Append `_2` / `_3` to the LATER write. The library DB's
  `id` (the djb2 hash) stays the canonical key; on-disk filename
  collision is purely an art-routing concern.
- **Migration partial failure** — same playbook as portable-install:
  write sentinel only after success; on failure log loudly + retry
  next launch.
- **Audio bus contention** — kiosk plan's 5 buses might compete for
  cpal output. Plan: one cpal stream per bus, hardware mixer combines
  via OS. If that's a perf problem we'll discover it and revisit.
- **Existing-installs that already use `set_manual_cover`** — their
  files live at `media/covers/<systemId>/rom-<hash>.<ext>`. Phase 5
  migration covers them.
- **Operator who upgrades MID-libretro-thumbnails-sync** — sync
  writes new-layout paths to `media.json` while old files still on
  disk under `media/covers/`. Phase 5 migration handles the cleanup
  on next launch; the in-flight sync isn't broken since the new
  files land at new paths.

## Related

- [kiosk-shell/KIOSK_PLAN.md](../kiosk-shell/KIOSK_PLAN.md) — §6
  defines the 5-bus audio model this plan honors; §2.4 defines the
  `.oatheme` package format that owns theme-default audio.
- [portable-install/README.md](../portable-install/README.md) — same
  one-shot migration pattern (sentinel-guarded recursive copy) used
  here for Phase 5.
- [docs/NEXT.md](../../NEXT.md) — this work listed in MEDIUM band
  for future scheduling.
- [docs/PARKING_LOT.md](../../PARKING_LOT.md) — sidebar v3.4
  per-container art-slot work could consume platform-media slots
  once this lands.
