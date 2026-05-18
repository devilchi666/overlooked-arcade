import {
  createEffect,
  createMemo,
  createSignal,
  For,
  onCleanup,
  onMount,
  Show,
  type Accessor,
  type Component,
} from "solid-js";
import { createVirtualizer } from "@tanstack/solid-virtual";
import LibraryTile from "./LibraryTile";
import type { EntryGroup } from "../library/filter";
import type { RomEntry } from "../library/types";

type Props = {
  /** Pre-filtered + sorted + grouped list. Empty group label = render no
   *  header. Caller is responsible for re-deriving on signal changes; this
   *  component just renders what it's given. */
  groups: EntryGroup[];
  /** Optional overrides if caller wants tighter / looser geometry. Defaults
   *  match the Phase 2.5 layout token defaults. */
  tileWidth?: number;
  gap?: number;
  /** Tile aspect ratio cap. Computed row height = tileWidth / aspect + title.
   *  Default 0.75 (3:4 portrait) which covers the worst case for the systems
   *  we ship. */
  minAspect?: number;
  onLaunch: (entry: RomEntry) => void;
  onShowSaves?: (entry: RomEntry) => void;
  onPickContext?: (entry: RomEntry, position: { x: number; y: number }) => void;
  onFocus?: (entry: RomEntry) => void;
};

type GridRow =
  | { kind: "header"; groupId: string; label: string }
  | { kind: "tiles"; groupId: string; entries: RomEntry[] };

const HEADER_HEIGHT = 48;
const TITLE_BLOCK_HEIGHT = 56; // tile bottom title/system label slot
const ROW_VERTICAL_PADDING = 12;

const VirtualLibraryGrid: Component<Props> = (props) => {
  let scrollRef: HTMLDivElement | undefined;
  const [containerWidth, setContainerWidth] = createSignal(0);

  const tileWidth = () => props.tileWidth ?? 220;
  const gap = () => props.gap ?? 12;
  const aspect = () => props.minAspect ?? 0.75; // width / height

  // Derive column count from container width (excluding the layout token
  // padding). Snaps to >=1 column even on tiny windows.
  const columnCount = createMemo(() => {
    const cw = containerWidth();
    if (cw < tileWidth() + 32) return 1;
    return Math.max(1, Math.floor((cw - 32 + gap()) / (tileWidth() + gap())));
  });

  // Flatten grouped entries into a flat row list — alternating header rows
  // (when label non-empty) and tile rows (with up to `cols` entries each).
  const rows = createMemo<GridRow[]>(() => {
    const cols = columnCount();
    const result: GridRow[] = [];
    for (const g of props.groups) {
      if (g.label.length > 0) {
        result.push({ kind: "header", groupId: g.id, label: g.label });
      }
      for (let i = 0; i < g.entries.length; i += cols) {
        result.push({
          kind: "tiles",
          groupId: g.id,
          entries: g.entries.slice(i, i + cols),
        });
      }
    }
    return result;
  });

  // Estimated row height — varies between header (small) and tile (large).
  // tile row height = tileWidth / aspect + title block + vertical padding.
  const tileRowHeight = createMemo(
    () => Math.round(tileWidth() / aspect()) + TITLE_BLOCK_HEIGHT + ROW_VERTICAL_PADDING,
  );

  const virtualizer = createVirtualizer({
    get count() { return rows().length; },
    getScrollElement: () => scrollRef ?? null,
    estimateSize: (index) => {
      const row = rows()[index];
      if (!row) return tileRowHeight();
      return row.kind === "header" ? HEADER_HEIGHT : tileRowHeight();
    },
    overscan: 4,
  });

  onMount(() => {
    if (!scrollRef) return;
    const ro = new ResizeObserver((entries) => {
      const w = entries[0]?.contentRect.width ?? 0;
      setContainerWidth(w);
    });
    ro.observe(scrollRef);
    setContainerWidth(scrollRef.getBoundingClientRect().width);
    onCleanup(() => ro.disconnect());
  });

  // When column count changes (due to resize), tell the virtualizer to
  // re-measure so row offsets stay correct.
  createEffect(() => {
    void columnCount();
    void tileRowHeight();
    virtualizer.measure();
  });

  return (
    <div
      ref={scrollRef!}
      class="h-full overflow-y-auto overscroll-contain"
      style={{ "scroll-padding-top": "8px" }}
    >
      <div
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          width: "100%",
          position: "relative",
        }}
      >
        <For each={virtualizer.getVirtualItems()}>
          {(vItem) => {
            const row: Accessor<GridRow | undefined> = () => rows()[vItem.index];
            return (
              <div
                data-index={vItem.index}
                ref={(el) => queueMicrotask(() => virtualizer.measureElement(el))}
                style={{
                  position: "absolute",
                  top: 0,
                  left: 0,
                  width: "100%",
                  transform: `translateY(${vItem.start}px)`,
                  "contain": "layout paint",
                }}
              >
                <Show when={row()?.kind === "header"}>
                  <div class="px-(--layout-content-padding-x) pt-4 pb-2">
                    <h3 class="text-[0.65rem] font-semibold uppercase tracking-[0.3em] text-(--color-oa-ink-dim)">
                      {(row() as Extract<GridRow, { kind: "header" }>).label}
                    </h3>
                    <div class="mt-2 h-px bg-white/5" />
                  </div>
                </Show>
                <Show when={row()?.kind === "tiles"}>
                  <div
                    class="grid px-(--layout-content-padding-x) py-(--layout-content-padding-y)"
                    style={{
                      "grid-template-columns": `repeat(${columnCount()}, minmax(0, 1fr))`,
                      gap: `${gap()}px`,
                      "padding-top": `${ROW_VERTICAL_PADDING / 2}px`,
                      "padding-bottom": `${ROW_VERTICAL_PADDING / 2}px`,
                    }}
                  >
                    <For each={(row() as Extract<GridRow, { kind: "tiles" }>).entries}>
                      {(entry) => (
                        <div style={{ "content-visibility": "auto", "contain-intrinsic-size": "auto 320px 360px" }}>
                          <LibraryTile
                            entry={entry}
                            onLaunch={props.onLaunch}
                            onShowSaves={props.onShowSaves}
                            onPickContext={props.onPickContext}
                            onFocus={props.onFocus}
                          />
                        </div>
                      )}
                    </For>
                  </div>
                </Show>
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
};

export default VirtualLibraryGrid;
