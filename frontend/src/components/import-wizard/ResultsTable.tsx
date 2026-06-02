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
  type JSX,
} from "solid-js";
import { createVirtualizer } from "@tanstack/solid-virtual";
import type { ScanConfidence } from "../../library/ingest";
import { systemThemes, type SystemId } from "../../themes/registry";
import { useDomQueryFocusGroup } from "../../nav/focus";

// Phase 1B Slice 2 — Per-ROM results table for the Import Wizard.
//
// Mounts inside the wizard's Step 2 "Scan + review". One row per
// scanned ROM. Columns: checkbox / file / detected system / suggested
// title / confidence / status / skip-toggle. Inline editing for
// system + title; bulk-select toolbar that appears on ≥1 row selected;
// sort + filter; @tanstack/solid-virtual virtualization for 5K+ row
// libraries.
//
// Owns no scan state — the parent wizard passes `rows` (already
// classified by the Slice 1 backend smart-scan) and `onRowChange` /
// `onBulkChange` callbacks. Edit drafts are stored in a signal keyed
// by row.key so virtualized scroll doesn't drop in-progress edits.

export type TableRow = {
  /// Stable id (path + archiveInnerPath). Used by the parent to apply
  /// onRowChange patches; survives sort + filter.
  key: string;
  // Immutable from scan
  path: string;
  fileName: string;
  extension: string;
  archiveInnerPath?: string;
  systemHint?: SystemId;
  confidence?: ScanConfidence;
  sha1?: string;
  // Operator-mutable
  systemId: SystemId | null;
  title: string;
  skipped: boolean;
  selected: boolean;
};

export type ResultsTableProps = {
  rows: Accessor<TableRow[]>;
  onRowChange: (key: string, patch: Partial<TableRow>) => void;
  onBulkChange: (keys: string[], patch: Partial<TableRow>) => void;
};

type SortKey = "fileName" | "systemId" | "title" | "confidence" | "status";
type SortDir = "asc" | "desc";

const ROW_HEIGHT = 48;

const CONFIDENCE_PILL_STYLES: Record<ScanConfidence, { ring: string; bg: string; text: string; label: string }> = {
  hash: {
    ring: "border-emerald-400/40",
    bg: "bg-emerald-500/10",
    text: "text-emerald-300",
    label: "Hash",
  },
  header: {
    ring: "border-cyan-400/40",
    bg: "bg-cyan-500/10",
    text: "text-cyan-300",
    label: "Header",
  },
  extension: {
    ring: "border-white/15",
    bg: "bg-white/[0.04]",
    text: "text-(--color-oa-ink-dim)",
    label: "Ext",
  },
  hint: {
    ring: "border-amber-400/40",
    bg: "bg-amber-500/10",
    text: "text-amber-300",
    label: "Hint",
  },
};

type RowStatus = "ready" | "needsSystem" | "skipped";

const STATUS_PILL_STYLES: Record<RowStatus, { ring: string; bg: string; text: string; label: string }> = {
  ready: {
    ring: "border-emerald-400/40",
    bg: "bg-emerald-500/10",
    text: "text-emerald-300",
    label: "✓ Ready",
  },
  needsSystem: {
    ring: "border-amber-400/40",
    bg: "bg-amber-500/10",
    text: "text-amber-300",
    label: "⚠ Needs system",
  },
  skipped: {
    ring: "border-white/10",
    bg: "bg-white/[0.02]",
    text: "text-(--color-oa-ink-dim)",
    label: "⊘ Skipped",
  },
};

function statusFor(row: TableRow): RowStatus {
  if (row.skipped) return "skipped";
  if (row.systemId === null) return "needsSystem";
  return "ready";
}

const CONFIDENCE_RANK: Record<ScanConfidence | "none", number> = {
  hash: 0,
  header: 1,
  extension: 2,
  hint: 3,
  none: 4,
};

const STATUS_RANK: Record<RowStatus, number> = {
  needsSystem: 0,
  ready: 1,
  skipped: 2,
};

function compareRows(a: TableRow, b: TableRow, key: SortKey, dir: SortDir): number {
  let cmp = 0;
  switch (key) {
    case "fileName":
      cmp = a.fileName.toLowerCase().localeCompare(b.fileName.toLowerCase());
      break;
    case "systemId":
      cmp = (a.systemId ?? "").localeCompare(b.systemId ?? "");
      break;
    case "title":
      cmp = a.title.toLowerCase().localeCompare(b.title.toLowerCase());
      break;
    case "confidence":
      cmp = CONFIDENCE_RANK[a.confidence ?? "none"] - CONFIDENCE_RANK[b.confidence ?? "none"];
      break;
    case "status":
      cmp = STATUS_RANK[statusFor(a)] - STATUS_RANK[statusFor(b)];
      break;
  }
  return dir === "asc" ? cmp : -cmp;
}

const CELL_BASE = "px-2 py-2 text-xs text-(--color-oa-ink)";
const HEADER_CELL =
  "px-2 py-2 text-[0.6rem] font-semibold uppercase tracking-widest text-(--color-oa-ink-dim) select-none";

// Column widths — use flex/min-width tokens to balance file vs system
// vs title across the wizard's ~720px content width. Checkbox is fixed.
const COL = {
  checkbox: "w-8 shrink-0",
  file: "flex-[2.2] min-w-0",
  system: "flex-[1.4] min-w-0",
  title: "flex-[2.0] min-w-0",
  confidence: "w-20 shrink-0",
  status: "w-32 shrink-0",
  actions: "w-16 shrink-0",
};

const ResultsTable: Component<ResultsTableProps> = (props) => {
  const [sortKey, setSortKey] = createSignal<SortKey>("fileName");
  const [sortDir, setSortDir] = createSignal<SortDir>("asc");
  const [filterText, setFilterText] = createSignal("");
  const [showSkipped, setShowSkipped] = createSignal(false);

  // Inline-edit state — `editingCell` is "<rowKey>:system" or
  // "<rowKey>:title". `editDraft` holds the in-progress value keyed
  // by the same id so a virtualized-row remount restores it.
  const [editingCell, setEditingCell] = createSignal<string | null>(null);
  const [editDrafts, setEditDrafts] = createSignal<Record<string, string>>({});

  // Show-path popover — open path popover for a single row at a time.
  const [pathPopoverKey, setPathPopoverKey] = createSignal<string | null>(null);

  // Bulk Change-system popover anchor.
  const [bulkSystemOpen, setBulkSystemOpen] = createSignal(false);

  /// Filtered + sorted view of the rows. Sort doesn't mutate the
  /// underlying row state (selected stays with each row's identity)
  /// because rows is an Accessor — we return a derived array.
  const visibleRows = createMemo<TableRow[]>(() => {
    const text = filterText().trim().toLowerCase();
    const includeSkipped = showSkipped();
    const filtered = props.rows().filter((r) => {
      if (!includeSkipped && r.skipped) return false;
      if (text.length === 0) return true;
      return (
        r.fileName.toLowerCase().includes(text) ||
        r.title.toLowerCase().includes(text)
      );
    });
    const key = sortKey();
    const dir = sortDir();
    return [...filtered].sort((a, b) => compareRows(a, b, key, dir));
  });

  const selectedRows = createMemo(() => props.rows().filter((r) => r.selected));
  const selectedCount = createMemo(() => selectedRows().length);

  /// Tri-state header checkbox over the CURRENTLY VISIBLE rows.
  /// Header click selects all visible rows; if everything visible is
  /// already selected, clears the visible selection.
  const headerCheckboxState = createMemo<"none" | "some" | "all">(() => {
    const v = visibleRows();
    if (v.length === 0) return "none";
    const selectedVisible = v.filter((r) => r.selected).length;
    if (selectedVisible === 0) return "none";
    if (selectedVisible === v.length) return "all";
    return "some";
  });

  function toggleHeaderSelection() {
    const state = headerCheckboxState();
    const visibleKeys = visibleRows().map((r) => r.key);
    props.onBulkChange(visibleKeys, { selected: state !== "all" });
  }

  function toggleRowSelection(row: TableRow) {
    props.onRowChange(row.key, { selected: !row.selected });
  }

  function clickHeader(key: SortKey) {
    if (sortKey() === key) {
      setSortDir(sortDir() === "asc" ? "desc" : "asc");
    } else {
      setSortKey(key);
      setSortDir("asc");
    }
  }

  function sortGlyph(key: SortKey): string {
    if (sortKey() !== key) return "";
    return sortDir() === "asc" ? " ▲" : " ▼";
  }

  // --- Inline edit helpers ---------------------------------------------

  function beginEdit(rowKey: string, field: "system" | "title", initial: string) {
    const id = `${rowKey}:${field}`;
    setEditDrafts({ ...editDrafts(), [id]: initial });
    setEditingCell(id);
  }

  function updateDraft(id: string, value: string) {
    setEditDrafts({ ...editDrafts(), [id]: value });
  }

  function commitEdit(rowKey: string, field: "system" | "title") {
    const id = `${rowKey}:${field}`;
    const draft = editDrafts()[id];
    if (draft !== undefined) {
      if (field === "title") {
        props.onRowChange(rowKey, { title: draft });
      } else {
        // system field — only apply if the value is a registered SystemId.
        if (systemThemes[draft as SystemId]) {
          props.onRowChange(rowKey, { systemId: draft as SystemId });
        }
      }
    }
    cancelEdit(id);
  }

  function cancelEdit(id: string) {
    const next = { ...editDrafts() };
    delete next[id];
    setEditDrafts(next);
    if (editingCell() === id) setEditingCell(null);
  }

  // Close path popover + cancel any in-progress edit on Escape /
  // outside click. Outside-click handler is window-level; the cell
  // input + popover stop propagation so they don't self-dismiss.
  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        const id = editingCell();
        if (id) cancelEdit(id);
        if (pathPopoverKey()) setPathPopoverKey(null);
        if (bulkSystemOpen()) setBulkSystemOpen(false);
      }
    };
    const onDocClick = (e: MouseEvent) => {
      const t = e.target as HTMLElement | null;
      if (!t) return;
      if (pathPopoverKey() && !t.closest("[data-oa-path-popover]")) {
        setPathPopoverKey(null);
      }
      if (bulkSystemOpen() && !t.closest("[data-oa-bulk-system-popover]")) {
        setBulkSystemOpen(false);
      }
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("mousedown", onDocClick);
    onCleanup(() => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("mousedown", onDocClick);
    });
  });

  // --- Virtualizer -----------------------------------------------------

  let scrollRef: HTMLDivElement | undefined;
  const virtualizer = createVirtualizer({
    get count() { return visibleRows().length; },
    getScrollElement: () => scrollRef ?? null,
    estimateSize: () => ROW_HEIGHT,
    overscan: 8,
  });

  onMount(() => {
    if (!scrollRef) return;
    const ro = new ResizeObserver(() => virtualizer.measure());
    ro.observe(scrollRef);
    onCleanup(() => ro.disconnect());
  });

  createEffect(() => {
    void visibleRows().length;
    virtualizer.measure();
  });

  // --- Controller-nav --------------------------------------------------

  // Row-level vertical focus group. Cell-level edits stay mouse +
  // keyboard for Slice 2 — DPad LEFT/RIGHT would need 2D nav we don't
  // have yet. Focus moves between rows; A toggles row's checkbox.
  let tableContainerRef: HTMLDivElement | undefined;
  useDomQueryFocusGroup({
    id: "import-wizard-table",
    containerRef: () => tableContainerRef,
    orientation: "vertical",
    onActivate: (_i, el) => el.click(),
  });

  // --- Render ----------------------------------------------------------

  const allSystemIds = Object.keys(systemThemes) as SystemId[];

  function renderConfidenceBadge(c?: ScanConfidence): JSX.Element {
    if (!c) return null;
    const s = CONFIDENCE_PILL_STYLES[c];
    return (
      <span class={`inline-block self-start rounded border ${s.ring} ${s.bg} px-1.5 py-0.5 text-[0.55rem] uppercase tracking-widest ${s.text}`}>
        {s.label}
      </span>
    );
  }

  function renderStatusBadge(row: TableRow): JSX.Element {
    const s = STATUS_PILL_STYLES[statusFor(row)];
    return (
      <span class={`inline-block self-start rounded border ${s.ring} ${s.bg} px-1.5 py-0.5 text-[0.55rem] uppercase tracking-widest ${s.text}`}>
        {s.label}
      </span>
    );
  }

  function renderRow(row: TableRow): JSX.Element {
    const editingSystem = () => editingCell() === `${row.key}:system`;
    const editingTitle = () => editingCell() === `${row.key}:title`;
    const systemDisplay = () => row.systemId ? (systemThemes[row.systemId]?.displayName ?? row.systemId) : "—";

    return (
      <div
        data-oa-table-row="true"
        tabindex="0"
        class="flex items-center gap-1 border-b border-white/5 hover:bg-white/[0.04] data-[oa-focus=true]:bg-white/[0.06]"
        classList={{ "opacity-50": row.skipped }}
        onKeyDown={(e) => {
          // Space / Enter on the row (not inside an input) toggles select.
          const t = e.target as HTMLElement;
          if (t.tagName === "INPUT" || t.tagName === "SELECT" || t.isContentEditable) return;
          if (e.key === " " || e.key === "Enter") {
            e.preventDefault();
            toggleRowSelection(row);
          }
        }}
      >
        {/* Checkbox */}
        <div class={`${CELL_BASE} ${COL.checkbox} flex items-center justify-center`}>
          <input
            type="checkbox"
            class="accent-(--color-system-accent)"
            checked={row.selected}
            onChange={() => toggleRowSelection(row)}
            onClick={(e) => e.stopPropagation()}
            aria-label={`Select ${row.fileName}`}
          />
        </div>

        {/* File */}
        <div class={`${CELL_BASE} ${COL.file} flex items-center gap-1.5`}>
          <span class="truncate font-mono text-[0.7rem]" title={row.fileName}>
            {row.fileName}
          </span>
          <Show when={row.archiveInnerPath}>
            <span class="shrink-0 rounded border border-white/10 bg-white/[0.04] px-1 py-0.5 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)" title={row.archiveInnerPath}>
              archive
            </span>
          </Show>
          <button
            type="button"
            class="shrink-0 rounded px-1 text-[0.7rem] text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
            onClick={(e) => {
              e.stopPropagation();
              setPathPopoverKey(pathPopoverKey() === row.key ? null : row.key);
            }}
            title="Show path"
            aria-label="Show path"
          >
            ⓘ
          </button>
          <Show when={pathPopoverKey() === row.key}>
            <div
              data-oa-path-popover="true"
              class="absolute z-50 mt-6 max-w-[480px] rounded-md border border-white/10 bg-(--color-oa-bg-deep) px-3 py-2 text-[0.65rem] font-mono text-(--color-oa-ink) shadow-lg shadow-black/60"
              onClick={(e) => e.stopPropagation()}
            >
              <p class="break-all">{row.path}</p>
            </div>
          </Show>
        </div>

        {/* Detected system */}
        <div class={`${CELL_BASE} ${COL.system}`}>
          <Show
            when={editingSystem()}
            fallback={
              <button
                type="button"
                class="w-full rounded px-1.5 py-1 text-left text-xs hover:bg-white/[0.08] disabled:opacity-50"
                disabled={row.skipped}
                onClick={(e) => {
                  e.stopPropagation();
                  beginEdit(row.key, "system", row.systemId ?? "");
                }}
                title={row.systemId ?? "Click to set system"}
              >
                {systemDisplay()}
              </button>
            }
          >
            <select
              class="w-full rounded border border-(--color-system-accent)/60 bg-(--color-oa-bg-deep) px-1 py-1 text-xs text-(--color-oa-ink)"
              value={editDrafts()[`${row.key}:system`] ?? ""}
              onClick={(e) => e.stopPropagation()}
              onChange={(e) => {
                updateDraft(`${row.key}:system`, e.currentTarget.value);
                commitEdit(row.key, "system");
              }}
              onBlur={() => commitEdit(row.key, "system")}
            >
              <option value="">— pick a system —</option>
              <For each={allSystemIds}>
                {(sid) => (
                  <option value={sid}>{systemThemes[sid].displayName}</option>
                )}
              </For>
            </select>
          </Show>
        </div>

        {/* Suggested title */}
        <div class={`${CELL_BASE} ${COL.title}`}>
          <Show
            when={editingTitle()}
            fallback={
              <button
                type="button"
                class="w-full truncate rounded px-1.5 py-1 text-left text-xs hover:bg-white/[0.08] disabled:opacity-50"
                disabled={row.skipped}
                onClick={(e) => {
                  e.stopPropagation();
                  beginEdit(row.key, "title", row.title);
                }}
                title={row.title}
              >
                {row.title || <span class="text-(--color-oa-ink-dim)">—</span>}
              </button>
            }
          >
            <input
              type="text"
              class="w-full rounded border border-(--color-system-accent)/60 bg-(--color-oa-bg-deep) px-1.5 py-1 text-xs text-(--color-oa-ink)"
              value={editDrafts()[`${row.key}:title`] ?? ""}
              onInput={(e) => updateDraft(`${row.key}:title`, e.currentTarget.value)}
              onClick={(e) => e.stopPropagation()}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  commitEdit(row.key, "title");
                } else if (e.key === "Escape") {
                  e.preventDefault();
                  cancelEdit(`${row.key}:title`);
                }
              }}
              onBlur={() => commitEdit(row.key, "title")}
              autofocus
            />
          </Show>
        </div>

        {/* Confidence */}
        <div class={`${CELL_BASE} ${COL.confidence}`}>
          {renderConfidenceBadge(row.confidence)}
        </div>

        {/* Status */}
        <div class={`${CELL_BASE} ${COL.status}`}>
          {renderStatusBadge(row)}
        </div>

        {/* Actions — Skip toggle */}
        <div class={`${CELL_BASE} ${COL.actions} flex items-center justify-center`}>
          <button
            type="button"
            class="rounded border border-white/10 bg-white/[0.02] px-2 py-0.5 text-[0.6rem] uppercase tracking-widest hover:bg-white/[0.08]"
            classList={{
              "text-(--color-oa-ink-dim)": !row.skipped,
              "text-amber-300 border-amber-400/40 bg-amber-500/10": row.skipped,
            }}
            onClick={(e) => {
              e.stopPropagation();
              props.onRowChange(row.key, { skipped: !row.skipped });
            }}
            title={row.skipped ? "Unskip this ROM" : "Skip this ROM"}
          >
            {row.skipped ? "Unskip" : "Skip"}
          </button>
        </div>
      </div>
    );
  }

  // Header is rendered outside the virtualizer's scroll content so it
  // stays visible while operator scrolls the row body.
  function renderHeader(): JSX.Element {
    return (
      <div class="sticky top-0 z-10 flex items-center gap-1 border-b border-white/10 bg-(--color-oa-bg-deep)">
        <div class={`${HEADER_CELL} ${COL.checkbox} flex items-center justify-center`}>
          <input
            type="checkbox"
            class="accent-(--color-system-accent)"
            checked={headerCheckboxState() === "all"}
            ref={(el) => {
              if (el) el.indeterminate = headerCheckboxState() === "some";
            }}
            onChange={toggleHeaderSelection}
            aria-label="Select all visible"
          />
        </div>
        <button type="button" class={`${HEADER_CELL} ${COL.file} text-left hover:text-(--color-oa-ink)`} onClick={() => clickHeader("fileName")}>
          File{sortGlyph("fileName")}
        </button>
        <button type="button" class={`${HEADER_CELL} ${COL.system} text-left hover:text-(--color-oa-ink)`} onClick={() => clickHeader("systemId")}>
          Detected system{sortGlyph("systemId")}
        </button>
        <button type="button" class={`${HEADER_CELL} ${COL.title} text-left hover:text-(--color-oa-ink)`} onClick={() => clickHeader("title")}>
          Suggested title{sortGlyph("title")}
        </button>
        <button type="button" class={`${HEADER_CELL} ${COL.confidence} text-left hover:text-(--color-oa-ink)`} onClick={() => clickHeader("confidence")}>
          Conf.{sortGlyph("confidence")}
        </button>
        <button type="button" class={`${HEADER_CELL} ${COL.status} text-left hover:text-(--color-oa-ink)`} onClick={() => clickHeader("status")}>
          Status{sortGlyph("status")}
        </button>
        <div class={`${HEADER_CELL} ${COL.actions}`}></div>
      </div>
    );
  }

  function renderBulkToolbar(): JSX.Element {
    return (
      <Show when={selectedCount() > 0}>
        <div class="mb-2 flex items-center gap-2 rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/10 px-3 py-2">
          <span class="text-xs font-semibold text-(--color-system-accent-soft)">
            {selectedCount()} selected
          </span>
          <span class="text-(--color-oa-ink-dim)">·</span>
          <div class="relative">
            <button
              type="button"
              class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs hover:bg-white/[0.08]"
              onClick={(e) => {
                e.stopPropagation();
                setBulkSystemOpen(!bulkSystemOpen());
              }}
            >
              Change system ▾
            </button>
            <Show when={bulkSystemOpen()}>
              <div
                data-oa-bulk-system-popover="true"
                class="absolute left-0 top-full z-50 mt-1 max-h-[280px] w-56 overflow-y-auto rounded-md border border-white/10 bg-(--color-oa-bg-deep) py-1 shadow-lg shadow-black/60"
                onClick={(e) => e.stopPropagation()}
              >
                <For each={allSystemIds}>
                  {(sid) => (
                    <button
                      type="button"
                      class="block w-full px-3 py-1 text-left text-xs text-(--color-oa-ink) hover:bg-white/[0.08]"
                      onClick={() => {
                        const keys = selectedRows().map((r) => r.key);
                        props.onBulkChange(keys, { systemId: sid });
                        setBulkSystemOpen(false);
                      }}
                    >
                      {systemThemes[sid].displayName}
                    </button>
                  )}
                </For>
              </div>
            </Show>
          </div>
          <button
            type="button"
            class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs hover:bg-white/[0.08]"
            onClick={() => {
              const keys = selectedRows().map((r) => r.key);
              props.onBulkChange(keys, { skipped: true });
            }}
          >
            Skip
          </button>
          <button
            type="button"
            class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs hover:bg-white/[0.08]"
            onClick={() => {
              const keys = selectedRows().map((r) => r.key);
              props.onBulkChange(keys, { skipped: false });
            }}
          >
            Unskip
          </button>
          <div class="flex-1" />
          <button
            type="button"
            class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
            onClick={() => {
              const keys = selectedRows().map((r) => r.key);
              props.onBulkChange(keys, { selected: false });
            }}
          >
            Clear
          </button>
        </div>
      </Show>
    );
  }

  function renderFilterBar(): JSX.Element {
    return (
      <div class="mb-2 flex items-center gap-3">
        <input
          type="text"
          placeholder="Filter by file or title…"
          value={filterText()}
          onInput={(e) => setFilterText(e.currentTarget.value)}
          class="flex-1 rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs text-(--color-oa-ink) placeholder:text-(--color-oa-ink-dim) focus-visible:border-(--color-system-accent) focus-visible:outline-none"
        />
        <label class="flex cursor-pointer items-center gap-1.5 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          <input
            type="checkbox"
            class="accent-(--color-system-accent)"
            checked={showSkipped()}
            onChange={(e) => setShowSkipped(e.currentTarget.checked)}
          />
          Show skipped
        </label>
        <span class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          {visibleRows().length} / {props.rows().length}
        </span>
      </div>
    );
  }

  return (
    <div ref={tableContainerRef} class="flex min-h-0 flex-1 flex-col">
      {renderBulkToolbar()}
      {renderFilterBar()}
      <div class="flex min-h-0 flex-1 flex-col overflow-hidden rounded-md border border-white/10 bg-black/20">
        {renderHeader()}
        <div ref={scrollRef!} class="min-h-0 flex-1 overflow-y-auto overscroll-contain">
          <div
            style={{
              height: `${virtualizer.getTotalSize()}px`,
              width: "100%",
              position: "relative",
            }}
          >
            <For each={virtualizer.getVirtualItems()}>
              {(vItem) => {
                const row = () => visibleRows()[vItem.index];
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
                    <Show when={row()}>{renderRow(row()!)}</Show>
                  </div>
                );
              }}
            </For>
          </div>
        </div>
        <Show when={visibleRows().length === 0}>
          <div class="flex flex-1 items-center justify-center text-xs text-(--color-oa-ink-dim)">
            <Show
              when={props.rows().length === 0}
              fallback={<p>Nothing matches that filter — try different words.</p>}
            >
              <p>No ROMs found yet.</p>
            </Show>
          </div>
        </Show>
      </div>
    </div>
  );
};

export default ResultsTable;
