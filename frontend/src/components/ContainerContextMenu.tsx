import { onCleanup, onMount, Show, type Component } from "solid-js";

import type { RomEntry } from "../library/types";
import type { ContainerNode } from "../views/types";
import { countGamesUnder } from "../views/resolver";

type Props = {
  container: ContainerNode | null;
  position: { x: number; y: number } | null;
  entries: RomEntry[];
  onClose: () => void;
  /// Hide this container (and all its descendants implicitly via the
  /// per-node hidden flag) from the sidebar tree. Operator can re-show
  /// from Settings → Library → Sidebar systems (container-level un-
  /// hide UI lands with the v3 View Editor; for now the operator
  /// rebuilds the default view by deleting views.json).
  onHide: (containerId: string) => void;
};

const ITEM_CLASS =
  "flex w-full items-center justify-between gap-3 px-3 py-1.5 text-left text-sm text-(--color-oa-ink) hover:bg-white/[0.06]";

/// Right-click context menu for a container row in the sidebar tree.
/// Carries a single "Hide from sidebar" item in v1 per SIDEBAR_TIER_PLAN
/// §3.6 (full container CRUD lands with the v3 View Editor).
const ContainerContextMenu: Component<Props> = (props) => {
  const totalCount = () => {
    const c = props.container;
    if (!c) return 0;
    return countGamesUnder(c, props.entries);
  };

  function closeAfter<T>(fn: () => T): T {
    const r = fn();
    props.onClose();
    return r;
  }

  function hide() {
    if (!props.container) return;
    closeAfter(() => props.onHide(props.container!.id));
  }

  function onWindowKey(e: KeyboardEvent) {
    if (e.key === "Escape") props.onClose();
  }
  function onWindowClick(e: MouseEvent) {
    const target = e.target as HTMLElement | null;
    if (!target || !target.closest("[data-container-context-root]")) {
      props.onClose();
    }
  }
  onMount(() => {
    window.addEventListener("keydown", onWindowKey, true);
    window.addEventListener("mousedown", onWindowClick, true);
  });
  onCleanup(() => {
    window.removeEventListener("keydown", onWindowKey, true);
    window.removeEventListener("mousedown", onWindowClick, true);
  });

  return (
    <Show when={props.container && props.position} keyed>
      {(_) => {
        const pos = props.position!;
        return (
          <div
            data-container-context-root
            class="fixed z-50 min-w-[14rem] overflow-hidden rounded-md border border-white/10 bg-(--color-oa-bg-deep)/95 text-sm shadow-2xl shadow-black/60 backdrop-blur"
            style={{ left: `${pos.x}px`, top: `${pos.y}px` }}
            onClick={(e) => e.stopPropagation()}
          >
            <div class="border-b border-white/5 px-3 py-2">
              <p class="truncate text-xs font-medium text-(--color-oa-ink)">
                {props.container!.label}
              </p>
              <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                {totalCount()} {totalCount() === 1 ? "game" : "games"}
              </p>
            </div>
            <ul class="py-1">
              <li>
                <button type="button" class={ITEM_CLASS} onClick={hide}>
                  <span>Hide from sidebar</span>
                  <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                    👁
                  </span>
                </button>
              </li>
            </ul>
          </div>
        );
      }}
    </Show>
  );
};

export default ContainerContextMenu;
