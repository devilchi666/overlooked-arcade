// Collections manager — CRUD over the operator-built custom collections
// (platform/library/customCollections). New in the Settings IA redesign
// (Slice 1): collections are library *organization* (flat curated member sets
// like "Shmups I own"), distinct from sidebar layouts (Views) and from
// appearance — so they live under the new Organize landing.
//
// This surface manages the collection ROWS only (create / rename / delete).
// Membership (which games are in a collection) is edited from the library via
// the tile context menu's "Add to collection" submenu — unchanged.

import { createSignal, For, onMount, Show, type Component } from "solid-js";
import { usePlatform } from "@oa/platform/platformContext";
import { confirm } from "@oa/platform/lib/confirm";
import { pushToast } from "@oa/platform/lib/toast";

export const CollectionsManager: Component = () => {
  const { customCollections } = usePlatform();

  // Refresh on mount so an operator who created collections elsewhere this
  // session sees them (cheap; the store also hydrates at startup).
  onMount(() => {
    void customCollections.refresh();
  });

  const [newName, setNewName] = createSignal("");
  const [renamingId, setRenamingId] = createSignal<string | null>(null);
  const [renameDraft, setRenameDraft] = createSignal("");

  async function createNew() {
    const name = newName().trim();
    if (!name) return;
    setNewName("");
    const id = await customCollections.createCollection(name);
    if (id) pushToast("success", `Created collection “${name}”.`);
  }

  function beginRename(id: string, current: string) {
    setRenamingId(id);
    setRenameDraft(current);
  }
  async function commitRename(id: string) {
    const name = renameDraft().trim();
    setRenamingId(null);
    if (name) await customCollections.renameCollection(id, name);
  }

  async function removeCollection(id: string, name: string) {
    if (!(await confirm(
      `Delete the “${name}” collection? The games stay in your library — only the collection grouping is removed.`,
      { title: "Delete collection", confirmLabel: "Delete", danger: true },
    ))) return;
    await customCollections.deleteCollection(id);
    pushToast("success", `Deleted “${name}”.`);
  }

  return (
    <div class="space-y-3">
      <h3 class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
        Collections
      </h3>
      <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
        Curated sets of games that cross system boundaries (e.g. “Shmups I own”).
        Add games to a collection from the library — right-click a tile → Add to
        collection.
      </p>

      <Show
        when={customCollections.state.collections.length > 0}
        fallback={
          <p class="text-xs text-(--color-oa-ink-dim)">
            No collections yet. Create one below, then add games from the library.
          </p>
        }
      >
        <ul class="space-y-1">
          <For each={customCollections.state.collections}>
            {(c) => (
              <li class="flex items-center justify-between gap-3 rounded border border-white/5 bg-white/[0.02] px-3 py-2 text-xs">
                <Show
                  when={renamingId() === c.id}
                  fallback={
                    <span class="flex-1 truncate text-(--color-oa-ink)">{c.name}</span>
                  }
                >
                  <input
                    class="flex-1 rounded border border-white/15 bg-black/30 px-2 py-1 text-(--color-oa-ink) outline-none focus:border-(--color-system-accent)"
                    value={renameDraft()}
                    onInput={(e) => setRenameDraft(e.currentTarget.value)}
                    onBlur={() => void commitRename(c.id)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") void commitRename(c.id);
                      if (e.key === "Escape") setRenamingId(null);
                    }}
                    ref={(el) => queueMicrotask(() => el.focus())}
                  />
                </Show>
                <span class="shrink-0 text-(--color-oa-ink-dim) tabular-nums">
                  {c.memberCount} {c.memberCount === 1 ? "game" : "games"}
                </span>
                <div class="flex shrink-0 gap-2">
                  <button
                    type="button"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      beginRename(c.id, c.name);
                    }}
                    class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                  >
                    Rename
                  </button>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      void removeCollection(c.id, c.name);
                    }}
                    class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-red-500/15 hover:text-(--color-oa-ink)"
                  >
                    Delete
                  </button>
                </div>
              </li>
            )}
          </For>
        </ul>
      </Show>

      <div class="flex items-center gap-2 pt-1">
        <input
          class="flex-1 rounded border border-white/15 bg-black/30 px-2 py-1 text-xs text-(--color-oa-ink) outline-none focus:border-(--color-system-accent)"
          placeholder="New collection name…"
          value={newName()}
          onInput={(e) => setNewName(e.currentTarget.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") void createNew();
          }}
        />
        <button
          type="button"
          disabled={!newName().trim()}
          onClick={(e) => {
            e.currentTarget.blur();
            void createNew();
          }}
          class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:cursor-not-allowed disabled:opacity-50"
        >
          Create
        </button>
      </div>
    </div>
  );
};

export default CollectionsManager;
