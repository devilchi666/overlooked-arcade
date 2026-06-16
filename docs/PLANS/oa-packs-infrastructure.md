# oa-packs — content-pack distribution infrastructure (arc plan)

**Status:** Planned 2026-06-15. No code yet. Slice 1 queued in
[NEXT.md](../NEXT.md) HIGH band.

**Owner-of-decisions:** the operator. This document records what was
decided in the 2026-06-15 planning discussion + the slice roadmap.

**Parent design:** [content-packs.md](content-packs.md) — the locked
2026-05-28 design sketch (registry shape, sha256 trust anchor,
operator-initiated-only, privacy panel, pack zip/manifest structure,
layered loading, install/update/uninstall flows). **That doc is the
"what"; this doc is the "how + in what order," plus the decisions the
2026-06-15 discussion added/changed (CP1–CP5 below).**

**Decisions:** [features/content-packs/DECISIONS.md](../features/content-packs/DECISIONS.md) (CP1–CP5).

---

## Goal in one line

Build the single, operator-initiated mechanism for distributing optional
content packs to OA installs — **schema + verify + install first, hosting
later** — so every future pack-shaped stream (emulator recipes, editorial
DISCOVER content, themes, per-system asset bundles, cheats, metadata)
rides one foundation instead of N bespoke updaters.

## Why now / why this shape

The External Emulator Depth arc's Slice 2 wants to "refresh emulator
recipes without an OA rebuild" (ED2). A one-off recipe updater would be
throwaway. The recipe updater is just **one consumer** of a general
distribution channel that [content-packs.md](content-packs.md) already
designed for editorial/themes/assets. Build the channel; recipes become a
pack `type`.

---

## Decisions added/changed in the 2026-06-15 discussion (CP1–CP5)

Full text in [features/content-packs/DECISIONS.md](../features/content-packs/DECISIONS.md).
Summary:

- **CP1 — Hosting is deferred; the registry URL is config, not a
  compile-time constant.** Slice 1 needs zero hosting knowledge (pure
  local-zip verify/install). When the fetch slice lands, the registry URL
  is a runtime config value so OA can point at any host (GitHub org, own
  domain, CDN, self-host) without a code change. The `overlooked-arcade`
  GitHub org doesn't need to exist until the first pack is published. The
  contract is the JSON **shape**, not the URL.
- **CP2 — The early lock-in risk is the schemas + on-disk layout, not
  hosting.** Get the **registry JSON schema**, the **`manifest.yml`
  schema**, and the **on-disk layout** (`<exe_dir>/<type>/community/<pack_id>/`)
  right early, because changing them later churns every already-published
  pack. Everything else (hosting, signing, federation, the pack roster)
  is deferrable and the schema already reserves seams (`depends_on`,
  `min_oa_version`, a future `source` field).
- **CP3 — Pack `type` is additive data + a dispatch arm; never a schema
  break.** New pack kinds (recipes, themes, cheats, metadata) slot in one
  at a time as we go — the anti-lock-in mechanism, mirroring how emulator
  `accepts_archives` was added as a declarative field. This is the native
  mode, not an exception.
- **CP4 — "Has a bundled baseline" is a per-pack-type property, not a
  global rule.** content-packs.md §7 says "no `builtin/` tier — OA ships
  with zero pack content." That's correct for **editorial** (DISCOVER is
  empty until you install). It is **wrong for emulator recipes**, which
  ship bundled in the install (`config/emulators/*.yaml` — BizHawk
  launches today with no pack download) and treat pack updates as an
  **override** of a working baseline. So baseline-vs-empty is decided
  per type. Mirrors theming **D44** ("keep the default bundled;
  externalization is additive"). The core loader must not bake in the
  editorial-only "zero builtin" assumption.
- **CP5 — Emulator recipes become a pack `type` (`emulator-recipes`);
  External Emulator Depth Slice 2 rides this infra, not a standalone
  updater.** The recipe override tier loads on top of the bundled
  `config/emulators/` baseline (CP4). This is the first real non-editorial
  consumer and the proof that the type-dispatch model (CP3) holds.

---

## New crate

Per [content-packs.md](content-packs.md) §12: a new **`crates/oa-packs/`**
crate (workspace prefix `oa-`). Pure, dependency-light core (serde,
`sha2`, `zip` — all either present or tiny); **no Tauri, no network in the
pure core.** Network + Tauri glue lives in the shell and reuses the
existing `core_installer.rs` download/extract/progress patterns +
`http_retry.rs`. This keeps the verify/validate/install logic
unit-testable with zero I/O beyond a temp dir.

### Reuse audit (2026-06-15)

The downloader half already exists and must be reused, not rebuilt:

- `apps/oa-shell/src/core_installer.rs` — `zip::ZipArchive` extraction,
  download-with-progress events (`oa://core-download-progress` pattern),
  cancel handling, `.partial` staging, atomic install into a target dir.
- `apps/oa-shell/src/http_retry.rs` — `reqwest` (rustls) wrapper with
  transient/permanent (5xx/404/4xx) classification + one retry.
- `zip` + `reqwest` are already workspace deps.

**Genuinely missing** (what this arc adds): sha256 verification (`sha2`),
registry fetch+parse, manifest validation, staged install into the
per-type `community/` layout + rollback retention, the privacy panel /
allow-network toggle / network log, and pack-`type` dispatch.

---

## Slices

### Slice 1 — `oa-packs` crate: pure verify + validate + local-zip install `[QUEUED]`

**No network, no Tauri. Fully unit-testable.** Matches content-packs.md
§12 step 1.

- Scaffold `crates/oa-packs/` + add to the workspace.
- **Registry + manifest serde types** — `Registry`, `PackEntry`
  (id/type/name/version/url/sha256/size_bytes/depends_on/min_oa_version/
  license/homepage), `Manifest`. `type` is a string/open enum (CP3).
  These two types ARE the contract (CP2) — review carefully.
- `verify(zip_bytes, expected_sha256) -> Result<()>` via `sha2`;
  mismatch rejects (content-packs.md §5).
- `validate_manifest_against_registry(manifest, entry)` — id/version/
  type/name must match; `min_oa_version` gate. Refuse mismatch.
- `install_from_local_zip(zip_path, entry, dest_root)` — stage to a temp
  dir → verify sha256 → unzip → validate manifest → atomic move into
  `<dest_root>/<type>/community/<pack_id>/`. No download.
- **Per-type baseline awareness (CP4):** the install/load layout must
  support a bundled baseline tier for types that have one (recipes) and
  none for types that don't (editorial). At minimum, don't hard-code
  "no builtin"; model it as a per-type flag/param.
- Unit tests: good-hash installs, bad-hash rejects, manifest mismatch
  rejects, atomic-move leaves no partial on failure, version compare.
- `cargo test -p oa-packs` green.

**Demoable acceptance:** install a hand-built local pack zip from a path;
a tampered zip (wrong sha256) is rejected; a manifest that disagrees with
its registry entry is rejected.

### Slice 2 — registry fetch + download + Tauri commands (network behind the gate)

- Registry fetch from a **config-supplied URL** (CP1) — not a constant.
- Download (reuse `core_installer` + `http_retry` patterns) → hand bytes
  to Slice-1 `verify`/`install`.
- Tauri commands: `oa_packs_list`, `oa_packs_fetch_registry`,
  `oa_packs_install`, `oa_packs_update`, `oa_packs_uninstall`.
- **Allow-network gate:** every network command returns
  `Err(NetworkDisabled)` synchronously when the master toggle is OFF
  (content-packs.md §9), before any call.

### Slice 3 — Settings → Content → Packs panel + full lifecycle

- Installed / Available / Updates sections (content-packs.md §9).
- End-to-end install / update / uninstall + 14-day rollback retention
  (§8). Conflict warning surface (§7).

### Slice 4 — Privacy panel + allow-network toggle + network log

- Settings → Privacy: disclose every URL OA hits + when; master toggle
  (default ON, one click OFF); per-call network-log ring buffer (§9).

### Slice 5 — first consumers (one at a time, CP3/CP5)

- **`emulator-recipes` pack type** — override tier on the bundled
  `config/emulators/` baseline (CP4/CP5). Closes External Emulator Depth
  Slice 2.
- **`editorial` pack type** — OA Editorial Baseline → DISCOVER (the
  content-packs.md §10 original first pack). Zero-builtin (CP4).
- Themes / per-system asset bundles / cheats / metadata accrete later,
  each an additive type + dispatch arm (CP3).

---

## Explicitly deferred / out of scope (un-lock-in posture)

- **Hosting / the actual registry URL + GitHub org** — later (CP1).
- **Cryptographic signing** (minisign/sigstore) — deferred until community
  packs are common (content-packs.md §11). sha256-from-registry is the v1
  trust anchor.
- **Federation** (operator-added custom registries) — deferred; schema
  reserves a future `source` field (content-packs.md §4).
- **Cores** — explicitly NOT a pack type; libretro buildbot +
  `core_installer.rs` already handle that ecosystem (content-packs.md §2).
- **ROMs / BIOS / keys** — never, ever.

## Dependencies / cross-refs

- **External Emulator Depth Slice 2** (recipe updates) becomes Slice 5's
  `emulator-recipes` consumer (CP5) — see
  [external-emulator-depth.md](external-emulator-depth.md).
- **DISCOVER** ([discover-tab-retroverse, archived]) is the editorial
  consumer.
- **Theming `.oatheme` loader** + per-system asset bundles are future
  pack types; cross-ref theming D44 (CP4).
- **per-system-descriptors.md** L3 ("content packs") overlaps — reconcile
  when that slice and Slice 5 meet.

## Verification approach

- Slice 1: `cargo test -p oa-packs` green — pure, no I/O beyond temp dirs.
- Slice 2+: `cargo test -p oa-shell` green + operator smoke of the
  visible Settings surface before merge.
- One branch per arc/phase per the operator's branch workflow; merge to
  main at playtestable milestones.

## Open questions deferred to execution time

- **Slice 1 baseline modeling (CP4)** — per-type flag on the pack-type
  descriptor vs a param on the loader. Decide at execution start; keep it
  small.
- **`min_oa_version` source** — where the running OA version comes from
  (Cargo pkg version vs a dedicated constant).
- **Slice 5 ordering** — recipes-first (closes a live arc) vs editorial-
  first (the doc's original first pack). Lean recipes-first.
