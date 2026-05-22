// ViewsStore — Solid composable that hydrates views.json from Rust on
// mount, seeds defaults / migrates from legacy `layout.systemOrder` when
// the file is absent, and writes back via the Tauri `set_views` command
// on every mutation. Mirrors LayoutStore's shape (signals + setters +
// a hydration gate). Per SIDEBAR_TIER_PLAN.md §2.3 + §2.6.

import { batch, createEffect, createMemo, createSignal, onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/core";

import { buildDefaultFormFactorView, DEFAULT_VIEW_ID, LEGACY_VIEW_ID } from "./defaults";
import { buildLegacyFlatView } from "./migration";
import { CURRENT_SCHEMA_VERSION, type View, type ViewNode, type ViewsConfig } from "./types";

type LayoutLite = { systemOrder?: string[] };

export function createViewsStore() {
  const [config, setConfig] = createSignal<ViewsConfig>({
    schemaVersion: CURRENT_SCHEMA_VERSION,
    activeViewId: DEFAULT_VIEW_ID,
    views: [buildDefaultFormFactorView()],
    bannerDismissed: false,
  });
  const [hydrated, setHydrated] = createSignal(false);

  const activeView = createMemo<View | null>(() => {
    const c = config();
    return c.views.find((v) => v.id === c.activeViewId) ?? c.views[0] ?? null;
  });

  onMount(async () => {
    let next: ViewsConfig | null = null;
    try {
      next = await invoke<ViewsConfig | null>("get_views");
    } catch (e) {
      console.warn("[oa-views] get_views failed:", e);
    }
    if (next) {
      // Existing file — trust it (Rust-side migrate_inplace has already
      // bumped schemaVersion if needed).
      setConfig(next);
    } else {
      // No file yet — decide migration path based on legacy systemOrder.
      let legacyOrder: string[] = [];
      try {
        const layout = await invoke<LayoutLite>("get_layout");
        if (Array.isArray(layout.systemOrder)) legacyOrder = layout.systemOrder;
      } catch (e) {
        console.warn("[oa-views] get_layout failed during migration:", e);
      }
      const defaultView = buildDefaultFormFactorView();
      let seeded: ViewsConfig;
      if (legacyOrder.length > 0) {
        const legacyView = buildLegacyFlatView(legacyOrder);
        seeded = {
          schemaVersion: CURRENT_SCHEMA_VERSION,
          activeViewId: LEGACY_VIEW_ID,
          views: [defaultView, legacyView],
          bannerDismissed: false,
        };
      } else {
        seeded = {
          schemaVersion: CURRENT_SCHEMA_VERSION,
          activeViewId: DEFAULT_VIEW_ID,
          views: [defaultView],
          bannerDismissed: false,
        };
      }
      setConfig(seeded);
      try {
        await invoke("set_views", { config: seeded });
      } catch (e) {
        console.warn("[oa-views] initial set_views failed:", e);
      }
    }
    setHydrated(true);
  });

  // Write-through to disk on every mutation after hydration. Gated on
  // hydrated() so the initial default-state setConfig doesn't echo
  // back before the disk read completes.
  createEffect(() => {
    if (!hydrated()) return;
    const snapshot = config();
    invoke("set_views", { config: snapshot }).catch((e) =>
      console.warn("[oa-views] set_views failed:", e),
    );
  });

  // ── Mutations ────────────────────────────────────────────────────

  function setActiveView(viewId: string): void {
    setConfig((prev) => ({ ...prev, activeViewId: viewId }));
  }

  function setBannerDismissed(dismissed: boolean): void {
    setConfig((prev) => ({ ...prev, bannerDismissed: dismissed }));
  }

  function toggleExpanded(nodeId: string): void {
    setConfig((prev) => mapActiveView(prev, (view) => {
      const expanded = new Set(view.expandedNodes);
      if (expanded.has(nodeId)) expanded.delete(nodeId);
      else expanded.add(nodeId);
      return { ...view, expandedNodes: [...expanded] };
    }));
  }

  function setNodeHidden(nodeId: string, hidden: boolean): void {
    setConfig((prev) => mapActiveView(prev, (view) => ({
      ...view,
      root: mapNode(view.root, nodeId, (n) => ({ ...n, hidden })),
    })));
  }

  function reorderChildren(parentId: string, newOrder: string[]): void {
    setConfig((prev) => mapActiveView(prev, (view) => ({
      ...view,
      root: mapNode(view.root, parentId, (n) => {
        if (!("children" in n)) return n;
        const byId = new Map<string, ViewNode>();
        for (const c of n.children) byId.set(c.id, c);
        const reordered: ViewNode[] = [];
        const seen = new Set<string>();
        for (const id of newOrder) {
          const child = byId.get(id);
          if (child && !seen.has(id)) {
            seen.add(id);
            reordered.push(child);
          }
        }
        // Append any children not in newOrder (defensive — UI should
        // pass the full set, but a stale newOrder shouldn't lose nodes).
        for (const c of n.children) {
          if (!seen.has(c.id)) reordered.push(c);
        }
        return { ...n, children: reordered };
      }),
    })));
  }

  /// Convenience for PR-γ's top-level drag-reorder — equivalent to
  /// reorderChildren("root", newOrder) but spelled for readability at
  /// the call site.
  function reorderTopLevel(newOrder: string[]): void {
    reorderChildren("root", newOrder);
  }

  /// Replace the entire active view's tree. Used when applying
  /// `reorderForFormFactor` after the operator picks "Try Form Factor
  /// view" — swaps the seeded default for the reordered version.
  function replaceView(viewId: string, replacement: View): void {
    setConfig((prev) => ({
      ...prev,
      views: prev.views.map((v) => (v.id === viewId ? replacement : v)),
    }));
  }

  /// Batched migration commit — apply Option C reorder + switch active
  /// view + dismiss banner in one update so write-through fires once.
  function commitTryFormFactor(reorderedDefault: View): void {
    batch(() => {
      replaceView(DEFAULT_VIEW_ID, reorderedDefault);
      setActiveView(DEFAULT_VIEW_ID);
      setBannerDismissed(true);
    });
  }

  return {
    config,
    activeView,
    hydrated,
    setActiveView,
    setBannerDismissed,
    toggleExpanded,
    setNodeHidden,
    reorderChildren,
    reorderTopLevel,
    replaceView,
    commitTryFormFactor,
  };
}

export type ViewsStore = ReturnType<typeof createViewsStore>;

// ── Internal tree-mutation helpers ─────────────────────────────────

function mapActiveView(config: ViewsConfig, fn: (view: View) => View): ViewsConfig {
  return {
    ...config,
    views: config.views.map((v) => (v.id === config.activeViewId ? fn(v) : v)),
  };
}

type NodeLike = ViewNode | View["root"];

/// Recursive tree map — replaces the matching node via `transform` and
/// rebuilds the path back to the root. Non-matching subtrees are
/// preserved by reference (Solid's reactivity sees the new top-level
/// object and re-runs the write-through effect; downstream consumers
/// that memoize per-node identity still skip work for unchanged
/// subtrees).
function mapNode<T extends NodeLike>(node: T, targetId: string, transform: (n: T) => T): T {
  if (node.id === targetId) return transform(node);
  if (!("children" in node)) return node;
  return {
    ...node,
    children: node.children.map((c) => mapNode(c, targetId, transform as never)),
  } as T;
}
