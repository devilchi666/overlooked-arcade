# Next — cross-system priority queue

What to ship next across the project, ordered by leverage. **Per-system status lives in `docs/cores/<id>/ROADMAP.md`** — this file is just the cross-system view of what to pick up next when you have a fresh session.

Each item: short scope, rough line estimate, gating (operator-driven / blocked on infra / ready to ship), where the work lives.

When you close an item, the matching PR also flips the corresponding `⬜` to `✅` in the relevant per-core ROADMAP — see CLAUDE.md "ROADMAP hygiene" for the policy.

---

## Pipelined sequence (three major arcs interleaved)

**Decided 2026-05-26.** Three major arcs below — Guided Setup, Per-System Custom UI, and Game Info Panel — share a foundation and can pipeline through subsequent stages.

### The shared foundation: Phase 0 — Controller-nav primitives ✅ SHIPPED 2026-05-26

Merged to main as `feat/controller-nav-primitives` (5 phase commits). See
[docs/features/controller-nav/](features/controller-nav/) for the slice
breakdown + decisions. Five deliverables landed:

- ✅ Focus manager — `nav/focus.ts::useFocusGroup` (vertical / horizontal / grid)
- ✅ Gamepad → UI event layer — `nav/gamepad.ts` Web Gamepad API rAF poller
- ✅ Focus-ring component pattern — `[data-oa-focus="true"]` 2px outline (in frontend/src/index.css)
- ✅ On-screen hint bar — `nav/HintBar.tsx` with `HintRegion` provider stack
- ✅ Settings → Controller-nav — Display dialog gains a Controller navigation section

A follow-on **completion pass** (`feat/controller-nav-completion`,
merged to main 2026-05-26) extended focus + back-stack coverage to
every remaining interactive surface — global back stack, sidebar
containers, every Dialog, top toolbar menu bar, chained popovers
(CorePicker / RegionPicker), right-sidebar action row, plus a fix to
suppress the frontend gamepad poller while gilrs owns input and three
post-test fixes (library grid DPad wrap-across-rows, menu bar focus
ring visibility + disabled filter + dynamic content support, and a
cross-cutting `data-oa-focus-active` CSS broadening). See
[features/controller-nav/ROADMAP.md](features/controller-nav/ROADMAP.md)
"Completion pass (post-Phase 0)" for the slice inventory and
[features/controller-nav/SESSION_LOG.md](features/controller-nav/SESSION_LOG.md)
for the 2026-05-26 completion-pass entry. Per-System UI Stage 1 is
the next major arc.

### Strict sequence to the inflection point

```
Phase 0 ✓ (controller-nav, shipped 2026-05-26)
       ↓
Per-System UI Stage 1 (polish layer, ~5-7w) — IN FLIGHT 2026-05-26
   - ✅ Slice 1 — SystemUIConfig data model + registry baseline +
        Settings → Display "Per-system experiences" master toggle +
        prefers-reduced-motion plumbing + feature-folder scaffold
        (merged to main 2026-05-26)
   - ✅ Slice 2 — Per-system SFX wiring: Rust `resolve_ui_sound`
        resolver cascade (operator override → per-system bundle →
        `_baseline` → silence), frontend `playSystemUiSound` helper
        gating on master toggle + audioProfile, library-grid
        navigate / launch call sites (merged to main 2026-05-26)
   - ✅ Slice 3 — Per-system background renderer: new
        `apps/oa-shell/src/system_ui_assets.rs` Rust module with
        `resolve_background_asset` cascade + `<SystemBackground>`
        component rendering static (gradient + optional image),
        animated (looping `<video>`), or shader (fallback to static
        until Slice 8) paths. Source chain: hover → focused →
        activeView → pinned. Merged to main 2026-05-27. Static path
        operator-validated; animated path code-complete pending
        Slice 7 NES pilot content.
   - ✅ Slice 4 — Boot animation framework: SystemBootAnimation
        component triggered by `activeSystemId` transition (sidebar
        entry), `oa-boot-fade` CSS keyframe. Toggle semantics
        (refined after playtest): sub-toggle OFF → no overlay
        (instant), ON + no reduced-motion → 1 s full,
        `prefers-reduced-motion` → 200 ms cross-fade as the
        accessibility floor. Per-system `boot-intro` SFX dispatched
        whenever the visual fires. Skippable on any input. Settings
        sub-toggle gated on master. `boot-intro` event added to
        `resolve_ui_sound` so pilots can dispatch the SFX. Merged
        to main 2026-05-27.
   - ✅ Slice 5 — Tile flourish system: tileShape enum →
        aspect-ratio override on the cover container (+ rounded-full
        for circle); interactionStyle enum → `data-oa-interaction`
        attribute driving CSS transition timing + hover transform
        (delayed = 360 ms LCD-feel; physical = 220 ms spring +
        click pulse). Baseline `instant` keeps Tailwind defaults.
        Master toggle off falls back to today's behaviour. Merged
        to main 2026-05-27.
   - ⬜ Slice 6 — Game Boy pilot full build
   - ⬜ Slice 7 — NES pilot full build
   - ⬜ Slice 8 — Vectrex pilot + custom-component escape hatch
   - ⬜ Slice 9 — Per-core README "Per-system UI" sections
   - See [features/per-system-ui/ROADMAP.md](features/per-system-ui/ROADMAP.md)
       ↓
Game Info Panel v1 (polish for Per-System Stage 1, ~3-4w)
   - YAML front-matter data model + parser
   - KNOWN_GAME_BUGS migration into structured per-game entries
   - Tile-hover card + long-press full panel + tile badge
   - Operator "Edit locally" via SQLite override table
   - Inline "Apply best emulator" + "Apply controls" actions
   - "Submit correction" surface (stubbed for v1 — clipboard copy)
       ↓
[INFLECTION POINT — ≈ 10-14 weeks from green-light]
```

**Why this order:** Per-System Stage 1 is the identity moment — it makes OA feel different from the field. Game Info Panel v1 is the **practical complement** — once every system feels alive, the natural next ask is "what is THIS specific game about, what version is it, will it work, which core is best?" Shipping the info panel as polish on top of Stage 1 lands the operator's first complete-feeling experience: themed library + per-game depth. Onboarding polish (guided setup) and behavior depth (Per-System Stages 2-3) come after, against a much richer product.

**No interleaving until those three are done.** Discipline matters. Half-finishing multiple arcs is the failure mode this sequence avoids.

### After the inflection point — interleave by session feel

```
Phase 0 ✓
Per-System Stage 1 ✓
Game Info Panel v1 ✓
       ↓
   ╔════════════════════════════════════════════════════════════╗
   ║  Pick by session — all tracks pipeline freely              ║
   ║                                                            ║
   ║  Guided Setup Track (~5-6w cumulative):                    ║
   ║    Phase 1B  Wizard upgrade (~3-4w)                        ║
   ║    Phase 2B  Curated core selection (~1w)                  ║
   ║    Phase 2C  Folder management (~1w)                       ║
   ║    Phase 2D  First-system bindings + KNOWN_GAME_BUGS (~1w) ║
   ║                — auto-applies per-game core overrides from ║
   ║                  the same KNOWN_GAME_BUGS data this plan   ║
   ║                  migrated; shared infrastructure win       ║
   ║    Phase 2E  Help suppression (~3-4d)                      ║
   ║    Phase 2F  Existing-operator re-entry (~3-4d)            ║
   ║                                                            ║
   ║  Per-System UI Stage 2 — Behavior layer (~4-6w):           ║
   ║    Per-system navigation (carousel / list / wheel)         ║
   ║    Per-system interaction style (instant / delayed /       ║
   ║      physical)                                             ║
   ║    Per-system tile emphasis                                ║
   ║    5-10 more systems tuned to showcase tier                ║
   ║                                                            ║
   ║  Per-System UI Stage 3 — Experience layer (~6-10w):        ║
   ║    In-game overlays themed per system                      ║
   ║    Library ↔ game transitions themed                       ║
   ║    Per-system metadata priorities (consumes Game Info      ║
   ║      Panel fields for the per-system priority routing)     ║
   ║    All ~40 systems tuned past baseline                     ║
   ║                                                            ║
   ║  Game Info Panel v2 (~3-5w, infra-heavy):                  ║
   ║    Scraper infrastructure (GitHub Actions on data repo)    ║
   ║    Separate overlooked-arcade-game-info data repo          ║
   ║    Daily auto-sync from data repo to OA installs           ║
   ║    GitHub Issue → auto-PR community contribution flow      ║
   ║    Wikipedia/etc richer-source integration (later)         ║
   ╚════════════════════════════════════════════════════════════╝
```

Each phase is a shippable PR. Pick whichever feels right session-to-session. Order across phases doesn't matter after the inflection point; there are no hard dependencies.

### Total estimate

- **Phase 0 + Per-System Stage 1 + Game Info Panel v1 (the inflection point):** ~10-14 weeks. Foundation + identity-defining demo + per-game depth. Shippable as a complete inflection on its own.
- **Full vision (all three arcs through Per-System Stage 3 + Game Info Panel v2):** ~25-37 weeks.

### Shared-infrastructure savings

Pipelining compounds code reuse:
- Focus manager + hint bar + audio dispatcher built in Phase 0 power all three arcs throughout
- `SystemUIConfig` registry pattern (Per-System Stage 1) reuses the shape of `LIGHT_GUN_SYSTEMS` (shipped 2026-05-25) — same declarative-table pattern across systems
- Per-system SFX (Per-System Stage 1) routes through the existing 4-bus audio mixer (shipped 2026-05-24 in media-taxonomy)
- Per-system bindings card (Guided Setup Phase 2D) reuses the same per-system theming + audio that Per-System Stage 1 builds
- **Structured per-game data format (Game Info Panel v1) is consumed by**: Guided Setup Phase 2D (auto-apply per-game core overrides from KNOWN_GAME_BUGS at import commit) AND Per-System UI Stage 3 (`metadataPriority` field drives per-system priority routing using the same fields). Three features share one structured source — defining it once unlocks all three.

Probably 15-25% off the total vs running the three arcs as fully separate work streams.

### Kiosk shell scheduling — separate, after the full pipeline

The kiosk shell ([docs/features/kiosk-shell/KIOSK_PLAN.md](features/kiosk-shell/KIOSK_PLAN.md)) is its own major arc, scoped at multi-month effort. After this plan locks, kiosk shell's positioning shifted (per 2026-05-26 DECISIONS Q): it becomes the theme editor for power users that **consumes the built-in per-system experiences as starting defaults**. Kiosk shell scheduling happens after the per-system-UI / guided-setup pipeline ships, when there's a richer product to wrap a kiosk mode around.

---

## NEXT MAJOR ARC — Guided Setup

**Planning locked 2026-05-25.** Full plan at [docs/PLANS/guided-setup.md](PLANS/guided-setup.md).

Upgrade the existing Import Wizard into a guided-setup flow:
- Smart ROM/system matching (hash → header → extension → folder-hint)
- Per-system readiness checklist (single component, reused in Settings)
- Curated CPU-tier core selection (`sysinfo` crate + per-system tier table; no benchmarking)
- Controller-navigable from day one (DPad + focus rings, Steam Big Picture style)
- Optional canonical folder layout (opt-in, mode-aware default)
- Per-game KNOWN_GAME_BUGS overrides applied at commit
- Help / tip suppression with criticality tier (load-bearing alerts never suppressible)

**Audience priority:** couch gamers primary, cabinet builders secondary (kiosk shell later), desktop users tertiary (already served).

**Voice:** warm + curator/enthusiast. Sample copy in the plan.

**Phase 0 = controller-nav primitives** (~2-3 weeks frontend infrastructure: focus manager, gamepad → UI event layer, focus-ring component pattern, on-screen hint bar). Shippable independently — makes a few existing screens controller-navigable as a proof-of-concept before the wizard work.

**Phases 1-6** (~6-7 weeks): wizard upgrade, curated core selection, folder management, first-system bindings + KNOWN_GAME_BUGS, help suppression, existing-operator re-entry.

**Total estimate:** 8-10 weeks of focused work. Awaiting operator green-light to start Phase 0.

Dwarfs the MEDIUM band below — when implementation starts, this arc dominates the roadmap for ~2 months. MEDIUM-band shader work + light-gun playtest can pipeline alongside if multiple sessions overlap.

---

## NEXT MAJOR ARC — Per-System Custom UI

**Planning locked 2026-05-25 → 2026-05-26.** Full plan at [docs/PLANS/per-system-ui.md](PLANS/per-system-ui.md).

Make each system feel like its own mini-experience. Per-system audio, boot animations, navigation behavior, layout structure, tile flourishes. This is the **default OA experience** (not a power-user feature); a "Per-system experiences" toggle in Settings lets the minority who want a uniform plain library opt out.

Shipped in three stages, each fully working:

- **Stage 1 — Polish layer** (~5-7 weeks): `SystemUIConfig` data model + per-system SFX + boot animations + tile flourishes + per-system backgrounds + Settings toggle. 3 pilots fully built (Game Boy → NES → Vectrex); all 37 other systems get a tasteful baseline config so the whole library feels themed.
- **Stage 2 — Behavior layer** (~4-6 weeks): per-system navigation (grid / carousel / list / wheel), per-system interaction style (instant / delayed / physical), per-system tile emphasis. Library view only; in-game UI uniform. 5-10 more systems tuned to showcase tier (Jaguar, PS1, Saturn, MAME, TG-16 candidates).
- **Stage 3 — Experience layer** (~6-10 weeks): in-game overlays (pause, quick settings, save-state UI) themed per system. Library ↔ game transitions themed. Per-system metadata priorities. All ~40 systems tuned past baseline.

**Architecture:** hybrid. Config-driven SystemUIConfig DSL for most systems; per-system Solid component escape hatch for signature cases (Vectrex confirmed; others TBD).

**Audio sourcing:** multi-source. CC0 pack baseline + original recordings for pilots + AI-generated for hard-to-source synthesized sounds (Vectrex vector blips). No community submission on the desktop normal version (theme ecosystem WAIT lock unaffected).

**Mode separation locked:**
- **Themed** (default ON): per-system custom UI as designed
- **No theme** (Settings toggle OFF): uniform plain library; no audio, no animations, no flourishes
- **Kiosk** (future, separate plan): theme editor for power users; consumes built-in per-system experiences as starting defaults

**Total estimate:** ~15-23 weeks across all three stages. Stage 1 alone is shippable as a real feature (~5-7 weeks).

**Status (2026-05-26):** Stage 1 is in flight on
`feat/per-system-ui-stage-1-slice-1`; foundation slice (data model +
toggle + reduced-motion plumbing) shipped, awaiting operator
playtest before Slice 2. Tracked at
[features/per-system-ui/](features/per-system-ui/).

**Order vs guided-setup is deferred.** Both arcs are multi-month. Options: (a) sequence — finish guided-setup first, then this; (b) parallel — pipeline if multiple sessions overlap, sharing controller-nav primitives between guided-setup Phase 0 and per-system-UI Stage 1; (c) inverse — this first, then guided-setup. Operator's call.

---

## NEXT MAJOR ARC — Game Info Panel

**Planning locked 2026-05-26.** Full plan at [docs/PLANS/game-info-panel.md](PLANS/game-info-panel.md).

**Scheduling: ships as polish on top of Per-System UI Stage 1** in the strict-sequence portion of the pipeline (see "Pipelined sequence" above). Third step after Phase 0 + Per-System Stage 1.

Surface structured reference data per game in OA's library — date, publisher, region, version, player count, controls supported, known bugs, best-emulator recommendations, operator-editable short summary. **Not editorial, not recommendations** (those would belong in a future Play History Intelligence feature).

**v1 scope (tight, ~3-4 weeks):**
- YAML front-matter data model in per-system markdown (`docs/cores/<id>/games-info.md`)
- One-time migration: existing `KNOWN_GAME_BUGS.md` free-form markdown → structured entries
- Tile-hover compact card + long-press / `i` full panel + tile badge for known issues
- Operator local edits in SQLite override table; field-typed precedence merges sources
- Inline "Apply best emulator" + "Apply controls" buttons wire to existing `GameOverrides`
- "Submit correction" surface stubbed (clipboard copy + informational toast) for v1

**v1 sources:** supplied `.dat` files (libretro-database) that OA already syncs + KNOWN_GAME_BUGS migration. No scraper running. No separate data repo. No community pipeline.

**v2 architecture FULLY DESIGNED but DEFERRED** (~3-5 weeks when it lands):
- Scheduled scraper in GitHub Actions on the data repo
- Separate `overlooked-arcade-game-info` data repo (lower contribution bar, cleaner versioning)
- Daily auto-sync from data repo to OA installs + manual "check now" button
- GitHub Issue → auto-PR community contribution flow with maintainer review
- Wikipedia / TheGamesDB / ScreenScraper richer-source integration paths

**Shared infrastructure with other arcs:**
- Guided Setup Phase 2D auto-applies per-game core overrides using the same structured KNOWN_GAME_BUGS data this v1 migrates — one structured source, two features consuming it
- Per-System UI Stage 3 `metadataPriority` field drives per-system priority routing using the same fields this plan defines

**Distinct from theme ecosystem WAIT lock (DECISIONS G).** Game info is a factual database, not a creative ecosystem. Dead-ecosystem trap doesn't apply — value exists at v1 even with zero community contributions because OA ships with seed data from existing `.dat` sources.

---

## HIGH — ready to ship next

These are operator-independent and the infrastructure they sit on already exists.

(Empty as of the last audit pass. The HIGH batch ran 2026-05-21 and closed: disc-id extraction across all CD systems, multi-repo cover sync for gb + wonderswan, N64 .v64/.n64 byte-swap, N64 WASD analog default.)

When something lands in this bucket, name it concretely (`apps/oa-shell/src/<path>` + scope + estimate) so the next session can pick it up without re-deriving.

---

## MEDIUM — Phase 3+ polish

~~1. Dedicated `vector-phosphor` shader preset for Vectrex~~ —
   **SHIPPED 2026-05-29** on `feat/vectrex-vector-phosphor-shader`.
   New `ShaderPreset::VectorPhosphor` (id=5) + wider-σ (9-tap σ≈2.5)
   Gaussian bloom with luminance bright-pass + persistent ping-pong
   history accumulator at ~80ms half-life. New files:
   `crates/oa-render/shaders/vector_blur.wgsl`,
   `crates/oa-render/shaders/persistence.wgsl`,
   `shaders/presets/vector-phosphor.preset.toml`. Vectrex's
   `defaultShaderPreset` flipped `crt-lite` → `vector-phosphor`.
   Operator design input locked: white tint, σ≈2.5 bloom, ~80ms
   persistence. Per-`docs/cores/vectrex/SESSION_LOG.md` 2026-05-29
   entry + ROADMAP flip.

~~2. Dedicated `vb-monochrome` shader for Virtual Boy~~ —
   **SHIPPED 2026-05-30** on `feat/virtualboy-completion-pack`.
   New `ShaderPreset::VbMonochrome` (id=6) — pure-red palette
   enforcement + vertical scanline darken at the source-column rate
   (mimics the VB's spinning-mirror LED column scanner) + soft
   circular vignette (eyepiece framing). Single-pass — branches in
   `blit.wgsl`. `themes/registry.ts` virtualboy `defaultShaderPreset`
   flipped `plain` → `vb-monochrome`. Operator design locked:
   vertical scanlines + soft vignette + red-only (no visor reflection
   in v1 — would obscure gameplay). Per
   `docs/cores/virtualboy/SESSION_LOG.md` 2026-05-30 entry + ROADMAP
   flip.

~~3. Per-system `lcd-handheld` default binding~~ — **SHIPPED 2026-05-24**
   alongside the media-taxonomy wave. `defaultShaderPreset: "lcd-handheld"`
   wired in `frontend/src/themes/registry.ts` for `gb` / `gbc` / `gba` /
   `gamegear` / `ngp` / `wonderswan` / `pokemini` / `psp`. Per-core ROADMAPs
   flipped ✅ for each. Operator validation against real handheld captures
   remains a stretch polish item but doesn't gate the default.

4. **Jaguar KP8–KP_HASH keyboard-passthrough dispatch** (~80 lines).
   - Bits 16-20 in `bindings.rs::jaguar` already exist. Wire libretro `RETRO_DEVICE_KEYBOARD` events from the upper-bit presses through to Virtual Jaguar.
   - Wants VJ-specific RETROK keycode validation against a running core.

~~5. Multi-system light-gun smoke-test validation~~ — **SHIPPED 2026-05-25**
   on `feat/light-gun-harness`. Original audit framing was wrong:
   "POINTER device dispatch is shipped" only covered the touch/stylus
   shape (NDS). Most classical light-gun cores (FCEUmm Zapper, snes9x
   Super Scope, Genesis Plus GX Light Phaser, Beetle Saturn Virtua
   Gun, Beetle PSX GunCon, Flycast HotD) poll `RETRO_DEVICE_LIGHTGUN`
   (id=4), not POINTER (id=6). Pre-fix `cb_input_state` rejected
   everything that wasn't JOYPAD/POINTER → light-gun cores got zeros.
   This branch adds the LIGHTGUN branch (in
   `crates/oa-libretro/src/state.rs::lightgun_field_value`) wiring
   SCREEN_X / SCREEN_Y / TRIGGER + deprecated relative X/Y aliases.
   AUX / START / SELECT / DPAD / RELOAD return 0 (Phase 2 Bindings UI
   work). IS_OFFSCREEN also returns 0 — proper reload-by-aim-off-screen
   needs an `in_viewport` flag on InputState (Phase 2). 18 new unit
   tests across `oa-libretro`, `oa-input`, `oa-shell::light_gun_systems`
   cover both dispatch helpers + viewport coord math edge cases
   (sweep monotonicity, out-of-viewport sentinel, extreme-coord
   clamping). Declarative `apps/oa-shell/src/light_gun_systems.rs`
   table catalogues nes/snes/sms/saturn/psx/dreamcast/nds with their
   expected device type + flagship test title + validation status.
   Per-system operator playtest is the remaining work — code is
   ready.

~~6. Full media taxonomy + LaunchBox-shape storage~~ — **SHIPPED 2026-05-24**
   on `feat/media-taxonomy` (`--no-ff` merge to main). 7 phase commits;
   see [docs/features/media-taxonomy/SESSION_LOG.md](features/media-taxonomy/SESSION_LOG.md)
   for per-phase ship details + commit shas. Followup stretch polish
   (audio override UI surfaces, kiosk wheel-art consumption) lives
   in [PARKING_LOT.md](PARKING_LOT.md).

~~7. scummvm + dosbox onboarding~~ — **SHIPPED 2026-05-24** across two
   `--no-ff` merges:
   - Phase 1 (scummvm, `0b56bd8`): `feat/dosbox-and-scummvm` —
     SystemId variant + bindings + `.scummvm` descriptor routing
     through `RomSource::Path` + per-core `system_dir` subdirectory
     + keyboard passthrough + frontend theme + per-core docs.
   - Phase 2 (dosbox, `b6fea2c`): `feat/dosbox-onboarding` — SystemId
     variant + bindings + new `is_directory_path_system` helper +
     new `scan_service::run_dir_scan_blocking` + new
     `start_background_directory_scan` Tauri command +
     `GameOverrides.dosbox_entry_point` field + Import Wizard
     dual-mode scan dispatch + theme + per-core docs.
   - See [docs/features/dosbox-and-scummvm/](features/dosbox-and-scummvm/)
     for the cross-stream SESSION_LOG and the locked plan.
   - Both pending operator playtest with real `.dll` cores + game
     data on disk before per-core ROADMAP Phase 1 entries flip ✅.

---

## LOWER — operator-driven or Phase 3+ stretch

1. ~~**Controller-nav v2 polish**~~ — **SHIPPED 2026-05-26** on
   `feat/controller-nav-v2-polish` (three commits, pending operator
   playtest + merge). Closed three of the four bullets the LOWER band
   originally tracked:
   - ✅ QuickSettings sub-views (rewind / TAS / video / memory / disc) —
     each gains a focus group + back handler. Slice 1 (`b87493d`)
     uses a new `useDomQueryFocusGroup` helper in
     `frontend/src/nav/focus.ts` (DOM-query + MutationObserver +
     identity-tracked focused element, generalized from the MenuBar
     pattern); the rewind scrubber uses an `onDirection` override so
     DPad left/right scrubs the timeline when the slider is focused.
   - ✅ Right-sidebar read-only widget rows — Slice 2 (`c883af3`)
     makes the sidebar body one DOM-query group keyed by
     `data-oa-sidebar-row`. R1 from the library grid still lands on
     Play (createEffect snaps `focusedIndex` to `widgetCount()` while
     inactive). Operators DPad up through widget rows; A on a widget
     row is a no-op.
   - ⬜ Right-sidebar header utility chrome (pin toggle +
     sidebar-hide button) — mouse-only by design, not part of the
     play path. Will stay this way unless operator playtest surfaces
     a real need.
   - ✅ MenuBar focus-index-shift edge case — Slice 3 (`567d0de`)
     tracks the focused button by element identity through
     MutationObserver rebinds, so a disabled→enabled flip that
     inserts a row before the focused index no longer drags the ring
     onto a different logical button.

2. **SNES Mouse + Super Multitap** (~200 lines, niche). Mario Paint, Bomberman.
3. **O2 per-game keyboard-layout overlay UI** (~150 lines). Quest for the Rings overlays. Frontend image picker + in-game overlay surface.
4. **Vectrex translucent overlay rendering + aspect override** (~250 lines combined). Plastic color-strip per-game PNG; Vectrex CRT portrait 3:4 default.
5. **NDS microphone input** (~200 lines). Blow/voice puzzles. Deferred until operator playtest forces it.
6. **NDS per-game touch overlay UI** (~250 lines). Visual stylus cursor.
7. **NDS multi-touch** (~80 lines, niche). POINTER index 1+.
8. **Sega CD 3-button vs 6-button pad mode override** (~100 lines + DATA work).
9. **SMS Light Phaser** (~120 lines, shared light-gun infra).
10. **Genesis MD-specific button glyphs polish** (UI). A/B/C diamond + 6-button shoulder visualization.
11. **NGP-mono vs NGPC library-tile differentiation** (~60 lines). Badge or subtitle.
12. **PCFX FMV streaming validation** (operator). PC-FX is FMV-heavy.

---

## DEFERRED — blocked on shared infra not yet triggered

These wait for a single, larger infrastructure pass that benefits many systems at once. Each line item below names what unlocks the deferred work.

- ~~System-agnostic cheat code path~~ — **SHIPPED** across two passes.
  The end-to-end machinery (DB schema + CRUD + frame-loop dispatch +
  libretro `retro_cheat_set` wiring + `CheatsDialog` UI + auto-arm on
  launch) shipped earlier under RetroArch parity slice 5 (see
  `apps/oa-shell/src/library_db.rs::Cheat` + `main.rs::apply_cheats`).
  Per-system named code formats (Game Genie / GameShark / Action
  Replay v3 / CodeBreaker / Pro Action Replay / etc., declared per
  system in `apps/oa-shell/src/cheat_formats.rs` + surfaced via the
  `list_cheat_formats` Tauri command) shipped 2026-05-24 — adds
  per-system Type-picker entries with operator-side regex validation
  for nes / snes / genesis / segacd / sega32x / sms / gamegear / gb /
  gbc / gba / 2600 / n64. Per-core ROADMAP "Game Genie / cheat
  support — operator-driven validation" bullets remain ⬜ pending
  actual operator playtest against running cores.
- **GameCube Wii Remote / Nunchuk / Classic Controller dispatch** (~500 lines, new libretro device type, Phase 2.5).
- **Dreamcast VMU peripheral** (~400 lines, secondary screen + device dispatch).
- **Real OS-level accelerometer access** (~250 lines, Windows Sensor API / Linux iio / macOS Core Motion). Phase G's keyboard-arrows-as-tilt fallback handles GBA Boktai / Kirby Tilt 'n' Tumble / WarioWare Twisted! today; a real accelerometer would let operators with tablet hardware or USB IMU devices play with native motion.
- **Trackball / mouse delta semantics validation** (~80 lines + operator testing). Libretro `RETRO_DEVICE_MOUSE` is spec'd as delta-based; the existing pointer-as-mouse dispatch may need a small adjustment to feed delta-X/Y rather than absolute coords for MAME arcade trackball games (Marble Madness, Centipede). Verify-as-needed when an operator tests an actual trackball cabinet.
- **Custom-built Vectrex vector renderer** (~500 lines, Phase 3+). Replace vecx raster with native wgpu vector-stroke rendering.
- **Modern VR for Virtual Boy via OpenXR** (~800 lines, Phase 2+). Side-by-side dual-perspective to a headset.
- **Right D-pad bindings for Virtual Boy** (~150 lines). Unlocks Mario Clash, VB Wario Land, Teleroboxer, Red Alarm, Vertical Force. (Was gated on "shared analog infra"; that infra is shipped, so this is now ready — moved up to MEDIUM if operator wants to pick it up.)
- ~~**Jaguar CD support**~~ — **SHIPPED 2026-05-27** on
  `feat/new-systems-jagcd-32xcd-stv`. New `jagcd` slug + Rust
  `SystemId::JaguarCd` variant + `check_jagcd_bios` + CD-shape
  dispatch arm + per-core docs. Operator playtest in flight
  (BIOS + ROM in hand). See [docs/cores/jagcd/](cores/jagcd/).
- ~~**32X-CD games**~~ — **SHIPPED (code-only) 2026-05-27** on the
  same branch. New `sega32xcd` slug routing to
  `oa_core::SystemId::SegaCd` (stacked-override pattern), default
  core swapped to PicoDrive, BIOS check reuses
  `check_sega_cd_bios`. Operator playtest deferred until BIOS +
  ROM available. See [docs/cores/sega32xcd/](cores/sega32xcd/).
- ~~**ST-V arcade variant** of Saturn~~ — **SHIPPED (code-only)
  2026-05-27** on the same branch. New `stv` slug aliased to
  `oa_core::SystemId::Mame` (pure alias — no new oa-core variant,
  no separate BIOS check, MAME's stv driver handles everything).
  Operator playtest deferred until BIOS + ROM set available.
  See [docs/cores/stv/](cores/stv/).

---

## DOC / DATA / TRIAGE

Not code — content / curation / validation work.

- **KNOWN_GAME_BUGS triage** for each system once playtime surfaces real issues.
- **Per-game shader curation** — opinionated per-title defaults for known-quirky titles across all systems.
- **Region badges + publisher / developer logos** (already in `docs/PARKING_LOT.md`).
- **2600 homebrew / hack tile distinction** — per-game source-of-origin tag.
- **NEC PC-FX cover-art curation** — Japan-only library; titles ship Japanese by default and need operator-set English aliases for searchability.
- **MAME ROM-set name resolution** — per-game metadata sync against MAME listxml.

---

## Cross-system infrastructure inventory

What's already shipped that future work can lean on. Cite these in PRs that close per-core ROADMAP items.

- **Save states** — `oa_libretro::LibretroCore::save_state` / `load_state` + multi-slot UI + thumbnails (Phase 1.5 + Phase 4).
- **Rewind / TAS / video / memory inspector / milestones** — Phase 4 slices A-F.
- **Shader pipeline** — `crates/oa-render/src/lib.rs::ShaderPreset` (Plain / Scanlines / CrtLite / Phosphor / LcdHandheld) + `shaders/presets/*.preset.toml` + hot-reload + per-game/per-system override.
- **Per-system settings page** — slice 2.8.C. Closes per-system core override, shader, bloom, aspect, overscan, bezel, region/revision priority, rewind config, analog routing, keyboard passthrough.
- **Per-game settings drawer** — slice 2.8.D. All of the above stack on top per-game; plus `core_options` map, `patch_path`, `keypad_layout_note`.
- **Core-option dynamic visibility** — libretro `SET_CORE_OPTIONS_DISPLAY` + `SET_CORE_OPTIONS_UPDATE_DISPLAY_CALLBACK` honored end-to-end. Cores that hide dependent options (Beetle PSX "Lightgun crosshair color" when "Lightgun" off; PCSX2 "GS renderer" sub-options when "Software" selected; etc.) now filter correctly in the per-system + per-game panels. Visibility refreshes after each value change via `Core::refresh_option_visibility`.
- **Library folders: SQLite-only** — SQLite `folders` table is the single source of truth. `list_folders` carries `display_order` (drag-reorder persists via `reorder_folders`), `folder_rules`, scan settings, watch flag. Settings store exposes `libraryFolders() / libraryFolderRows() / addLibraryFolderPath / removeLibraryFolderById / reorderLibraryFolderIds / refreshLibraryFolders`. One-shot `migrate_folders_from_local_storage` runs on settings-store init to absorb any legacy localStorage entries.
- **Shared analog input infrastructure (Phases A–G)** — closes the entire Phase 3 input umbrella. Per-game libretro device-type override across all 5 ports (`GameOverrides.libretro_device` + `libretro_device_port1..4`, `arm_libretro_device` walks every port). Per-button analog pressure (`InputState.analog_buttons[16]`, gilrs L2/R2 trigger axes). Mouse-as-stick analog source (`MouseSource::{X, Y, Xy}`). Per-game device-type UI in `PerGameSettingsDrawer` Input tab with collapsible Additional ports (1–4). Rumble interface (`RETRO_ENVIRONMENT_GET_RUMBLE_INTERFACE` → gilrs force-feedback, lazy-built per (port × effect-kind) effect handles with `set_gain` for magnitude). Sensor interface (`RETRO_ENVIRONMENT_GET_SENSOR_INTERFACE` with keyboard-arrows-as-tilt fallback for GBA / NDS gyroscope titles). Closes ~12 per-core ⬜ bullets that previously cited "shared analog input infra" as their gate.
- **Quick settings overlay** — slice 2.8.B. In-game pause menu.
- **Window + scaling modes** — Phase 2.
- **Aspect override** — `system_settings::default_display_aspect` + `SystemSettings.display_aspect_override` + `GameOverrides.display_aspect_override`. Per-system defaults: GBA → 1.5.
- **Audio device picker** — shipped.
- **Library scan + Import Wizard + watcher** — Phase 2.7.
- **SQLite library** — Phase 2.5.
- **Hash ROM identification** — `rom_hashes::resolve_rom_hashes_for_system`. `HeaderRule` extended with `ByteSwap` for N64 .v64/.n64 normalization.
- **Media sync** — `media::sync_media_for_system` + `repos_for_system_id` (multi-repo: gb DMG+CGB, wonderswan WS+WSC) + `repos_for_entry` (gamecube → GC/Wii classifier via `is_wii_dump`).
- **Core installer + buildbot catalog UI**.
- **BIOS pre-checks** — CD-launch dispatch covers 9 CD systems; cart-shape covers nds/neogeo/coleco/intv/o2/channelf/5200/pokemini/gba/jaguar (10 systems). Neogeo BIOS flavour-tagged stock vs Universe. GBA pre-check is warn-on-missing (mGBA HLE works); jaguar pre-check is block-on-missing (Virtual Jaguar requires jagboot).
- **Keyboard passthrough** + Game-Focus toggle + Ctrl+G. Default-on for `mame`, `msx`, `msx2`, `5200`.
- **Analog axes** — `InputState.axes` + `compute_stick_output` with keyboard fallback + deadzone + sensitivity + per-system default routing (`default_analog_routing("n64") → WASD`).
- **POINTER + LIGHTGUN devices** — `oa_core::InputState.pointer` is now `(x, y, pressed, in_viewport)` + `cb_input_state` dispatch for both `RETRO_DEVICE_POINTER` (touch/stylus shape, NDS et al.) AND `RETRO_DEVICE_LIGHTGUN` (classical gun shape, NES Zapper / Saturn Virtua Gun / PSX GunCon / Dreamcast HotD / SMS Light Phaser / SNES Super Scope / Atari 7800 XEGS Light Gun). Pure helper functions `pointer_field_value` + `lightgun_field_value` in `crates/oa-libretro/src/state.rs` are exhaustively unit-tested. `InputPoller::poll_pointer` + `PointerViewport` (window-relative mapping fed from `Renderer::last_viewport()` per frame); pointer outside the viewport reports `(0, 0, false, false)` so light-gun cores polling `LIGHTGUN_IS_OFFSCREEN` see the reload-by-aim gesture (House of the Dead 2, Time Crisis series, Lethal Enforcers, Confidential Mission). IS_OFFSCREEN plumbed end-to-end 2026-05-27. Catalogue of known light-gun systems + device-type expectations in `apps/oa-shell/src/light_gun_systems.rs`. Remaining Phase 2 gap for full light-gun support: LIGHTGUN AUX/START/SELECT/DPAD/RELOAD bindings UI for gun-side physical buttons.
- **Direct-launch CLI** — `--system` / `--core` / per-game lookup + bootstrap-hint so the emu thread loads the right .dll on first launch.
- **Disc-id extraction** — `cd_id.rs::extractors` covers pce-cd, segacd, saturn, psx/ps2, neocd, pcfx, gamecube, dreamcast; 3DO returns None by design.
- **Per-system theming** — `frontend/src/themes/systems.css` + `registry.ts`.
- **Bindings UI** — `SystemBindingsEditor.tsx` renders button-name chips per system.
- **CJK font fallbacks** — `frontend/src/index.css::--font-display` covers PC-FX + FDS Japanese-only libraries.
- **Multi-core CPU awareness (rayon + tokio blocking pool + zstd + parallel boot)** — Shipped 2026-05-21 on `feat/multicore-cpu-awareness`. Workspace gains `rayon` (1.10); five cold-path bottlenecks now parallelize. Media sync wraps `generate_thumbnail` in `tokio::task::spawn_blocking` so decode/resize/encode runs across cores while `buffer_unordered(8)` keeps the network side busy. ROM hash resolve pre-populates the `hash_cache` via `par_iter` inside `spawn_blocking` — the cartridge read+SHA-1+header-strip work saturates all cores before the for-loop's DB-write phase. Rewind ring (`oa-savestate`) compresses every snapshot at zstd level 1 — 5–10× memory reduction lets the 64 MiB cap hold proportionally more rewind history. Boot-time `archive::sweep_temp` + `read_media_db` + `read_media_prefs` + `library_db::open` fan out to four `std::thread::spawn` workers, joining at point-of-use so the wgpu/WebView init runs concurrently with the disk reads — 100-400ms cold-start savings. Project-wide rationale lives in `docs/DECISIONS.md` 2026-05-21 entry.

When you add new cross-system infrastructure, append it here so the next session knows it can be leaned on.
