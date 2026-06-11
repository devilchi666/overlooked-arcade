import { render } from "solid-js/web";
import App from "./App";
import "./index.css";
import { installConsoleLogBridge } from "@oa/platform/lib/logbridge";
import { ensureSystemPaletteBaseline } from "@oa/platform/themes/systemPalettes";

// Install the console.* → Rust log bridge before any other module
// runs. Existing `console.log("[oa-…] …")` call sites automatically
// route into the unified Rust log stream from here on; the bridge
// also captures uncaught errors + unhandled promise rejections.
installConsoleLogBridge();

// Inject the per-system `[data-system]` accent baseline (S5.2 — the typed
// SYSTEM_PALETTES map, replacing the static ./themes/systems.css @import) into
// document.head BEFORE first render, so per-system accents are present before
// any component mounts (no flash).
ensureSystemPaletteBaseline();

const root = document.getElementById("root");
if (!root) throw new Error("#root missing from index.html");

render(() => <App />, root);
