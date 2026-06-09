import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show, type Component } from "solid-js";
import { open as pickFile } from "@tauri-apps/plugin-dialog";
import type { LibraryStore } from "@oa/platform/library/store";
import type { CustomCollectionsStore } from "@oa/platform/library/customCollections";
import type { RomEntry, VariantInfo } from "@oa/platform/library/types";
import { useMedia } from "@oa/platform/library/media";
import { captureFocusReturn, useFocusGroup } from "../../nav/focus";
import { useBackHandler } from "../../nav/back";
import { HintRegion } from "../../nav/HintBar";

type Props = {
  entry: RomEntry | null;
  position: { x: number; y: number } | null;
  library: LibraryStore;
  /// Slice 12 — Retroverse custom collections. The "Add to collection ▸"
  /// submenu reads + mutates this store. Optional so the legacy /
  /// non-Retroverse code paths can opt out by not passing it; absent
  /// store hides the menu entry.
  customCollections?: CustomCollectionsStore;
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
  /// Open the per-game Input dialog (libretro device-type select +
  /// keypad/lightgun reference + per-port wiring).
  onOpenInput: (entry: RomEntry) => void;
  /// Open the per-game Core options dialog (libretro core option
  /// curation per game — region, speedhack, BIOS-skip, etc.).
  onOpenCoreOptions: (entry: RomEntry) => void;
  /// Open the per-game Screenshot gallery dialog (F12 captures viewer).
  onOpenScreenshots: (entry: RomEntry) => void;
  /// Open the per-game Display dialog (display aspect override —
  /// e.g. 16:9 for OutRun on a 4:3-default system).
  onOpenDisplay: (entry: RomEntry) => void;
  /// Open the per-game Shaders dialog (shader preset override per
  /// game — e.g. CRT-bezel for SF2 but plain for Tetris).
  onOpenShaders: (entry: RomEntry) => void;
  /// Open the per-game Cheats dialog (cheat code entry / toggle
  /// per game — Game Genie, Pro Action Replay codes, etc.).
  onOpenCheats: (entry: RomEntry) => void;
  /// Open the per-game Rewind settings dialog (rewind enable /
  /// capture interval / buffer size overrides — distinct from the
  /// QuickSettings in-game rewind scrubber).
  onOpenRewind: (entry: RomEntry) => void;
  /// Open the per-game Milestones dialog (memory-watcher completion
  /// trigger curation — set per-game checkpoints that fire when
  /// the watched memory region matches a target).
  onOpenMilestones: (entry: RomEntry) => void;
  /// Slice 12 — open the NewCollectionDialog seeded with this rom.
  /// Wired by App.tsx when customCollections is in play. Absent =
  /// "+ New collection…" tail entry in the submenu is hidden.
  onOpenNewCollection?: (romId: string) => void;
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
  /// Slice 12 — leading checkmark column for the Add-to-collection
  /// submenu rows so the operator can see at a glance which lists a
  /// game is already in.
  leadingGlyph?: string;
  onActivate: () => void;
  /// Optional secondary action — currently only used by the per-variant
  /// rows to expose the pin/unpin star (X on controller).
  onSecondary?: () => void;
  secondaryGlyph?: string;
  secondaryTitle?: string;
};

/// Slice 12 — top-level vs Add-to-collection sub-view. Mirrors
/// SystemContextMenu's main / move-category pattern. The list itself
/// is rebuilt per view (no nesting through `<Show>`) so the focus
/// group's index → action mapping stays consistent.
///
/// 2026-06-01 — added `per-game-settings` sub-view collapsing the
/// seven Phase D dialog rows (Input / Core options / Display /
/// Shaders / Cheats / Rewind / Milestones) behind a single top-level
/// entry. Same precedent as Add-to-collection: back navigation in the
/// sub-view returns to main rather than closing the menu.
type MenuView = "main" | "add-to-collection" | "per-game-settings";

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

  function openInput() {
    if (!props.entry) return;
    closeAfter(() => props.onOpenInput(props.entry!));
  }

  function openCoreOptions() {
    if (!props.entry) return;
    closeAfter(() => props.onOpenCoreOptions(props.entry!));
  }

  function openScreenshots() {
    if (!props.entry) return;
    closeAfter(() => props.onOpenScreenshots(props.entry!));
  }

  function openDisplay() {
    if (!props.entry) return;
    closeAfter(() => props.onOpenDisplay(props.entry!));
  }

  function openShaders() {
    if (!props.entry) return;
    closeAfter(() => props.onOpenShaders(props.entry!));
  }

  function openCheats() {
    if (!props.entry) return;
    closeAfter(() => props.onOpenCheats(props.entry!));
  }

  function openRewind() {
    if (!props.entry) return;
    closeAfter(() => props.onOpenRewind(props.entry!));
  }

  function openMilestones() {
    if (!props.entry) return;
    closeAfter(() => props.onOpenMilestones(props.entry!));
  }

  function removeFromLibrary() {
    if (!props.entry) return;
    closeAfter(() => void props.library.remove(props.entry!.id));
  }

  /// Retroverse-UI Phase C3 — flip favorite. Closes the menu after firing
  /// so the result is immediately visible (heart fill flips, smart-list
  /// count updates). Library store handles optimistic update + Rust
  /// persistence + revert-on-failure.
  function toggleFavorite() {
    if (!props.entry) return;
    const next = !props.entry.favorite;
    const id = props.entry.id;
    closeAfter(() => void props.library.setFavorite(id, next));
  }

  /// Retroverse-UI Phase C3 — flip completed. Same shape as toggleFavorite.
  function toggleCompleted() {
    if (!props.entry) return;
    const next = !props.entry.completed;
    const id = props.entry.id;
    closeAfter(() => void props.library.setCompleted(id, next));
  }

  // Slice 12 — Add to collection ▸ sub-view state. Only mounts while
  // the menu is open (the outer keyed Show body re-runs on every
  // open). Reset to "main" on close so a re-open lands on the top
  // level even if the operator was deep in the sub-view.
  const [menuView, setMenuView] = createSignal<MenuView>("main");

  /// Slice 12 — toggle collection membership. Optimistic via the store;
  /// after firing we stay in the sub-view so the operator can drop the
  /// game into multiple lists in one menu opening. B (or clicking
  /// "← Back") returns to the main view.
  async function toggleCollectionMembership(collectionId: string) {
    if (!props.customCollections || !props.entry) return;
    const romId = props.entry.id;
    const isMember = props.customCollections.state.members[collectionId]?.has(romId) === true;
    if (isMember) {
      await props.customCollections.removeFromCollection(collectionId, romId);
    } else {
      await props.customCollections.addToCollection(collectionId, romId);
    }
  }

  function openNewCollectionFromMenu() {
    if (!props.onOpenNewCollection || !props.entry) return;
    const romId = props.entry.id;
    closeAfter(() => props.onOpenNewCollection!(romId));
  }

  /// Flat ordered item list. Mirrors the rendered order; conditional
  /// sections fold in via if-guards rather than `<Show>` so the focus
  /// group's index → action mapping stays consistent. Returns empty
  /// when the menu is closed so the focus group's itemCount stays at
  /// 0 and the manager won't dispatch nav events into the void.
  const items = createMemo<MenuItem[]>(() => {
    if (!props.entry) return [];
    // Slice 12 — Add to collection sub-view renders an entirely
    // different list. Top "← Back" returns to main, then one row per
    // existing collection (checkmark if the game is already in it),
    // then a trailing "+ New collection…" entry.
    if (menuView() === "add-to-collection") {
      const list: MenuItem[] = [];
      list.push({
        key: "add-back",
        label: "← Back",
        onActivate: () => setMenuView("main"),
      });
      const entry = props.entry;
      const store = props.customCollections;
      if (store) {
        for (const col of store.state.collections) {
          const isMember = store.state.members[col.id]?.has(entry.id) === true;
          list.push({
            key: `add-${col.id}`,
            label: col.name,
            leadingGlyph: isMember ? "✓" : "○",
            badge: String(col.memberCount),
            onActivate: () => void toggleCollectionMembership(col.id),
          });
        }
      }
      if (props.onOpenNewCollection) {
        list.push({
          key: "add-new",
          label: "+ New collection…",
          badgeAccent: true,
          onActivate: openNewCollectionFromMenu,
        });
      }
      return list;
    }

    // Per-game settings sub-view — the seven Phase D split dialogs
    // (Input / Core options / Display / Shaders / Cheats / Rewind /
    // Milestones). Top-level shows a single "Per-game settings ▸"
    // entry that transitions here. B / Esc returns to main.
    if (menuView() === "per-game-settings") {
      const list: MenuItem[] = [];
      list.push({
        key: "settings-back",
        label: "← Back",
        onActivate: () => setMenuView("main"),
      });
      list.push({ key: "input", label: "Input mapping…", onActivate: openInput });
      list.push({ key: "core-options", label: "Core options…", onActivate: openCoreOptions });
      list.push({ key: "display", label: "Display…", onActivate: openDisplay });
      list.push({ key: "shaders", label: "Shaders…", onActivate: openShaders });
      list.push({ key: "cheats", label: "Cheats…", onActivate: openCheats });
      list.push({ key: "rewind", label: "Rewind settings…", onActivate: openRewind });
      list.push({ key: "milestones", label: "Milestones…", onActivate: openMilestones });
      list.push({ key: "screenshots", label: "Screenshots…", onActivate: openScreenshots });
      return list;
    }

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
    // Retroverse-UI Phase C3 — Favorite + Mark completed flags drive the
    // COLLECTIONS smart-lists. Available from this menu in both legacy +
    // Retroverse UIs (the same context menu is reused in both).
    list.push({
      key: "favorite",
      label: props.entry?.favorite ? "Remove from favorites" : "Add to favorites",
      onActivate: toggleFavorite,
    });
    list.push({
      key: "completed",
      label: props.entry?.completed ? "Mark as not completed" : "Mark as completed",
      onActivate: toggleCompleted,
    });
    // Slice 12 — Add to collection ▸ submenu entry. Only shown when
    // the customCollections store is plumbed (Retroverse mode); the
    // legacy-mode shell doesn't surface custom collections so the
    // entry is hidden to avoid a dead end.
    if (props.customCollections) {
      const count = props.customCollections.state.collections.length;
      list.push({
        key: "add-to-collection",
        label: "Add to collection",
        badge: count > 0 ? `${count} ›` : "›",
        badgeAccent: true,
        onActivate: () => setMenuView("add-to-collection"),
      });
    }
    list.push({
      key: "core",
      label: "Change core…",
      badge: props.entry?.coreOverride ? "override" : undefined,
      badgeAccent: true,
      onActivate: changeCore,
    });
    list.push({
      key: "per-game-settings",
      label: "Per-game settings",
      badge: "›",
      badgeAccent: true,
      onActivate: () => setMenuView("per-game-settings"),
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
    onCancel: () => {
      // B in any sub-view returns to the top level instead of closing
      // the whole menu — mirrors the unified spec operators have for
      // sub-region menus.
      if (menuView() !== "main") {
        setMenuView("main");
        return;
      }
      props.onClose();
    },
  });

  // Reset focus to the first row whenever the sub-view changes so the
  // operator lands on "← Back" / "Launch" rather than wherever the
  // previous view's cursor was.
  createEffect(() => {
    void menuView();
    setFocusedIndex(0);
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
        // close. captureFocusReturn snapshots whichever group was active
        // BEFORE we activate ours, so close-handlers return focus to
        // that surface across all Retroverse tabs (not just LIBRARY).
        useBackHandler(() => {
          // Same sub-view → main step-back as onCancel above.
          if (menuView() === "add-to-collection") {
            setMenuView("main");
            return;
          }
          props.onClose();
        });
        const restoreFocus = captureFocusReturn();
        onMount(() => {
          onMenuMount();
          setMenuView("main");
          setFocusedIndex(0);
        });
        onCleanup(restoreFocus);
        return (
          <div
            data-tile-context-root
            class="fixed z-50 min-w-[16rem] overflow-hidden rounded-md border border-white/10 bg-(--color-oa-bg-deep)/95 text-sm shadow-2xl shadow-black/60 backdrop-blur"
            style={{ left: `${pos.x}px`, top: `${pos.y}px` }}
            onClick={(e) => e.stopPropagation()}
            data-system={props.entry!.systemId}
          >
            <HintRegion hints={{
              a: menuView() === "add-to-collection" ? "Toggle" : "Activate",
              b: menuView() === "add-to-collection" ? "Back" : "Close",
              x: "Pin/Unpin",
            }} />
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
                      <Show when={item.leadingGlyph}>
                        <span
                          class="w-3 shrink-0 text-center text-[0.7rem]"
                          classList={{
                            "text-(--color-system-accent)": item.leadingGlyph === "✓",
                            "text-(--color-oa-ink-dim)": item.leadingGlyph !== "✓",
                          }}
                          aria-hidden="true"
                        >
                          {item.leadingGlyph}
                        </span>
                      </Show>
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
