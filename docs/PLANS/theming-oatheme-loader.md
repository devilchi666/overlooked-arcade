# `.oatheme` runtime loader — declarative-first (Theming ARC 2 "P")

**Status:** **P.1 (declarative runtime themes) S1–S3 SHIPPED 2026-06-16** on
branch `theme-oatheme-loader-slice-1` (pre-merge, operator playtesting).
Direction **A (declarative-first)** locked by the operator 2026-06-16; decisions
formalized as D45–D49 in
[features/theming-substrate/DECISIONS.md](../features/theming-substrate/DECISIONS.md).
P.2 (runtime custom-JS themes) remains DEFERRED.

**Owner-of-decisions:** the operator.

**Parent context:** the last open thread of **Theming ARC 2 — Per-System
Layout Substrate** ([theming-arc-2-per-system-layout.md](theming-arc-2-per-system-layout.md),
slice "P"), which is itself the last open thread of **ARC 1** (the original
§6 Phase 5 `.oatheme` loader sketch in
[theming-substrate.md](theming-substrate.md) §"Phase 5"). Decisions land in
[features/theming-substrate/DECISIONS.md](../features/theming-substrate/DECISIONS.md)
(D44+ at execution). Rides the content-pack channel shipped 2026-06-16
([features/content-packs/](../features/content-packs/), CP1–CP9).

---

## Goal in one line

Let OA discover + load + switch themes that live **on disk** (not baked into
the Vite bundle) — and distribute them through the content-pack channel —
**without** running author-supplied JavaScript at runtime.

## The fork, and why A

A theme's `entry` is a Solid component (real JS). Loading author JS at
runtime in the Tauri WebView is the hard, deferred part of ARC 1, for three
reasons:

1. **Shared singletons** — a runtime theme can't bundle its own Solid /
   `@oa/platform` (two Solid instances = broken reactivity); it needs an
   import-map / externalized-deps mechanism to reuse the host's singletons.
2. **Origin / CSP** — dynamic `import()` of JS served off the asset protocol
   from `tauri://localhost` is blocked without CSP/import-map work (the "CSP
   allowlist ARC 1 deferred"). Note `tauri.conf.json` currently sets
   `csp: null` — but the module-loader origin rules still bite.
3. **Trust** — it is arbitrary code execution from a downloaded pack;
   sha256-from-registry only proves the bytes match the registry listing.

ARC 2's own scope is **"fully declarative, no scripting."** So the
consistent reading is: **runtime `.oatheme` themes are declarative — data
only, no code.** A built-in generic shell renders them. Custom-code shells
(Retroverse-class) stay **build-time built-ins**; the scripted escape hatch
is **ARC 3 (Rhai)**. This is option **A**, locked 2026-06-16.

**Honest ceiling:** a declarative theme is a *single-surface browse shell* —
bare / CoverFlow-class (pick a layout primitive per view/system, palette,
background, sounds, glyph set, settings). It **cannot** express Retroverse's
multi-tab / detail-panel structure. That high ceiling stays compiled-in /
ARC 3. Documenting the floor honestly is the point (low floor, high ceiling).

---

## Decisions (locked 2026-06-16 — formalize as D44–D47 at execution)

- **PD1 — Runtime `.oatheme` themes are declarative-only in ARC 2 (P.1);
  custom-JS loading is deferred (P.2).** Sidesteps the shared-singleton +
  CSP + arbitrary-code-execution problems and matches ARC 2's "no scripting"
  scope + the declarative-first philosophy.
- **PD2 — On-disk theme manifest is `theme.toml`; themes live at
  `<exe_dir>/themes/community/<id>/`** — the same `<type>/community/<id>`
  shape the pack channel installs to (CP2), with `themes` as the pack
  `type` string (CP3). A loose-folder dev path is reserved (hot-reload is a
  nice-to-have, not required for P.1).
- **PD3 — One built-in `DeclarativeShell` renders every declarative theme**
  by interpreting the manifest (`views` → layout primitive via the ARC 2
  `resolveLayout`/`useResolvedLayout` machinery) + tokens + `ThemeBackground`
  + glyph set + `settings_schema`. Declarative themes ship **zero code**.
- **PD4 — Themes distribute as the `themes` pack type** on the oa-packs
  channel (CP3/CP5 pattern). **No bundled baseline** (built-ins are
  compiled in; community disk themes are additive — like editorial, CP4).
  The pack's `manifest.yml` (oa-packs identity layer) and the theme's
  `theme.toml` (theme-definition layer) coexist in the pack zip; the pack
  installer reads the former, the theme loader the latter.

---

## On-disk format

```
<exe_dir>/themes/community/<theme_id>/
├── manifest.yml        # oa-packs pack identity (type: themes) — for the channel
├── theme.toml          # the theme definition (mirrors ThemeManifest fields)
├── tokens.toml         # optional — ThemeTokens overrides (palette/typography/geometry)
├── per-system.toml     # optional — perSystemTokens + per-system views
└── system-ui/          # optional — backgrounds / sounds / boot anim, the S5.1 cascade
    ├── _baseline/backgrounds/default.png
    └── <system>/backgrounds/default.png
```

`theme.toml` declares the same surface `validateTheme()` already checks:
`id`/`name`/`version`/`schema_version`/`oa_version`/`default_route`/`routes`/
`surfaces`/`glyph_set`/`per_system_ui`/`views`/`settings_schema`. The
declarative theme has **no `entry`/`entry_export`** — those become
implicit (`DeclarativeShell`); the loader supplies them.

---

## Slices — P.1 (declarative runtime themes)

### P.1 S1 — on-disk format + Rust loader + discovery command `[SHIPPED 2026-06-16 — branch theme-oatheme-loader-slice-1]`

- Define the `theme.toml` (+ optional `tokens.toml`/`per-system.toml`)
  schema as serde structs mirroring the TS `ThemeManifest`/`ThemeTokens`
  declarative surface.
- `apps/oa-shell/src/theme_loader.rs`: discover `<exe_dir>/themes/community/<id>/`
  (+ reserve a loose-folder dev path), parse the TOML, return
  `Vec<DiskThemeDescriptor>` (manifest fields + tokens + per-system + the
  theme's absolute base path for asset resolution). Skip-on-malformed +
  logged, never fatal (mirrors `emulator_profiles`/`oa-packs`). Tauri
  command `oa_themes_list_disk`. No frontend consumer yet.
- Unit tests: parse good/bad TOML, skip malformed, base-path resolution.
- **Acceptance:** a hand-placed `<exe_dir>/themes/community/foo/theme.toml`
  shows up in `oa_themes_list_disk` output; a malformed one is skipped.

### P.1 S2 — `DeclarativeShell` generic theme component `[SHIPPED 2026-06-16 — branch theme-oatheme-loader-slice-1]`

- A built-in shell (`themes/declarative/` or
  `platform/theme/declarativeShell.tsx`) — a Solid `ThemeEntry` that renders
  a single browse surface from a manifest: resolves the per-view/per-system
  layout via `useResolvedLayout`, mounts the matching primitive
  (List/Grid/Carousel/Wheel — all already built), paints `ThemeBackground`,
  honors the glyph set + `settings_schema`. Tokens are already injected by
  App's `.oa-theme-mount` wrapper, so the shell needs no token code.
- A `diskThemeToPackage(desc)` mapper turning a `DiskThemeDescriptor` into a
  `ThemePackage { manifest, entry: DeclarativeShell, tokens, perSystemTokens }`.
- **Dogfood:** re-express `bare` (or a new `bare-declarative` fixture)
  purely as `theme.toml` + tokens, rendered by `DeclarativeShell` — proving
  a zero-code theme works end to end.
- **Acceptance:** the dogfood theme is visually equivalent to hand-coded
  `bare`; vitest covers the manifest→primitive mapping.

### P.1 S3 — register disk themes + Appearance picker + the `themes` pack type `[SHIPPED 2026-06-16 — branch theme-oatheme-loader-slice-1; P.1 COMPLETE]`

- App merges `oa_themes_list_disk` results (via `diskThemeToPackage`) into
  the registry alongside `BUILTIN_THEMES`; `validateTheme` runs on disk
  themes (invalid excluded, same as builtins). Appearance picker lists disk
  themes; selecting one persists `active_theme_id` + restarts (ARC 1's
  swap-by-restart model — unchanged).
- Add `themes` to `oa-packs` `default_pack_type_specs` (`has_bundled_baseline:
  false`). A `themes` pack installs to `<exe_dir>/themes/community/<id>/` via
  the existing pack pipeline → the loader discovers it → it appears in the
  picker. Full install/update/rollback for free on the channel.
- **Acceptance:** install a hand-built `themes` pack zip from the Packs panel
  (or hand-place a folder) → restart → it's selectable in Appearance and
  renders; uninstall → it's gone. **No network required** (hand-placed path).

## P.2 — runtime custom-JS themes `[DEFERRED]`

Out of scope for this arc. When demand justifies it: dynamic `import()` of a
theme's pre-built `dist/` via an import-map that externalizes Solid +
`@oa/platform` to the host singletons, plus the CSP `script-src` allowlist
for the asset origin. The CSP work then becomes load-bearing for ARC 3's
Rhai sandboxing. Until then, custom-code shells are build-time built-ins.

---

## Reuse audit (don't rebuild)

- **Pack channel** (`crates/oa-packs/` + `apps/oa-shell/src/packs*.rs`,
  shipped 2026-06-16) — `themes` is just a new pack `type`; install /
  verify / rollback / network gate / Privacy log all already work.
- **Layout primitives + resolver** (ARC 2 L3a/L4a/L4b: `resolveLayout`,
  `useResolvedLayout`, `LibraryView`'s primitive Switch, `WheelNav`/
  `CarouselNav`/`ListNav`/`GridNav`) — the shell composes these, builds none.
- **Tokens + per-system palettes + `ThemeBackground` + glyph sets +
  `settings_schema`** (S3/S5.1/S5.2/S5.3/S5.4) — all consumed as-is.
- **`validateTheme()`** (pure, data-only) — validates disk themes unchanged.
- **Theme registry + active-theme swap-by-restart** (S2/S4) — disk themes
  register through the same path.
- **`emulator_profiles`/`oa-packs` loader patterns** — discovery,
  skip-on-malformed, `<exe_dir>` resolution mirror those.

## Verification approach

- P.1 S1: `cargo test -p oa-shell` green (TOML parse + discovery).
- P.1 S2–S3: frontend `tsc` + eslint + vitest green; operator smoke — a
  hand-placed declarative theme renders + switches; a `themes` pack installs
  via the Packs panel.
- One branch per arc/phase; merge at a playtestable milestone.

## Open questions — RESOLVED at execution

- **TOML vs JSON for `theme.toml`** — ✅ **TOML** (D46). `serde`+`toml` is a
  workspace dep; manifests were sketched as TOML in §6 Phase 5; the format is an
  internal detail behind the loader.
- **Hot-reload (loose-folder dev mode)** — ✅ path **reserved**, not wired
  (`resolve_themes_dev_dir` → `<exe_dir>/themes/dev/`, scanned at startup, no
  file-watch). Swap-by-restart is the shipped model; hot-reload stays deferred
  until it earns its keep (D46).
- **`views`/`settings_schema` vocabulary the `DeclarativeShell` needs** — ✅
  settled minimal (D49): the shell resolves `views["game-browse"].layout` via the
  ARC 2 L3 resolver, and interprets one recognized setting (`compactRows` → list
  density). Other declared controls render + persist but are inert in the generic
  shell; the vocabulary accretes additively (CP3-style).
