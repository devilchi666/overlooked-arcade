# Sidebar — Session Log

Entries for the sidebar tier + view editor work. These were originally
filed under `docs/cores/nds/SESSION_LOG.md` because nds was the active core
when the cross-system work shipped — re-filed here 2026-05-22 as part of
the docs reorg so cross-cutting work has a proper home.

---

## 2026-05-22 — Sidebar editor stack: v2.1 → v3.3 (cross-system)

Long stretch after PR-γ. Seven PRs shipped sequentially closing
out the post-tier-plan v2 work, then the v3 View Editor story.
Picked up cleanly each time via the same workflow (pre-feature
branch → phase commits → push → merge --no-ff → delete both
sides). v3.4 per-container art slots intentionally parked on
`../../PARKING_LOT.md` pending storage + format design.

- **Shipped:** v2.1 + v2.2 + v2.3 + v3.1 + v3.2 + v3.5 + v3.3.

  - **v2.1 — Manufacturers view + view picker (`8ad5eb2`):**
    Second shipped default `buildDefaultManufacturerView` auto-
    buckets 41 systems into 15 vendor containers
    (Nintendo / Sega / Sony / NEC / Atari / SNK / Bandai /
    Microsoft / Coleco / Mattel / Magnavox / Fairchild / GCE /
    Panasonic / Other). `ensureShippedDefaults` reconciler in
    ViewsStore appends Manufacturers to existing operators'
    views.json on hydrate (idempotent). Sidebar header gets a
    native <select> view picker — replaces the static
    "Platforms" SectionHeader when more than one view is
    registered.

  - **v2.2 — Cross-container drag + Move-to-category submenu
    (`07e4694`):**
    `moveNode(nodeId, targetParentId, insertBeforeId | null)` in
    ViewsStore — DFS to locate node + parent, remove from
    source, insert into target. Cycle guard refuses self-or-
    descendant moves. `handleSidebarDragEnd` two-path: same-
    parent reorder via reorderChildren (γ.2 unchanged); cross-
    parent leaf drag via moveNode. Container drags across
    scopes still silent no-op. SystemContextMenu gains stacked-
    view "Move to category…" submenu with flat container list
    excluding the leaf's current parent.

  - **v2.3 — Un-hide containers UX (`e34a62a`):**
    `collectHiddenContainers(node)` in resolver.ts —
    DFS-collect any depth. LibraryManagerPage's Sidebar systems
    section gains a "Hidden containers" sub-section that lists
    currently-hidden containers in the active view with [Show]
    buttons. Closes the γ.3 follow-up where hiding a container
    via right-click was a one-way trip without editing
    views.json.

  - **VIEW_EDITOR_PLAN.md + decision lock (`b7628b5`):**
    docs-only commit — locked design contract for v3 with 13
    open Qs resolved (8 by operator, 5 defaulted). Notable
    operator override: Q7 picks native <input type="color">
    over the recommended curated 16-swatch palette.

  - **v3.1 — Views tab + view metadata CRUD (`b58812e`):**
    New "Views" tab in LibraryManagerPage between Library +
    Game media. `createView` / `renameView` / `deleteView` +
    `cloneViewTree` deep-clone helper. Four templates: blank,
    copy-formfactor, copy-manufacturer, copy-legacy. Inline
    rename + two-step delete confirmation. Shipped views are
    editable; only user-built can be deleted.

  - **v3.2 — ViewEditorPane (tree + properties + drag)
    (`b98852d`):**
    Two-pane editor per sibling `VIEW_EDITOR_PLAN.md` §0.3 mockup.
    Selecting a view's [Edit] swaps the tab body for the
    editor. Tree pane reuses γ.2 + v2.2.1's SortableContainer/
    LeafNode unchanged (drag-reorder works inside the editor
    exactly as in the sidebar). Properties pane: per-kind rule
    editor (5-option select for formFactor, 15-option for
    manufacturer, search-filterable multi-select for
    systemIds); [Delete container] / [Remove from view]
    buttons. AddLeafPicker popover with search filter — only
    surfaces systems NOT already in the view. Container CRUD
    mutations (addContainer, setContainerLabel,
    setContainerRule, addPlatformLeaf, removeNode) +
    generateContainerId namespaced as `${viewId}:container:N`.

  - **v3.5 — Schema v2 + explicitlyRemoved + auto-extend
    (`b325c70`):**
    Three-commit closure of the v3.2 delete-leaf gap. v3.5.1
    bumps schema 1 → 2 in both Rust + frontend; View struct
    gains `explicitly_removed: Vec<String>` with serde default;
    `migrate_inplace` updated; two new Rust unit tests.
    v3.5.2 hooks `removeNode` (push leaf's systemId onto
    explicitlyRemoved) + `addPlatformLeaf` (clear from
    explicitlyRemoved when the operator re-adds). v3.5.3
    extends `ensureShippedDefaults` with a second pass that
    auto-extends FormFactor + Manufacturer views with newly-
    registered systems on hydrate, SKIPPING any in
    explicitlyRemoved — Flat-Legacy stays frozen-after-seed.
    Never reorders existing leaves; never modifies hidden
    flags.

  - **v3.3 — Per-container accent picker (`d37ae71`):**
    `setContainerAccent(nodeId, accent | null)` mutation.
    ContainerProperties gets a native <input type="color"> +
    Clear button. SidebarTreeNode ContainerRow applies the
    accent as an inline `--color-system-accent` CSS variable
    when set — cascades into the container's own chrome
    (active-row tint, twisty, count badge) without leaking
    into descendant leaves (those have their own data-system
    rule that resets the variable for their subtree).

- **Almost:** N/A — clean serial landings, each PR merged with
  operator green-light. v3.5 schema migration verified via the
  two new cargo unit tests + by inspecting the upgraded
  views.json on disk.

- **Next:** v3 is complete-enough; v3.4 (per-container art
  slots) stays parked on `../../PARKING_LOT.md` until storage +
  format design lands. Other sidebar gaps tracked in the prior
  "What's left" inventory (Home/Favorites/Recent/Continue
  quick destinations + Playlists/Smart Views placeholders +
  drag-handle touch visibility) remain unresolved. Pick next
  work from `../../NEXT.md` or the parking lot.

---

## 2026-05-22 — Sidebar tier PR-γ: tree render + drag + per-node hide + migration banner (cross-system)

Third (and final) PR of sibling `SIDEBAR_TIER_PLAN.md`. The visible-
change PR — replaces β's invisible plumbing with the real tree
sidebar. Four phase commits squashed via `--no-ff` merge into main
as `ab5a335`.

- **Shipped:** PR-γ of 3 from sibling `SIDEBAR_TIER_PLAN.md`. Plan
  complete; tree sidebar fully landed.
  - **γ.1 Recursive tree render (`4865a75`)**: new
    `frontend/src/layout/SidebarTreeNode.tsx` — recursive component
    rendering container rows (twisty + label + cumulative count
    badge) and leaf rows (per-system accent dot + shortName + own
    count). Indent scales 0.75rem per depth level. Twisty click
    toggles expand state without navigating; label click navigates
    to the node. `LeftSidebar.tsx` rewrite: `filterTree` walks the
    active view bottom-up applying legacy `layout.hiddenSystems` +
    `autoHideEmptySystems` + per-node `hidden` gates with the active
    leaf always preserved (so its ancestor containers stay visible).
    Auto-expand-ancestors-of-active is render-time only —
    operator's collapse choice survives navigation. Collapsed-mode
    sidebar degrades to flat icon list (`CollapsedLeaf`) since
    container chrome doesn't fit in the icon column. "Systems"
    section header renamed → "Platforms" per plan §0.
  - **γ.2 Drag-reorder (`d0b6c8b`)**: solid-dnd nested-scope pattern
    per plan §3.5. `SortableContainerNode` wires `createSortable`
    on the container `<li>` and embeds a per-container
    `SortableProvider` around its leaf children;
    `SortableLeafNode` wires sortable on each leaf. Drag handles
    (⋮⋮, hidden-until-hover) bound to scope drag activation away
    from the navigation buttons. `handleSidebarDragEnd` gates
    drops via a `parentOfId` lookup — same-parent commits;
    cross-parent silent no-op (cross-container drag deferred to a
    post-v1 PR). Writes against the UNFILTERED view tree so hidden
    siblings keep their positions.
    `StaticContainerNode` kept as scaffolding for v3+ deep trees
    (the recursive non-sortable path).
  - **γ.3 Container hide + Settings reconciliation (`4e6bb2a`)**:
    new `frontend/src/components/ContainerContextMenu.tsx` — right-
    click on a container row opens a minimal menu with a single
    "Hide from sidebar" item. `SidebarTreeNode.tsx` ContainerRow
    accepts `onContextMenu`; SidebarTreeContext gains
    `onContainerContextMenu`; LeftSidebar.tsx threads
    `onContainerContext` through. `App.tsx` adds the new
    `containerContextFor` signal and mounts ContainerContextMenu
    sister to SystemContextMenu. New `hideSystemInActiveView`
    helper dual-writes per-node `hidden` flag + legacy
    `layout.hiddenSystems` set. Both code paths (System menu
    "Hide from sidebar" + SystemContextMenu's `onHideSystem`)
    route through the helper. Container hide redirects the
    operator off the hidden subtree if they were viewing a node
    inside it. `LibraryManagerPage.tsx` system-visibility
    checkboxes accept the `views` prop, read state from per-node
    hidden first (active view) with legacy fallback, write to
    both representations. Soft-migration model: no schema bump,
    no one-shot pass — every operator interaction updates both
    representations so the per-node flags catch up with the
    legacy set over a single session. `resolver.ts` adds
    `nodeContainsId` for the redirect predicate.
  - **γ.4 Migration banner (`3467076`)**: new
    `frontend/src/components/SidebarMigrationBanner.tsx` — top-of-
    sidebar banner shown when active view is `flat-legacy` AND
    `bannerDismissed` is false (hidden in collapsed-sidebar mode
    since multi-line copy doesn't fit). "Try Form Factor view"
    button applies `reorderForFormFactor` (β.2's Option C —
    preserves operator's relative ordering within each form-
    factor bucket), commits via the batched
    `viewsStore.commitTryFormFactor` mutation. "Stay on Flat
    (Legacy)" just `setBannerDismissed(true)`. Per-system accent
    border + tinted background so the banner picks up the active
    theme without competing with the primary nav.

- **Almost:** N/A — operator validated end-to-end (forced the
  upgrade-install path by patching layout.json's systemOrder + 
  deleting views.json; banner appeared on Flat-Legacy, Try Form
  Factor button correctly applied Option-C reorder with SNES
  above NES in Consoles and Lynx above GB in Handhelds).

- **Next:** Sidebar tier plan complete; pick up the next item from
  `../../NEXT.md` or `../../PARKING_LOT.md`. Candidate cleanups
  flagged during this session: (a) CLAUDE.md appData path fixed
  here (was stale `com.oa.overlooked-arcade`, now correct
  `dev.overlookedarcade.shell`); (b) `LEGACY_VIEW_ID` constant is
  exported but the underlying view only ever appears via the
  one-shot migration — no current code path needs to query for it
  later, candidate to inline if it stays unused; (c) cross-
  container drag in the sidebar tree (deferred from γ.2 — would
  surface a red drop indicator + a "Move to container…" right-
  click submenu per plan §8 v3).

---

---

Older entries (PR-α, PR-β — the earlier infrastructure PRs of the same plan) live in [SESSION_LOG_ARCHIVE.md](SESSION_LOG_ARCHIVE.md) to keep this file under the ~150-line session-start cap.
