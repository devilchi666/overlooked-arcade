# Content packs (oa-packs) — session log

Most recent entry first. Three lines each: **Shipped / Almost / Next.**

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
