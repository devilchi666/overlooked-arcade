// Motion diagnostics toggle (Theming ARC 3 Thrust M; D58.9).
//
// The motion players (`SpecTransition` / `AmbientMotion` / the M1 `ViewTransition`)
// each emit a per-play `[oa-theme-motion]` console line — invaluable while
// diagnosing whether a play fired, what spec it used, and whether reduced-motion
// kicked in (it diagnosed the reduced-motion question on 2026-06-18). But it's log
// spam during normal use, so the stream is GATED behind this runtime flag rather
// than always-on (and rather than stripped — the code stays until final ship, per
// D58.9: a toggle beats always-on-then-strip; flip it as needed until the motion
// work is perfected).
//
// SHAPE: a localStorage-backed boolean signal (default OFF), mirroring the
// frontend-owned, per-install pref pattern of `themeSettings.ts` — survives the
// restart-based theme swap. Lives in `platform/` so BOTH the players (platform)
// AND the engine Settings toggle (engine → platform is an allowed import) share
// one source of truth. The Settings UI flips it live; the players read it lazily
// at log time (not reactively — a `console.log` gate needs no reactivity), so the
// toggle takes effect on the next play with no restart.

import { createSignal } from "solid-js";

const STORAGE_KEY = "oa.motionDebug";

function load(): boolean {
  if (typeof localStorage === "undefined") return false;
  try {
    return localStorage.getItem(STORAGE_KEY) === "1";
  } catch {
    return false;
  }
}

const [motionDebug, setMotionDebugSignal] = createSignal(load());

/// True when the `[oa-theme-motion]` diagnostic stream is enabled. Reactive (so
/// the Settings toggle repaints), but the players read it via `motionDebugEnabled()`
/// at log time — a non-reactive read is fine for a logging gate.
export { motionDebug };

/// Non-reactive read for the players' log gate. Cheap; avoids creating a tracking
/// dependency from inside an unrelated `createEffect`.
export function motionDebugEnabled(): boolean {
  return motionDebug();
}

/// Flip the diagnostics on/off + persist. Called by the Settings → Experimental →
/// Dev tools toggle.
export function setMotionDebug(on: boolean): void {
  setMotionDebugSignal(on);
  if (typeof localStorage === "undefined") return;
  try {
    if (on) localStorage.setItem(STORAGE_KEY, "1");
    else localStorage.removeItem(STORAGE_KEY);
  } catch {
    // Quota / storage disabled — accept the in-memory-only fallback.
  }
}
