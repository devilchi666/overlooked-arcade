import { defineConfig } from "vite";
import { fileURLToPath } from "node:url";
import solid from "vite-plugin-solid";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [solid(), tailwindcss()],
  clearScreen: false,
  resolve: {
    alias: {
      // Theming Substrate ARC 1 Phase 2 — explicit boundary marker
      // for code Phase 2+ migrates out of routes/themes/components and
      // into `src/platform/`. Reading: themes (later: Phase 6 onwards
      // when Retroverse becomes one of N themes) import only from
      // @oa/platform; engine code is free to import from either side.
      // ESLint boundary enforcement lands in Phase 4 alongside the
      // Tauri-bridge hardening (plan §6 Phase 4).
      "@oa/platform": fileURLToPath(new URL("./src/platform", import.meta.url)),
    },
  },
  server: {
    port: 5173,
    strictPort: true,
    host: "127.0.0.1",
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "esnext",
    sourcemap: true,
  },
});
