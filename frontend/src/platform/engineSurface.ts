// Platform — engine surface visibility signal.
//
// The "engine surface" is the fullscreen takeover that hosts Settings,
// Library Manager, Import Wizard, BIOS pre-checks, Core installer,
// System Health, and Background Jobs (per docs/features/theming-substrate/SURFACES.md).
// Themes don't render it; the engine always does. Themes summon it via
// the EngineSummonIcon platform component or the F12 hotkey / Select+Start
// controller chord wired in App.tsx.
//
// Lives in `platform/` because both engine code and theme code read or
// write this signal: themes call openEngineSurface() from their corner-
// icon click; engine code reads engineSurfaceOpen() to decide whether
// to render the takeover. Centralizing the signal here keeps engine
// and theme code from importing from each other.
//
// Phase 1 of the Theming Substrate arc (ARC 1). Plan:
// docs/PLANS/theming-substrate.md §6 Phase 1.

import { createSignal, type Accessor } from "solid-js";
import { onNavEvent } from "@oa/platform/nav";

const [openSig, setOpenSig] = createSignal(false);

/// Reactive accessor — subscribe to engine-surface visibility in
/// components. RetroverseShell uses this to gate L1/R1 tab cycling
/// (when the engine surface is open, gamepad input belongs to it).
export const engineSurfaceOpen: Accessor<boolean> = openSig;

/// Open the engine surface. Idempotent — calling on an already-open
/// surface is a no-op.
export function openEngineSurface(): void {
  setOpenSig(true);
}

/// Close the engine surface. Returns the operator to whichever theme
/// view they came from (themes preserve their own state across the
/// takeover overlay).
export function closeEngineSurface(): void {
  setOpenSig(false);
}

/// Toggle the engine surface. The shared implementation behind all
/// three summon affordances (F12 / Select+Start / corner icon).
export function toggleEngineSurface(): void {
  setOpenSig((v) => !v);
}

/// Wire the Select+Start controller chord — pressing both buttons
/// together (in either order, within ~600ms) toggles the engine
/// surface. App.tsx calls this once at mount; the returned dispose
/// function cleans up the listener.
///
/// We listen on the existing onNavEvent bus rather than poll directly
/// so the chord respects setNavEnabled(false) when the operator turns
/// gamepad nav OFF — they shouldn't get an inadvertent summon from a
/// dropped controller.
export function wireEngineSummonChord(): () => void {
  // Held-until-up timestamps for the two chord buttons. We treat the
  // chord as fired when both have a non-zero held timestamp within
  // CHORD_WINDOW_MS of each other. After firing, we clear both so a
  // long-held chord doesn't re-fire on auto-repeat.
  const CHORD_WINDOW_MS = 600;
  let selectHeldAt = 0;
  let startHeldAt = 0;

  const tryFire = (now: number): void => {
    if (selectHeldAt === 0 || startHeldAt === 0) return;
    const delta = Math.abs(selectHeldAt - startHeldAt);
    if (delta > CHORD_WINDOW_MS) return;
    if (now - Math.max(selectHeldAt, startHeldAt) > CHORD_WINDOW_MS) return;
    selectHeldAt = 0;
    startHeldAt = 0;
    toggleEngineSurface();
  };

  return onNavEvent((event) => {
    if (event.kind !== "button") return;
    if (event.button !== "select" && event.button !== "start") return;
    const now = performance.now();
    if (event.phase === "down") {
      if (event.button === "select") selectHeldAt = now;
      else startHeldAt = now;
      tryFire(now);
    } else if (event.phase === "up") {
      if (event.button === "select") selectHeldAt = 0;
      else startHeldAt = 0;
    }
  });
}
