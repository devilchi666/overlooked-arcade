// Standard domain-editor shell for the Systems hub (Pillar-B byproduct): a
// per-system header (accent stripe + name + subtitle) + a scrollable body. No
// data-nav-region of its own (see HubGrid) so UP/DOWN reaches the breadcrumb.
// `HubSection` is the section-card used to group control rows inside an editor.

import { Show, type Component, type JSX } from "solid-js";
import type { SystemId } from "@oa/platform/themes/registry";

export const PanelScaffold: Component<{
  system?: SystemId;
  title: string;
  subtitle?: string;
  /// When true the body is a bounded `flex-1` region with NO outer scroll —
  /// for children that manage their OWN internal scrolling (e.g. the
  /// game-metadata pane's 3-pane layout with a scrolling game list). The
  /// default body adds `overflow-y-auto` for simple top-to-bottom content.
  fill?: boolean;
  children: JSX.Element;
}> = (props) => (
  <div class="flex h-full min-h-0 flex-col gap-4" data-system={props.system}>
    <header class="flex items-center gap-3 rounded-xl border border-white/5 bg-white/[0.02] px-4 py-3">
      <span
        class="inline-block h-3 w-3 shrink-0 rounded-full bg-(--color-system-accent)"
        aria-hidden="true"
      />
      <div class="min-w-0 flex-1">
        <p class="truncate text-base font-semibold text-(--color-oa-ink)">{props.title}</p>
        <Show when={props.subtitle}>
          <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            {props.subtitle}
          </p>
        </Show>
      </div>
    </header>
    <Show
      when={props.fill}
      fallback={
        <div class="flex min-h-0 flex-1 flex-col gap-6 overflow-y-auto pr-1">{props.children}</div>
      }
    >
      <div class="flex min-h-0 flex-1">{props.children}</div>
    </Show>
  </div>
);

/// A titled section card grouping related control rows (mirrors the `Card`
/// previously local to PerSystemSettingsBody).
export const HubSection: Component<{ title: string; children: JSX.Element }> = (props) => (
  <section class="rounded-xl border border-white/5 bg-white/[0.02] p-5">
    <h3 class="mb-4 text-[0.65rem] font-semibold uppercase tracking-[0.3em] text-(--color-system-accent)">
      {props.title}
    </h3>
    {props.children}
  </section>
);
