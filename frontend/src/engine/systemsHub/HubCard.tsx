// Shared card primitive for the Systems hub (Pillar-B byproduct). A whole-card
// native <button> (so the spatial nav engine discovers + drives it with zero
// per-control wiring) styled like the Game-media card the operator chose: theme
// accent stripe + title + subtitle + a free body slot (status rows / blurb).
// Classes lifted from LibraryManagerPage's per-system card.

import { Show, type Component, type JSX } from "solid-js";
import type { SystemId } from "@oa/platform/themes/registry";

export const HubCard: Component<{
  /// Drives the per-system accent stripe via `data-system` + the palette cascade.
  system?: SystemId;
  title: string;
  subtitle?: string;
  onActivate: () => void;
  disabled?: boolean;
  children?: JSX.Element;
}> = (props) => (
  <button
    type="button"
    data-system={props.system}
    disabled={props.disabled}
    onClick={(e) => {
      e.currentTarget.blur();
      props.onActivate();
    }}
    class="flex h-full flex-col gap-3 rounded-lg border border-white/10 bg-white/[0.03] p-3 text-left transition hover:border-white/20 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent) disabled:cursor-not-allowed disabled:opacity-50"
  >
    <div class="flex items-start gap-3">
      <span
        class="mt-1 h-8 w-1 shrink-0 rounded-full bg-(--color-system-accent)"
        aria-hidden="true"
      />
      <div class="min-w-0 flex-1">
        <h4 class="truncate text-sm font-semibold text-(--color-oa-ink)">{props.title}</h4>
        <Show when={props.subtitle}>
          <p class="text-[0.65rem] text-(--color-oa-ink-dim)">{props.subtitle}</p>
        </Show>
      </div>
    </div>
    {props.children}
  </button>
);
