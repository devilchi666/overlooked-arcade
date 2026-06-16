# Content packs (oa-packs) — feature

The single, operator-initiated mechanism for distributing **optional**
content packs to OA installs: emulator recipes, editorial DISCOVER
content, themes, per-system asset bundles, cheats, metadata enrichment.
One foundation, many pack types — never N bespoke updaters.

**Not covered:** cores (libretro buildbot + `core_installer.rs` handle
those), ROMs, BIOS, keys. Ever.

## Where this lives

- **Design (the "what"):** [PLANS/content-packs.md](../../PLANS/content-packs.md)
  — the locked 2026-05-28 sketch: registry shape, sha256 trust anchor,
  operator-initiated-only, privacy panel, pack zip/manifest structure,
  layered loading, install/update/uninstall flows.
- **Arc plan (the "how + order"):** [PLANS/oa-packs-infrastructure.md](../../PLANS/oa-packs-infrastructure.md)
  — slices + the CP1–CP5 decisions from the 2026-06-15 discussion.
- **Decisions:** [DECISIONS.md](DECISIONS.md) (CP1–CP5).
- **Code (when it lands):** `crates/oa-packs/` (pure verify/validate/
  install core) + Tauri/network glue in `apps/oa-shell/` reusing
  `core_installer.rs` + `http_retry.rs`.

## Load-bearing principles (CP1–CP5)

1. **Hosting is later; the registry URL is config, not a constant (CP1).**
   Slice 1 is fully offline.
2. **The schemas + on-disk layout are the early lock-in (CP2)** — get the
   registry JSON, `manifest.yml`, and `<exe_dir>/<type>/community/<pack_id>/`
   layout right; defer the rest.
3. **Pack `type` is additive data + a dispatch arm (CP3)** — add kinds one
   at a time, never a schema break.
4. **Bundled-baseline is per-type (CP4)** — recipes ship a working
   baseline + override tier; editorial is empty-until-installed.
5. **Recipes are the first non-editorial consumer (CP5)** — the External
   Emulator Depth arc's recipe-update slice rides this infra.

## Status

- **Slice 1 ⬜ queued** — `oa-packs` crate: pure sha256 verify + manifest
  validation + install-from-local-zip, with tests. No network/Tauri.
  Queued in [NEXT.md](../../NEXT.md) HIGH band.
- Slices 2–5: registry fetch + Tauri commands → Settings panel + lifecycle
  → privacy panel → first consumers (recipes, then editorial). See the arc
  plan.
