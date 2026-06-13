// Settings → Controllers — live controller test window (Phase 2.5).
// See docs/PLANS/controller-identity-substrate.md + DECISIONS D14.
//
// The MVP observability surface: a passive, live read-out of every connected
// pad showing (1) its resolved identity + which profile (if any) matched,
// (2) raw button/axis input as it happens, and (3) the NORMALIZED result —
// which logical NavButton/NavDirection each raw input maps to. That last
// column is what proves the mapping is correct (e.g. raw button 1 → "A"
// = Confirm on the Faceoff pad).
//
// It reuses describePad() so it shows EXACTLY what the nav poller uses — no
// duplicated mapping logic. While open it pauses controller navigation (so
// pressing buttons to test doesn't toggle real settings underneath); leave
// with the mouse. This same live-capture surface is the primitive the
// Phase-3 "press each button" wizard + a future remap UI build on.
//
// `Index` (not `For`) drives the button/axis lists: the arrays are stable in
// length and mutate in value every frame, which is exactly Index's contract
// (per-slot accessors, no per-frame teardown).

import { Index, Show, createSignal, onCleanup, onMount, type Accessor, type Component } from "solid-js";
import {
  decodeHat,
  describePad,
  isNavEnabled,
  setNavEnabled,
  type PadDiagnostic,
} from "@oa/platform/nav";

/// One pad's per-frame snapshot for rendering.
type PadFrame = {
  diag: PadDiagnostic;
  pressed: boolean[]; // raw button index → currently pressed
  axes: number[]; // raw axis values
};

function snapshot(): PadFrame[] {
  const pads = navigator.getGamepads?.() ?? [];
  const frames: PadFrame[] = [];
  for (const pad of pads) {
    if (!pad) continue;
    frames.push({
      diag: describePad(pad),
      pressed: pad.buttons.map((b) => b?.pressed ?? false),
      axes: pad.axes.map((a) => a ?? 0),
    });
  }
  return frames;
}

/// What a raw button index resolves to under the pad's effective layout.
function resolveButton(diag: PadDiagnostic, i: number): string {
  const nav = diag.layout.buttons[i];
  if (nav) return nav.toUpperCase();
  const dir = diag.layout.dpad[i];
  if (dir) return `dpad ${dir}`;
  return "—";
}

export const ControllersSettings: Component = () => {
  const [frames, setFrames] = createSignal<PadFrame[]>(snapshot());
  let raf: number | null = null;

  onMount(() => {
    // Pause nav while testing so test presses don't drive the UI underneath.
    const wasEnabled = isNavEnabled();
    setNavEnabled(false);

    const tick = (): void => {
      setFrames(snapshot());
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);

    onCleanup(() => {
      if (raf !== null) cancelAnimationFrame(raf);
      setNavEnabled(wasEnabled);
    });
  });

  return (
    <div class="flex flex-col gap-4">
      <section class="rounded-xl border border-amber-400/20 bg-amber-400/[0.04] p-4">
        <p class="text-[0.75rem] leading-relaxed text-(--color-oa-ink-dim)">
          Controller navigation is <span class="text-(--color-oa-ink)">paused</span> while this
          panel is open, so test presses don't change settings underneath. Use the mouse to
          leave. Press buttons / move sticks on your pad — the live read-out below shows the raw
          input and what each control maps to.
        </p>
      </section>

      <Show
        when={frames().length > 0}
        fallback={
          <section class="rounded-xl border border-white/10 bg-white/[0.02] p-8 text-center">
            <p class="text-[0.85rem] text-(--color-oa-ink-dim)">
              No controller detected. Plug one in (or press a button to wake it) and it will
              appear here.
            </p>
          </section>
        }
      >
        <Index each={frames()}>{(frame) => <PadCard frame={frame} />}</Index>
      </Show>
    </div>
  );
};

const PadCard: Component<{ frame: Accessor<PadFrame> }> = (props) => {
  const diag = (): PadDiagnostic => props.frame().diag;
  return (
    <section class="rounded-xl border border-white/10 bg-white/[0.02] p-5">
      <header class="mb-3">
        <h3 class="text-[0.7rem] font-semibold uppercase tracking-[0.3em] text-(--color-oa-ink)">
          Port {diag().index} — {diag().name || "Unknown controller"}
        </h3>
        <dl class="mt-2 grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-[0.72rem] text-(--color-oa-ink-dim)">
          <dt>device-key</dt>
          <dd class="font-mono text-(--color-oa-ink)">{diag().deviceKey}</dd>
          <dt>VID:PID</dt>
          <dd class="font-mono">
            {diag().vid ?? "—"}:{diag().pid ?? "—"}
          </dd>
          <dt>browser mapping</dt>
          <dd>{diag().mapping === "standard" ? "standard (browser-canonical)" : "non-standard"}</dd>
          <dt>profile</dt>
          <dd>
            <Show when={diag().profiled} fallback={<span>none — using standard layout</span>}>
              <span class="text-emerald-300">matched (controllers.json)</span>
            </Show>
          </dd>
          <dt>buttons / axes</dt>
          <dd>
            {diag().buttonCount} / {diag().axisCount}
          </dd>
        </dl>
      </header>

      <div class="grid grid-cols-2 gap-4 sm:grid-cols-3">
        <div>
          <h4 class="mb-2 text-[0.65rem] uppercase tracking-[0.25em] text-(--color-oa-ink-dim)">
            Buttons
          </h4>
          <div class="flex flex-col gap-1">
            <Index each={props.frame().pressed}>
              {(down, i) => (
                <div
                  class="flex items-center justify-between rounded px-2 py-1 text-[0.72rem] transition-colors"
                  classList={{
                    "bg-emerald-400/20 text-(--color-oa-ink)": down(),
                    "text-(--color-oa-ink-dim)": !down(),
                  }}
                >
                  <span class="font-mono">b{i}</span>
                  <span>{resolveButton(diag(), i)}</span>
                </div>
              )}
            </Index>
          </div>
        </div>

        <div class="col-span-1 sm:col-span-2">
          <h4 class="mb-2 text-[0.65rem] uppercase tracking-[0.25em] text-(--color-oa-ink-dim)">
            Axes
          </h4>
          <div class="flex flex-col gap-1">
            <Index each={props.frame().axes}>
              {(v, i) => {
                const isHat = (): boolean => diag().layout.hatAxis === i;
                const clamped = (): number => Math.max(-1, Math.min(1, v()));
                return (
                  <div class="flex items-center gap-2 text-[0.72rem]">
                    <span class="w-14 font-mono text-(--color-oa-ink-dim)">
                      a{i}
                      {isHat() ? "*" : ""}
                    </span>
                    <div class="relative h-2 flex-1 overflow-hidden rounded bg-white/10">
                      <div
                        class="absolute top-0 h-full bg-sky-400/60"
                        style={{
                          // Fill from center outward in the direction of the value.
                          left: `${clamped() < 0 ? 50 + clamped() * 50 : 50}%`,
                          width: `${Math.abs(clamped()) * 50}%`,
                        }}
                      />
                    </div>
                    <span class="w-24 text-right font-mono text-(--color-oa-ink-dim)">
                      {v().toFixed(2)}
                      {isHat() ? ` ${decodeHat(v()) ?? "·"}` : ""}
                    </span>
                  </div>
                );
              }}
            </Index>
          </div>
          <p class="mt-2 text-[0.65rem] text-(--color-oa-ink-dim)">
            * = DPad HAT axis (decoded direction shown at right)
          </p>
        </div>
      </div>
    </section>
  );
};
