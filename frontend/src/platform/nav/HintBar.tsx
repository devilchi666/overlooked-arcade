// Controller-nav hint bar. Persistent footer that shows the operator
// which buttons do what on the current screen.
//
// Model: components register a hint set via `<HintRegion hints={...}>`.
// Regions live in a module-level stack — the most recently mounted
// (deepest in the tree) wins. Solid mount/unmount ordering means a
// child region naturally takes precedence over its parent; an opened
// dialog's region overrides the underlying screen.
//
// VERB-NATIVE (DECISIONS D18): hints are keyed by semantic VERB
// (`confirm` / `back` / `secondary` / …), not by physical button. The bar
// resolves each verb → its currently-bound button (via navBindings, swap-aware)
// → a glyph from the active GlyphSet. So remapping an input — or flipping the
// A/B swap — re-paints every on-screen hint automatically, for free. The two
// directional descriptors (`dpad` / `stick`) carry fixed glyphs (direction
// remapping is identity in S1).
//
// Auto-hidden when no gamepad has been seen this session. Reappears
// on first connect; the connect itself prompts the operator that the
// pad is wired up.

import { createEffect, createMemo, createSignal, For, onCleanup, Show, type Accessor, type Component, type JSX } from "solid-js";
import { hasSeenGamepad } from "./gamepad";
import { isSwapAB, navBindings } from "./navBindings";
import { DEFAULT_GLYPH_SET, verbGlyph } from "./glyphs";
import type { NavVerb } from "./verbs";
import { nowPlaying } from "@oa/platform/lib/audio";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";

/// A hint is keyed by the semantic verb it describes, plus the two directional
/// descriptors (`dpad` / `stick`) that let a page narrate its movement model
/// ("DPad switch region", "Stick navigate") alongside the action verbs.
export type HintKey = NavVerb | "dpad" | "stick";

export type Hints = Partial<Record<HintKey, string>>;

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
/// descriptors first (DPad + stick describing how to move), then the
/// focused-item action verbs, then the structural / global verbs.
const HINT_ORDER: HintKey[] = [
  "dpad",
  "stick",
  "Confirm",
  "Back",
  "Secondary",
  "Tertiary",
  "PrevSection",
  "NextSection",
  "Menu",
  "OpenQuickSettings",
];

/// Resolve a hint key to the glyph to paint. Directional descriptors are fixed;
/// verbs resolve through the current bindings (swap-aware) so the glyph tracks
/// remaps. Returns null when no glyph applies (e.g. an unbound reserved verb) —
/// the bar omits that entry.
function glyphForKey(key: HintKey, swap: boolean): string | null {
  if (key === "dpad") return DEFAULT_GLYPH_SET.dpad;
  if (key === "stick") return DEFAULT_GLYPH_SET.stick;
  return verbGlyph(key, navBindings(), swap);
}

export const HintBar: Component = () => {
  const currentHints = createMemo<Hints>(() => {
    const s = stack();
    return s.length === 0 ? {} : s[s.length - 1].hints;
  });
  const visibleEntries = createMemo(() => {
    const hints = currentHints();
    const swap = isSwapAB();
    return HINT_ORDER.flatMap((key) => {
      const label = hints[key];
      if (label === undefined) return [];
      const glyph = glyphForKey(key, swap);
      if (glyph === null) return [];
      return [{ key, glyph, label }];
    });
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
                    {entry.glyph}
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
