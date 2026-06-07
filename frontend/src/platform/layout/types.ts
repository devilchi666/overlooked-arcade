// Platform — layout types.
//
// Hosts the shared layout shapes that platform stores + theme code +
// filter helpers all need to agree on. Extracted from
// `layout/LeftSidebar.tsx` in Theming Substrate ARC 1 Phase 2 Slice B
// because the LeftSidebar component itself stays in `src/layout/` for
// now (Slice C moves the shared components into `platform/components/`)
// but the navigation-state type it exports is platform-territory.

/// Operator-visible navigation filter. `all` shows every entry in
/// the library; `view-node` is a pointer into the active view's tree
/// (`viewId` + `nodeId`), not a bare SystemId. The runtime resolves
/// filterable SystemIds via `resolveNodeSystemIds` (views/resolver.ts).
export type SidebarView =
  | { kind: "all" }
  | { kind: "view-node"; viewId: string; nodeId: string };
