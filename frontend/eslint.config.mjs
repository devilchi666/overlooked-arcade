// ESLint flat config — ARCHITECTURAL BOUNDARY LINTER ONLY.
//
// This is deliberately NOT a style/quality linter (TypeScript's `tsc` +
// review cover that). Its single job is to enforce the platform / engine /
// theme layer boundary documented in
// docs/features/theming-substrate/SURFACES.md §"Layer boundary contract",
// so new features/fixes can't silently re-couple the layers.
//
// Enforced zones (all green) — dependencies point DOWN (theme → engine →
// platform); the platform FOUNDATION never depends on anything above it:
//   - platform/** ↛ routes/**     (platform must not import theme)
//   - platform/** ↛ engine/**     (platform must not import engine)
//   - platform/** ↛ components/** (platform must not import the grab-bag)
//   - engine/**   ↛ routes/**     (engine must not import theme)
//   - engine/**   ↛ components/** (engine must not import the grab-bag)
//
// PLUS the API boundary (Phase 4, the typed Tauri bridge — Slice 6 closer,
// 2026-06-10):
//   - raw invoke() banned outside platform/api/  (no-restricted-imports)
// Every component/engine/theme module reaches the backend ONLY through a typed
// wrapper in platform/api/<domain>Api; the command-name string lives in exactly
// one place. The src/platform/api/** override at the bottom re-allows raw
// invoke() there (it IS the bridge).
//
// The src/components/ grab-bag is now fully DRAINED (theming-grabbag-drain,
// 2026-06-09): every file relocated to engine/ (manager surfaces) or
// platform/components/ (shared per-game / in-game UI), and src/components/
// removed. The two components/** zones above are ratchets that keep anything
// from landing back in an unclassified grab-bag.

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
            {
              target: "./src/platform",
              from: "./src/components",
              message:
                "Boundary: platform/ must not import the unclassified components/ " +
                "grab-bag. If a component is platform UI, move it into " +
                "platform/components/ (Slice 2 grab-bag drain).",
            },
            {
              target: "./src/engine",
              from: "./src/routes",
              message:
                "Boundary: engine/ must not import theme code (routes/). The engine " +
                "surface owns its content — relocate it into engine/ and read stores " +
                "via usePlatform(), not useTheme().",
            },
            {
              target: "./src/engine",
              from: "./src/components",
              message:
                "Boundary: engine/ must not import the unclassified components/ " +
                "grab-bag. Engine-manager surfaces live in engine/; shared per-game " +
                "UI lives in platform/components/. The grab-bag is drained — nothing " +
                "should land back in src/components/.",
            },
            {
              target: "./src/routes",
              from: "./src/components",
              message:
                "Boundary: theme (routes/) must not import the unclassified " +
                "components/ grab-bag. The grab-bag is drained to zero — shared UI " +
                "lives in platform/components/, engine surfaces in engine/. Don't " +
                "re-create an unclassified src/components/ bucket.",
            },
          ],
        },
      ],
      // API boundary (Phase 4): raw invoke() is corralled into platform/api/.
      // Everything else imports a typed wrapper from @oa/platform/api/<domain>Api
      // instead. The src/platform/api/** override below re-allows it there.
      "no-restricted-imports": [
        "error",
        {
          paths: [
            {
              name: "@tauri-apps/api/core",
              importNames: ["invoke"],
              message:
                "Raw invoke() is corralled into platform/api/. Import a typed " +
                "wrapper from @oa/platform/api/<domain>Api instead, or add one " +
                "there. (convertFileSrc from the same module is still allowed.)",
            },
          ],
        },
      ],
    },
  },
  {
    // The typed Tauri bridge IS the one place raw invoke() lives.
    files: ["src/platform/api/**/*.{ts,tsx}"],
    rules: { "no-restricted-imports": "off" },
  },
];
