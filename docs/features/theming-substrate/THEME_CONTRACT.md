# Theme Contract

The theme-facing contract for the Overlooked Arcade theming substrate — the
peer of [SURFACES.md](SURFACES.md). SURFACES.md says *which surfaces* are
engine vs theme territory; this doc says *what a theme may declare and
override*, and what the substrate guarantees in return.

**Status:** ARC 1. Written in Phase 3 S3 (the token layer). The S4 load-time
validator (`platform/theme/validate.ts`) checks a theme against this document;
the `bare` theme (`themes/bare/`) is its canonical minimal-valid fixture.

A theme is a `ThemePackage` (`frontend/src/platform/theme/types.ts`):

```ts
type ThemePackage = {
  manifest: ThemeManifest;          // metadata (id / version / surfaces / capabilities)
  entry: Component<{ surface }>;    // the root component the shell mounts
  tokens?: Partial<ThemeTokens>;    // design-token overrides (S3)
};
```

A theme consumes **only** the platform layer — `usePlatform()` stores, the host
services (`useTheme()`), `@oa/platform/nav` (verbs + primitives), `@oa/platform/api`,
media, the per-system registry, and the platform-homed `EngineSummonIcon`. It
**never** imports `engine/`, `routes/`, or another theme (enforced by the
`themes/**` ESLint zones).

---

## 1. Manifest (`ThemeManifest`)

Authored as a typed object in ARC 1 (read from `theme.toml` in a later phase).
Fields (`platform/theme/manifest.ts`):

| field | meaning |
| --- | --- |
| `id` | stable, directory-safe, lowercase (e.g. `coverflow`) |
| `name` | display name in Settings → Themes (e.g. `CoverFlow`) |
| `version` | the theme's own semver |
| `schema_version` | manifest schema revision (currently `1`) |
| `oa_version` | semver range of the OA shell the theme supports |
| `entry` / `entry_export` | entry module path + export (informational in ARC 1) |
| `default_route` / `routes` | route ids the theme registers |
| `context_slots` | platform store slices the theme consumes |
| `required_engine_capabilities` | engine capabilities the theme refuses to run without (e.g. `multi-monitor`, `attract-mode`) — **empty in ARC 1** |
| `reserves_corner` | engine-summon-icon corner — `"top-right"` in ARC 1 |
| `surfaces` | named surfaces the theme renders — **`["main"]` only in ARC 1** (D20b) |

## 2. Entry component

`Component<{ surface: ThemeSurface }>`. Surface-aware from theme #1; ARC 1
honors exactly `"main"` (the primary single-monitor shell). The entry reads
stores via `usePlatform()`, host services via `useTheme()`, navigates via the
verb layer + nav primitives, and reaches the backend only via `@oa/platform/api`.

## 3. Navigation — the verb vocabulary (S1, DECISIONS D18)

Themes navigate by **semantic verbs**, never raw buttons. The shell-nav verb set:

`Confirm` · `Back` · `Secondary` · `Tertiary` · `Up`/`Down`/`Left`/`Right` ·
`PrevSection`/`NextSection` · `Menu` · `OpenQuickSettings` · (reserved:
`Search`/`Favorite`/`Page`).

Button→verb mapping is an **OA-wide per-user** config (`navBindings`,
`nav_bindings.json`) — a theme **restyles hints + picks layouts but never
redefines a verb's meaning** (that's a per-user contract). The HintBar renders
glyphs from the current input→verb map, so a remap repaints every hint for
free. Consume the `list`/`grid` primitives (`@oa/platform/nav`) or
`useFocusGroup` directly; both are verb-native.

## 4. Design tokens (S3) — what a theme MAY override

A theme declares `tokens?: Partial<ThemeTokens>` — a typed object the shell
injects as CSS custom properties **scoped to the theme-mount wrapper**. Omit
any key to inherit the `:root` default. Source of truth:
`platform/theme/tokens.ts`.

**Palette** — `bg` · `bgDeep` · `ink` · `inkDim` · `accent` · `accentSoft` ·
`accentGlow` · `focusRing`
**Typography** — `fontDisplay` (keep CJK fallbacks if you replace Inter)
**Geometry** — `tileRadius` · `gridGap` · `sectionSpacing`

Each maps 1:1 to a CSS var (`bg → --color-oa-bg`, etc.). Values are CSS value
strings (`oklch(...)`, a font stack, a length).

**Per-system colour still cascades.** `accent` is the *base/fallback*; elements
that carry a `data-system` attribute get the per-system accent from
`themes/systems.css` (higher specificity). A theme chooses how much per-system
identity to show (D19) — consume `data-system`, or don't.

### The contract RULE (D2 guarantee)

- A theme styles **only** via (a) its `tokens` object and (b) its own scoped
  classes. It **MUST NOT** write a global `:root` override or a `<style>` block
  that sets a token / engine variable. The substrate scopes theme tokens to the
  theme mount; the engine surface (Settings / Library Manager / Import / System
  Health) is a *sibling* of that mount and always reads the `:root` defaults, so
  a theme **cannot restyle engine territory**. Writing global token overrides
  would break that guarantee — the S4 validator rejects it.

### Reserved — motion (ARC 2, not yet a theme axis)

The `--motion-*` durations + `--ease-*` easings exist and are honored by the
a11y baseline, but **themes do not drive animation in ARC 1**. The cinematic /
motion axis — transitions, video backgrounds, attract mode, WGSL shader chrome
— is ARC 2-3 (the BigBox-style capability the substrate builds toward). When it
lands it will extend this contract with a `motion`/`transitions` token category;
it is intentionally absent from `ThemeTokens` today so the contract doesn't make
a promise the substrate can't yet keep.

## 5. Accessibility baseline (S3) — NOT overridable

Inherited by every theme, sitting *outside* the token contract:

- **`prefers-reduced-motion`** collapses the shared motion tokens + neutralizes
  transitions/animations app-wide (`index.css`). Motion-sensitive users get a
  still UI regardless of theme.
- **Focus-visible ring** — the nav primitives render a ring from `--oa-focus-ring`
  (defaults to the accent; theme-overridable via `tokens.focusRing`). Keyboard
  parity with the gamepad is required.
- The default palette is contrast-checked; a theme overriding palette tokens is
  responsible for its own contrast.

## 6. What the S4 validator checks (and what it can't)

`validateTheme(pkg)` (`platform/theme/validate.ts`) is a pure function over a
theme's **declarative surface** — its manifest + its typed `tokens` object. It
returns structured `errors` (disqualifying → the theme is excluded from the
picker and can't be the active shell) and `warnings` (non-fatal → loaded, but
surfaced). It runs at registration (dev-loud) and as a Vitest CI gate over
`BUILTIN_THEMES` + the `bare` fixture, so a built-in drifting from this contract
**fails the build**.

**Enforced now (data):**
- Manifest required fields present + non-empty; `schema_version` ∈
  `SUPPORTED_SCHEMA_VERSIONS` (`{1}` in ARC 1; the message distinguishes
  "targets a newer schema — update OA" from "unsupported schema").
- `surfaces` non-empty and ⊆ the surfaces the shell honors (`["main"]` in ARC 1).
- `required_engine_capabilities` ⊆ the engine's advertised capabilities
  (`ENGINE_CAPABILITIES` — **empty in ARC 1**, so only `[]` validates).
- `tokens` keys ∈ `ThemeTokens` (the `TOKEN_VAR` map); values non-empty strings.
  *This is the data half of the §4 no-override rule:* a theme can only set token
  keys that map to known, sibling-scoped CSS vars, so even a hostile token
  **value** can't escape the theme mount.
- Warnings: non-directory-safe `id`; `default_route` ∉ `routes`.

**Backed structurally (not by the validator):** the §4 "no global `:root` /
engine-variable override" guarantee. The real protection is the S3
**sibling-scope mount** (a theme's scoped tokens physically cannot reach engine
territory) + the ESLint **layer boundary** (a theme can't import `engine/`).

**Deferred (a known gap):** a theme that bypasses the token system entirely — a
`<style>:root{…}</style>` in its JSX, a `document.head.appendChild`, or an
imported global stylesheet — is **invisible** to a package-object validator (the
entry is an opaque component; the validator never inspects source text or
rendered output). Closing that needs either a fragile source-scanning lint or a
runtime DOM guard; both are heavier than S4 and a **Phase-5 / untrusted-author**
concern (on-disk `.oatheme`s). Built-in themes are reviewed, so the structural
guarantee + boundary lint are sufficient for ARC 1. (DECISIONS D24.)
