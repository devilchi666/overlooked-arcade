# Unified Navigation Tree — systems + collections + filters as one node tree, each rendered by a per-node view

**Planned 2026-06-18. Operator-approved (commit as next arc, incremental). Branch:
`feat/unified-nav-tree` (suggested). Feature folder:
[docs/features/unified-nav-tree/](../features/unified-nav-tree/).**

> The authoritative theme-view model lives in
> [features/unified-nav-tree/VIEW_MODEL.md](../features/unified-nav-tree/VIEW_MODEL.md) — read it
> before touching anything view/layout related. It is the anti-drift artifact for this arc.

## Context

This started as "Declarative Showcase S3" (list-row polish for file themes) and uncovered a deeper,
foundational issue the operator had sensed but couldn't name through the jargon. Two findings
converged:

1. **File themes can't do per-system views.** The one built-in `DeclarativeShell` renders a *flat,
   all-systems browse* with a single layout — it passes `() => null` for the system
   (`declarativeShell.tsx:117`), so the per-system layout machinery (which exists) has nothing to
   attach to.
2. **Collections drifted from the original vision.** The intent was that *collections ARE the
   navigation layer* — a user-authored tree where each node is a **system**, a **group of systems**,
   or a **filtered set of games** (Favorites / 2-Player / …), nested arbitrarily, drag-arranged, each
   node rendered by a **selectable view** — i.e. rival BigBox's per-level-per-node view model. The
   audit (2026-06-18) showed this was **never actually built that way**: navigation shipped as a
   systems-only `views` tree, and collections shipped as a *separate flat tab*. No decision ever
   recorded the unified intent; the two halves were simply never joined.

**The key realization:** these are the *same* foundation. BigBox's model — and the operator's — is
"navigation is a tree of nodes; each node is rendered by a view you can change on the fly and that's
remembered." Once that exists: per-system views fall out for free, collections become first-class
navigation, and the declarative file-theme shell just renders "the current node's view" — closing
the file-theme gap **and** the collections gap **and** BigBox parity at once.

**The good news:** almost every piece already exists. The work is a *join*, not a rebuild.

## What already exists (the foundation is strong)

| Piece the vision needs | Status today | Where |
|---|---|---|
| User-authored nestable tree (drag / reorder / group / hide / expand) | ✅ shipped | `platform/views/` (`types.ts`, `store.ts`, resolver, View Editor); `platform/components/LeftSidebar.tsx` + `SidebarTreeNode.tsx` |
| "Node carries a rule; runtime filters library by the rule" | ✅ core philosophy | `_archive/features/sidebar/SIDEBAR_TIER_PLAN.md` §0 ("view-as-projection, not folder-containment") |
| Extensible rule DSL | ✅ `formFactor` / `manufacturer` / `systemIds` | `ContainerRule` in `platform/views/types.ts:81` |
| Collections storage | ✅ (flat, games-only today) | `custom_collections` + `custom_collection_members` (`library_db.rs:1916`); `customCollections.ts` (`members: Map<id, Set<romId>>`) |
| Filtered game-sets (Favorites / players / dump-quality) | ✅ predicates + 8-axis filter | smart-list predicates (`CollectionsPage.tsx`); `VariantFilters` (`library_groups.rs:147`); `filter.ts` pipeline |
| Per-node view cascade + override store | ✅ exists (keyed by system today) | `platform/theme/layoutResolver.ts` + the `layoutOverrides` store |
| Pure filter seam for non-system membership | ✅ **already reserved** | `filterEntries(... viewSystemIds, ...)` doc comment (`filter.ts:24-43`) |

## The one missing piece (the keystone)

Every node today resolves to **`SystemId[]`** via `resolveNodeSystemIds(view, nodeId)`
(`platform/views/resolver.ts:55`), and `LibraryView` filters games by system. **Collections and
filters resolve to *games directly*, not via systems.** So the single architectural change the whole
arc pivots on: **generalize node resolution + the library filter from "systems-only" to "systems OR
an explicit game set OR a predicate."** Everything else is additive on top.

## The arc — slices (each playtestable; D59-style: reserve the contract, wire incrementally)

- **Slice 1 — Keystone: node→games resolution + collections navigable in the sidebar.** *(detail below)*
- **Slice 2 — Filter / smart-list nodes + membership composition. ✅ SHIPPED 2026-06-19**
  (`feat/unified-nav-tree-s2`, awaiting playtest + merge; dump-quality deferred per **NT6** — needs a
  node-meaning + variant-vs-default call). A `filter` rule kind backed by
  the existing smart-list predicates + `VariantFilters` (Favorites / Recently Played / Multi-Player /
  dump-quality as tree nodes). Reserve the rule shape in Slice 1 (done), wire it here. **Also lands
  the NT4 composition model:** reserve a per-node `filterWithinParent?: boolean` on `ContainerNode`
  (TS + Rust mirror, default off) and make resolution **ancestry-aware** — `resolveNodeMembership`
  grows to fold parent ∩ child intersections up the path from root (stopping at the nearest off
  ancestor). Off = independent "standard" membership (today's behaviour); on = narrow within parent.
  This is the load-bearing change flagged in NT4: a node can no longer be resolved by id in
  isolation. Cross-axis (game-set ∩ system-group) intersection falls out once membership composes.
- **Slice 3 — Per-node view cascade (the BigBox behavior).** Implements the full
  [VIEW_MODEL.md](../features/unified-nav-tree/VIEW_MODEL.md): the cascade per node, the
  `layoutOverrides` key generalized `(themeId, systemId, view)` → `(themeId, nodeId, view)`, and the
  live in-context "change view" control (generalizing the shipped L5 system-keyed Layout editor).
  Honors the operator-locked styling-vs-freedom tradeoff.
- **Slice 4 — Curation into the tree; retire the Collections tab.** Drag games onto a node, "new
  collection from selection," reorder/nest collections in the tree. Fold the `Collections` tab's
  editing affordances into the one nav tree; deprecate the separate route.
- **Slice 5 — Declarative shell renders the unified tree (the file-theme payoff).** `DeclarativeShell`
  walks the same `views` tree + per-node views instead of a flat browse. **Closes the original
  per-system-view gap; gives BigBox parity to file themes.** The parked **Declarative Showcase S3**
  (row thumbnails / metadata / recognized-settings vocab) resumes here as render detail.
- **Slice 6+ — Depth & polish.** Manufacturer-/system-browse view *levels* (the other declared
  `ViewType`s), per-node art slots, collection import/export, deeper nesting affordances.

## Slice 1 (keystone) — concrete, executable

Goal after this slice: **click a collection (e.g. "Favorites") in the Retroverse sidebar and see its
games in the center grid, in the active view** — proving node→games end-to-end. Frontend-heavy + a
small Rust types mirror for persistence round-trip.

1. **Types — add the `collection` rule kind.** `platform/views/types.ts:81` — extend `ContainerRule`:
   `| { kind: "collection"; collectionId: string }`. A "collection node" is just a `ContainerNode`
   whose rule is `collection` (reuses nesting, accent, art, hide for free). Reserve
   `{ kind: "filter"; spec: … }` as a declared-but-inert shape now (Slice 2 wires it) per D59. Mirror
   the variant in the Rust `ContainerRule` enum in `apps/oa-shell/src/views.rs` (internally-tagged
   `#[serde(tag = "kind")]`, camelCase) so the tree round-trips through `get_views`/`set_views`. Add
   a `views.rs` round-trip unit test.
2. **Resolver — add a membership path.** `platform/views/resolver.ts` — add `NodeMembership =
   { kind: "systems"; systemIds } | { kind: "games"; romIds }` and
   `resolveNodeMembership(view, nodeId, collectionMembers)`. System / group / root nodes →
   `{ systems }` (delegates to today's `resolveNodeSystemIds`, which stays). A `collection`-rule
   container → `{ games: collectionMembers.get(collectionId) ?? empty }`. `collectionMembers` is the
   `Map<id, Set<romId>>` the `customCollections` store already holds — no new backend call. Keep
   `resolveNodeSystemIds` intact for existing callers (counts, deep-link).
3. **Library filter — accept an explicit game set.** `platform/library/filter.ts:33` — widen
   `filterEntries` to also take `viewRomIds: ReadonlySet<string> | null`; when non-null, slice by
   `romIds.has(e.id)` instead of by system. Uses the seam the doc comment already reserved; existing
   system path unchanged.
4. **LibraryView — consume membership.** `themes/retroverse/LibraryView.tsx` (`viewSystemIds` memo
   ~102-113 → `filterEntries` call ~149-200): resolve `resolveNodeMembership(...)` and pass either
   `viewSystemIds` or `viewRomIds`. Collapse/group/sort pipeline downstream unchanged.
5. **Sidebar — surface collections as nodes (cheapest proof).** Inject a read-only **"Collections"**
   container into the active view's render, children = one `collection`-rule node per row from the
   `customCollections` store. Clickable/navigable now without new editing UI (drag-into-tree +
   placement persistence is Slice 4). Clicking routes through the existing
   `onNavigate({ kind: "view-node", … })` path; the new membership resolution does the rest.

**Out of Slice 1 (deferred; contracts reserved now):** filter/smart nodes (S2), per-node view
selection/override (S3), drag-to-curate + tab retirement (S4), declarative-shell rendering (S5),
arbitrary nesting/reordering of collections in the tree (S4/S6).

## Sequencing & relationship to other work

- **Supersedes/reframes** the queued *Declarative Showcase S3*; that polish resumes inside Slice 5.
- **Distinct from** the *Unified Navigation & Panel System* feature (`features/unified-nav/` — that's
  spatial/input nav; this is the navigation *content model*). Named separately to avoid collision.
- **Coordinate with, don't block on,** *Virtual Library Phase B* (Casual/Preservation modes +
  Collection Health) — orthogonal render-mode toggle; its `VariantFilters` are reused by Slice 2.
- **Branch:** one feature branch for the arc; merge to main at playtestable milestones (Slice 1 is one).

## Open design forks to settle in later slices (NOT Slice 1)

- ✅ **SETTLED 2026-06-18 (NT4) — node membership composition.** Both folder-organization AND
  narrowing, via a per-node "filter within parent" toggle (default off). Resolution becomes
  ancestry-aware; lands in Slice 2. See DECISIONS NT4.
- View-memory granularity & "change view on the fly" UI (S3) — operator's rule is known
  (default-for-all + per-node override, per-game later); the UI surface + override-key shape is the fork.
- Do collections auto-appear in every view, or become operator-arranged nodes? (S1 = auto section;
  S4 = drag-arranged + persisted placement.)
- Smart-list/filter persistence: a predicate AST per `_archive/PLANS/collections-tab-retroverse.md`
  §12, evaluated at render (S2).
- Collection membership resolution home: frontend store (S1, cheap) vs. a backend query for large
  libraries (revisit if perf needs it).

## Verification

- **Per step (frontend):** `cd frontend && ./node_modules/.bin/tsc --noEmit && npm run lint &&
  ./node_modules/.bin/vitest run src/platform/views src/platform/library` (+ `src/platform/theme`
  once S3/S5 touch the resolver).
- **Rust (Slice 1 types mirror):** `cargo test -p oa-shell views`. Do **not** `cargo fmt -p oa-shell`
  (whole-crate churn) — match style by hand / fmt touched files.
- **End-to-end (operator, `cargo tauri build`):** Slice 1 — a "Collections" section appears in the
  Retroverse sidebar; clicking a collection shows exactly its member games in the center grid;
  existing system/group nodes behave unchanged. Watch the stale-`<exe_dir>/themes/` shadow landmine.
