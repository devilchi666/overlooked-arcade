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

- 2026-06-05 — Per-system controller setup hints to per-game games.yaml
  Why it came up: 2026-06-05 band-aid audit (post-dynamic-controller-info arc) flagged `frontend/src/components/GameDialogs.tsx:842-889` as hand-curated per-system content — 4 Show blocks with text like "Robotron 2084 + twin-stick games: set Port 0 = Standard Pad AND Port 1 = Standard Pad", "Saturn 3D Pad: set Port 0 = '3D Pad / Analog' above", "Wii peripherals: Skyward Sword → Wii Remote + Nunchuk", "Super Multitap (5+ players): Super Bomberman 3 / 4 / 5...". The right architectural home is a per-game `setup_note` or `peripherals_note` field in `config/systems/<id>/games.yaml` — the operator (or content-pack curator) authors per-game hints once, the dialog renders them dynamically when that game is opened.
  Why deferred: this is content-pack-aligned work, not infrastructure. The current hand-curated copy is accurate + useful and only changes when a new quirky multi-controller game gets added. Bigger payoff comes once the content-pack distribution channel (`docs/PLANS/content-packs.md`) lands and per-game hints become community-curatable. Until then, the 4 hardcoded Show blocks are cheap to maintain.
  Reconsider when: content-packs ship + an operator-facing "edit this game's setup note" surface lands; OR a 5th system needs hint copy and the operator notices the boilerplate.
  Cross-ref: `frontend/src/components/GameDialogs.tsx:842-889`; `docs/PLANS/content-packs.md`; the band-aid audit findings table in this session's SESSION_LOG.

- 2026-06-05 — Single-system overlay-eligibility Sets (STYLUS_SYSTEMS, HOTSPOT_SYSTEMS)
  Why it came up: same 2026-06-05 audit identified `frontend/src/components/StylusOverlay.tsx::STYLUS_SYSTEMS = new Set(["nds"])` + two copies of `HOTSPOT_SYSTEMS = new Set(["nds"])` (in `QuickSettings.tsx` + `TouchHotspotOverlay.tsx`) as hand-curated per-system Sets. Could theoretically derive from "core advertises POINTER device" via the new controller-info cache (touch overlays only make sense for systems with stylus input).
  Why deferred: audit-confirmed these are OA-specific UI overlay decisions, NOT band-aid shape. The cores don't publish "should you show a touch overlay" — that's an OA UX call. Plus they're single-system today, no maintenance burden, and the second touch-overlay system (Wii U gamepad if Cemu ever lands? or a future GBA touch-emulation experiment?) would warrant a unified eligibility-check helper at THAT point, not preemptively.
  Reconsider when: a 2nd system needs the overlay (then refactor to a shared helper, optionally driven off the cache).
  Cross-ref: `frontend/src/components/{StylusOverlay,TouchHotspotOverlay,QuickSettings}.tsx`; the band-aid audit findings table in this session's SESSION_LOG.

- 2026-06-05 — Genesis pad-binding family Set (GENESIS_SYSTEMS)
  Why it came up: same audit — `frontend/src/components/GenesisPadReference.tsx:42` hardcodes `new Set(["genesis", "segacd", "sega32x", "sega32xcd"])` as the 4 systems that share Genesis's 6-button-pad reference panel.
  Why deferred: also OA-UX-shaped not band-aid-shaped. Genesis Plus GX is one core covering all 4 OA system_ids, so they genuinely share the same pad binding table — the Set is reflecting an existing-as-data fact (one core, four system slugs). When per-system-descriptors gets an optional `binding_table_family` field this can derive from there; until then the Set is 4 lines of static truth.
  Reconsider when: per-system-descriptor schema grows family-grouping support for any other reason (e.g. MAME ↔ FBAlpha sharing arcade button conventions).
  Cross-ref: `frontend/src/components/GenesisPadReference.tsx`.

- 2026-06-06 — Perf HUD invisible after QuickSettings toggle (single-window gameplay)
  Why it came up: Surfaced during Theming Substrate Phase 2 Slice A playtest. Operator toggled "Show performance HUD" row in QuickSettings during single-window gameplay; the row label flipped to "Hide…" (so the signal IS updating) but the HUD div never appeared at the expected `right-3 top-14 z-[45]` position. Wiring verified intact across the entire arc (`git log` shows `PerformanceHud.tsx` untouched since `3b791c2`; `QuickSettings.tsx` perf-HUD-row path untouched since `dff8097`; the toggle landed pre-Phase-1, so this isn't an arc regression).
  Why deferred: operator chose to ship Slice A and chase perf HUD separately. Need diagnostic info to investigate further — likely candidates: (a) z-index stacking-context trap from a parent provider, (b) Tauri WebView2 compositing oddity with `bg-black/65` over wgpu pixels in single-window, (c) some CSS class on `body[data-shell="single-window"]` shadowing fixed-positioned overlays. Pre-existing bug, not blocking.
  Reconsider when: any operator hits this often enough to want a fix; or when adding the Phase 4 ESLint boundary work happens to touch this region; or when an in-engine-surface Performance toggle gets added (which would also serve as a workaround surface).
  Cross-ref: `frontend/src/components/PerformanceHud.tsx`; `frontend/src/components/QuickSettings.tsx` (perf-hud action row); `frontend/src/App.tsx` lines 245 + 1581 + 1767 (signal + prop wire + mount); 2026-06-06 Phase 2 Slice A playtest exchange.

- 2026-06-06 — Custom-built Vectrex vector renderer (effectively obsoleted by vector-phosphor shader preset)
  Why it came up: long-standing Phase 3+ stretch goal — replace vecx's rasterized output with native wgpu vector-stroke rendering. Listed under NEXT.md DEFERRED with "~500 LOC, Phase 3+" tag for several months.
  Why deferred: The `vector-phosphor` shader preset shipped 2026-05-29 (new `ShaderPreset::VectorPhosphor`, wider-σ 9-tap Gaussian bloom with luminance bright-pass + persistent ping-pong history accumulator at ~80ms half-life). Makes vecx output look like vector strokes with bloom + persistence — operator validated, ships as Vectrex's `defaultShaderPreset`. The shader gets ~95% of the visual win for ~0% of the work the native vector renderer would require. The remaining 5% would be "purity" (no rasterization in the pipeline) — not a functionality fix any operator would notice.
  Reconsider when: an operator can point at a specific vecx output the shader gets wrong AND that a native vector renderer would fix. Pure-vector rendering also unlocks resolution-independent zooming (you can scale Vectrex output without losing detail), which would matter for kiosk/cabinet display modes that don't exist yet — revisit if/when those ship.
  Cross-ref: `docs/cores/vectrex/SESSION_LOG.md` 2026-05-29 entry (vector-phosphor ship); `crates/oa-render/shaders/vector_blur.wgsl` + `persistence.wgsl`; `shaders/presets/vector-phosphor.preset.toml`; the 2026-06-06 NEXT.md DEFERRED-band audit that surfaced this.

- 2026-06-08 — Import Wizard scan appears frozen during the hash/identify pass (no progress feedback)
  Why it came up: Operator imported a 340-file N64 collection (zipped ROMs on E:\) during HW-render M1 work. The Import Wizard's "Scanning… N files scanned" bar sat at ~340 for ~2.5 min with no movement — looked like a hard freeze (operator force-killed once before realizing it eventually continues). NOT a true hang: it completes if you wait.
  Root cause: `run_scan_blocking` (apps/oa-shell/src/scan_service.rs) emits `oa://library-scan-progress` only during the directory walk / archive-peek phase (the single emit at scan_service.rs:201). AFTER the walk it runs `apply_smart_classification` (~scan_service.rs:268), a rayon parallel SHA-1 hash pass over every cart row to identify ROMs by canonical hash — and that pass emits ZERO progress. So the bar stays pinned at the final walk count while it silently decompresses + hashes hundreds of large ROMs (N64 set: 340 archives, ~10–30 MB each, off an external drive). The bigger the collection, the longer the dead-air. The log confirms it: per-archive "peek archive" lines stop at the last file, then minutes of silence with no per-file hash logging, then the "smart-classification … elapsed=Nms" + "end job" lines.
  Fix direction: surface the hash/identify pass in the UI. Either (a) emit `oa://library-scan-progress` from inside `apply_smart_classification` as each row is hashed (thread a progress counter through the rayon pass — careful: it's parallel, so use an AtomicUsize + throttled emit like the walk does), or (b) add a distinct wizard phase/label ("Identifying games… X/Y") so the operator knows hashing is happening and isn't frozen. (a) is the better UX (real percentage); (b) is cheaper. Also consider a per-row or per-pass timeout/skip on the hash decompression so a single pathological archive can't stall the whole pass (the PEEK phase already has ARCHIVE_PEEK_TIMEOUT = 15s at scan_service.rs:37; the hash decompression has no equivalent guard).
  Reconsider when: HW-render M1 wraps, OR any operator hits this and is annoyed enough to want it now (it's a pure UX/feedback fix — the import works, it just looks dead). Not a regression — pre-existing since the smart-classification pass landed; unrelated to the HW-render branch.
  Cross-ref: `apps/oa-shell/src/scan_service.rs` (`run_scan_blocking` ~line 266–274, `apply_smart_classification` ~line 419+, the lone progress emit at line 201, `ARCHIVE_PEEK_TIMEOUT` at line 37); frontend Import Wizard progress UI (`oa://library-scan-progress` consumer).

- 2026-06-11 — Keyboard nav-remap UI + controller-profile auto-config (input-binding completeness)
  Why it came up: building the D18 shell-nav remap Settings UI (`feat/theming-nav-remap-settings`, the gamepad half). Operator asked how *real* inputs bind to our verbs, which surfaced the asymmetry between the two channels:
    • **Gamepad is COMPLETE.** The browser Web Gamepad API normalizes any controller to "standard layout" (button 0 = bottom face, 1 = right, …); `gamepad.ts::BUTTON_NAMES` maps index→NavButton (a/b/x/y/…); `navBindings.gamepad` maps NavButton→verb (the shipped remap UI edits this). So "A/B/X/Y" in the UI ARE the physical buttons. D-pad/stick feed directions structurally.
    • **Keyboard infra is wired but has NO editing UI.** The keydown listener (`focus.ts:214`) already maps `KeyboardEvent.key`→verb DIRECTLY via `navBindings.keyboard` and dispatches any bound key (directional verbs go through `applyDirection`, actions through `dispatchVerb`). Only the four arrows ship in `DEFAULT_BINDINGS.keyboard`; there is no Settings surface to add/rebind keys. So e.g. WASD-for-movement is literally just adding `w→Up, a→Left, s→Down, d→Right` — the dispatch already exists, the **author-it UI** is the only missing piece.
  TODO 1 — **Keyboard nav-remap UI.** A press-to-capture surface (a peer card next to the shipped gamepad "Button mapping" in Settings → Controls → Controller navigation): per verb, "press a key to bind", multiple keys per verb allowed (so WASD + arrows coexist), remove-key chips, Reset to defaults. Cover the directional verbs (the clean WASD case — no native-control conflict) + the action verbs. EXCLUDE Enter/Space from binding (they already activate focused `<button>`s natively — binding them risks a double-fire; this is the "native-control coexistence audit" the `focus.ts` keyboard comment flagged). Escape hatch is structural: **F12** (hardcoded engine-summon, never in `navBindings`) always reaches Settings → Reset, so a user can't strand themselves. Estimated small (the model + dispatch are done; it's a capture-mode UI on the existing `navBindings.keyboard` map + `setNavBindings`).
  TODO 2 — **A real default keyboard map.** Today only arrows→movement. Want a sensible out-of-box keyboard map (e.g. arrows + WASD → movement, Enter/Esc handled natively, maybe Tab/Shift-Tab → sections) shipped in `DEFAULT_BINDINGS.keyboard`, so keyboard-only operators get a usable shell without touching Settings.
  TODO 3 — **Per-controller-ID gameplay-binding auto-config (bigger, separate arc).** Operator flagged: down the road, auto-set the GAMEPLAY bindings (the per-system emulated-console mappings — `SystemBindingsEditor`, NOT the nav layer) per detected controller by id/profile, across multiple systems. I.e. plug in an 8BitDo / DualSense / Xbox pad and OA recognizes it (Gamepad `id` string + button/axis count, cf. the existing HID HAT-axis decode reference) and applies a known-good default mapping for each system's core, instead of the operator hand-binding every system. This is input-infrastructure, distinct from the shell-nav verbs — it lives over `oa-input` + the per-system bindings, and pairs naturally with a controller-profile registry (per-vendor default maps).
  Why deferred: operator's call — the shipped gamepad nav-remap covers the immediate need, the keyboard infra already dispatches, and it all lives in platform Settings so the keyboard UI can land anytime without blocking. TODO 3 is a meaningful input arc of its own, best scoped when the multi-system gameplay-binding UX is the focus.
  Reconsider when: a keyboard-only operator wants WASD/rebinding (TODO 1+2 — small, do together); OR the project picks up a dedicated controller/input arc (TODO 3, with a controller-profile registry).
  Cross-ref: `frontend/src/platform/nav/{navBindings.ts,focus.ts:214,gamepad.ts:24}`; the shipped gamepad remap in `frontend/src/engine/SettingsSections.tsx::NavRemapCard` + `navBindings.ts::rebindGamepadVerb`; DECISIONS D30; the HID HAT-axis decode reference (memory) for non-standard pad detection; `SystemBindingsEditor` (gameplay bindings) for TODO 3.

- 2026-06-11 — Platform-owned DevTools / console seam (replacing the deleted Retroverse-coupled one)
  Why it came up: Theming Phase 6 (Retroverse-as-theme, `feat/theming-retroverse-as-theme`) deleted App.tsx's `__retroverse_debug` DevTools global. It was a DEV-only `window.__retroverse_debug` helper (currentRoute / setRoute / cycleForward/Backward) wired in App.tsx against Retroverse's private `currentRoute` tab-routing signal — a Phase-A scaffold from before Retroverse's real tab strip existed. Phase 6 made `currentRoute` theme-private (moved into `themes/retroverse/`), so a debug global hardcoded in App.tsx against one theme's route model is both obsolete (the tab strip exercises the signal now) and an architectural smell (App.tsx reaching into theme internals — the very coupling Phase 6 removed). Deleted per operator call.
  Why deferred: nothing currently needs it (the tab strip + the in-app Debug log view cover today's needs). When dev hooks ARE wanted again, the proper home is **platform**, not App.tsx against a theme: a small theme-agnostic dev-console seam (e.g. `platform/devtools.ts` exposing a namespaced `window.__oa_dev` with registerable probes) that any active shell — Retroverse, CoverFlow, bare, future themes — can contribute to, gated on `import.meta.env.DEV`. That keeps App.tsx free of theme-internal reaches and gives every theme the same affordance instead of privileging Retroverse.
  Reconsider when: a debugging need surfaces that the in-app Debug log (`Help → Debug log…`) + browser DevTools don't cover — e.g. wanting to script the active theme's nav/route state from the console across themes; or a Theme Studio (ARC 3) wants a live inspection hook.
  Cross-ref: deleted block was `frontend/src/App.tsx` (the `__retroverse_debug` onMount, removed in Phase 6 C2); DECISIONS D31 (theming-substrate); `frontend/src/themes/retroverse/currentRoute.ts` (the now-theme-private signal it drove); the three-output logger (`docs/DECISIONS.md` 2026-05-18) + `Help → Debug log…` as today's runtime-inspection surface.

- 2026-06-12 — "Hide BIOS" library-view filter + manual "mark as BIOS" override
  Why it came up: while building the Metadata editor (S3), operator asked whether to add "BIOS" as a Release type so BIOS entries can be filtered out of views. Concluded that's the wrong layer — BIOS is an intrinsic entity kind, not a game release kind, and OA already auto-detects it (`title_parse.rs::is_bios` from No-Intro/TOSEC `(BIOS)`/`[BIOS]` flags; flows to `GameVariant.is_bios`; the parser comment notes "the Preservation Vault filter excludes BIOS by default").
  Why deferred: this is library-views work, not metadata-editing — keep it out of the Metadata arc. Two pieces: (1) a **"Hide BIOS" view/collection filter** reading the existing `is_bios` flag (ideally on by default in browse views), and (2) a small **manual "mark as BIOS / not a game" override** for entries whose filename lacks the `(BIOS)` marker so auto-detection missed them (intrinsic-kind flag — NOT a Release-type value; would need an override field + read-path merge like the metadata layer).
  Reconsider when: a library-views/filtering arc is in scope, or BIOS clutter in views becomes a real playtest annoyance. Do (1) first (covers most of the value automatically); (2) is the belt-and-suspenders escape hatch.
  Cross-ref: `apps/oa-shell/src/title_parse.rs` (`is_bios`, `is_bios_flag`); `frontend/src/platform/library/types.ts` (`isBios` on the variant); `library_groups.rs` (`GameVariant.is_bios`); the "Preservation Vault filter" mention in title_parse.rs; the metadata override-layer pattern (`game_metadata_overrides`) as the model for a manual-flag override if pursued.

- 2026-06-12 — Updatable bundled data files (gamecontrollerdb + others)
  Why it came up: the Controller Identity arc bundles SDL `gamecontrollerdb` (controller mappings), which the upstream community revises constantly; OA also ships other reference data that drifts over time (e.g. metadata/No-Intro-style naming data, BIOS hash lists, future scraper indexes). We need a general story for refreshing these without shipping a whole new build — operator-triggered "check for updates" and/or a periodic fetch, with a bundled snapshot as the offline floor.
  Why deferred: Controller Identity Phase 2 ships a bundled snapshot, which is enough to function; the update/refresh mechanism is a cross-cutting infra arc of its own (versioned data manifests, fetch + cache + verify, settings UI, offline fallback). Plan it as a dedicated arc later, not inline with the controller work.
  Reconsider when: the first bundled data snapshot goes visibly stale (a common pad missing from our shipped gamecontrollerdb), or a second drifting data file lands and the "how do we update this?" question repeats. Design once, cover all such files. NOTE the emulator-offline rule (CLAUDE.md "No network calls from emulator code") — this is shell/launcher-side update tooling, not emulator-runtime networking.

- 2026-06-12 — Per-controller glyph sets (Nintendo / Xbox / PlayStation button symbols)
  Why it came up: Controller Identity Phase 2 — operator noted the on-screen hint glyphs don't match their controller's physical labels. This is a presentation-layer concern (Layer 3): show A/B/X/Y in Nintendo positions for a Switch pad, ▲○✕□ for PlayStation, etc., keyed by the controller identity this arc already resolves. Distinct from mapping correctness (Layer 1).
  Why deferred: operator chose to PROVE the mapping (Layer 1, via the test window) before investing in cosmetic glyph sets — conflating the two makes both harder to debug. Cousin of dynamic-input-descriptors (Phase 6, per-game *labels*), but the physical button *symbols* are this arc's job.
  Reconsider when: the test window confirms mapping is solid; then wire identity → glyph set. Cross-ref: controller-identity DECISIONS D15; the nav glyph system (`frontend/src/platform/nav/glyphs.*`).
  **2026-06-12 — Partially unparked.** The TEXT labels shipped (DECISIONS D16): `controllerFamily.ts` + the SDL-derived `controllerTypes.json` resolve a label family (Nintendo/Xbox/PlayStation/generic) and the test window shows each pad's real A/B/X/Y labels. Only the pictographic GLYPH ICONS (symbol art in the hint bar) remain parked — and now have a clean hook: `family → glyph set` reusing `resolveFamily()`.

- 2026-06-12 — Reduced-layout control schemes (pads missing shoulders / sticks / buttons)
  Why it came up: Controller Identity — operator noted some controllers lack shoulders (and arcade sticks lack sticks/many buttons), so a fixed nav scheme that assumes L1/R1 for tab-cycling etc. won't be reachable on all hardware. Need verb-fallback schemes so every nav verb is reachable on a minimal button complement.
  Why deferred: depends on the remap/verb-binding UI + the wizard (later slices of this arc); design it alongside those, not before mapping is proven.
  Reconsider when: the remap UI / Phase-3 wizard is in scope. Cross-ref: `navBindings.ts` (verb map), controller-identity DECISIONS D14/D15.

- 2026-06-12 — Arcade-cabinet keyboard-encoder input (iPAC-style joystick/button/spinner → keystrokes)
  Why it came up: Controller Identity — operator flagged that cab makers route joysticks/buttons/spinners/trackballs through keyboard-encoder boards (iPAC, KADE, etc.) that emit keystrokes, not gamepad HID. Supporting cabinets means a whole input path that looks like a keyboard but is really N players' physical controls, plus spinner/trackball relative-axis handling.
  Why deferred: "major planning" per operator — a dedicated arc (encoder profiles, per-player key→canonical mapping, relative-axis devices) well beyond the current pad-identity work.
  Reconsider when: cabinet/arcade deployment becomes a target. Cross-ref: the keyboard mapping layer (`crates/oa-input` `KeyboardMapping`); controller-identity arc.

- 2026-06-14 — Re-add DevTools access (dev-only) to Settings → About
  Why it came up: operator noticed the WebView DevTools were removed by mistake a while back, and F12 (the old open-devtools key in some setups) now summons the engine surface. During development it's useful to open the WebView inspector.
  What: add a dev-only "Open DevTools" affordance in Settings → About (Tauri `WebviewWindow.openDevtools()` / the `devtools` Cargo feature, gated to debug builds). Don't ship it in release.
  Status: ✅ SHIPPED 2026-06-14 (per-system-hub arc) — `engine/DevToolsPanel.tsx` in Settings → About: logging toggles (spatial nav / focus nav / gamepad-raw), Open inspector (Rust `open_devtools`, debug_assertions-gated), Spawn test job, Copy log path, Open logs folder, Reload UI, Restart app, data-dir readout.
  REMAINING (re-gate before any PUBLIC release — the operator builds with `cargo tauri build`, a RELEASE build, for their own use, so the dev tooling must work there): (1) the panel renders UNCONDITIONALLY because `import.meta.env.DEV` is false in a built bundle — re-gate behind a reliable build-time check; (2) `devtools` is in oa-shell's DEFAULT Cargo features so the WebView inspector + `open_devtools` work in their release builds — move it to an opt-in `--features devtools` before shipping to end users. Both are intentional for now so the single-user dev workflow has DevTools.

- 2026-06-14 — "system" vs "platform" terminology audit across the UI
  Why it came up: splitting per-system metadata into "Platform Metadata" (the console's facts) vs "Game Metadata" (the games' facts) in the Systems hub, the operator picked "Platform Metadata" but flagged that OA uses "system" everywhere else (system id, per-system, Systems hub, System Health) while much of the emulator scene + other frontends (LaunchBox "Platforms", RetroArch "Systems"/"Cores", EmulationStation "systems") mixes both.
  What: survey how leading frontends (LaunchBox, ES-DE, RetroArch, Playnite) + the broader scene use "system" vs "platform", decide on ONE canonical term for OA's UI, and apply it consistently (sidebar labels, hub, health, metadata, docs). Currently inconsistent: code/ids use "system"; the new metadata card says "Platform".
  Reconsider when: doing a UI-copy/terminology pass, or before any public-facing release where consistency matters. Low-risk, copy-only (ids/keys stay "system" regardless).
