import { createMemo, createSignal, For, onCleanup, onMount, Show, type Component } from "solid-js";
import { open as pickFile } from "@tauri-apps/plugin-dialog";
import type { LibraryStore } from "../library/store";
import type { RomEntry, VariantInfo } from "../library/types";
import { useMedia } from "../library/media";
import { activateFocusGroup, useFocusGroup } from "../nav/focus";
import { useBackHandler } from "../nav/back";
import { HintRegion } from "../nav/HintBar";

type Props = {
  entry: RomEntry | null;
  position: { x: number; y: number } | null;
  library: LibraryStore;
  onClose: () => void;
  /// Re-launch the ROM (same as left-click on the tile).
  onLaunch: (entry: RomEntry) => void;
  /// Open the SaveSlotsModal for this game.
  onShowSaves: (entry: RomEntry) => void;
  /// Open the GameInfoModal — hero artwork + metadata + screenshots + saves.
  onShowGameInfo: (entry: RomEntry) => void;
  /// Open the RegionPicker modal — only meaningful when the game has ≥2 boxart variants.
  onPickRegion: (entry: RomEntry) => void;
  /// Open the CorePickerMenu anchored at the same coordinates.
  onPickCore: (entry: RomEntry, position: { x: number; y: number }) => void;
  /// Open the per-game settings drawer (Phase 2.8 slice D).
  onOpenProperties: (entry: RomEntry) => void;
};

type MenuItem = {
  /// Unique key inside this render pass (used for keying the For loop).
  key: string;
  label: string;
  /// Trailing badge text (right side) — e.g. "Enter" shortcut, override
  /// flag, variant count.
  badge?: string;
  badgeAccent?: boolean;
  disabled?: boolean;
  /// Danger styling for destructive actions ("Remove from library").
  danger?: boolean;
  onActivate: () => void;
  /// Optional secondary action — currently only used by the per-variant
  /// rows to expose the pin/unpin star (X on controller).
  onSecondary?: () => void;
  secondaryGlyph?: string;
  secondaryTitle?: string;
};

const ITEM_CLASS =
  "flex w-full items-center justify-between gap-3 px-3 py-1.5 text-left text-sm text-(--color-oa-ink) hover:bg-white/[0.06] disabled:cursor-default disabled:text-(--color-oa-ink-dim) disabled:hover:bg-transparent";

const FOCUS_RING_CLASS = "";

/// Right-click context menu for a library tile. Unifies cover overrides,
/// saves, core selection, and library removal under one popover. Sections
/// (Launch / Cover / Saves / Core / Remove) match LaunchBox's per-game menu.
const TileContextMenu: Component<Props> = (props) => {
  const media = useMedia();

  // Boxart variants (region / front / back image picks) — *distinct*
  // from the multi-region game-file variants below.
  const coverVariants = createMemo(() => {
    const e = props.entry;
    if (!e) return [];
    return media.media(e.id)?.boxart ?? [];
  });
  const hasCover = createMemo(() => coverVariants().length > 0);
  const hasMultipleVariants = createMemo(() => coverVariants().length >= 2);

  // Multi-file game-version group (different regions / revisions of the
  // same underlying title). Absent for single-file games.
  const gameGroup = createMemo(() => {
    const e = props.entry;
    if (!e) return null;
    return props.library.groupsByVariantId().get(e.id) ?? null;
  });
  const gameVariants = createMemo<VariantInfo[]>(() => gameGroup()?.variants ?? []);
  const hasGameVariants = createMemo(() => gameVariants().length >= 2);
  const entriesById = createMemo(() => {
    const map = new Map<string, RomEntry>();
    for (const e of props.library.state.entries) {
      map.set(e.id, e);
    }
    return map;
  });

  function launchVariant(v: VariantInfo) {
    const target = entriesById().get(v.id);
    if (!target) return;
    closeAfter(() => props.onLaunch(target));
  }
  async function pinAsDefault(v: VariantInfo) {
    const group = gameGroup();
    if (!group) return;
    await props.library.setGroupDefault(group.systemId, group.displayBaseTitle, v.id);
    props.onClose();
  }
  async function clearDefault() {
    const group = gameGroup();
    if (!group) return;
    await props.library.clearGroupDefault(group.systemId, group.displayBaseTitle);
    props.onClose();
  }

  function variantLabel(v: VariantInfo): string {
    const parts: string[] = [];
    if (v.region) parts.push(v.region);
    if (v.revision > 0) parts.push(`Rev ${v.revision}`);
    if (v.isPrerelease) parts.push("β");
    return parts.length > 0 ? parts.join(" · ") : "Release";
  }

  function closeAfter<T>(fn: () => T): T {
    const r = fn();
    props.onClose();
    return r;
  }

  function launch() {
    if (!props.entry) return;
    closeAfter(() => props.onLaunch(props.entry!));
  }

  async function pickCoverFile() {
    const entry = props.entry;
    if (!entry) return;
    let picked: string | string[] | null = null;
    try {
      picked = await pickFile({
        multiple: false,
        directory: false,
        filters: [{ name: "Image", extensions: ["png", "jpg", "jpeg", "webp"] }],
      });
    } catch (e) {
      console.warn("pickFile failed:", e);
      props.onClose();
      return;
    }
    if (!picked || Array.isArray(picked)) {
      props.onClose();
      return;
    }
    try {
      await media.setManualCover(entry.id, entry.systemId, picked);
    } catch (e) {
      console.warn("setManualCover failed:", e);
    }
    props.onClose();
  }

  function pickRegion() {
    if (!props.entry) return;
    closeAfter(() => props.onPickRegion(props.entry!));
  }

  async function clearCover() {
    if (!props.entry) return;
    try {
      await media.clearMedia(props.entry.id);
    } catch (e) {
      console.warn("clearMedia failed:", e);
    }
    props.onClose();
  }

  function showSaves() {
    if (!props.entry) return;
    closeAfter(() => props.onShowSaves(props.entry!));
  }

  function showGameInfo() {
    if (!props.entry) return;
    closeAfter(() => props.onShowGameInfo(props.entry!));
  }

  function changeCore() {
    if (!props.entry || !props.position) return;
    closeAfter(() => props.onPickCore(props.entry!, props.position!));
  }

  function openProperties() {
    if (!props.entry) return;
    closeAfter(() => props.onOpenProperties(props.entry!));
  }

  function removeFromLibrary() {
    if (!props.entry) return;
    closeAfter(() => void props.library.remove(props.entry!.id));
  }

  /// Flat ordered item list. Mirrors the rendered order; conditional
  /// sections fold in via if-guards rather than `<Show>` so the focus
  /// group's index → action mapping stays consistent. Returns empty
  /// when the menu is closed so the focus group's itemCount stays at
  /// 0 and the manager won't dispatch nav events into the void.
  const items = createMemo<MenuItem[]>(() => {
    if (!props.entry) return [];
    const list: MenuItem[] = [];
    list.push({ key: "launch", label: "Launch", badge: "Enter", onActivate: launch });
    if (hasGameVariants()) {
      for (const v of gameVariants()) {
        list.push({
          key: `variant-${v.id}`,
          label: `${variantLabel(v)}${v.isDefault ? "  ★ default" : ""}`,
          onActivate: () => launchVariant(v),
          onSecondary: () => (v.isDefault ? clearDefault() : pinAsDefault(v)),
          secondaryGlyph: v.isDefault ? "★" : "☆",
          secondaryTitle: v.isDefault ? "Clear pinned default" : "Pin as default",
        });
      }
    }
    list.push({ key: "pick-cover", label: "Pick cover file…", onActivate: pickCoverFile });
    if (hasMultipleVariants()) {
      list.push({
        key: "pick-region",
        label: "Pick region…",
        badge: `${coverVariants().length} variants`,
        badgeAccent: true,
        onActivate: pickRegion,
      });
    }
    list.push({
      key: "clear-cover",
      label: "Clear cover",
      disabled: !hasCover(),
      onActivate: clearCover,
    });
    list.push({ key: "saves", label: "Save states…", onActivate: showSaves });
    list.push({ key: "info", label: "Game info…", onActivate: showGameInfo });
    list.push({
      key: "core",
      label: "Change core…",
      badge: props.entry?.coreOverride ? "override" : undefined,
      badgeAccent: true,
      onActivate: changeCore,
    });
    list.push({ key: "props", label: "Game properties…", onActivate: openProperties });
    list.push({
      key: "remove",
      label: "Remove from library",
      danger: true,
      onActivate: removeFromLibrary,
    });
    return list;
  });

  const [focusedIndex, setFocusedIndex] = createSignal(0);
  const focusGroup = useFocusGroup({
    id: "tile-context-menu",
    orientation: "vertical",
    itemCount: () => items().length,
    focusedIndex,
    setFocusedIndex,
    onActivate: (i) => {
      const it = items()[i];
      if (it && !it.disabled) it.onActivate();
    },
    onSecondary: (i) => {
      const it = items()[i];
      if (it && !it.disabled) it.onSecondary?.();
    },
    onCancel: () => props.onClose(),
  });

  // Activate the menu's focus group on mount + register as the back
  // handler so B closes the menu. Order matters: useBackHandler must
  // appear after props.entry settles so the inner Show's mount drives
  // it — done by the outer `keyed` Show.
  function onMenuMount(): void {
    focusGroup.activate();
  }

  function onWindowKey(e: KeyboardEvent) {
    if (e.key === "Escape") props.onClose();
  }
  function onWindowClick(e: MouseEvent) {
    const target = e.target as HTMLElement | null;
    if (!target || !target.closest("[data-tile-context-root]")) {
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
    <Show when={props.entry && props.position} keyed>
      {(_) => {
        const pos = props.position!;
        // Mount-scoped hooks: this branch only exists while the menu is
        // open, so the back handler + focus activation auto-clean on
        // close. The cleanup explicitly transfers activation back to
        // the library grid so a B-press doesn't strand focus in this
        // (now-itemless) group.
        useBackHandler(() => props.onClose());
        onMount(() => {
          onMenuMount();
          setFocusedIndex(0);
        });
        onCleanup(() => activateFocusGroup("library-grid"));
        return (
          <div
            data-tile-context-root
            class="fixed z-50 min-w-[16rem] overflow-hidden rounded-md border border-white/10 bg-(--color-oa-bg-deep)/95 text-sm shadow-2xl shadow-black/60 backdrop-blur"
            style={{ left: `${pos.x}px`, top: `${pos.y}px` }}
            onClick={(e) => e.stopPropagation()}
            data-system={props.entry!.systemId}
          >
            <HintRegion hints={{ a: "Activate", b: "Close", x: "Pin/Unpin" }} />
            <div class="border-b border-white/5 px-3 py-2">
              <p class="truncate text-xs font-medium text-(--color-oa-ink)">{props.entry!.title}</p>
              <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                Tile actions
              </p>
            </div>
            <ul class="py-1">
              <For each={items()}>
                {(item, index) => (
                  <li
                    ref={(el) => focusGroup.bind(index(), el)}
                    data-oa-focus={focusedIndex() === index() ? "true" : undefined}
                    data-oa-focus-active={focusGroup.isActive() ? "true" : undefined}
                    classList={{ [FOCUS_RING_CLASS]: true }}
                    class="flex items-center"
                  >
                    <button
                      type="button"
                      class={ITEM_CLASS}
                      classList={{
                        "text-(--color-oa-ink-dim) hover:bg-red-500/10 hover:text-(--color-oa-ink)": item.danger === true,
                      }}
                      disabled={item.disabled}
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
                    <Show when={item.onSecondary}>
                      <button
                        type="button"
                        class="mr-1 rounded px-2 py-1 text-xs text-(--color-oa-ink-dim) hover:bg-white/[0.06] hover:text-(--color-system-accent)"
                        onClick={item.onSecondary}
                        title={item.secondaryTitle}
                        aria-label={item.secondaryTitle}
                      >
                        {item.secondaryGlyph}
                      </button>
                    </Show>
                  </li>
                )}
              </For>
            </ul>
          </div>
        );
      }}
    </Show>
  );
};

export default TileContextMenu;
