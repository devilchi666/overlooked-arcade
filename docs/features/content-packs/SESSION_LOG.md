# Content packs (oa-packs) — session log

Most recent entry first. Three lines each: **Shipped / Almost / Next.**

---

## 2026-06-16 — Slice 3: Settings → Content → Packs panel + lifecycle + rollback retention

- **Shipped (on `oa-packs-slice-2`, bundling Slices 2+3 as one playtestable
  unit):** The operator-facing pack manager.
  **Rust** — rollback retention in `packs.rs`: uninstall + update/install-over
  now MOVE the prior version into `<data_dir>/packs-rollback/<id>-<version>/`
  (retain, not hard-delete) after the replacement bytes are in hand; 14-day
  GC; 3 new commands `oa_packs_list_rollbacks` / `oa_packs_rollback`
  (reversible swap) / `oa_packs_discard_rollback`, all registered. **Frontend**
  — `platform/api/packsApi.ts` (typed wrappers for all 11 commands, no raw
  invoke at call sites) + `engine/PacksSettings.tsx` panel wired into
  `SettingsPanel.tsx` as a new CONTENT category "Packs" (📦): Registry &
  network card (editable config registry URL + Save/Reset per CP1, allow-
  network `SettingRow` toggle, operator-initiated Browse button, Last
  checked), Installed / Available / Updates / Recoverable-versions sections,
  per-action busy state, `confirm()` on destructive actions, `pushToast` on
  every outcome. Registry is fetched ONLY on Browse — never on mount
  (content-packs.md §3). `cargo test -p oa-shell packs` = 9 green (added
  retain→rollback test), clippy clean for the new files; frontend `tsc
  --noEmit` + eslint clean.
- **Fix (same day, playtest):** uninstall did nothing — the confirm dialog
  closed but the pack stayed. Cause: rollback retention was under
  `<data_dir>` (AppData, often `C:`) while packs live under `<exe_dir>`
  (often a different drive), and Windows `rename` can't cross volumes, so
  the retain step errored. Moved retention to `<exe_dir>/.packs-rollback/`
  (same volume → atomic move) + added a `move_dir` copy-fallback. Recorded
  as **CP7**. The rollback commands now thread `PacksRoot`, not `AppDataDir`.
- **Almost:** Progress events / a download progress bar (deferred — installs
  show a busy spinner, not a byte bar; graft `core_installer`'s
  `oa://…-progress` pattern when wanted). Conflict-warning surface (§7) not
  built — our manifest schema doesn't model content-level ids yet, so cross-
  pack id collisions can't be detected until a consumer (Slice 5) defines
  content ids. The Privacy panel proper is Slice 4 (the toggle currently
  lives in this panel).
- **Next:** Operator playtest. Offline-testable now: drop a folder at
  `<exe_dir>/<type>/community/<id>/manifest.yml` → it shows under Installed →
  Uninstall (→ Recoverable versions) → Restore. Full online flow testable by
  pointing the registry URL at any reachable `registry.json` (CP1) — note the
  `min_oa_version` gate is `0.0.1` today (CP6), so test packs should omit it.
  Then merge Slices 2+3 to main, and Slice 4 (Privacy panel + network log).

---

## 2026-06-16 — Slice 2: registry fetch + download + Tauri commands (network behind the gate)

- **Shipped:** Shell-side network + Tauri glue around the pure `oa-packs`
  crate (branch `oa-packs-slice-2`; Slice 1 merged to main first).
  `apps/oa-shell/src/packs_prefs.rs` — `PacksPrefs` at
  `appDataDir/packs/prefs.json` (`registry_url` seeded with the §4 default
  but config-overridable per CP1; `allow_network` default ON; `last_checked`),
  mirroring `library_prefs`. `apps/oa-shell/src/packs.rs` — `PacksRoot`
  (`<exe_dir>`) managed at startup; 8 Tauri commands wired into
  `generate_handler`: `oa_packs_get_prefs` / `set_registry_url` /
  `set_allow_network` (local prefs), `oa_packs_list` / `oa_packs_uninstall`
  (local fs scan of `<exe_dir>/*/community/*/manifest.yml`, type-agnostic per
  CP3), `oa_packs_fetch_registry` / `oa_packs_install` / `oa_packs_update`
  (network). Reuses `http_retry::get_*_with_retry` — did NOT rebuild a
  downloader. Network gate: `ensure_network_allowed` returns a synchronous
  `NETWORK_DISABLED:` sentinel before any request when the toggle is OFF
  (§9). Backend-authoritative trust chain: install/update fetch the registry
  + look up the entry by id themselves, never trusting a frontend sha256 →
  `oa_packs::verify` → `install_from_local_zip` into
  `<exe_dir>/<type>/community/<id>/` (CP2). `min_oa_version` gate sourced
  from `CARGO_PKG_VERSION`. Decisions recorded as **CP6**. oa-shell compiles
  clean (`cargo clean -p oa-shell` + check), 8 new unit tests green
  (gate on/off, prefs round-trip/migrate/malformed, scan finds-packs/ignores
  non-pack dirs), clippy clean for the new files.
- **Almost:** No operator-facing surface yet — there's nothing to playtest
  in-app until the Settings → Packs panel (Slice 3) renders these commands.
  End-to-end install was not exercised against a live registry (no hosting
  yet, CP1); the trust/install path itself is covered by Slice 1's crate
  tests + this slice's gate/scan tests.
- **Next:** Slice 3 — Settings → Content → Packs panel (Installed / Available
  / Updates) + `lib/packs.ts` service wrapping these commands + 14-day
  rollback retention + conflict warnings (content-packs.md §8–§9). Then
  Slice 4 (Privacy panel + network log) and Slice 5 (first consumers:
  `emulator-recipes`, then `editorial`).

---

## 2026-06-15 — Slice 1: `oa-packs` crate (pure verify + validate + local-zip install)

- **Shipped:** New `crates/oa-packs/` crate — pure, no network, no Tauri
  (branch `oa-packs-slice-1`). Contract types: `Registry` + `PackEntry`
  (`registry.json`) and `Manifest` (`manifest.yml`), additive-friendly —
  optional fields default, unknown fields ignored (forward-compatible),
  `type` an open `String` not a closed enum (CP3). `verify(zip_bytes,
  expected_sha256)` (sha2, lowercase-hex, case-insensitive on expected) —
  mismatch rejects (§5 trust anchor). `validate_manifest_against_registry`
  — id/version/type/name must match + `min_oa_version` gate (running OA
  threaded in, never a constant; stricter of registry/manifest bound wins).
  `install_from_local_zip` = read+verify → stage under `<dest_root>/.staging/`
  (same-volume rename) → extract (zip-slip-guarded via `enclosed_name`) →
  parse+validate manifest → atomic `rename` into
  `<dest_root>/<type>/community/<pack_id>/`; a `StagingDir` RAII guard
  guarantees no partial dir at the destination on any failure. CP4 baseline
  modelled per-type, not global: `PackTypeSpec` + `default_pack_type_specs`
  (`emulator-recipes`=baseline, `editorial`=none, unknown→none) +
  `baseline_for_type`. 14 tests green (`cargo test -p oa-packs`), clippy
  clean: good-hash install into the per-type layout, wrong-hash reject,
  tampered-zip reject, manifest-disagreement reject, min-oa-version gate,
  missing-manifest reject, atomic update-replace, no-staging-leak on every
  failure path, numeric version compare, registry/manifest wire-format parse.
  All I/O inside temp dirs; no network anywhere.
- **Deviations from the plan/sketch (intentional, documented here):** deps
  are `serde, serde_yaml, sha2, zip, thiserror, log` (+ `serde_json`
  dev-only) — `serde_yaml` is required to parse `manifest.yml` (the plan's
  "serde/sha2/zip" list omitted the YAML reader; oa-shell already pins the
  same 0.9 line), `thiserror` gives a typed `PackError` so tests assert
  *why* an install was refused. Added `name` as a **required identity
  field** on `Manifest` — content-packs.md §6's example omits it but §6's
  prose validates it; we own the schema (CP2) so the manifest now
  self-declares its display name. `sha2` + `serde_yaml` promoted to
  workspace deps.
- **Almost:** n/a — Slice 1 scope complete.
- **Next:** Slice 2 — registry fetch from a **config-supplied** URL (CP1,
  never a constant) + download (reuse `core_installer.rs` / `http_retry.rs`,
  do NOT rebuild a downloader) handing verified bytes to Slice-1
  `verify`/`install`, behind the allow-network gate (every network command
  returns `NetworkDisabled` synchronously when OFF). Merge `oa-packs-slice-1`
  to main at a playtestable milestone per the branch workflow.

---

## 2026-06-15 — Arc planned (planning discussion, no code)

- **Shipped:** The oa-packs infrastructure arc plan
  ([PLANS/oa-packs-infrastructure.md](../../PLANS/oa-packs-infrastructure.md))
  on top of the locked 2026-05-28 design ([PLANS/content-packs.md](../../PLANS/content-packs.md)).
  5 decisions (CP1–CP5): hosting is deferred + registry URL is config not a
  constant (CP1); the schemas + on-disk layout are the early lock-in, not
  hosting (CP2); pack `type` is additive data + a dispatch arm (CP3);
  bundled-baseline is per-type — recipes ship a baseline + override tier,
  editorial is empty-until-installed (CP4); emulator recipes become the
  first non-editorial pack type and the External Emulator Depth arc's
  recipe-update slice rides this infra (CP5). Reuse audit done: the
  download/extract/progress half already exists in
  `apps/oa-shell/src/core_installer.rs` + `http_retry.rs` (+ `zip`/`reqwest`
  deps); what's genuinely missing is sha256 verify, registry/manifest
  parse+validate, per-type staged install, the privacy panel, and type
  dispatch.
- **Almost:** n/a (planning only).
- **Next:** Slice 1 — scaffold `crates/oa-packs/`: pure sha256 verify +
  manifest-vs-registry validation + install-from-local-zip (no network, no
  Tauri), with unit tests. Queued in NEXT.md HIGH band. Plan → docs →
  /clear; execution is a fresh self-contained session.
