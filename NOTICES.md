# Third-Party Notices

This project is GPLv2 (see `LICENSE`). It vendors third-party C source code in `crates/oa-<sys>-sys/vendor/`. Each vendored upstream is enumerated here with its origin and license.

Populated as cores are vendored. Empty for now.

---

## Vendored cores

| Crate | Upstream | License | Vendored from (commit / date) | Local modifications |
|-------|----------|---------|-------------------------------|---------------------|
| (none yet) | | | | |

---

## Rust dependencies

Tracked in `Cargo.lock` after Phase 1 scaffolding lands. License audit via `cargo-deny` or `cargo-license` runs in CI before any binary release.

## Frontend dependencies

Tracked in `frontend/package.json` and `frontend/pnpm-lock.yaml` after Phase 2 scaffolding lands.
