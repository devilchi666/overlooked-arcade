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
import type { EntryGroup } from "../library/filter";
import { useMedia } from "../library/media";
import type { RomEntry } from "../library/types";
import { systemThemes } from "../themes/registry";

type Props = {
  groups: EntryGroup[];
  onLaunch: (entry: RomEntry) => void;
  onShowSaves?: (entry: RomEntry) => void;
  onPickContext?: (entry: RomEntry, position: { x: number; y: number }) => void;
  onFocus?: (entry: RomEntry) => void;
  /// Currently-selected entry id (or null). Each row compares its own entry
  /// id to derive its `selected` state.
  selectedId?: () => string | null;
  /// Lookup for the variant count badge — see VirtualLibraryGrid.
  variantCountFor?: (id: string) => number | undefined;
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
                  <DetailRow
                    entry={(row() as Extract<ListRow, { kind: "game" }>).entry}
                    onLaunch={props.onLaunch}
                    onShowSaves={props.onShowSaves}
                    onPickContext={props.onPickContext}
                    onFocus={props.onFocus}
                    selectedId={props.selectedId}
                    variantCount={
                      props.variantCountFor?.(
                        (row() as Extract<ListRow, { kind: "game" }>).entry.id,
                      )
                    }
                  />
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
      data-system={props.entry.systemId}
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
