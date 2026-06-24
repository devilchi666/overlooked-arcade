import { createMemo, createSignal, For, Show, type Component } from "solid-js";
import {
  closestCenter,
  DragDropProvider,
  DragDropSensors,
  SortableProvider,
  type DragEventHandler,
} from "@thisbeyond/solid-dnd";

import type { LibraryStore } from "@oa/platform/library/store";
import { systemThemes, type SystemId, type FormFactorTag, type ManufacturerTag } from "@oa/platform/themes/registry";
import {
  SortableContainerNode,
  SortableLeafNode,
  type SidebarTreeContext,
} from "@oa/platform/components/SidebarTreeNode";
import { countGameSetNode, findNode, findNodePath, isGameSetNode } from "@oa/platform/views/resolver";
import { resolveDragOutcome } from "@oa/platform/views/dragResolve";
import type { ViewsStore } from "@oa/platform/views/store";
import type {
  ContainerNode,
  ContainerRule,
  PlatformNode,
  View,
  ViewNode,
} from "@oa/platform/views/types";
import { DEFAULT_VIEW_ID, LEGACY_VIEW_ID, MANUFACTURER_VIEW_ID, ROOT_NODE_ID } from "@oa/platform/views/defaults";
import type { CustomCollection, CustomCollectionsStore } from "@oa/platform/library/customCollections";
import {
  asSmartListKind,
  SMART_LISTS,
  SMART_LISTS_BY_KIND,
  type SmartListKind,
} from "@oa/platform/library/smartLists";

type Props = {
  views: ViewsStore;
  library: LibraryStore;
  customCollections: CustomCollectionsStore;
  view: View;
  onBack: () => void;
};

/// Two-pane editor for a single view per VIEW_EDITOR_PLAN.md §0.3 +
/// §2. Tree on the left with selection-by-click, drag-reorder (reuses
/// γ.2 + v2.2.1's sortable nodes), and inline [+ Add container] /
/// [+ Add leaf] affordances. Properties pane on the right shows
/// editable fields for the selected node (label / rule kind / rule
/// value / delete) with per-kind editors.
///
/// All edits are live (write-through to views.json via ViewsStore) —
/// no Save/Cancel buttons per the §0.2 lock. Shipped views are
/// editable here with the same affordances as user-built views;
/// VIEW_EDITOR_PLAN.md §0.6 documents the warning that upgrades may
/// reset shipped-view structure.
const ViewEditorPane: Component<Props> = (props) => {
  const [selectedNodeId, setSelectedNodeId] = createSignal<string | null>(null);

  const selectedNode = createMemo(() => {
    const id = selectedNodeId();
    if (!id) return null;
    return findNode(props.view, id);
  });

  const isShipped = () =>
    props.view.id === DEFAULT_VIEW_ID ||
    props.view.id === MANUFACTURER_VIEW_ID ||
    props.view.id === LEGACY_VIEW_ID;

  /// Default target for [+ Add ...] buttons: the selected container if
  /// any, else root. Leaves can't be a target — they're not containers.
  /// Game-set nodes (collection / smart-list filter) are containers in the
  /// data model but their children are ignored by the resolver + unrendered
  /// in the tree, so they aren't valid add-targets either — adding into one
  /// falls back to root rather than silently orphaning the new node.
  const addTargetId = createMemo<string>(() => {
    const sel = selectedNode();
    if (sel && "children" in sel && !isGameSetNode(sel)) return sel.id;
    return ROOT_NODE_ID;
  });

  function handleAddContainer() {
    const newId = props.views.addContainer(addTargetId(), "New container", null);
    if (newId) setSelectedNodeId(newId);
  }

  function handleAddLeaf(systemId: SystemId) {
    const newId = props.views.addPlatformLeaf(addTargetId(), systemId);
    if (newId) setSelectedNodeId(newId);
  }

  /// Unified Navigation Tree Slice 1 — add a smart-list filter node. One of
  /// the six built-ins, persisted as a `filter`-rule container; the resolver
  /// evaluates its predicate at render. The label defaults to the built-in's
  /// name (operator can rename it in the properties pane).
  function handleAddSmartList(kind: SmartListKind) {
    const def = SMART_LISTS_BY_KIND[kind];
    const newId = props.views.addContainer(addTargetId(), def.label, {
      kind: "filter",
      spec: { kind },
    });
    if (newId) setSelectedNodeId(newId);
  }

  /// Unified Navigation Tree Slice 1 — add a curated-collection node. The
  /// `collection`-rule container resolves to the collection's member set.
  function handleAddCollection(collection: CustomCollection) {
    const newId = props.views.addContainer(addTargetId(), collection.name, {
      kind: "collection",
      collectionId: collection.id,
    });
    if (newId) setSelectedNodeId(newId);
  }

  /// Membership count for game-set rows in the editor tree. Collections read
  /// the eager `memberCount`; smart-list filters evaluate over the entries.
  const collectionCounts = createMemo(
    () => new Map(props.customCollections.state.collections.map((c) => [c.id, c.memberCount])),
  );
  const gameSetCount = (node: ContainerNode): number =>
    countGameSetNode(node, {
      entries: props.library.state.entries.filter((e) => !e.seed),
      collectionMemberCounts: collectionCounts(),
      nowSecs: Math.floor(Date.now() / 1000),
    }) ?? 0;

  function handleRemove(nodeId: string) {
    props.views.removeNode(nodeId);
    if (selectedNodeId() === nodeId) setSelectedNodeId(null);
  }

  // Tree context for the editor — selection by click, expand toggles
  // the persisted state (shared with the sidebar's expanded set —
  // operator expands a container in editor → sidebar reflects it too).
  const editorCtx: SidebarTreeContext = {
    get entries() {
      return props.library.state.entries;
    },
    isExpanded: (nodeId: string) => props.view.expandedNodes.includes(nodeId),
    isActiveNode: (nodeId: string) => selectedNodeId() === nodeId,
    onToggleExpanded: (nodeId: string) => props.views.toggleExpanded(nodeId),
    onNavigateToNode: (nodeId: string) => setSelectedNodeId(nodeId),
    gameSetCount,
    // Slice 2 — the editor is the deep-drag authoring surface.
    nestedSortable: true,
  };

  // Drag-reorder is resolved by the shared, unit-tested `resolveDragOutcome`
  // (platform/views/dragResolve). It generalizes the old leaf-only, 2-level
  // logic to any node kind at any depth: same-parent → reorder; cross-parent
  // onto a folder → nest into it; cross-parent onto a row → insert beside it.
  // Cycles + the root are rejected at the helper.
  const topLevelIds = createMemo(() => props.view.root.children.map((c) => c.id));

  const handleDragEnd: DragEventHandler = ({ draggable, droppable }) => {
    if (!droppable) return;
    const outcome = resolveDragOutcome(
      props.view,
      String(draggable.id),
      String(droppable.id),
    );
    if (!outcome) return;
    if (outcome.kind === "reorder") {
      props.views.reorderChildren(outcome.parentId, outcome.order);
    } else {
      props.views.moveNode(outcome.nodeId, outcome.targetParentId, outcome.insertBeforeId);
    }
  };

  return (
    <div class="flex h-full min-h-0 max-w-5xl flex-col gap-4">
      <header class="flex items-center gap-3">
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            props.onBack();
          }}
          class="rounded border border-white/10 bg-white/[0.04] px-3 py-1.5 text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
        >
          ← Back to views
        </button>
        <h2 class="flex-1 truncate text-sm font-semibold uppercase tracking-[0.3em] text-(--color-oa-ink)">
          Editing: {props.view.name}
        </h2>
        <span
          class="rounded border px-1.5 py-0.5 text-[0.55rem] uppercase tracking-widest"
          classList={{
            "border-(--color-system-accent)/30 text-(--color-system-accent)": isShipped(),
            "border-white/10 text-(--color-oa-ink-dim)": !isShipped(),
          }}
        >
          {isShipped() ? "Shipped" : "Custom"}
        </span>
      </header>

      <Show when={isShipped()}>
        <div class="rounded-md border border-amber-500/30 bg-amber-500/[0.06] p-3 text-[0.7rem] text-amber-100">
          Editing a shipped view. Your changes persist across launches, but
          a future app update may reset this view's structure if the default
          changes substantially. For lasting customization, consider
          creating a Custom view from a copy of this one.
        </div>
      </Show>

      <div class="grid min-h-0 flex-1 grid-cols-[1fr_22rem] gap-4">
        {/* ── Tree pane ─────────────────────────────────────── */}
        <div class="flex min-h-0 flex-col gap-2 rounded border border-white/5 bg-white/[0.02] p-3">
          <div class="flex items-center justify-between">
            <h3 class="text-[0.6rem] uppercase tracking-[0.3em] text-(--color-oa-ink-dim)">
              Tree
            </h3>
            <p class="text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Add into: <span class="text-(--color-oa-ink)">
                {addTargetId() === ROOT_NODE_ID
                  ? "root"
                  : (findNode(props.view, addTargetId()) as ContainerNode | null)?.label ?? addTargetId()}
              </span>
            </p>
          </div>

          <div class="min-h-0 flex-1 overflow-y-auto">
            <DragDropProvider onDragEnd={handleDragEnd} collisionDetector={closestCenter}>
              <DragDropSensors />
              <SortableProvider ids={topLevelIds()}>
                <ul class="space-y-0.5">
                  <For each={props.view.root.children}>
                    {(child) =>
                      child.kind === "container" ? (
                        <SortableContainerNode
                          container={child as ContainerNode}
                          depth={0}
                          ctx={editorCtx}
                        />
                      ) : (
                        <SortableLeafNode
                          leaf={child as PlatformNode}
                          depth={0}
                          ctx={editorCtx}
                        />
                      )
                    }
                  </For>
                </ul>
              </SortableProvider>
            </DragDropProvider>
          </div>

          <div class="flex flex-wrap gap-2 border-t border-white/5 pt-2">
            <button
              type="button"
              onClick={(e) => {
                e.currentTarget.blur();
                handleAddContainer();
              }}
              class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-(--color-system-accent)/15 hover:text-(--color-oa-ink)"
            >
              + Container
            </button>
            <AddLeafPicker view={props.view} onPick={handleAddLeaf} />
            <AddSmartListPicker onPick={handleAddSmartList} />
            <AddCollectionPicker
              customCollections={props.customCollections}
              onPick={handleAddCollection}
            />
          </div>
        </div>

        {/* ── Properties pane ───────────────────────────────── */}
        <div class="min-h-0 overflow-y-auto rounded border border-white/5 bg-white/[0.02] p-3">
          <Show
            when={selectedNode()}
            fallback={
              <p class="text-xs text-(--color-oa-ink-dim)">
                Select a node from the tree to edit its properties.
              </p>
            }
          >
            {(node) => (
              <Show
                when={"children" in node()}
                fallback={
                  <LeafProperties
                    leaf={node() as PlatformNode}
                    onRemove={() => handleRemove(node().id)}
                  />
                }
              >
                <ContainerProperties
                  container={node() as ContainerNode}
                  isRoot={node().id === ROOT_NODE_ID}
                  isTopLevel={(findNodePath(props.view, node().id)?.length ?? 0) <= 2}
                  views={props.views}
                  customCollections={props.customCollections}
                  onRemove={() => handleRemove(node().id)}
                />
              </Show>
            )}
          </Show>
        </div>
      </div>
    </div>
  );
};

// ── Subcomponents ─────────────────────────────────────────────────

const ContainerProperties: Component<{
  container: ContainerNode;
  isRoot: boolean;
  /// True when this node's parent is the root (i.e. it's a top-level node).
  /// "Filter within parent" has no effect here — the root is "all games" —
  /// so the UI annotates that case rather than misleading the operator.
  isTopLevel: boolean;
  views: ViewsStore;
  customCollections: CustomCollectionsStore;
  onRemove: () => void;
}> = (props) => {
  const ruleKind = () => props.container.rule?.kind ?? "none";
  /// Game-set nodes (collection / smart-list filter) are authored via the
  /// dedicated "+ Smart List" / "+ Collection" buttons, not the system-rule
  /// radio. Their properties pane identifies the kind + lets the operator
  /// re-pick the target, rather than offering the system-keyed rule radio.
  const isGameSet = () =>
    ruleKind() === "collection" || ruleKind() === "filter";

  function setKind(kind: "none" | ContainerRule["kind"]) {
    if (kind === "none") {
      props.views.setContainerRule(props.container.id, null);
    } else if (kind === "formFactor") {
      props.views.setContainerRule(props.container.id, { kind: "formFactor", value: "console" });
    } else if (kind === "manufacturer") {
      props.views.setContainerRule(props.container.id, { kind: "manufacturer", value: "nintendo" });
    } else if (kind === "systemIds") {
      props.views.setContainerRule(props.container.id, { kind: "systemIds", values: [] });
    }
  }

  return (
    <div class="space-y-3 text-xs">
      <p class="text-[0.6rem] uppercase tracking-[0.3em] text-(--color-oa-ink-dim)">
        Container
      </p>
      <label class="block">
        <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          Label
        </span>
        <input
          type="text"
          value={props.container.label}
          onInput={(e) => props.views.setContainerLabel(props.container.id, e.currentTarget.value)}
          class="mt-1 block w-full rounded border border-white/15 bg-(--color-oa-bg-deep) px-2 py-1 text-(--color-oa-ink) outline-none focus:border-(--color-system-accent)"
        />
      </label>

      <Show
        when={!props.isRoot}
        fallback={
          <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            Root container — rule is always "match everything"
          </p>
        }
      >
        <Show when={isGameSet()}>
          <GameSetRuleEditor
            container={props.container}
            views={props.views}
            customCollections={props.customCollections}
          />
        </Show>

        <Show when={!isGameSet()}>
        <fieldset class="space-y-1">
          <legend class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            Rule
          </legend>
          <RuleKindRadio current={ruleKind()} value="none" label="None (match everything)" onChange={setKind} />
          <RuleKindRadio current={ruleKind()} value="formFactor" label="Form factor" onChange={setKind} />
          <RuleKindRadio current={ruleKind()} value="manufacturer" label="Manufacturer" onChange={setKind} />
          <RuleKindRadio current={ruleKind()} value="systemIds" label="Explicit system list" onChange={setKind} />
        </fieldset>

        <Show when={props.container.rule?.kind === "formFactor"}>
          <label class="block">
            <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Form factor value
            </span>
            <select
              value={(props.container.rule as { kind: "formFactor"; value: FormFactorTag }).value}
              onChange={(e) =>
                props.views.setContainerRule(props.container.id, {
                  kind: "formFactor",
                  value: e.currentTarget.value as FormFactorTag,
                })
              }
              class="mt-1 block w-full rounded border border-white/15 bg-(--color-oa-bg-deep) px-2 py-1 text-(--color-oa-ink) outline-none focus:border-(--color-system-accent)"
            >
              <option value="console">Console</option>
              <option value="handheld">Handheld</option>
              <option value="computer">Computer</option>
              <option value="arcade">Arcade</option>
              <option value="other">Other</option>
            </select>
          </label>
        </Show>

        <Show when={props.container.rule?.kind === "manufacturer"}>
          <label class="block">
            <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Manufacturer value
            </span>
            <select
              value={(props.container.rule as { kind: "manufacturer"; value: ManufacturerTag }).value}
              onChange={(e) =>
                props.views.setContainerRule(props.container.id, {
                  kind: "manufacturer",
                  value: e.currentTarget.value as ManufacturerTag,
                })
              }
              class="mt-1 block w-full rounded border border-white/15 bg-(--color-oa-bg-deep) px-2 py-1 text-(--color-oa-ink) outline-none focus:border-(--color-system-accent)"
            >
              <option value="nintendo">Nintendo</option>
              <option value="sega">Sega</option>
              <option value="sony">Sony</option>
              <option value="nec">NEC</option>
              <option value="atari">Atari</option>
              <option value="snk">SNK</option>
              <option value="bandai">Bandai</option>
              <option value="microsoft">Microsoft</option>
              <option value="coleco">Coleco</option>
              <option value="mattel">Mattel</option>
              <option value="magnavox">Magnavox</option>
              <option value="fairchild">Fairchild</option>
              <option value="gce">GCE</option>
              <option value="panasonic">Panasonic</option>
              <option value="other">Other</option>
            </select>
          </label>
        </Show>

        <Show when={props.container.rule?.kind === "systemIds"}>
          <SystemIdsPicker
            current={(props.container.rule as { kind: "systemIds"; values: SystemId[] }).values}
            onChange={(values) =>
              props.views.setContainerRule(props.container.id, { kind: "systemIds", values })
            }
          />
        </Show>
        </Show>
      </Show>

      <Show when={!props.isRoot}>
        <div class="space-y-1 border-t border-white/5 pt-2">
          <label class="flex cursor-pointer items-start gap-2">
            <input
              type="checkbox"
              class="mt-0.5"
              checked={props.container.filterWithinParent ?? false}
              onChange={(e) =>
                props.views.setFilterWithinParent(props.container.id, e.currentTarget.checked)
              }
            />
            <span class="flex-1">
              <span class="text-(--color-oa-ink)">Filter within parent</span>
              <span class="mt-0.5 block text-[0.65rem] leading-snug text-(--color-oa-ink-dim)">
                On: show only this node's items that are <em>also</em> in its
                parent (e.g. 2-Player inside Nintendo → 2-player Nintendo games).
                Off: this node shows its full set; its parent is just a folder
                around it.
                <Show when={props.isTopLevel}>
                  {" "}
                  <span class="text-amber-300/80">
                    No effect here — this node sits at the top level (parent is
                    "all games"). Nest it to use this.
                  </span>
                </Show>
              </span>
            </span>
          </label>
        </div>
      </Show>

      <Show when={!props.isRoot}>
        <div class="space-y-1">
          <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            Accent
          </span>
          <div class="flex items-center gap-2">
            <input
              type="color"
              value={props.container.accent ?? "#888888"}
              onInput={(e) =>
                props.views.setContainerAccent(props.container.id, e.currentTarget.value)
              }
              aria-label="Container accent color"
              class="h-7 w-10 cursor-pointer rounded border border-white/15 bg-(--color-oa-bg-deep)"
            />
            <span class="flex-1 truncate text-[0.7rem] text-(--color-oa-ink-dim)">
              {props.container.accent ?? "default"}
            </span>
            <Show when={props.container.accent}>
              <button
                type="button"
                onClick={(e) => {
                  e.currentTarget.blur();
                  props.views.setContainerAccent(props.container.id, null);
                }}
                class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
              >
                Clear
              </button>
            </Show>
          </div>
        </div>
      </Show>

      <Show when={!props.isRoot}>
        <div class="border-t border-white/5 pt-2">
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              props.onRemove();
            }}
            class="rounded border border-red-500/30 bg-red-500/10 px-2 py-1 text-[0.6rem] uppercase tracking-wider text-red-200 transition hover:bg-red-500/20"
          >
            Delete container
          </button>
        </div>
      </Show>
    </div>
  );
};

const LeafProperties: Component<{
  leaf: PlatformNode;
  onRemove: () => void;
}> = (props) => {
  const theme = () => systemThemes[props.leaf.systemId];
  return (
    <div class="space-y-3 text-xs">
      <p class="text-[0.6rem] uppercase tracking-[0.3em] text-(--color-oa-ink-dim)">
        Platform leaf
      </p>
      <div>
        <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">System</span>
        <p class="mt-1 text-(--color-oa-ink)">
          {theme()?.displayName ?? props.leaf.systemId}
        </p>
        <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          id: {props.leaf.systemId}
        </p>
      </div>
      <div class="border-t border-white/5 pt-2">
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            props.onRemove();
          }}
          class="rounded border border-red-500/30 bg-red-500/10 px-2 py-1 text-[0.6rem] uppercase tracking-wider text-red-200 transition hover:bg-red-500/20"
        >
          Remove from view
        </button>
      </div>
    </div>
  );
};

const RuleKindRadio: Component<{
  current: string;
  value: "none" | ContainerRule["kind"];
  label: string;
  onChange: (v: "none" | ContainerRule["kind"]) => void;
}> = (props) => (
  <label class="flex cursor-pointer items-center gap-2 text-xs">
    <input
      type="radio"
      name="rule-kind"
      value={props.value}
      checked={props.current === props.value}
      onChange={() => props.onChange(props.value)}
    />
    <span>{props.label}</span>
  </label>
);

const SystemIdsPicker: Component<{
  current: SystemId[];
  onChange: (values: SystemId[]) => void;
}> = (props) => {
  const [filter, setFilter] = createSignal("");
  const allSystems = Object.values(systemThemes).sort((a, b) =>
    a.displayName.localeCompare(b.displayName),
  );
  const filtered = createMemo(() => {
    const f = filter().trim().toLowerCase();
    if (!f) return allSystems;
    return allSystems.filter(
      (s) => s.displayName.toLowerCase().includes(f) || s.id.toLowerCase().includes(f),
    );
  });
  const selectedSet = createMemo(() => new Set(props.current));

  function toggle(systemId: SystemId) {
    const set = new Set(props.current);
    if (set.has(systemId)) set.delete(systemId);
    else set.add(systemId);
    props.onChange([...set]);
  }

  return (
    <div class="space-y-2">
      <label class="block">
        <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          Pick systems ({props.current.length} selected)
        </span>
        <input
          type="text"
          value={filter()}
          onInput={(e) => setFilter(e.currentTarget.value)}
          placeholder="Filter…"
          class="mt-1 block w-full rounded border border-white/15 bg-(--color-oa-bg-deep) px-2 py-1 text-xs text-(--color-oa-ink) outline-none focus:border-(--color-system-accent)"
        />
      </label>
      <ul class="max-h-64 space-y-0.5 overflow-y-auto rounded border border-white/5 bg-(--color-oa-bg-deep)/40 p-1">
        <For each={filtered()}>
          {(s) => (
            <li>
              <label class="flex cursor-pointer items-center gap-2 rounded px-2 py-1 text-[0.7rem] text-(--color-oa-ink) transition hover:bg-white/[0.04]">
                <input
                  type="checkbox"
                  checked={selectedSet().has(s.id)}
                  onChange={() => toggle(s.id)}
                />
                <span class="flex-1 truncate">{s.displayName}</span>
                <span class="text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                  {s.id}
                </span>
              </label>
            </li>
          )}
        </For>
      </ul>
    </div>
  );
};

const AddLeafPicker: Component<{
  view: View;
  onPick: (systemId: SystemId) => void;
}> = (props) => {
  const [open, setOpen] = createSignal(false);
  const [filter, setFilter] = createSignal("");
  const existingSystemIds = createMemo(() => {
    const out = new Set<SystemId>();
    collectLeafSystemIds(props.view.root, out);
    return out;
  });
  const candidates = createMemo(() => {
    const taken = existingSystemIds();
    const f = filter().trim().toLowerCase();
    return Object.values(systemThemes)
      .filter((s) => !taken.has(s.id))
      .filter((s) =>
        !f || s.displayName.toLowerCase().includes(f) || s.id.toLowerCase().includes(f),
      )
      .sort((a, b) => a.displayName.localeCompare(b.displayName));
  });

  return (
    <div class="relative">
      <button
        type="button"
        onClick={(e) => {
          e.currentTarget.blur();
          setOpen(!open());
        }}
        class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-(--color-system-accent)/15 hover:text-(--color-oa-ink)"
      >
        + Leaf {open() ? "▴" : "▾"}
      </button>
      <Show when={open()}>
        <div class="absolute left-0 z-30 mt-1 w-72 rounded border border-white/10 bg-(--color-oa-bg-deep)/95 p-2 shadow-2xl backdrop-blur">
          <input
            type="text"
            value={filter()}
            onInput={(e) => setFilter(e.currentTarget.value)}
            placeholder="Filter systems…"
            autofocus
            class="block w-full rounded border border-white/15 bg-(--color-oa-bg-deep) px-2 py-1 text-xs text-(--color-oa-ink) outline-none focus:border-(--color-system-accent)"
          />
          <ul class="mt-2 max-h-72 space-y-0.5 overflow-y-auto">
            <Show
              when={candidates().length > 0}
              fallback={
                <li class="px-2 py-1 text-[0.7rem] text-(--color-oa-ink-dim)">
                  No systems available — all are already in this view.
                </li>
              }
            >
              <For each={candidates()}>
                {(s) => (
                  <li>
                    <button
                      type="button"
                      onClick={(e) => {
                        e.currentTarget.blur();
                        props.onPick(s.id);
                        setOpen(false);
                        setFilter("");
                      }}
                      class="flex w-full items-center gap-2 rounded px-2 py-1 text-left text-[0.7rem] text-(--color-oa-ink) transition hover:bg-white/[0.05]"
                    >
                      <span class="flex-1 truncate">{s.displayName}</span>
                      <span class="text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                        {s.id}
                      </span>
                    </button>
                  </li>
                )}
              </For>
            </Show>
          </ul>
        </div>
      </Show>
    </div>
  );
};

/// Properties editor for a game-set node — identifies the rule kind and
/// lets the operator re-pick the target (which built-in smart list, or which
/// collection). Unified Navigation Tree Slice 1: deliberately separate from
/// the system-rule radio so the two authoring models don't blur.
const GameSetRuleEditor: Component<{
  container: ContainerNode;
  views: ViewsStore;
  customCollections: CustomCollectionsStore;
}> = (props) => {
  const rule = () => props.container.rule;
  const currentSmartList = (): SmartListKind | "" => {
    const r = rule();
    if (r?.kind !== "filter") return "";
    return asSmartListKind(r.spec) ?? "";
  };
  const currentCollectionId = (): string => {
    const r = rule();
    return r?.kind === "collection" ? r.collectionId : "";
  };
  const collections = () => props.customCollections.state.collections;
  return (
    <div class="space-y-2">
      <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
        {rule()?.kind === "collection" ? "Collection" : "Smart list"} — a set of
        games, not systems
      </p>
      <Show when={rule()?.kind === "filter"}>
        <label class="block">
          <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            Built-in list
          </span>
          <select
            value={currentSmartList()}
            onChange={(e) =>
              props.views.setContainerRule(props.container.id, {
                kind: "filter",
                spec: { kind: e.currentTarget.value },
              })
            }
            class="mt-1 block w-full rounded border border-white/15 bg-(--color-oa-bg-deep) px-2 py-1 text-(--color-oa-ink) outline-none focus:border-(--color-system-accent)"
          >
            <For each={SMART_LISTS}>
              {(l) => <option value={l.kind}>{l.glyph} {l.label}</option>}
            </For>
          </select>
        </label>
      </Show>
      <Show when={rule()?.kind === "collection"}>
        <Show
          when={collections().length > 0}
          fallback={
            <p class="text-[0.7rem] text-(--color-oa-ink-dim)">
              No collections yet. Create one in the Collections section, then
              re-pick it here.
            </p>
          }
        >
          <label class="block">
            <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Collection
            </span>
            <select
              value={currentCollectionId()}
              onChange={(e) =>
                props.views.setContainerRule(props.container.id, {
                  kind: "collection",
                  collectionId: e.currentTarget.value,
                })
              }
              class="mt-1 block w-full rounded border border-white/15 bg-(--color-oa-bg-deep) px-2 py-1 text-(--color-oa-ink) outline-none focus:border-(--color-system-accent)"
            >
              <For each={collections()}>
                {(c) => <option value={c.id}>{c.name}</option>}
              </For>
            </select>
          </label>
        </Show>
      </Show>
    </div>
  );
};

/// Toolbar dropdown to add a smart-list filter node — one of the six
/// built-ins. Mirrors AddLeafPicker's open/filter/pick shape.
const AddSmartListPicker: Component<{
  onPick: (kind: SmartListKind) => void;
}> = (props) => {
  const [open, setOpen] = createSignal(false);
  return (
    <div class="relative">
      <button
        type="button"
        onClick={(e) => {
          e.currentTarget.blur();
          setOpen(!open());
        }}
        class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-(--color-system-accent)/15 hover:text-(--color-oa-ink)"
      >
        + Smart List {open() ? "▴" : "▾"}
      </button>
      <Show when={open()}>
        <div class="absolute left-0 z-30 mt-1 w-64 rounded border border-white/10 bg-(--color-oa-bg-deep)/95 p-2 shadow-2xl backdrop-blur">
          <ul class="space-y-0.5">
            <For each={SMART_LISTS}>
              {(l) => (
                <li>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      props.onPick(l.kind);
                      setOpen(false);
                    }}
                    class="flex w-full items-center gap-2 rounded px-2 py-1 text-left text-[0.7rem] text-(--color-oa-ink) transition hover:bg-white/[0.05]"
                  >
                    <span class="text-sm leading-none">{l.glyph}</span>
                    <span class="flex-1 truncate">{l.label}</span>
                  </button>
                </li>
              )}
            </For>
          </ul>
        </div>
      </Show>
    </div>
  );
};

/// Toolbar dropdown to add a curated-collection node. Lists the operator's
/// custom collections; empty-state nudges them to create one first.
const AddCollectionPicker: Component<{
  customCollections: CustomCollectionsStore;
  onPick: (collection: CustomCollection) => void;
}> = (props) => {
  const [open, setOpen] = createSignal(false);
  const collections = () => props.customCollections.state.collections;
  return (
    <div class="relative">
      <button
        type="button"
        onClick={(e) => {
          e.currentTarget.blur();
          setOpen(!open());
        }}
        class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-(--color-system-accent)/15 hover:text-(--color-oa-ink)"
      >
        + Collection {open() ? "▴" : "▾"}
      </button>
      <Show when={open()}>
        <div class="absolute left-0 z-30 mt-1 w-64 rounded border border-white/10 bg-(--color-oa-bg-deep)/95 p-2 shadow-2xl backdrop-blur">
          <Show
            when={collections().length > 0}
            fallback={
              <p class="px-2 py-1 text-[0.7rem] text-(--color-oa-ink-dim)">
                No collections yet — create one in the Collections section
                below.
              </p>
            }
          >
            <ul class="max-h-72 space-y-0.5 overflow-y-auto">
              <For each={collections()}>
                {(c) => (
                  <li>
                    <button
                      type="button"
                      onClick={(e) => {
                        e.currentTarget.blur();
                        props.onPick(c);
                        setOpen(false);
                      }}
                      class="flex w-full items-center gap-2 rounded px-2 py-1 text-left text-[0.7rem] text-(--color-oa-ink) transition hover:bg-white/[0.05]"
                    >
                      <span class="text-sm leading-none">★</span>
                      <span class="flex-1 truncate">{c.name}</span>
                      <Show when={c.memberCount > 0}>
                        <span class="text-[0.55rem] tabular-nums uppercase tracking-widest text-(--color-oa-ink-dim)">
                          {c.memberCount}
                        </span>
                      </Show>
                    </button>
                  </li>
                )}
              </For>
            </ul>
          </Show>
        </div>
      </Show>
    </div>
  );
};

// ── Utilities ──────────────────────────────────────────────────────

function collectLeafSystemIds(node: ContainerNode | ViewNode, into: Set<SystemId>): void {
  if ("kind" in node && node.kind === "platform") {
    into.add(node.systemId);
    return;
  }
  if (!("children" in node)) return;
  for (const child of node.children) collectLeafSystemIds(child, into);
}

export default ViewEditorPane;
