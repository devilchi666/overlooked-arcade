# Navigation Structure — the research & the vision (anti-drift source of truth)

> **Why this file exists.** On 2026-06-18 we did extensive research (BigBox study + a six-audit
> sweep of our own code) that grounded this whole arc. That research lived **only in the planning
> conversation**, which was later `/clear`ed — so the *plan* survived but the *intent behind it*
> did not, and the framing drifted toward "themes." On 2026-06-19 the operator caught the drift
> again. This file captures the research in the repo so it can't be lost a third time. Read it
> before touching navigation / collections / views. Plain language on purpose — no jargon fog.

## The one-line framing (locked 2026-06-19)

This feature is **Navigation Structure**: *how the user organizes and walks their library.* It is a
**user-authored, settings-level** feature. It is **not** a theme feature — themes only *optionally
style* the views a structure uses; they never own the structure itself. The goal is to make
Navigation Structure **robust, feature-filled, and options-filled — the most ways possible for a
user to set up how they navigate.**

## The operator's vision (2026-06-18, in their words)

> "Collections… it's supposed to be part of the navigation level. The idea was collections was the
> way users made lists and groups of systems etc, or categories like 2-player games / favorites…
> You were supposed to be able to drag them in and out and rearrange them in an order and as deep
> as you wanted to go. It was supposed to rival BigBox's way of doing navigation lists."

Unpacked:

- **The navigation layer IS a tree the user builds and arranges themselves.** There is no wall
  between "the menu" and "my lists" — shaping a collection *is* shaping your navigation.
- **A node can be:**
  - a **single system** (NES),
  - a **group of systems** ("Nintendo Handhelds"),
  - a **filtered / auto-populated set of games** ("2-Player", "Favorites", "Beat-'em-ups"), or
  - a **hand-curated list** (games the user explicitly added).
- **You drag nodes in and out, reorder them, and nest them as deep as you want.**
- The user can **set a default view for everything**, then **override the view on any individual
  node** — eventually down to a single game (future).

## How BigBox does it (the thing we're rivaling)

BigBox is a **hierarchy of navigation levels, and each level has its own swappable "view":**

`Platform Categories → Platforms / Playlists → Games → Game Details` (plus a Marquee screen for a
second monitor).

- **Platform level** — a wheel/list of all your systems (e.g. *PlatformWheel*, *TextFilters*).
- **Games level** — *CoverFlow*, *Wall* (grid), *Wheel*, *Text list*, etc.
- **Details level** — a *GameDetails* panel.
- **Change the view on the fly** (a "change view" button), and BigBox **remembers your choice per
  platform** (NES can be CoverFlow while Arcade is a Wheel), as deep as the hierarchy goes.
- **Playlists are first-class navigation nodes** in that hierarchy — they sit alongside platforms
  and can nest inside them.

Sources: [Big Box Views](https://feedback.launchbox-app.com/help/articles/9450321-big-box-views) ·
[Change View Per Platform](https://www.youtube.com/watch?v=pgxJtgcGzxM) ·
[Separate view per platform category (feature request)](https://feedback.launchbox.gg/p/separate-view-per-platform-category-in-bigbox)

## How we *outdo* BigBox

BigBox's scheme is **fairly fixed** (Categories → Platforms/Playlists → Games). Ours is a **fully
user-authored, arbitrarily-nested tree.** Same "navigation = nodes with swappable, remembered
views" idea — but we win on the **organize-it-yourself** axis: any node kind, any depth, any
arrangement, all authored by the user. That is the ambition: *the most flexible navigation
structure of any frontend.*

## The view cascade (who decides how a node is drawn)

Separate axis from *structure* — this is *styling*, and it's the only place themes participate.
Lowest → highest wins:

1. **Global default** — a default view for everything.
2. **Theme author's per-view default** — the theme ships its intended look for a level/node.
3. **Per-node override** — the user sets a specific view on any individual node (this system, this
   collection… eventually this game).
4. **User override wins** — with an honest caveat: if the user flips a node to a view the author
   never themed, that view falls back to **baseline** styling and may look plain/wrong. *That's the
   user's call* (polish traded for control). Authors who theme *all* views let the user flip freely
   with no downside. We never lock the user out of an unstyled view.

## What our code already has vs. what's missing (the 2026-06-18 audit)

The good surprise: OA's data contract was **designed around exactly this hierarchy** — the renderer
just never caught up. Almost 1:1:

| BigBox concept | OA's existing contract | Built? |
|---|---|---|
| Platform Categories level | `manufacturer-browse` view type | ❌ declared, no renderer |
| Platforms/systems level (wheel of systems) | `system-browse` view type | ❌ declared, no renderer |
| Games level | `game-browse` view type | ✅ but **flat** (all systems at once) |
| Game Details | `game-details` view type | ⚠️ overlay only |
| View styles (CoverFlow/Wall/Wheel/Text) | `LayoutPrimitive` (carousel/grid/wheel/list) | ✅ |
| "Remember a view per platform" | the `per_system` layout map | ⚠️ parsed + validated, was unused (L3–L5 now wire it for game-browse) |
| "Change view on the fly, remembered" | the `(theme, system, view)` override store + Settings "Layout" override UI | ✅ exists, renderer-agnostic |
| Playlists as first-class nav nodes | custom collections | ⚠️ existed as a **separate tab**, not a nav node (Slice 1 made them navigable) |
| Marquee (2nd monitor) | reserved `surfaces` (marquee/control-panel) | ⏸ reserved |

## The drift — two layers stacked (why it felt like fog)

1. **Collections drift:** collections shipped as a *separate flat tab/surface*, not as the
   tree-building primitive for navigation. (Intent existed; no plan/decision ever wrote it down.)
2. **Renderer drift:** the declarative shell draws *one flat level* (`game-browse`, all systems at
   once) instead of the multi-level hierarchy the contract already describes; `system-browse` /
   `manufacturer-browse` have no renderer.
3. **Framing drift (added 2026-06-19):** the plan + `VIEW_MODEL.md` led with the *theme-styling*
   cascade and housed work under `themes/retroverse/`, which buried the **navigation-structure**
   feature under theme jargon — the recurring "jargon messed me up" problem. The structure is the
   user's (a Settings concern); themes only style it.

## Where the shipped slices actually sit (honest status, 2026-06-19)

- **Slice 1 (merged):** generalized a node to resolve to *games* (not just systems); made custom
  collections navigable via a read-only sidebar section. ✅ the right low-level primitive.
- **Slice 2 (on `feat/unified-nav-tree-s2`, not merged):** filter/smart-list nodes (Favorites /
  Recently Played / …) + NT4 ancestry-aware membership (nest a node and narrow within its parent).
  ✅ also the right primitive — **but** bolted onto the existing *flat* game-browse sidebar, and
  framed/filed as theme work, so it does **not yet** deliver the navigation-*levels* hierarchy or a
  Settings authoring surface.

**Net:** the membership/depth *primitives* are sound and reusable. What's missing for the vision is
(a) the **navigation-levels hierarchy** (system-browse / drill-in, the renderer the contract
describes) and (b) a **Settings authoring surface** where the user builds/arranges the tree — plus
the breadth of node kinds + arrangement options that make it "robust, feature-filled."

## Open questions for the plan (to settle with the operator)

- **Authoring home:** what does the Settings "Navigation Structure" editor look like, and how does
  it relate to the existing sidebar View Editor + the Collections tab (which retires)?
- **Node kinds, in full:** system · system-group · auto/smart filter · hand-curated list · the
  nav *levels* themselves (manufacturer/system browse) — what's the complete catalog, and how
  much nesting/cross-kind mixing do we expose?
- **Auto vs. curated lists:** smart (predicate, auto-populating) vs. manual (explicit members) vs.
  curated (shipped via content packs) — how do all three live as nodes?
- **Per-node view override UI** (the cascade's step 3/4) — where it lives, how "change on the fly"
  is surfaced, and how it stays a *structure*-owned setting that themes merely style.
- **Defaults & sharing:** export/import a navigation structure; ship starter structures.
