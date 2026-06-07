// Per-system boot animation overlay.
//
// Triggered by a transition in `activeSystemId` — when the operator
// enters a system (sidebar nav into NES, etc.), a brief tinted overlay
// fades in, holds, fades out. Plan §10 frequency: every entry. Plan
// §10 length: medium (~1-1.5s) so the operator feels the system but
// doesn't get annoyed on repeated entry.
//
// Defaults to a CSS-keyframe radial gradient tinted with the new
// system's `--color-system-accent`. Pilot slices (6-8) can override
// per-system via `<exe_dir>/assets/system-ui/<systemId>/boot-animation/`
// once that path becomes a real consumer — Slice 4 ships the framework
// only; per-pilot keyframes load in pilot slices.
//
// Skippable on any input — mouse / keyboard / gamepad. The animation
// is short enough that not skipping is fine for first-time entries,
// but operators bouncing through systems should be able to fast-skip.
// Reduced-motion compresses everything to a 200 ms fade.

import {
  createEffect,
  createSignal,
  on,
  onCleanup,
  onMount,
  Show,
  type Component,
} from "solid-js";
import type { SystemId } from "@oa/platform/themes/registry";
import { isPerSystemUiEnabled } from "@oa/platform/themes/systemUiSound";
import { isBootAnimationsEnabled } from "@oa/platform/themes/systemBootAnimation";
import { prefersReducedMotion } from "@oa/platform/lib/reducedMotion";
import { onNavEvent } from "../nav/gamepad";
import { dispatchUiSound } from "@oa/platform/lib/audio";

type Props = {
  /// Reactive accessor for the currently-active system (typically
  /// derived from `viewToSystemId(currentView())` in App.tsx — i.e.
  /// what the sidebar filter is set to). The animation fires on
  /// each non-null transition; null → null and same → same don't
  /// fire.
  activeSystemId: () => SystemId | null;
};

/// Full animation duration in milliseconds when boot animations are
/// enabled and reduced-motion is not in effect. Plan §10 medium tier.
const BOOT_DURATION_MS = 1000;

/// Short cross-fade used when `prefers-reduced-motion: reduce` is on
/// (accessibility floor — operator hasn't opted out, the OS has
/// communicated a motion-sensitivity preference). The
/// "Boot animations" toggle being OFF is treated as "no animation at
/// all" — see the trigger effect for the early-return.
const BOOT_FADE_MS = 200;

const SystemBootAnimation: Component<Props> = (props) => {
  // The system the overlay is currently animating. null when no
  // animation is in flight. Distinct from `activeSystemId` — that
  // changes the moment the operator picks a new system in the
  // sidebar; this lags by the boot duration so the overlay can
  // unmount cleanly on its own timer.
  const [bootingSystemId, setBootingSystemId] = createSignal<SystemId | null>(null);
  // Cached duration so the unmount timeout matches whatever path the
  // animation entered through (full / fade). Re-read on each trigger.
  const [animDuration, setAnimDuration] = createSignal(BOOT_DURATION_MS);
  let lastActive: SystemId | null = props.activeSystemId();
  let timeoutId: number | undefined;

  const cancelTimeout = () => {
    if (timeoutId !== undefined) {
      window.clearTimeout(timeoutId);
      timeoutId = undefined;
    }
  };

  const skip = () => {
    if (bootingSystemId() === null) return;
    cancelTimeout();
    setBootingSystemId(null);
  };

  // `on(...)` is Solid's explicit-deps wrapper that lets us read
  // `props.activeSystemId()` reactively while keeping the body's other
  // signal reads (toggles, reduced-motion) untracked — otherwise
  // flipping the Settings toggle would re-fire the animation. We only
  // want changes to `activeSystemId` to drive the trigger.
  createEffect(
    on(
      () => props.activeSystemId(),
      (active) => {
        if (active === null) {
          lastActive = null;
          return;
        }
        if (active === lastActive) return;
        lastActive = active;
        if (!isPerSystemUiEnabled()) return;
        // "Boot animations" toggle OFF → skip the overlay entirely
        // (operator wants instant transitions). Re-enabling the
        // toggle restores the full 1 s animation on the next entry.
        if (!isBootAnimationsEnabled()) return;
        // prefers-reduced-motion is the accessibility floor —
        // orthogonal to the toggle, kicks in when the OS-level
        // setting is on. Drops the full 1 s overlay to a 200 ms
        // cross-fade so the system-identity hint still happens
        // without the long animation.
        const duration = prefersReducedMotion() ? BOOT_FADE_MS : BOOT_DURATION_MS;
        cancelTimeout();
        setAnimDuration(duration);
        setBootingSystemId(active);
        timeoutId = window.setTimeout(() => {
          setBootingSystemId(null);
          timeoutId = undefined;
        }, duration);
        // Fire the per-system boot-intro SFX in parallel with the
        // visual. Pilot slices drop a boot-intro.<ext> per system;
        // resolver returns null + dispatch no-ops for systems
        // without one. Reduced-motion still fires the SFX since the
        // visual still plays (just shorter); the toggle-off path
        // returned earlier so the SFX never reaches here.
        void dispatchUiSound(active, "boot-intro");
      },
    ),
  );

  // Skip on any input — mouse / keyboard / gamepad. Listeners attach
  // at mount and stay live; the skip() guard makes them no-ops when
  // no animation is in flight, so they're cheap when idle.
  onMount(() => {
    const onMouse = (e: MouseEvent) => {
      // Ignore mousemove — only deliberate clicks / right-clicks
      // count. Otherwise moving the cursor onto the OA window would
      // suppress every boot.
      if (e.type === "mousemove") return;
      skip();
    };
    const onKey = () => skip();
    const unsubGamepad = onNavEvent(() => skip());
    document.addEventListener("mousedown", onMouse, { capture: true });
    document.addEventListener("keydown", onKey, { capture: true });
    onCleanup(() => {
      document.removeEventListener("mousedown", onMouse, { capture: true });
      document.removeEventListener("keydown", onKey, { capture: true });
      unsubGamepad();
      cancelTimeout();
    });
  });

  return (
    <Show when={bootingSystemId()}>
      {(sysId) => (
        <div
          class="oa-boot-animation pointer-events-none fixed inset-0 z-40"
          data-system={sysId()}
          aria-hidden="true"
          style={{
            "--oa-boot-duration": `${animDuration()}ms`,
          }}
        />
      )}
    </Show>
  );
};

export default SystemBootAnimation;
