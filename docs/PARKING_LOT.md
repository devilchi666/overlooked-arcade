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
