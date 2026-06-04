# Parking Lot

Out-of-scope ideas worth keeping but not pursuing now. Anything that isn't current-phase work for the active core goes here.

Append-only. Date entries. When an item moves into scope, link the deciding entry in `docs/DECISIONS.md` and strike the parking-lot entry (don't delete — history is reference).

---

## Format

```
- YYYY-MM-DD — short title
  Why it came up: <one line>
  Why deferred: <one line>
```

---

## Items

- ~~2026-05-15 — Per-system overscan / safe-area / aspect-correction quirks~~
  Why it came up: Phase 2 scaling modes need a per-system "true aspect ratio" + overscan crop config to make "aspect-correct fit" really accurate (PCE's non-square pixels, NTSC overscan, etc.).
  Why deferred: implement the basic scaling modes first; system-specific aspect tuning becomes per-core polish in each system's bring-up.
  **2026-05-19 — Closed.** Both halves shipped end-to-end. `SystemSettings.display_aspect_override` + `SystemSettings.overscan_crop_override` (`OverscanCropPrefs { top, bottom, left, right }`) in `apps/oa-shell/src/system_settings.rs` mirror onto `GameOverrides` in `apps/oa-shell/src/library_db.rs` with the same shapes; per-game → per-system → core-default resolution feeds the renderer viewport math via the existing Display launch-wiring chain.

- ~~2026-05-15 — Per-game scaling-mode override~~
  Why it came up: some games look right pixel-perfect, others (text-heavy or 240p artwork) look better stretched. Phase 2's per-game default is enough to start; full per-game override UI is more.
  Why deferred: cover the basic global default first; per-game UX comes with the library + save-state work.
  **2026-05-19 — Closed.** Shipped as part of Phase 2.8 slice D's per-game settings drawer + Phase 3 slice B's launch-path wiring. Storage in `GameOverrides.scaling_override`, UI in `PerGameSettingsDrawer.tsx` Display tab, runtime push via `App.handleLaunch`'s scaling resolution chain.

- ~~2026-05-19 — External drag-drop file import~~
  Why it came up: drag-drop from Explorer onto the library window stopped delivering paths reliably across both shell modes. Investigated 2026-05-19 against Tauri 2.11.1 + wry 0.55.1 + WebView2 on Windows 11; tried opaque-window pivot (Diff A) with WebView visibility toggling, persistent corner widget, on-demand overlay window — none caught drops in single-window mode and the diagnostic revealed two-window mode (which used to work per Phase 2.6 docs) is also broken now. Root cause unclear without bisecting commits + WebView2 runtime version + Tauri/wry releases.
  Why deferred: every diagnostic path opened more questions than it closed.
  **2026-05-20 — Won't fix.** Operator decided OA doesn't need external drag-drop file import. The Import Wizard (toolbar `⋯ → Import folder…`) + `Settings → Library → Add` cover the use case completely. No further effort to be spent on diagnosing the Tauri/wry/WebView2 regression. Internal HTML5 drag (sidebar reorder, region priority list) is unaffected — that path lives entirely inside Chromium and continues working.

- 2026-05-19 — Region badges + publisher / developer logos
  Why it came up: the multi-region grouping work (Versions submenu, group tiles) reads better with a flag next to each region label + a publisher logo on the game-info surfaces. Researched sources: flag-icons (lipis.dev, MIT SVG) and Flagpedia for region flags; LaunchBox "Publisher & Developer Logos" pack + EmuMovies Logo Submissions for company logos. Both LaunchBox + EmuMovies key on the canonical publisher name we already store in `games.publisher` — no fuzzy matching needed. Assets ship next to the .exe in `<exe_dir>/assets/regions/` and `<exe_dir>/assets/publishers/` (same model as cores/ + system/).
  Why deferred: feature is fully functional with text-only labels; logos are pure polish and need a download + license-check pass before shipping. Pick up after the next core onboarding lands.

- ~~2026-05-20 — Direct-launch CLI v2 — `--state-file PATH` actual restore~~
  Why it came up: the direct-launch CLI (shipped 2026-05-20 on `feat/direct-launch-cli`) accepts `--state-file PATH` but doesn't yet wire it through to the emu thread — the frontend logs a warning and the operator falls back to `--slot N`. Operators using LaunchBox's "play from save" feature might want arbitrary save-state files restored at launch.
  Why deferred: needs a new `restore_state_file(path)` Tauri command (or extension of `launch_rom`) that bypasses the per-game slot directory convention and loads a state from an arbitrary path. Two-line implementation but wants a real-world need to ground the file-path semantics (relative-to-cwd? absolute only? expand `~`? validate state-file header?).
  **2026-05-21 — Closed (Phase I).** Wired via `EmuCommand::LoadRom.restore_state_path` (PathBuf), threaded through `launch_rom` Tauri command (`stateFile: Option<String>` param), through `launchRom` JS, through `handleLaunch`. Emu thread's LoadRom handler reads + `core.load_state` after the rom load completes, atomically. `--slot` and `--state-file` mutually exclusive at CLI parse (RetroArch convention). State-file existence validated at CLI parse so a missing file errors before any subprocess work.

- 2026-05-20 — Direct-launch CLI v2 — Multi-instance
  Why it came up: running two `oa-shell.exe` direct-launches in parallel (split-screen on one machine, or "open two games at once" workflows) would let LaunchBox treat OA as a single multi-instance emulator. v1 doesn't support this — log file locking (`oa-current.log` truncate-each-launch convention), singleton libretro core state, and the per-system-default core-pref file (`cores.json`) all assume one process.
  Why deferred: low-demand feature; current operators run one game at a time. Revisit if real-world LaunchBox / BigBox configs surface multi-instance needs.

- ~~2026-05-20 — Direct-launch CLI v2 — Archive inner-ROM addressing~~
  Why it came up: `oa-shell.exe "set.zip#inner.nes"` would let a launcher pass a single ROM out of a multi-game .zip directly, without having to extract first. The `archive::extract_for_launch` plumbing already exists for library launches — direct-launch just doesn't parse the `#inner` suffix from CLI args.
  Why deferred: requires CLI to teach about the `<path>#<inner>` encoding (existing library code uses it as a `file_path` column convention), validation that the inner path actually exists in the archive, and the same UnknownExtension / AmbiguousExtension flow on the inner extension. v1 operators with multi-game .zips can scan them into the library first and launch by hash-matched library row.
  **2026-05-21 — Partially closed (Phase H).** Single-ROM `.zip` / `.7z` archives auto-extract transparently — `oa-shell.exe "ActRaiser 2 (USA).zip"` peeks inside, finds the single inner `.sfc`, sets `archive_inner_path`, and launches with the standard `archive::extract_for_launch` plumbing. Hash-lookup against `library_db` hashes the inner bytes to match the library DB's sha1 convention. MAME / Neo Geo romsets pass through as-is via `--system mame` (or `neogeo`) or the `.p1+.s1` auto-detection. **Still open:** explicit `<path>#<inner>` syntax for multi-game archives (operators with multi-ROM archives still need to scan via Import Wizard first), and CD images inside archives (multi-file extract-to-temp).
  **2026-05-21 — Fully closed (Phase I).** Explicit `<archive>#<inner>` syntax shipped via `resolve_explicit_archive_inner` — power-user form bypasses the Phase H single-ROM requirement, validates inner against the archive's table of contents (typos error with available-inner list). Cart inners auto-infer system; CD inners require `--system`. The synthesized RomEntry folds the inner path into both `id` (so two different inners in the same .zip get distinct entryIds for temp-dir cleanup) and `filePath` (mirroring the library DB's encoded convention).

- ~~2026-05-20 — Direct-launch CLI v2 — CD images inside archives~~
  Why it came up: a `.cue + .bin` set wrapped in a `.zip` didn't auto-extract in direct-launch — operators had to extract the CD set to a folder and pass the `.cue` directly with `--system <psx|saturn|…>`.
  Why deferred (initially): the library handles this via `extract_to_temp`; CLI needed to detect + route the same way. `accepted_rom_extensions` was restricted to cart shapes for Phase H to keep scope tight.
  **2026-05-21 — Closed (Phase I).** `resolve_archive` extended to accept `.cue / .ccd / .toc / .m3u` inner extensions in the peek filter. Single CD inner → `--system` required (CD formats are ambiguous), `archive_inner_path` set to the inner, `launch_rom`'s existing `is_cd_entry_extension` branch fires `archive::extract_to_temp` to a temp dir keyed off entryId. Synthesized RomEntry's id derived from `<archive>#<inner>` so re-launches reuse + clean the right temp.

- 2026-05-21 — Direct-launch CLI v2 — Launcher-parity flags
  Why it came up: shipped direct-launch covers ROM path + system + core + slot + state-file + tas-replay + fullscreen + archive auto-extract. Missing for full LaunchBox / BigBox parity: `--monitor N` (per-game monitor pinning), `--no-fullscreen` / `--windowed` (counterpart to `--fullscreen`), `--scaling MODE` (aspect-correct / integer / stretch / fill), `--shader NAME` (preset override).
  Why deferred: every one of these is configurable via the per-system / per-game library cascade today, so the value of CLI exposure is "operator wants to vary per-launch without library DB rows." Real-world ask hasn't surfaced. Each is ~30 min of work; ship in a batch when LaunchBox power users actually surface the friction.

- 2026-05-21 — Direct-launch CLI v2 — Kiosk / arcade-cabinet flags
  Why it came up: museum installs / dedicated arcade cabinets want to lock the experience down — no Quick Settings, no Esc, no close button, no Ctrl+Q. `--kiosk` disables all exit/UI hotkeys; `--auto-restart` relaunches on game exit; `--idle-timeout SEC` quits on N seconds of no input; `--no-overlays` suppresses toasts + perf HUD. ~2-3 hours for the full batch.
  Why deferred: distinct audience expansion (arcade-cabinet operators) rather than refinement of the existing LaunchBox-style audience. Ship when someone actually builds an OA cabinet.

- 2026-05-21 — Direct-launch CLI v2 — Diagnostics flags
  Why it came up: debugging a LaunchBox config today means "launch and see what happens" — slow feedback loop. `--verbose` / `-v` bumps log level. `--probe` / `--dry-run` parses + resolves everything (CLI, system inference, archive peek, hash lookup, core resolution) and outputs the resolved `DirectLaunchConfig` as JSON to stdout / MessageBox without launching. `--list-systems` prints every supported system slug + default core .dll + extension list for discoverability. ~2 hours for the batch.
  Why deferred: nice-to-have. The existing log file at `appData/logs/oa-current.log` already provides enough signal for most diagnostics. Pick up if launcher-config friction becomes a real cost.

- 2026-05-20 — Direct-launch CLI v2 — Persistent kiosk profile
  Why it came up: a true kiosk install would auto-launch the same game on every boot (arcade cabinet shipped to a museum, etc.), not just on `--rom` invocation. Operator would configure once, OS auto-runs OA, OA auto-runs the game, no library ever shown.
  Why deferred: needs a persistent on-disk "kiosk mode" flag (`appData/kiosk.json`?), boot-time validation that the configured ROM still exists, and a way for the operator to override-to-library temporarily without erasing the kiosk config. None of these are hard, but no real-world deployment is asking for them yet.

- 2026-05-22 — Sidebar v3.4 — Per-container art slots
  Why it came up: View Editor v3 ships container metadata (label, rule, accent) but not per-container artwork. Operators with custom views — e.g. a "Handhelds" container or a "Decade: 1990s" container — would like to attach a thumbnail / banner image to the container header so the sidebar reads more like a curated shelf and less like a folder list. Design spec lives in `docs/features/sidebar/VIEW_EDITOR_PLAN.md` §0.8 + §4.
  Why deferred: needs storage + format design before code work — where do operator-uploaded images live on disk (per-view file? appData/views-art/?), what aspect ratios are supported, how do they interact with per-system accent + theming, and how does the kiosk shell consume the same field. Picked up after the kiosk theming substrate (Q8/Q11 dependencies) lands so art slots can share that asset pipeline.

- 2026-05-22 — Kiosk / Cabinet mode (full design)
  Why it came up: long planning conversation 2026-05-22 settled the entire BigBox-class kiosk feature set — theming substrate (4-layer + Rhai), in-engine Theme Studio, attract mode (3 tiers), 5-bus audio mixer, multi-monitor surfaces, launch ceremony, in-game menu, configurable controller bindings, named views w/ arbitrary hierarchies, kid mode, accessibility floor, federated theme distribution, 7-phase implementation plan.
  Why deferred: full plan captured in `docs/features/kiosk-shell/KIOSK_PLAN.md`. Phase 0 of that plan IS the current settings/IA polish work (`docs/features/ui-polish/UI_MENU_BAR_PLAN.md`). Kiosk shell itself is Phase 1+ — picked up after the desktop UI lands at a polished baseline.

- ~~2026-05-24 — ScummVM `--detect` auto-generation of `.scummvm` files~~
  **2026-05-24 — Closed via options A + B both shipped.** Option B
  (curated sentinel-filename heuristic) shipped first on
  `feat/scummvm-auto-detect`: new `apps/oa-shell/src/scummvm_detect.rs`
  ships a table of ~18 well-known SCUMM games + ScummVM freewares;
  new `ScummvmDetectDialog` (Import Wizard Step 2 banner) walks a
  parent folder, runs sentinel detection on each subdir, lets the
  operator confirm + edit + write `.scummvm` descriptors in bulk.
  Option A (standalone ScummVM CLI shell-out) followed on
  `feat/scummvm-cli-option` as a power-user mode toggle in the same
  dialog: new `apps/oa-shell/src/scummvm_cli.rs` auto-discovers
  `scummvm.exe` in standard install paths + `$PATH`, runs
  `scummvm --detect --recursive --path=<dir>`, parses the CLI output
  with a defensive line-by-line parser that handles modern 2.x and
  variant column widths, and overlays CLI matches onto the
  directory-walker's canonical subdir list. Operators with a
  standalone install flip to CLI mode for the full ~400-game
  catalog; everyone else stays on the curated table. Option C
  (bundled detector) stays ruled out — significant maintenance
  burden vs the two-option approach now shipped.

- 2026-05-24 — DOSBox per-game `dosbox.conf` editor
  Why it came up: dosbox-pure honors a per-game `dosbox.conf` in the game directory automatically; operators who need to tune cycles / sound card / memory currently hand-edit that file outside OA. An in-app conf editor (drawer panel rendering `[autoexec]` + tuning knobs as form fields) would close the alt-tab loop.
  Why deferred: dosbox-pure also exposes most tuning via core options, which the existing per-game core-options drawer (slice 2.8.D) renders automatically. The remaining gap is `[autoexec]` and `[dosbox]` section editing — niche enough that hand-editing is acceptable for v1.

- 2026-05-24 — Unify `ScanMode` dispatch
  Why it came up: Phase 2 of feat/dosbox-and-scummvm shipped two parallel scanner functions (`run_scan_blocking` for extension-mode, `run_dir_scan_blocking` for directory-mode) and two parallel Tauri commands (`start_background_scan`, `start_background_directory_scan`). Functional but split — a `ScanMode` enum with `Files { extensions } | Descriptors { extension } | Directories { depth }` would consolidate the dispatch when a third engine-launcher type lands.
  Why deferred: with only one directory-mode system (dosbox) today the split is review-friendly; consolidating prematurely would add abstraction without a third use case to justify it. Pick up when a future engine launcher (Game.com, Twine, PuzzleScript) adds the third scan shape.

- 2026-05-25 — Theme ecosystem (Rhai-scripted + `.oatheme` archive + federated index repo)
  Why it came up: ChatGPT advisor session reviewed `docs/features/kiosk-shell/KIOSK_PLAN.md` Phase 2 design — TOML layout + Rhai scripts + Theme Studio editor + signed `.oatheme` archive + federated theme distribution. The capability is fully designed and would deepen OA's "premium feel" story.
  Why deferred: classic dead-ecosystem trap. Theme ecosystems require simultaneous demand (users wanting themes) AND supply (theme authors producing them). With OA's current user count, neither side reaches critical mass; the author would maintain the entire ecosystem alone with no community contributions, locking in maintenance cost without product value. Per-system CSS hardcoded in `frontend/src/themes/registry.ts` is enough for the curator audience that exists today. Reconsider if/when (a) the kiosk shell launches AND (b) there's clear community pull (e.g. multiple operators independently asking "how do I share themes?"). Until both: stay with per-system CSS; operators who want different aesthetics patch the CSS in their build.

- 2026-05-25 — License pivot from GPL-2.0 to permissive (MIT / Apache 2.0)
  Why it came up: ChatGPT advisor session asked whether the GPL-2.0 binary license should stay or move permissive once the dynamic-load pivot fully lands. Permissive licensing would lower the bar for community contributions + forks + downstream ecosystem use. GPL cores stay GPL inside their .dll regardless — the shell license is independent post-pivot.
  Why deferred: not deferred per se — **decision accepted, timing deferred.** Plan: move shell to MIT or Apache 2.0 once (a) the dynamic-load pivot is complete (vendored static crates `oa-pce-sys` / `oa-pce` retired from the workspace build, ✅ already done as of 2026-05-16 architecture pivot), AND (b) the installer ships only our own DLL builds of any forked cores (so the GPL propagation surface area is purely behind the `cores/` runtime-load boundary). The "ship our own DLLs" condition is partial today (we use community-built nightlies); the pivot is "in progress." When complete, file a new DECISIONS entry that supersedes the 2026-05-15 "GPLv2 binary-wide" lock and update workspace `Cargo.toml` `[workspace.package].license` to the new choice.
  Commercialization-risk read: commercial actors will copy regardless of license; OA's defense is vision + execution speed + non-commercial intent, not legal walls. Mission-aligned: a permissive license matches "gift to the retro community" more cleanly than copyleft enforcement.


- ~~2026-05-29 — Now-playing chip "playback failed" subscription~~ —
  **SHIPPED 2026-05-29** on `feat/now-playing-failure-event`. Rust
  `audio_thread_main` now holds `Option<AppHandle>` + emits
  `oa://audio-playback-failed { bus, reason }` on file-open / decode
  / sink-alloc failures (all three Play-command branches + cold-
  start no-default-device drain). Frontend `lib/audio.ts` listens at
  startup; payload.bus === "platform-music" clears the `nowPlaying`
  signal. `AudioPlayerHandle::spawn` signature widened to accept the
  handle; `None` keeps emission off for headless test contexts.

- 2026-05-29 — Drag-reorder for custom collection members
  Why it came up: Slice 12's `custom_collection_members` table carries a `sort_order` column populated on add but no UI surfaces drag-reorder. The schema is ready; only the UI is missing.
  Why deferred: low-leverage polish — operators today add games in the order they want to see them. Reorder becomes useful only when membership lists grow large enough that "the order I added matters less than the order I want to play." Pick up when an operator complaint surfaces.

- 2026-05-29 — Per-game release region in MediaDb GameMetadata
  Why it came up: DISCOVER v1 wanted a "By region" axis but `GameMetadata` carries publisher / developer / year / genre / players / description — no `region`. Region lives per cover-art variant in `MediaVariant.region`. Shipped as "By publisher" instead.
  Why deferred: needs a metadata-source decision — libretro-database has region tags per ROM hash but they're per-dump (USA vs Japan vs Europe), not per-game. Aggregating per-game (a game in MULTIPLE regions like Castlevania: SotN) is a small data-model choice that needs operator input. Pick up if DISCOVER's "By region" becomes the most-requested axis.

- 2026-05-29 — games-table genre/year/developer/publisher columns are dead code
  Why it came up: v1 schema (line 994 of `library_db.rs`) declares year/genre/developer/publisher columns but no Rust code ever writes to them — metadata enrichment writes to MediaDb instead. The columns are read by no code path either.
  Why deferred: dropping the columns is a v15 migration. Keeping them is a future "write enrichment to both places" option (e.g. for SQL-side queries / FTS5 searches on developer/publisher) which is real value. Cleanup decision: either populate them via the metadata-sync path, or migrate to remove them. No urgency either way. Cross-ref: `docs/features/retroverse-ui/DECISIONS.md` 2026-05-29 "DISCOVER v1: 4 data-driven axes."

- 2026-05-31 — Two-shell architectural decision (Retroverse opinionated; Kiosk hosts customization) + Per-System UI Stage 2+ routing
  Why it came up: legacy-Shell-deletion Phase 1 restored `SystemBackground` + `SystemBootAnimation` + `StylusOverlay` to Retroverse mode (pre-deletion these only rendered inside the legacy `<Shell>`'s `<main>`). Operator confirmed `SystemBackground` visibly conflicts with the Retroverse chrome — its 50%-opaque radial-gradient overlay sits on top of `RetroverseShell` because it's a root-level sibling with no positioned ancestor + later in DOM order than the shell. Interim workaround: Settings → Display → Per-system experiences master toggle OFF.

  **Investigation surfaced the real issue.** Per-System UI Stage 1's visual overlays were designed around a "central transparent library area with per-system art bleeding through" — fundamentally different from Retroverse's "opinionated tabbed shell where Retroverse owns the visual identity." Per-System UI Stage 2's vision (per-system *navigation*: wheels / carousels / lists per system) goes even further — assumes a layout-flex shell, which Retroverse explicitly isn't. The conflict is design-intents disagreeing, not a layering bug.

  **Decision (2026-05-31):** Two-shell future. Retroverse stays opinionated + clean (Heroic Games Launcher peer); a separate **Kiosk** shell hosts the themable / heavily-customized experience (BigBox peer). Naming locked: Kiosk (matching existing `docs/features/kiosk-shell/`). Kiosk shell stays **back-burner** for now — defer the work until there's appetite to spec it. The Kiosk plan in `docs/features/kiosk-shell/KIOSK_PLAN.md` already pegs it as "theme editor for power users that consumes the built-in per-system experiences as starting defaults" (2026-05-26 DECISIONS Q), which matches this routing.

  **Per-System UI Stage 1 split:**

  | Stage 1 piece                                        | Retroverse | Kiosk |
  |------------------------------------------------------|:----------:|:-----:|
  | Per-system audio (navigate / launch / boot-intro SFX) | ✅ keeps   | ✅ inherits |
  | Per-system accent colors (`--color-system-accent`)    | ✅ keeps   | ✅ inherits |
  | Tile flourishes (LCD-feel hover, physical click pulse)| ✅ keeps   | ✅ inherits |
  | `SystemBackground` (full-viewport gradient + art)     | ❌ removed | ✅ home |
  | `SystemBootAnimation` (transient tint flash overlay)  | ❌ removed | ✅ home |
  | `StylusOverlay` (cursor reticle for NDS)              | ✅ keeps (no z-conflict) | ✅ inherits |

  **Per-System UI Stage 2 routing:**
  - Visual / layout parts (per-system navigation: carousel / wheel / list; per-system tile emphasis): Kiosk-only.
  - Audio sub-part (per-system in-game SFX — sword swings tinted differently per system, etc.): ships in **both** shells when picked up (audio is layout-agnostic).

  **Per-System UI Stage 3 routing:** case-by-case when picked up. In-game overlays + library↔game transitions may work in either shell; metadata-priority field is data-only and shared.

  **Immediate fix (2026-05-31):** `SystemBackground` + `SystemBootAnimation` removed from `frontend/src/App.tsx` on `feat/retroverse-per-system-overlay-fix`. The components themselves stay in the codebase (`frontend/src/components/SystemBackground.tsx` + `SystemBootAnimation.tsx`) as future Kiosk consumers. `StylusOverlay` retained in Retroverse. `hoveredSystemId` signal + its mouseover tracker dropped (only consumer was `SystemBackground`); `pinnedEntry` memo dropped (only consumer was `SystemBackground`'s fallback chain). The master toggle's audio + accent + tile-flourish gates continue to work uncontroversially in Retroverse.

  Why deferred: Kiosk shell is multi-month design + implementation work. No operator demand for it yet; Retroverse covers the daily-driver case completely. Pick up when an operator use case (cabinet build / power-user customizer) actually surfaces. Until then, the visual-overlay components sit in-tree as ready-to-consume building blocks.

  Cross-refs: legacy Shell deletion commit `274df1e` (`feat/retroverse-legacy-deletion`); overlay-fix branch `feat/retroverse-per-system-overlay-fix`; `docs/PLANS/per-system-ui.md` (Stage 1+ plan); `docs/features/kiosk-shell/KIOSK_PLAN.md` (Kiosk design notes); `docs/DECISIONS.md` 2026-05-26 Q (Kiosk-as-theme-editor framing).

- 2026-06-01 — Forty more L2 `system-info.yaml` files (the unrepresented systems)
  Why it came up: System Info Panel v1 shipped 2026-06-01 with L2
  YAMLs for only 5 of 45 systems (snes / nes / genesis / psx / gb —
  the entries hand-migrated from `systemMetadataStubs.ts`). The
  other ~40 systems' panels show MAME L1 fields (CPU / sound /
  resolution / refresh rate / max players) but "—" in every L2-only
  row (type / generation / release_date / discontinued / units_sold
  / multiplayer / hero blurb / sidebar subline) and the curated
  peripheral list. Each L2 file is shippable independently; the
  panel degrades gracefully whether the file exists or not.

  Why deferred: operator scheduling. Mechanical content work — no
  hard blocker, just wants a calmer window to do the 3-system pilot
  + 37-system fill. Not on the critical path for anything; doesn't
  gate future features.

  **Authoring methodology (decided 2026-06-01 — record so we don't
  re-derive next time):**

  Source-per-field plan — minimize redundancy with L1:

  | Field | Source |
  |---|---|
  | `manufacturer` / `year` / `cpu` / `sound` / `resolution` / `refresh_rate` / `max_players` | Already in L1 from MAME — **omit from L2** unless MAME's emit needs polishing. Skip-by-default; only override when the L1 string reads wrong (e.g. TG-16 resolution comes out as `1088 × 242` from the pixel-clock dims — override to `256 × 224`). |
  | `type` / `generation` / `architecture` | Public knowledge (Wikipedia-level). |
  | `release_date` (full month + day) / `discontinued` / `units_sold` | Public knowledge; WebSearch for the exact dates / numbers when uncertain. |
  | `color_palette` / `display_ratio` / `media` / `storage` / `ram` / `video_output` / `aspect_ratio` | Public knowledge. |
  | `multiplayer` | Free-form, name the actual adapter for the system ("2 local; 4 via Team Player" / "2 local; up to 8 via Multitap" / "Single player only"). |
  | `peripherals` (name + glyph) | Curated list — not just the raw MAME hints. Glyph conventions: 🎮 gamepads, 🔫 light guns, 🖱️ mice, 🔗 multitaps, 🎤 mics, 📷 cameras, 🏎️ wheels, 📳 rumble, 💾 memory carts, 📡 wireless, 👣 mats, 🤖 R.O.B.-class oddities. |
  | `release_flag` | 🇯🇵 / 🇺🇸 / 🇬🇧 / 🇩🇪 by country of first release. |
  | `tagline` | Format-match the 5 existing: "{BIT}-BIT {FORM-FACTOR}" e.g. "16-BIT HOME CONSOLE", "8-BIT HANDHELD". |
  | `blurb` | 2-3 sentences in the existing voice — slightly editorial, name-check 2-3 key franchises, period-aware. See the 5 shipped files for the calibration; samples drafted for Saturn + Lynx already validated as voice-matching. |
  | `sidebar_subline` | Format-match: "{BIT}-BIT · {YEAR}" e.g. "16-BIT · 1990". |

  Process per system (~5 min each, ~3-4 hours total for the 40):
  1. Note what L1 already gives — only override / supplement.
  2. Draft L2-only fields from training data.
  3. WebSearch anything genuinely uncertain (mostly units-sold for
     obscure systems, exact discontinuation dates).
  4. Use `# UNCERTAIN: <reason>` YAML comments inline where
     confidence is low so the operator's review pass catches them.
  5. Commit in family-grouped batches (~5-10 per commit) so voice
     can be refined early before the full 37 land:
     - Pilot batch (3 files, validate voice): saturn + lynx +
       dreamcast (console + handheld + CD-era; operator green-
       lights or asks for voice adjustments)
     - Nintendo handhelds: gbc / gba
     - Sega: sms / gamegear / segacd / sega32x / sega32xcd
     - NEC: tg16 / pce-cd / pcfx
     - SNK: neogeo / neocd / ngp
     - Atari: 2600 / 5200 / 7800 / lynx (lynx already pilot) / jaguar / jagcd
     - Sony: ps2 / psp
     - Nintendo: n64 / nds / gamecube / virtualboy / pokemini
     - 80s home computers / consoles: msx / msx2 / coleco / intv / o2 / channelf / vectrex / wonderswan
     - 3do
     - mame / stv — special (MAME itself is arcade-collective)
     - dosbox / scummvm — engine launchers, no MAME L1, blurb covers what the engine IS not a single system

  Verification:
  - `load_curated_records_parses_all_shipped_yamls` test (already
    in `apps/oa-shell/src/system_info.rs`) asserts every shipped
    YAML parses. Each commit must keep it green.
  - Restart `cargo tauri dev` between batches; HOME panel populates
    with new fields immediately via the bake-on-launch hash detection.

  Risk: my blurb voice may not match the operator's exactly.
  Mitigation: 3-system pilot first; operator green-lights or asks
  for voice adjustments; only then proceed with the other 37.

  Branch shape (when picked up): `feat/system-info-l2-yamls` or
  similar. Pilot is its own commit; family batches each their own
  commit. Final commit updates the
  `load_curated_records_parses_all_shipped_yamls` test's lower
  bound from 5 to ~45 (or removes the bound entirely once all
  systems ship).

  Cross-refs: `docs/PLANS/system-info-panel-v1.md` §10 (notes the
  ~40 outstanding systems); `docs/cores/SCHEMA.md` "system-info.yaml
  Schema reference" section (the field documentation); the 5
  shipped templates at `docs/cores/{snes,nes,genesis,psx,gb}/system-info.yaml`.

- 2026-06-02 — Plugin / Extension API
  Why it came up: ChatGPT advisor session 2026-06-01 flagged "no plugin / extension API" as a missing pillar — third-party developers can't extend OA with custom cores, custom views, or new system support outside the existing libretro-.dll-in-`cores/` mechanism + the planned content-pack system.
  Why deferred: considered + deferred per the non-commercial-gift project model. A plugin API adds maintenance + security + version-compat burden that scales poorly for a one-person + occasional-contributor project. The existing extensibility surfaces (libretro `.dll` drop-in for cores, content packs per `docs/PLANS/content-packs.md` for art + metadata + per-system overrides, theme-CSS hand-patch path) already cover the realistic extension cases. A formal plugin SDK would attract demand for a versioned stable API contract that doesn't fit OA's "operator-first, ship the right thing" posture. Reconsider only if a clear community pull emerges that genuinely can't be served via content packs or libretro cores.
  **2026-06-03 update — PARTIALLY UN-PARKED.** The 2026-06-03 launcher-abstraction reversal (see `docs/DECISIONS.md` 2026-06-03 entry + `docs/PLANS/virtual-library-and-launcher-arc.md`) introduces per-emulator profile YAMLs at `config/emulators/<id>.yaml` that surface operator-editable emulator configuration. This is **not** a generic plugin API — it's a closed set of trait impls (`LibretroLauncher` + `ExternalProcessLauncher`) with constrained per-emulator profile semantics (download URL, launch args template, install location, capability flags), mirroring the per-system-descriptor pattern's discipline. The generic third-party Rust SDK rejection stays in force; only the narrow "operator points OA at additional emulator profiles" case is un-parked.
  Cross-ref: `docs/CHATGPT_BRIEFING.md` (the briefing that surfaced this); `docs/PLANS/per-system-descriptors.md` (the data-consolidation arc that absorbs most plugin-API-flavored needs into the L1/L2/L3/L4 layer model instead); `docs/PLANS/virtual-library-and-launcher-arc.md` (the launcher abstraction + install pipeline that un-parks the narrow case).

- 2026-06-02 — Community curation layer (featured collections / spotlights / "game of the week" platform)
  Why it came up: ChatGPT advisor session 2026-06-01 flagged "no community curation layer" as a missing pillar — OA has no in-product feed of curated collections, no "operator's pick of the week," no spotlights surfaced from a central server.
  Why deferred: classic dead-ecosystem trap — same reasoning as the 2026-05-25 theme-ecosystem entry. Curation platforms need simultaneous demand (operators wanting curated content) AND supply (curators producing it). With OA's user count, neither side reaches critical mass and the maintainer ends up curating everything alone. If featured-collection demand ever materializes, it rides on the existing content-pack distribution channel (`docs/PLANS/content-packs.md`) — a "Game of the Week" pack publishes the same way any other pack does, no separate platform infrastructure needed.
  **Conflict-resolution tooling stays OPEN** (not parked). Distinct from the curation-platform idea — conflict tooling (per-game override diff / rollback / per-pack merge conflicts) may become operator-personal value as the operator accumulates L3 packs + L4 overrides; revisit as part of L4 maturity work when the per-system descriptor consolidation arc lands.
  Cross-ref: `docs/CHATGPT_BRIEFING.md`; `docs/PLANS/per-system-descriptors.md` (defines the L3 pack distribution surface that any future "curation" content would ship through); 2026-05-25 theme-ecosystem parking-lot entry above.

- 2026-06-01 — QuickSettings (in-game) entry point for Shaders + Core options
  Why it came up: the 2026-06-01 Phase D dialog wiring shipped six Shipped-D dialogs (Display / Audio / Shaders / Cheats / Rewind / Milestones / Core options / Input) as TileContextMenu items — the pre-launch surface. While playing a game, the operator should also be able to tweak Shaders + Core options without quitting back to the library; QuickSettings is the in-game tuning surface. Live preview would let the operator iterate on a shader preset against the actual rendered output.
  Why deferred: pure polish — TileContextMenu covers the operator's primary "set this once per game" workflow, and quick A/B testing of presets isn't yet a clear pain point. Revisit when an operator session surfaces actual demand. Restored to PARKING_LOT on 2026-06-03 from a follow-up note that was inline on the now-trimmed `docs/NEXT.md` HIGH-band "Phase D dialog wiring" entry.

- 2026-06-01 — TileContextMenu length / "Per-game settings ▸" sub-view refactor
  Why it came up: 2026-06-01 Phase D wiring brought the TileContextMenu per-game settings count to 11 items (Input mapping / Display / Audio / Shaders / Gameplay / Cheats / Rewind / Milestones / Core options / Game properties / Add to collection). If menu length becomes operator-painful, the existing `Add to collection ▸` sub-view pattern can absorb most of the per-game settings items under a single `Per-game settings ▸` parent.
  Why deferred: 11 items is fine today per operator playtest. Premature to refactor before there's a real readability complaint. Restored to PARKING_LOT on 2026-06-03 from the same inline NEXT.md note as above.

- 2026-06-04 — Retire bespoke `oa://*-progress` event channels in favor of `oa://job-event` as single source of truth
  Why it came up: 2026-06-04 audit §3 (dual-channel state) — every long-running operation fires BOTH a bespoke `oa://library-sync` / `oa://library-metadata-sync` / `oa://rom-hash-resolve-progress` / `oa://library-scan-progress` event AND the generic `oa://job-event` carrying overlapping data. Different frontend consumers wire only one or the other (BackgroundJobsBar reads `oa://job-event` exclusively; LibraryManagerPage + ImportWizard + ingest.ts read the bespoke channels). Drift between the two channels is what produced the original media-sync progress bar bug. The audit-derived sweep introduced JobScope to keep them in sync per-call-site, but the structural fix is to retire one channel entirely.
  Why deferred: multi-file frontend refactor. Each bespoke listener needs to switch to filtering `oa://job-event` snapshots by `target_id` / `system_id` and reconstruct the rich payload (currentRomTitle / lastAction summary lines) from job state instead of from the bespoke event. Doable but spans LibraryManagerPage, ImportWizard, ingest.ts, App.tsx watcher events, plus a backend pass to remove the bespoke `app.emit("oa://*-progress", ...)` calls now that no consumer listens. Out of scope for the 2026-06-04 audit sweep which targeted local discipline fixes; revisit if another operator-reported "progress UI doesn't match registry state" bug surfaces.
  Cross-ref: `docs/DECISIONS.md` 2026-06-04 "Audit-derived sweep" entry "Considered and rejected" section.

- 2026-06-04 — Aggressive publisher-tag stripping in the relaxed disc-filename matcher (v3)
  Why it came up: 2026-06-04 TOSEC-bridge v2 matcher (DECISIONS) cleared 78 of 105 Dreamcast unidentified games but left 27 stragglers; ~10 of those carry TOSEC-style `(Publisher)` paren tags that catalog rows don't have — `Carrier  (Jaleco)(USA)`, `Daytona USA  (Hasbro - Sega)(USA)`, `Grand Theft Auto 2  (Rockstar)(USA)`, `Soul Fighter  (Red Orb)(USA)`, `Stunt GP  (EON)(Europe)`, `GK - Giant Killers  (AAA Game)(Europe)`, etc. A v3 relaxed pass that strips any single-segment paren not in a preserve-keyword whitelist would catch these.
  Why deferred: false-positive risk. The rule shape would be "strip any single-segment paren whose body isn't a region word + isn't a known catalog-variant keyword (Beta/Proto/Demo/Sample/Trial/Kiosk/Taikenban) + isn't a Disc N". This catches the publisher tags but also strips legitimate Redump tags as they land upstream — `(Earlier)`, `(Later)`, `(Greatest Hits)`, `(Player's Choice)`, `(Limited Edition)`, etc. The whitelist would need ongoing maintenance against the Redump dat shape. Cleaner alternative is title-similarity matching (see next entry). Revisit if a future operator audit shows publisher-tag misses are a persistent class > ~15% of library that doesn't get caught by adding the keyword to the strip allowlist case-by-case.
  Cross-ref: `docs/DECISIONS.md` 2026-06-04 "Tiered disc-filename fuzzy matcher" entry "Considered and rejected" section.

- 2026-06-04 — Title-similarity / edit-distance disc-filename matching (v3 alternate)
  Why it came up: same context as above. The OTHER ~10 of 27 DC stragglers + ~5 PSX stragglers miss because the catalog row has a publisher-prefixed or expanded title that the operator's filename doesn't carry — catalog `Disney-Pixar Buzz Lightyear of Star Command (USA)` vs operator `Buzz Lightyear of Star Command (USA)`; catalog `Fisher-Price Rescue Heroes - Molten Menace (USA)` vs operator `Rescue Heroes - Molten Menace (USA)`; catalog `Pac-Man World - 20th Anniversary (USA)` vs operator `Pac-Man World (USA)`. Exact-match keys can't bridge these — would need similarity scoring (trigram, Jaro-Winkler, or edit distance) + a confidence threshold + ranked top-N candidate selection.
  Why deferred: materially larger arc. Needs: (1) a similarity-ranking pass over the same fuzzy-index when both strict + relaxed miss, (2) a confidence threshold that doesn't accept low-quality matches (a 30% confidence pick from the top-N is worse than no match — it stamps the wrong canonical title), (3) an audit UI for the operator to confirm "this title seems close but not exact" matches before they're committed (vs the current strict/relaxed paths which are confident-enough to commit without confirmation). Each piece is doable; the combined surface is several weeks of work for incremental gain. Reconsider when v2's residual is the dominant operator pain point AND the residual clusters in catalog-side title-divergence rather than catalog gaps.
  Cross-ref: `docs/DECISIONS.md` 2026-06-04 "Tiered disc-filename fuzzy matcher" entry "Considered and rejected" section.

- 2026-06-04 — Manual-identify input form in the unidentified-games dialog
  Why it came up: 2026-06-04 unidentified-games dialog (DECISIONS) shipped as read-only + reveal-in-folder so the operator can audit + investigate but can't stamp an identification by hand. Future workflows might want: typing a canonical title manually + having the backend resolve to the matching catalog row, OR pasting a known SHA-1 + looking it up in `rom_hashes` directly, OR a "hide / won't fix" state for truly-uncataloged games (Action Replay, GameShark, etc.) so they stop appearing in the audit list.
  Why deferred: each option is a distinct UI surface with its own persistence semantics. Operator usage of the v1 read-only surface will clarify which workflow is actually wanted — premature to build all three. Specifically watch for: operator re-opens the unidentified dialog after Identify ROMs runs and the remaining list is still substantial; operator manually renames files repeatedly to chase catalog matches (suggests they want a manual-title input bypass); operator complains about specific titles staying in the list permanently (suggests the "hide/won't fix" state is the right shape).
  Cross-ref: `docs/DECISIONS.md` 2026-06-04 "Unidentified-games audit surface" entry "Considered and rejected" section.

- 2026-06-04 — Async I/O cutover Phase 2: core_installer.rs `std::fs::*` → `tokio::fs::*`
  Why it came up: 2026-06-04 spot-audit identified ~4 `std::fs::*` blocking calls inside `run_download_core_inner` async fn body (create_dir_all + metadata + read(zip_partial) + write(dll_partial) + rename for atomic finalize, lines ~1280, 1287, 1530, 1583, 1624). Phase 1 (metadata.rs + rom_hashes.rs cache I/O) shipped on `perf/async-fs-and-render-measurement`; Phase 2 explicitly deferred.
  Why deferred: marginal value is unclear. The core_installer flow is operator-initiated (user clicks Install on a single core or runs Guided Setup's bulk prompt), not a hot per-frame or per-rom path. The blocking ops are mixed sizes — `create_dir_all` + `metadata` are KB-sized and sub-ms; `read(zip_partial)` + `write(dll_partial)` are multi-MB and can block 10-50ms but happen exactly once per install. Phase 2's ~100 LOC refactor needs to preserve resume-logic correctness (Range header math depends on partial-file size from `std::fs::metadata`) and cancellation semantics (already correct under JobScope::resume but the await points shift). Risk : reward is currently inverted vs Phase 1, which was clean 1:1 substitution on hot cache paths. Reconsider when: (a) operator reports UI stutter during single or bulk core installs, (b) the renderer instrumentation (shipped same branch) clears its question and frees up scoping capacity for the next async-pattern work, or (c) a tokio::process::Command cutover lands first that establishes the resume-aware async-I/O pattern across the codebase.
  Cross-ref: `apps/oa-shell/src/core_installer.rs::run_download_core_inner`; Phase 1 lives in `apps/oa-shell/src/metadata.rs::get_system_metadat_cached` + `apps/oa-shell/src/rom_hashes.rs::sync_rom_hashes_for_system` (both wired through `tokio::fs::{read,write,create_dir_all}`).
