# Content packs (oa-packs) — session log

Most recent entry first. Three lines each: **Shipped / Almost / Next.**

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
