import { createMemo, createResource, createSignal, For, onCleanup, onMount, Show, type Component } from "solid-js";
import * as coresApi from "@oa/platform/api/coresApi";
import type { LibraryStore } from "@oa/platform/library/store";
import type { RomEntry } from "@oa/platform/library/types";
import { captureFocusReturn, useFocusGroup } from "@oa/platform/nav";
import { useBackHandler } from "@oa/platform/nav";
import { HintRegion } from "@oa/platform/nav";

type CoreEntry = {
  fileName: string;
  libraryName: string;
  libraryVersion: string;
  validExtensions: string;
};

type Props = {
  /// The rom this picker is for. Null = menu hidden.
  entry: RomEntry | null;
  /// Position to anchor the menu (right-click event coordinates).
  position: { x: number; y: number } | null;
  library: LibraryStore;
  onClose: () => void;
};

/// Right-click context menu for setting a per-game core override. Lists every
/// detected core whose `valid_extensions` includes the rom's extension; plus a
/// "(default)" entry that clears the override and falls back to the per-system
/// pref. Closes on Escape, click-outside, or after a selection.
const CorePickerMenu: Component<Props> = (props) => {
  const [cores] = createResource(
    () => (props.entry ? props.entry.id : null),
    async () => {
      try {
        return await coresApi.listCores<CoreEntry>();
      } catch {
        return [];
      }
    },
  );

  const romExt = createMemo(() => {
    const e = props.entry;
    if (!e) return "";
    const dot = e.filePath.lastIndexOf(".");
    return dot >= 0 ? e.filePath.slice(dot + 1).toLowerCase() : "";
  });

  // A core advertises support via `validExtensions` like "pce|cue|chd|sgx".
  // We only show cores that include the rom's extension — picking a core that
  // doesn't handle the file would just fail load_game and the user wouldn't
  // know why. Always shows ALL cores if the rom has no extension (fallback).
  const compatibleCores = createMemo(() => {
    const list = cores() ?? [];
    const ext = romExt();
    if (!ext) return list;
    return list.filter((c) =>
      c.validExtensions
        .split("|")
        .map((x) => x.trim().toLowerCase())
        .includes(ext),
    );
  });

  function pick(fileName: string | null) {
    if (!props.entry) return;
    void props.library.setCoreOverride(props.entry.id, fileName);
    props.onClose();
  }

  /// Flat ordered item list. Index 0 is the "(Default — auto-detect)"
  /// row that clears the override; subsequent indices are the compatible
  /// cores. Empty when the menu is closed so the focus group ignores
  /// nav events.
  type PickItem =
    | { kind: "default" }
    | { kind: "core"; core: CoreEntry };
  const items = createMemo<PickItem[]>(() => {
    if (!props.entry) return [];
    return [{ kind: "default" }, ...compatibleCores().map<PickItem>((c) => ({ kind: "core", core: c }))];
  });

  const [focusedIndex, setFocusedIndex] = createSignal(0);
  const focusGroup = useFocusGroup({
    id: "core-picker-menu",
    orientation: "vertical",
    itemCount: () => items().length,
    focusedIndex,
    setFocusedIndex,
    onActivate: (i) => {
      const it = items()[i];
      if (!it) return;
      if (it.kind === "default") pick(null);
      else pick(it.core.fileName);
    },
    onCancel: () => props.onClose(),
  });

  function onWindowKey(e: KeyboardEvent) {
    if (e.key === "Escape") props.onClose();
  }
  function onWindowClick(e: MouseEvent) {
    // Click anywhere outside the menu closes it. The menu stops propagation
    // on its own clicks (see the root container below).
    const target = e.target as HTMLElement | null;
    if (!target || !target.closest("[data-core-picker-root]")) {
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
        useBackHandler(() => props.onClose());
        const restoreFocus = captureFocusReturn();
        onMount(() => {
          focusGroup.activate();
          setFocusedIndex(0);
        });
        onCleanup(restoreFocus);
        return (
          <div
            data-core-picker-root
            class="fixed z-50 min-w-[18rem] overflow-hidden rounded-md border border-white/10 bg-(--color-oa-bg-deep)/95 text-sm shadow-2xl shadow-black/60 backdrop-blur"
            style={{ left: `${pos.x}px`, top: `${pos.y}px` }}
            onClick={(e) => e.stopPropagation()}
          >
            <HintRegion hints={{ Confirm: "Pick", Back: "Close" }} />
            <div class="border-b border-white/5 px-3 py-2">
              <p class="truncate text-xs font-medium text-(--color-oa-ink)">{props.entry!.title}</p>
              <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">Run with core</p>
            </div>
            <ul class="max-h-72 overflow-y-auto py-1">
              <For each={items()}>
                {(item, index) => {
                  if (item.kind === "default") {
                    return (
                      <li
                        ref={(el) => focusGroup.bind(index(), el)}
                        data-oa-focus={focusedIndex() === index() ? "true" : undefined}
                        data-oa-focus-active={focusGroup.isActive() ? "true" : undefined}
                      >
                        <button
                          type="button"
                          class="flex w-full items-center justify-between px-3 py-1.5 text-left hover:bg-white/[0.06]"
                          onMouseEnter={() => setFocusedIndex(index())}
                          onClick={() => pick(null)}
                        >
                          <span class="text-(--color-oa-ink)">(Default — auto-detect)</span>
                          <Show when={!props.entry!.coreOverride}>
                            <span class="text-[0.6rem] uppercase tracking-widest text-(--color-system-accent)">active</span>
                          </Show>
                        </button>
                      </li>
                    );
                  }
                  const c = item.core;
                  const active = () => props.entry!.coreOverride === c.fileName;
                  return (
                    <li
                      ref={(el) => focusGroup.bind(index(), el)}
                      data-oa-focus={focusedIndex() === index() ? "true" : undefined}
                      data-oa-focus-active={focusGroup.isActive() ? "true" : undefined}
                    >
                      <button
                        type="button"
                        class="flex w-full flex-col items-start gap-0.5 px-3 py-1.5 text-left hover:bg-white/[0.06]"
                        onMouseEnter={() => setFocusedIndex(index())}
                        onClick={() => pick(c.fileName)}
                      >
                        <span class="flex w-full items-center justify-between">
                          <span class="truncate text-(--color-oa-ink)">{c.libraryName}</span>
                          <Show when={active()}>
                            <span class="text-[0.6rem] uppercase tracking-widest text-(--color-system-accent)">active</span>
                          </Show>
                        </span>
                        <span class="truncate text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                          {c.libraryVersion} · {c.fileName}
                        </span>
                      </button>
                    </li>
                  );
                }}
              </For>
              <Show when={cores.loading}>
                <li class="px-3 py-2 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                  Scanning cores…
                </li>
              </Show>
              <Show when={!cores.loading && compatibleCores().length === 0}>
                <li class="px-3 py-2 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                  No compatible cores for .{romExt() || "(no ext)"}
                </li>
              </Show>
            </ul>
          </div>
        );
      }}
    </Show>
  );
};

export default CorePickerMenu;
