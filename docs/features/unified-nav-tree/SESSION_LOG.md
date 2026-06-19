# Unified Navigation Tree — Session Log

## 2026-06-18 — Arc planned + documented (no code yet)

**Shipped:** The arc was discovered and scoped. A session that began as "Declarative Showcase S3"
surfaced two converging gaps (file themes can't do per-system views; collections never joined the
nav tree). Three parallel audits mapped: the declarative shell + theme contract, the Retroverse
code-theme structure, the locked theming decisions, the collections data model, the navigation
sidebar, and the grouping/filter primitives + documented intent. Plus BigBox research (per-level
per-node views, change-on-the-fly, remembered). Findings: the `views` sidebar is already a strong
user-authored tree; collections are a separate flat tab; the one missing keystone is node→games
resolution. Operator approved committing this as the next arc, built incrementally. Docs stood up:
[PLANS/unified-nav-tree.md](../../PLANS/unified-nav-tree.md),
[README.md](README.md), the authoritative [VIEW_MODEL.md](VIEW_MODEL.md) (the anti-drift artifact
the operator asked for), [DECISIONS.md](DECISIONS.md) (NT1–NT3). Queued Slice 1 in NEXT.md HIGH band;
added to ACTIVE_WORK "In flight"; INDEX updated.

**Almost:** n/a (planning + docs only).

**Next:** **Slice 1 (keystone)** — add the `collection` `ContainerRule` kind (TS + Rust mirror +
round-trip test); add `resolveNodeMembership` (systems | games) to `platform/views/resolver.ts`;
widen `filterEntries` with `viewRomIds`; consume membership in Retroverse `LibraryView`; inject a
read-only "Collections" section into the sidebar so a collection is clickable and shows its games.
Frontend-heavy + small Rust types mirror. Verify: tsc + lint + vitest (`platform/views`,
`platform/library`) + `cargo test -p oa-shell views`. See the plan's "Slice 1" section.

## 2026-06-18/19 — Slice 1 (keystone) — ✅ SHIPPED + MERGED to main (`1c9a493`)

**Merged 2026-06-19** — operator chose to merge (branch `feat/unified-nav-tree` pushed for backup).

**Shipped (all 5 steps; tsc + lint + 261 vitest + 869 cargo green):**

1. **`collection` rule kind + reserved inert `filter` (D59).** `ContainerRule` in
   `platform/views/types.ts` gained `{ kind: "collection"; collectionId }` and the
   declared-but-inert `{ kind: "filter"; spec: FilterRuleSpec }` (Slice 2 wires the predicate).
   Mirrored in the Rust `views.rs` enum (`Collection { collection_id }` with explicit
   `#[serde(rename = "collectionId")]`; `Filter { spec: serde_json::Value }`). New
   `views_collection_and_filter_rules_round_trip` test asserts the camelCase JSON shape + a
   write→read disk round-trip for both kinds.
2. **`resolveNodeMembership` (systems | games).** New sibling in `platform/views/resolver.ts`
   returning `NodeMembership = {systems} | {games}`. Collection-rule containers + synthesized
   `collection:<id>` ids → `{ games: members.get(id) ?? ∅ }`; filter rules → `{ games: ∅ }` (inert);
   everything else delegates to the **untouched** `resolveNodeSystemIds`. Added
   `collectionNodeIdFor`/`parseCollectionNodeId` to `views/defaults.ts` (mirrors the
   `platform:<systemId>` synth-leaf pattern). 8 new resolver unit tests.
3. **`filterEntries` `viewRomIds` seam.** `platform/library/filter.ts` now takes
   `viewRomIds: ReadonlySet<string> | null`; when non-null it slices by `romIds.has(e.id)` and takes
   precedence over the system path (the seam the doc comment already reserved). 6 new filter tests.
4. **Membership consumed in `LibraryView`** (the file lives at `platform/components/LibraryView.tsx`,
   not `themes/retroverse/` as the plan text said — it's a platform component the theme renders). A
   `membership` memo drives derived `viewSystemIds`/`viewRomIds`; collection nodes show their name in
   the title via a theme-supplied `collectionName` lookup. Wired in `themes/retroverse/LibraryPage.tsx`:
   a `collectionMembers` map memo + `ensureMembers` effect (lazy member load on activation) + a
   collection-aware header title/count.
5. **Read-only "Collections" sidebar section** (`platform/components/LeftSidebar.tsx`, below the
   Platforms tree, expanded-mode only, hidden when empty). Each row navigates to
   `collection:<id>`; Retroverse controller-nav picks the rows up for free via the page-level
   `[data-oa-sidebar-row]` DOM-query group.

**Almost:** nothing partial — all 5 steps complete and green. Note the file-path drift in the plan
(step 4/5 say `themes/retroverse/LibraryView.tsx`; the real homes are `platform/components/`
LibraryView + LeftSidebar, consumed by `themes/retroverse/LibraryPage.tsx`).

**Next:** **Slice 2** — filter / smart-list nodes + the NT4 composition model. (a) Wire the reserved
`filter` rule to the existing smart-list predicates + `VariantFilters` (Favorites / Recently Played /
Multi-Player / dump-quality as tree nodes; predicate AST per
`_archive/PLANS/collections-tab-retroverse.md` §12, evaluated at render). (b) **NT4** — reserve a
per-node `filterWithinParent?: boolean` on `ContainerNode` (TS + Rust mirror, default off) and make
`resolveNodeMembership` **ancestry-aware** (fold parent ∩ child up the path from root, stopping at the
nearest off ancestor). The load-bearing change: a node can no longer be resolved by id in isolation.

## 2026-06-18 — NT4 decided (node membership composition)

**Shipped:** Recorded decision **NT4** + folded it into the plan (Slice 2) after the operator asked
whether a node can contain other nodes "and go even deeper." Outcome: **both** folder-organization
and narrowing, chosen per node by a "filter within parent" toggle (default off). Off = independent
"standard" set; on = intersect with parent; consecutive on-nodes chain up to the nearest off
ancestor. Cross-axis (game-set ∩ system-group) intersection falls out for free. The load-bearing
consequence — resolution becomes ancestry-aware — is captured so it's designed in, not retrofitted.
Kept on the **membership** axis, deliberately separate from VIEW_MODEL's **layout** axis (conflation
is what drifted before).

**Almost:** n/a (decision + docs).

**Next:** as above — Slice 2 builds NT4 + the filter nodes together.

## 2026-06-19 — Slice 2 (filter nodes + NT4 composition) — ✅ SHIPPED on `feat/unified-nav-tree-s2` (awaiting playtest + merge)

Forks discussed in prose + operator approved ("I don't object") before code. tsc + lint + **31 vitest**
(`platform/views` + `platform/library`) + **10 cargo `views`** green.

**Shipped:**

1. **Shared smart-list registry** — `platform/library/smartLists.ts` is now the single source for the
   six built-in predicates (Favorites / Recently Played / Completed / Multi-Player / Hidden Gems / Last
   Played), lifted out of the COLLECTIONS-tab page so **both** consumers share one definition. Predicates
   are **pure** (`nowSecs` from an eval ctx, not ambient `Date.now()`). Adds `asSmartListKind` (narrow an
   opaque spec) + `evaluateSmartList`. `CollectionsPage.tsx` refactored to consume it (camelCase kinds;
   its presentation-only sorting stays local). New `smartLists.test.ts` (8).
2. **`filter`-rule nodes wired (was inert).** `resolveNodeMembership` now evaluates a `filter` rule's
   `spec` via the registry → `{ games }`. Synthesized `filter:<kind>` ids added
   (`filterNodeIdFor`/`parseFilterNodeId` in `views/defaults.ts`, twin of the collection ids).
3. **NT4 ancestry-aware resolver (the load-bearing change).** New `findNodePath` (root→node chain);
   `resolveNodeMembership` folds parent ∩ child down the path honoring `filterWithinParent` (off = own
   membership / today's behaviour; on = intersect with parent; stops at nearest off ancestor).
   **Cross-axis** `{games} ∩ {systems}` collapses to games-in-an-allowed-system via the entries map.
   The 3rd arg changed from a bare `collectionMembers` map to a single **`MembershipContext`**
   `{ collectionMembers?, entries?, nowSecs? }` (optional, graceful-degrade). resolver tests grew to 17
   (NT4 narrow / off / cross-axis + filter-node + `findNodePath`).
4. **`filterWithinParent` reserved (D59).** Optional on TS `ContainerNode`; `#[serde(default)]
   filter_within_parent: bool` on Rust `ContainerNode` (+ round-trip/serde-default test). No UI sets it
   yet — the resolver honors it; the toggle + nesting are Slice 4.
5. **Read-only "Smart Lists" sidebar section** (`LeftSidebar` `filterNodes` prop, twin of `collections`;
   counts computed in `LibraryPage` where the entries live). Click Favorites → its matches fill the grid;
   `LibraryView` + the Retroverse header card name + count the active filter node.

**Almost / deferred:** **dump-quality** filter node — reserved (a `dumpQuality` spec kind resolves inert
via `asSmartListKind` returning null). As a *navigation node* it needs a product meaning ("show bad
dumps"? "verified only"?) + an any-vs-default-variant call, and its data lives variant-side
(`groupsByVariantId`), not on `RomEntry`. Wires once the semantics are decided. **NT4 narrowing is
clickable only at the resolver/test level this slice** — the operator can't set "filter within parent"
or nest filter nodes until Slice 4 adds that UI (same shape as Slice 1 shipping membership before
drag-into-tree).

**Next:** **Slice 3** — per-node view cascade (the BigBox change-view-on-the-fly behavior): generalize
the `layoutOverrides` key `(themeId, systemId, view)` → `(themeId, nodeId, view)` + the live in-context
"change view" control (generalize the shipped L5 system-keyed Layout editor). OR **Slice 4** —
drag-to-curate + the `filterWithinParent` toggle UI + real in-tree filter/collection nodes (makes NT4
narrowing operator-clickable) + retire the Collections tab.

## 2026-06-19 — Arc RE-ANCHORED as "Navigation Structure" + full program planned (no new code)

Mid-session, the operator flagged that the arc had drifted into theme framing ("this was not meant to
be a theme thing… it's supposed to be in the settings system"). Investigation confirmed it: the actual
2026-06-18 BigBox research + vision lived **only in a `/clear`ed planning conversation**, so the *plan*
survived but the *intent* didn't — the recurring drift. Recovered it from the transcript and persisted
it as the anti-drift artifact **[RESEARCH.md](RESEARCH.md)**.

**Shipped (docs only):**
- **RESEARCH.md** — the vision (collections ARE the navigation layer; user-authored nested tree), the
  BigBox study (4 levels, view-per-level, change-on-the-fly, remembered, playlists as nodes), how we
  outdo it (arbitrary user nesting), and the two-layer drift diagnosis.
- **Two Explore audits** mapped reality: the **authoring** surface (Settings → "Organize My
  Collection" already hosts the View Editor + Collections Manager + show/hide-systems; full container
  CRUD; drag is leaf-only) and the **settings/override** surface (`SettingsPanel` categories;
  `layoutOverrides` keyed `theme/system/view`; only `game-browse` has a renderer). **Verdict:
  reconnect + enrich, not rebuild.**
- **Re-anchored the arc** (DECISIONS **NT7**): Navigation Structure = Settings-level, user-authored;
  themes only style. Operator chose to **plan the whole arc up front** and **unify the three editor
  cards into one "Navigation Structure" editor**. Rewrote `PLANS/unified-nav-tree.md` to the 7-slice
  program (authoring-richness first → unify → per-node views → drill-through levels → polish); README
  reframed; VIEW_MODEL.md demoted to the subordinate styling-cascade reference.

**Almost:** Slice 2 (filter nodes + NT4) is green on `feat/unified-nav-tree-s2` but unmerged — it
merges as *substrate*; its read-only sidebar sections are transitional (replaced by real authored
nodes in arc Slice 1).

**Next:** **Arc Slice 1** — make collections & smart-lists real, authored, draggable tree nodes via
the View Editor + sidebar (wire the reserved `collection`/`filter` rules into the authoring UI). Decide
the Slice 2 branch fate (merge as substrate) first.
