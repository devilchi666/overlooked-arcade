# Media Taxonomy — Session Log

Entries for the media-taxonomy work. Three lines per entry:
**Shipped / Almost / Next**.

---

## 2026-05-23 — Phase 1: data model + folder layout (`7c1b0e9`)

First commit on `feat/media-taxonomy`. The 8 locked design decisions
from the README readback got compressed into a clean foundation:
MediaKind enum jumps from 5 → 27 variants (full LaunchBox taxonomy),
GameMedia + SelectedMedia gain a slot per variant, set_manual_cover
writes to the new `media/<sys>/<kind>/<rom_stem>.<ext>` layout via a
freshly-pub `library_db::find_game_by_id` lookup.

- **Shipped:** `MediaKind` expanded to 27 kebab-case variants;
  `GameMedia` + `SelectedMedia` carry one slot each;
  `#[serde(alias = "boxart"/"snap"/"title"/"cart")]` keeps legacy
  `media.json` files deserializing forward without a migration step.
  New helpers `sanitize_filename_stem` / `rom_stem_from_path` /
  `media_path_for_slot` / `next_variant_filename` lay groundwork for
  Phases 2-5. `ingest_manual_for_slot` accepts kind + rom_stem;
  `set_manual_cover` Tauri command grew an optional kind arg and
  looks up rom_stem from library_db. Frontend types mirror Rust
  with v1 keys retained as defensive read-side fallbacks; coverUrl
  dispatches through `variantsForKind` / `pinnedIndexForKind`.
  +21 new media tests (43 total, 385 oa-shell).
- **Almost:** N/A — workspace tests + frontend build green.
- **Next:** Phase 2 — wire the new layout into the libretro-thumbnails
  sync path with the operator-art-wins guard.

---

## 2026-05-23 — Phase 2: sync to new layout + operator-art-wins (`c2d0976`)

Operator confirmed Phase 1 worked end-to-end ("seems to be working")
— manual covers landing at human-readable paths. Phase 2 extends the
same canonical layout to the libretro-thumbnails sync, with the guard
that operator manual art never gets clobbered by automated sync.

- **Shipped:** `sync_single_rom` now writes downloaded covers to the
  canonical layout via `next_sync_path_for_slot`. Operator-art-wins
  enforced naturally: when primary `<rom_stem>.<ext>` is occupied
  (manual cover, prior sync), new variants land at `-02`/`-03` instead
  of clobbering. Cache check switched from path-based (broken now
  that variant.path drifts across runs) to sha-based via the new
  `variant_sha_present_in_slot` helper — two checks: pre-download
  fast-path + post-download dedup. `ingest_manual_for_slot` gains
  eviction logic: prior synced variant at primary gets renamed to
  `-02` before the manual claims primary, db updated. Thumbnail dir
  flattened to one .webp per unique sha. +8 new tests (51 media,
  393 oa-shell).
- **Almost:** N/A — workspace + frontend green.
- **Next:** Phase 3 — LaunchBox/EmuMovies art-pack importer.

---

## 2026-05-23 — Phase 3: art-pack importer (`2edfc1d`)

New `art_pack_importer.rs` module lets operators drag-drop a LaunchBox
Images folder (or EmuMovies download, or any folder structured the
same way) and have the art routed into the canonical layout via
fuzzy-match against library titles. Same 0.95 threshold as the
libretro-thumbnails sync — high enough that "Sonic" doesn't catch
"Sonic 2".

- **Shipped:** `art_pack_importer.rs` (~480 lines incl. tests). Two
  layouts auto-detected: multi-platform (root has folders like
  "Sega Genesis", "Super Nintendo Entertainment System") and
  single-platform (root has kind folders like "Box - Front"). Static
  maps cover all 41 OA systems + capitalization variants + Wii→
  gamecube (Dolphin host); all 25 LaunchBox kind names → Phase 1
  MediaKind variants. Routes through `ingest_manual_for_slot` so
  Phase 2's eviction logic kicks in automatically. Dry-run mode
  returns a structured `ImportReport` so the UI previews counts
  before commit. Live-mode flushes media.json once at the end (not
  per-entry). New `library_db::list_games_for_system` helper scopes
  fuzzy match per-platform. Frontend `ImportArtPackDialog.tsx` with
  folder picker, system override dropdown, Analyze/Import buttons,
  per-platform × per-kind report grid. Library Manager → Game media
  tab gains the "Import art pack…" entry button. +10 tests
  (403 oa-shell).
- **Almost:** N/A — workspace + frontend green.
- **Next:** Phase 4 — audio playback service.

---

## 2026-05-23 — Phase 4: 4-bus audio mixer + override fields (`b71057c`)

Wraps rodio (cpal + symphonia for .ogg/.opus/.mp3/.flac/.wav/.m4a)
into a thread-owned 4-bus mixer per the kiosk plan. The fifth bus
(live game audio) stays on the dedicated `oa-audio` crate. Phase 4
ships the audio primitives + override fields + frontend service;
auto-wiring to UI events (debounced focus → BGM swap, click →
ui-sound dispatch) is deferred to Phase 6/kiosk polish.

- **Shipped:** `audio_player.rs` (~380 lines). 4 buses (platform-music,
  ui-sounds, ceremony, snap-audio), each with its own rodio Sink so
  volumes + play/stop are independent. Audio thread owns
  rodio::OutputStream (which is !Send); Tauri command handlers send
  commands over mpsc. Default volumes pre-set so cues + ceremony
  punch through music (music 0.5 < cues 0.7 < ceremony 0.85) —
  obviates per-pair ducking matrix for v1. Tauri commands:
  play_audio, stop_audio, set_audio_volume, resolve_platform_music
  (3-tier cascade: game override → system override → silence),
  resolve_ui_sound (per-system override for one of 6 event names).
  SystemSettings gains `platform_music_path` + 6 `ui_sound_*`
  fields; GameOverrides gains `platform_music_path`. All optional
  + skip_serializing_if so legacy files round-trip cleanly. Audio
  thread degrades gracefully when no output device is available
  (logs warn + drains commands as no-ops). Frontend `lib/audio.ts`
  with `playAudio` / `stopAudio` / `setAudioVolume` /
  `dispatchPlatformMusic` / `dispatchUiSound`. New `AppDataDir(PathBuf)`
  Tauri-state newtype for commands that don't already go through a
  stateful service. +12 tests (412 oa-shell).
- **Almost:** Auto-wiring to UI events deferred — service is in place
  + invokable from console; settings UI for SETTING the override
  paths lands in Phase 6.
- **Next:** Phase 5 — existing-install migration.

---

## 2026-05-23 — Phase 5: existing-install migration + sentinel (`92c2403`)

Operator's existing install has ~1776 entries in the pre-Phase-1
layout. Phase 5 walks the in-memory MediaDb on launch and brings
them into the canonical layout: manual covers moved from
`media/covers/<sys>/rom-<hash>.<ext>` to
`media/<sys>/<kind>/<rom_stem>.<ext>`, synced art copied out of the
cache dir to canonical kind dirs (cache kept for Phase 2's fast-path
re-sync cache check).

- **Shipped:** `data_dir::migrate_media_naming` (~660 lines added to
  data_dir.rs incl. 8 tests). Sentinel-guarded by
  `.media-taxonomy-migrated` in `<data_dir>` so the migration runs
  at most once per install. Wired into main.rs setup AFTER both
  library_db and media_db workers join — migration needs library_db
  for rom_stem lookups + write lock on the shared MediaDb. Operator-
  art-wins guard from Phase 2 applies to migrated data too
  (collisions get -02/-03 suffixes via next_variant_filename).
  Manual variants iterated before synced within each slot vec so
  manuals claim primary when contested. Five graceful-failure
  counters (manual_renamed / synced_copied / skipped_already_new /
  skipped_lookup_failed / skipped_file_missing) for diagnostic
  surfaces. Migration emits `oa://media-updated` batch on success
  so frontend re-hydrates without restart. +8 tests (421 oa-shell).
- **Almost:** N/A — workspace green.
- **Next:** Phase 6 — platform media + per-system UI.

---

## 2026-05-24 — Phase 6: platform media + per-system UI (`d8dd98a`)

Per-system "platform media" — hardware photos, controllers, marquees,
wheel art, banners. Distinct from GameMedia: one image per slot per
system (not per ROM), so the data model is `Option<MediaVariant>`
per slot. 9 slots cover the LaunchBox "Platform - X" set; the
kiosk shell (separate work stream) will consume `wheel/<systemId>.png`
for its tile UI when that stream picks up.

- **Shipped:** `platform_media.rs` (~430 lines incl. 9 tests). 9-slot
  taxonomy: banner, clear-logo, console, controller, fanart, marquee,
  photo, wheel, background. Files at
  `media/platform/<slot>/<systemId>.<ext>`; index at
  `<data_dir>/library/platform-media.json` with the same atomic-write
  + `.corrupt`-backup pattern as media.json. PlatformSlot enum with
  ALL invariant (locked count = 9). Three Tauri commands:
  get_platform_media_index, set_platform_media (writes file +
  updates db + emits `oa://platform-media-updated`),
  clear_platform_media (best-effort file delete + db update). Three
  media.rs helpers promoted to pub(crate) via thin _pub wrappers so
  the new module can reuse atomic_write_bytes / detect_format /
  sha1_hex without duplicating. New `PlatformMediaDialog.tsx`:
  system dropdown (41 systems via systemThemes registry) + 9-slot
  grid with preview cell + Choose…/Clear per slot. Listens to
  `oa://platform-media-updated` so set/clear roundtrips don't need
  refetch. Library Manager → Game media gains "Platform media…"
  button. +9 tests (430 oa-shell).
- **Almost:** Kiosk-shell wheel UI consumption (use
  `wheel/<systemId>.png` as the system tile logo, gradient fallback)
  deferred per the README's "prep work for kiosk shell" note. The
  data + UI to SET the images ship here; consumption lands with the
  kiosk shell work stream.
- **Next:** Phase 7 — docs + SESSION_LOG (this file).

---

## 2026-05-24 — Phase 7: docs + merge prep

Final phase before the `--no-ff` merge to main. Writes the SESSION_LOG
entries for Phases 1-6, flips the README status from PLANNED →
shipped, updates ACTIVE_WORK + NEXT.

- **Shipped:** This SESSION_LOG with one entry per phase, citing the
  shipping commit. README.md status banner flipped from
  "📐 PLANNED, not implemented" to "✅ shipped" with the merge
  date. ACTIVE_WORK.md moved media-taxonomy out of the "in flight"
  list and into "Recently completed" with all 7 commit shas. NEXT.md
  pruned the MEDIUM-band reference (no longer applicable).
- **Almost:** N/A — pure docs work, no code change.
- **Next:** Operator end-to-end approval on the final pre-merge build;
  then `--no-ff` merge `feat/media-taxonomy` to main, delete branch
  both sides. Followup items in PARKING_LOT.md (audio override UI
  surfaces, kiosk wheel-art consumption) are stretch polish — not
  blocking the merge.
