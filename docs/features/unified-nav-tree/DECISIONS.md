# Unified Navigation Tree — Decisions

Append-only. Prefix `NT` (Nav Tree).

## NT1 — Navigation is one user-authored tree of nodes (2026-06-18)

Navigation is a single user-authored tree; each node is a **system**, a **group of systems**, a
**collection** (explicit game set), or a **filter** (predicate over games). Each node is rendered by
a **view** chosen via the per-node cascade (see [VIEW_MODEL.md](VIEW_MODEL.md)). **Collections are
node kinds in this tree, not a separate tab.**

**Why:** this is the operator's original vision and BigBox's model ("navigation = tree of nodes,
each with a view, changeable on the fly, remembered"). It also closes the file-theme per-system-view
gap — the declarative shell just renders "the current node's view." Supersedes the implicit
`views`-tree-only navigation + separate-`Collections`-tab split.

## NT2 — The theme-view model is documented authoritatively in VIEW_MODEL.md (2026-06-18)

The full model — two meanings of "view" (layout vs level); the lowest→highest cascade (engine
default → theme level default → theme node default → user override); the two persistence homes
(theme manifest for author defaults, the generalized `(themeId, nodeId, view)` override store for
user overrides); the styling-vs-freedom tradeoff (user override wins even into an un-themed view, at
their own cosmetic risk; never lock the user out); on-the-fly change; the BigBox mapping — lives in
[VIEW_MODEL.md](VIEW_MODEL.md).

**Why:** this is exactly the concept that drifted before (conflated "views" → two disconnected
systems). The operator explicitly asked for it to be documented well so it can't drift again. One
authoritative file beats re-derivation.

## NT3 — Collections-as-navigation was never formally specced; this arc is the join (2026-06-18)

The 2026-06-18 audit found **no** plan or decision that ever wrote down "collections = the navigation
tree." The systems-only `views` tree and the flat `Collections` tab were separate from day one
(`SIDEBAR_TIER_PLAN.md` + `_archive/PLANS/collections-tab-retroverse.md`). The unified intent lived
only in the operator's head.

**Why record it:** so the history is honest. This arc is a *join of two never-joined halves*, not a
regression fix — nothing to "restore," everything additive on a strong foundation.

## NT4 — Node membership composes via a per-node "filter within parent" toggle (2026-06-18)

A node's membership can either stand alone or **narrow its parent**, chosen per node by a single
boolean (default **off**). Operator's words: "a checkmark box on a collection or node that says
filter; if you don't filter it just shows the standard."

- **Off (default) — independent / "standard":** the node resolves on its own membership regardless
  of where it sits. A "2-player" node shows *all* 2-player games; its parent is purely visual
  organization. No surprises, no implicit coupling.
- **On — "filter within parent":** the node's membership is intersected with its parent's effective
  membership. "2-player" (on) under "Favorites" = *favorite ∧ 2-player*.
- **Chaining:** consecutive on-nodes compose up the chain — `gun (on) ⊂ 2-player (on) ⊂ Favorites`
  = *favorite ∧ 2-player ∧ gun*. The accumulation stops at the nearest **off** ancestor, which is the
  base set (and at the root, which is "all games"). This is what makes "go even deeper" behave as
  expected.
- **Cross-axis intersection falls out for free:** a game-set/filter node set to "on" under a
  **system group** intersects with that group's games (e.g. "2-player" on, under "Nintendo" =
  2-player Nintendo games). Membership is the common currency, so systems ∩ games composes once
  resolution is ancestry-aware.

**Why this shape:** it delivers *both* models the operator wants (folder-organization AND
narrowing) from one orthogonal switch, rather than two node kinds or a global mode. It's additive —
default-off means existing flat nodes are unchanged.

**Architectural consequence (load-bearing):** composition makes membership **ancestry-dependent** —
a node can no longer be resolved by id in isolation (`resolveNodeMembership(view, nodeId, …)` today).
The composing resolver needs the node's **path from root** to fold the intersections. Build this into
the resolver when filter nodes + real nesting land (Slices 2–4); reserve the per-node boolean on
`ContainerNode` (TS + Rust mirror) at the start of Slice 2 (D59-style reserve-then-wire). Note this
is the **membership** axis (which games a node contains), kept deliberately separate from the
**view/layout** axis in [VIEW_MODEL.md](VIEW_MODEL.md) — conflating the two is exactly what drifted
before.

## NT5 — Filter specs are named built-ins now; the general predicate AST is a reserved future arm (2026-06-19)

**Built in Slice 2.** A `filter`-rule node's `spec` carries a **named built-in kind**
(`{ kind: "favorites" | "recentlyPlayed" | "completed" | "multiPlayer" | "hiddenGems" | "lastPlayed" }`),
evaluated frontend-side at render via the shared `platform/library/smartLists.ts` registry — the exact
predicates the COLLECTIONS tab already shipped, now lifted to a single source both consumers share.

**Why not the full smart-query AST yet** (the `_archive/PLANS/collections-tab-retroverse.md` §12 "all
PS1 RPGs < 1998" builder): it's a much larger surface (field vocabulary, boolean ops, a builder UI) and
nothing this slice needs. The persisted `spec` stays **opaque JSON** (`Record<string, unknown>` in TS /
`serde_json::Value` in Rust, as reserved in Slice 1), so a `{ kind: "query"; ast: … }` arm is purely
**additive** later with no schema migration — the same low-floor/high-ceiling D59 discipline used across
the project. `asSmartListKind` returns `null` for any unrecognized spec, so unknown/future kinds resolve
to an empty set rather than throwing.

**Eval home:** frontend, at render, over live library state (plan §12 + Slice 1's "frontend store, cheap;
revisit for a backend query only if perf needs it"). Predicates are pure (`nowSecs` injected) so the
resolver is deterministic + unit-testable.

## NT6 — Dump-quality is reserved, not wired, pending node semantics (2026-06-19)

The Slice 2 brief listed **dump-quality** as a target filter node alongside Favorites / Recently Played /
Multi-Player. It's **deferred** (the `dumpQuality` spec kind is recognized-as-unknown → resolves inert):

- **Data lives variant-side, not on `RomEntry`.** Dump-quality (`dumpStatus`/`isHack`/…) is on
  `VariantInfo` inside `groupsByVariantId()`; the canonical predicate is Rust's `variant_passes_filters`.
  The other three filters are clean `RomEntry`-field reads.
- **The node's *meaning* is a genuine product question.** As navigation, does "dump-quality" show *bad*
  dumps (preservation triage), *verified only*, or something else? And is a tile a "bad dump" if its
  **default** variant is bad, or if **any** variant is? Inventing that silently would be a band-aid.

Wire it once those are decided (operator call) — likely as a small typed spec arm `{ kind: "dumpQuality";
exclude: … }` reusing the `VariantFilters` flag set, projected onto tiles via the default variant.

## NT7 — Re-anchored as "Navigation Structure" (Settings-level, user-authored); themes only style (2026-06-19)

The arc is **Navigation Structure**: a Settings-level, **user-authored** feature for how the library
is organized and walked. Themes contribute *only* default styling for a node's view (the cascade in
[VIEW_MODEL.md](VIEW_MODEL.md)); they never own the structure. This **supersedes the theme-led
framing** of the original plan (the "theme view model" centerpiece + work filed under `themes/`
paths), which buried the feature under jargon and caused recurring drift.

**Why:** the operator's vision (captured in [RESEARCH.md](RESEARCH.md)) is "collections ARE the
navigation layer — a tree you build and arrange." The audit confirmed the *structure* logic lives in
`platform/` (theme-agnostic) and is authored in Settings; only the optional *styling* touches themes.
Leading with themes inverted the emphasis. RESEARCH.md is now the source of truth; VIEW_MODEL.md is
the subordinate *styling-cascade* reference.

**Consequences (operator-approved 2026-06-19):**
- **One editor:** the three "Organize My Collection" cards (Sidebar layouts / Collections / Sidebar
  systems) unify into a single **"Navigation Structure"** editor; the Settings category is renamed
  (arc Slice 4). The separate Collections tab (`themes/retroverse/CollectionsPage.tsx`) retires.
- **Whole arc planned up front** — 7 slices, authoring-richness first (lists-as-nodes → deep nesting →
  builder), then unify, then per-node views, then drill-through levels, then polish. See the plan.
- **Slice 2 (`feat/unified-nav-tree-s2`) merges as substrate;** its read-only sidebar sections are
  transitional, replaced by real authored nodes in arc Slice 1.

## NT8 — Arc Slice 2 deep-drag: generalize the existing heuristic, editor-only, controller-move deferred (2026-06-23)

Arc Slice 2 (deep nesting + the `filterWithinParent` toggle) shipped as a **pure-UI** slice — the
substrate (the `filterWithinParent` field, ancestry-aware `resolveNodeMembership`, depth-capable
`moveNode`, `LibraryView` resolution) was already complete on main. Three forks were settled in prose
with the operator before code:

- **Drop heuristic — generalize, don't rebuild.** The drag keeps the pre-Slice-2 rule, lifted from
  leaf-only/2-level to **any node kind at any depth**: *same parent → reorder; cross-parent dropped onto
  a folder → nest into it; cross-parent dropped onto a row → insert beside it.* This was chosen over
  building explicit indent/drop-zones (the fork-1 alternative) because it satisfies the playtest
  (nest a list inside a group), reuses proven logic, and — critically — **does not regress top-level
  folder reorder** (which a naive "drop-onto-folder-always-nests" rule would have broken).
  - **Known, accepted nuance:** dropping a *top-level sibling onto a collapsed folder's row* reorders
    (same-parent), it does not nest. You nest by creating-into a selected folder or dropping onto the
    folder's expanded children. A drop-intent refinement (pointer-position / drop-zones) is deferred to
    a later slice if playtest shows it's needed. This is a closestCenter single-signal limitation, not a
    bug.
  - The rule lives in **one pure, unit-tested place** — `platform/views/dragResolve.ts`
    (`resolveDragOutcome` → `reorder | move | null`, cycle-safe, root-immovable) — so it can't drift
    between the editor and any future sidebar consumer.
- **Editor-only deep-drag.** The new nested-sortable rendering is gated behind an opt-in
  `nestedSortable` flag on `SidebarTreeContext`; only the View Editor sets it. The **live sidebar keeps
  its leaf-only reorder behaviour byte-for-byte** (lowest regression risk; the editor is the authoring
  surface). Enabling deep-drag in the live sidebar later is a one-line flag flip once desired.
- **Controller/keyboard move deferred.** Slice 2 is mouse-drag only. A controller-accessible move
  (right-click / "Move to…" path) is future work; `moveNode` is already the hook for it. Recorded so it
  isn't mistaken for an oversight.
