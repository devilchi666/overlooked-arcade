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

export type Hints = Partial<Record<NavButton, string>>;

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

/// Order buttons appear in the bar. Reads left-to-right Xbox-style:
/// A/B then X/Y then shoulder + start.
const BUTTON_ORDER: NavButton[] = [
  "a",
  "b",
  "x",
  "y",
  "l1",
  "r1",
  "start",
  "select",
];

/// Display label for the button glyph. Xbox-style — Y top, A bottom,
/// X left, B right. Operators on Nintendo-convention layouts can swap
/// labels via Slice E's A/B swap setting (Phase 0 baseline keeps
/// Xbox glyphs).
const BUTTON_GLYPH: Record<NavButton, string> = {
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
    const hints = currentHints();
    return BUTTON_ORDER.filter((b) => hints[b]).map((b) => ({ button: b, label: hints[b]! }));
  });
  return (
    <Show when={hasSeenGamepad() && visibleEntries().length > 0}>
      <div
        class="oa-hint-bar pointer-events-none fixed inset-x-0 bottom-0 z-40 flex justify-center px-4 pb-3"
        aria-hidden="true"
      >
        <div class="oa-hint-bar-inner flex items-center gap-4 rounded-full border border-white/10 bg-black/60 px-4 py-1.5 text-[0.75rem] font-medium text-(--color-oa-ink) backdrop-blur-md">
          <For each={visibleEntries()}>
            {(entry) => (
              <div class="oa-hint-entry flex items-center gap-2">
                <span class="oa-hint-glyph inline-flex h-5 min-w-5 items-center justify-center rounded-full bg-white/15 px-1 text-[0.65rem] font-bold text-(--color-oa-ink)">
                  {BUTTON_GLYPH[entry.button]}
                </span>
                <span class="oa-hint-label text-(--color-oa-ink-dim)">{entry.label}</span>
              </div>
            )}
          </For>
        </div>
      </div>
    </Show>
  );
};
