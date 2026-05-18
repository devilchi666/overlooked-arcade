import { createResource, createSignal, For, onCleanup, onMount, Show, type Component } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { launchRom } from "../library/launch";
import type { RomEntry } from "../library/types";

type SaveSlot = {
  slot: number;
  exists: boolean;
  sizeBytes: number;
  modifiedAtMs?: number;
  thumbnailDataUrl?: string;
};

type Props = {
  entry: RomEntry | null;
  onClose: () => void;
  /// Optional callback fired after a successful slot-launch, so the parent can
  /// update status / close the library overlay in single-window mode.
  onLaunchedFromSlot?: (entry: RomEntry, slot: number) => void;
};

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatModified(ms?: number): string {
  if (!ms) return "";
  const d = new Date(ms);
  return d.toLocaleString();
}

const SaveSlotsModal: Component<Props> = (props) => {
  const [refreshKey, setRefreshKey] = createSignal(0);

  const [slots, { refetch }] = createResource(
    () => (props.entry ? { path: props.entry.filePath, _: refreshKey() } : null),
    async (input): Promise<SaveSlot[]> => {
      if (!input) return [];
      return invoke<SaveSlot[]>("list_save_slots", { romPath: input.path });
    },
  );

  async function handleDelete(slot: number) {
    if (!props.entry) return;
    try {
      await invoke("delete_save_slot", { romPath: props.entry.filePath, slot });
      setRefreshKey((k) => k + 1);
      void refetch();
    } catch (e) {
      console.warn("delete_save_slot failed:", e);
    }
  }

  async function handleLaunchSlot(slot: number) {
    if (!props.entry) return;
    const entry = props.entry;
    const result = await launchRom(entry, slot);
    if (result.kind === "launched") {
      props.onLaunchedFromSlot?.(entry, slot);
      props.onClose();
    } else if (result.kind === "error") {
      console.warn("launch_rom (with slot) failed:", result.message);
    }
  }

  const escHandler = (e: KeyboardEvent) => {
    if (e.key === "Escape" && props.entry !== null) {
      e.stopPropagation();
      props.onClose();
    }
  };
  onMount(() => window.addEventListener("keydown", escHandler, { capture: true }));
  onCleanup(() => window.removeEventListener("keydown", escHandler, { capture: true }));

  return (
    <Show when={props.entry}>
      {(entry) => (
        <div
          class="fixed inset-0 z-50 grid place-items-center bg-black/60 backdrop-blur-sm"
          onClick={(e) => {
            if (e.currentTarget === e.target) props.onClose();
          }}
        >
          <div
            class="w-full max-w-3xl rounded-lg border border-white/10 bg-(--color-oa-bg-deep) shadow-2xl shadow-black/60"
            data-system={entry().systemId}
            role="dialog"
            aria-modal="true"
          >
            <header class="flex items-center justify-between border-b border-white/5 px-6 py-4">
              <div>
                <p class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-system-accent)">
                  Save states
                </p>
                <h2 class="mt-1 text-base font-semibold text-(--color-oa-ink)">
                  {entry().title}
                </h2>
              </div>
              <button
                type="button"
                onClick={(e) => {
                  e.currentTarget.blur();
                  props.onClose();
                }}
                class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
              >
                Close
              </button>
            </header>

            <section class="grid grid-cols-2 gap-3 px-6 py-6 sm:grid-cols-3 md:grid-cols-5">
              <For each={slots() ?? []}>
                {(s) => (
                  <article
                    class="group flex flex-col overflow-hidden rounded-md border border-white/5 bg-white/[0.03] transition"
                    classList={{
                      "opacity-50": !s.exists,
                      "hover:border-(--color-system-accent)/60 hover:bg-white/[0.06]": s.exists,
                    }}
                  >
                    <button
                      type="button"
                      disabled={!s.exists}
                      onClick={(e) => {
                        e.currentTarget.blur();
                        void handleLaunchSlot(s.slot);
                      }}
                      class="relative aspect-[4/3] w-full overflow-hidden bg-black/40 text-left transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent) disabled:cursor-not-allowed"
                      title={s.exists ? `Launch with slot ${s.slot} restored` : "Empty slot"}
                    >
                      <Show
                        when={s.thumbnailDataUrl}
                        fallback={
                          <div class="grid h-full place-items-center text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                            {s.exists ? "no thumb" : "empty"}
                          </div>
                        }
                      >
                        {(url) => (
                          <img
                            src={url()}
                            alt={`Slot ${s.slot} thumbnail`}
                            class="absolute inset-0 h-full w-full object-contain"
                          />
                        )}
                      </Show>
                      <span class="absolute left-1 top-1 rounded bg-black/60 px-1.5 py-0.5 text-[0.6rem] font-semibold text-(--color-system-accent-soft) backdrop-blur">
                        {s.slot}
                      </span>
                      <Show when={s.exists}>
                        <span class="pointer-events-none absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/70 to-transparent px-2 py-1 text-[0.6rem] font-medium uppercase tracking-widest text-(--color-system-accent-soft) opacity-0 transition-opacity group-hover:opacity-100">
                          Launch + restore
                        </span>
                      </Show>
                    </button>
                    <div class="flex flex-col gap-0.5 px-2 py-2 text-[0.65rem] text-(--color-oa-ink-dim)">
                      <Show
                        when={s.exists}
                        fallback={<span>empty slot</span>}
                      >
                        <span>{formatSize(s.sizeBytes)}</span>
                        <span class="truncate">{formatModified(s.modifiedAtMs)}</span>
                        <button
                          type="button"
                          onClick={(e) => {
                            e.currentTarget.blur();
                            handleDelete(s.slot);
                          }}
                          class="mt-1 rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                        >
                          Delete
                        </button>
                      </Show>
                    </div>
                  </article>
                )}
              </For>
            </section>

            <footer class="border-t border-white/5 px-6 py-3 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Click a saved slot to launch and restore it. In-game: pick a slot with
              <code class="font-mono normal-case"> 0</code>–
              <code class="font-mono normal-case">9</code> then
              <code class="font-mono normal-case"> F5</code>/
              <code class="font-mono normal-case">F8</code> to save/load.
            </footer>
          </div>
        </div>
      )}
    </Show>
  );
};

export default SaveSlotsModal;
