// Retroverse-UI Phase C3 Slice 12 — create-collection dialog.
//
// Opened from the COLLECTIONS sidebar "+ New collection" button and
// from the TileContextMenu "Add to collection ▸" submenu's
// "+ New collection…" tail entry. In the tile-menu path the caller
// passes a `romId`, and on successful create the dialog also drops
// that rom into the new collection so the operator's flow matches
// "I'm right-clicking a tile because I want this game in a new list."
//
// Renders inside the Dialog primitive so it inherits the back-stack
// + focus-restore + inert-overlay polish shipped in the menu-polish
// pass.

import { createEffect, createSignal, Show, type Component } from "solid-js";
import { Dialog } from "../layout/Dialog";
import type { CustomCollectionsStore } from "../library/customCollections";
import type { RomId } from "../library/types";

type Props = {
  open: boolean;
  onClose: () => void;
  /// If non-null, the new collection automatically gets this rom as
  /// its first member on successful create. Comes from the tile-
  /// menu's "+ New collection…" path.
  seedRomId: RomId | null;
  customCollections: CustomCollectionsStore;
};

const NewCollectionDialog: Component<Props> = (props) => {
  const [name, setName] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  let inputRef: HTMLInputElement | undefined;

  // Reset the name field every time the dialog opens (so a previous
  // session's stale value doesn't leak back), and focus the input
  // after the Dialog has mounted its overlay.
  createEffect(() => {
    if (props.open) {
      setName("");
      setBusy(false);
      // Focus on the next microtask so the Dialog's mount completes
      // and the inert focus group doesn't reclaim the browser focus.
      queueMicrotask(() => inputRef?.focus());
    }
  });

  async function submit() {
    const trimmed = name().trim();
    if (!trimmed || busy()) return;
    setBusy(true);
    const id = await props.customCollections.createCollection(trimmed);
    if (id && props.seedRomId) {
      await props.customCollections.addToCollection(id, props.seedRomId);
    }
    setBusy(false);
    if (id) props.onClose();
  }

  return (
    <Dialog
      open={props.open}
      onClose={props.onClose}
      title="New collection"
      size="sm"
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          void submit();
        }}
        class="flex flex-col gap-4"
      >
        <label class="flex flex-col gap-1.5">
          <span class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            Collection name
          </span>
          <input
            ref={inputRef}
            type="text"
            value={name()}
            onInput={(e) => setName(e.currentTarget.value)}
            placeholder="e.g. Couch co-op, Long flights, Side quests"
            maxLength={120}
            disabled={busy()}
            class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-2 text-sm text-(--color-oa-ink) placeholder:text-(--color-oa-ink-dim) focus-visible:border-(--color-system-accent) focus-visible:outline-none disabled:opacity-50"
          />
        </label>
        <Show when={props.seedRomId}>
          <p class="rounded-md border border-(--color-system-accent)/30 bg-(--color-system-accent)/10 px-3 py-2 text-[0.65rem] text-(--color-oa-ink-dim)">
            The game you right-clicked will be added to this collection on
            create.
          </p>
        </Show>
        <div class="flex items-center justify-end gap-2">
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              props.onClose();
            }}
            disabled={busy()}
            class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:opacity-50"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={busy() || name().trim().length === 0}
            class="rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/20 px-3 py-1.5 text-xs font-semibold uppercase tracking-wider text-(--color-oa-ink) transition hover:bg-(--color-system-accent)/30 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {busy() ? "Creating…" : "Create"}
          </button>
        </div>
      </form>
    </Dialog>
  );
};

export default NewCollectionDialog;
