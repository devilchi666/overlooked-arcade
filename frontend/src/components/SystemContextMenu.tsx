import { onCleanup, onMount, Show, type Component } from "solid-js";
import type { LibraryStore } from "../library/store";
import { systemThemes, type SystemId } from "../themes/registry";

type Props = {
  systemId: SystemId | null;
  position: { x: number; y: number } | null;
  library: LibraryStore;
  onClose: () => void;
  /// Navigate to the system-filtered library (same as left-click on the
  /// sidebar entry — duplicated here for discoverability).
  onShowLibrary: (id: SystemId) => void;
  /// Open the per-system Bindings dialog for this system.
  onOpenBindings: (id: SystemId) => void;
  /// Open the default per-system settings dialog (Display overrides) for
  /// this system. The rest of the per-system surfaces are reachable from
  /// the top-bar System ▾ menu.
  onOpenSettings: (id: SystemId) => void;
  /// Hide this system from the left sidebar. Lives in LayoutPrefs.
  /// hiddenSystems; user can unhide from Settings → Library → Sidebar
  /// systems. Mirrors LaunchBox's "uncheck a platform" UX.
  onHideSystem: (id: SystemId) => void;
};

const ITEM_CLASS =
  "flex w-full items-center justify-between gap-3 px-3 py-1.5 text-left text-sm text-(--color-oa-ink) hover:bg-white/[0.06]";

/// Right-click context menu for a system entry in the left sidebar. Anchored
/// at the click coordinates (passed as `position`). Mirrors the
/// `TileContextMenu` UX — Esc closes, click-outside closes.
const SystemContextMenu: Component<Props> = (props) => {
  const theme = () => (props.systemId ? systemThemes[props.systemId] : null);
  const gameCount = () => {
    const id = props.systemId;
    if (!id) return 0;
    return props.library.state.entries.filter((e) => e.systemId === id && !e.seed).length;
  };

  function closeAfter<T>(fn: () => T): T {
    const r = fn();
    props.onClose();
    return r;
  }

  function showLibrary() {
    if (!props.systemId) return;
    closeAfter(() => props.onShowLibrary(props.systemId!));
  }

  function openBindings() {
    if (!props.systemId) return;
    closeAfter(() => props.onOpenBindings(props.systemId!));
  }

  function openSettings() {
    if (!props.systemId) return;
    closeAfter(() => props.onOpenSettings(props.systemId!));
  }

  function hideSystem() {
    if (!props.systemId) return;
    closeAfter(() => props.onHideSystem(props.systemId!));
  }

  function onWindowKey(e: KeyboardEvent) {
    if (e.key === "Escape") props.onClose();
  }
  function onWindowClick(e: MouseEvent) {
    const target = e.target as HTMLElement | null;
    if (!target || !target.closest("[data-system-context-root]")) {
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
    <Show when={props.systemId && props.position} keyed>
      {(_) => {
        const pos = props.position!;
        return (
          <div
            data-system-context-root
            class="fixed z-50 min-w-[14rem] overflow-hidden rounded-md border border-white/10 bg-(--color-oa-bg-deep)/95 text-sm shadow-2xl shadow-black/60 backdrop-blur"
            style={{ left: `${pos.x}px`, top: `${pos.y}px` }}
            onClick={(e) => e.stopPropagation()}
            data-system={props.systemId!}
          >
            <div class="border-b border-white/5 px-3 py-2">
              <p class="truncate text-xs font-medium text-(--color-oa-ink)">
                {theme()?.displayName ?? props.systemId}
              </p>
              <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                {gameCount()} {gameCount() === 1 ? "game" : "games"}
              </p>
            </div>
            <ul class="py-1">
              <li>
                <button type="button" class={ITEM_CLASS} onClick={showLibrary}>
                  <span>Show library</span>
                  <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                    Click
                  </span>
                </button>
              </li>
              <li>
                <button type="button" class={ITEM_CLASS} onClick={openBindings}>
                  <span>Edit bindings…</span>
                  <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                    ⌨
                  </span>
                </button>
              </li>
              <li>
                <button type="button" class={ITEM_CLASS} onClick={openSettings}>
                  <span>System settings…</span>
                  <span class="text-[0.6rem] uppercase tracking-widest text-(--color-system-accent)">
                    ⚙
                  </span>
                </button>
              </li>
              <li class="border-t border-white/5 mt-1 pt-1">
                <button type="button" class={ITEM_CLASS} onClick={hideSystem}>
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

export default SystemContextMenu;
