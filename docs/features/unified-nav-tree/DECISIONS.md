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
