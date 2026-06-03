// GameMediaManagePanel — per-system "granular ops" side panel.
//
// Slides in from the right of the Game-media tab when the operator
// clicks [Manage…] on a system card. Exposes the five granular ops
// (Sync media / Sync metadata / Clear metadata / Sync hashes /
// Identify ROMs) with one-line descriptions + the system's current
// counts, each as a single [Run] button.
//
// All ops route through the existing handlers in LibraryManagerPage —
// this component is pure UI, the parent owns state. Kicked-off ops
// surface progress through the BackgroundJobsBar at the bottom of the
// viewport.
//
// Plan: docs/PLANS/settings-declutter-system-health.md Phase 5.

import {
  createMemo,
  For,
  onCleanup,
  onMount,
  Show,
  type Accessor,
  type Component,
} from "solid-js";
import { systemThemes, type SystemId } from "../themes/registry";

export type GameMediaManageStats = {
  total: number;
  identified: number;
  covered: number;
  metadataed: number;
};

export type GameMediaManagePanelProps = {
  /// Active system id. When null, the panel renders nothing — the
  /// parent controls open/closed state by toggling this signal.
  systemId: Accessor<SystemId | null>;
  /// Stats for the active system. Updates live as ops complete.
  stats: Accessor<GameMediaManageStats | undefined>;
  /// True while ANY op is in flight on this system. Disables every
  /// [Run] button to prevent button-mash sequences (server-side
  /// per-system mutex serializes anyway; this is a UX guard).
  busy: Accessor<boolean>;
  /// Op-runner callbacks — these wrap the existing handlers in
  /// LibraryManagerPage. The panel doesn't own any op-running state.
  onSyncMedia: (id: SystemId) => Promise<void> | void;
  onSyncMetadata: (id: SystemId) => Promise<void> | void;
  onClearMetadata: (id: SystemId) => Promise<void> | void;
  onSyncHashes: (id: SystemId) => Promise<void> | void;
  onIdentifyRoms: (id: SystemId) => Promise<void> | void;
  onClose: () => void;
};

type Op = {
  id: "identify" | "covers" | "metadata" | "clear-metadata" | "hashes";
  title: string;
  description: string;
  ctaLabel: string;
  destructive?: boolean;
};

const OPS: readonly Op[] = [
  {
    id: "identify",
    title: "Identify ROMs",
    description:
      "Hash every ROM against the libretro-database hash DB and overwrite filenames with the canonical title on a match.",
    ctaLabel: "Run",
  },
  {
    id: "covers",
    title: "Sync covers",
    description:
      "Pull boxart / snapshots / title screens from the libretro-thumbnails repo for every identified ROM. Honors the 'Only sync identified' + 'Kinds to fetch' preferences above.",
    ctaLabel: "Run",
  },
  {
    id: "metadata",
    title: "Sync metadata",
    description:
      "Fetch year / genre / developer / publisher from the libretro-database catalog and stamp it onto MediaDb's GameMetadata for every identified ROM.",
    ctaLabel: "Run",
  },
  {
    id: "clear-metadata",
    title: "Clear metadata",
    description:
      "Wipe year / genre / developer / publisher fields for every game in this system. Cover art is NOT touched. Used to scrub stale cross-system data; re-run Sync metadata after to repopulate.",
    ctaLabel: "Run",
    destructive: true,
  },
  {
    id: "hashes",
    title: "Refresh hash database",
    description:
      "Re-pull the libretro-database hash DAT for this system into our local rom_hashes table. Cached; the cached copy is reused for one week.",
    ctaLabel: "Refresh",
  },
];

const GameMediaManagePanel: Component<GameMediaManagePanelProps> = (props) => {
  const open = createMemo(() => props.systemId() !== null);

  const systemName = createMemo(() => {
    const id = props.systemId();
    if (!id) return "";
    return systemThemes[id]?.displayName ?? id;
  });

  function onEsc(e: KeyboardEvent) {
    if (e.key === "Escape" && open()) {
      e.stopPropagation();
      props.onClose();
    }
  }

  onMount(() => {
    window.addEventListener("keydown", onEsc, true);
  });
  onCleanup(() => {
    window.removeEventListener("keydown", onEsc, true);
  });

  function runOp(op: Op) {
    const id = props.systemId();
    if (!id) return;
    switch (op.id) {
      case "identify":
        void props.onIdentifyRoms(id);
        break;
      case "covers":
        void props.onSyncMedia(id);
        break;
      case "metadata":
        void props.onSyncMetadata(id);
        break;
      case "clear-metadata":
        void props.onClearMetadata(id);
        break;
      case "hashes":
        void props.onSyncHashes(id);
        break;
    }
  }

  function opCountLine(op: Op): string | undefined {
    const s = props.stats();
    if (!s) return undefined;
    switch (op.id) {
      case "identify":
        return `${s.identified}/${s.total} identified`;
      case "covers":
        return `${s.covered}/${s.total} with covers`;
      case "metadata":
        return `${s.metadataed}/${s.total} with metadata`;
      case "clear-metadata":
        return s.metadataed > 0 ? `${s.metadataed} entries to clear` : "Nothing to clear";
      case "hashes":
        return undefined;
    }
  }

  return (
    <Show when={open()}>
      {/* Backdrop — semi-transparent overlay that closes on click. Sits
          below the panel itself so clicks on the panel don't bubble
          and trigger close. */}
      <div
        class="fixed inset-0 z-40 bg-black/40 backdrop-blur-[1px]"
        onClick={(e) => {
          e.stopPropagation();
          props.onClose();
        }}
        aria-hidden="true"
      />

      {/* Panel — slides in from the right. z-45 sits above page chrome
          and below the BackgroundJobsBar (z-55) + modals (z-50). */}
      <aside
        role="dialog"
        aria-modal="true"
        aria-label={`${systemName()} game media operations`}
        data-system={props.systemId() ?? undefined}
        class="fixed right-0 top-0 z-45 flex h-full w-full max-w-[480px] flex-col gap-4 overflow-y-auto border-l border-white/10 bg-(--color-oa-bg-deep) p-6 shadow-2xl"
      >
        <header class="flex items-start justify-between gap-3">
          <div class="min-w-0">
            <p class="text-[0.55rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
              Game media ops
            </p>
            <h2 class="mt-1 truncate text-lg font-semibold text-(--color-oa-ink)">
              {systemName()}
            </h2>
            <Show when={props.stats()}>
              {(s) => (
                <p class="mt-0.5 text-[0.7rem] text-(--color-oa-ink-dim)">
                  {s().total} game{s().total === 1 ? "" : "s"} in library
                </p>
              )}
            </Show>
          </div>
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              props.onClose();
            }}
            class="rounded-md border border-white/10 bg-white/[0.04] px-2.5 py-1 text-xs text-(--color-oa-ink-dim) transition hover:border-white/20 hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
            aria-label="Close panel"
          >
            ✕
          </button>
        </header>

        <ul class="flex flex-col gap-3">
          <For each={OPS}>
            {(op) => (
              <li
                class="rounded-lg border bg-white/[0.02] px-3 py-3"
                classList={{
                  "border-white/10": !op.destructive,
                  "border-rose-500/20": !!op.destructive,
                }}
              >
                <div class="flex items-start justify-between gap-3">
                  <div class="min-w-0 flex-1">
                    <h3
                      class="text-[0.8rem] font-semibold"
                      classList={{
                        "text-(--color-oa-ink)": !op.destructive,
                        "text-rose-200": !!op.destructive,
                      }}
                    >
                      {op.title}
                    </h3>
                    <p class="mt-1 text-[0.65rem] leading-relaxed text-(--color-oa-ink-dim)">
                      {op.description}
                    </p>
                    <Show when={opCountLine(op)}>
                      <p class="mt-1.5 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)/80">
                        {opCountLine(op)}
                      </p>
                    </Show>
                  </div>
                  <button
                    type="button"
                    disabled={props.busy()}
                    onClick={(e) => {
                      e.currentTarget.blur();
                      runOp(op);
                    }}
                    class="shrink-0 rounded-md px-3 py-1.5 text-[0.65rem] font-semibold uppercase tracking-wider transition disabled:cursor-not-allowed disabled:opacity-50"
                    classList={{
                      "border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 text-(--color-oa-ink) hover:border-(--color-system-accent) hover:bg-(--color-system-accent)/25":
                        !op.destructive,
                      "border border-rose-500/30 bg-rose-500/10 text-rose-200 hover:border-rose-400/60 hover:bg-rose-500/20":
                        !!op.destructive,
                    }}
                  >
                    {op.ctaLabel}
                  </button>
                </div>
              </li>
            )}
          </For>
        </ul>

        <footer class="mt-auto flex justify-end pt-2">
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              props.onClose();
            }}
            class="rounded-md border border-white/10 bg-white/[0.04] px-4 py-1.5 text-[0.7rem] font-semibold uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:border-white/20 hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
          >
            Done
          </button>
        </footer>
      </aside>
    </Show>
  );
};

export default GameMediaManagePanel;
