# Navigation Structure — the full arc

> **Re-anchored 2026-06-19.** This arc was previously framed as a "Unified Navigation Tree / theme
> view model," which drifted into theme jargon and buried the real feature. It is re-anchored here as
> **Navigation Structure**: a **Settings-level, user-authored** feature. Themes only optionally
> *style* the views; they never own the structure. Vision + BigBox study + drift history live in the
> anti-drift artifact [features/unified-nav-tree/RESEARCH.md](../features/unified-nav-tree/RESEARCH.md)
> — read it first.

## Context

"Collections" and "sidebar views" were always meant to be **one thing**: a user-authored navigation
tree where a node can be a system, a group of systems, an auto/smart list, or a hand-curated list —
dragged in/out, nested as deep as you want, each node rendered by a view you can change and that's
remembered. Goal: **rival and outdo BigBox** (whose scheme is fairly fixed) on the
*organize-it-yourself* axis. The 2026-06-18 audit found almost everything is already built — it just
isn't *unified* or *exposed*. **This is reconnect + enrich, not rebuild.** Operator chose
(2026-06-19) to plan the whole arc up front and to unify the editor.

## Substrate already built (the plumbing the arc drives — keep it)

- **Data model + persistence:** `views.json` tree (`platform/views/types.ts` + Rust
  `apps/oa-shell/src/views.rs`). `ContainerNode` with `rule` ∈ {`formFactor`, `manufacturer`,
  `systemIds`, **`collection`**, **`filter`**} + **`filterWithinParent`** (NT4). All round-trip.
- **Membership:** `resolveNodeMembership` is ancestry-aware (NT4), resolving a node to systems OR an
  explicit game set; `filterEntries` has the `viewRomIds` seam.
- **Smart-list predicates:** `platform/library/smartLists.ts` (6 pure built-ins + `evaluateSmartList`
  + `asSmartListKind`), shared by the Collections tab and the resolver.
- **Store mutations:** `platform/views/store.ts` — `addContainer`, `setContainerRule`,
  `addPlatformLeaf`, `removeNode`, `reorderChildren`, **`moveNode` (containers too, cycle-guarded)**,
  `setContainerAccent`, `setNodeHidden`, view CRUD.
- **Editor + home:** Settings → **Organize My Collection** (`engine/OrganizeLanding.tsx`) hosts the
  **View Editor** (`engine/ViewEditorPane.tsx` + `ViewsManagerTab.tsx`), a Collections Manager, and
  show/hide systems.
- **Per-node view cascade substrate:** `platform/theme/layoutResolver.ts` + `layoutOverrides.ts`
  (localStorage, keyed `theme/system/view`) + the per-system Layout editor
  (`engine/systemsHub/domains/LayoutEditor.tsx`). `ViewType` (4 levels) + `LayoutPrimitive` (5) in
  `platform/theme/manifest.ts`; only `game-browse` has a renderer (`LibraryView`).
- **Slice 1 (merged) + Slice 2 (branch `feat/unified-nav-tree-s2`, green, not merged):** node→games,
  filter nodes wired to the predicates, NT4 ancestry-aware resolver. **Slice 2 merges as
  *substrate*;** its read-only sidebar "Collections"/"Smart Lists" sections are **transitional**,
  replaced by real authored nodes in Arc-Slice 1.

## Target end-state

**Settings → Navigation Structure** (renamed from "Organize My Collection") — **one** editor, one
tree, every node kind first-class, draggable, nestable to any depth:

- **Node kinds:** system · system-group (by maker / by type / explicit set) · **smart list** (a
  filter you build) · **curated collection** · **folder/heading** (organization, no filter).
- **Per-node options:** label · icon · accent · art/logo · the node's **view** (with the cascade) ·
  **"filter within parent"** toggle · hide.
- **Walk it:** systems screen → drill into a system → its games → details (the BigBox levels), each
  honoring per-node views, change-on-the-fly + remembered.
- **Share it:** export/import a structure; ship starter structures.

## The arc — slices (each playtestable; contracts-first; reuse the substrate)

1. **Collections & smart-lists as real, authored tree nodes.** Wire the reserved `collection`/`filter`
   rules into the editor ("+ Smart List", "+ Collection" → `addContainer`+`setContainerRule`) and the
   sidebar (render as normal draggable rows); retire the synthesized read-only sections. *Playtest:*
   add Favorites + a collection into your tree, drag them, they persist + fill the grid.
2. **Deep nesting + "filter within parent."** Generalize the `solid-dnd` drag to nested containers at
   any depth (today leaf-only; `moveNode` already supports it) + a `filterWithinParent` checkbox.
   *Playtest:* nest "2-Player" inside "Nintendo", tick → 2-player Nintendo games.
3. **Smart-list builder + node-kind richness.** A builder to compose predicates (genre/players/year/
   region/favorite/dump-quality…), growing the predicate AST over the opaque `filter.spec` (NT5, no
   schema bump); "new list from current filter/selection"; folders/headings + per-node icon/label/art.
   *Playtest:* build "PS1 RPGs before 1998" from scratch.
4. **Unify the editor + rename to "Navigation Structure"; retire the Collections tab.** Fold the three
   `OrganizeLanding` cards into one tree editor; rename the `SettingsPanel` category; deprecate
   `themes/retroverse/CollectionsPage.tsx`. *Playtest:* one editor, no orphan tab.
5. **Per-node views (change on the fly, remembered).** Generalize the override key `system → node`
   (additive) in `layoutOverrides`/`layoutResolver`; node-aware Layout editor; live "change view"
   control in `LibraryView`. Honor the styling-vs-freedom tradeoff. *Playtest:* Favorites→CoverFlow,
   NES→Wall, remembered.
6. **Drill-through levels (BigBox hierarchy).** Render `system-browse` (wheel/list of systems) +
   `manufacturer-browse` + real `game-details`, with drill-in nav, reusing the `LayoutPrimitive`
   nav primitives. Largest / most independent. *Playtest:* systems screen → NES → games → details.
7. **Sharing, starter structures, file-theme payoff, polish.** Export/import; starter structures;
   per-node art rendering; the declarative shell walks the *same* tree + per-node views (closes the
   original per-system-view gap for file themes — the parked Declarative Showcase S3 resumes here).

## Key reused infrastructure (don't rebuild)

| Need | Reuse |
|---|---|
| Tree mutations (add/move/nest/rule/hide) | `platform/views/store.ts` (`moveNode` already nests) |
| Membership (systems / games / NT4 compose) | `platform/views/resolver.ts` `resolveNodeMembership` |
| Smart-list eval | `platform/library/smartLists.ts` |
| Editor shell + node properties | `engine/ViewEditorPane.tsx`, `ViewsManagerTab.tsx` |
| Settings home + category routing | `engine/OrganizeLanding.tsx`, `engine/SettingsPanel.tsx` |
| Per-node view cascade + store | `platform/theme/layoutResolver.ts`, `layoutOverrides.ts`, `systemsHub/domains/LayoutEditor.tsx` |
| Sidebar tree render + drag | `platform/components/LeftSidebar.tsx`, `SidebarTreeNode.tsx` |
| Collections CRUD | `platform/library/customCollections.ts`, Rust `library_db.rs` |
| View levels + primitives | `platform/theme/manifest.ts` (`ViewType`/`LayoutPrimitive`), `platform/nav` |

## Verification

- **Per slice (frontend):** `cd frontend && ./node_modules/.bin/tsc --noEmit && npm run lint &&
  ./node_modules/.bin/vitest run src/platform/views src/platform/library` (+ `src/platform/theme` for
  Slice 5). **Rust (when `views.rs` changes):** `cargo test -p oa-shell views` (no whole-crate fmt).
- **End-to-end (operator, `cargo tauri build`):** each slice's playtest line. Watch the stale
  `<exe_dir>/themes/` shadow landmine.

## Relationship to other work

- **Supersedes** the old "theme view model" framing of this arc (which [VIEW_MODEL.md](../features/unified-nav-tree/VIEW_MODEL.md)
  documents — kept as the *styling-cascade* reference, now subordinate to RESEARCH.md).
- **Distinct from** `features/unified-nav/` (spatial/input nav). This is the navigation *content model*.
- **Declarative Showcase S3** resumes inside Slice 7.
