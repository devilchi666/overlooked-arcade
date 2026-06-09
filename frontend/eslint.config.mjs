// ESLint flat config — ARCHITECTURAL BOUNDARY LINTER ONLY.
//
// This is deliberately NOT a style/quality linter (TypeScript's `tsc` +
// review cover that). Its single job is to enforce the platform / engine /
// theme layer boundary documented in
// docs/features/theming-substrate/SURFACES.md §"Layer boundary contract",
// so new features/fixes can't silently re-couple the layers.
//
// Slice 1 (theming-boundary-enforcement) enforces the two invariants that
// hold today after one fix:
//   - platform/** must not import theme code (routes/**)
//   - platform/** must not import the engine layer (engine/**)
// i.e. the PLATFORM FOUNDATION never depends on anything above it.
//
// KNOWN-violating edges deferred to Slice 2 (the components/ grab-bag drain)
// are tracked in SURFACES.md and intentionally NOT yet enforced here, so
// `npm run lint` stays green:
//   - engine/** → routes/** (SettingsPanel pulls Settings content from
//     theme files; fixed by relocating that content into engine/)
//   - platform/** → components/** (SystemCoresStrip; fixed by classifying
//     the grab-bag)
//   - raw invoke() outside platform/api/ (Phase 4 — the typed Tauri bridge)
// Each becomes a new zone / rule here as its batch lands.

import tseslint from "typescript-eslint";
import importPlugin from "eslint-plugin-import";

export default [
  {
    ignores: ["dist/**", "node_modules/**", "src-tauri/**", "*.config.*"],
  },
  {
    files: ["src/**/*.{ts,tsx}"],
    // This config runs no style rules, so existing `// eslint-disable`
    // comments targeting style rules (e.g. no-unused-vars) are inert here —
    // don't flag them as "unused". They stay meaningful for a future full
    // lint pass.
    linterOptions: { reportUnusedDisableDirectives: "off" },
    languageOptions: {
      parser: tseslint.parser,
      parserOptions: {
        ecmaFeatures: { jsx: true },
        sourceType: "module",
      },
    },
    // `@typescript-eslint` is registered (not enabled) so pre-existing
    // `// eslint-disable @typescript-eslint/...` comments in the codebase
    // resolve to a known rule instead of erroring. We don't turn its rules
    // on — this config lints boundaries only, not style.
    plugins: { "@typescript-eslint": tseslint.plugin, import: importPlugin },
    settings: {
      "import/resolver": {
        typescript: { project: "./tsconfig.json" },
      },
    },
    rules: {
      "import/no-restricted-paths": [
        "error",
        {
          zones: [
            {
              target: "./src/platform",
              from: "./src/routes",
              message:
                "Boundary: platform/ must not import theme code (routes/). " +
                "Platform is the foundation — pass what the component needs in " +
                "via props/stores; the theme (routes/) supplies the handler.",
            },
            {
              target: "./src/platform",
              from: "./src/engine",
              message:
                "Boundary: platform/ must not import the engine layer (engine/). " +
                "Engine sits above platform and imports it — never the reverse.",
            },
          ],
        },
      ],
    },
  },
];
