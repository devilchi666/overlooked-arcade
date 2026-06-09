// Retroverse-UI Phase C3 Slice 12 — create / rename collection dialog.
//
// Two modes share the same Dialog surface so the operator gets a
// consistent text-input UX whether they're naming a new list (create)
// or relabeling an existing one (rename):
//
//   - create: shown from the COLLECTIONS sidebar "+ New collection"
//     button and from the TileContextMenu "+ New collection…" tail
//     entry. Optional `seedRomId` drops a rom into the new collection
//     on successful create — used by the tile-menu path so the
//     operator's "make a list and add this game to it" flow is a
//     single dialog open.
//
//   - rename: shown from the right-click context menu on a custom
//     collection sidebar row. Pre-populates the input with the
//     current name; on submit calls renameCollection.
//
// Both run inside the Dialog primitive so they inherit the inert-
// overlay + back-stack + focus-restore polish shipped in the menu-
// polish pass.

import { createEffect, createSignal, Show, type Component } from "solid-js";
import { Dialog } from "@oa/platform/components/Dialog";
import type { CustomCollectionsStore } from "@oa/platform/library/customCollections";
import type { RomId } from "@oa/platform/library/types";
// CollectionDialogMode now lives in @oa/platform/dialogs (platform owns
// dialog state).
import type { CollectionDialogMode } from "@oa/platform/dialogs";

type Props = {
  /// Non-null = open in this mode. Null = closed. The mode shape
  /// carries every piece of context the dialog needs so a single
  /// signal in App.tsx controls both surfaces.
  mode: CollectionDialogMode | null;
  onClose: () => void;
  customCollections: CustomCollectionsStore;
};

const NewCollectionDialog: Component<Props> = (props) => {
  const [name, setName] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  let inputRef: HTMLInputElement | undefined;

  const isRename = () => props.mode?.kind === "rename";
  const title = () => (isRename() ? "Rename collection" : "New collection");
  const submitLabel = () => (isRename() ? "Save" : "Create");
  const busyLabel = () => (isRename() ? "Saving…" : "Creating…");

  // Reset / hydrate the name field every time the dialog opens.
  // Rename pre-fills with the current name; create starts empty.
  createEffect(() => {
    const mode = props.mode;
    if (mode === null) return;
    setName(mode.kind === "rename" ? mode.currentName : "");
    setBusy(false);
    // Focus + select on next microtask so the Dialog overlay's mount
    // completes first.
    queueMicrotask(() => {
      inputRef?.focus();
      if (mode.kind === "rename") inputRef?.select();
    });
  });

  async function submit() {
    const trimmed = name().trim();
    const mode = props.mode;
    if (!trimmed || busy() || mode === null) return;
    setBusy(true);
    if (mode.kind === "create") {
      const id = await props.customCollections.createCollection(trimmed);
      if (id && mode.seedRomId) {
        await props.customCollections.addToCollection(id, mode.seedRomId);
      }
      setBusy(false);
      if (id) props.onClose();
    } else {
      await props.customCollections.renameCollection(mode.collectionId, trimmed);
      setBusy(false);
      props.onClose();
    }
  }

  return (
    <Dialog
      open={props.mode !== null}
      onClose={props.onClose}
      title={title()}
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
        <Show when={props.mode?.kind === "create" && (props.mode as { seedRomId: RomId | null }).seedRomId}>
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
            {busy() ? busyLabel() : submitLabel()}
          </button>
        </div>
      </form>
    </Dialog>
  );
};

export default NewCollectionDialog;
