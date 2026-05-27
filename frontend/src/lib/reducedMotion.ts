// `prefers-reduced-motion: reduce` accessor.
//
// Per-System Custom UI (Stage 1) needs a reactive signal that listens
// to the OS-level "reduce motion" preference so boot animations + per-
// system tile flourishes + transition timings can collapse to short
// fades when the operator has accessibility motion-reduction enabled.
//
// Implemented as a module-level signal so multiple consumers share one
// MediaQueryList subscription instead of each setting up their own
// listener. Returns the current value synchronously on first call;
// updates reactively when the OS preference toggles mid-session.

import { createSignal, type Accessor } from "solid-js";

const QUERY = "(prefers-reduced-motion: reduce)";

const [reducedMotion, setReducedMotion] = createSignal(
  typeof window !== "undefined" && typeof window.matchMedia === "function"
    ? window.matchMedia(QUERY).matches
    : false,
);

if (typeof window !== "undefined" && typeof window.matchMedia === "function") {
  const mq = window.matchMedia(QUERY);
  // `change` fires whenever the OS preference flips; modern browsers
  // expose addEventListener, older Safari only supported the deprecated
  // addListener — we lean on the modern API since OA's WebView is
  // Tauri-managed (recent WebView2 / WKWebView).
  mq.addEventListener("change", (e) => setReducedMotion(e.matches));
}

/// Reactive accessor for the OS-level reduce-motion preference.
/// Consumers like the boot-animation framework + tile flourish system
/// short-circuit long-form animations when this is true.
export const prefersReducedMotion: Accessor<boolean> = reducedMotion;
