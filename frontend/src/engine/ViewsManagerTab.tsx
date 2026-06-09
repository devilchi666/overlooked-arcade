import { createMemo, createSignal, For, Show, type Component } from "solid-js";

import type { LibraryStore } from "@oa/platform/library/store";
import type { ViewsStore, NewViewTemplate } from "@oa/platform/views/store";
import {
  DEFAULT_VIEW_ID,
  LEGACY_VIEW_ID,
  MANUFACTURER_VIEW_ID,
} from "@oa/platform/views/defaults";
import ViewEditorPane from "./ViewEditorPane";

type Props = {
  views: ViewsStore;
  library: LibraryStore;
};

/// Library Manager → Views tab body. Per VIEW_EDITOR_PLAN.md §1, ships
/// the v3.1 surface — list + create + rename + delete + set-active.
/// Tree-structure editing (add/remove containers, edit rules) lands
/// in v3.2 as the editor pane that swaps in when a view is selected.
///
/// Shipped views (kind = user-builtin) carry an editable name + a
/// rename affordance with a warning banner; delete is disabled per
/// VIEW_EDITOR_PLAN.md §0.6 (operator-confirmed). User-built views
/// have full CRUD.
const ViewsManagerTab: Component<Props> = (props) => {
  const cfg = () => props.views.config();
  const allViews = () => cfg().views;
  const activeId = () => cfg().activeViewId;

  // v3.2 — editor mode. When set, the tab body swaps from the list to
  // the ViewEditorPane for that view. Cleared on [← Back to views].
  const [editingViewId, setEditingViewId] = createSignal<string | null>(null);
  const editingView = createMemo(() => {
    const id = editingViewId();
    if (!id) return null;
    return allViews().find((v) => v.id === id) ?? null;
  });

  // Inline rename state — only one view in rename mode at a time.
  const [renamingId, setRenamingId] = createSignal<string | null>(null);
  const [renameDraft, setRenameDraft] = createSignal("");

  // Two-step delete confirmation — first click sets the row's id here,
  // second click commits. Clearing on row blur / outside-click would
  // require capture handlers; keep it simple — operator just doesn't
  // click and the row stays armed until they navigate away.
  const [confirmingDeleteId, setConfirmingDeleteId] = createSignal<string | null>(null);

  // New-view form state.
  const [newViewOpen, setNewViewOpen] = createSignal(false);
  const [newViewName, setNewViewName] = createSignal("");
  const [newViewTemplate, setNewViewTemplate] = createSignal<NewViewTemplate>("blank");

  function beginRename(viewId: string, currentName: string) {
    setRenamingId(viewId);
    setRenameDraft(currentName);
  }

  function commitRename() {
    const id = renamingId();
    if (!id) return;
    const next = renameDraft().trim();
    if (next) props.views.renameView(id, next);
    setRenamingId(null);
    setRenameDraft("");
  }

  function cancelRename() {
    setRenamingId(null);
    setRenameDraft("");
  }

  function handleDelete(viewId: string) {
    if (confirmingDeleteId() === viewId) {
      props.views.deleteView(viewId);
      setConfirmingDeleteId(null);
    } else {
      setConfirmingDeleteId(viewId);
    }
  }

  function nameTaken(name: string, ignoreId: string | null = null): boolean {
    const lower = name.trim().toLowerCase();
    if (!lower) return false;
    return allViews().some(
      (v) => v.id !== ignoreId && v.name.trim().toLowerCase() === lower,
    );
  }

  function handleCreate() {
    const name = newViewName().trim();
    if (!name || nameTaken(name)) return;
    const newId = props.views.createView(name, newViewTemplate());
    props.views.setActiveView(newId);
    setNewViewOpen(false);
    setNewViewName("");
    setNewViewTemplate("blank");
  }

  function cancelCreate() {
    setNewViewOpen(false);
    setNewViewName("");
    setNewViewTemplate("blank");
  }

  // Identify shipped vs custom views. Shipped defaults are deletable
  // only via factory reset / views.json edit per the §0.6 lock.
  function isShipped(viewId: string): boolean {
    return (
      viewId === DEFAULT_VIEW_ID ||
      viewId === MANUFACTURER_VIEW_ID ||
      viewId === LEGACY_VIEW_ID
    );
  }

  return (
    <Show
      when={!editingView()}
      fallback={
        <ViewEditorPane
          views={props.views}
          library={props.library}
          view={editingView()!}
          onBack={() => setEditingViewId(null)}
        />
      }
    >
    <div class="max-w-2xl space-y-6">
      <header>
        <h2 class="text-sm font-semibold uppercase tracking-[0.3em] text-(--color-oa-ink)">
          Sidebar views
        </h2>
        <p class="mt-1 text-xs text-(--color-oa-ink-dim)">
          A view defines how the sidebar groups your systems. Switch the
          active view from the picker at the top of the Platforms tree.
          Editing a shipped view (Platforms / Manufacturers / Flat Legacy)
          renames it but the underlying structure may reset on a future
          app update.
        </p>
      </header>

      <ul class="space-y-1">
        <For each={allViews()}>
          {(view) => {
            const isActive = () => activeId() === view.id;
            const isRenaming = () => renamingId() === view.id;
            const armedForDelete = () => confirmingDeleteId() === view.id;
            return (
              <li
                class="relative flex items-center gap-3 rounded border px-3 py-2 text-xs transition"
                classList={{
                  "border-(--color-system-accent)/40 bg-(--color-system-accent)/[0.05]":
                    isActive(),
                  "border-white/5 bg-white/[0.02]": !isActive(),
                }}
              >
                <Show when={isActive()}>
                  <span
                    class="pointer-events-none absolute left-0 top-1/2 h-6 w-0.5 -translate-y-1/2 rounded-r bg-(--color-system-accent)"
                    aria-hidden="true"
                  />
                </Show>
                <Show
                  when={isRenaming()}
                  fallback={
                    <span class="flex-1 truncate text-(--color-oa-ink)">{view.name}</span>
                  }
                >
                  <input
                    type="text"
                    value={renameDraft()}
                    onInput={(e) => setRenameDraft(e.currentTarget.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") commitRename();
                      else if (e.key === "Escape") cancelRename();
                    }}
                    onBlur={commitRename}
                    autofocus
                    class="flex-1 rounded border border-white/15 bg-(--color-oa-bg-deep) px-2 py-1 text-(--color-oa-ink) outline-none focus:border-(--color-system-accent)"
                  />
                </Show>
                <span
                  class="rounded border px-1.5 py-0.5 text-[0.55rem] uppercase tracking-widest"
                  classList={{
                    "border-(--color-system-accent)/30 text-(--color-system-accent)":
                      isShipped(view.id),
                    "border-white/10 text-(--color-oa-ink-dim)": !isShipped(view.id),
                  }}
                >
                  {isShipped(view.id) ? "Shipped" : "Custom"}
                </span>
                <Show
                  when={!isRenaming()}
                  fallback={null}
                >
                  <button
                    type="button"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      setEditingViewId(view.id);
                    }}
                    class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-(--color-system-accent)/15 hover:text-(--color-oa-ink)"
                  >
                    Edit
                  </button>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      beginRename(view.id, view.name);
                    }}
                    class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                  >
                    Rename
                  </button>
                </Show>
                <Show when={!isShipped(view.id) && !isRenaming()}>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      handleDelete(view.id);
                    }}
                    class="rounded border px-2 py-0.5 text-[0.6rem] uppercase tracking-wider transition"
                    classList={{
                      "border-red-500/40 bg-red-500/15 text-red-200 hover:bg-red-500/25":
                        armedForDelete(),
                      "border-white/10 bg-white/[0.04] text-(--color-oa-ink-dim) hover:bg-red-500/15 hover:text-(--color-oa-ink)":
                        !armedForDelete(),
                    }}
                  >
                    {armedForDelete() ? "Confirm delete" : "Delete"}
                  </button>
                </Show>
                <Show
                  when={!isActive() && !isRenaming()}
                  fallback={
                    <Show when={isActive()}>
                      <span class="rounded border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 px-2 py-0.5 text-[0.6rem] uppercase tracking-wider text-(--color-system-accent)">
                        Active
                      </span>
                    </Show>
                  }
                >
                  <button
                    type="button"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      props.views.setActiveView(view.id);
                    }}
                    class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-(--color-system-accent)/15 hover:text-(--color-oa-ink)"
                  >
                    Set active
                  </button>
                </Show>
              </li>
            );
          }}
        </For>
      </ul>

      <Show
        when={newViewOpen()}
        fallback={
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              setNewViewOpen(true);
            }}
            class="rounded border border-dashed border-white/15 bg-white/[0.02] px-3 py-2 text-xs text-(--color-oa-ink-dim) transition hover:border-(--color-system-accent)/40 hover:bg-(--color-system-accent)/[0.05] hover:text-(--color-oa-ink)"
          >
            + New view…
          </button>
        }
      >
        <div class="space-y-3 rounded border border-white/10 bg-white/[0.03] p-3">
          <p class="text-[0.65rem] uppercase tracking-[0.3em] text-(--color-oa-ink-dim)">
            New view
          </p>
          <label class="block text-xs text-(--color-oa-ink)">
            Name
            <input
              type="text"
              value={newViewName()}
              onInput={(e) => setNewViewName(e.currentTarget.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && newViewName().trim() && !nameTaken(newViewName())) {
                  handleCreate();
                } else if (e.key === "Escape") {
                  cancelCreate();
                }
              }}
              placeholder="My Custom View"
              autofocus
              class="mt-1 block w-full rounded border border-white/15 bg-(--color-oa-bg-deep) px-2 py-1 text-(--color-oa-ink) outline-none focus:border-(--color-system-accent)"
            />
            <Show when={newViewName().trim() && nameTaken(newViewName())}>
              <span class="mt-1 block text-[0.6rem] uppercase tracking-widest text-red-300">
                A view with that name already exists
              </span>
            </Show>
          </label>
          <fieldset class="space-y-1">
            <legend class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Start from
            </legend>
            <TemplateOption
              value="blank"
              label="Blank tree"
              hint="Empty root container — build from scratch"
              current={newViewTemplate()}
              onChange={setNewViewTemplate}
            />
            <TemplateOption
              value="copy-formfactor"
              label="Copy of Platforms"
              hint="Form-factor grouping (Consoles / Handhelds / Computers / Arcade)"
              current={newViewTemplate()}
              onChange={setNewViewTemplate}
            />
            <TemplateOption
              value="copy-manufacturer"
              label="Copy of Manufacturers"
              hint="Vendor grouping (Nintendo / Sega / Sony / NEC / …)"
              current={newViewTemplate()}
              onChange={setNewViewTemplate}
            />
            <Show
              when={cfg().views.some((v) => v.id === LEGACY_VIEW_ID)}
            >
              <TemplateOption
                value="copy-legacy"
                label="Copy of Flat (Legacy)"
                hint="Single flat list in your customized system order"
                current={newViewTemplate()}
                onChange={setNewViewTemplate}
              />
            </Show>
          </fieldset>
          <div class="flex items-center gap-2">
            <button
              type="button"
              onClick={(e) => {
                e.currentTarget.blur();
                handleCreate();
              }}
              disabled={!newViewName().trim() || nameTaken(newViewName())}
              class="rounded bg-(--color-system-accent) px-3 py-1.5 text-[0.65rem] font-semibold uppercase tracking-wider text-(--color-oa-bg-deep) transition hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-50"
            >
              Create
            </button>
            <button
              type="button"
              onClick={(e) => {
                e.currentTarget.blur();
                cancelCreate();
              }}
              class="rounded border border-white/10 bg-white/[0.04] px-3 py-1.5 text-[0.65rem] font-semibold uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
            >
              Cancel
            </button>
          </div>
        </div>
      </Show>
    </div>
    </Show>
  );
};

const TemplateOption: Component<{
  value: NewViewTemplate;
  label: string;
  hint: string;
  current: NewViewTemplate;
  onChange: (v: NewViewTemplate) => void;
}> = (props) => (
  <label class="flex cursor-pointer items-start gap-2 rounded px-2 py-1 text-xs text-(--color-oa-ink) transition hover:bg-white/[0.03]">
    <input
      type="radio"
      name="new-view-template"
      value={props.value}
      checked={props.current === props.value}
      onChange={() => props.onChange(props.value)}
      class="mt-1"
    />
    <span class="flex-1">
      <span class="block text-(--color-oa-ink)">{props.label}</span>
      <span class="block text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
        {props.hint}
      </span>
    </span>
  </label>
);

export default ViewsManagerTab;
