# Theme View Model — authoritative (the anti-drift artifact)

**This is the part that drifted before.** The word "views" got conflated and the navigation +
collections systems were built disconnected. This file is the single authoritative model for how a
view is **decided, kept, and shown**. Read it before touching anything view/layout related; update
it (not your memory) when the model changes.

Status: model locked 2026-06-18 (Unified Navigation Tree arc). Consumers wired incrementally across
the arc's slices — see [../../PLANS/unified-nav-tree.md](../../PLANS/unified-nav-tree.md).

## Two meanings of "view" — lock these so we never conflate them again

- **View-as-layout** — *how* a node's items are drawn: `list / grid / carousel / wheel / custom`
  (the `LayoutPrimitive` enum). The BigBox "CoverFlow vs Wall vs Wheel vs Text" axis.
- **View-as-level** — the navigation *levels* themselves: `manufacturer-browse / system-browse /
  game-browse / game-details` (the `ViewType` enum). Each level is itself a view that gets a layout.
- **This arc delivers** per-node *view-as-layout* (rendered for file themes in Slice 5); the extra
  *view-as-level* screens land in Slice 6. Both already exist as *declared* contract (`ViewType` +
  `LayoutPrimitive` in `platform/theme/manifest.ts`); only `game-browse` has a renderer today.

## The per-node view cascade — who decides which layout a node uses (lowest → highest wins)

Mirrors the existing `platform/theme/layoutResolver.ts` cascade; this arc **generalizes its axis
from "system" to "node"** so collections/filters/groups participate too.

1. **Engine default** — every `LayoutPrimitive` is always renderable from a baseline
   (`ENGINE_DEFAULT_LAYOUTS`). Guarantees a node *always* shows, even with zero theme styling.
2. **Theme default for the level** — `theme.toml` `views[viewType].layout`.
3. **Theme default for the node** — `views[viewType].per_system[systemId]` today; **extended by this
   arc** to per-node-kind / per-node (a collection's default, a specific node's default).
4. **User override for the node** — the user's explicit choice for *this* node, set live, remembered.

## How each view choice is KEPT — two homes, never mixed

- **Theme-author defaults** live in the **theme** (`theme.toml` manifest `views`). Shipped with the
  theme; read-only to the user. "What the author intended each level/node to look like."
- **User overrides** live in the **override store** (`layoutOverrides`, localStorage, scoped to the
  active theme). This arc **generalizes its key `(themeId, systemId, view)` → `(themeId, nodeId,
  view)`** so any node (system / group / collection / filter) can carry an override; system nodes
  keep working unchanged. "Remembered on the fly" = a write here; survives restart, per theme.
  Per-game override later is the same key with a game id as `nodeId` — additive, no redesign.

## How each view is SHOWN — render

- The renderer resolves the cascade **for the current node** and mounts the matching platform nav
  primitive (`GridNav / ListNav / CarouselNav / WheelNav / CustomNav` — all already renderer-agnostic).
- Today only `LibraryView` (Retroverse) consumes this, keyed by the selected *system*. **Slice 5**
  makes `DeclarativeShell` consume the *same* cascade keyed by *node*, so file themes get it too.

## Theme styling vs. user freedom — the honest tradeoff (operator-locked)

- A view is **always renderable** (engine baseline). A theme **styles only the views it chooses to**.
- If the user overrides a node to a view the theme **didn't** style, it renders with baseline styling
  and may look plain / off — **that is the user's call** (they trade polish for control). Authors who
  style **all** views let the user flip freely with no downside. We never *lock* a user out of an
  unstyled view; it just falls back to baseline. Do not "fix" this into a restriction later.

## On-the-fly "change view" — BigBox parity

- A live, in-context "change view" control sets the per-node user override immediately and persists
  it (BigBox's change-view button + remembered-per-platform). This is **Slice 3's** user surface.
- The engine Per-System Hub **"Layout" editor already shipped (L5)** does a *static, system-keyed*
  version of exactly this. Generalize it to nodes + add the live in-context switcher — **reuse, don't
  rebuild.**

## BigBox mapping (keep the intent legible)

`manufacturer-browse` ↔ BigBox Platform-Category · `system-browse` ↔ the wheel/list of all systems ·
`game-browse` ↔ Games view · `game-details` ↔ Game Details. `LayoutPrimitive` ↔ BigBox view styles
(CoverFlow/Wall/Wheel/Text). The per-node override store ↔ BigBox "remember a view per platform."
**We are rebuilding BigBox's per-level-per-node, change-on-the-fly, remembered view model —
declaratively, as data.**
