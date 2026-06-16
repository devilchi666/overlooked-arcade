# Content packs (oa-packs) — decisions

Append-only. Decision ids are **CP**-prefixed. These extend/refine the
locked 2026-05-28 design in [PLANS/content-packs.md](../../PLANS/content-packs.md)
with what the 2026-06-15 planning discussion added or changed.

---

## CP1 — Hosting is deferred; the registry URL is config, not a constant (2026-06-15)

**Decision:** OA does not need to decide where packs are hosted now.
Slice 1 (verify + manifest validation + install-from-local-zip) needs
**zero hosting knowledge** — it runs fully offline. When the fetch slice
lands, the registry URL is a **runtime config value**, never a
compile-time constant, so OA can point at any host (a GitHub org, OA's
own domain, a CDN, a self-host) without a code change. The
`overlooked-arcade` GitHub org in content-packs.md §4 is a placeholder
and need not exist until the first pack is published.

**Why (operator):** "do we need to know where things are hosted now or is
that a later thing? I dont want to lock Myself into anything." Hosting is
the most deferrable decision; the contract is the JSON **shape**, not the
URL. Treating the URL as data keeps every distribution choice reversible.

**How to apply:** Build Slice 1 with no network. Store the registry URL in
OA prefs (with the content-packs.md §4 default as a seed, overridable).
Never hard-code it into `oa-packs`.

---

## CP2 — The early lock-in risk is the schemas + on-disk layout, not hosting (2026-06-15)

**Decision:** The three things to get right early — because changing them
later churns every already-published pack — are the **registry JSON
schema**, the **`manifest.yml` schema**, and the **on-disk layout**
(`<exe_dir>/<type>/community/<pack_id>/`). Everything else (hosting,
signing, federation, the pack roster) is deferrable; the schema already
reserves seams (`depends_on`, `min_oa_version`, a future `source` field).

**Why:** Schemas are the contract between OA and every pack author.
Hosting/signing/federation are swappable behind that contract. Spend the
design care where reversal is expensive.

**How to apply:** Review the serde types in Slice 1 as the load-bearing
artifact. Keep them additive-friendly (optional fields, open `type`).

---

## CP3 — Pack `type` is additive data + a dispatch arm; never a schema break (2026-06-15)

**Decision:** New pack kinds (emulator-recipes, themes, cheats, metadata,
per-system assets) are added one at a time as a new `type` value + a
dispatch arm for install location and loading. Adding a type is additive
data, never a schema break.

**Why (operator):** "Id like to discuss the packs and how we are adding
these to this system as we go. I dont want to lock Myself into anything."
This is the anti-lock-in mechanism and the native mode of the system —
the same shape as adding the emulator `accepts_archives` field as
declarative data. Cross-ref [[ED2]] (recipes are updatable data) and the
theming "low floor / high ceiling, declarative-first" philosophy.

---

## CP4 — "Has a bundled baseline" is a per-pack-type property, not a global rule (2026-06-15)

**Decision:** content-packs.md §7's "no `builtin/` tier — OA ships with
zero pack content" is correct for **editorial** (DISCOVER is empty until a
pack is installed) but **wrong for emulator recipes**, which ship bundled
in the install (`config/emulators/*.yaml`) and treat pack updates as an
**override** of a working baseline. So baseline-vs-empty is decided **per
type**. The `oa-packs` core loader must NOT bake in the editorial-only
"zero builtin" assumption.

**Why:** OA must launch BizHawk out of the box with no pack download (the
recipes shipped in the External Emulator Depth arc), yet still let a
recipe update refresh a changed CLI flag. That's bundled-baseline +
override tier — a different shape from editorial's empty-until-installed.
Directly mirrors theming **D44** ("keep the default bundled;
externalization is additive, version-gated").

**How to apply:** Model baseline as a per-type flag/param in the loader.
Recipe loading = bundled `config/emulators/` → `community/<pack>/` override
(last wins). Editorial loading = `community/` only.

---

## CP5 — Emulator recipes become a pack type; the depth arc's Slice 2 rides this infra (2026-06-15)

**Decision:** The External Emulator Depth arc's "update recipes without an
OA rebuild" (ED2 / its Slice 2) is implemented as an `emulator-recipes`
pack **type** in this system, not a standalone updater. It is the first
real non-editorial consumer (oa-packs arc Slice 5) and the proof that the
type-dispatch model (CP3) + per-type baseline (CP4) hold.

**Why:** A one-off recipe updater would be throwaway; the recipe updater
is just one consumer of the general channel content-packs.md already
designed. Building the channel and making recipes a type avoids duplicate
download/verify/install machinery.

**Cross-ref:** [[external-emulator-depth]] Slice 2; this arc's Slice 5.

---

## CP6 — Slice 2 execution decisions: prefs location, min_oa_version source, network sentinel, dest_root (2026-06-16)

Resolves the "open questions deferred to execution time" in
[PLANS/oa-packs-infrastructure.md](../../PLANS/oa-packs-infrastructure.md)
plus the glue choices Slice 2 (network + Tauri commands) had to make.

- **Registry URL + allow-network live in `appDataDir/packs/prefs.json`**
  (`PacksPrefs` in `apps/oa-shell/src/packs_prefs.rs`), same file pattern
  as `library_prefs`. `registry_url` is **seeded** with the content-packs.md
  §4 URL via `DEFAULT_REGISTRY_URL` but the persisted value wins and is
  operator-overridable — honoring CP1 ("URL is config, never a constant").
  Setting it to empty resets to the seed. The `oa-packs` crate still has
  zero URL knowledge.
- **`min_oa_version` source = `env!("CARGO_PKG_VERSION")`** (the workspace
  version, `0.0.1` today), threaded into `oa_packs::install_from_local_zip`
  as data — *not* a constant inside the crate. **Consequence:** any pack
  with a `min_oa_version` above `0.0.1` is gated out until OA's version
  climbs; test/local packs should omit `min_oa_version` or set it `<= 0.0.1`.
  Chosen over a dedicated version constant because the Cargo version is the
  single source already bumped on release.
- **Network gate = a synchronous `Err("NETWORK_DISABLED: …")` sentinel
  string** returned by `ensure_network_allowed` at the top of every
  network command, before any request is built (content-packs.md §9).
  Commands return `Result<_, String>` per the codebase convention; the
  string is prefixed `NETWORK_DISABLED:` so the frontend (Slices 3–4) can
  match it. The toggle defaults ON.
- **dest_root = `<exe_dir>`** (`PacksRoot`, resolved like `resolve_cores_dir`
  minus the `/cores`). Installs land at `<exe_dir>/<type>/community/<id>/`
  — the literal `<type>` segment per **CP2**, which supersedes
  content-packs.md §7's older semantic naming (`discover/`, `themes/`).
  `oa_packs_list`/`uninstall` are type-agnostic: any top-level dir with a
  `community/` child is a pack-type root (CP3 — no hard-coded type set).
- **Trust chain is backend-authoritative:** `install`/`update` fetch the
  registry themselves and look up the entry by id; they never trust a
  frontend-supplied `sha256`. registry → `entry.sha256` →
  `oa_packs::verify` → `install_from_local_zip`.
- **Deferred to later slices (unchanged):** rollback retention + progress
  events + the Settings → Packs panel (Slice 3); the Privacy panel +
  network-log ring buffer (Slice 4). The allow-network *pref* + *gate* ship
  now (Slice 2) so the gate is testable; only its operator-facing UI waits.

---

## CP7 — Rollback retention lives under `<exe_dir>`, not `<data_dir>` (2026-06-16)

**Decision:** Rollback retention (content-packs.md §8) stores prior pack
versions under **`<exe_dir>/.packs-rollback/<id>-<version>/`** — the
install's own volume — overriding §8's stated `<data_dir>/packs-rollback/`
location.

**Why (caught in Slice 3 playtest):** Retention works by `rename`-ing the
pack directory out of `<exe_dir>/<type>/community/`. On Windows a `rename`
**cannot cross volumes**, and `<exe_dir>` routinely sits on a different
drive than `<data_dir>` (AppData) — the common case is the operator running
from `G:\…\target\release\` (or a portable install on `D:`) while AppData is
on `C:`. With retention under `<data_dir>`, every uninstall/update silently
failed the move (the command errored; the pack stayed put). Keeping
retention on the install's own volume makes the move atomic, exactly as
Slice 1's `.staging/` dir does for installs. The leading-dot name keeps
`.packs-rollback` from looking like a pack-type root to the installed-pack
scan (it has no `community/` child).

**How to apply:** `rollback_root(root)` joins `<root>/.packs-rollback` where
`root` is the `PacksRoot` (`<exe_dir>`). A `move_dir` helper tries `rename`
then falls back to recursive copy + remove, so even an unexpected
cross-volume move degrades gracefully instead of failing. The download
staging area (`<data_dir>/packs/.download/`) is unaffected — it's only ever
*read*, never renamed across volumes.

---

## CP8 — `emulator-recipes` override semantics: whole-profile-by-id, last-wins, applied at startup (2026-06-16)

**Decision:** The first pack consumer, `emulator-recipes` (closing External
Emulator Depth ED2/Slice 2), overlays the bundled `config/emulators/`
baseline with these rules:

- **Pack layout:** a recipe pack carries `emulators/<id>.yaml` files (mirrors
  the bundled dir) under `<exe_dir>/emulator-recipes/community/<pack_id>/`.
- **Whole-profile-by-id, last wins.** A pack's `<id>.yaml` *replaces* the
  baseline profile for that id entirely (no field-level deep-merge); a new
  `id` adds an emulator. This matches content-packs.md §7 "last wins" and
  ED2's "publish a new recipe = the new flag is live" — simple, predictable,
  and the unit of update is the whole recipe.
- **Deterministic ordering + conflict surface.** Packs apply in
  alphabetical pack-id order, so "last wins" is reproducible. Two packs
  touching the same emulator id is a [`RecipeConflict`] (winner + losers),
  logged and surfaced to the Packs panel. This is the first pack type with
  content-level ids, so it's where content-packs.md §7's conflict-warning
  surface gets its first real data.
- **Applied at startup, not hot-reloaded.** Profiles load once into
  `AppState.emulator_profiles`; a recipe pack install/update/uninstall takes
  effect on the **next launch**. The Packs panel says so. Hot-reload would
  need `AppState` behind a lock (a bigger change) and is deferred — the
  operator-initiated, restart-to-apply model is honest and sufficient for v1.

**Why:** Recipes are updatable *data* (ED2) — a changed CLI flag must not
force an OA rebuild. Routing recipe updates through the pack channel as a
`type` (CP3/CP5) with a baseline+override tier (CP4) delivers exactly that,
and proves the type-dispatch model holds for a non-editorial consumer.

**How to apply:** `EmulatorProfiles::load_default` loads the baseline then
calls `apply_recipe_overrides(<exe_dir>/emulator-recipes/community/)`.
`oa_packs_recipe_overrides` exposes the active overrides + conflicts to the
UI. New recipe-pack content is just data; no code change unless a genuinely
new launch *mechanism* is needed (the ED2 data/code boundary).

---

## CP9 — Recipe overrides hot-reload on pack change (supersedes CP8's restart-to-apply, 2026-06-16)

**Decision:** Reverses CP8's "applied at startup, restart to apply." A pack
install/update/uninstall/rollback now **hot-reloads** the recipe override
tier immediately — no app restart.

**Why (playtest):** With restart-to-apply, uninstalling the test recipe pack
moved it to rollback but left the "Emulator recipe overrides" panel + the
External Emulators name still showing `oa-test-recipes` — a stale override
referencing a pack that's no longer installed. That's confusing and reads as
a bug; a "restart to apply" note doesn't fix the staleness. Per the
no-band-aid principle, the proper fix is to make the snapshot reloadable.

**How:** `AppState.emulator_profiles` is now `RwLock<EmulatorProfiles>`. New
command `oa_packs_reload_recipes` re-runs `EmulatorProfiles::load_default`
(re-reading the bundled baseline + every installed `emulator-recipes` pack)
and swaps the snapshot under the write lock. The Packs panel calls it after
each pack mutation, then refetches the overrides view, so the panel +
External Emulators reflect the change at once. All read sites clone the
profile out of a short-lived read lock; reload only happens on operator-
initiated pack actions, never mid-launch, so lock contention is a non-issue.
Launches started *after* a reload use the new recipes (the in-process
launcher reads the snapshot at launch time).
