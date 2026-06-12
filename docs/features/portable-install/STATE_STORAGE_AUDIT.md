# State-storage + portability audit

**Date:** 2026-06-11 · **Type:** read-only audit + recommendation (NO code shipped)
**Queued from:** `docs/NEXT.md` HIGH band → "Portability + state-storage audit"
**Cross-links:** virtual-library arc **S9** (beside-the-ROM media + library-roots
model + relative-path portability + NAS/offline rules) in
[../../PLANS/virtual-library-and-launcher-arc.md](../../PLANS/virtual-library-and-launcher-arc.md);
guided-setup LaunchBox research §10 Q1.

---

## TL;DR — the verdict

**OA today is NOT folder-move portable, and the dominant blocker is one
architectural fact: every filesystem path OA persists is stored ABSOLUTE.**
ROM paths, registered scan-folder paths, and canonical cover paths are all
full host paths (`C:\ROMs\…`, `/mnt/nas/…`). Move the folder — or carry the
DB to a Linux cabinet — and every game row dangles.

Two pieces are *already* portable and should be the model:
- **BIOS** resolves by convention (`<exe_dir>/system/<name>`), never stored. Carries fine.
- **The data dir itself** travels via the shipped `portable.txt` marker → `<exe_dir>/settings/`.

The good news from Part B: OA's per-user state, while *scattered* across ~13
JSON files + 6 localStorage keys + SQLite, is **already nearly all under one
portable `<data_dir>` tree** (or migrates there in portable mode). The
operator's "consolidate into SQLite" instinct is right *in spirit* but mostly
**already solved at the directory level** — the files all move together. The
real win is **not** dragging everything into SQLite (which costs human-
editability and couples the frontend to the backend); it's:
1. **Make paths root-relative** (the S9 roots model) — this is the actual
   portability blocker, and it's independent of the file-vs-DB question.
2. **Pull the 6 frontend `localStorage` keys** (the only state that does NOT
   live in `<data_dir>` and so does NOT travel with a portable copy) into the
   backend so the portable tree is complete.

**Cheapest path to "move a folder and it just works"** is at the bottom
(§C). It is a roots-model migration + a localStorage eviction — **not** a
grand DB consolidation.

---

# PART A — path storage & the library-roots gap

## A.1 Every path-bearing store, classified

| Store | Column / field | Stored as | Proof |
| --- | --- | --- | --- |
| SQLite `games` | `file_path` (NOT NULL UNIQUE) | **ABSOLUTE** | written from `scan_service.rs` `entry.path().to_string_lossy()` (`std::fs::read_dir` yields absolute) → `library_db.rs` INSERT (`file_path` ≈ `:2260`) |
| SQLite `games` | `archive_inner_path` | RELATIVE-to-archive (POSIX) | inner path within a `.zip`/`.7z`; the archive's own path in `file_path` is absolute |
| SQLite `games` | `cover_path` | **ABSOLUTE** (but effectively dead — legacy localStorage carry-over, not populated by current scan/enrichment) | `library_db.rs` ~`:5927` comment |
| SQLite `game_identities` | `canonical_cover_path` | **ABSOLUTE** | UPDATE via `IdentityMetadataUpdate`, `library_db.rs` ~`:3783/:3793` |
| SQLite `folders` | `path` (NOT NULL UNIQUE) | **ABSOLUTE** | `add_folder` writes the raw param, `library_db.rs` ~`:5732` |
| `media.json` (MediaDb) | per-slot thumbnail / asset paths | paths into the managed `<data_dir>/media/…` tree (OA-owned, so they re-derive under a moved data dir) | `media.rs` read/write `:383–421` |
| `platform-media.json` | per-system art slot paths | same managed-tree shape | `platform_media.rs` `:157–191` |
| BIOS | (none stored) | **DERIVED by convention** `<exe_dir>/system/<name>`; only the SHA-1 *status* is computed at check time | `main.rs` `:1020`, `:3814` |
| GameOverrides JSON | `patch_path`, `bezel_image_path` | **ABSOLUTE** (per-game, low-frequency) | `library_db.rs` `:77`, `:100` |
| `emulators.json` | external-emulator binary path | **ABSOLUTE** (intentionally machine-local — an installed `.exe`) | `emulator_profiles.rs` `:187` |

**Net:** the load-bearing library paths (`games.file_path`,
`game_identities.canonical_cover_path`, `folders.path`) are **all absolute,
single-string, never root-indirected.** Archive inner-path is the only
relative one, and that's relative to the (absolute) archive, not to a root.

## A.2 Does OA model multiple registered roots?

**Partially — the table exists, the indirection does not.** There IS a
`folders` table (`id, path, scan_subfolders, subfolders_are_systems,
watch_enabled, last_scanned_at` — `library_db.rs:2096`), and a
`folder_rules` child table FK'd to it. So OA already has a real **multi-root
registry** concept and the `add_folder`/`list_folders` plumbing
(`LibraryFolderRow`, surfaced in the Import Wizard + Settings → Library).

**But `games` has NO `folder_id` foreign key.** A game is keyed on its
absolute `file_path`, not on `(folder_id, path-relative-to-root)`. The
`folders` table is consumed as *scan targets + watch registry*, not as a
**resolution indirection layer**. So the one row OA would need to make every
entry relocatable — the root → absolute-location mapping S9 calls for —
isn't wired into entry resolution at all. The folders table is two-thirds of
the S9 roots model already sitting in the schema; it's missing the `games.
root_id` FK + the store-relative / resolve-at-read change.

**This is the central A-gap.** S9's model is `(root_id, relative)` with a few
machine-local root rows; OA has absolute paths everywhere and a `folders`
table that doesn't participate in resolution.

## A.3 Scan target selection

Scans take an **ad-hoc folder argument per import** (`run_scan_blocking` /
`run_dir_scan_blocking` invoked with an explicit path), not "iterate every
registered root and rescan." The `folders` rows + the filesystem watcher
(`watcher.rs`) give a registered-folder *concept*, but the canonical scan is
folder-at-a-time. (A "rescan all registered roots" loop would be a natural
companion to the roots migration.)

## A.4 Removed-ROM handling — the NAS-safety question (CRITICAL)

**Finding: OA does NOT have a rescan-purge sweep at all — and what removal
logic exists CANNOT distinguish root-unreachable from file-deleted.**

- **No scan-time purge.** `run_scan_blocking` / `add_games` only ADD
  (`INSERT OR IGNORE`). Nothing in the scan path stats existing rows and
  deletes the missing ones. So an offline NAS is *passively* safe against the
  scan path — but only because no purge sweep exists yet, not by design.
- **Removal is watcher-driven + opt-in.** The fs-watcher emits
  `oa://library-watch-removed` per deleted file (`watcher.rs:140/182`); the
  frontend handler (`App.tsx:1232–1282`) **keeps the entry by default**
  ("user might be moving / renaming") and only hard-deletes when
  `autoRemoveOnDelete` is ON, via `findGameIdByPath` → `library.remove(id)`.
- **The danger is latent, not yet shipped.** An offline NAS emits no
  watcher events (the FS is just gone), so today nothing purges it. BUT: the
  watcher distinguishes nothing about *roots* — it acts on individual path
  events. The moment anyone adds the obvious missing feature ("Scan for
  removed ROMs: stat every row, drop the gone ones" — which the LaunchBox
  research explicitly flags as a feature to build), it will, without a roots
  model, **purge an entire spun-down NAS library.** This is exactly the
  classic LaunchBox bug S9 warns against.
- **Launch-time check is fine** (`main.rs:11641/11740` error "not a file"
  instead of deleting) — that's a per-launch guard, not a sweep.

**Conclusion:** safe *today only by absence*. Any future removed-ROM sweep
MUST be built on the roots model (mark **Unavailable** when the *root* is
unreachable; only consider per-file deletion when the root resolves). Don't
ship the sweep before the roots model.

## A.5 Hash cache (re-hash avoidance)

- **Disc systems: persistent, mtime+size-keyed.** `game_disc_tracks` caches
  `(game_id, file_mtime, file_size, tracks)`; unchanged disc → no re-hash
  (`rom_hashes.rs:2434–2493`). NAS-friendly.
- **Cart ROMs: in-process only.** Keyed `(path, inner)` for the duration of
  one scan (`rom_hashes.rs:1531`); no persistent mtime cache → carts re-hash
  every identify run. Slow over the network; S9's `(path,size,mtime)` cache
  guidance is unmet for carts.

## A.6 What breaks on separate-drive / external / NAS / cross-OS

| Scenario | Today | Why |
| --- | --- | --- |
| ROMs on `D:` then drive becomes `E:` | every entry dangles | absolute `file_path` baked the letter in; no root row to re-point |
| Move system folder to another disk | every entry + canonical cover dangles | absolute paths; no relative re-resolution |
| Windows desktop DB → Linux cabinet | total breakage | `C:\…` paths meaningless on Linux; casing + separators differ |
| NAS asleep at launch | entries persist (safe *for now*) | no purge sweep exists yet — see A.4 caveat |
| NAS letter/mount changes | dangles, no re-point prompt | no root indirection to re-point |
| Re-scan over network | full re-hash of carts each pass | no persistent cart hash cache |

---

# PART B — consolidate scattered config into SQLite?

## B.1 Complete inventory of every persisted store

### Backend JSON, under `<data_dir>` (travels with a portable copy)

| Store | Path | Holds | Class |
| --- | --- | --- | --- |
| **Library DB** | `<data_dir>/library/games.sqlite` | games, identities, folders, rules, FTS, disc-track cache | per-user state |
| `media.json` | `<data_dir>/library/media.json` | per-ROM artwork variants + sources | per-user state |
| `media-prefs.json` | `<data_dir>/library/media-prefs.json` | region priority, sync kinds | per-user state |
| `prefs.json` (`LibraryPrefs`) | `<data_dir>/library/prefs.json` | region/revision priority, disc-track mode, **cpu_tier_override**, **active_theme_id** | per-user state |
| `job-prefs.json` | `<data_dir>/library/job-prefs.json` | background-job behavior toggles | per-user state |
| `platform-media.json` | `<data_dir>/library/platform-media.json` | per-system hardware art | per-user state |
| `cpu-tier.json` | `<data_dir>/cpu-tier.json` | detected CPU tier cache | **per-install / cache** (invalidates on hardware swap) |
| `layout.json` | `<data_dir>/layout.json` | sidebar/widget/view layout + **window geometry** | per-user state (geometry is per-install-ish) |
| `presentation.json` | `<data_dir>/presentation.json` | desktop/theater/cabinet mode | per-user state |
| `bindings/<sys>.json` | `<data_dir>/bindings/<sys>.json` | per-system input maps | per-user state |
| `cores.json` | `<data_dir>/cores.json` | per-system preferred core `.dll` | per-user state |
| `systems/<sys>.json` | `<data_dir>/systems/<sys>.json` | per-system setting overrides | per-user state |
| `emulators.json` | `<data_dir>/emulators.json` | external-emulator binary paths | **per-install** (absolute exe paths) |
| `launchers.json` | `<data_dir>/launchers.json` | per-system external-launcher choice | per-user state |
| media files | `<data_dir>/media/**` | actual art bytes + thumbs | per-user state (managed cache) |
| logs | `<data_dir>/logs/**` | rolling logs | **cache/logs** |

### Frontend `localStorage` (does NOT live under `<data_dir>` — does NOT travel)

| Key | Owner | Holds | Class |
| --- | --- | --- | --- |
| `oa.settings.v1` | `platform/settings/store.ts:65` | OA-wide prefs: scaling, window, shader, run-ahead, rewind, controller-nav, perSystemUi, **profileDisplayName/Avatar** | **per-user state** |
| `oa.themeSettings` | `platform/theme/themeSettings.ts:27` | per-theme `{themeId:{k:v}}` settings namespace | **per-user state** |
| `oa.core.<sys>.v1` | `systems/store.ts:17` | per-system frontend bucket (mostly reserved) | per-user state |
| `oa.library.activeTab` | `engine/LibraryManagerPage.tsx:264` | last library-manager tab | UI ephemeral |
| `oa.systemHealth.activeTab` | `engine/SystemHealthPage.tsx:55` | last health-page tab | UI ephemeral |
| `oa.retroverse.dailyRoulette` | `themes/retroverse/PlayNowPage.tsx:112` | daily pick lock | ephemeral cache |
| `oa.library.v1` (legacy) | `platform/library/store.ts:35` | pre-SQLite library — migrated then **removed** | dead |

No `sessionStorage` / `IndexedDB`. `activeThemeId` + glyph set are **not**
localStorage — `activeThemeId` lives in `LibraryPrefs` (`prefs.json`), glyph
set is theme-bound.

### Shipped content — ships WITH the install, read-only

| Store | Path | Holds |
| --- | --- | --- |
| system descriptors | `<exe_dir>/config/systems/<id>/{system,bios,games}.yaml` | per-system metadata, BIOS requirements, curated lists |
| emulator profiles | `<exe_dir>/config/emulators/<id>.yaml` | external-emulator launch descriptors |
| shader presets (built-in) | `include_str!` compiled in (+ optional `<exe_dir>/shaders/presets/*.preset.toml` user overlays) | render pipeline presets |
| theme manifests | `themes/**` `theme.toml` + assets | shipped theme definitions |
| `portable.txt` | `<exe_dir>/portable.txt` | presence-only mode marker |

## B.2 The key realization

**Scattered ≠ non-portable.** Almost all per-user state already lives under a
single `<data_dir>` tree, resolved at runtime by `data_dir.rs` (AppData by
default; `<exe_dir>/settings/` when `portable.txt` is present, with a one-shot
AppData→portable migration). Copy that one tree and ~13 JSON files + SQLite +
media all move together. The "scatter" the operator dislikes is a *cosmetic /
hand-editing* concern, **not** a portability one — for the files under
`<data_dir>`.

**The genuine portability holes are exactly two, and neither is "too many
files":**
1. **Absolute paths inside the DB** (Part A) — breaks cross-machine/-drive/-OS
   even if the whole tree is copied perfectly.
2. **The 6 frontend `localStorage` keys** — the *only* per-user state that
   does NOT live under `<data_dir>`. They sit in the WebView's origin store
   and are silently left behind by a portable copy. This is the real
   consolidation target, and SQLite/backend is the right home **because it
   makes them travel**, not because one-big-file is inherently better.

## B.3 Per-store recommendation

| Store | Recommendation | Rationale |
| --- | --- | --- |
| `oa.settings.v1` (localStorage) | **MOVE to backend** (`prefs.json` or a `settings` table) | only-non-traveling user state; holds the OA-wide prefs + user profile — must survive a portable move |
| `oa.themeSettings` (localStorage) | **MOVE to backend** | per-theme settings should travel; the per-theme-settings namespace already exists conceptually in S5 |
| `oa.core.<sys>.v1` (localStorage) | **MOVE to backend** (fold into `systems/<sys>.json`) | duplicates a per-system concept the backend already owns |
| `oa.*.activeTab`, `dailyRoulette` (localStorage) | **KEEP local / leave** | genuinely ephemeral UI; not worth a round-trip; fine to lose on a move |
| `oa.library.v1` | already removed — **none** | dead post-migration |
| Library DB (paths) | **KEEP as SQLite, but migrate to `(root_id, relative)`** | the Part-A fix; the DB is the right store, the *path encoding* is the bug |
| `media.json` / `platform-media.json` / `media-prefs.json` | **KEEP as files** (S9: convention-first resolution, MediaDb as override layer) | human-inspectable; atomic-write + `.corrupt` recovery already shipped; moving to SQLite buys nothing portability-wise (already under `<data_dir>`) and loses the file-level resilience |
| `prefs.json`, `job-prefs.json`, `layout.json`, `presentation.json`, `cores.json`, `systems/<sys>.json`, `launchers.json`, `bindings/<sys>.json` | **KEEP as files** | already travel with `<data_dir>`; hand-editable JSON is a feature for power users (per the low-floor/high-ceiling pillar); consolidating them into SQLite trades editability + git-diffability for nothing portability adds |
| `cpu-tier.json`, `emulators.json` | **KEEP as files, mark per-install** | intentionally machine-local (detected hardware / installed exe paths). Must NOT travel as-is; a portable copy should re-detect / re-point these |
| `config/systems/**`, `config/emulators/**`, theme manifests, shader presets | **LEAVE as shipped content** | descriptors that belong WITH the install, not in a user DB. Putting them in SQLite would fight the convention-over-config + theme-ecosystem direction |
| logs | **KEEP as files** | cache; truncated per launch |

### Why NOT "everything into SQLite"

**Pros of full consolidation:** one file; atomic backup; transactional;
no scatter.
**Cons (decisive here):** (1) loses hand-editability of config — directly
against the low-floor/high-ceiling + convention-over-config pillars and the
power-user hand-edit story; (2) couples frontend prefs to the Rust backend
via invoke round-trips for what is today a synchronous localStorage read
(every settings read becomes async); (3) buys **zero** portability for the
~13 files that already travel under `<data_dir>`; (4) fights S9's deliberate
"MediaDb is files, convention-first" direction; (5) shipped descriptors
don't belong in a per-user DB at all. The portability problem is **absolute
paths + non-traveling localStorage**, and neither is solved by relocating
already-portable JSON into SQLite.

---

# PART C — overall verdict + cheapest path to "move a folder and it just works"

**Verdict:** OA is one architectural change away from drive-move portability
and two changes away from cross-OS portability. The blocker is **absolute
paths**, not file scatter. The operator's consolidation instinct should be
**redirected** from "pull config into SQLite" (mostly a non-problem) to
"make DB paths relative + evict the 6 localStorage keys."

**Cheapest path, in dependency order:**

1. **Roots model (the load-bearing change).** Add `games.root_id` FK + store
   `path-relative-to-root`; resolve at read time against the `folders` row
   (already exists). Re-encode `canonical_cover_path` + per-game
   `patch_path`/`bezel_image_path` the same way (or move art fully to S9
   convention resolution and drop stored cover paths). One machine-local
   `folders.path` re-point → entire library re-resolves. ~the bulk of the
   remediation; schema migration v23→v24 + a one-time backfill (bucket each
   existing absolute path under its longest-matching registered folder;
   un-rooted strays get an implicit root). **Do this first; everything else
   is cheap.**
2. **Evict the 6 localStorage keys** to the backend so a portable copy is
   complete (`oa.settings.v1`, `oa.themeSettings`, `oa.core.*` → backend;
   leave the 3 ephemeral ones). Pairs with the theming S5 per-theme-settings
   namespace work.
3. **Volume-GUID / label tracking** (Windows) on the `folders` row so an
   external drive auto-relocates across letter changes; cross-OS root syntax
   (`\\NAS\retro` ↔ `/mnt/nas` ↔ `smb://`) lives only in the root row.
4. **Removed-ROM sweep — only after #1.** Build "scan for removed ROMs" to
   mark **Unavailable** when the *root* is unreachable and only consider
   per-file deletion when the root resolves. **Never ship this before the
   roots model** (it would purge a sleeping NAS).
5. **Persistent cart hash cache** `(path,size,mtime)` to match the disc cache,
   so network rescans don't re-hash unchanged carts.
6. **Casing standardization** on the chosen S9 media-convention dialect
   (Linux cabinets are case-sensitive).

Items 1–2 alone deliver "copy the portable folder between Windows machines /
re-point one drive and it just works." Item 3 covers external-drive letter
churn. Items 1+3+casing extend it to the Windows-desktop ↔ Linux-cabinet
cross-OS case. The roots table is **already two-thirds present in the
schema** — this is a wiring + migration job, not a new subsystem.

**One-line answer to the operator:** don't consolidate config into SQLite —
it mostly already travels as one `<data_dir>` tree. Make the DB's paths
**root-relative** and pull the **6 browser-localStorage keys** into the
backend; that, not de-scattering files, is what makes a moved folder "just
work."
