// Built-in theme registry (the concrete theme list).
//
// Theming Substrate ARC 1 Phase 3 S2. This is the ONE place that knows the
// concrete set of bundled themes. It lives in themes/ (theme-layer) — NOT in
// platform/ — because platform must never import a theme (platform ↛ theme).
// App.tsx imports this list and injects it into the platform-owned registry
// via `registerThemes(BUILTIN_THEMES)` at boot, so platform owns the switch
// machinery while staying ignorant of which themes exist (the same
// inversion-avoiding pattern as platform/libraryAdmin + engineSurface, D13).
//
// ORDER MATTERS: the first entry is the default theme (used when no
// active-theme preference is stored). Retroverse stays first — it's the
// dogfood default.
//
// Build-time bundled only in ARC 1 (DECISIONS D6) — no .oatheme zips / runtime
// dynamic import. Adding a theme = add its package object here.

import type { ThemePackage } from "@oa/platform/theme/types";
import { retroverse } from "./retroverse";
import { coverflow } from "./coverflow";
import { bare } from "./bare";
import { bareDeclarative } from "./declarative-bare";

// Order: first = default (Retroverse). `bare` is the minimal valid reference
// theme added in S4 — operator-selectable (the lowest-floor shell, browse +
// launch + engine access, no tokens) and the validator's canonical fixture.
// `bareDeclarative` (P.1 S2 dogfood) re-expresses `bare` as pure data rendered
// by the built-in DeclarativeShell — the zero-code proof, beside `bare` for A/B.
export const BUILTIN_THEMES: ThemePackage[] = [retroverse, coverflow, bare, bareDeclarative];
