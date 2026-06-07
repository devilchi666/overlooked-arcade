// Controller-nav hint bar. Persistent footer that shows the operator
// which buttons do what on the current screen.
//
// Model: components register a hint set via `<HintRegion hints={...}>`.
// Regions live in a module-level stack — the most recently mounted
// (deepest in the tree) wins. Solid mount/unmount ordering means a
// child region naturally takes precedence over its parent; an opened
// dialog's region overrides the underlying screen.
//
// Auto-hidden when no gamepad has been seen this session. Reappears
// on first connect; the connect itself prompts the operator that the
// pad is wired up.

import { createEffect, createMemo, createSignal, For, onCleanup, Show, type Accessor, type Component, type JSX } from "solid-js";
import type { NavButton } from "./types";
import { hasSeenGamepad } from "./gamepad";
import { isSwapAB } from "./focus";
import { nowPlaying } from "@oa/platform/lib/audio";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";

/// Pseudo-glyphs surfaced in the hint bar that don't correspond to
/// a single NavButton — the DPad and the left stick each get a slot
/// so pages can describe their navigation model ("DPad switch region",
/// "Stick navigate") alongside the face-button glyphs.
export type HintGlyph = NavButton | "dpad" | "stick";

export type Hints = Partial<Record<HintGlyph, string>>;

type StackEntry = { id: number; hints: Hints };

const [stack, setStack] = createSignal<StackEntry[]>([]);
let nextId = 0;

/// Push a hint set for the lifetime of this component. Pops on
/// cleanup. Multiple regions can be mounted at once; the most recently
/// mounted (deepest in the tree) wins because Solid mounts children
/// after parents. Updates to the `hints` prop flow into the stack
/// entry without changing its position.
export const HintRegion: Component<{ hints: Hints | Accessor<Hints>; children?: JSX.Element }> = (props) => {
  const id = nextId++;
  const resolved = (): Hints => {
    const h = props.hints;
    return typeof h === "function" ? (h as Accessor<Hints>)() : h;
  };
  let pushed = false;
  createEffect(() => {
    const h = resolved();
    setStack((s) => {
      if (!pushed) {
        pushed = true;
        return [...s, { id, hints: h }];
      }
      return s.map((e) => (e.id === id ? { ...e, hints: h } : e));
    });
  });
  onCleanup(() => {
    setStack((s) => s.filter((e) => e.id !== id));
  });
  return <>{props.children}</>;
};

/// Order glyphs appear in the bar. Reads left-to-right: navigation
/// glyphs first (DPad + stick describing how to move), then A/B/X/Y,
/// then shoulder + start.
const HINT_ORDER: HintGlyph[] = [
  "dpad",
  "stick",
  "a",
  "b",
  "x",
  "y",
  "l1",
  "r1",
  "start",
  "select",
];

/// Display label for the glyph. Xbox-style — Y top, A bottom, X left,
/// B right. Operators on Nintendo-convention layouts can swap labels
/// via Slice E's A/B swap setting (Phase 0 baseline keeps Xbox glyphs).
const HINT_GLYPH: Record<HintGlyph, string> = {
  dpad: "✥",
  stick: "○",
  a: "A",
  b: "B",
  x: "X",
  y: "Y",
  l1: "LB",
  r1: "RB",
  l2: "LT",
  r2: "RT",
  start: "≡",
  select: "▢",
  l3: "L3",
  r3: "R3",
  home: "⌂",
};

export const HintBar: Component = () => {
  const currentHints = createMemo<Hints>(() => {
    const s = stack();
    return s.length === 0 ? {} : s[s.length - 1].hints;
  });
  const visibleEntries = createMemo(() => {
    let hints = currentHints();
    // Nintendo-layout swap: render the A glyph next to the B label and
    // vice versa so the operator sees which physical button confirms.
    if (isSwapAB() && (hints.a !== undefined || hints.b !== undefined)) {
      hints = { ...hints, a: hints.b, b: hints.a };
    }
    return HINT_ORDER.filter((b) => hints[b]).map((b) => ({ button: b, label: hints[b]! }));
  });
  /// Now-playing label sourced from the platform-music bus signal.
  /// Empty string when nothing is playing → the chip stays hidden.
  const nowPlayingLabel = createMemo<string>(() => {
    const np = nowPlaying();
    if (!np) return "";
    const theme = systemThemes[np.systemId as SystemId];
    return theme?.displayName ?? np.systemId;
  });
  // The bar mounts whenever EITHER hints OR a now-playing chip wants
  // to render, so launching a game that triggers platform music makes
  // the bar materialize on its own even if no controller is connected
  // yet (mouse-only operators still see "now playing" feedback).
  const shouldRender = createMemo(
    () => (hasSeenGamepad() && visibleEntries().length > 0) || nowPlayingLabel() !== "",
  );
  return (
    <Show when={shouldRender()}>
      <div
        class="oa-hint-bar pointer-events-none fixed inset-x-0 bottom-0 z-[60] flex justify-center px-4 pb-3"
        aria-hidden="true"
      >
        <div class="oa-hint-bar-inner flex items-center gap-4 rounded-full border border-white/10 bg-black/60 px-4 py-1.5 text-[0.75rem] font-medium text-(--color-oa-ink) backdrop-blur-md">
          {/* Now-playing chip — sits on the left of the bar with its
              own equalizer-bar pulse so the operator gets a passive
              "the system is alive" cue without having to focus the
              audio settings. Hidden when no platform music is on the
              bus. */}
          <Show when={nowPlayingLabel() !== ""}>
            <div class="oa-now-playing-entry flex items-center gap-2 border-r border-white/10 pr-3">
              <span
                class="oa-now-playing-glyph relative inline-flex h-5 w-6 items-center justify-end gap-[2px]"
                aria-hidden="true"
              >
                <span class="block h-2 w-[2px] animate-[oa-eq_900ms_ease-in-out_infinite] rounded-sm bg-(--color-system-accent)" style={{ "animation-delay": "0ms" }} />
                <span class="block h-3 w-[2px] animate-[oa-eq_900ms_ease-in-out_infinite] rounded-sm bg-(--color-system-accent)" style={{ "animation-delay": "120ms" }} />
                <span class="block h-2 w-[2px] animate-[oa-eq_900ms_ease-in-out_infinite] rounded-sm bg-(--color-system-accent)" style={{ "animation-delay": "240ms" }} />
              </span>
              <span class="oa-now-playing-label text-(--color-oa-ink-dim)">
                {nowPlayingLabel()}
              </span>
            </div>
          </Show>
          {/* Existing button hints. Only show when a gamepad has been
              seen — otherwise the now-playing chip floats on its own. */}
          <Show when={hasSeenGamepad()}>
            <For each={visibleEntries()}>
              {(entry) => (
                <div class="oa-hint-entry flex items-center gap-2">
                  <span class="oa-hint-glyph inline-flex h-5 min-w-5 items-center justify-center rounded-full bg-white/15 px-1 text-[0.65rem] font-bold text-(--color-oa-ink)">
                    {HINT_GLYPH[entry.button]}
                  </span>
                  <span class="oa-hint-label text-(--color-oa-ink-dim)">{entry.label}</span>
                </div>
              )}
            </For>
          </Show>
        </div>
      </div>
    </Show>
  );
};
