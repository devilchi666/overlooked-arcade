import { render } from "solid-js/web";
import App from "./App";
import "./index.css";
import { installConsoleLogBridge } from "./lib/logbridge";

// Install the console.* → Rust log bridge before any other module
// runs. Existing `console.log("[oa-…] …")` call sites automatically
// route into the unified Rust log stream from here on; the bridge
// also captures uncaught errors + unhandled promise rejections.
installConsoleLogBridge();

const root = document.getElementById("root");
if (!root) throw new Error("#root missing from index.html");

render(() => <App />, root);
