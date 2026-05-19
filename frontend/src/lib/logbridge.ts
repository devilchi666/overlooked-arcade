// Frontend → Rust log bridge.
//
// Wraps `console.log/info/warn/error/debug` so every browser-side log
// also lands in the Rust `log` crate's unified stream — same ring,
// same file, same `Help → Debug log…` view. Existing `console.*` call
// sites in the codebase don't change; they just start showing up in
// the Rust log too.
//
// Each wrapper still forwards to the original `console.*` so the
// WebView DevTools console stays normal — bridge is additive, not a
// replacement. Failures to invoke the Tauri command swallow silently
// (no log spam when the Tauri runtime isn't ready yet during early
// SolidJS hydration).

import { invoke } from "@tauri-apps/api/core";

type Level = "error" | "warn" | "info" | "debug" | "trace";

const ORIGINALS = {
  log: console.log.bind(console),
  info: console.info.bind(console),
  warn: console.warn.bind(console),
  error: console.error.bind(console),
  debug: console.debug.bind(console),
};

/// Convert each console arg to a string we can ship over IPC. Objects
/// JSON-stringify; everything else uses String(). Cyclic objects fall
/// back to their default toString.
function formatArg(a: unknown): string {
  if (a === null) return "null";
  if (a === undefined) return "undefined";
  const t = typeof a;
  if (t === "string") return a as string;
  if (t === "number" || t === "boolean" || t === "bigint") return String(a);
  if (a instanceof Error) {
    return a.stack ? a.stack : `${a.name}: ${a.message}`;
  }
  try {
    return JSON.stringify(a);
  } catch {
    return String(a);
  }
}

/// Best-effort parser for the `[oa-…]` target tag many call sites use
/// (e.g. `console.log("[oa-launch] handleLaunch called", entry)`). The
/// prefix becomes the Rust log `target`; the rest is the message.
function splitTarget(message: string): { target: string; body: string } {
  const m = message.match(/^\[([^\]]+)\]\s*(.*)$/s);
  if (!m) return { target: "", body: message };
  return { target: m[1], body: m[2] };
}

function ship(level: Level, args: unknown[]): void {
  const joined = args.map(formatArg).join(" ");
  const { target, body } = splitTarget(joined);
  // Fire-and-forget. If Tauri isn't ready yet (very early frontend
  // boot) the catch swallows the InvalidArgs error; the originals
  // already printed to DevTools, so nothing is lost.
  void invoke("log_from_frontend", {
    level,
    target,
    message: body,
  }).catch(() => {});
}

let installed = false;

/// Install the bridge. Call once during app boot — multiple calls are
/// no-ops. After install, `console.*` calls also go to Rust.
export function installConsoleLogBridge(): void {
  if (installed) return;
  installed = true;

  console.log = (...args: unknown[]) => {
    ORIGINALS.log(...args);
    ship("info", args);
  };
  console.info = (...args: unknown[]) => {
    ORIGINALS.info(...args);
    ship("info", args);
  };
  console.warn = (...args: unknown[]) => {
    ORIGINALS.warn(...args);
    ship("warn", args);
  };
  console.error = (...args: unknown[]) => {
    ORIGINALS.error(...args);
    ship("error", args);
  };
  console.debug = (...args: unknown[]) => {
    ORIGINALS.debug(...args);
    ship("debug", args);
  };

  // window.onerror catches uncaught exceptions; bridge them too so the
  // unified log shows synchronous crashes alongside ordinary logs.
  window.addEventListener("error", (e) => {
    ship("error", [
      `uncaught: ${e.message}`,
      e.filename ? `at ${e.filename}:${e.lineno}:${e.colno}` : "",
    ]);
  });
  window.addEventListener("unhandledrejection", (e) => {
    const reason = e.reason;
    ship("error", [
      "unhandled promise rejection:",
      reason instanceof Error ? (reason.stack ?? reason.message) : formatArg(reason),
    ]);
  });
}
