# Per-system descriptor consolidation — multi-slice arc

**Status:** Planning (operator-approved 2026-06-01, execution deferred to a future session). No code yet.

**Owner-of-decisions:** the operator. This document records what was decided + the implementation roadmap. Revisit individual decisions if any feel wrong when the executing session begins.

---

## Context

After Phase 1B (guided-setup wizard upgrade) closed 2026-06-01, the operator asked ChatGPT for gap-spotting feedback against the [CHATGPT_BRIEFING.md](../CHATGPT_BRIEFING.md). ChatGPT flagged several "missing pillars" in OA's architecture. The operator's read of those: most were SaaS-flavored and off-philosophy (plugin API, community curation, conflict tooling — all parked this session), but two threads were genuinely worth pursuing:

1. **Unify where per-system information is stored** so it's editable outside OA + sets up content packs to land cleanly. **(This arc covers this.)**
2. **Formalize the L1/L2/L3 data hierarchy** that's currently shipped ad-hoc in System Info Panel v1 + Game Info Panel v1 + core_options. **(Falls out of this arc as the explicit layer model.)**

A separate future plan will cover **decision-trace UI** ("Why is this setting X?" affordances over the resolution chain). That work depends on the data model this arc builds.

### What's scattered today (the consolidation target)

Per-system data lives in ~8 places:

| Source | Today | Moves to |
| --- | --- | --- |
| `apps/oa-shell/src/main.rs::*_BIOS_KNOWN_HASHES` (19 systems) | Hardcoded Rust const tables | `config/systems/<id>/bios.yaml` |
| `apps/oa-shell/src/core_installer.rs::CATALOG` (50+ entries) | Hardcoded; `systems[]` array per entry | `config/systems/<id>/system.yaml` (cores section) |
| `apps/oa-shell/src/light_gun_systems.rs::LIGHT_GUN_SYSTEMS` | Hardcoded ~7-entry table | `config/systems/<id>/system.yaml` (light-gun section) |
| `apps/oa-shell/src/rom_hashes.rs::libretro_dat_refs_for_system` | Hardcoded 45-arm match | `config/systems/<id>/system.yaml` (libretro_dat_refs) |
| `apps/oa-shell/src/main.rs` device-id dropdowns (Saturn, GC, etc.) | Hardcoded `DEVICE_ID_OPTIONS_*` arrays | `config/systems/<id>/system.yaml` (input section) |
| `docs/cores/<id>/system-info.yaml` (L2 curated for System Info Panel) | In-tree docs | `config/systems/<id>/system.yaml` (merged) |
| `docs/cores/<id>/games-info.md` (Game Info Panel YAML records) | In-tree docs | `config/systems/<id>/games.yaml` |
| `frontend/src/themes/registry.ts::systemThemes` | TS const | **stays** for now (defer to kiosk-mode work) |

### Operator-locked decisions (2026-06-01)

- **Scope:** Rust const tables + in-tree machine-readable docs. NOT frontend `systemThemes` registry / CSS yet (revisit when kiosk-mode themes work happens).
- **File shape:** **three files per system** — `system.yaml` + `games.yaml` + `bios.yaml` — in a new top-level `config/systems/<id>/` folder.
- **Load model:** **runtime load on app start.** Rust reads the YAMLs into in-memory structures; content packs slot in as additional YAMLs layered on top.
- **Migration strategy:** pilot first — design + 3 representative systems in Slice 1, evaluate, then sweep the remaining 38 in Slice 2. **Pilot systems: GB + PSX + NDS** (small-cart, OR-of-variant BIOS, multi-file AllRequired BIOS — three different shapes covered).
- **Layer model going forward (explicit):**

  | Layer | Source | Editable by |
  | --- | --- | --- |
  | **L1 — Engine defaults** | Rust const (fundamental fallback only) | Code change |
  | **L2 — OA shipped descriptors** | `<repo>/config/systems/<id>/` (in-tree, ships with OA) | OA dev + accepted PRs |
  | **L3 — Installed content packs** | `<appDataDir>/content-packs/<pack>/systems/<id>/` | Pack publisher; operator installs |
  | **L4 — Operator overrides** | SQLite tables (as today) | Operator via OA UI |

  Higher layer wins per-field. L3 + L4 wire-up happens in **Slice 3**; Slices 1 + 2 ship L2 only (L1 keeps the existing hardcoded fallback for safety).

---

## Slice 1 — schema design + loader + 3 pilot systems

### What changes

#### New `apps/oa-shell/src/system_descriptor.rs`

Three serde-derived structs matching the YAML files:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]  // typos in field names fail loud
pub struct SystemDescriptor {
    pub id: String,                        // matches the folder name + SystemId slug
    pub display_name: String,
    pub short_name: String,
    pub release_year: Option<u32>,
    pub manufacturer: Option<String>,
    // ... System Info Panel L2 fields (refresh_rate, cpu, ram, peripherals, etc.)

    pub default_core: Option<String>,      // operator-overridable fallback
    pub default_shader_preset: Option<String>,
    pub extensions: Vec<String>,
    pub libretro_dat_refs: Vec<LibretroDatRef>,

    pub cores: Vec<CoreEntryDescriptor>,   // was CATALOG.iter().filter
    #[serde(default)]
    pub light_gun: Option<LightGunDescriptor>,  // optional, only for ~7 systems
    #[serde(default)]
    pub input: Option<InputDescriptor>,    // device-id options dropdowns
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BiosDescriptor {
    pub semantics: BiosSemantics,          // any_of | all_required
    pub required_for_launch: bool,         // PSX hard-requires; GBA optional
    pub files: Vec<BiosFileEntry>,         // canonical hash table
    pub sourcing_hint: Option<String>,     // "Where to get it" body
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GamesDescriptor {
    pub schema_version: u32,
    pub games: Vec<GameInfoEntry>,         // matches existing GameInfo shape
}
```

Plus supporting `LibretroDatRef`, `CoreEntryDescriptor`, `LightGunDescriptor`, `InputDescriptor`, `BiosFileEntry`, `GameInfoEntry` (mirrors existing `game_info::GameInfo` shape so the migration is a rename, not a reshape).

#### New `apps/oa-shell/src/system_registry.rs`

The runtime loader + lookup surface:

```rust
pub struct SystemRegistry {
    by_id: HashMap<String, LoadedSystem>,  // SystemId → all three files merged
}

pub struct LoadedSystem {
    pub descriptor: SystemDescriptor,
    pub bios: Option<BiosDescriptor>,      // None for systems without BIOS
    pub games: Option<GamesDescriptor>,    // None for systems without games-info
    pub source_path: PathBuf,              // for error reporting + L3 layering later
}

impl SystemRegistry {
    pub fn load_from_in_tree(config_root: &Path) -> Result<Self, RegistryError>
    pub fn get(&self, system_id: &str) -> Option<&LoadedSystem>
    // ... read-only accessors per concern
}
```

Loaded once at app startup, stored in Tauri state, accessed by every consumer that used to read the hardcoded const tables.

#### Loader behavior

- Walks `config/systems/` (resolved relative to the workspace root for in-tree; via embedded asset path for shipped builds — TBD during impl, likely `include_dir!` crate or similar for the shipped case).
- For each `<id>/` subfolder, reads `system.yaml` (required), `bios.yaml` (optional — system may have no BIOS), `games.yaml` (optional — system may have no curated game records).
- `deny_unknown_fields` catches typos at load time.
- **Hot-fails on app start** if any system's YAMLs are malformed — doesn't silently skip the broken system. Error message names the file + the serde error path so operator can fix quickly.
- Logs a summary at INFO: "loaded N systems from config/systems/ in Xms".

#### Migration of 3 pilot systems (GB + PSX + NDS)

For each pilot:

1. **Write `config/systems/<id>/system.yaml`** — merge existing `docs/cores/<id>/system-info.yaml` + extract CATALOG cores filtered by `systems.includes(<id>)` + extract `libretro_dat_refs_for_system` arm + (for PSX/NDS only) extract input device-id options.
2. **Write `config/systems/<id>/bios.yaml`** — port the relevant `*_BIOS_KNOWN_HASHES` const into structured form. GB has no BIOS table; bios.yaml is omitted. PSX = `any_of` semantics, 18 candidate files. NDS = `all_required` semantics, 3 files.
3. **Write `config/systems/<id>/games.yaml`** — rename + move existing `docs/cores/<id>/games-info.md`. Schema shape stays the same (the existing parser handles it).
4. **Delete the corresponding content from `docs/cores/<id>/`** — `system-info.yaml` + `games-info.md` go away. README, ROADMAP, SESSION_LOG, DECISIONS, KNOWN_GAME_BUGS.md all stay (human-narrative).

#### Consumer wiring (fallback to hardcoded const for the 38 unmigrated systems)

The hard part. Each consumer of the old const tables needs a "prefer-registry, fall back to hardcoded const" lookup pattern:

```rust
// rom_hashes.rs example:
fn libretro_dat_refs_for_system_resolved(
    registry: &SystemRegistry,
    system_id: &str,
) -> &[DatRef] {
    if let Some(sys) = registry.get(system_id) {
        return &sys.descriptor.libretro_dat_refs;  // L2 source
    }
    libretro_dat_refs_for_system(system_id)  // L1 hardcoded fallback (existing fn)
}
```

Consumers updated for Slice 1:

- `apps/oa-shell/src/main.rs::check_*_bios` for PSX + NDS (GB has no BIOS) — use registry-loaded `BiosDescriptor` instead of the hardcoded `PSX_BIOS_KNOWN_HASHES` / `NDS_BIOS_KNOWN_HASHES`. Other 17 BIOS systems still use the const tables.
- `apps/oa-shell/src/core_installer.rs::available_cores` — when building the catalog, MERGE the in-tree const CATALOG with the registry's `cores` (the 3 pilots' cores get filtered out of the const path and read from the registry).
- `apps/oa-shell/src/rom_hashes.rs::libretro_dat_refs_for_system` — same merge pattern.
- `apps/oa-shell/src/main.rs::known_hashes_for_system` (the dispatcher added in Slice 5 for `install_bios_file`) — same merge.
- `apps/oa-shell/src/game_info.rs::parse_games_info_file` — change the load path for PSX + NDS to `config/systems/<id>/games.yaml` instead of `docs/cores/<id>/games-info.md`.
- `apps/oa-shell/src/system_info.rs::*` — same path change for the System Info Panel L2 load.

After Slice 1: PSX + NDS run entirely off the registry (zero hardcoded references to their data); GB runs off the registry except for BIOS (it has none). All other systems unchanged — they keep reading from hardcoded const.

### Tests

- New `system_descriptor::tests::all_in_tree_yamls_parse_and_validate` — walks `config/systems/`, loads every descriptor, asserts every one passes serde + cross-validates (e.g., `cores[].base` not empty, `extensions[]` not empty, `bios.files[].sha1` is 40 hex chars uppercase).
- Per-pilot regression tests — `psx_bios_check_via_registry_matches_legacy_const` + `nds_multi_file_bios_check_via_registry`.
- 615 oa-shell tests stay green; add ~10 new tests for the registry + loader.

### Verification

1. `cargo test -p oa-shell` ≥ 625 green.
2. `cargo tauri dev`; launch a PSX game with a known-canonical BIOS already in `<exe_dir>/system/` → ✓ BIOS ready in readiness checklist + game launches as before.
3. Launch an NDS game with only `bios7.bin` present → BIOS pill expands inline showing `bios7.bin ✓ Ready`, `bios9.bin ⚠ Missing`, `firmware.bin ⚠ Missing` (Slice 5 UX unchanged).
4. Drop a PSX BIOS file via "Pick BIOS file…" → install succeeds, pill flips ⚠ → ✓.
5. Open the readiness checklist for GB; ✓ Core installed (Gambatte matched via the registry's cores list); ↪ BIOS not required (no bios.yaml in `config/systems/gb/`).
6. `Help → Debug log…` shows "system_registry: loaded 3 systems from config/systems/ in Xms" at startup, plus warnings if any in-tree YAML failed to parse (none should — test catches those before app start).
7. The migrated `docs/cores/gb/`, `docs/cores/psx/`, `docs/cores/nds/` folders now contain ONLY human-narrative files (README, ROADMAP, SESSION_LOG, KNOWN_GAME_BUGS.md, DECISIONS.md). The .yaml + .md data-bearing files have moved.

---

## Slice 2 — migration tool + remaining 38 systems

### What changes

#### `apps/oa-shell/src/bin/migrate_systems.rs` (new dev binary)

```bash
cargo run --bin migrate_systems
```

Walks the existing Rust const tables (PSX_BIOS_KNOWN_HASHES etc.) and the existing `docs/cores/<id>/system-info.yaml` + `games-info.md` files, emits the 3 YAMLs per system into `config/systems/<id>/`. Idempotent — re-running overwrites the in-tree files. Manual hand-review pass for the special cases (Neo Geo cart's bespoke zip handling, MAME's listxml integration, ScummVM + DOSBox engine-launcher shapes).

#### Mass migration

Run the tool, hand-review the 38 emitted folders. For each system:

- Move `docs/cores/<id>/system-info.yaml` + `games-info.md` into the new `config/systems/<id>/` (or have the tool do this — its `system.yaml` output already merges the system-info-yaml content).
- Cross-check that the tool's emitted `bios.yaml` matches the hardcoded const (canonical filenames + SHA-1s must round-trip exactly).

#### Remove the old hardcoded const tables

After all 41 systems migrated:

- Delete `PSX_BIOS_KNOWN_HASHES` and the other 18 BIOS const tables from `main.rs` (~700 LOC gone).
- Delete `libretro_dat_refs_for_system`'s 45-arm match in `rom_hashes.rs` (~250 LOC). Becomes a one-liner pulling from the registry.
- Delete the entries of `CATALOG` in `core_installer.rs` (~600 LOC for the static array). The Rust const stays as a tiny stub (for the engine-default fallback) OR is removed entirely.
- Delete `light_gun_systems::LIGHT_GUN_SYSTEMS` (~150 LOC). Same pattern.
- Delete the `DEVICE_ID_OPTIONS_*` static arrays in main.rs (~80 LOC).

Net code reduction: **~1,800 LOC removed**, replaced by ~80 LOC of loader + accessor methods.

#### Remove the "fall back to hardcoded const" merge logic from Slice 1

Since every system is now in the registry, the consumer shims can become direct lookups:

```rust
fn libretro_dat_refs_for_system(registry: &SystemRegistry, system_id: &str) -> &[DatRef] {
    registry.get(system_id)
        .map(|s| s.descriptor.libretro_dat_refs.as_slice())
        .unwrap_or(&[])
}
```

#### `docs/cores/<id>/` cleanup

For each of the 38 remaining systems, move the existing `system-info.yaml` + `games-info.md` into `config/systems/<id>/`. The `docs/cores/<id>/` folder now contains only human-narrative files. `docs/INDEX.md` + `docs/cores/SCHEMA.md` updated to reflect the new location.

### Tests

- `cargo test` ≥ 625 green (no regressions from deleting the const tables since their consumers all read the registry now).
- New tests asserting every system has at least `system.yaml` present in `config/systems/<id>/`.

### Verification

1. Every operator-facing path that read a per-system const before now reads via the registry. No regressions in: BIOS resolution detail per system, bulk-core-install modal's recommended-cores list, smart-scan's libretro-database hash lookups, light-gun detection, per-game device-id dropdown options.
2. Build size shrinks slightly (less embedded const data).
3. Operator can edit `config/systems/saturn/bios.yaml` directly, restart OA, see the new BIOS known-hashes reflected in the readiness checklist without a recompile.

---

## Slice 3 — L3 + L4 layer wiring + schema validation tooling

### What changes

#### L3 — content-pack layer

Extend the loader to walk `<appDataDir>/content-packs/<pack-name>/systems/<id>/` after walking the in-tree `config/systems/<id>/`. Each pack contributes its own descriptor / bios / games files which get **deep-merged** per-field with the L2 in-tree version.

Merge semantics:

- For scalar fields (`display_name`, `default_core`): higher layer wins if present.
- For array fields (`cores[]`, `bios.files[]`): merge by primary key (`cores[].base`, `bios.files[].name`); higher layer overrides per entry; new entries added.
- For `games.games[]`: merge by primary key (title + serial); higher layer overrides.

The existing `manifest.yml` in each content pack (per `docs/PLANS/content-packs.md`) gets a new optional `systems_overrides` type that signals "this pack contains per-system YAML overrides at `systems/`". The pack loader registers them with the SystemRegistry.

#### L4 — operator overrides via SQLite

Existing SQLite tables (`system_info_overrides`, `game_info_overrides`) already hold L3-shape data per the System Info Panel + Game Info Panel work. Extend the registry's merge step to apply those overrides on top of L3 packs. Persistence + UI unchanged — operator edits via the same panels that exist today.

#### JSON Schema generation

Use the `schemars` crate (or hand-write) to generate `docs/schema/system-descriptor.schema.json` + `docs/schema/bios-descriptor.schema.json` + `docs/schema/games-descriptor.schema.json` from the Rust types. These ship in the repo for contributors / content-pack authors to validate their YAMLs against externally (using `ajv-cli`, JSON Schema Store, or similar).

#### CI lint

GitHub Action that runs `cargo test descriptor_validate_all_in_tree` on PR. Already a test from Slice 1; CI just runs it. Fails the PR if any `config/systems/<id>/*.yaml` is malformed.

### Tests

- New `registry::merge_layers_test` — given an in-tree L2 + a fake L3 pack + fake L4 overrides, asserts the merged output is correct per-field.
- ≥ 635 oa-shell tests.

### Verification

1. Drop a hand-rolled content pack at `<appDataDir>/content-packs/bios-extras/systems/saturn/bios.yaml` adding a new SHA-1 to the Saturn known-hashes. Restart OA. Readiness checklist's BIOS pill now recognizes the new hash as canonical for the operator's file.
2. Edit a system in System Info Panel UI → the L4 SQLite override gets written; restart OA; readiness checklist reflects the L4 override above the L3 pack above the L2 in-tree default.
3. Run `ajv validate -s docs/schema/system-descriptor.schema.json -d config/systems/psx/system.yaml` from the command line; succeeds for valid YAML, fails clearly for malformed.

---

## Critical files

| Path | Change |
| --- | --- |
| `apps/oa-shell/src/system_descriptor.rs` | NEW — serde-derived structs for the 3 file shapes |
| `apps/oa-shell/src/system_registry.rs` | NEW — runtime loader + lookup surface; Slice 3 adds layer-merge logic |
| `apps/oa-shell/src/main.rs` | Slice 1: shim the 3 pilot systems' const consumers; Slice 2: delete the 18 BIOS const tables + DEVICE_ID arrays |
| `apps/oa-shell/src/core_installer.rs` | Slice 1: merge in-tree CATALOG with registry; Slice 2: delete most of CATALOG |
| `apps/oa-shell/src/rom_hashes.rs` | Slice 1: shim `libretro_dat_refs_for_system`; Slice 2: replace the 45-arm match |
| `apps/oa-shell/src/light_gun_systems.rs` | Slice 2: delete the LIGHT_GUN_SYSTEMS table |
| `apps/oa-shell/src/game_info.rs` + `system_info.rs` | Slice 1: change load paths for pilots; Slice 2: change for all systems |
| `apps/oa-shell/src/bin/migrate_systems.rs` | NEW (Slice 2) — dev binary to bulk-migrate the 38 remaining systems |
| `config/systems/<id>/system.yaml` + `bios.yaml` + `games.yaml` | NEW for 3 pilots in Slice 1, 38 more in Slice 2 |
| `docs/cores/<id>/system-info.yaml` + `games-info.md` | DELETED — content moved to `config/systems/<id>/` |
| `docs/schema/*.schema.json` | NEW (Slice 3) — JSON Schema for external validators |
| `.github/workflows/*.yml` (or wherever CI is) | NEW (Slice 3) — descriptor-validate job |
| `docs/CHATGPT_BRIEFING.md` | Update file-map section to point at `config/systems/<id>/` instead of scattered Rust + docs |
| `docs/INDEX.md` + `docs/cores/SCHEMA.md` | Update routing references |

## Reused utilities

- `serde` + `serde_yaml` — already in `Cargo.toml` for the existing System Info Panel + Game Info Panel parsers. Same patterns extend.
- `schemars` crate — possibly new addition for Slice 3 JSON Schema generation; small additional dep.
- Existing `BiosFile` + `BiosFileStatus` types from Slice 5 — the `BiosFileEntry` in `bios.yaml` deserializes into a `BiosFile` for free (same field names + types).
- Existing `GameInfo` shape from `game_info.rs` — `GameInfoEntry` in `games.yaml` matches it exactly; the existing parser just changes load path.
- Existing `CatalogEntry` shape from `core_installer.rs` — `CoreEntryDescriptor` in `system.yaml` is a near-1:1 port.

## Out of scope (for this arc)

- **Frontend `systemThemes` registry consolidation.** Stays as TS const in `frontend/src/themes/registry.ts` until the kiosk-mode theme work warrants pulling it. The 3-file YAML schema reserves a `theme:` block for the future migration so we don't have to rev the schema later.
- **Theme CSS variables consolidation.** Same reasoning.
- **L1 engine-default fallback removal.** Some hardcoded fallbacks stay in code (e.g., a system with no YAML still gets minimal bindings via `bindings.rs` default arms). Removing those entirely is a follow-up after we trust the YAML pipeline.
- **Plugin / extension API.** PARKING_LOT entry written 2026-06-01; not on this arc.
- **Community curation layer.** PARKING_LOT entry; not on this arc.
- **Decision-trace / "Why is this setting X?" UI.** Separate plan after this lands. The data model this arc builds is the foundation; the UI is the next slice.
- **KNOWN_GAME_BUGS.md migration to games.yaml.** Separate arc per the existing docs/NEXT.md entry; aligned with this work since games.yaml is the destination, but not blocking this arc.

## Risks

- **Schema turning out wrong after Slice 1.** Mitigation = pilot-first approach is designed for this. If we discover Slice 1 missed a shape (e.g., GBA's optional-BIOS semantics needs a new flag), we rev the schema before Slice 2 — much cheaper than re-migrating 41 systems.
- **`include_dir!` for shipped builds.** The runtime-load model needs the YAML files to ship with the binary somehow. Options: (a) `include_dir!` crate that embeds them at compile-time; (b) ship as a sibling `config/` folder next to `oa-shell.exe`; (c) `xtask`-style build script that copies them into the Tauri bundle. Decision during Slice 1 implementation — `include_dir!` is the most likely pick (single binary, no install-time folder management).
- **Runtime parse errors at app start.** Worst case is OA refuses to launch because a YAML is malformed. Mitigation: tests catch this before any commit lands; CI catches it on PR. Operator can never ship a broken YAML — the build fails first. For operator-installed content packs (Slice 3): malformed packs get logged + skipped individually with a warning toast; don't take down the whole registry load.
- **The migration tool (Slice 2) emitting subtly-wrong YAML.** Mitigation: roundtrip-test the tool — feed it the current const tables, emit YAML, parse the YAML back, assert it produces the same hashmap. Anything that doesn't round-trip is a tool bug; fix before sweep.
- **Massive PR for Slice 2.** Migrating 38 systems is a +5000-LOC YAML PR + -1800-LOC Rust PR. Operator playtest pass is hard to scope. Mitigation: do Slice 2 in two batches — half the systems first, the other half a week later. Or write a per-system review checklist the operator walks system-by-system.

## When this arc starts

This plan is approved + queued (2026-06-01) but deferred. The executing session should:

1. **Re-read this plan in full** — design choices were operator-decided in the planning session; don't re-litigate locked decisions without a fresh AskUserQuestion check-in.
2. **Confirm the operator still wants Slice 1 specifically** before branching. Phase 1B closure was the natural pause point; the operator may have new priorities by the time this picks up.
3. **Branch as `feat/per-system-descriptors-slice-1`** per the standard branch workflow (see [feedback-branch-workflow] memory).
4. **Plan SLICE 1 phase commits** in the new session — design + loader + 3 pilots is multi-commit work; phase boundaries are roughly: `system_descriptor.rs + system_registry.rs scaffolding` → `GB pilot + tests` → `PSX pilot + BiosDescriptor wiring + tests` → `NDS pilot + multi-file AllRequired semantics + tests` → `docs/cores cleanup + verification`.
5. **Re-validate the loader-vs-include_dir decision** during Slice 1 — that's the one design call still open. Options listed in the Risks section above.

---

*Plan written 2026-06-01 after Phase 1B closure + ChatGPT gap-spotting session. Off-tree planning context lives at `C:\Users\Devilchi\.claude\plans\spicy-shimmying-crescent.md` for the session that wrote this; in-tree this doc is the source of truth.*
