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
