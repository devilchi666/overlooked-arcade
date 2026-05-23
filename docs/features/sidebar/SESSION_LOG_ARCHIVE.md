# Sidebar — Session Log Archive

Older entries split out of docs/features/sidebar/SESSION_LOG.md on 2026-05-22 to keep the live file under the ~150-line cap. The two most-recent entries (umbrella v2.1→v3.3 summary + PR-γ tree-render PR) live inline; the earlier infrastructure PRs (PR-α + PR-β) live here.

---


## 2026-05-22 — Sidebar tier PR-β: views.json + ViewsStore + SidebarView fold (cross-system)

Second PR of sibling `SIDEBAR_TIER_PLAN.md`. Pure-plumbing PR — zero
user-visible UI change. The sidebar still renders as a flat list of
platform leaves; the data model underneath is now a tree of containers
+ platform nodes driven by the new ViewsStore. PR-γ will replace the
flat render with the recursive tree + count badges + migration banner.

Pre-session note: previous attempt at PR-β was lost to a power outage
mid-implementation. Restarted fresh from `e6e8f4b`; the discarded WIP
was structurally similar to what shipped here (caught up on lessons
learned from the inspection rather than blindly re-deriving).

- **Shipped:** PR-β of 3 from sibling `SIDEBAR_TIER_PLAN.md`. Three
  phase commits squashed via `--no-ff` merge into main as `a049dd4`.
  - **β.1 Rust persistence (`461d29b`)**: new
    `apps/oa-shell/src/views.rs` with the full ViewsConfig schema
    (ViewsConfig / View / ViewKind / ViewNode / ContainerNode /
    PlatformNode / ContainerRule). Atomic writes (tempfile + rename)
    — views.json carries enough operator state that a power-loss
    mid-write would actually hurt, unlike layout.json. `read_views`
    returns `Option<ViewsConfig>` (not a defaulted struct) so the
    frontend can distinguish "file present" from "first launch" and
    pick the right migration path. Two new Tauri commands
    `get_views` / `set_views`, registered in `invoke_handler!` next
    to layout's. Five unit tests cover round-trip, atomic-write
    cleanup, missing-file → None, camelCase + tagged-enum JSON
    shape (must match frontend types), and forward-compat parse of
    older configs lacking `bannerDismissed`.
  - **β.2 Frontend views/ module (`00a2f00`)**: five new files
    under `frontend/src/views/` totalling 638 LOC.
    - `types.ts` mirrors the Rust serde shape one-for-one
      (internally-tagged ViewNode discriminant).
    - `defaults.ts` auto-buckets every registered system into 5
      form-factor containers (Consoles / Handhelds / Computers /
      Arcade / Other), leaves sorted by displayName within each
      bucket. Node-id helpers (`platformNodeIdFor`,
      `parsePlatformNodeId`) keep the encoding stable across
      producer / consumer / synth-fallback.
    - `resolver.ts` ships `findNode` (DFS), `flattenLeaves` (PR-β's
      flat render source), `resolveNodeSystemIds` (container rule
      eval against the registry, with synth-leaf fallback for
      deep-links outside the active view), `countGamesUnder`
      (PR-γ count badges), `synthesizeLeafForSystem`.
    - `migration.ts` builds the "Flat (Legacy)" view from the
      operator's pre-views `layout.systemOrder` (preserves their
      drag-reorder work as a selectable view). `reorderForFormFactor`
      implements Option C — preserves operator's relative ordering
      within each form-factor bucket when they pick "Try Form Factor
      view" in PR-γ's banner.
    - `store.ts` — `createViewsStore()` Solid composable matching
      LayoutStore's shape. Hydrates from `get_views`; seeds defaults
      + optional Flat-Legacy on first launch (migration path
      decided via `get_layout`'s `systemOrder`); write-through to
      `set_views` on every mutation, gated on `hydrated()`.
      Mutations: setActiveView, setBannerDismissed, toggleExpanded,
      setNodeHidden, reorderChildren, reorderTopLevel, replaceView,
      commitTryFormFactor (batched).
  - **β.3 SidebarView fold (`6ccf22d`)**: hard rewrite of the
    SidebarView discriminant — `{ kind: "system"; id }` removed
    entirely (no alias period; TypeScript catches every missed
    call site), replaced with `{ kind: "view-node"; viewId; nodeId }`.
    Five call sites in App.tsx + LeftSidebar + LibraryView updated
    via two new helpers (viewForSystem / viewToSystemId). LeftSidebar
    renders from `flattenLeaves(activeView.root)` with per-leaf
    parent-id tracking; drag-reorder writes to
    `ViewsStore.reorderChildren` for same-parent drops, silently
    no-ops cross-parent drops (cross-container drag is PR-γ
    surface). `library/filter.ts` `filterEntries` now takes a
    resolved `viewSystemIds: ReadonlyArray<SystemId> | null` —
    pure module stays ViewsStore-unaware. LibraryView accepts
    `views` prop and resolves the selection via
    `resolveNodeSystemIds` (full filter) + a single-system
    accessor for page chrome (header / data-system / title).

- **Almost:** N/A — clean three-phase landing. Operator confirmed
  visual no-change + builds green; pushed to origin/main as
  `a049dd4`.

- **Next:** PR-γ from sibling `SIDEBAR_TIER_PLAN.md` §3 — the
  visible-change PR. New `SidebarTreeNode` recursive component
  with twisties + recursive count badges, replaces LeftSidebar's
  flat render with the tree. Migration banner for upgrade installs
  (showing "Try Form Factor view" vs "Stay on Flat (Legacy)").
  Right-click container "Hide from sidebar." Cascade auto-hide-empty
  for containers. Within-container + top-level drag-reorder via
  solid-dnd SortableProvider scopes. LibraryManagerPage
  system-visibility checkboxes rewired from `layout.hiddenSystems`
  to `viewsStore.setNodeHidden(platform_node_id, ...)`.

---

## 2026-05-22 — Sidebar tier PR-α: registry tagging + gb/gbc split + msx/msx2 stubs (cross-system)

First PR of the sidebar hierarchy execution sheet
(sibling `SIDEBAR_TIER_PLAN.md`). Frontend registry now carries the
`formFactor` + `manufacturer` tags the upcoming default Platforms view
consumes, plus the three system splits/adds the plan's bucket lists
call for. Originally logged under `nds` (then ACTIVE_CORE) because
the work is cross-system UI infra, same pattern the UI polish PRs
followed.

- **Shipped:** PR-α of 3 from sibling `SIDEBAR_TIER_PLAN.md`.
  - **Type plumbing:** `FormFactorTag` + `ManufacturerTag` union types
    added to `frontend/src/themes/registry.ts`; `SystemTheme` extended
    with required `formFactor` + `manufacturer` fields.
  - **Registry audit:** all 38 existing entries tagged per the plan
    §1.2 tables (28 console / 9 handheld pre-split / 1 arcade; MAME →
    manufacturer `other` per operator confirmation pending MAME-import
    metadata work).
  - **gb/gbc split:** existing combined `gb` entry split into
    `gb` (.gb, DMG only, displayName "Nintendo Game Boy") and the new
    `gbc` (.gbc, CGB only, displayName "Nintendo Game Boy Color").
    Both share the Gambatte core via `oa_core::SystemId::Gb`. New
    `[data-system="gbc"]` block in `systems.css` (translucent-cart
    magenta at hue 320°, distinct from gb pea-green / Lynx purple
    290° / SNES violet 270°). Rust dispatch arms in `bindings.rs`
    (bit_for, buttons_for, to_libretro_bits, defaults_for) all extend
    `"gb" =>` to `"gb" | "gbc" =>` since gbc reuses gb's hardware
    layout. Thumbnail-repo split in `media.rs` + libretro-database
    split in `rom_hashes.rs` + metadata-source split in `metadata.rs`
    so each slug resolves to its own dat / repo. `cli.rs`
    `slug_for_ext` no longer aliases `.gbc → "gb"`; each extension
    routes to its own slug.
  - **MSX / MSX2 stubs:** new `msx` + `msx2` entries in
    `frontend/src/themes/registry.ts` with `formFactor: "computer"`
    + `manufacturer: "microsoft"`. Extensions list empty by design
    so the library scanner produces no entries for them today.
    `[data-system="msx"]` + `[data-system="msx2"]` CSS blocks (royal
    blue at hue 250°, MSX2 brighter at L=0.65 vs MSX L=0.55).
    `main.rs` `parse_system_id` already had `"msx" | "msx2" → SystemId::Msx`;
    `default_core_dll_for_system` extended with the same arm pointing
    at `bluemsx_libretro.dll`. Bindings / BIOS pre-check / per-folder
    `.rom` disambiguation **deferred to a dedicated "MSX system add"
    follow-up PR** — declared-but-not-yet-runnable in the spirit of
    the plan's PR scope discipline.
  - Frontend roster: 41 systems (28 console + 11 handheld with the
    gbc split + 2 computer + 1 arcade). `npm run typecheck` clean.
  - Rust: 405 tests passing across the workspace (oa-shell 342 +
    other crates 63), including the bindings test loops which now
    iterate gbc as well.
- **Almost:** PR-β (views infrastructure + SidebarView fold) — depends
  on operator validation of PR-α first.
- **Next:** After operator thumbs-up + merge, branch `feat/sidebar-tier-beta`
  and start §2 of the plan (views.json schema, Rust commands, ViewsStore,
  default + legacy view construction, migration seeding logic,
  SidebarView discriminant rewrite, library filter extension). Sidebar
  still renders flat in PR-β; PR-γ does the tree UI.

Follow-ups noted for future PRs (filed here so they're not lost):
- **Full MSX/MSX2 wiring** — pick canonical extensions (likely .mx1
  / .mx2 / .dsk + per-folder rule for .rom), populate `MSX_BUTTONS`
  + remap + `default_msx_bindings`, plumb through bindings.rs test
  arrays, add BIOS pre-check shape, document in
  `docs/cores/msx/README.md`. Probably a single dedicated branch
  rather than rolling into PR-β.
- **Library Manager visibility** — the system-visibility checkboxes
  in `LibraryManagerPage.tsx` don't yet know about msx/msx2. Will be
  handled by PR-γ's wholesale Settings reconciliation against the
  active view's leaves.
