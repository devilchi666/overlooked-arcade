import { createMemo, createSignal, For, onCleanup, onMount, Show, type Component } from "solid-js";
import type { LibraryStore } from "../library/store";
import { systemThemes, type SystemId } from "../themes/registry";
import { activateFocusGroup, useFocusGroup } from "../nav/focus";
import { useBackHandler } from "../nav/back";
import { HintRegion } from "../nav/HintBar";

export type MoveTarget = {
  /// The container's node id in the active view (e.g. "container:handheld").
  id: string;
  /// Human-readable label rendered in the submenu (e.g. "Handhelds").
  label: string;
};

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
  /// Containers in the active view that this system isn't already in —
  /// the "Move to category…" submenu lists these. Empty array (or
  /// missing) hides the submenu entry entirely (covers the
  /// single-container case + the synth-leaf case + custom-view edge
  /// cases). PR-v2.2.2.
  moveTargets?: MoveTarget[];
  /// Move the system's leaf into the chosen container in the active
  /// view. The leaf is appended to the container's children (operator
  /// can drag-reorder afterward for a specific position).
  onMoveToContainer?: (containerId: string) => void;
};

type MenuItem = {
  key: string;
  label: string;
  badge?: string;
  badgeAccent?: boolean;
  onActivate: () => void;
};

const ITEM_CLASS =
  "flex w-full items-center justify-between gap-3 px-3 py-1.5 text-left text-sm text-(--color-oa-ink) hover:bg-white/[0.06]";

type MenuView = "main" | "move-category";

const SystemContextMenu: Component<Props> = (props) => {
  const theme = () => (props.systemId ? systemThemes[props.systemId] : null);
  const gameCount = () => {
    const id = props.systemId;
    if (!id) return 0;
    return props.library.state.entries.filter((e) => e.systemId === id && !e.seed).length;
  };
  const [menuView, setMenuView] = createSignal<MenuView>("main");
  const showMoveItem = () => (props.moveTargets?.length ?? 0) > 0;

  function closeAfter<T>(fn: () => T): T {
    const r = fn();
    setMenuView("main");
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

  function moveToContainer(containerId: string) {
    if (!props.systemId || !props.onMoveToContainer) return;
    closeAfter(() => props.onMoveToContainer!(containerId));
  }

  /// Flat item list for the current sub-view. Empty when the menu is
  /// closed (so the focus group ignores events).
  const items = createMemo<MenuItem[]>(() => {
    if (!props.systemId) return [];
    if (menuView() === "move-category") {
      const list: MenuItem[] = [];
      list.push({ key: "back", label: "← Back", badge: "Move", onActivate: () => setMenuView("main") });
      for (const t of props.moveTargets ?? []) {
        list.push({ key: `move-${t.id}`, label: t.label, onActivate: () => moveToContainer(t.id) });
      }
      return list;
    }
    const list: MenuItem[] = [];
    list.push({ key: "show", label: "Show library", badge: "Click", onActivate: showLibrary });
    list.push({ key: "bindings", label: "Edit bindings…", badge: "⌨", onActivate: openBindings });
    list.push({ key: "settings", label: "System settings…", badge: "⚙", badgeAccent: true, onActivate: openSettings });
    if (showMoveItem()) {
      list.push({ key: "move", label: "Move to category…", badge: "›", onActivate: () => setMenuView("move-category") });
    }
    list.push({ key: "hide", label: "Hide from sidebar", badge: "👁", onActivate: hideSystem });
    return list;
  });

  const [focusedIndex, setFocusedIndex] = createSignal(0);
  const focusGroup = useFocusGroup({
    id: "system-context-menu",
    orientation: "vertical",
    itemCount: () => items().length,
    focusedIndex,
    setFocusedIndex,
    onActivate: (i) => items()[i]?.onActivate(),
    onCancel: () => {
      // In sub-views, B steps back to main. Otherwise close the menu.
      if (menuView() === "move-category") {
        setMenuView("main");
        return;
      }
      props.onClose();
    },
  });

  // Reset focus when switching sub-views so the operator lands on the
  // first item rather than wherever they were in the previous list.
  function resetFocusOnViewChange(): void {
    setFocusedIndex(0);
  }

  function onWindowKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (menuView() === "move-category") {
        setMenuView("main");
        return;
      }
      props.onClose();
    }
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
        useBackHandler(() => {
          if (menuView() === "move-category") {
            setMenuView("main");
            return;
          }
          props.onClose();
        });
        onMount(() => {
          focusGroup.activate();
          setFocusedIndex(0);
        });
        onCleanup(() => activateFocusGroup("left-sidebar"));
        return (
          <div
            data-system-context-root
            class="fixed z-50 min-w-[14rem] overflow-hidden rounded-md border border-white/10 bg-(--color-oa-bg-deep)/95 text-sm shadow-2xl shadow-black/60 backdrop-blur"
            style={{ left: `${pos.x}px`, top: `${pos.y}px` }}
            onClick={(e) => e.stopPropagation()}
            data-system={props.systemId!}
          >
            <HintRegion hints={{
              a: "Activate",
              b: menuView() === "move-category" ? "Back" : "Close",
            }} />
            <div class="border-b border-white/5 px-3 py-2">
              <p class="truncate text-xs font-medium text-(--color-oa-ink)">
                {theme()?.displayName ?? props.systemId}
              </p>
              <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                {gameCount()} {gameCount() === 1 ? "game" : "games"}
              </p>
            </div>
            {/* Re-render the list when menuView flips so focus resets to 0. */}
            <ul class="py-1">
              <For each={items()}>
                {(item, index) => {
                  // Reactive cue: re-key item rows so the bind() refs
                  // re-fire when the sub-view changes.
                  void menuView();
                  void resetFocusOnViewChange;
                  return (
                    <li
                      ref={(el) => focusGroup.bind(index(), el)}
                      data-oa-focus={focusedIndex() === index() ? "true" : undefined}
                      data-oa-focus-active={focusGroup.isActive() ? "true" : undefined}
                    >
                      <button
                        type="button"
                        class={ITEM_CLASS}
                        onMouseEnter={() => setFocusedIndex(index())}
                        onClick={item.onActivate}
                      >
                        <span class="truncate">{item.label}</span>
                        <Show when={item.badge}>
                          <span
                            class="text-[0.6rem] uppercase tracking-widest"
                            classList={{
                              "text-(--color-system-accent)": item.badgeAccent === true,
                              "text-(--color-oa-ink-dim)": item.badgeAccent !== true,
                            }}
                          >
                            {item.badge}
                          </span>
                        </Show>
                      </button>
                    </li>
                  );
                }}
              </For>
            </ul>
          </div>
        );
      }}
    </Show>
  );
};

export default SystemContextMenu;
