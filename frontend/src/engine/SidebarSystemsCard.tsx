// Sidebar-systems visibility card — lifted out of LibraryManagerPage's
// Library tab during the Settings IA redesign (Slice 1). This is *organization*
// (what shows in the left sidebar), not file management, so it lives under the
// new Organize landing rather than Library. Self-contained via usePlatform()
// (engine ↛ theme): reads layout / views / library stores directly.

import { For, Show, type Component } from "solid-js";
import { usePlatform } from "@oa/platform/platformContext";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import { collectHiddenContainers, findNode } from "@oa/platform/views/resolver";
import { platformNodeIdFor } from "@oa/platform/views/defaults";

export const SidebarSystemsCard: Component = () => {
  const platform = usePlatform();
  const layout = platform.layout;
  const views = platform.views;
  const library = platform.library;

  return (
    <div class="space-y-2">
      <h3 class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
        Sidebar systems
      </h3>
      <label class="flex items-center gap-2 text-xs text-(--color-oa-ink)">
        <input
          type="checkbox"
          checked={layout.autoHideEmptySystems()}
          onChange={(e) => layout.setAutoHideEmptySystems(e.currentTarget.checked)}
        />
        <span>Auto-hide systems with no games</span>
      </label>

      {/* Hidden containers — operator can right-click a container in the
          sidebar tree to hide it; this surfaces the un-hide affordance so it's
          not a one-way trip. Walks the active view at any depth. */}
      <Show when={collectHiddenContainers(views.activeView()?.root ?? {
        id: "_empty", label: "", rule: null, accent: null, art: null, hidden: false, children: []
      }).length > 0}>
        <div class="mt-4 space-y-1">
          <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            Hidden containers
          </p>
          <ul class="space-y-1">
            <For each={collectHiddenContainers(views.activeView()?.root ?? {
              id: "_empty", label: "", rule: null, accent: null, art: null, hidden: false, children: []
            })}>
              {(container) => (
                <li class="flex items-center justify-between gap-3 rounded border border-white/5 bg-white/[0.02] px-3 py-2 text-xs">
                  <span class="flex-1 truncate text-(--color-oa-ink-dim)">
                    {container.label}
                  </span>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      views.setNodeHidden(container.id, false);
                    }}
                    class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-(--color-system-accent)/15 hover:text-(--color-oa-ink)"
                  >
                    Show
                  </button>
                </li>
              )}
            </For>
          </ul>
        </div>
      </Show>
      <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
        Uncheck a system to hide it from the left sidebar. Hidden systems still live
        in the registry; per-system files (bindings, settings) are preserved.
      </p>
      <ul class="space-y-1">
        <For each={Object.keys(systemThemes) as SystemId[]}>
          {(id) => {
            const theme = systemThemes[id];
            const count = () =>
              library.state.entries.filter((e) => e.systemId === id && !e.seed).length;
            /// Source of truth is the active view's per-leaf `hidden` flag; the
            /// legacy flat `layout.hiddenSystems` set is checked as a fallback so
            /// systems not present in the active view's tree still honor the
            /// operator's hide intent. Writes go to both during the migration.
            const hidden = () => {
              const active = views.activeView();
              if (active) {
                const node = findNode(active, platformNodeIdFor(id));
                if (node && "kind" in node && node.kind === "platform" && node.hidden) {
                  return true;
                }
              }
              return layout.hiddenSystems().includes(id);
            };
            return (
              <li class="flex items-center gap-3 rounded border border-white/5 bg-white/[0.02] px-3 py-2 text-xs">
                <input
                  type="checkbox"
                  checked={!hidden()}
                  onChange={(e) => {
                    const list = layout.hiddenSystems();
                    const show = e.currentTarget.checked;
                    if (show) {
                      layout.setHiddenSystems(list.filter((s) => s !== id));
                    } else if (!list.includes(id)) {
                      layout.setHiddenSystems([...list, id]);
                    }
                    views.setNodeHidden(platformNodeIdFor(id), !show);
                  }}
                />
                <span class="flex-1 truncate text-(--color-oa-ink)">{theme.displayName}</span>
                <span class="text-(--color-oa-ink-dim) tabular-nums">{count()}</span>
              </li>
            );
          }}
        </For>
      </ul>
    </div>
  );
};

export default SidebarSystemsCard;
