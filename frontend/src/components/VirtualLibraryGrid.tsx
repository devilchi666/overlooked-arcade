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
import type { EntryGroup } from "@oa/platform/library/filter";
import type { RomEntry } from "@oa/platform/library/types";
import { useFocusGroup } from "../nav/focus";
import { isPerSystemUiEnabled, playSystemUiSound } from "@oa/platform/themes/systemUiSound";
import { DEFAULT_TILE_ASPECT, systemThemes, type SystemId } from "@oa/platform/themes/registry";
import { uiConfigFor, type UITileShape } from "@oa/platform/themes/systemUIConfigs";

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
  /// Controller-nav Y handler — opens the game info modal for the
  /// focused tile.
  onShowInfo?: (entry: RomEntry) => void;
  /// Accessor for the current selection id. Each tile compares its own
  /// entry id to derive its `selected` state — passed as a getter (not a
  /// raw value) so virtualized tiles only re-render when their own
  /// selected-ness actually flips, not when any other tile is picked.
  selectedId?: () => string | null;
  /// Lookup for the variant count to render the ▼N badge. Returns
  /// `undefined` (or 1) for single-file games — no badge. Passed as a
  /// function so the grid doesn't re-render every tile when a single
  /// group's pin changes.
  variantCountFor?: (id: string) => number | undefined;
  /// Retroverse-UI Phase C3 — pass-through favorite toggle. Forwarded
  /// to LibraryTile so the heart overlay can fire without the tile
  /// reaching into the library store directly.
  onToggleFavorite?: (entry: RomEntry, value: boolean) => void;
  /// Retroverse-UI LIBRARY polish — pass-through for the per-tile
  /// system-label header strip. See LibraryTile's prop docs.
  showSystemHeader?: boolean;
  /// Override the focus-group neighbours used for both DPad edge-
  /// spillover (LEFT at the top-left tile, RIGHT at the bottom-right
  /// tile) and the L1/R1 shoulder-bumper transfer. Defaults to the
  /// legacy `left-sidebar` / `right-sidebar` ids; Retroverse mode
  /// passes the per-page region group ids instead (e.g.
  /// `retroverse-library-left` / `retroverse-library-right`) so
  /// transfers land on the Retroverse page's regions rather than
  /// the legacy Shell's sidebars (which aren't mounted in Retroverse).
  focusGroupNeighbours?: { left?: string; right?: string };
};

type GridRow =
  | { kind: "header"; groupId: string; label: string }
  | { kind: "tiles"; groupId: string; entries: RomEntry[] };

const HEADER_HEIGHT = 48;
// Tile chrome below the cover area: title block (`flex flex-col gap-1 px-3 py-3`,
// h3 text-sm + p text-[0.65rem]) ~= 61px + 2px button border = ~63px. Round to
// 64 so estimates land at-or-above measured height; an *over*-estimate makes
// totalSize shrink on first measurement (thumb appears to jump forward — barely
// perceptible), an *under*-estimate makes it grow (thumb lags behind the mouse
// during a drag — the bug we're fixing).
const TILE_CHROME_HEIGHT = 64;
const ROW_VERTICAL_PADDING = 12;
const DEFAULT_ASPECT = 3 / 4;

// Parse "W/H" tile-aspect strings (matches the format in `systemThemes`).
function parseAspectString(s: string | undefined): number {
  if (!s) return DEFAULT_ASPECT;
  const m = s.match(/^\s*([\d.]+)\s*\/\s*([\d.]+)\s*$/);
  if (!m) return DEFAULT_ASPECT;
  const w = parseFloat(m[1]);
  const h = parseFloat(m[2]);
  return w > 0 && h > 0 ? w / h : DEFAULT_ASPECT;
}

function aspectForTileShape(shape: UITileShape): number | null {
  switch (shape) {
    case "square":
    case "circle": return 1;
    case "portrait-3:4": return 0.75;
    case "landscape-4:3": return 4 / 3;
    case "wide-16:9": return 16 / 9;
    case "auto":
    default: return null;
  }
}

// Mirror LibraryTile's tileAspectStyle resolution so estimate matches the
// shape the tile actually renders.
function aspectForSystem(sysId: string): number {
  if (isPerSystemUiEnabled()) {
    const override = aspectForTileShape(uiConfigFor(sysId as SystemId).tileShape);
    if (override !== null) return override;
  }
  return parseAspectString(
    systemThemes[sysId as SystemId]?.tileAspect ?? DEFAULT_TILE_ASPECT,
  );
}

/// Hybrid column-fitting helper. The caller-provided `target` width is
/// the operator's slider preference (e.g. 220px). We try every viable
/// column count and pick the one whose actual per-column width lands
/// within ±20% of target, choosing the closest to target if multiple
/// fit. Fallback: pure floor-divide at the unscaled target width
/// (matches the pre-slider behavior so extreme cases still render
/// sanely).
export function fitColumns(containerWidth: number, target: number, gap: number) {
  const usable = containerWidth - 32; // matches existing padding budget
  if (usable <= target * 0.5) {
    // Window too narrow to fit even a half-target tile — snap to 1 column
    // at whatever width the container provides.
    return { cols: 1, width: Math.max(usable, target * 0.5) };
  }
  const MIN_FACTOR = 0.8;
  const MAX_FACTOR = 1.2;
  const minWidth = target * MIN_FACTOR;
  const maxWidth = target * MAX_FACTOR;
  let best: { cols: number; width: number; delta: number } | null = null;
  for (let cols = 1; cols <= 32; cols++) {
    const w = (usable - gap * (cols - 1)) / cols;
    if (w < minWidth || w > maxWidth) continue;
    const delta = Math.abs(w - target);
    if (best === null || delta < best.delta) {
      best = { cols, width: w, delta };
    }
  }
  if (best !== null) {
    return { cols: best.cols, width: best.width };
  }
  // Fallback (extreme case): no col count fits within ±20% — use floor
  // divide at the target. Visual consequence: small gap on the right
  // edge. Acceptable.
  const cols = Math.max(1, Math.floor((usable + gap) / (target + gap)));
  return { cols, width: target };
}

const VirtualLibraryGrid: Component<Props> = (props) => {
  let scrollRef: HTMLDivElement | undefined;
  const [containerWidth, setContainerWidth] = createSignal(0);

  // Flat list of entries across all groups — used by controller-nav
  // focus model. Headers are skipped (focus only lands on tiles).
  const flatEntries = createMemo<RomEntry[]>(() => {
    const out: RomEntry[] = [];
    for (const g of props.groups) out.push(...g.entries);
    return out;
  });

  // Derive focused index from the parent-owned selectedId accessor.
  // Keeps "selection" and "focused tile" as one concept; mouse hover +
  // gamepad DPad both feed the same setFocusedEntry sink upstream.
  const focusedIndex = createMemo(() => {
    const sid = props.selectedId?.();
    const list = flatEntries();
    if (!sid || list.length === 0) return 0;
    const i = list.findIndex((e) => e.id === sid);
    return i >= 0 ? i : 0;
  });

  // Index → DOM element registry for the visible-virtualized subset.
  // Doubles as the focus.ts bind() target AND the source of bounding
  // rects for context-menu anchoring.
  const tileEls = new Map<number, HTMLDivElement>();

  const tileTarget = () => props.tileWidth ?? 220;
  const gap = () => props.gap ?? 12;
  const aspect = () => props.minAspect ?? 0.75; // width / height

  // Hybrid scaling: target tile width comes from the slider; actual
  // rendered width flexes ±20% to fill columns cleanly.
  const fit = createMemo(() => fitColumns(containerWidth(), tileTarget(), gap()));
  const columnCount = createMemo(() => fit().cols);
  const actualTileWidth = createMemo(() => fit().width);

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

  // Tile row height for a given aspect ratio. CSS grid sizes a row to
  // its tallest cell, so for mixed-system rows we use the SMALLEST
  // aspect (= tallest tile). Keeping this aligned with the rendered
  // height matters: the virtualizer calls `measureElement` post-mount
  // and a discrepancy between estimate and measured size shifts
  // `totalSize` underneath an in-progress scrollbar drag — the native
  // scrollbar thumb then lags behind the mouse cursor because its
  // position is recomputed against the new scrollHeight each frame.
  const rowHeightForAspect = (a: number): number =>
    Math.round(actualTileWidth() / a) + TILE_CHROME_HEIGHT + ROW_VERTICAL_PADDING;

  const virtualizer = createVirtualizer({
    get count() { return rows().length; },
    getScrollElement: () => scrollRef ?? null,
    estimateSize: (index) => {
      const row = rows()[index];
      if (!row) return rowHeightForAspect(aspect());
      if (row.kind === "header") return HEADER_HEIGHT;
      let minAspect = Infinity;
      for (const e of row.entries) {
        const a = aspectForSystem(e.systemId);
        if (a < minAspect) minAspect = a;
      }
      if (!isFinite(minAspect)) minAspect = aspect();
      return rowHeightForAspect(minAspect);
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

  // When column count or actual tile width changes (due to resize or
  // slider change), tell the virtualizer to re-measure so row offsets
  // stay correct. Also re-fire when the per-system UI master toggle
  // flips, since that swaps the tile-shape override path and therefore
  // every row's estimate.
  createEffect(() => {
    void columnCount();
    void actualTileWidth();
    void isPerSystemUiEnabled();
    virtualizer.measure();
  });

  // Map a flat entry index to its row index in the virtualizer's row
  // list (headers interleave; tile rows hold up to `columnCount` entries).
  const rowIndexForEntryIndex = (entryIdx: number): number => {
    const r = rows();
    let consumed = 0;
    for (let i = 0; i < r.length; i++) {
      const row = r[i];
      if (row.kind === "header") continue;
      const rowLen = row.entries.length;
      if (entryIdx < consumed + rowLen) return i;
      consumed += rowLen;
    }
    return -1;
  };

  const focusGroup = useFocusGroup({
    id: "library-grid",
    orientation: "grid",
    itemCount: () => flatEntries().length,
    columns: columnCount,
    focusedIndex,
    setFocusedIndex: (next) => {
      const list = flatEntries();
      if (list.length === 0) return;
      const clamped = Math.max(0, Math.min(list.length - 1, next));
      const entry = list[clamped];
      if (entry) {
        props.onFocus?.(entry);
        // Per-System UI Stage 1 Slice 2: a DPad-driven cursor move
        // fires the focused system's "navigate" SFX. Mouse hover is
        // intentionally silent — sounds are gamepad-centric for couch
        // play in v1. setFocusedIndex only fires from gamepad nav
        // (mouse interactions flow through props.onFocus directly).
        playSystemUiSound(entry.systemId as SystemId, "navigate");
      }
    },
    onActivate: (i) => {
      const e = flatEntries()[i];
      if (e) {
        // Per-System UI Stage 1 Slice 2: a gamepad A-press on a tile
        // fires the system's "launch" SFX before the launch handler
        // runs. Mouse click on a tile goes through a different path
        // and stays silent in v1.
        playSystemUiSound(e.systemId as SystemId, "launch");
        props.onLaunch(e);
      }
    },
    onSecondary: (i) => {
      const e = flatEntries()[i];
      const el = tileEls.get(i);
      if (!e) return;
      if (el && props.onPickContext) {
        const r = el.getBoundingClientRect();
        props.onPickContext(e, { x: r.left + r.width / 2, y: r.top + r.height / 2 });
      }
    },
    onTertiary: (i) => {
      const e = flatEntries()[i];
      if (e) props.onShowInfo?.(e);
    },
    neighbours: {
      left: props.focusGroupNeighbours?.left ?? "left-sidebar",
      right: props.focusGroupNeighbours?.right ?? "right-sidebar",
    },
  });

  // Keep the focused tile visible. `align: "auto"` scrolls only when
  // the row isn't already on screen — mouse hover within the visible
  // range is a no-op.
  createEffect(() => {
    const i = focusedIndex();
    if (flatEntries().length === 0) return;
    const rowIdx = rowIndexForEntryIndex(i);
    if (rowIdx >= 0) {
      virtualizer.scrollToIndex(rowIdx, { align: "auto" });
    }
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
                      "grid-template-columns": `repeat(${columnCount()}, ${actualTileWidth()}px)`,
                      gap: `${gap()}px`,
                      "padding-top": `${ROW_VERTICAL_PADDING / 2}px`,
                      "padding-bottom": `${ROW_VERTICAL_PADDING / 2}px`,
                    }}
                  >
                    <For each={(row() as Extract<GridRow, { kind: "tiles" }>).entries}>
                      {(entry) => {
                        const flatIdx = createMemo(() =>
                          flatEntries().findIndex((e) => e.id === entry.id),
                        );
                        const isFocused = () =>
                          flatIdx() >= 0 && focusedIndex() === flatIdx();
                        return (
                          <div
                            ref={(el) => {
                              if (!el) return;
                              // Capture flatIdx at mount time. The
                              // memo can re-evaluate to a different
                              // value (search/filter shifts entries)
                              // OR transiently to -1 before
                              // flatEntries() settles; if the cleanup
                              // reads `flatIdx()` lazily it would
                              // delete the wrong bind entry.
                              const idx = flatIdx();
                              if (idx < 0) return;
                              tileEls.set(idx, el);
                              focusGroup.bind(idx, el);
                              onCleanup(() => {
                                tileEls.delete(idx);
                                focusGroup.bind(idx, null);
                              });
                            }}
                            data-oa-focus={isFocused() ? "true" : undefined}
                            data-oa-focus-active={focusGroup.isActive() ? "true" : "false"}
                          >
                            <LibraryTile
                              entry={entry}
                              onLaunch={props.onLaunch}
                              onShowSaves={props.onShowSaves}
                              onPickContext={props.onPickContext}
                              onFocus={(e) => {
                                focusGroup.activate();
                                props.onFocus?.(e);
                              }}
                              selected={props.selectedId?.() === entry.id}
                              variantCount={props.variantCountFor?.(entry.id)}
                              onToggleFavorite={props.onToggleFavorite}
                              showSystemHeader={props.showSystemHeader}
                            />
                          </div>
                        );
                      }}
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
