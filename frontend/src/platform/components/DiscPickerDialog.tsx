// DiscPickerDialog — Phase A1 Sub-phase 4 disc selection overlay.
//
// When the operator clicks a library tile representing a multi-disc set
// (entry.discSetId !== undefined), this dialog opens with one button per
// disc in the set, sorted by disc_number. Click a disc → close + launch
// that specific game.
//
// Triggered from LibraryView's wrapped onLaunch handler. Lives outside
// VirtualLibraryGrid / DetailListView so both view modes share the
// same picker.

import {
  createSignal,
  For,
  onCleanup,
  onMount,
  Show,
  type Component,
} from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import type { RomEntry } from "@oa/platform/library/types";

type Props = {
  /// Representative entry of the disc-set tile that was clicked. Has
  /// .discSetId set (non-undefined). Title is already stripped of the
  /// "(Disc N)" suffix by `collapseDiscSets`.
  entry: RomEntry;
  /// Called when the operator picks a disc. The member is one of the
  /// games returned by `list_disc_set_members` (full RomEntry with
  /// real file_path etc.) — caller forwards to the existing launch
  /// path.
  onLaunch: (member: RomEntry) => void;
  /// Operator dismissed the picker (clicked backdrop, pressed Escape,
  /// or hit "Cancel"). Caller clears the picker entry signal so the
  /// dialog unmounts.
  onClose: () => void;
};

const DiscPickerDialog: Component<Props> = (props) => {
  const [members, setMembers] = createSignal<RomEntry[]>([]);
  const [loadError, setLoadError] = createSignal<string | null>(null);

  // Fetch on mount. The Tauri command list_disc_set_members returns
  // members sorted by disc_number ASC already; we render in that
  // order with no further sort.
  onMount(() => {
    const setId = props.entry.discSetId;
    if (setId === undefined) {
      setLoadError("Tile has no discSetId");
      return;
    }
    invoke<RomEntry[]>("list_disc_set_members", { discSetId: setId })
      .then((m) => setMembers(m))
      .catch((e) => {
        console.warn("DiscPickerDialog: list_disc_set_members failed:", e);
        setLoadError(String(e));
      });
  });

  // Escape closes the picker without launching.
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      props.onClose();
    }
  };
  window.addEventListener("keydown", onKeyDown);
  onCleanup(() => window.removeEventListener("keydown", onKeyDown));

  const pickDisc = (member: RomEntry) => {
    props.onLaunch(member);
    props.onClose();
  };

  return (
    <div
      class="fixed inset-0 z-[70] grid place-items-center bg-black/70 backdrop-blur-sm p-6"
      role="dialog"
      aria-modal="true"
      aria-labelledby="oa-disc-picker-title"
      onClick={(e) => {
        if (e.currentTarget === e.target) props.onClose();
      }}
    >
      <div class="flex w-full max-w-md flex-col overflow-hidden rounded-xl border border-white/10 bg-(--color-oa-bg-deep) shadow-2xl">
        <header class="flex items-start gap-3 border-b border-white/5 px-5 py-4">
          <span
            class="mt-0.5 inline-block h-7 w-7 shrink-0 text-center text-xl leading-7"
            aria-hidden="true"
          >
            💿
          </span>
          <div class="min-w-0 flex-1">
            <h2
              id="oa-disc-picker-title"
              class="text-sm font-semibold uppercase tracking-[0.25em] text-(--color-oa-ink)"
            >
              Pick a disc
            </h2>
            <p class="mt-1.5 truncate text-[0.85rem] text-(--color-oa-ink)">
              {props.entry.title}
            </p>
          </div>
        </header>
        <div class="flex flex-col gap-2 px-5 py-4">
          <Show
            when={loadError() === null}
            fallback={
              <p class="rounded-md border border-rose-500/30 bg-rose-500/10 px-3 py-2 text-[0.7rem] text-rose-200">
                Couldn't load disc list: {loadError()}
              </p>
            }
          >
            <Show
              when={members().length > 0}
              fallback={
                <p class="text-[0.75rem] text-(--color-oa-ink-dim)">Loading…</p>
              }
            >
              <For each={members()}>
                {(member) => (
                  <button
                    type="button"
                    class="flex items-center justify-between gap-3 rounded-md border border-white/10 bg-white/[0.02] px-4 py-3 text-left text-(--color-oa-ink) transition hover:border-(--color-system-accent)/60 hover:bg-(--color-system-accent)/10"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      pickDisc(member);
                    }}
                  >
                    <span class="text-[0.85rem] font-semibold">
                      Disc {member.discNumber ?? "?"}
                    </span>
                    <span class="truncate text-[0.7rem] text-(--color-oa-ink-dim)">
                      {member.title}
                    </span>
                  </button>
                )}
              </For>
            </Show>
          </Show>
        </div>
        <div class="flex items-center justify-end gap-2 bg-white/[0.02] px-5 py-3">
          <button
            type="button"
            class="rounded-md border border-white/10 px-3 py-1.5 text-xs font-semibold uppercase tracking-wider text-(--color-oa-ink) transition hover:border-white/30 hover:bg-white/5"
            onClick={(e) => {
              e.currentTarget.blur();
              props.onClose();
            }}
          >
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
};

export default DiscPickerDialog;
