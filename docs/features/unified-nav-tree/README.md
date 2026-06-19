# Unified Navigation Tree

**One user-authored tree of nodes — system · group-of-systems · collection · filter — each rendered
by a per-node view (the BigBox model, declaratively).** Reunites the two systems that drifted apart:
the systems-only `views` sidebar tree and the separate flat `Collections` tab.

- **Plan + slices:** [../../PLANS/unified-nav-tree.md](../../PLANS/unified-nav-tree.md)
- **🔑 Authoritative view model (read first):** [VIEW_MODEL.md](VIEW_MODEL.md) — how a view is
  decided / kept / shown; the two meanings of "view"; the cascade; the styling-vs-freedom tradeoff;
  BigBox mapping. This is the anti-drift artifact.
- **Decisions:** [DECISIONS.md](DECISIONS.md) (NT1–NT3)
- **Log:** [SESSION_LOG.md](SESSION_LOG.md)

## Why this exists

Began as "Declarative Showcase S3" and surfaced that (1) file themes can't do per-system views
(`DeclarativeShell` is a flat all-systems browse) and (2) collections were *meant* to be the
navigation layer but shipped as a separate tab. Both are the same foundation: navigation as a tree
of nodes, each node rendered by a selectable+remembered view. Building it closes the per-system-view
gap, the collections gap, and BigBox parity at once.

## Keystone

Every node resolves to `SystemId[]` today (`platform/views/resolver.ts:55`) → games filtered by
system. Collections/filters resolve to **games directly**. The one architectural change the arc
pivots on: generalize node resolution + the library filter from *systems-only* to *systems OR an
explicit game set OR a predicate*. Everything else is additive.

## Distinct from `features/unified-nav/`

That feature is the spatial/input navigation engine (focus movement, layers). **This** is the
navigation *content model* (what's in the tree, how nodes render). They compose; they are not the
same thing.

## Status

Planned + documented 2026-06-18 (operator-approved: next arc, incremental). **Slice 1 (keystone)
queued — not yet started.** Declarative Showcase S3 polish parks and resumes inside Slice 5.
