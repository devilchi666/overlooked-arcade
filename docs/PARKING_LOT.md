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

- 2026-05-20 — Direct-launch CLI v2 — `--state-file PATH` actual restore
  Why it came up: the direct-launch CLI (shipped 2026-05-20 on `feat/direct-launch-cli`) accepts `--state-file PATH` but doesn't yet wire it through to the emu thread — the frontend logs a warning and the operator falls back to `--slot N`. Operators using LaunchBox's "play from save" feature might want arbitrary save-state files restored at launch.
  Why deferred: needs a new `restore_state_file(path)` Tauri command (or extension of `launch_rom`) that bypasses the per-game slot directory convention and loads a state from an arbitrary path. Two-line implementation but wants a real-world need to ground the file-path semantics (relative-to-cwd? absolute only? expand `~`? validate state-file header?).

- 2026-05-20 — Direct-launch CLI v2 — Multi-instance
  Why it came up: running two `oa-shell.exe` direct-launches in parallel (split-screen on one machine, or "open two games at once" workflows) would let LaunchBox treat OA as a single multi-instance emulator. v1 doesn't support this — log file locking (`oa-current.log` truncate-each-launch convention), singleton libretro core state, and the per-system-default core-pref file (`cores.json`) all assume one process.
  Why deferred: low-demand feature; current operators run one game at a time. Revisit if real-world LaunchBox / BigBox configs surface multi-instance needs.

- 2026-05-20 — Direct-launch CLI v2 — Archive inner-ROM addressing
  Why it came up: `oa-shell.exe "set.zip#inner.nes"` would let a launcher pass a single ROM out of a multi-game .zip directly, without having to extract first. The `archive::extract_for_launch` plumbing already exists for library launches — direct-launch just doesn't parse the `#inner` suffix from CLI args.
  Why deferred: requires CLI to teach about the `<path>#<inner>` encoding (existing library code uses it as a `file_path` column convention), validation that the inner path actually exists in the archive, and the same UnknownExtension / AmbiguousExtension flow on the inner extension. v1 operators with multi-game .zips can scan them into the library first and launch by hash-matched library row.

- 2026-05-20 — Direct-launch CLI v2 — Persistent kiosk profile
  Why it came up: a true kiosk install would auto-launch the same game on every boot (arcade cabinet shipped to a museum, etc.), not just on `--rom` invocation. Operator would configure once, OS auto-runs OA, OA auto-runs the game, no library ever shown.
  Why deferred: needs a persistent on-disk "kiosk mode" flag (`appData/kiosk.json`?), boot-time validation that the configured ROM still exists, and a way for the operator to override-to-library temporarily without erasing the kiosk config. None of these are hard, but no real-world deployment is asking for them yet.
