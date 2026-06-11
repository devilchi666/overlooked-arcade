// Retroverse-UI Phase A Slice 4 — top-level route signal.
//
// The Retroverse design uses 5 top-toolbar tabs as peers (HOME / LIBRARY
// / COLLECTIONS / PLAY NOW / DISCOVER). `SidebarView`
// (kind: "all" | "view-node") models LIBRARY's *internal* filter
// state, not the top-level tab. currentRoute is layered above
// SidebarView so the two stay decoupled. Phase B's RetroverseShell + top-tab
// strip are the first surface that read this; Phase A wires the signal
// without UI consumers.
//
// Default is "library" — matches today's landing surface so toggling
// the experimental flag ON for the first time doesn't disorient the
// operator. Once the per-tab pages all ship, the default may move to
// "home" per docs/PLANS/retroverse-ui-rollout.md §9 open questions.
//
// No persistence in Phase A — the signal lives only for the current
// session. Phase B will add localStorage persistence once a real tab
// strip exists (early-return otherwise would persist a route the
// operator can't visually verify).
//
// Theming Substrate ARC 1 Phase 1 (2026-06-06): dropped the "settings"
// route. SETTINGS moved out of Retroverse's tab list into the engine-
// owned EngineManagerSurface, summoned via F12 / Select+Start / corner
// icon. See docs/features/theming-substrate/SURFACES.md.

import { createSignal, type Accessor } from "solid-js";

export type RetroverseRoute =
  | "home"
  | "library"
  | "collections"
  | "play-now"
  | "discover";

/// Declaration order = visual order in the top-tab strip. Phase B's
/// cycleRouteForward / cycleRouteBackward use this for L1/R1 navigation
/// between tabs.
export const RETROVERSE_ROUTES: readonly RetroverseRoute[] = [
  "home",
  "library",
  "collections",
  "play-now",
  "discover",
];

const [routeSig, setRouteSig] = createSignal<RetroverseRoute>("library");

/// Reactive accessor — subscribe to route changes in Solid components.
export const currentRoute: Accessor<RetroverseRoute> = routeSig;

/// Direct setter — used by the top-tab strip click handlers (Phase B)
/// and the Phase A debug DevTools helpers (`__retroverse_debug`).
export function setCurrentRoute(route: RetroverseRoute): void {
  setRouteSig(route);
}

/// Advance to the next route in declaration order, wrapping. Returns
/// the new route. Used by Phase A debug helpers + Phase B's R1 shoulder
/// bumper on the top-tab strip.
export function cycleRouteForward(): RetroverseRoute {
  const current = routeSig();
  const idx = RETROVERSE_ROUTES.indexOf(current);
  const next = RETROVERSE_ROUTES[(idx + 1) % RETROVERSE_ROUTES.length] ?? "library";
  setRouteSig(next);
  return next;
}

/// Step back one route in declaration order, wrapping. Mirror of
/// cycleRouteForward — used by Phase B's L1 shoulder bumper.
export function cycleRouteBackward(): RetroverseRoute {
  const current = routeSig();
  const idx = RETROVERSE_ROUTES.indexOf(current);
  const len = RETROVERSE_ROUTES.length;
  const next = RETROVERSE_ROUTES[(idx - 1 + len) % len] ?? "library";
  setRouteSig(next);
  return next;
}
