import {
  createEffect,
  createMemo,
  For,
  onCleanup,
  onMount,
  Show,
  type Component,
} from "solid-js";
import { createVirtualizer } from "@tanstack/solid-virtual";
import type { EntryGroup } from "@oa/platform/library/filter";
import { useMedia } from "@oa/platform/library/media";
import type { RomEntry } from "@oa/platform/library/types";
import { systemThemes } from "@oa/platform/themes/registry";
import { useFocusGroup } from "../nav/focus";

type Props = {
  groups: EntryGroup[];
  onLaunch: (entry: RomEntry) => void;
  onShowSaves?: (entry: RomEntry) => void;
  onPickContext?: (entry: RomEntry, position: { x: number; y: number }) => void;
  onFocus?: (entry: RomEntry) => void;
  /// Controller-nav Y handler — opens the game info modal for the
  /// focused row. Same shape as VirtualLibraryGrid's onShowInfo.
  onShowInfo?: (entry: RomEntry) => void;
  /// Currently-selected entry id (or null). Each row compares its own entry
  /// id to derive its `selected` state.
  selectedId?: () => string | null;
  /// Lookup for the variant count badge — see VirtualLibraryGrid.
  variantCountFor?: (id: string) => number | undefined;
  /// Override neighbours for DPad edge-spillover and L1/R1 — defaults
  /// match VirtualLibraryGrid (legacy `left-sidebar` / `right-sidebar`).
  focusGroupNeighbours?: { left?: string; right?: string };
};

type ListRow =
  | { kind: "header"; label: string; groupId: string }
  | { kind: "game"; entry: RomEntry };

const HEADER_HEIGHT = 48;
const GAME_ROW_HEIGHT = 76;

const DetailListView: Component<Props> = (props) => {
  let scrollRef: HTMLDivElement | undefined;

  const rows = createMemo<ListRow[]>(() => {
    const result: ListRow[] = [];
    for (const g of props.groups) {
      if (g.label.length > 0) {
        result.push({ kind: "header", label: g.label, groupId: g.id });
      }
      for (const entry of g.entries) {
        result.push({ kind: "game", entry });
      }
    }
    return result;
  });

  // Flat game-row list (headers stripped) for the focus group's
  // index → entry mapping. Same shape as VirtualLibraryGrid's
  // flatEntries — the focus group walks games only; the operator
  // can't focus headers.
  const flatGameEntries = createMemo<RomEntry[]>(() => {
    const out: RomEntry[] = [];
    for (const g of props.groups) out.push(...g.entries);
    return out;
  });

  const focusedIndex = createMemo(() => {
    const sid = props.selectedId?.();
    if (sid === null || sid === undefined) return 0;
    const idx = flatGameEntries().findIndex((e) => e.id === sid);
    return idx >= 0 ? idx : 0;
  });

  // Shares the "library-grid" id with VirtualLibraryGrid — only one
  // of the two renders at a time (capsule vs list view mode), so
  // there's no collision. LibraryPage's delegating effect activates
  // "library-grid" regardless of view mode and the matching component
  // takes over.
  const focusGroup = useFocusGroup({
    id: "library-grid",
    orientation: "vertical",
    itemCount: () => flatGameEntries().length,
    focusedIndex,
    setFocusedIndex: (next) => {
      const list = flatGameEntries();
      if (list.length === 0) return;
      const clamped = Math.max(0, Math.min(list.length - 1, next));
      const entry = list[clamped];
      if (entry) props.onFocus?.(entry);
    },
    onActivate: (i) => {
      const e = flatGameEntries()[i];
      if (e) props.onLaunch(e);
    },
    onSecondary: (i) => {
      const e = flatGameEntries()[i];
      if (!e) return;
      const el = rowEls.get(i);
      if (el && props.onPickContext) {
        const r = el.getBoundingClientRect();
        props.onPickContext(e, { x: r.left + r.width / 2, y: r.top + r.height / 2 });
      }
    },
    onTertiary: (i) => {
      const e = flatGameEntries()[i];
      if (e) props.onShowInfo?.(e);
    },
    neighbours: {
      left: props.focusGroupNeighbours?.left ?? "left-sidebar",
      right: props.focusGroupNeighbours?.right ?? "right-sidebar",
    },
  });

  // DOM-element registry by flat game index — used for both bind() to
  // the focus manager and bounding-rect lookup on X-press
  // (context-menu anchor).
  const rowEls = new Map<number, HTMLButtonElement>();

  // Keep the focused row scrolled into view. Use the virtualizer's
  // own scrollToIndex so the row's virtual index (in `rows`, with
  // headers interleaved) lines up with the visible viewport. The
  // focus group works against flat game indices — we re-derive the
  // row index here.
  const rowIndexForGameIndex = (gameIdx: number): number => {
    const r = rows();
    let g = 0;
    for (let i = 0; i < r.length; i++) {
      if (r[i].kind === "game") {
        if (g === gameIdx) return i;
        g++;
      }
    }
    return -1;
  };
  createEffect(() => {
    const i = focusedIndex();
    if (flatGameEntries().length === 0) return;
    const rowIdx = rowIndexForGameIndex(i);
    if (rowIdx >= 0) {
      virtualizer.scrollToIndex(rowIdx, { align: "auto" });
    }
  });

  const virtualizer = createVirtualizer({
    get count() { return rows().length; },
    getScrollElement: () => scrollRef ?? null,
    estimateSize: (index) => {
      const row = rows()[index];
      if (!row) return GAME_ROW_HEIGHT;
      return row.kind === "header" ? HEADER_HEIGHT : GAME_ROW_HEIGHT;
    },
    overscan: 8,
  });

  onMount(() => {
    if (!scrollRef) return;
    const ro = new ResizeObserver(() => virtualizer.measure());
    ro.observe(scrollRef);
    onCleanup(() => ro.disconnect());
  });

  createEffect(() => {
    void rows().length;
    virtualizer.measure();
  });

  return (
    <div ref={scrollRef!} class="h-full overflow-y-auto overscroll-contain">
      <div
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          width: "100%",
          position: "relative",
        }}
      >
        <For each={virtualizer.getVirtualItems()}>
          {(vItem) => {
            const row = () => rows()[vItem.index];
            return (
              <div
                data-index={vItem.index}
                style={{
                  position: "absolute",
                  top: 0,
                  left: 0,
                  width: "100%",
                  transform: `translateY(${vItem.start}px)`,
                  contain: "layout paint",
                }}
              >
                <Show when={row()?.kind === "header"}>
                  <div class="px-(--layout-content-padding-x) pt-4 pb-2">
                    <h3 class="text-[0.65rem] font-semibold uppercase tracking-[0.3em] text-(--color-oa-ink-dim)">
                      {(row() as Extract<ListRow, { kind: "header" }>).label}
                    </h3>
                    <div class="mt-2 h-px bg-white/5" />
                  </div>
                </Show>
                <Show when={row()?.kind === "game"}>
                  {(() => {
                    const r = row() as Extract<ListRow, { kind: "game" }>;
                    // Map this row's entry id to its flat game index so the
                    // focus manager binds the right slot. Captured at mount
                    // via the ref callback below.
                    const flatIdx = createMemo(() =>
                      flatGameEntries().findIndex((e) => e.id === r.entry.id),
                    );
                    const isFocused = () =>
                      flatIdx() >= 0 && focusedIndex() === flatIdx();
                    return (
                      <DetailRow
                        entry={r.entry}
                        onLaunch={props.onLaunch}
                        onShowSaves={props.onShowSaves}
                        onPickContext={props.onPickContext}
                        onFocus={(e) => {
                          // Mouse hover / focus claims active group so
                          // subsequent DPad input goes to this list.
                          focusGroup.activate();
                          props.onFocus?.(e);
                        }}
                        selectedId={props.selectedId}
                        variantCount={
                          props.variantCountFor?.(r.entry.id)
                        }
                        focusedActive={isFocused() && focusGroup.isActive()}
                        bindRef={(el) => {
                          if (!el) return;
                          const idx = flatIdx();
                          if (idx < 0) return;
                          rowEls.set(idx, el);
                          focusGroup.bind(idx, el);
                          onCleanup(() => {
                            rowEls.delete(idx);
                            focusGroup.bind(idx, null);
                          });
                        }}
                      />
                    );
                  })()}
                </Show>
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
};

const DetailRow: Component<{
  entry: RomEntry;
  onLaunch: (e: RomEntry) => void;
  onShowSaves?: (e: RomEntry) => void;
  onPickContext?: (e: RomEntry, position: { x: number; y: number }) => void;
  onFocus?: (e: RomEntry) => void;
  selectedId?: () => string | null;
  variantCount?: number;
  /// True when this row is the active focus target via gamepad. The
  /// `data-oa-focus*` attributes drive the same ring styling tiles use.
  focusedActive?: boolean;
  /// Ref hook called with the button element on mount + with null on
  /// unmount. DetailListView's parent uses this to bind the button to
  /// the focus group's index.
  bindRef?: (el: HTMLButtonElement | null) => void;
}> = (props) => {
  const media = useMedia();
  const theme = () => systemThemes[props.entry.systemId];
  const meta = () => media.media(props.entry.id)?.metadata;
  const coverSrc = () =>
    media.coverUrl(props.entry.systemId, props.entry.id, "box-front", "thumb");
  const selected = () => props.selectedId?.() === props.entry.id;

  return (
    <button
      type="button"
      ref={(el) => props.bindRef?.(el)}
      data-system={props.entry.systemId}
      data-oa-focus={props.focusedActive ? "true" : undefined}
      data-oa-focus-active={props.focusedActive ? "true" : "false"}
      // Single click selects; double click launches. See LibraryTile.tsx for
      // the rationale (hover no longer drives selection).
      onClick={() => props.onFocus?.(props.entry)}
      onDblClick={() => props.onLaunch(props.entry)}
      onFocus={() => props.onFocus?.(props.entry)}
      onContextMenu={(e) => {
        if (props.entry.seed || !props.onPickContext) return;
        e.preventDefault();
        props.onPickContext(props.entry, { x: e.clientX, y: e.clientY });
      }}
      aria-pressed={selected()}
      class="group flex w-full items-center gap-4 px-(--layout-content-padding-x) py-2 text-left transition hover:bg-white/[0.03] focus-visible:bg-white/[0.05]"
      classList={{
        "bg-(--color-system-accent)/10 border-l-2 border-(--color-system-accent)": selected(),
      }}
      style={{ "content-visibility": "auto", "contain-intrinsic-size": `auto 100% ${GAME_ROW_HEIGHT}px` }}
    >
      <div class="relative h-14 w-20 shrink-0 overflow-hidden rounded border border-white/10 bg-(--color-oa-bg-deep)">
        <Show
          when={coverSrc()}
          fallback={
            <div
              class="absolute inset-0"
              style={{
                background:
                  "radial-gradient(circle at 30% 25%, var(--color-system-glow), transparent 60%), linear-gradient(135deg, var(--color-system-accent) 0%, var(--color-oa-bg-deep) 100%)",
              }}
            />
          }
        >
          {(src) => (
            <img
              src={src()}
              alt=""
              loading="lazy"
              decoding="async"
              class="h-full w-full object-contain"
            />
          )}
        </Show>
      </div>
      <div class="min-w-0 flex-1">
        <p class="truncate text-sm font-medium text-(--color-oa-ink)">
          {props.entry.title}
          <Show when={(props.variantCount ?? 0) > 1}>
            <span class="ml-2 rounded bg-white/5 px-1.5 py-0.5 text-[0.6rem] font-semibold uppercase tracking-widest text-(--color-system-accent-soft)">
              ▼ {props.variantCount}
            </span>
          </Show>
        </p>
        <p class="mt-0.5 truncate text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          <span class="text-(--color-system-accent)">{theme().shortName}</span>
          <Show when={meta()?.year}>{(y) => <> · {y()}</>}</Show>
          <Show when={meta()?.developer}>{(d) => <> · {d()}</>}</Show>
        </p>
      </div>
      <div class="hidden shrink-0 text-right text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim) md:block">
        <Show when={meta()?.genre}>{(g) => <p>{g()}</p>}</Show>
        <Show when={meta()?.players}>{(p) => <p>{p()}p</p>}</Show>
      </div>
      <Show when={props.entry.coreOverride}>
        <span
          class="shrink-0 rounded bg-black/65 px-1.5 py-0.5 text-[0.55rem] font-medium uppercase tracking-widest text-(--color-system-accent-soft)"
          title={`Custom core: ${props.entry.coreOverride}`}
        >
          Core ◆
        </span>
      </Show>
      <Show when={props.entry.seed}>
        <span class="shrink-0 rounded bg-black/60 px-1.5 py-0.5 text-[0.55rem] font-medium uppercase tracking-widest text-(--color-oa-ink-dim)">
          Preview
        </span>
      </Show>
    </button>
  );
};

export default DetailListView;
