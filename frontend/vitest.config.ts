import { defineConfig } from "vitest/config";
import { fileURLToPath } from "node:url";
import solid from "vite-plugin-solid";

// Vitest config for the frontend's first unit-test surface, stood up in
// Theming Substrate ARC 1 Phase 3 S4 to give the theme contract a CI gate
// (scope-call #7). It reuses the app's Solid transform + the `@oa/platform`
// alias so a test can import real theme packages (which pull in Solid
// components) exactly as the app resolves them. jsdom gives those component
// modules a DOM at import time — the tests never RENDER, they validate the
// declarative manifest/token surface, but the import graph (RetroverseShell &
// co.) expects a browser-ish global.
export default defineConfig({
  plugins: [solid()],
  resolve: {
    alias: {
      // Mirror vite.config.ts — prefix alias covers `@oa/platform/*` subpaths.
      "@oa/platform": fileURLToPath(new URL("./src/platform", import.meta.url)),
    },
  },
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
