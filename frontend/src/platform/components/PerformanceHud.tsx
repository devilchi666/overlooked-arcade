// Performance HUD overlay (Tools → Performance HUD).
//
// Two layers of telemetry:
//
//   1. UI FPS — `requestAnimationFrame` cadence inside the WebView.
//      Useful for spotting jank in the React/Solid layer (overflowing
//      menus, dialog mount cost, big lists rerendering). Runs at the
//      host display refresh rate when nothing's blocking.
//
//   2. EMU FPS — emulator-thread fps + audio counters polled from Rust
//      via `get_perf_stats`. Only shown when a core is loaded; matches
//      the core's nominal fps (PCE 59.83, SNES 60.10, etc.) when the
//      host can keep up. Audio dropped > 0 means the encoder fell
//      behind — typically a sign the rewind buffer cap or run-ahead is
//      starving the audio thread.

import { createEffect, createSignal, onCleanup, onMount, Show, type Component } from "solid-js";
import { getPerfStats } from "@oa/platform/api/systemApi";

type Props = {
  visible: boolean;
};

type PerfStats = {
  coreLoaded: boolean;
  fps: number;
  frameCount: number;
  audioPushed: number;
  audioDropped: number;
  coreFpsNominal: number;
};

const SAMPLE_WINDOW_MS = 1000;
const EMU_POLL_INTERVAL_MS = 250;

export const PerformanceHud: Component<Props> = (props) => {
  // --- UI render-loop FPS (rAF) -------------------------------------------
  const [uiFps, setUiFps] = createSignal(0);
  const [uiFrameTimeMs, setUiFrameTimeMs] = createSignal(0);

  let rafId: number | undefined;
  let frameCount = 0;
  let windowStart = 0;
  let lastFrame = 0;
  let frameTimeAccum = 0;

  function rafTick(t: number) {
    if (windowStart === 0) windowStart = t;
    if (lastFrame !== 0) frameTimeAccum += t - lastFrame;
    lastFrame = t;
    frameCount += 1;

    const elapsed = t - windowStart;
    if (elapsed >= SAMPLE_WINDOW_MS) {
      setUiFps((frameCount * 1000) / elapsed);
      setUiFrameTimeMs(frameTimeAccum / Math.max(frameCount, 1));
      frameCount = 0;
      frameTimeAccum = 0;
      windowStart = t;
    }
    rafId = window.requestAnimationFrame(rafTick);
  }

  // --- Emu-thread telemetry (Tauri command poll) --------------------------
  const [emu, setEmu] = createSignal<PerfStats | null>(null);
  let emuPollId: number | undefined;

  function startEmuPoll() {
    if (emuPollId !== undefined) return;
    const fetch = () => {
      void getPerfStats<PerfStats>()
        .then((s) => setEmu(s ?? null))
        .catch(() => {});
    };
    fetch();
    emuPollId = window.setInterval(fetch, EMU_POLL_INTERVAL_MS);
  }
  function stopEmuPoll() {
    if (emuPollId !== undefined) {
      window.clearInterval(emuPollId);
      emuPollId = undefined;
    }
  }

  // Start polling only when the HUD is visible — saves the round-trip
  // cost (Tauri ipc isn't free; ~0.3 ms per call) when nobody's looking.
  createEffect(() => {
    if (props.visible) startEmuPoll();
    else stopEmuPoll();
  });

  onMount(() => {
    rafId = window.requestAnimationFrame(rafTick);
  });
  onCleanup(() => {
    if (rafId !== undefined) window.cancelAnimationFrame(rafId);
    stopEmuPoll();
  });

  return (
    <Show when={props.visible}>
      <div
        class="pointer-events-none fixed right-3 top-14 z-[45] flex flex-col gap-1 rounded-md border border-white/10 bg-black/65 px-2.5 py-1.5 font-mono text-[0.7rem] text-(--color-oa-ink) shadow-lg backdrop-blur"
        role="status"
        aria-live="polite"
      >
        {/* UI row — always visible */}
        <div class="flex items-baseline gap-3">
          <span class="w-8 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            UI
          </span>
          <span class="tabular-nums text-(--color-system-accent)">
            {uiFps().toFixed(1)} fps
          </span>
          <span class="tabular-nums text-(--color-oa-ink-dim)">
            {uiFrameTimeMs().toFixed(2)} ms
          </span>
        </div>
        {/* Emu rows — only when a core is loaded */}
        <Show when={emu()?.coreLoaded}>
          <div class="flex items-baseline gap-3 border-t border-white/5 pt-1">
            <span class="w-8 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Emu
            </span>
            <span
              class="tabular-nums"
              classList={{
                "text-(--color-system-accent)":
                  Math.abs(emu()!.fps - emu()!.coreFpsNominal) < 1,
                "text-amber-300":
                  Math.abs(emu()!.fps - emu()!.coreFpsNominal) >= 1,
              }}
              title={`Nominal ${emu()!.coreFpsNominal.toFixed(2)} fps`}
            >
              {emu()!.fps.toFixed(1)} fps
            </span>
            <span class="tabular-nums text-(--color-oa-ink-dim)">
              {emu()!.frameCount.toLocaleString()} f
            </span>
          </div>
          <div class="flex items-baseline gap-3">
            <span class="w-8 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Snd
            </span>
            <span class="tabular-nums text-(--color-oa-ink-dim)">
              {emu()!.audioPushed.toLocaleString()} +
            </span>
            <span
              class="tabular-nums"
              classList={{
                "text-(--color-oa-ink-dim)": emu()!.audioDropped === 0,
                "text-red-300": emu()!.audioDropped > 0,
              }}
              title={
                emu()!.audioDropped > 0
                  ? "Audio drops — host fell behind, or buffer too small"
                  : "No audio drops"
              }
            >
              {emu()!.audioDropped.toLocaleString()}
            </span>
          </div>
        </Show>
      </div>
    </Show>
  );
};
