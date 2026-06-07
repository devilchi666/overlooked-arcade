// Per-game touch hotspot overlay.
//
// Renders labelled accent-coloured rectangles over the NDS bottom
// screen showing where in-game tappable regions live. Reads
// `touch_hotspots` from the running game's MergedGameInfo (Phase 1
// schema addition) and maps the NDS-bottom-screen native coords
// (0..256 × 0..192) to viewport pixels via a contain-fit calculation
// against the standard NDS combined-frame aspect (256×384).
//
// Toggle: per-session signal owned by App.tsx, surfaced via the
// QuickSettings "Show touch hints" row. Gate: running game must be
// NDS, signal must be ON, and gameMode must be on (full-bleed game
// rendering). Two-window mode is unsupported for v1 — the native
// game window's coordinate space is decoupled from the WebView.
//
// v1 layout assumption: melonDS default stacked vertical layout
// (top screen above bottom; combined frame is 256 wide × 384 tall;
// bottom screen occupies y[192..384]). Non-default melonDS layouts
// (side-by-side, top-only, etc.) misplace the hotspots until v2
// reads the layout core option.

import {
  createMemo,
  createResource,
  createSignal,
  For,
  onCleanup,
  onMount,
  Show,
  type Component,
} from "solid-js";
import type { SystemId } from "../themes/registry";
import { systemUIConfigs } from "../themes/systemUIConfigs";
import { getGameInfo, type TouchHotspot } from "../library/gameInfo";

type Props = {
  /// Reactive accessor for the SystemId + romId of the currently-
  /// running game, or null when no game is running. Drives both the
  /// gate (must be NDS) and the lookup (which game's hotspots to
  /// fetch).
  running: () => { systemId: SystemId; romId: string; romHash?: string; romTitle?: string } | null;
  /// Reactive accessor for the per-session "Show touch hints" toggle.
  /// When false, the overlay unmounts entirely (no fetch either).
  enabled: () => boolean;
};

/// Per-system gate. Reads `touchInputSupported` from the per-system
/// UI registry — collapses the historical HOTSPOT_SYSTEMS /
/// STYLUS_SYSTEMS / QuickSettings triplicate into one source of truth
/// (Theming Substrate ARC 1 Phase 2 cleanup).
function isTouchSystem(systemId: SystemId): boolean {
  return systemUIConfigs[systemId]?.touchInputSupported === true;
}

/// NDS combined framebuffer dimensions (top + bottom stacked vertically).
/// melonDS's default layout puts the top screen at y[0..192] and the
/// bottom screen at y[192..384] of a 256×384 image. Both screens are
/// 256×192 native.
const NDS_FRAME_W = 256;
const NDS_FRAME_H = 384;
const NDS_SCREEN_W = 256;
const NDS_SCREEN_H = 192;
const NDS_FRAME_ASPECT = NDS_FRAME_W / NDS_FRAME_H; // 0.667 (portrait)

const TouchHotspotOverlay: Component<Props> = (props) => {
  const [vpW, setVpW] = createSignal(window.innerWidth);
  const [vpH, setVpH] = createSignal(window.innerHeight);

  // Track viewport resize so the overlay re-maps on window resize /
  // fullscreen toggle. Cheap — single resize listener.
  onMount(() => {
    const onResize = () => {
      setVpW(window.innerWidth);
      setVpH(window.innerHeight);
    };
    window.addEventListener("resize", onResize);
    onCleanup(() => window.removeEventListener("resize", onResize));
  });

  // Only fetch hotspots when we have a running NDS game AND the
  // toggle is on. The resource auto-refetches on `running` change so
  // launching a different NDS game refreshes the hotspot set.
  const [hotspots] = createResource<TouchHotspot[], Props["running"]>(
    () => {
      const r = props.running();
      if (!r) return null as never;
      if (!isTouchSystem(r.systemId)) return null as never;
      if (!props.enabled()) return null as never;
      return props.running;
    },
    async (runningAccessor) => {
      const r = runningAccessor();
      if (!r) return [];
      try {
        const info = await getGameInfo({
          systemId: r.systemId,
          romId: r.romId,
          romHash: r.romHash,
          romTitle: r.romTitle,
        });
        return info?.touchHotspots ?? [];
      } catch (e) {
        console.warn("[oa-hotspots] getGameInfo failed:", e);
        return [];
      }
    },
  );

  // Compute the contain-fit rectangle of the NDS combined frame
  // inside the current viewport. The wgpu renderer paints the game
  // framebuffer to the WebView's transparent background using
  // object-fit:contain-style math; this mirrors that so the overlay
  // tracks the actual game image.
  const fittedRect = createMemo(() => {
    const w = vpW();
    const h = vpH();
    const viewportAspect = w / h;
    let fitW: number;
    let fitH: number;
    if (viewportAspect > NDS_FRAME_ASPECT) {
      // Viewport is wider than NDS portrait aspect — pillarbox.
      fitH = h;
      fitW = h * NDS_FRAME_ASPECT;
    } else {
      // Viewport is narrower or equal — letterbox.
      fitW = w;
      fitH = w / NDS_FRAME_ASPECT;
    }
    const left = (w - fitW) / 2;
    const top = (h - fitH) / 2;
    return { left, top, width: fitW, height: fitH };
  });

  // Map a hotspot (NDS bottom-screen native coords) to a viewport-
  // pixel rect inside the bottom half of the fitted frame.
  const mapHotspot = (hs: TouchHotspot) => {
    const fit = fittedRect();
    const screenLeft = fit.left;
    // Bottom screen starts at half the combined-frame height.
    const screenTop = fit.top + fit.height * (NDS_SCREEN_H / NDS_FRAME_H);
    const screenWidth = fit.width;
    const screenHeight = fit.height * (NDS_SCREEN_H / NDS_FRAME_H);

    const px = screenLeft + (hs.x / NDS_SCREEN_W) * screenWidth;
    const py = screenTop + (hs.y / NDS_SCREEN_H) * screenHeight;
    const pw = (hs.w / NDS_SCREEN_W) * screenWidth;
    const ph = (hs.h / NDS_SCREEN_H) * screenHeight;
    return { left: px, top: py, width: pw, height: ph };
  };

  return (
    <Show when={props.enabled() && (hotspots() ?? []).length > 0}>
      <div class="pointer-events-none fixed inset-0 z-30" aria-hidden="true">
        <For each={hotspots() ?? []}>
          {(hs) => {
            const rect = mapHotspot(hs);
            return (
              <div
                class="absolute"
                style={{
                  left: `${rect.left}px`,
                  top: `${rect.top}px`,
                  width: `${rect.width}px`,
                  height: `${rect.height}px`,
                }}
              >
                {/* Outlined rectangle — thin accent-colored border,
                    matches StylusOverlay's reticle style. Slightly
                    softened with a subtle shadow so it reads against
                    bright + dark game frames both. */}
                <div
                  class="absolute inset-0 rounded-md border-2"
                  style={{
                    "border-color": "var(--color-system-accent, currentColor)",
                    "box-shadow": "0 0 0 1px rgba(0,0,0,0.4)",
                    opacity: 0.85,
                  }}
                />
                {/* Floating label — positioned just above the box.
                    Uses the accent color for the chip background +
                    high-contrast ink for the text so it reads cleanly
                    against any game frame underneath. Top-align below
                    the box if the hotspot is too close to the top
                    edge of the bottom screen. */}
                <div
                  class="absolute left-0 -top-6 whitespace-nowrap rounded-md border border-(--color-system-accent)/50 bg-(--color-oa-bg-deep)/90 px-1.5 py-0.5 text-[0.65rem] font-semibold uppercase tracking-wider text-(--color-system-accent) backdrop-blur"
                  style={{ opacity: 0.95 }}
                >
                  {hs.label}
                </div>
              </div>
            );
          }}
        </For>
      </div>
    </Show>
  );
};

export default TouchHotspotOverlay;
