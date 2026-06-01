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

**Phase 0 = controller-nav primitives** (~2-3 weeks frontend infrastructure: focus manager, gamepad → UI event layer, focus-ring component pattern, on-screen hint bar). ✅ shipped 2026-05-26 — see [features/controller-nav/](features/controller-nav/).

**Phase 1B — wizard upgrade** (~3-4 weeks) is in flight. Slice 1
(smart-scan emission + Settings → Library entry point) shipped
2026-06-01 (`5ef8062` merge). Slices 2-6 remaining — see
[features/guided-setup/README.md](features/guided-setup/README.md)
for the slice table and the HIGH band entry below for Slice 2's
concrete scope.

**Phases 2B-2F** (~4-5 weeks): curated core selection, folder management, first-system bindings + KNOWN_GAME_BUGS, help suppression, existing-operator re-entry.

**Total estimate:** 8-10 weeks of focused work.

Dwarfs the MEDIUM band below — while in flight, this arc dominates the roadmap for ~2 months. MEDIUM-band shader work + light-gun playtest can pipeline alongside if multiple sessions overlap.

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

When something lands in this bucket, name it concretely (`apps/oa-shell/src/<path>` + scope + estimate) so the next session can pick it up without re-deriving.

### ~~Guided Setup Phase 1B Slice 2 — per-ROM results table~~ ✅ SHIPPED 2026-06-01

Merged to main as `04fa975` (`feat/guided-setup-phase-1b-slice-2`, one
phase commit `f5e2527`). New
`frontend/src/components/import-wizard/ResultsTable.tsx` (~700 lines) —
virtualized table via `@tanstack/solid-virtual`. Confidence badges via
`CONFIDENCE_PILL_STYLES` mirroring `BIOS_PILL_STYLES`. Inline-edit for
system + title (ViewsManagerTab signal-pair pattern). Click-to-sort
headers, filter input, Show-skipped toggle. Bulk-select with Gmail-style
toolbar (Change system ▾ · Skip · Unskip · Clear). Wizard steps
`4 → 3 (Folder / Review / Confirm)`; old per-folder rules editor lives
in an `<details>` "Advanced — extension overrides" expander below the
table. `commitRowsToEntries()` replaces `bucketScanned()` and honors
per-row Change-system / Edit-title / Skip overrides at commit. Frontend
`npm run typecheck` silent.

### ~~Guided Setup Phase 1B Slice 3 — per-system readiness checklist~~ ✅ SHIPPED 2026-06-01

Merged to main as `b57f3e7` (`feat/guided-setup-phase-1b-slice-3`, one
phase commit `2020b4e`). New
`frontend/src/components/import-wizard/SystemReadinessChecklist.tsx`
(~300 lines) rendering 5 pills per system (Core installed via
`list_cores ∩ systemThemes extensions` / BIOS present via
`get_bios_status` / Bindings always ✓ / 2 placeholder pills for
Slice 4). New `apps/oa-shell/src/main.rs::open_bios_folder` Tauri
command for the "Open BIOS folder" action. Two surfaces: new wizard
Step 3 (between Review and Confirm; step counter `3 → 4`) AND a
second "System readiness" SettingsCard in
`SettingsSections.tsx::LibrarySettings` alongside the existing
"Re-scan with smart detection" card. 615 oa-shell tests green; npm
typecheck silent.

### Guided Setup Phase 1B Slice 4 — bulk missing-core download + Core options pill

Two things land together because they share a Rust core-installer
infrastructure and the Slice 3 placeholders are already shaped to
consume the data they'll produce.

**Part A — Bulk core download.** Wire the existing
`apps/oa-shell/src/core_installer.rs` to the "Install core…" stub on
the readiness checklist. Per plan §5 Step 6: "We need 4 cores for
these systems. Download from libretro buildbot? `[Download all
(12 MB)]` `[Skip]` `[Pick individually]`". On-demand only — never
silent. The stub today dispatches a `window.CustomEvent("oa://readiness-stub-toast")`;
Slice 4 listens for that event from a real bulk-prompt modal +
fires `core_installer::download_core` per selection.

**Part B — Core options pill.** Currently renders the muted
`— Coming Slice 4` placeholder. Swap to a real ✓ / ↪ status by
reading the per-system `core_options.rs` catalog. The `read()` /
`write()` helpers are module-private today — Slice 4 needs to add a
small `list_core_options_for_system(system_id) -> Option<usize>`
Tauri command returning the option count (✓ when > 0, ↪ when 0 / no
catalog).

**Part C (optional, can defer)** — KNOWN_GAME_BUGS pill real status.
The structured `games-info.md` schema (System Info Panel work)
covers some systems but the count API doesn't exist yet. If
Slice 4 grows too large, leave the KNOWN_GAME_BUGS pill as a
placeholder + queue Slice 4b separately.

**Where the work lives:**
- `apps/oa-shell/src/core_installer.rs` already exists with the
  download path; needs a thin Tauri command wrapper (`download_core`,
  `list_available_cores_for_system` if not present).
- `frontend/src/components/import-wizard/SystemReadinessChecklist.tsx`
  — listen for the "Install core…" click directly (drop the
  CustomEvent stub) and open a new `MissingCoreBulkPrompt.tsx` modal
  (lives in the same `import-wizard/` subdir). The Core options pill
  reads from a new `useCoreOptionsCount(systemId)` resource.
- New `frontend/src/components/import-wizard/MissingCoreBulkPrompt.tsx`
  — modal listing the missing systems' suggested cores, total
  download size, per-row Skip / Pick toggle, primary "Download all"
  button.

**Scope:** ~1 week for A + B; ~1.5 weeks if C lands too.

**Plan:** [features/guided-setup/README.md](features/guided-setup/README.md) §"Phase 1B — Wizard upgrade" + `docs/PLANS/guided-setup.md` §5 Step 6 + §13 Phase 2.

### ~~Phase D dialog wiring — six orphaned per-game dialogs~~ ✅ SHIPPED 2026-06-01

Six commits on `feat/wire-phase-d-orphans` closing the orphan inventory in operator-utility order: `552236b` Core options → `c5fd427` Display → `7ad1556` Shaders → `5508b30` Cheats → `5351f80` Rewind settings → `aeef83e` Milestones. All seven Phase D split dialogs (Input from `feat/lightgun-bindings-ui` commit `ee8c8e1` + these six) now reachable via TileContextMenu between `Change core…` and `Game properties…`. Each commit follows the mechanical `Input mapping…` pattern (Props field + handler + menu row + App.tsx setGameDialog wire). 60 lines total; `npm run typecheck` silent throughout.

**Follow-up parked:** Shaders + Core options would benefit from a parallel QuickSettings (in-game) entry point with live preview — TileContextMenu is the pre-launch surface; QuickSettings is the in-game tuning surface. Tile menu also has 11 settings items now; if menu length becomes operator-painful, refactor to a `Per-game settings ▸` sub-view mirroring the existing `Add to collection ▸` pattern.

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

~~4. **Jaguar KP8–KP_HASH keyboard-passthrough dispatch**~~ —
   **SHIPPED** alongside the original Jaguar onboarding. Bits 16-20
   are masked out of `libretro_bits` by `jaguar_to_libretro_bits`
   and forwarded to Virtual Jaguar via `retro_keyboard_event_t` in
   the emu-thread frame loop (`apps/oa-shell/src/main.rs:6134-6148`).
   Mapping table at `apps/oa-shell/src/bindings.rs::jaguar_high_bit_to_retro_key`
   with bitmask helper `JAGUAR_HIGH_BIT_MASK`. KP_HASH maps to
   `RETROK_HASH` (35, since libretro defines no `RETROK_KP_HASH`).
   Edge-detected per-bit so a single mask compare skips work when
   no high-bit transitions happened. Tests at
   `bindings.rs:4671-4702`. VJ keycode validation against running
   cores remains operator-driven (Iron Soldier weapon select, AvP
   map screen) — same playtest gap that gates per-core ROADMAP
   Phase 1 entries across systems.

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

~~2. **SNES Super Multitap**~~ — **SHIPPED 2026-05-30** as
   `552fd79` (Phase 2 of `feat/gameplay-fixes-batch`).
   `DEVICE_ID_OPTIONS_SNES = [{ id: 257, generic: "Super Multitap
   (4-port adapter)" }]` in `frontend/src/components/GameDialogs.tsx`
   layered into `deviceOptionsForSystem("snes")`. Hand-encoded
   `((1 << 8) | RETRO_DEVICE_JOYPAD) = 257` matches snes9x's
   CTL_MP5 wire value (same pattern Dolphin uses for Wii subclasses
   — not the canonical `RETRO_DEVICE_SUBCLASS` macro's `+1`).
   `arm_libretro_device` dispatches it as an arbitrary u32. SNES
   Mouse half was already shipped earlier via the generic id=2
   route + per-system label override. `snes/ROADMAP.md` line 31
   flipped ⬜→✅. Operator playtest of 8-player Bomberman titles
   remains a separate operator-playtest gate.
3. **O2 per-game keyboard-layout overlay UI** (~150 lines). Quest for the Rings overlays. Frontend image picker + in-game overlay surface.
4. **Vectrex translucent overlay rendering** (~150 lines). Plastic
   color-strip per-game PNG composited over the framebuffer. Aspect
   override half already shipped 2026-05-24 (Vectrex CRT portrait
   3:4 via `system_settings::default_display_aspect("vectrex") =
   Some(0.75)`; `vectrex/ROADMAP.md` line 34 ✅). Overlay half
   remains ⬜.
5. **NDS microphone input** (~200 lines). Blow/voice puzzles. Deferred until operator playtest forces it.
~~6. **NDS per-game touch hotspot overlay**~~ — **SHIPPED 2026-05-31**
   on `feat/nds-touch-hotspots`. Schema extension: new
   `touch_hotspots: [{ label, x, y, w, h }]` optional field on
   `GameInfo` (`apps/oa-shell/src/game_info.rs`); coords in NDS
   bottom-screen native space (0..256 × 0..192). New
   `frontend/src/components/TouchHotspotOverlay.tsx` renders
   accent-coloured labelled rectangles via contain-fit math.
   Per-session "Show touch hints" toggle in QuickSettings
   ActionsPanel, NDS-gated. Seed entries for Phantom Hourglass +
   Brain Age + Trauma Center in `docs/cores/nds/games-info.md`.
   `nds/ROADMAP.md` line 48 ✅. V1 limitation: assumes default
   melonDS stacked-vertical screen layout; non-default layouts
   misplace hotspots until v2 reads the core option.
~~7. **NDS multi-touch**~~ — **SHIPPED 2026-05-30** on
   `feat/gameplay-fixes-batch`. POINTER `index` parameter now
   dispatches per-finger: `index = 0` → primary, `index = 1` →
   secondary, `index ≥ 2` → zero. `POINTER_COUNT` reports total
   pressed across both slots (0 / 1 / 2). New `pointer_secondary`
   field on `oa_core::InputState` + `input_pointer_secondary[port]`
   mirror in `crates/oa-libretro/src/state.rs` + extended
   `pointer_field_value(primary, secondary, index, id)` signature.
   V1 plumbing — `InputPoller::poll` leaves secondary at
   `(0, 0, false, false)`; a second-finger source (second-mouse /
   real touchscreen / Surface pen) lands as additive operator-
   driven follow-up at the poll site. Tests:
   `pointer_field_value_index_1_returns_secondary_coords`,
   `_index_out_of_range_returns_zero`, `_count_sums_pressed_slots`,
   `_count_unaffected_by_out_of_range_index`. nds/ROADMAP.md
   line 50 flipped ⬜→✅.
8. **Sega CD 3-button vs 6-button pad mode override** (~100 lines + DATA work).
~~9. **SMS Light Phaser**~~ — **SHIPPED 2026-05-25** via the
   `feat/light-gun-harness` branch. Dispatch wired in
   `crates/oa-libretro/src/state.rs::lightgun_field_value` alongside
   nes/saturn/psx/dreamcast/atari7800; catalogued at
   `apps/oa-shell/src/light_gun_systems.rs:102` with `WiringShipped`
   status. Operator playtest of Operation Wolf / Rambo III /
   Shooting Gallery / Marksman Shooting on real Phaser hardware is
   the remaining gap (tracked under MEDIUM #5's playtest matrix). No
   SMS-specific code work remains.
~~10. **Genesis MD-specific button glyphs polish**~~ — **SHIPPED
    2026-06-01.** New `frontend/src/components/GenesisPadReference.tsx`
    renders the physical 6-button Mega Drive pad (X-Y-Z above
    A-B-C + D-pad + Mode + Start) with each face button labeled by
    its current keyboard / gamepad binding. Mounted in both
    `SystemBindingsEditor` (per-system Bindings dialog) and
    `GameDialogs` per-game Input dialog via a shared
    `GENESIS_SYSTEMS` set — all four Genesis-family slugs
    (genesis / segacd / sega32x / sega32xcd) pick it up since
    `apps/oa-shell/src/bindings.rs:1820` routes them all to the
    same `GENESIS_BUTTONS` table. `genesis/ROADMAP.md` line 70 ✅.
~~11. **NGP-mono vs NGPC library-tile differentiation**~~ —
    **SHIPPED 2026-05-24** alongside the media-taxonomy wave. Tile
    `shortName` reads "NGP" for `.ngp` files / "NGPC" for `.ngc`
    files via `subsystemLabel` in
    `frontend/src/components/LibraryTile.tsx`. `ngp/ROADMAP.md`
    line 40 ✅.
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
- ~~**MAME ROM-set name resolution** — per-game metadata sync against MAME listxml.~~ ✅ Shipped 2026-06-01 (bundled `mame-games-slim.json` + L1/L3 SQLite tables + ingest cutover; see `docs/cores/mame/ROADMAP.md` Phase 1.5).

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
- **POINTER + LIGHTGUN devices** — `oa_core::InputState.pointer` is now `(x, y, pressed, in_viewport)` + `pointer_secondary` companion for multi-touch (index 1+) + `cb_input_state` dispatch for both `RETRO_DEVICE_POINTER` (touch/stylus shape, NDS et al.) AND `RETRO_DEVICE_LIGHTGUN` (classical gun shape, NES Zapper / Saturn Virtua Gun / PSX GunCon / Dreamcast HotD / SMS Light Phaser / SNES Super Scope / Atari 7800 XEGS Light Gun). Pure helper functions `pointer_field_value(primary, secondary, index, id)` + `lightgun_field_value(pointer, buttons, id)` in `crates/oa-libretro/src/state.rs` are exhaustively unit-tested (30 tests covering both helpers + viewport coord math edge cases). `InputPoller::poll_pointer` + `PointerViewport` (window-relative mapping fed from `Renderer::last_viewport()` per frame); pointer outside the viewport reports `(0, 0, false, false)` so light-gun cores polling `LIGHTGUN_IS_OFFSCREEN` see the reload-by-aim gesture (House of the Dead 2, Time Crisis series, Lethal Enforcers, Confidential Mission). IS_OFFSCREEN plumbed end-to-end 2026-05-27. POINTER multi-touch (index 0 → primary, 1 → secondary, ≥2 → zero; COUNT reports 0/1/2 total pressed) plumbed 2026-05-30 via Phase 3 of `feat/gameplay-fixes-batch`. LIGHTGUN gun-side buttons (AUX_A/B/C + START + SELECT + DPAD + RELOAD) plumbed 2026-05-30 via Phase 4 of `feat/gameplay-fixes-batch` — `InputState.lightgun_buttons: u32` (bit position == libretro id) + State mirror + `oa_input::lightgun_buttons_from_joypad_bits` derives the bitmask from per-port RetroPad bindings (no new bindings UI). Catalogue of known light-gun systems + device-type expectations in `apps/oa-shell/src/light_gun_systems.rs`.
- **Direct-launch CLI** — `--system` / `--core` / per-game lookup + bootstrap-hint so the emu thread loads the right .dll on first launch.
- **Disc-id extraction** — `cd_id.rs::extractors` covers pce-cd, segacd, saturn, psx/ps2, neocd, pcfx, gamecube, dreamcast; 3DO returns None by design.
- **Per-system theming** — `frontend/src/themes/systems.css` + `registry.ts`.
- **Bindings UI** — `SystemBindingsEditor.tsx` renders button-name chips per system.
- **CJK font fallbacks** — `frontend/src/index.css::--font-display` covers PC-FX + FDS Japanese-only libraries.
- **Multi-core CPU awareness (rayon + tokio blocking pool + zstd + parallel boot)** — Shipped 2026-05-21 on `feat/multicore-cpu-awareness`. Workspace gains `rayon` (1.10); five cold-path bottlenecks now parallelize. Media sync wraps `generate_thumbnail` in `tokio::task::spawn_blocking` so decode/resize/encode runs across cores while `buffer_unordered(8)` keeps the network side busy. ROM hash resolve pre-populates the `hash_cache` via `par_iter` inside `spawn_blocking` — the cartridge read+SHA-1+header-strip work saturates all cores before the for-loop's DB-write phase. Rewind ring (`oa-savestate`) compresses every snapshot at zstd level 1 — 5–10× memory reduction lets the 64 MiB cap hold proportionally more rewind history. Boot-time `archive::sweep_temp` + `read_media_db` + `read_media_prefs` + `library_db::open` fan out to four `std::thread::spawn` workers, joining at point-of-use so the wgpu/WebView init runs concurrently with the disk reads — 100-400ms cold-start savings. Project-wide rationale lives in `docs/DECISIONS.md` 2026-05-21 entry.
- **libretro memory map storage** — Shipped 2026-05-30 on `feat/libretro-env-callbacks-batch`. `RETRO_ENVIRONMENT_SET_MEMORY_MAPS` (env 36) parses the descriptor array into `oa_core::MemoryDescriptor` values (`flags`, `offset`, `start`, `select`, `disconnect`, `len`, `addrspace`) accessible via `Core::memory_map()`; host base pointers stored separately as `usize` in `State.memory_map_ptrs`. Cleared on `load_rom` alongside rotation so back-to-back swaps don't inherit stale descriptors. Unblocks future RetroAchievements rcheevos integration, cheat-search address translation, and AI/scripting layers that read game state by symbolic guest address. 3 unit tests in `state.rs::tests` cover null pointer / zero-count / 2-region NES-shape map.
- **libretro core OSD → toast** — Shipped 2026-05-30 on same branch. `RETRO_ENVIRONMENT_SET_MESSAGE` (env 6) + `SET_MESSAGE_EXT` (env 60) + `GET_MESSAGE_INTERFACE_VERSION` (env 59, returns v1). Cores' OSD messages ("Save state slot N saved", "Disc swapped", BIOS-fallback warnings) queue as `oa_core::CoreMessage { text, level, log_only }` on `State.pending_messages`; the emu thread drains each frame via `Core::drain_messages()` and emits as `oa://toast` events through the existing `emit_toast` helper. Toasts pick up the active system theme via `current_system_id`. `target=LOG` messages log-only (skip toast); duration/priority/type fields ignored in v1 (toast stack has its own schedule). Future polish: route `MESSAGE_TYPE_PROGRESS` to a progress-bar widget.
- **libretro `SET_SUPPORT_NO_GAME` flag + `load_no_rom()`** — Shipped 2026-05-30 on same branch. Env 18 captures the bool into `State.supports_no_game`; `LibretroCore::supports_no_game()` exposes it to the shell. `LibretroCore::load_no_rom()` calls `retro_load_game(NULL)` for cores that advertised support (DOSBox-Pure built-in browser, ScummVM engine launcher, etc.). Post-load common work (controller port wiring, av_info snapshot) extracted into `finish_load()` shared with `load_rom`. Shell-side UI button for bootless launch still ⬜ — operator-driven, low priority.
- **libretro disc-control v2 extras** — Shipped 2026-05-30 on same branch. Four v2-only function pointers previously stored-but-unused now have `LibretroCore` methods: `add_disc_image()`, `replace_disc_image(idx, path)`, `set_initial_disc_image(idx, path)` (multi-disc resume; cores that register interface late can't honor — returns false), `disc_image_path(idx)`. `DiscInfo` gains `paths: Vec<String>` populated from `get_image_path` for v2 cores; v1 fallback returns empty. `read_disc_string_field` helper collapses get_image_label / get_image_path buffer-fill duplication. Frontend `QuickSettings.tsx` `DiscInfo` type extended with optional `paths` field for future tooltip polish.
- **Game Info Panel v1** — Shipped 2026-05-30 on `feat/game-info-panel-v1`. Three-layer data model (file layer at `docs/cores/<id>/games-info.md` + SQLite `game_info_overrides` table + field-typed precedence merge) feeding three UI surfaces. Rust types live in `apps/oa-shell/src/game_info.rs` (GameInfo / GameInfoOverride / MergedGameInfo / GameInfoBadge); SQLite migration v15 adds the overrides table; six Tauri commands cover read (`get_game_info`, `get_game_info_override`), write (`set_game_info_override`, `delete_game_info_override`), and bulk queries (`list_game_info_overridden`, `list_game_info_badges`). Frontend surfaces: Retroverse `GameDetailPanel` gains Operator note / Controls / Recommended core (+Apply best emulator action wired through `update_game_core_override`) / Known issues sections; `LibraryTile` gains bottom-right `⚠ N` + `✎` badges; `GameInfoModal` gains a 4th "Game info" tab with an inline editor (short summary + controls supported + recommended core + bugs add/remove + Submit correction stub). Files: `apps/oa-shell/src/game_info.rs`, schema in `docs/cores/SCHEMA.md`, plan in `docs/PLANS/game-info-panel.md`. v1 ship includes a seed `psx/games-info.md` (Tomb Raider + FF7); cross-system migration of `KNOWN_GAME_BUGS.md` content is operator-driven follow-up. v2 evolution (separate data repo + scraper + GitHub-Issue submission flow) designed but deferred per plan §11. **Schema extended 2026-05-31** with the optional `touch_hotspots: [{ label, x, y, w, h }]` field — NDS-specific in practice today; coordinates in NDS bottom-screen native space (0..256 × 0..192). The new `TouchHotspotOverlay` component (`frontend/src/components/TouchHotspotOverlay.tsx`) renders labelled accent outlines over the bottom-screen area while a stylus-using game runs; toggle lives in QuickSettings → "Show touch hints" (per-session, NDS-gated). Seed entries in `docs/cores/nds/games-info.md` (Phantom Hourglass / Brain Age / Trauma Center). v1 limitation: assumes default melonDS stacked-vertical screen layout; non-default layouts (side-by-side, top-only) misplace hotspots until v2 reads the core option.

- **System Info Panel v1** — Shipped 2026-06-01 on `feat/system-info-panel-v1`. Three-layer per-system metadata replacing the hand-typed-5-of-45 `frontend/src/routes/retroverse/systemMetadataStubs.ts`: L1 (MAME baseline, baked at launch from `assets/mame-source/listxml-slim.json` + `history-slim.xml` shipped by `tools/mame-extractor/`), L2 (curated YAML at `docs/cores/<id>/system-info.yaml`, baked into SQLite `system_info_curated`), L3 (per-install operator overrides in SQLite `system_info_overrides`). Rust types + parsers + field-typed merge live in `apps/oa-shell/src/system_info.rs`; SQLite migration v16 adds the four tables (`system_info_mame` / `_curated` / `_overrides` / `_meta`); six Tauri commands cover read (`get_system_info` merged, `get_system_info_override` raw L3, `get_system_info_curated` raw L2), write (`set_system_info_override`, `delete_system_info_override`, `reset_system_info_to_default`), and operator-driven L1 re-import (`refresh_mame_system_info`). The operator-driven refresh in `apps/oa-shell/src/mame_import.rs` mirrors the maintainer-time `tools/mame-extractor/` parser in-process — detects MAME at `<exe_dir>/Emulators/MAME/` first, shells out to `mame -listxml` + reads local `history.xml`, overwrites L1 without touching L2 or L3. Frontend surfaces: Retroverse `SystemInfoPanel` + `HomePage` hero consume the merged record via `getSystemInfo`; new `PerSystemInfoSection` (per-system Settings drill-in) is the L3 edit UI with form-row-per-field input + provenance badges (no badge = L1 default; slate "curated" = L2; accent "edited" = L3) + peripherals editor + Reset all overrides button; `StorageSettings` gains a "Refresh MAME system info" card with folder-picker fallback. Schema reference in `docs/cores/SCHEMA.md` (system-info.yaml section); plan in `docs/PLANS/system-info-panel-v1.md`. v1 seed L2 YAMLs for snes / nes / genesis / psx / gb (5 of 45 systems migrated from the old stub data); remaining 40 fall through to L1 only. Three OA slugs lack MAME data entirely in MAME 0.288 (`3do` model-specific only, `msx` + `msx2` software-list-only) and stay L2-only per plan §5's same recipe DOSBox/ScummVM use. v2 candidates (session-scoped re-imports → sticky; bundled-only L1 → scheduled refresh from `overlooked-arcade-system-info` repo) stay parked. **SCHEMA_VERSION constant trap** surfaced + fixed during the rollout: the early-return `if current == SCHEMA_VERSION` in `bootstrap_schema` short-circuits all migrations when the constant isn't bumped with each new migration. Game Info Panel v1's v14→v15 had also shipped without the bump (silently absent `game_info_overrides` on any v14 install); constant now sits at 16 with a long inline comment calling out the trap.
- **Project-wide `Emulators/` convention** — Shipped 2026-06-01 with System Info Panel v1 Phase 1a. Top-level `Emulators/` directory at the repo / install root is the canonical home for every third-party emulator binary OA shells out to (MAME today; DOSBox-X / ScummVM standalone / etc. eventually). `tools/bump-mame.sh` + `apps/oa-shell/src/mame_import.rs::detect_mame_binary` both probe `<root>/Emulators/MAME/mame.exe` first; the shipped install applies the same shape at `<exe_dir>/Emulators/MAME/`. `/Emulators/` added to `.gitignore`. Future external emulators follow the recipe `<root>/Emulators/<name>/`.

When you add new cross-system infrastructure, append it here so the next session knows it can be leaned on.
