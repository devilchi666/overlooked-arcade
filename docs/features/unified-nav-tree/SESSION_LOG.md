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

## 2026-06-18 — Slice 1 (keystone) — code-complete, awaiting operator playtest

**Shipped (branch `feat/unified-nav-tree`, all 5 steps; tsc + lint + 261 vitest + 869 cargo green):**

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

**Next:** operator playtest via `cargo tauri build` (acceptance: a "Collections" section appears in the
Retroverse sidebar; clicking a collection shows exactly its member games; system/group nodes
unchanged — watch the stale `<exe_dir>/themes/` shadow landmine). Then merge to main. After that,
**Slice 2** (filter / smart-list nodes — wire the reserved `filter` rule to the existing smart-list
predicates + `VariantFilters`).
