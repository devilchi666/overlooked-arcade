# Navigation Structure

**A Settings-level, user-authored navigation tree — system · group-of-systems · smart list · curated
collection · folder — dragged in/out, nested as deep as you want, each node rendered by a view you
can change and that's remembered.** The goal is to rival and *outdo* BigBox on the
organize-it-yourself axis. Themes only optionally *style* the views; they never own the structure.

> Re-anchored 2026-06-19 from a theme-led "Unified Navigation Tree" framing that drifted into jargon.
> The folder name stays `unified-nav-tree` for history; the feature is **Navigation Structure**.

- **🔑 Vision + BigBox research + drift history (read first):** [RESEARCH.md](RESEARCH.md) — the
  anti-drift source of truth.
- **Plan + slices:** [../../PLANS/unified-nav-tree.md](../../PLANS/unified-nav-tree.md)
- **Styling-cascade reference (subordinate to RESEARCH):** [VIEW_MODEL.md](VIEW_MODEL.md) — how a
  node's *view* (layout) is decided/kept/shown. This is the *styling* axis only; structure is the
  user's.
- **Decisions:** [DECISIONS.md](DECISIONS.md) (NT1–NT7)
- **Log:** [SESSION_LOG.md](SESSION_LOG.md)

## The reframe in one line

This is **not** a theme feature. The structure (what nodes exist, how they nest, what each contains)
is **authored in Settings → Navigation Structure** and persisted to `views.json`. Themes contribute
*only* default styling for views, which the user can override.

## Reconnect, not rebuild

The 2026-06-18 audit found the home (Settings → "Organize My Collection"), the data model
(`views.json` + the `collection`/`filter`/`filterWithinParent` rules), the membership resolver, the
smart-list predicates, and the per-node view-cascade substrate **already exist** — they just aren't
unified or fully exposed. The arc enriches + unifies them. See the plan for the 7-slice program.

## Distinct from `features/unified-nav/`

That feature is the spatial/input navigation engine (focus movement, layers). **This** is the
navigation *content model* (what's in the tree, how nodes render). They compose; not the same thing.

## Status

Arc re-anchored + full program planned 2026-06-19 (operator-approved). **Substrate:** Slice-1 merged;
Slice-2 (filter nodes + NT4) green on `feat/unified-nav-tree-s2`, merges as plumbing. **Arc-Slice 1
(collections & smart-lists as real authored nodes) queued — not yet started.**
