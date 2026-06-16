# Next — cross-system priority queue

What to ship next across the project, ordered by leverage. **Per-system status lives in `docs/cores/<id>/ROADMAP.md`** — this file is just the cross-system view of what to pick up next when you have a fresh session.

Each item: short scope, rough line estimate, gating (operator-driven / blocked on infra / ready to ship), where the work lives.

When you close an item, the matching PR also flips the corresponding `⬜` to `✅` in the relevant per-core ROADMAP — see CLAUDE.md "ROADMAP hygiene" for the policy.

---

## Shipped & superseded arcs (historical pointer)

The 2026-05-26 "pipelined sequence" (Controller-nav → Per-System UI → Game Info
Panel, interleaved with Guided Setup) has shipped or been superseded. Per-system
status lives in `docs/cores/<id>/ROADMAP.md`; what to pick up next is the **HIGH
band** below. Current per-arc status:

- **Controller-nav primitives** — ✅ shipped 2026-05-26; archived at
  [_archive/features/controller-nav/](_archive/features/controller-nav/). Going
  forward superseded by the Unified-Nav spatial engine
  ([features/unified-nav/](features/unified-nav/)).
- **Guided Setup** — Phase 0 + Phase 1B (wizard upgrade) + Phase 2 (CPU-tier
  curated cores) shipped; later phases (folder mgmt, help-suppression,
  existing-operator re-entry) partial/queued. Plan:
  [PLANS/guided-setup.md](PLANS/guided-setup.md).
- **Per-System Custom UI** — Stage 1 machinery shipped (now in `platform/`);
  **superseded/merged into Theming ARC 2** (D32/D33/D34) — pilots + Stage 2/3
  re-home there as Retroverse content. Current model:
  [PLANS/theming-arc-2-per-system-layout.md](PLANS/theming-arc-2-per-system-layout.md).
- **Game Info Panel** — v1 shipped 2026-05-30 (see the cross-system infra
  inventory at the foot of this file); v2 (scraper + data repo + community
  pipeline) designed + deferred
  ([_archive/PLANS/game-info-panel.md](_archive/PLANS/game-info-panel.md) §11).
- **Background jobs + persistent progress bar** — ✅ closed; archived
  ([_archive/features/background-jobs/](_archive/features/background-jobs/)).
  Load-bearing infra every long-running op now consumes.
- **Per-track SHA-1 (disc-shape systems)** — folded into the Virtual Library arc
  as Slice A1; shipped + pivoted to a fuzzy-filename primary; plan archived
  ([_archive/PLANS/disc-track-sha1-matching.md](_archive/PLANS/disc-track-sha1-matching.md)).

---

## NEXT MAJOR ARC — Virtual library + preservation architecture + launcher-agnostic frontend

**Planning locked 2026-06-03.** Full plan at
[docs/PLANS/virtual-library-and-launcher-arc.md](PLANS/virtual-library-and-launcher-arc.md).
**Operator priority: high — drives the next ~14–22 weeks.**

Two strategic shifts pulling the next arc:
1. The virtual library + preservation depth (Pokémon Red as one
   parent identity, variants as children) gets promoted from "runtime
   view" (already shipped in `library_groups.rs`) to "schema model"
   (new `game_identities` SQLite table).
2. OA's role shifts from "premium libretro frontend" to "premium
   frontend for retro emulation, period" — external standalone
   emulators (Cemu / RPCS3 / Lime3DS) join libretro cores via a new
   `Launcher` trait. Reverses the 2026-05-16 libretro-only DECISIONS
   entry; partially un-parks the 2026-06-02 plugin-API PARKING_LOT
   entry.

**Phase order:** A (identification depth) → E (schema promotion) → B
(two-mode UX + Collection Health) → C (launcher abstraction) → D
(external install pipeline) → F (Preservation Vault) → G (crates
split). Phase H (CLI) deferred.

**Phase A absorbs the disc-track SHA-1 plan above as Slice A1.**

**Position:** Foundation (Phase 0 — paperwork) in flight on
`feat/virtual-library-arc-foundation`. Phase A1 (disc-track SHA-1)
queued behind operator review of the Phase 0 commit.

---

## HIGH — ready to ship next

These are operator-independent and the infrastructure they sit on already exists.

When something lands in this bucket, name it concretely (`apps/oa-shell/src/<path>` + scope + estimate) so the next session can pick it up without re-deriving.

### oa-packs infrastructure — Slice 3 (Settings → Packs panel + lifecycle) `[ARC opened 2026-06-15]`

**Plan:** [PLANS/oa-packs-infrastructure.md](PLANS/oa-packs-infrastructure.md) ·
**Design:** [PLANS/content-packs.md](PLANS/content-packs.md) ·
**Decisions:** [features/content-packs/DECISIONS.md](features/content-packs/DECISIONS.md) (CP1–CP6).

The single operator-initiated content-pack distribution channel — built
**schema + verify + install first, hosting later** (CP1). Every future
pack-shaped stream (emulator recipes, editorial DISCOVER, themes, asset
bundles, cheats, metadata) rides this instead of N bespoke updaters. Pack
`type` is additive data + a dispatch arm (CP3); bundled-baseline is
per-type (CP4); emulator recipes are the first non-editorial consumer and
fold the External Emulator Depth recipe-update slice into this infra (CP5).

- **Slice 1 ✅ SHIPPED + MERGED to main (2026-06-15):** `crates/oa-packs/` —
  pure `Registry`/`PackEntry`/`Manifest` contract types, `verify`,
  `validate_manifest_against_registry`, `install_from_local_zip` (atomic,
  no-partial-on-failure), per-type `PackTypeSpec` baseline (CP4). 14 tests
  green, no network.
- **Slice 2 ✅ SHIPPED on `oa-packs-slice-2` (2026-06-16; awaiting
  merge):** `apps/oa-shell/src/packs.rs` + `packs_prefs.rs` — `PacksPrefs`
  at `appDataDir/packs/prefs.json` (config registry URL per CP1 + master
  network toggle), `PacksRoot` (`<exe_dir>`), 8 Tauri commands
  (`oa_packs_get_prefs`/`set_registry_url`/`set_allow_network`/`list`/
  `uninstall`/`fetch_registry`/`install`/`update`), synchronous
  `NETWORK_DISABLED` gate, reuses `http_retry` (no rebuilt downloader).
  Decisions = CP6. 8 unit tests green.

**Slice 3 scope (next — Settings UI + lifecycle, `frontend/` + small Rust):**
- Settings → Content → Packs panel: **Installed / Available / Updates**
  sections (content-packs.md §9) + `Check for updates` + `Last checked` +
  `Registry URL` display.
- `frontend/src/.../packs.ts` (or `platform/api/packsApi.ts`) wrapping the 8
  Slice-2 commands.
- 14-day rollback retention (§8): on update/uninstall move the prior version
  to `<data_dir>/packs-rollback/<pack_id>-<version>/`; GC older than 14 days;
  Rollback action in the Installed list.
- Conflict warning surface when two packs declare the same content id (§7).
- Network commands disabled in-panel with a tooltip when the master toggle
  is OFF (depends on Slice 4's Privacy panel for the toggle UI; until then
  the gate still enforces server-side).

**Then:** Slice 4 (Privacy panel + allow-network toggle UI + network-log
ring buffer) → Slice 5 (first consumers: `emulator-recipes` override tier on
the bundled `config/emulators/` baseline, closing External Emulator Depth
Slice 2; then `editorial` → DISCOVER).

**Reuse note (still applies):** `apps/oa-shell/src/core_installer.rs` is the
richer download cousin (job rows, resume, progress events) — Slice 3 can
graft its progress-event pattern onto pack download when the UI needs a
progress bar. Do NOT rebuild a downloader.

### External Emulator Depth — Slice 1 ✅ SHIPPED (schema accretion + ares/BizHawk profiles) `[ARC opened 2026-06-15]`

**Plan:** [PLANS/external-emulator-depth.md](PLANS/external-emulator-depth.md) ·
**Decisions:** [features/external-emulators/DECISIONS.md](features/external-emulators/DECISIONS.md) (ED1–ED6) ·
**Log:** [features/external-emulators/SESSION_LOG.md](features/external-emulators/SESSION_LOG.md).

New arc deepening the shipped launcher abstraction (VL Phase C): recipe
upgrade + independent updates · install pipeline (legal-gated) · OA-authored
control toward window-wrapping. Load-bearing principle: per-emulator
knowledge is **updatable data**, refreshed without an OA rebuild (ED2).

**Slice 1 ✅ SHIPPED + PLAYTESTED + MERGED to main (2026-06-15):**
- ✅ Per-OS `binary_name` map — untagged `BinaryName` enum (`Single` | `PerOs
  { windows, macos, linux }`) + `resolve()` in `emulator_profiles.rs`; single
  string stays valid (all 9 existing profiles unchanged). Consumers in `main.rs`
  resolve current-OS name.
- ✅ `config/emulators/ares.yaml` + `bizhawk.yaml` — single positional `{content}`
  (auto-detect). ares full per-OS map (15 systems); bizhawk per-OS map with macOS
  omitted (18 systems).
- ✅ MAME — **deferred** (the `content_mode` enum is not a clean 1-field add; needs
  real content resolution; in-process core already covers arcade). Reason in
  SESSION_LOG + RESEARCH doc.
- ✅ Reserved `--system`/`{system}` seam documented (not built) on
  `launch_args_template`.
- ✅ `all_shipped_profiles_parse_and_hold_invariants` extended + per-OS-map tests;
  `cargo test -p oa-shell` green (849 passed).
- ✅ **Playtest follow-ups (merged on the branch):** `accepts_archives` recipe
  field — BizHawk/ares load `.zip` natively, so the external path hands them the
  decoded outer archive path instead of erroring on archived content (was a
  pre-existing VL Phase C2 gap). Verified: EmuHawk opens + tries to boot an
  archived `coleco` game (firmware is a BizHawk-side concern OA never provides).

**Acceptance met + merged to main 2026-06-15.**

**Next: Slice 2** — recipe update delivery (rides content-pack infra).

**Note on sequencing:** Slice 2 (recipe-update delivery) depends on the
content-pack distribution infra (design-only today); Phase 2 (install) builds on
VL Phase D's `InstallableProfile` sketch; Phase 2 new-system wiring rides the
per-system-descriptor loader.

### Theming ARC 2 — L1…L5 ✅ MERGED to main; L6 PARKED, P (loader) remaining

- **L1 (per-system UI consumption opt-in, D33)** — ✅ MERGED 2026-06-15.
- **L2a (view/layout manifest contract)** — ✅ MERGED 2026-06-15. `ViewType`/`LayoutPrimitive` enums + `views` field + validator. D37.
- **L2b (D34 systemUIConfigs migration)** — ✅ MERGED 2026-06-15. Experiential config → Retroverse via `ThemePackage.perSystemUiConfigs`; `touchInputSupported` → platform-factual. D38.
- **L3a (layout resolver + persisted override store)** — ✅ MERGED 2026-06-15. Pure `resolveLayout` cascade + `useResolvedLayout` hook + `(theme,system,view)→layout` localStorage override store. No consumer. D39.
- **L3b (per-system layout wired into game-browse)** — ✅ MERGED 2026-06-15, operator playtested (NES→list, slider hides on list). Coexist model (D40): `layout` optional; `useDeclaredLayout` keyed on `selectedSystemId()`; `LibraryView` renders grid/list (carousel/wheel/custom→grid fallback) else the global viewMode. D40.
- **L4a (render carousel in game-browse)** — ✅ MERGED 2026-06-15, operator playtested (SNES coverflow). `LibraryView` renders a per-system `carousel` via `CarouselNav`; Retroverse demo SNES→carousel. `wheel`/`custom` still grid-fallback. D41.
- **L4b (radial WheelNav primitive + render `wheel`)** — ✅ MERGED to main, operator playtested (TG-16 wheel). Built the reserved `WheelNav` as a general angle→x/y engine (pure `wheelGeometry.ts`); **shape A (right-side vertical wheel) as the defaults** so B/C land later as presets (operator's stated intent); `LibraryView` renders `wheel` (4th Switch arm), ring radius from a measured pane height; Retroverse demo `tg16→wheel`. Two playtest fixes rode along: gentler feel (arc 80°/sideScale 0.85) + fast-scroll deform fix (snap transitions while scrolling). typecheck/lint/vitest(149)/build green. D42.
- **L5 (end-user per-system layout override UI)** — ✅ MERGED to main, operator playtested. New "Layout" domain card in the engine Per-System Settings Hub (`systemsHub/domains/LayoutEditor.tsx`) writing the L3 override store; per-view `SettingRow` picker (Theme-default sentinel + list/grid/carousel/wheel, no `custom`), inheritance chip + per-row Reset (D30), overrides keyed by active theme. Forks signed off: Hub card · ALL FOUR ViewTypes exposed (reserved views labeled "no renderer yet") · curated primitives. typecheck/lint/vitest(149)/build green. D43. **Wanted follow-on parked:** cross-theme "Copy from theme…" + "Set for all themes" convenience buttons (PARKING_LOT 2026-06-15).
- **L6 — Per-system character in Retroverse: make a pilot console feel distinctive (sound + background + boot + feel), end-to-end** — ⏸ **PARKED 2026-06-15** (operator; bigger priorities first). Plain English: make each console feel like its own world in Retroverse (Game Boy sounds like a Game Boy, Vectrex glows like phosphor), not just a colour swap. Machinery already shipped (per-system-ui Stage 1) + ARC 2 L2–L5 did the layout switching; what's left is mostly pilot **content** built as Retroverse theme content (D33/D34). Resume when per-system polish is the priority.
- **P — `.oatheme` runtime loader** — the last open ARC-2 thread (also the last open ARC-1 thread): on-disk discovery + extract + dynamic `import()` of a theme from a loose folder / `.oatheme` zip, plus the CSP allowlist ARC 1 deferred. Tees up ARC 3 (the CSP work becomes load-bearing for Rhai sandboxing). Sequence after L6 OR pull forward independently — it doesn't depend on L6.

The keystone slice of **Theming ARC 2 — Per-System Layout Substrate** (planned
2026-06-15; [PLANS/theming-arc-2-per-system-layout.md](PLANS/theming-arc-2-per-system-layout.md)).
Convert the **global `perSystemUiEnabled`-gated** tile-flourish + per-system-SFX
path in the shared platform grid into a **per-theme opt-in capability** (matching
how `<ThemeBackground>` already works — mounted only by CoverFlow). A theme
declares how much per-system UI it consumes (Retroverse: full; CoverFlow:
backgrounds only; `bare`: none); keep a **user master-off** as the
accessibility / reduced-motion / low-end escape (the one survivor of the old
global toggle). Files: `frontend/src/platform/components/LibraryTile.tsx`
(tileShape/interactionStyle), the grid-nav SFX dispatch, `platform/theme/manifest.ts`
(+ a `per_system_ui` capability field) + `validate.ts`; App bridges the active
theme's capability into the gated paths (same pattern as S5.3 glyph-set / S5.1
ambient themeId). Frontend-only + a touch of Rust if the SFX gate moves. ~small.
**Gate:** same library reads per-system under Retroverse, uniform under
CoverFlow/`bare`; master-off forces uniform under Retroverse. Pulled forward
ahead of the view/layout contract because it's the concrete user-visible defect
and establishes the platform-capability / theme-consumption split (D33/D34) the
rest of the arc builds on. Then **L2** (view/layout manifest contract +
`systemUIConfigs` experiential→Retroverse split) → L3 resolver + persisted user
override → L4 WheelNav → L5 override UI → L6 Per-System UI Stage 2/3 re-home → P
`.oatheme` loader.

### Settings IA Redesign — Slices 1–4 ✅ SHIPPED (arc core complete)

**The Settings-IA re-cut is DONE (all merged to main, operator-playtested):**
S1 IA re-skeleton + Library/Organize split (`e71eef0`), S2 Library re-point
(`59b0d52`), S3 declarative per-theme Appearance schema (`5386305`), S4 External
Emulators consolidation (`f3082ed`). Settings now reads **Import & Setup ·
Library · Organize My Collection · Systems · External Emulators ·
Themes/Appearance**. Plan + slice log:
**[PLANS/settings-ia-redesign.md](PLANS/settings-ia-redesign.md)** /
[features/settings-ia/](features/settings-ia/) (decisions D1–D7). **Slice 5
(Import & Setup depth) deferred — folds into guided-setup Phase 2.**

### External-emulator research pass — verify CLI + author profiles  *(next chosen work)*

The chosen follow-on after the Settings-IA arc. Run the research scoped in
**[RESEARCH/external-emulators.md](RESEARCH/external-emulators.md)**: verify the
command-line launch invocation (+ fullscreen flag, content format,
BIOS/firmware/keys, per-OS binary) for a batch of standalone emulators — both
systems OA can't run via cores (Cemu / RPCS3 / Ryujinx / Lime3DS / Vita3K /
Xenia / xemu / …) and ones it can but where users may prefer a standalone
(PCSX2 / DuckStation / PPSSPP / Dolphin / …) — then author real
`config/emulators/<id>.yaml` profiles (schema in
`apps/oa-shell/src/emulator_profiles.rs`; `dolphin.yaml` is the template). Fill
the doc's roster tables with verified data; flag which section-B systems need an
OA system id wired first. **Legal posture: zero ROMs / BIOS / keys, ever.**
Surfaces in Settings → External Emulators (shipped S4).

### Unified Navigation & Panel System — the substrate that makes panels "just work"

**Pivoted 2026-06-14** from the per-panel Controller-Nav Coverage sweep. The
opt-in-twice model (mount a group + tag every control) doesn't scale — most
engine panels stayed inert. Replaced by a **spatial-navigation engine**
(universal focusable auto-discovery + geometry movement + layer scoping, reusing
the Slice-1/2/3 activate layer) **plus** a unified, input-agnostic panel
structure/look. Full design + phases:
**[PLANS/unified-navigation-and-panels.md](PLANS/unified-navigation-and-panels.md)**;
feature folder [features/unified-nav/](features/unified-nav/).

**Phase 1 (Pillar A — spatial engine) ✅ SHIPPED + MERGED 2026-06-14**
(`feat/unified-nav-phase-1`). The spatial engine (`platform/nav/spatial.tsx`)
+ universal native-focusable discovery now drives the **whole Settings surface**
(sidebar, all category bodies, embedded sub-pages, dialogs, custom modals) with
**zero per-control wiring**. Movement model resolved to a **region-bias hybrid**
(UP/DOWN within a region, LEFT/RIGHT between — DECISIONS D1, matches the locked
nav spec). See [features/unified-nav/SESSION_LOG.md](features/unified-nav/SESSION_LOG.md)
+ DECISIONS D1–D4.

**Pillar B ✅ delivered by the Per-System Settings Hub arc** (its
`HubCard` / `HubGrid` / `PanelScaffold` primitives ARE the unified panel
structure; the HintBar fix shipped with it). **Remaining unified-nav phases
(next pickups):** Phase 2 — formalize the spatial scope into `Dialog` +
`EngineManagerSurface` (every dialog inherits nav for free; retire
`Dialog.navigate` markers) → Phase 3 — Retroverse tabs + thin adapters for the
virtualized library grid + carousels → Phase 4 — kiosk/arcade limited-button
input pass.

### Per-System Settings Hub — ✅ COMPLETE + MERGED 2026-06-14

Consolidated all per-system settings into the card-based **Systems** hub (systems
grid → per-system domain cards → editor), replacing the scattered Per-system /
Media / Metadata categories + the Library→Game-media grid + BIOS. S1–S5 + the
Platform/Game metadata split + a dev-only **DevTools panel** (Settings → About:
logging toggles + backend log streams + Open inspector) all shipped. Plan +
slices: **[PLANS/per-system-settings-hub.md](PLANS/per-system-settings-hub.md)**;
feature [features/per-system-hub/](features/per-system-hub/). **Parked
follow-ups (PARKING_LOT 2026-06-14):** system-vs-platform terminology audit;
re-gate the DevTools panel + move the `devtools` Cargo feature out of default
before any public release; per-game metadata → game-detail surface.

**Predecessor — Controller-Nav Coverage Slices 1–3 (2026-06-13/14):** Slices
1–2 (Settings row-nav + engine dialogs) **shipped + merged**; Slice 3
(ImportWizard) on `nav-coverage-slice3`, **folded** into the engine (reusable
infra kept; per-panel markers superseded). The activate layer (dispatch +
select-overlay + slider-adjust + Y-reset + deferred OSK) is reused wholesale.
History: [features/nav-coverage/](features/nav-coverage/).

### Controller Identity & Auto-Config — ✅ SHIPPED + MERGED 2026-06-13 (Phase 0→2.5 + label families + full SDL DB import)

**Planned 2026-06-12.** Full plan:
[PLANS/controller-identity-substrate.md](PLANS/controller-identity-substrate.md);
feature folder [features/controller-identity/](features/controller-identity/).
A foundational input-infrastructure arc: give every controller a **stable
identity (VID/PID)** and **auto-config** both shell-nav + per-system gameplay
bindings from three shared data files (`controllers.json` layout DB,
`systems-input.json` schema, `default-maps.json` canonical→system defaults) +
a press-the-buttons wizard. Fixes the non-standard-pad break (wired Switch
Pro) and the replug-shuffles-ports problem. Decisions D1–D8 locked; two
pollers stay separate, the *config* unifies. **Operator's current priority**
(surfaced from the Metadata controller-nav work).

**Phase 0 scope (the spike — small, gates everything, R1 = highest risk):**
prove a stable cross-layer `device-key`. (1) Frontend: parse `gamepad.id` →
`{vid,pid,name}` → device-key (the string embeds `Vendor: xxxx Product:
yyyy`). (2) Rust: resolve the VID/PID unknown — `gilrs` 0.11 doesn't expose a
UID publicly; options are **(a)** upgrade gilrs for the SDL `Gamepad::uuid()`
GUID, **(b)** a Windows raw-input API, or **(c)** frontend-as-identity-
authority (resolve the profile in JS, hand Rust the normalized mapping
per-port at launch). **Lean (a)+(c).** Output: a documented device-key format
+ the (a)/(b)/(c) decision, validated against the `[oa-gamepad] connected`
log for the operator's Switch Pro + one XInput pad. Anchors:
`frontend/src/platform/nav/gamepad.ts` (~`:201` id-logging), `crates/oa-input/
src/lib.rs` (~`:265` `port_pads`), `crates/oa-input/Cargo.toml` (gilrs dep).
Then **Phase 1** (identity in both layers + replug-stable ports) → **Phase 2**
(`controllers.json` normalization = the Switch Pro fix) → wizard → gameplay
auto-config → replug → compose with the core-side plans. Gating: ready.
Estimate: Phase 0 ≈ 1–2 days (mostly the gilrs spike); full arc ≈ several
weeks across phases.

### ✅ CLOSED — Metadata Curation (absorbed into the Per-System Settings Hub, 2026-06-15)

The override backend shipped (`game_metadata_overrides`, schema v24) and the
editor UI was **absorbed into the Per-System Settings Hub** — the standalone
`metadata` Settings category + `MetadataSettingsBody.tsx` were **removed** when
the Hub's **Game/Platform Metadata domain cards** (`engine/systemsHub/domains/
{Game,Platform}MetadataEditor.tsx`) shipped + merged 2026-06-14. No standalone
arc remains. Plan + feature folder archived (`_archive/PLANS/metadata-editing.md`,
`_archive/features/metadata-editing/`). Wave-2 ideas (undo stack, bulk
find-replace) — if ever wanted — would extend the Hub editors, not a separate
surface.

### ✅ Portability + state-storage audit — DONE 2026-06-11

Read-only sweep shipped. Findings:
[features/portable-install/STATE_STORAGE_AUDIT.md](features/portable-install/STATE_STORAGE_AUDIT.md)
(new `portable-install/` feature folder + README created).

**Verdict:** OA is **one architectural change from drive-move portability —
the blocker is absolute paths, NOT config scatter.** Every load-bearing path
(`games.file_path`, `game_identities.canonical_cover_path`, `folders.path`,
per-game `patch_path`/`bezel_image_path`) is stored **absolute, never
root-indirected**. The `folders` table already models a multi-root registry
(`add_folder`/`list_folders`) but `games` has **no `folder_id` FK** — it's
used as scan-targets, not a resolution indirection (the S9 gap is two-thirds
pre-built in the schema). BIOS (convention `<exe_dir>/system/<name>`) + the
`portable.txt` data-dir tree are already portable — the model to copy.

**Part B answer: mostly DON'T consolidate into SQLite.** ~13 backend JSON
files + SQLite already travel as one `<data_dir>` tree — scatter is cosmetic,
not a portability problem. The only per-user state that does NOT travel is **6
frontend `localStorage` keys** (`oa.settings.v1`, `oa.themeSettings`,
`oa.core.*` are real; 3 are ephemeral). Keep JSON as files (hand-editable —
low-floor/high-ceiling pillar); leave shipped descriptors with the install;
mark `cpu-tier.json`/`emulators.json` per-install (must not travel).

**NAS safety:** safe *today only by absence* — no rescan-purge sweep exists.
Any future "scan for removed ROMs" MUST be built on the roots model (mark
**Unavailable** on root-unreachable; only per-file-delete when the root
resolves) or it purges a sleeping NAS. **Don't ship the sweep before the
roots model.**

**Remediation queue this surfaced (sized separately, in dependency order) —
the "Portability remediation" arc, not yet planned:**
1. **Roots model** — `games.root_id` FK + relative paths, resolve against the
   existing `folders` row (schema v23→v24 + backfill). Load-bearing; do first.
2. **Evict the 3 real localStorage keys** to the backend (pairs w/ theming S5
   per-theme-settings namespace).
3. Volume-GUID/label tracking + cross-OS root syntax in the root row.
4. Removed-ROM sweep (**after #1 only**).
5. Persistent cart hash cache `(path,size,mtime)` (disc already has it).
6. Media-convention casing standardization (S9a dialect).

Items 1–2 deliver "copy the portable folder / re-point one drive → just
works." Feeds virtual-library **S9**.

### Libretro plumbing audit — gaps (from 2026-06-08 audit)

Surfaced by the read-only libretro plumbing sweep —
[docs/cores/AUDIT_2026-06-08.md](cores/AUDIT_2026-06-08.md). Verified against
core source + our `file:line`. **Most of the audit shipped on
`feat/libretro-plumbing-fixes` (2026-06-08)** — see below.

- ✅ **Polled keyboard + mouse input state** (H1+H2). Added a 512-wide held-key
  bitset + per-port mouse delta/button state to `State`, `RETRO_DEVICE_KEYBOARD`
  and `RETRO_DEVICE_MOUSE` arms in `cb_input_state` (in `state.rs`, with a pure
  `mouse_field_value` helper + tests), `set_keyboard_state`/`set_mouse_state` on
  `LibretroCore`, `poll_mouse_raw` in oa-input, and the per-frame push in the
  shell input loop (focus-gated). Unblocks the computer-core tier (MSX, DOSBox,
  5200, O2) + arcade trackball/spinner/paddle games.
- ✅ **Quick wins** (same branch): env `log::info!`→`debug!` spam downgrade (M2);
  core-option parser sentinel caps (M1); F8 paused-restore framebuffer nudge (M4);
  controller ports 0-4 wired post-load (M3, in `core.rs::finish_load`).
- ✅ **GET_SAVESTATE_CONTEXT** (M5, env 72) wired to run-ahead serialization;
  **GET_THROTTLE_STATE** (L2, env 71), **SET_VARIABLE** (L1, env 70),
  **GET_JIT_CAPABLE** (74), frame-truncation `log::warn` (M7), and ffi constants
  for envs 71-75 — all in the same branch.
- ⬜ **Deferred (feature-sized, not in the branch):** SET_SUBSYSTEM_INFO launch
  path (M6 — SGB / Sufami Turbo / BS-X; needs parse + store + subsystem-aware load
  + picker UI) and GET_MICROPHONE_INTERFACE (L3, env 75 — NDS mic; needs a new
  interface struct + audio-input source). Pick up when a target system needs them.
- ✅ **Bootless launch** — SHIPPED on `feat/bootless-launch` (2026-06-09;
  awaiting operator playtest). `LaunchRequest.no_rom` + `EmuCommand::LoadRom
  { no_rom }` thread a content-free flag through the Launcher seam to the
  LoadRom handler, which branches to `LibretroCore::load_no_rom()` after
  gating on the live `supports_no_game()` (refuses + toasts if the core
  didn't advertise `SET_SUPPORT_NO_GAME`). New `boot_without_game` +
  `system_supports_bootless` Tauri commands (libretro-only — external-routed
  systems are refused); `system_default_core_supports_bootless` allowlist
  (dosbox / scummvm; backstopped by the runtime check). Frontend:
  `bootWithoutGame` / `systemSupportsBootless` helpers, a `ThemeContext
  .onBootWithoutGame` bridge, and a "▶ Boot without game" button in
  SystemHeader shown only when the capability check passes (enters the
  in-game view with a synthetic "<System> (no game)" title, null
  runningEntry). 822 oa-shell tests pass (+1 capability test); frontend
  typecheck silent. So DOSBox-Pure / ScummVM boot to their built-in browser
  with no content.

### HW-Render Pipeline — Milestone 1 (Vulkan handshake, core on screen)

**Planning locked 2026-06-07.** Full plan at
[docs/PLANS/hw-render-pipeline.md](PLANS/hw-render-pipeline.md); feature
folder [docs/features/hw-render/](features/hw-render/). Implements the
libretro HW render interface so GPU-emulator cores (Dolphin,
paraLLEl-N64, Beetle PSX HW, Flycast, PPSSPP, Beetle Saturn HW) stop
crashing OA. **M1 scope:** add the `retro_hw_render_callback` + Vulkan
HW-negotiation structs to `crates/oa-libretro/src/ffi.rs`; replace the
`SET_HW_RENDER => false` arm in `crates/oa-libretro/src/state.rs` (store
the callback; answer `GET_PREFERRED_HW_RENDER` 56 + `GET_HW_RENDER_INTERFACE`
41) and special-case the `RETRO_HW_FRAME_BUFFER_VALID` sentinel in
`cb_video_refresh`; stand up `crates/oa-render/src/lib.rs` on the Vulkan
backend with a `VulkanHwContext` that shares the wgpu device handles;
branch the HW present path in `apps/oa-shell/src/main.rs`'s `LoadRom`
run loop. **Exit:** a GameCube game renders through internal
`dolphin_libretro` instead of crashing (simplest present bridge OK in
M1 — zero-copy is M2). **Gating:** slot **after VL Phase C3** (both
edit `main.rs`'s `LoadRom` handler — let C3 land first) and **before
Theming ARC 2 (WGSL)**. Vulkan-first per operator (DX12/GL contexts
added later only if needed). ~est. 1-2 sessions for M1.

### Theming Substrate — Phase 4: typed `platform/api/` Tauri bridge ✅ COMPLETE (all 6 slices)

**Queued 2026-06-09.** Plan:
[docs/PLANS/theming-platform-api-bridge.md](PLANS/theming-platform-api-bridge.md).
Feature folder [docs/features/theming-substrate/](features/theming-substrate/).

This is the **last platform/theme decoupling step**. The file-level boundary
is done + lint-enforced (six zones; the `src/components/` grab-bag fully
drained — see SESSION_LOG 2026-06-09). The one remaining coupling is
**content-level**: 351 raw `invoke()` calls across 54 files / 222 distinct
command names bind themes + components directly to backend command-name
strings. Phase 4 corrals them behind typed `platform/api/<domain>Api.ts`
wrappers + an ESLint rule banning raw `invoke()` outside `platform/api/`.

**Slice 1 — `settingsApi` + the wrapper convention ✅ SHIPPED on
`feat/theming-platform-api-settings` (2026-06-09; awaiting operator playtest +
merge).** Created `frontend/src/platform/api/settingsApi.ts` (28 typed wrappers)
and migrated the display/video/audio/shaders + system-settings + per-game-
overrides + shell-mode/kiosk + presentation-mode cluster across 13 files
(App.tsx AV/launch paths, `settings/store.ts`, `lib/audio.ts`, `layout/state.ts`,
`shader_presets.ts`, QuickSettings, GameDialogs, GamePropertiesDialog,
perSystemSections, AnalogBindingsSection, SettingsSections, SystemDialogs,
PerSystemSettingsBody). Convention locked: one typed named export per command,
thin pass-through (call sites keep their own `reportInvokeError`/try-catch),
command string lives only in the wrapper. Three judgment calls (see
SESSION_LOG 2026-06-09): (1) shape-divergent getters (`get_game_overrides` /
`get_system_settings` — each call site declares its own partial view) are
**generic with a canonical default type**, so every call site keeps its exact
local view via the type arg, zero type churn; (2) `layout/state.ts`'s
presentation/kiosk calls migrated but `get/set_layout` left for Slice 2's
`viewsApi` (separate call sites, not entangled); (3) AnalogBindingsSection's
`get_game_overrides` was passing `{ gameId }` where the backend wants `{ id }` —
a **latent bug** (the call silently errored, analog routing fell back to empty);
the typed wrapper corrects it. typecheck + lint green; every migrated command
string greps to only `settingsApi.ts`. The lint rule turns on later (Slice 6,
when the count hits zero). Module map + the 6-slice order are in the plan.

**Slice 2 — `libraryApi` + `collectionsApi` + `viewsApi` ✅ SHIPPED on the
same branch (2026-06-09; awaiting operator playtest + merge).** 37 wrappers
across 3 modules; migrated 8 files (`library/store.ts`, `customCollections.ts`,
`views/store.ts`, `layout/state.ts`'s leftover get/set_layout, `settings/store.ts`
folder commands, App.tsx library paths, ImportWizard folder commands,
routes/GameDetailPanel). Same generic-getter pattern (D14) for the two more
shape-divergent commands (`list_folders`/`add_folder` → LibraryFolderRow vs
Folder; `get_layout` → LayoutPrefs vs `{systemOrder}`). `ingest.ts` deliberately
left untouched (its commands belong to jobs/cores/media slices, not
library/collections/views — assign-by-concern). typecheck + lint green; every
migrated command greps to only its api module. See SESSION_LOG 2026-06-09.

**Slices 1-2 ✅ MERGED to main 2026-06-09** (merge `a5997e3`; operator
playtested).

**Slice 3 — `mediaApi` ✅ MERGED to main 2026-06-09** (merge `f5657c2`; operator
playtested). 28 wrappers / 11 files across art/metadata sync + game-info + mame
+ hashes. DECISIONS D15 (typed-binding modules move + re-export).

**Slice 4 — `coresApi` + `inputApi` ✅ MERGED to main 2026-06-10**
(`feat/theming-platform-api-cores-input`; operator playtested). 29 wrappers /
~50 call sites / 18 files: installed cores + buildbot
catalog + core-options + BIOS (coresApi, 18); bindings + input descriptors +
controller devices + analog routing + light-gun (inputApi, 11). DECISIONS D16 —
the `platform↛components` boundary forces component-local backend-contract types
to re-home INTO the api layer (the api module can't import from a component);
the analog `routing` blob stays a generic `R` param to avoid relocating the
prefs cluster. typecheck + lint green; frontend-only. See SESSION_LOG 2026-06-10.

**Slice 5 — the in-game / gameplay cluster ✅ MERGED to main 2026-06-10**
(`feat/theming-platform-api-gameplay`; operator playtested). Five modules /
~75 call sites / ~14 files, landed as the planned
two-PR split on one branch: PR A = `emulatorApi` (17) + `rewindTasApi` (15);
PR B = `cheatsApi` (12) + `milestonesApi` (6) + `captureApi` (9). launch.ts
stays a rich helper but routes its internal invokes through emulatorApi;
GameDialogs fully drained of raw invoke; namespace imports where wrapper names
shadow local handlers. typecheck + lint green; 56 command strings each grep to
only their api module. See SESSION_LOG 2026-06-10.

**Slice 6 — the closer — `jobsApi` + `systemApi` + `shellApi` ✅ MERGED to main
2026-06-10** (`feat/theming-platform-api-jobs-system-shell`; operator
playtested). 18 + 9 + 19 wrappers + straggler folds (libraryApi prefs /
unidentified, mediaApi clear-metadata); ~90 sites / 21 files. systemInfo.ts D15
move+re-export; logic modules route through wrappers. **Turned ON the
`no-restricted-imports` rule banning raw `invoke` outside `platform/api/`**
(probe-verified); every non-api file is invoke-free. typecheck + lint green.

**✅ PHASE 4 COMPLETE** — 14 typed `platform/api/` modules; the decoupling track
is closed at the file level (six boundary zones) AND the API level (the invoke
ban). A feature physically cannot re-couple platform/theme without ESLint
stopping the commit.

**Phase 4.5 — the EVENT corral ✅ MERGED to main 2026-06-10**
(`feat/theming-platform-api-events`; operator playtested). Closed the symmetric
coupling the post-Phase-4 audit flagged: Tauri event names. New
`platform/api/eventsApi.ts` (`OA_EVENTS` registry + `listenScoped`/`listenTo`/
`emitEvent`); ~30 sites / 16 files migrated (incl. a theme file emitting
`oa://toast` raw); a second `no-restricted-imports` entry bans raw `listen`/
`emit`/`once` outside `platform/api/` (probe-verified). Decision D17.
**Rode along (playtest fixes, operator-confirmed):** jobs-bar z-index
(55 → 65, was hidden behind the engine surface) + an in-app `confirm()`
replacing native `window.confirm`/`alert` (Tauri-2 ACL + async-bypass bug —
destructive-action guards silently never fired). **The foundation is now clean
on BOTH backend-contract channels (commands + events) — ready to build Phase 3
on top.**

**Next theming work — ARC 1 Phase 3, resequenced skeleton-first (2026-06-10).**
Design conversation locked three refinements — see
[PLANS/theming-substrate.md](PLANS/theming-substrate.md) §13 (addendum) +
DECISIONS **D18/D19/D20**:
- **D19** — per-system theming is a Retroverse feature, NOT a substrate contract.
  The substrate's job is **swappable whole-shells** (BigBox-style); per-system
  data stays platform-provided but *consuming* it is each theme's choice. Palette
  pillar is theme-first; per-system tokens are an optional sub-cascade.
- **D20** — kiosk/cabinet capabilities (attract, CRT/shader chrome, multi-monitor
  marquee/manuals/second-controls) are **platform features, engine-owned +
  theme-opt-in via manifest, deferred to ARC 2-3.** Two cheap seams reserved now:
  (a) theme-host lifecycle written as a general "platform preempts + restores the
  theme" pattern (attract later for free); (b) manifest declares named
  **surfaces**, ARC 1 honoring exactly one (`main`). CRT/shaders need nothing now.
- **Skeleton-first resequence** — pull the vertical slice forward: stand up TWO
  switchable whole-shells early (Retroverse + a rough **Wheel**), then deepen the
  substrate underneath a working swap. ARC boundaries unchanged.

**Revised slice order** (full detail in plan §13.3): **S1** nav foundation (lock
verb vocab; relocate `src/nav/` → `platform/nav/`; input→verb `navBindings`
OA-wide + `platform/api/` wrapper; `list`/`grid` primitives verb-native +
declarative props) **✅ shipped + merged 2026-06-10 (DECISIONS D21)** → **S2**
walking skeleton (minimal restart-based theme switch + Retroverse-as-default-theme
+ rough Wheel; **swap gate — the dream becomes visible**) → **S3** token layer
(design-token contract + a11y/motion baseline + engine-territory token isolation;
write `THEME_CONTRACT.md`) → **S4** versioned manifest + load-time validator + CI
fixture (`bare` theme = fixture) → **S5** substrate depth (palette JSON + scoped
CSS-var injection, asset resolver + `ui-sound` category, HintBar glyph-set seam,
per-theme settings namespace, `wheel`/`carousel`/`custom` primitives).
**Follow-on (after S2):** nav-remap Settings UI — gamepad + keyboard rebind to
verbs, conflict validation, always-reachable escape hatch, **"Reset to defaults"**
= operator-locked nav spec.
**S2 (walking skeleton) is the immediate next code.** Theme-swap = restart in ARC 1
(no hot-swap yet). S1 delivered the nav layer S2's themes consume:
`@oa/platform/nav` (verbs, `navBindings`, `list`/`grid` primitives, verb-native
HintBar).

> Earlier theming arcs (Phase 1 engine/theme surface separation, Phase 2
> platform extraction, the boundary-enforcement track, the grab-bag drain) are
> all **shipped + merged**. The 3-arc *enable-other-themes* track (Phase 3 nav
> primitives, Phase 5 packaging, Phase 6 Retroverse-as-theme, ARC 2-3 Rhai +
> WGSL + Theme Studio) follows Phase 4 and builds on the now-clean boundary.
> Decisions: [features/theming-substrate/DECISIONS.md](features/theming-substrate/DECISIONS.md).

~~### Guided Setup Phase 2 — curated CPU-tier core selection~~ —
**SHIPPED 2026-06-06** on `feat/guided-setup-cpu-tier`. Decision-locked
2026-06-06 per plan §7: heuristic is source of truth, benchmarks
explicitly deferred (possible Phase 2B if operator feedback names
the heuristic as the bottleneck). 4 code slices + the decision-lock
docs commit:

- Slice 1 — `apps/oa-shell/src/cpu_tier.rs`: `sysinfo` detection +
  `CpuTier { High | Mid | Low }` + `bucket_into_tier` (thresholds:
  ≥6 cores & ≥3.0 GHz → High; ≥4 cores & ≥2.5 GHz → Mid; else Low) +
  `detect_or_load` with cache to `<appDataDir>/cpu-tier.json` +
  `LibraryPrefs.cpu_tier_override: Option<CpuTier>` escape hatch +
  `detect_cpu_tier` Tauri command. 7 tests cover bucketing edges +
  brand-string clock parsing variants. Documented Steam Deck
  under-rate / old Xeon over-rate / thermal-throttled-laptop /
  GPU-bound-core limitations as the operator-override use cases.
- Slice 2 — `core_installer.rs::TIER_PREFERENCES` + `recommended
  _core_for_tier(system_id, tier)`. Per-system rows for psx / snes /
  n64 / genesis / saturn / ps2 / nds (7 systems). Two patterns: PSX +
  Saturn + Genesis have distinct cores per tier; SNES + N64 + PS2 +
  NDS reuse the accuracy core for High AND Mid (only Low picks the
  lighter alternative — avoids over-recommending lightweight cores
  to mid-tier hardware). 5 tests including a `tier_preference_bases
  _all_exist_in_catalog` referential-integrity check.
- Slice 3 — Settings → Performance card in `SettingsSections.tsx` +
  `SettingsPage.tsx` category wiring. Read-only detected hardware
  display (brand / cores / base clock) + tier chip with provenance
  hint + Auto / High / Mid / Low override drop-down that round-trips
  through `set_library_prefs`.
- Slice 4 — `recommended_core_for_system(systemId)` Tauri command +
  `SystemReadinessChecklist.tsx` consumes it via parallel resource +
  Core pill detail surfaces `Using {core} (high-tier pick)` on ✓
  rows and `Install {core} ({tier}-tier pick)` on ⚠ rows. Three
  suffix shapes: `(high-tier pick)` / `(override-set high-tier
  pick)` / `(system default)` for the single-core systems.

Plan: [docs/PLANS/guided-setup.md](PLANS/guided-setup.md) §7 + §13.


## MEDIUM — Phase 3+ polish

~~**2026-06-08 — Audio quality pass: clipping + clicking on some cores (NES confirmed).**~~
**ROOT CAUSE FIXED 2026-06-08** on `feat/audio-quality` (commit `0bb4e89`).
Not amplitude clipping — a **sample-rate-feed bug**. The shell never adopted
the core's real timing after `retro_load_game`: `LibretroCore::new` seeds a
placeholder `Timing` (44100 Hz / 60.000 fps) because most cores can't report
real `av_info` until a ROM is loaded; `finish_load` snapshots the true values
into the core, but the shell's local `timing` + the oa-audio sink were built
from the placeholder and never refreshed. The linear resampler was fed
`source_rate = 44100` for every core → wrong sample count + wrong pitch
(fceumm 48000 overproduced → ring overflow/glitch + high pitch; snes9x 32040
underproduced → underrun crackle + low pitch). PCE worked by coincidence
(real rate == placeholder); N64 worked only because it calls env 32, the one
path that already rebuilt the sink. Confirmed by debug-log feed-rate math
(48 kHz stereo device drains 96000 i16/s; measured Δpushed/s matched the
rate-ratio off-by exactly). **Fix:** after `load_rom` in BOTH the runtime
LoadRom handler and the cold-start direct-launch path, refresh
`timing = core.timing()` + rebuild the sink + retime the limiter when the real
rate/fps differs; env 32 stays the secondary later-revision path. Operator
playtest: "sounds much better."
- **Deferred:** (1) "damn accurate" verification pass — operator wants to
  confirm pitch/timing is exact across the lineup later. (2) IF any core still
  sounds genuinely hot after the rate fix, add a master soft-limiter in
  `oa-audio` (NOT added preemptively — the data said rate, not amplitude).
  Re-open this entry only if a true amplitude clip is observed.
- Files: `apps/oa-shell/src/main.rs` (both load paths). See
  `docs/features/audio-quality/SESSION_LOG.md`.


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

~~### Migrate analog-stick topology to per-system YAML descriptor~~ —
**SHIPPED 2026-06-05** on `feat/analog-sticks-yaml`. New
`AnalogSticksDescriptor` tagged enum (`kind: single|dual` with owned
`String` labels) added to `system_descriptor.rs` schema. All 8
systems with non-None topology (n64 / gamecube / dreamcast / psp /
psx / ps2 / saturn / virtualboy) carry the `analog_sticks:` block
in their `system.yaml`; digital-only systems omit the block. The
runtime `bindings::AnalogSticks` enum changed `&'static str` → owned
`String` to allow YAML-sourced data. `analog_sticks_for(system_id)`
now reads through `system_registry::global_registry()`. Data-
equivalence test asserts every previously-hardcoded mapping still
resolves identically. Closes the per-system-descriptors migration's
last Rust holdout — bindings.rs's match arm deleted. 732/732
oa-shell tests pass.

~~### Honor SET_MINIMUM_AUDIO_LATENCY (env 63) for crackle-free heavy frames~~ —
**SHIPPED 2026-06-05** on `feat/honor-min-audio-latency`. Parser in
`crates/oa-libretro/src/state.rs::parse_min_audio_latency_ms` reads
the `unsigned` ms payload + caps at the spec's 512 ms ceiling against
buggy cores; singleton State stores the value;
`oa_libretro::loaded_core_min_audio_latency_ms()` exposes it; oa-shell
LoadRom handler calls `oa_audio::AudioSink::ensure_min_latency_ms`
post-load which grows the ring buffer when the request exceeds the
default 16384-sample capacity (~170 ms at 48 kHz stereo, so most
cores' 64-100 ms requests no-op at 48 kHz). 5 parser tests (typical /
zero / cap / boundary / null) + 5 oa-audio math tests (per-sample-rate
capacity math + default-covers-typical sanity check). Operator
validation: launch Genesis / PSX / Dreamcast game, watch
`oa-current.log` for `SET_MINIMUM_AUDIO_LATENCY = N ms` then optionally
`growing ring buffer for N ms` if N exceeds default.


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
~~13. **Right D-pad bindings for Virtual Boy**~~ — **ALREADY SHIPPED 2026-05-24.** Verified 2026-06-06 against Beetle VB's `libretro.cpp`: the core polls the right D-pad via `input_state_cb(j, RETRO_DEVICE_ANALOG, RETRO_DEVICE_INDEX_ANALOG_RIGHT, RETRO_DEVICE_ID_ANALOG_{X,Y})` and applies its own deadzone. OA's shared analog routing already feeds that exact channel: `default_analog_routing("virtualboy")` in `apps/oa-shell/src/system_settings.rs:441` sets `gamepad_source: "right"` for port 0 + Numpad 8/2/4/6 keyboard fallback, and the per-system YAML (`config/systems/virtualboy/system.yaml`) declares `analog_sticks: Dual { left_label: "Left D-pad", right_label: "Right D-pad" }` so the per-system Bindings UI renders the right-stick panel labeled "Right D-pad" out of the box. `virtualboy/ROADMAP.md` line 32 ✅ already reflects this. Entry was double-stale in DEFERRED and then LOWER — the bullet I moved 2026-06-06 was wrong; this is shipped, not pickup-able. Striking now to match reality.
14. **Real OS-level accelerometer access** (~250 LOC). Windows Sensor API / Linux iio / macOS Core Motion. Phase G's keyboard-arrows-as-tilt fallback already handles GBA Boktai / Kirby Tilt 'n' Tumble / WarioWare Twisted!. Real-sensor access unlocks GBA tilt-native + NDS gyro-native motion + future Wii motion games (once Wii Remote dispatch reaches that gate). Moved out of DEFERRED 2026-06-06 — there's no shared-infra dependency, the platform-specific sensor bindings just haven't been written.
15. **Trackball / mouse delta semantics validation** (~30 LOC + operator testing on real hardware). Libretro `RETRO_DEVICE_MOUSE` is spec'd as delta-based; the current pointer-as-mouse dispatch sends absolute coords. Arcade trackball games (Marble Madness, Centipede on MAME) need delta. The fix derives delta from previous-frame absolute in `cb_input_state`. The "validation" half needs an operator with a real trackball cabinet — that's the actual gate, not infra. Moved out of DEFERRED 2026-06-06.
16. **GameCube Wii Remote / Nunchuk / Classic Controller dispatch** — code-side is ALREADY shipped via the dynamic-controller-info arc. Dolphin publishes the Wii peripherals via SET_CONTROLLER_INFO at ids 513 / 769 / 1025 / 1281 / 1537; they appear in the per-game Input dropdown automatically. Non-motion Wii games (Brawl + Classic Controller, NSMB Wii sideways grip, Trauma Center stylus, Skyward Sword IR-via-mouse) work today. **Motion-required games** (Wii Sports, Mario Galaxy spin, MotionPlus titles) wait on item #14 (real-OS accelerometer access). Moved out of DEFERRED 2026-06-06 as a pointer to that dependency — no work item under this bullet itself.

---

## DEFERRED — blocked on shared infra not yet triggered

These wait for a single, larger infrastructure pass that benefits many systems at once. Each line item below names what unlocks the deferred work.

**2026-06-06 band audit:** Several entries that had been sitting here turned out to be either (a) not actually blocked anymore (shared analog infra shipped, so VB right D-pad unblocked) or (b) effectively superseded by a different solution (vector-phosphor shader makes the Vectrex native vector renderer largely unnecessary). The surviving entries below are genuinely waiting on infra that doesn't exist yet.

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
- **Dreamcast VMU peripheral** (~400 LOC + ~1 week render infra). The VMU is the memory card with a 48×32 LCD that appears in-game (HotD 2 ammo counter, Sonic Adventure Chao pet, Skies of Arcadia compass). `oa-render` has no generic "secondary screen surface" concept — NDS dual-screen is a special case, not a reusable pattern. To ship VMU you'd genericalize the secondary-screen render path (~1 week of `oa-render` work) and then VMU dispatch is the easy half. Genuinely blocked on infra.
- **Modern VR for Virtual Boy via OpenXR** (~800 LOC + OpenXR integration + dual-eye render pipeline + real VR headset to test). Side-by-side dual-perspective to a headset. Phase 2+ stretch. Genuinely blocked on infra AND hardware.
- ~~**Custom-built Vectrex vector renderer**~~ — **EFFECTIVELY OBSOLETE.** Was listed as Phase 3+ stretch (~500 LOC replacing vecx raster with native wgpu vector strokes). The `vector-phosphor` shader preset shipped 2026-05-29 makes vecx output look like vector strokes with bloom + persistence — ~95% of the visual win for ~0% of the work. The native vector renderer would be a purity project not a functionality fix. Moved to PARKING_LOT 2026-06-06.
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
- **System info content — Wave 2 (Pass C)** — 29 systems still need their `system_info:` block in `config/systems/<id>/system.yaml`. Wikipedia-sourced methodology established in Pass A + B (5 + 12 systems shipped 2026-06-04 via `content/system-info-pass-a-pre-wave-1` + `content/system-info-pass-b-wave-1-verify`). Each entry needs: WebFetch Wikipedia → identify fields → drop where uncertain → write with established blurb voice → add `meta.contributors: [URL]`. Split into three sub-passes:
  - **C1 — Mainstream consoles + handhelds (12 systems)**: `gba, gbc, n64, gamecube, ps2, psp, nds, dreamcast, saturn, segacd, neogeo, neocd`. Wikipedia coverage strong; expect ~1 session at the Pass B cadence. Operator likely has editorial opinions on the big-name blurbs.
  - **C2 — Long-tail consoles (14 systems)**: `2600, 3do, 5200, channelf, intv, jagcd, jaguar, ngp, o2, pcfx, pokemini, sega32x, sega32xcd, stv`. Wikipedia coverage variable — well-covered for 2600 / Jaguar / Intellivision, thinner for PC-FX / ST-V / Channel F / Odyssey². Expect more honest field-dropping per the "no fabrication" methodology. ~1 session.
  - **C3 — Emulator host platforms (3 systems)**: `mame, scummvm, dosbox`. These aren't consoles — they're software platforms hosting other games. The `system_info` block needs adapting before writing (units_sold, media, release_date don't apply in the console sense). Schema-fit discussion before writing. ~0.5 session.
  Where the work lands: `config/systems/<id>/system.yaml` per system, `meta.contributors` list per entry. Tests at `apps/oa-shell/src/system_info.rs::tests::registry_load_finds_all_v1_panel_systems` exercise every shipped YAML at load time — green = parses cleanly. No code changes expected unless schema-fit adjustment is needed for C3.

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
- **Per-system theming** — `frontend/src/platform/themes/systemPalettes.ts` (typed `SYSTEM_PALETTES` map, injected as `[data-system]` CSS at boot; `systems.css` retired, D26) + `platform/themes/registry.ts`.
- **Bindings UI** — `engine/SystemBindingsEditor.tsx` renders button-name chips per system.
- **CJK font fallbacks** — `frontend/src/index.css::--font-display` covers PC-FX + FDS Japanese-only libraries.
- **Multi-core CPU awareness (rayon + tokio blocking pool + zstd + parallel boot)** — Shipped 2026-05-21 on `feat/multicore-cpu-awareness`. Workspace gains `rayon` (1.10); five cold-path bottlenecks now parallelize. Media sync wraps `generate_thumbnail` in `tokio::task::spawn_blocking` so decode/resize/encode runs across cores while `buffer_unordered(8)` keeps the network side busy. ROM hash resolve pre-populates the `hash_cache` via `par_iter` inside `spawn_blocking` — the cartridge read+SHA-1+header-strip work saturates all cores before the for-loop's DB-write phase. Rewind ring (`oa-savestate`) compresses every snapshot at zstd level 1 — 5–10× memory reduction lets the 64 MiB cap hold proportionally more rewind history. Boot-time `archive::sweep_temp` + `read_media_db` + `read_media_prefs` + `library_db::open` fan out to four `std::thread::spawn` workers, joining at point-of-use so the wgpu/WebView init runs concurrently with the disk reads — 100-400ms cold-start savings. Project-wide rationale lives in `docs/DECISIONS.md` 2026-05-21 entry.
- **libretro memory map storage** — Shipped 2026-05-30 on `feat/libretro-env-callbacks-batch`. `RETRO_ENVIRONMENT_SET_MEMORY_MAPS` (env 36) parses the descriptor array into `oa_core::MemoryDescriptor` values (`flags`, `offset`, `start`, `select`, `disconnect`, `len`, `addrspace`) accessible via `Core::memory_map()`; host base pointers stored separately as `usize` in `State.memory_map_ptrs`. Cleared on `load_rom` alongside rotation so back-to-back swaps don't inherit stale descriptors. Unblocks future RetroAchievements rcheevos integration, cheat-search address translation, and AI/scripting layers that read game state by symbolic guest address. 3 unit tests in `state.rs::tests` cover null pointer / zero-count / 2-region NES-shape map.
- **libretro core OSD → toast** — Shipped 2026-05-30 on same branch. `RETRO_ENVIRONMENT_SET_MESSAGE` (env 6) + `SET_MESSAGE_EXT` (env 60) + `GET_MESSAGE_INTERFACE_VERSION` (env 59, returns v1). Cores' OSD messages ("Save state slot N saved", "Disc swapped", BIOS-fallback warnings) queue as `oa_core::CoreMessage { text, level, log_only }` on `State.pending_messages`; the emu thread drains each frame via `Core::drain_messages()` and emits as `oa://toast` events through the existing `emit_toast` helper. Toasts pick up the active system theme via `current_system_id`. `target=LOG` messages log-only (skip toast); duration/priority/type fields ignored in v1 (toast stack has its own schedule). Future polish: route `MESSAGE_TYPE_PROGRESS` to a progress-bar widget.
- **libretro `SET_SUPPORT_NO_GAME` flag + `load_no_rom()`** — Shipped 2026-05-30 on same branch. Env 18 captures the bool into `State.supports_no_game`; `LibretroCore::supports_no_game()` exposes it to the shell. `LibretroCore::load_no_rom()` calls `retro_load_game(NULL)` for cores that advertised support (DOSBox-Pure built-in browser, ScummVM engine launcher, etc.). Post-load common work (controller port wiring, av_info snapshot) extracted into `finish_load()` shared with `load_rom`. Shell-side UI button for bootless launch still ⬜ — operator-driven, low priority.
- **libretro disc-control v2 extras** — Shipped 2026-05-30 on same branch. Four v2-only function pointers previously stored-but-unused now have `LibretroCore` methods: `add_disc_image()`, `replace_disc_image(idx, path)`, `set_initial_disc_image(idx, path)` (multi-disc resume; cores that register interface late can't honor — returns false), `disc_image_path(idx)`. `DiscInfo` gains `paths: Vec<String>` populated from `get_image_path` for v2 cores; v1 fallback returns empty. `read_disc_string_field` helper collapses get_image_label / get_image_path buffer-fill duplication. Frontend `QuickSettings.tsx` `DiscInfo` type extended with optional `paths` field for future tooltip polish.
- **Game Info Panel v1** — Shipped 2026-05-30 on `feat/game-info-panel-v1`. Three-layer data model (file layer at `docs/cores/<id>/games-info.md` + SQLite `game_info_overrides` table + field-typed precedence merge) feeding three UI surfaces. Rust types live in `apps/oa-shell/src/game_info.rs` (GameInfo / GameInfoOverride / MergedGameInfo / GameInfoBadge); SQLite migration v15 adds the overrides table; six Tauri commands cover read (`get_game_info`, `get_game_info_override`), write (`set_game_info_override`, `delete_game_info_override`), and bulk queries (`list_game_info_overridden`, `list_game_info_badges`). Frontend surfaces: Retroverse `GameDetailPanel` gains Operator note / Controls / Recommended core (+Apply best emulator action wired through `update_game_core_override`) / Known issues sections; `LibraryTile` gains bottom-right `⚠ N` + `✎` badges; `GameInfoModal` gains a 4th "Game info" tab with an inline editor (short summary + controls supported + recommended core + bugs add/remove + Submit correction stub). Files: `apps/oa-shell/src/game_info.rs`, schema in `docs/cores/SCHEMA.md`, plan in `docs/PLANS/game-info-panel.md`. v1 ship includes a seed `psx/games-info.md` (Tomb Raider + FF7); cross-system migration of `KNOWN_GAME_BUGS.md` content is operator-driven follow-up. v2 evolution (separate data repo + scraper + GitHub-Issue submission flow) designed but deferred per plan §11. **Schema extended 2026-05-31** with the optional `touch_hotspots: [{ label, x, y, w, h }]` field — NDS-specific in practice today; coordinates in NDS bottom-screen native space (0..256 × 0..192). The new `TouchHotspotOverlay` component (`frontend/src/platform/components/TouchHotspotOverlay.tsx`) renders labelled accent outlines over the bottom-screen area while a stylus-using game runs; toggle lives in QuickSettings → "Show touch hints" (per-session, NDS-gated). Seed entries in `docs/cores/nds/games-info.md` (Phantom Hourglass / Brain Age / Trauma Center). v1 limitation: assumes default melonDS stacked-vertical screen layout; non-default layouts (side-by-side, top-only) misplace hotspots until v2 reads the core option.

- **System Info Panel v1** — Shipped 2026-06-01 on `feat/system-info-panel-v1`. Three-layer per-system metadata replacing the hand-typed-5-of-45 `frontend/src/routes/retroverse/systemMetadataStubs.ts`: L1 (MAME baseline, baked at launch from `assets/mame-source/listxml-slim.json` + `history-slim.xml` shipped by `tools/mame-extractor/`), L2 (curated YAML at `docs/cores/<id>/system-info.yaml`, baked into SQLite `system_info_curated`), L3 (per-install operator overrides in SQLite `system_info_overrides`). Rust types + parsers + field-typed merge live in `apps/oa-shell/src/system_info.rs`; SQLite migration v16 adds the four tables (`system_info_mame` / `_curated` / `_overrides` / `_meta`); six Tauri commands cover read (`get_system_info` merged, `get_system_info_override` raw L3, `get_system_info_curated` raw L2), write (`set_system_info_override`, `delete_system_info_override`, `reset_system_info_to_default`), and operator-driven L1 re-import (`refresh_mame_system_info`). The operator-driven refresh in `apps/oa-shell/src/mame_import.rs` mirrors the maintainer-time `tools/mame-extractor/` parser in-process — detects MAME at `<exe_dir>/Emulators/MAME/` first, shells out to `mame -listxml` + reads local `history.xml`, overwrites L1 without touching L2 or L3. Frontend surfaces: Retroverse `SystemInfoPanel` + `HomePage` hero consume the merged record via `getSystemInfo`; new `PerSystemInfoSection` (per-system Settings drill-in) is the L3 edit UI with form-row-per-field input + provenance badges (no badge = L1 default; slate "curated" = L2; accent "edited" = L3) + peripherals editor + Reset all overrides button; `StorageSettings` gains a "Refresh MAME system info" card with folder-picker fallback. Schema reference in `docs/cores/SCHEMA.md` (system-info.yaml section); plan in `docs/PLANS/system-info-panel-v1.md`. v1 seed L2 YAMLs for snes / nes / genesis / psx / gb (5 of 45 systems migrated from the old stub data); remaining 40 fall through to L1 only. Three OA slugs lack MAME data entirely in MAME 0.288 (`3do` model-specific only, `msx` + `msx2` software-list-only) and stay L2-only per plan §5's same recipe DOSBox/ScummVM use. v2 candidates (session-scoped re-imports → sticky; bundled-only L1 → scheduled refresh from `overlooked-arcade-system-info` repo) stay parked. **SCHEMA_VERSION constant trap** surfaced + fixed during the rollout: the early-return `if current == SCHEMA_VERSION` in `bootstrap_schema` short-circuits all migrations when the constant isn't bumped with each new migration. Game Info Panel v1's v14→v15 had also shipped without the bump (silently absent `game_info_overrides` on any v14 install); constant now sits at 16 with a long inline comment calling out the trap.
- **Project-wide `Emulators/` convention** — Shipped 2026-06-01 with System Info Panel v1 Phase 1a. Top-level `Emulators/` directory at the repo / install root is the canonical home for every third-party emulator binary OA shells out to (MAME today; DOSBox-X / ScummVM standalone / etc. eventually). `tools/bump-mame.sh` + `apps/oa-shell/src/mame_import.rs::detect_mame_binary` both probe `<root>/Emulators/MAME/mame.exe` first; the shipped install applies the same shape at `<exe_dir>/Emulators/MAME/`. `/Emulators/` added to `.gitignore`. Future external emulators follow the recipe `<root>/Emulators/<name>/`.

When you add new cross-system infrastructure, append it here so the next session knows it can be leaned on.
