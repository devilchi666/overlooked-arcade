import { createMemo, onCleanup, onMount, Show, type Component } from "solid-js";
import { open as pickFile } from "@tauri-apps/plugin-dialog";
import type { LibraryStore } from "../library/store";
import type { RomEntry } from "../library/types";
import { useMedia } from "../library/media";

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

const ITEM_CLASS =
  "flex w-full items-center justify-between gap-3 px-3 py-1.5 text-left text-sm text-(--color-oa-ink) hover:bg-white/[0.06] disabled:cursor-default disabled:text-(--color-oa-ink-dim) disabled:hover:bg-transparent";

/// Right-click context menu for a library tile. Unifies cover overrides,
/// saves, core selection, and library removal under one popover. Sections
/// (Launch / Cover / Saves / Core / Remove) match LaunchBox's per-game menu.
const TileContextMenu: Component<Props> = (props) => {
  const media = useMedia();

  const variants = createMemo(() => {
    const e = props.entry;
    if (!e) return [];
    return media.media(e.id)?.boxart ?? [];
  });
  const hasCover = createMemo(() => variants().length > 0);
  const hasMultipleVariants = createMemo(() => variants().length >= 2);

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
        return (
          <div
            data-tile-context-root
            class="fixed z-50 min-w-[16rem] overflow-hidden rounded-md border border-white/10 bg-(--color-oa-bg-deep)/95 text-sm shadow-2xl shadow-black/60 backdrop-blur"
            style={{ left: `${pos.x}px`, top: `${pos.y}px` }}
            onClick={(e) => e.stopPropagation()}
            data-system={props.entry!.systemId}
          >
            <div class="border-b border-white/5 px-3 py-2">
              <p class="truncate text-xs font-medium text-(--color-oa-ink)">{props.entry!.title}</p>
              <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                Tile actions
              </p>
            </div>
            <ul class="py-1">
              <li>
                <button type="button" class={ITEM_CLASS} onClick={launch}>
                  <span>Launch</span>
                  <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">Enter</span>
                </button>
              </li>
              <li class="my-1 h-px bg-white/5" aria-hidden="true" />
              <li>
                <button type="button" class={ITEM_CLASS} onClick={pickCoverFile}>
                  <span>Pick cover file…</span>
                </button>
              </li>
              <Show when={hasMultipleVariants()}>
                <li>
                  <button type="button" class={ITEM_CLASS} onClick={pickRegion}>
                    <span>Pick region…</span>
                    <span class="text-[0.6rem] uppercase tracking-widest text-(--color-system-accent)">
                      {variants().length} variants
                    </span>
                  </button>
                </li>
              </Show>
              <li>
                <button
                  type="button"
                  class={ITEM_CLASS}
                  onClick={clearCover}
                  disabled={!hasCover()}
                >
                  <span>Clear cover</span>
                </button>
              </li>
              <li class="my-1 h-px bg-white/5" aria-hidden="true" />
              <li>
                <button type="button" class={ITEM_CLASS} onClick={showSaves}>
                  <span>Save states…</span>
                </button>
              </li>
              <li>
                <button type="button" class={ITEM_CLASS} onClick={showGameInfo}>
                  <span>Game info…</span>
                </button>
              </li>
              <li>
                <button type="button" class={ITEM_CLASS} onClick={changeCore}>
                  <span>Change core…</span>
                  <Show when={props.entry!.coreOverride}>
                    <span class="text-[0.6rem] uppercase tracking-widest text-(--color-system-accent)">
                      override
                    </span>
                  </Show>
                </button>
              </li>
              <li>
                <button type="button" class={ITEM_CLASS} onClick={openProperties}>
                  <span>Game properties…</span>
                </button>
              </li>
              <li class="my-1 h-px bg-white/5" aria-hidden="true" />
              <li>
                <button
                  type="button"
                  class={`${ITEM_CLASS} text-(--color-oa-ink-dim) hover:bg-red-500/10 hover:text-(--color-oa-ink)`}
                  onClick={removeFromLibrary}
                >
                  <span>Remove from library</span>
                </button>
              </li>
            </ul>
          </div>
        );
      }}
    </Show>
  );
};

export default TileContextMenu;
