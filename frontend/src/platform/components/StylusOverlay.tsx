// Visual stylus reticle overlay for stylus-using systems (NDS today).
//
// The OS cursor stays visible whenever the operator's mouse is over the
// OA window — they can SEE where they're aiming. What the OS cursor
// doesn't communicate is (a) "I'm in stylus mode for this game" and
// (b) "my tap is registering RIGHT NOW." This overlay adds a small
// reticle that follows the cursor with explicit press feedback:
//
//   - hollow ring  → stylus hovering (mouse over screen, button up)
//   - filled inset → stylus tapping (mouse button down — game receives
//                    pointer-pressed)
//
// Mounted in App.tsx alongside SystemBackground / SystemBootAnimation.
// Pointer-events: none so it never intercepts the operator's actual
// aim. Renders only while a stylus-using game is actively running.

import {
  createMemo,
  createSignal,
  onCleanup,
  onMount,
  Show,
  type Component,
} from "solid-js";
import type { SystemId } from "@oa/platform/themes/registry";
import { systemSupportsTouch } from "@oa/platform/themes/systemUIConfigs";

type Props = {
  /// SystemId of the currently-running game, or null when no game is
  /// running. Drives the systems-using-stylus gate below.
  runningSystemId: () => SystemId | null;
};

/// Per-system gate. Reads the platform FACTUAL touch-support lookup
/// (`systemSupportsTouch`, D34 — hardware fact, theme-independent) —
/// collapses the historical HOTSPOT_SYSTEMS / STYLUS_SYSTEMS /
/// QuickSettings triplicate into one source of truth (Theming
/// Substrate ARC 1 Phase 2 cleanup). Light-gun systems use the cursor
/// for AIM rather than TAP and stay opt-out; future stylus-vs-aim
/// splits add a finer field then.
function isTouchSystem(systemId: SystemId): boolean {
  return systemSupportsTouch(systemId);
}

const StylusOverlay: Component<Props> = (props) => {
  const [mouseX, setMouseX] = createSignal(0);
  const [mouseY, setMouseY] = createSignal(0);
  const [pressed, setPressed] = createSignal(false);
  // True once the cursor has entered the document at least once. Avoids
  // rendering the reticle at (0, 0) on first paint before any
  // mousemove has fired.
  const [tracking, setTracking] = createSignal(false);

  const enabled = createMemo(() => {
    const s = props.runningSystemId();
    return s !== null && isTouchSystem(s);
  });

  onMount(() => {
    const onMove = (e: MouseEvent) => {
      setMouseX(e.clientX);
      setMouseY(e.clientY);
      if (!tracking()) setTracking(true);
    };
    const onDown = (e: MouseEvent) => {
      // Only left-button taps register as stylus taps; right-click
      // stays free for context menus.
      if (e.button === 0) setPressed(true);
    };
    const onUp = (e: MouseEvent) => {
      if (e.button === 0) setPressed(false);
    };
    document.addEventListener("mousemove", onMove, { passive: true });
    document.addEventListener("mousedown", onDown, { passive: true });
    document.addEventListener("mouseup", onUp, { passive: true });
    onCleanup(() => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("mouseup", onUp);
    });
  });

  return (
    <Show when={enabled() && tracking()}>
      <div
        class="pointer-events-none fixed z-30"
        style={{
          left: `${mouseX() - 14}px`,
          top: `${mouseY() - 14}px`,
          width: "28px",
          height: "28px",
        }}
        aria-hidden="true"
      >
        {/* Outer ring — hollow when hovering, filled (with scale-in) on
            press. Uses the active system accent so the reticle matches
            the per-system theme. */}
        <div
          class="absolute inset-0 rounded-full border-2 transition-transform duration-100"
          style={{
            "border-color": "var(--color-system-accent, currentColor)",
            "background": pressed()
              ? "color-mix(in oklch, var(--color-system-accent), transparent 55%)"
              : "transparent",
            "opacity": pressed() ? 0.85 : 0.7,
            "transform": pressed() ? "scale(0.78)" : "scale(1)",
          }}
        />
        {/* Center dot — pinpoint precision target. */}
        <div
          class="absolute left-1/2 top-1/2 h-1 w-1 -translate-x-1/2 -translate-y-1/2 rounded-full"
          style={{
            "background": "var(--color-system-accent, currentColor)",
            "opacity": 0.9,
          }}
        />
      </div>
    </Show>
  );
};

export default StylusOverlay;
