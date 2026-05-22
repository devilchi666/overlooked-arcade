# Sidebar Hierarchy Plan — Concrete Execution Sheet

**Status:** Design locked 2026-05-22. Ready to execute.

**Purpose:** Replace the flat list of systems in the left sidebar with a tiered tree organized by form factor (Consoles / Handhelds / Computers / Arcade / Other) in v1, on top of a new "views" data model that's the same model the kiosk shell will consume in Phase 1 of `docs/KIOSK_PLAN.md`.

**Companion docs:**
- `docs/KIOSK_PLAN.md` §3.3 — "Named views with arbitrary hierarchies" — the data model this plan implements ahead of the kiosk shell.
- `docs/UI_POLISH_PLAN.md` — preceding desktop UI polish work (completed); the sidebar's `LeftSidebar.tsx` rewrite assumes the menu-bar architecture that polish established.
- `docs/UI_AUDIT.md` — original inventory of the flat-systems sidebar surface.

---

## 0. Locked design contract

The following decisions are committed — they're the answers to ~30 design questions worked through in the 2026-05-22 planning session. Future-Claude or another engineer picking this up should not relitigate.

**Architectural model:**
- **Hybrid view system** — top of tree explicit (FormFactor categories hand-defined in code), leaves are platforms (auto-generated per system). Pure-projection views (`Decade →`, `Genre →`) come in v4 of the post-v1 roadmap, not now.
- **Option 3 filter composition** — container nodes own their filter rule. Clicking a container shows games matching the container's rule; clicking a child shows games matching the child's rule. Children are independent — no superset requirement between parent and children.
- **View-as-projection, not folder-containment** — games never "belong to" a category folder. A view defines a tree; each node carries a rule; the runtime filters the library entries by the rule. The same game can appear under multiple nodes in different views simultaneously.

**Persistence:**
- New file `views.json` under `appDataDir`. Schema `version: 1`. Atomic writes from Rust side (tempfile + rename).
- Rust Tauri commands: `get_views()` / `set_views(views)`. Symmetric with existing `get_layout` / `set_layout`.
- Frontend `ViewsStore` (new) hydrates from `get_views` on mount; seeds defaults if absent; writes back via `set_views`.
- `expandedNodes: string[]` per view (separate list; lighter on diffs); `hidden: boolean` inline on each node.

**Registry tags (added this PR):**
- Every entry in `systemThemes` gains `formFactor: FormFactorTag` and `manufacturer: ManufacturerTag`.
- `formFactor` is consumed in v1 (default view filters by it). `manufacturer` is unused in v1 but tagged anyway — pre-emptive groundwork so v2's Manufacturer view is a pure UI change.

**Form-factor enum:**
`"console" | "handheld" | "computer" | "arcade" | "other"`

**Manufacturer enum:**
`"nintendo" | "sega" | "sony" | "nec" | "atari" | "snk" | "bandai" | "microsoft" | "coleco" | "mattel" | "magnavox" | "fairchild" | "gce" | "panasonic" | "other"`

**Default view shipped in v1:**
- ID: `default-formfactor`. Name: `"Platforms"`.
- Root container `Platforms` → Consoles / Handhelds / Computers / Arcade / Other (Other auto-fallback for any future system without a tag).
- Container accents: **neutral** (no override; leaf platforms keep their existing per-system accents).

**Form-factor assignments (final, 39 systems):**

| Bucket | Members (slugs) |
|---|---|
| **Console** (28) | nes, snes, n64, gamecube, genesis, sega32x, segacd, saturn, dreamcast, sms, psx, ps2, tg16, pce-cd, 3do, pcfx, atari7800, 2600, 5200, jaguar, neogeo, neocd, coleco, intv, o2, channelf, virtualboy, vectrex |
| **Handheld** (10) | gb, gbc, gba, nds, psp, lynx, gamegear, ngp, wonderswan, pokemini |
| **Computer** (2) | msx, msx2 |
| **Arcade** (1) | mame |
| **Other** (0) | (auto-fallback for future systems without a `formFactor` tag) |

Notable calls:
- **VirtualBoy** → Console (stationary, AC-powered, not pocketable).
- **Vectrex** → Console (built-in CRT; plays at home).
- **MSX / MSX2** → Computer (preserves the distinction for v2 form-factor work; both ship with keyboards and were marketed as home computers).
- **Neo Geo (AES) + Neo Geo CD** → Console (project's `neogeo` slug is AES-side; arcade MVS lives under MAME tooling).
- **MAME** → Arcade.

**Filter rule schema (v1):**

```ts
type ContainerRule =
  | { kind: "formFactor"; value: FormFactorTag }
  | { kind: "manufacturer"; value: ManufacturerTag }  // unused in v1
  | { kind: "systemIds"; values: SystemId[] };  // for v3 custom containers
```

Root container has `rule: null` (matches everything — equivalent to All Games). Leaf nodes carry `systemId: SystemId` instead of a rule.

**SidebarView discriminant fold:**

```ts
// Before (this plan removes the `system` variant):
type SidebarView =
  | { kind: "all" }
  | { kind: "system"; id: SystemId }
  | { kind: "library-manager" }
  | { kind: "cores" };

// After:
type SidebarView =
  | { kind: "all" }
  | { kind: "view-node"; viewId: string; nodeId: string }
  | { kind: "library-manager" }
  | { kind: "cores" };
```

Hard rewrite — no alias period. TypeScript catches every missed call site since the variant is removed entirely.

**Migration (Option D primary, Option C secondary):**
- First run after upgrade with existing `systemOrder`: views.json gets seeded with both the default FormFactor view AND a "Flat (Legacy)" view built from the operator's flat `systemOrder` (preserves customization). Active view = legacy until operator chooses.
- Top-of-sidebar banner offers: **[ Try Form Factor view ] [ Stay on Flat (Legacy) ]**.
- Clicking "Try Form Factor view" applies Option C — preserves operator's relative ordering within each form-factor bucket (NES retains its place relative to SNES inside Consoles).
- Clicking "Stay on Flat (Legacy)" keeps the legacy view active; FormFactor view stays seeded and selectable in v2's view picker.

**Sidebar layout (unchanged from today's structure, only the Systems section changes):**

```
┌─ Quick destinations (Home / All Games / Favorites / Recent / Continue) ─┐
├─ Platforms (the new tree) ──────────────────────────────────────────────┤
├─ Playlists (placeholder — "No playlists yet") ──────────────────────────┤
├─ Smart Views (placeholder — "No smart views yet") ──────────────────────┤
└─ Collapse toggle ───────────────────────────────────────────────────────┘
```

The Platforms section header label is `"Platforms"` (matches kiosk plan §3.4 terminology).

**Tree visual treatment:**
- Twisty triangles (▸ / ▾) on container nodes.
- Recursive game-count badges on every node (containers show total descendants' games; leaves show their own).
- One indent level per depth (~12px).
- Container row: twisty + label + count badge.
- Leaf row: small platform glyph (extension chip) + shortName + count badge.

**Click semantics:**
- Click root "Platforms" → All Games (`{ kind: "view-node", viewId, nodeId: "root" }`, runtime resolves to "show everything").
- Click container "Consoles" → All games whose system has `formFactor: console`.
- Click leaf "NES" → Games for the NES system.

**Right-click context menus (v1):**
- Container: **single item — "Hide from sidebar"** (no CRUD until v3).
- Leaf (platform): existing `SystemContextMenu` shape (Show library / Edit bindings… / System settings… / Hide from sidebar).

**Behaviors:**
- **Auto-hide-empty cascading**: if a container has zero visible descendants with games, the container is auto-hidden. Same `autoHideEmptySystems` toggle controls both leaf-level and container-level. Default: `false` on fresh install (so the tree is visible while operator is scanning); inherits existing value on upgrade.
- **Default expand state**: everything expanded on first launch; operator's expand/collapse choices persist per-view in `expandedNodes`.
- **Leaf-platform node ID synthesis fallback**: when a deep-link references a system not present in the active view's tree (e.g. operator on a custom view that excludes it), the runtime synthesizes a virtual leaf at the root level rather than erroring. Detail: how the synthesized leaf renders visually is a PR-β implementation decision.

**Drag-reorder scope in v1:**
- Within-container leaf reorder (NES moves above SNES inside Consoles).
- Top-level container reorder (Handhelds moves above Consoles).
- **Cross-container drag deferred to v2.** Operator who wants NES under a different category in v1 has no UI affordance; v3 CRUD provides "Move to category…" right-click submenu.

---

## 1. PR-α — System tag schema + audit

Trivial, no behavior change. Lands first.

### 1.1 Extend `SystemTheme` interface

**`frontend/src/themes/registry.ts`:**

```ts
export type FormFactorTag = "console" | "handheld" | "computer" | "arcade" | "other";

export type ManufacturerTag =
  | "nintendo" | "sega" | "sony" | "nec" | "atari" | "snk" | "bandai"
  | "microsoft" | "coleco" | "mattel" | "magnavox" | "fairchild" | "gce"
  | "panasonic" | "other";

export interface SystemTheme {
  // ... existing fields ...
  formFactor: FormFactorTag;        // NEW
  manufacturer: ManufacturerTag;    // NEW
}
```

### 1.2 Audit all 39 systems

Add both fields to every entry in the `systemThemes` map. Use the form-factor table from §0 plus the manufacturer table below.

| Manufacturer | Members (slugs) |
|---|---|
| **nintendo** | nes, snes, n64, gamecube, gb, gbc, gba, nds, virtualboy, pokemini |
| **sega** | sms, genesis, sega32x, segacd, saturn, dreamcast, gamegear |
| **sony** | psx, ps2, psp |
| **nec** | tg16, pce-cd, pcfx |
| **atari** | 2600, 5200, atari7800, lynx, jaguar |
| **snk** | neogeo, neocd, ngp |
| **bandai** | wonderswan |
| **microsoft** | msx, msx2 |
| **coleco** | coleco |
| **mattel** | intv |
| **magnavox** | o2 |
| **fairchild** | channelf |
| **gce** | vectrex |
| **panasonic** | 3do |
| **other** | mame |

### 1.3 Acceptance for PR-α

- `cd frontend && npm run typecheck` clean.
- All 39 systems have both `formFactor` and `manufacturer` populated.
- Optional unit test asserting every entry in `systemThemes` carries both fields.
- No runtime behavior change.

---

## 2. PR-β — Views infrastructure + `SidebarView` fold

The plumbing PR. Invisible visible-behavior change (the sidebar UI still renders as a flat list, just driven by the active view's leaves via DFS flattening). PR-γ replaces the flat render with the tree.

### 2.1 `views.json` shape

```json
{
  "schemaVersion": 1,
  "activeViewId": "default-formfactor",
  "views": [
    {
      "id": "default-formfactor",
      "name": "Platforms",
      "kind": "user-builtin",
      "expandedNodes": ["root", "container:console", "container:handheld", "container:computer", "container:arcade"],
      "root": {
        "id": "root",
        "kind": "container",
        "label": "Platforms",
        "rule": null,
        "accent": null,
        "art": null,
        "hidden": false,
        "children": [
          {
            "id": "container:console",
            "kind": "container",
            "label": "Consoles",
            "rule": { "kind": "formFactor", "value": "console" },
            "accent": null,
            "art": null,
            "hidden": false,
            "children": [
              { "id": "platform:nes",  "kind": "platform", "systemId": "nes",  "hidden": false },
              { "id": "platform:snes", "kind": "platform", "systemId": "snes", "hidden": false }
              /* … 26 more leaves … */
            ]
          },
          { "id": "container:handheld", "kind": "container", "label": "Handhelds", "rule": {"kind":"formFactor","value":"handheld"}, /* … 10 leaves … */ },
          { "id": "container:computer", "kind": "container", "label": "Computers", "rule": {"kind":"formFactor","value":"computer"}, /* … 2 leaves … */ },
          { "id": "container:arcade",   "kind": "container", "label": "Arcade",    "rule": {"kind":"formFactor","value":"arcade"},   /* … 1 leaf … */ }
        ]
      }
    }
  ]
}
```

Notes:
- `art: null` slot reserved on containers — no UI for editing in v1; v3+ paints banner/logo/clear-logo when CRUD lands.
- Root container has `rule: null` — runtime treats as "match everything."
- The `kind: "user-builtin"` on views distinguishes shipped defaults from theme-shipped advisory views (v2) and user-built custom views (v3).

### 2.2 Rust side

New module `apps/oa-shell/src/views.rs`:

```rust
pub struct ViewsConfig { /* mirror of JSON shape */ }
pub fn read_views(app_data_dir: &Path) -> Result<ViewsConfig, String>;
pub fn write_views(app_data_dir: &Path, views: &ViewsConfig) -> Result<(), String>;
```

`apps/oa-shell/src/main.rs`:
- Two new Tauri commands: `get_views`, `set_views`.
- Register in `invoke_handler!`.
- Atomic writes via tempfile + rename (same pattern as `presentation.json`).

Schema migration on read: when `schemaVersion` < current, run a migration function (no-op for v1 since this is the initial version).

### 2.3 Frontend infrastructure

New files:

- **`frontend/src/views/types.ts`** — TypeScript types mirroring the schema (`ViewNode`, `ContainerNode`, `PlatformNode`, `ContainerRule`, `View`, `ViewsConfig`).
- **`frontend/src/views/defaults.ts`** — `buildDefaultFormFactorView()` returns the seeded `Platforms` tree given the current `systemThemes` registry. Auto-buckets every registered system by its `formFactor`. Hand-coded order within buckets (or alphabetical by `displayName` if you want one less opinion-call).
- **`frontend/src/views/migration.ts`** — `buildLegacyFlatView(systemOrder)` converts an old `layout.json`-style `systemOrder: string[]` into a single-container "Flat (Legacy)" view shape. `reorderForFormFactor(view, systemOrder)` applies Option C — preserves the operator's relative ordering within each FormFactor bucket when they switch.
- **`frontend/src/views/resolver.ts`** — runtime filter evaluation. Given a node and the library entries, return the matching SystemIds. Container rules are evaluated against `systemThemes` to compute the matching system set. Includes the leaf-platform-synthesis fallback.
- **`frontend/src/views/store.ts`** — `ViewsStore` composable, shape similar to `LayoutStore`:
  - Signals: `viewsConfig()`, `activeView()`, `activeNode()`.
  - Hydrate on mount via `invoke("get_views")`; seed defaults if absent (first run); apply migration if existing `layout.systemOrder` present.
  - Write-through to Rust on every mutation via `invoke("set_views", ...)`, gated on `hydrated()`.
  - Methods: `setActiveView(viewId)`, `setActiveNode(nodeId)`, `toggleExpanded(nodeId)`, `setNodeHidden(nodeId, hidden)`, `reorderChildren(parentId, newOrder)`, `reorderTopLevel(newOrder)`.

### 2.4 `SidebarView` fold

**Hard rewrite — no alias period.**

`frontend/src/layout/LeftSidebar.tsx`:
- `SidebarView` discriminant: drop `{ kind: "system" }`, add `{ kind: "view-node"; viewId: string; nodeId: string }`.

`frontend/src/library/filter.ts`:
- `filterEntries(entries, view, query)` extended: when `view.kind === "view-node"`, look up the node in the active view, resolve its rule via `views/resolver.ts`, filter entries by the resolved SystemIds.

Walk every call site that constructs `{ kind: "system", id }`:
- `LeftSidebar.tsx` itself — clicking a system entry now creates `view-node`.
- `App.tsx` — routing logic, search-jump, history nav, anywhere that re-routes after a launch/unload.
- `GridControls.tsx` — view-title resolution.
- `LibraryView.tsx` — view-label switch.
- `SystemContextMenu.tsx` — "Show library" action.
- `TileContextMenu.tsx` — if any direct system navigation.
- Anywhere else TypeScript flags after the variant removal.

Estimated count: 15–20 sites. TypeScript catches misses since the variant is removed entirely.

### 2.5 The sidebar still renders flat in PR-β

To keep PR-β scope contained, `LeftSidebar.tsx`'s render in this PR is unchanged visually — it iterates a flat list of platforms. The list is derived from the active view's tree via DFS flattening (skip containers, emit leaves only). The flat sidebar reads from `ViewsStore` instead of the old `layout.systemOrder`.

This means PR-β has zero user-visible UI change (sidebar looks identical) but the data model is fully swapped. PR-γ then replaces the flat render with the tree render.

### 2.6 First-run seeding logic

In `ViewsStore` `onMount`:

```ts
const config = await invoke<ViewsConfig | null>("get_views");
if (config) {
  // Persistence exists — load it.
  setViewsConfig(config);
} else {
  // First run after upgrade OR fresh install.
  const layoutPrefs = await invoke<LayoutPrefs>("get_layout");
  const hasLegacyOrder = (layoutPrefs.systemOrder ?? []).length > 0;
  const defaultView = buildDefaultFormFactorView();
  if (hasLegacyOrder) {
    // Upgrade path — seed both views, legacy active.
    const legacyView = buildLegacyFlatView(layoutPrefs.systemOrder);
    const seeded: ViewsConfig = {
      schemaVersion: 1,
      activeViewId: legacyView.id,
      views: [defaultView, legacyView],
    };
    setViewsConfig(seeded);
    showMigrationBanner();  // PR-γ
  } else {
    // Fresh install — FormFactor view only, active.
    const seeded: ViewsConfig = {
      schemaVersion: 1,
      activeViewId: defaultView.id,
      views: [defaultView],
    };
    setViewsConfig(seeded);
  }
  await invoke("set_views", { views: seeded });
}
```

### 2.7 Acceptance for PR-β

- `cd frontend && npm run typecheck` clean.
- `cargo test --workspace` clean.
- Manual: clicking each sidebar platform entry still filters the library correctly (visually identical to today; routed through view-node under the hood).
- `views.json` exists in appDataDir after first launch.
- Old operator with `systemOrder` set: views.json contains both views; active is Flat (Legacy); FormFactor view is seeded but not yet visible (migration banner doesn't ship until PR-γ).
- Fresh install: views.json contains only FormFactor view, active.

---

## 3. PR-γ — Sidebar tree UI + migration banner

The visible-change PR.

### 3.1 New `TreeNode` component

`frontend/src/layout/SidebarTreeNode.tsx` — recursive component rendering a node.

Props: `node: ViewNode`, `depth: number`.

Container render:
- Twisty triangle (▸ collapsed / ▾ expanded). Click toggles `expandedNodes` via ViewsStore.
- Label (e.g., "Consoles").
- Count badge: total games across all descendants whose system matches the container's rule.
- Drag handle if applicable (top-level containers only in v1).
- Right-click → "Hide from sidebar" menu.

Leaf (platform) render:
- Small platform glyph (extension chip).
- Short name from `systemTheme.shortName`.
- Count badge: games for this system.
- Drag handle for within-container reorder.
- Right-click → existing `SystemContextMenu`.

Indent: `style={{ paddingLeft: \`${depth * 0.75}rem\` }}` (~12px per level).

### 3.2 `LeftSidebar.tsx` rewrite

Replace the existing flat `<For each={orderedSystemIds()}>` render with a recursive `<SidebarTreeNode>` rooted at the active view's `root.children`. (Root itself isn't rendered as a node — its children become the top-level entries.)

### 3.3 Recursive count badges

In `frontend/src/views/resolver.ts`:

```ts
function countGamesUnder(node: ViewNode, entries: RomEntry[]): number {
  if (node.kind === "platform") {
    return entries.filter((e) => e.systemId === node.systemId).length;
  }
  // Container — sum descendants.
  return node.children.reduce((sum, c) => sum + countGamesUnder(c, entries), 0);
}
```

Wrap in a `createMemo` per node so reactive updates are efficient.

### 3.4 Cascade auto-hide-empty

When `autoHideEmptySystems()` is true:
- Filter the rendered tree at render-time. A platform leaf with 0 games is omitted. A container with 0 visible descendants is omitted.
- Recursive: bottom-up evaluation.

Default for fresh install: `false`. For upgrade: inherit existing operator pref.

### 3.5 Drag-reorder

Within-container leaf reorder + top-level container reorder. Cross-container drag deferred to v2.

`@thisbeyond/solid-dnd` SortableProvider scopes:
- Top-level: SortableProvider around the `<For>` over `root.children`.
- Per container: SortableProvider around the `<For>` over each container's leaf children.

`onDragEnd` updates the corresponding `view.root.children` array via `ViewsStore.reorderChildren` or `reorderTopLevel`.

### 3.6 Right-click context menus

**Container**: new minimal context menu.

```tsx
<MenuItem label="Hide from sidebar" onClick={() => viewsStore.setNodeHidden(node.id, true)} />
```

**Leaf (platform)**: existing `SystemContextMenu` continues to work. Its "Hide from sidebar" action calls `viewsStore.setNodeHidden(node.id, true)` instead of pushing to `hiddenSystems`.

### 3.7 Migration banner

Top-of-sidebar banner shown when:
- Active view is `flat-legacy` (the seeded legacy view from PR-β's migration path).
- Banner-dismissed flag in `views.json` is false.

Copy:

```
┌─────────────────────────────────────────────────────────────────┐
│ We've reorganized your system list.                             │
│ Systems are now grouped by form factor (Consoles, Handhelds,    │
│ Computers, Arcade). Your customized order is preserved as a     │
│ "Flat (Legacy)" view if you'd rather not switch yet.            │
│                                                                 │
│ [ Try Form Factor view ]    [ Stay on Flat (Legacy) ]           │
└─────────────────────────────────────────────────────────────────┘
```

Behavior:
- **Try Form Factor view**: sets `activeViewId = "default-formfactor"`; applies Option C (`reorderForFormFactor`) to preserve relative ordering of platforms within each new bucket; dismisses banner.
- **Stay on Flat (Legacy)**: dismisses banner; active view stays on `flat-legacy`. The operator can still switch views any time (future v2 picker).

Add `bannerDismissed: boolean` field to `ViewsConfig` to track dismissal across launches.

### 3.8 Settings panel reconciliation

`LibraryManagerPage.tsx` has system-visibility checkboxes that today write to `hiddenSystems`. In PR-γ those checkboxes drive `viewsStore.setNodeHidden(platform_node_id, ...)` instead. Container-level hide is right-click-only in v1; the Settings panel grows container support in a later pass.

### 3.9 Acceptance for PR-γ

- `cd frontend && npm run typecheck` clean.
- Manual: sidebar renders as a tree with twisties, count badges, click-to-filter behavior.
- Fresh-install layout: sidebar shows expanded `Platforms` tree with empty containers (auto-hide off by default).
- Upgrade layout: sidebar still shows the operator's flat list (Flat-Legacy view active), with the migration banner at the top.
- Clicking "Try Form Factor view" switches active view; banner disappears; tree shows operator's platforms ordered by Option C within each FormFactor bucket.
- Right-click container → "Hide from sidebar" hides cascading.
- Within-container leaf drag-reorder works; top-level container drag-reorder works; cross-container drag is intentionally not implemented (drop is rejected with no visual feedback — acceptable for v1).
- `cargo test --workspace` unaffected (frontend-only PR).

---

## 4. Sequencing & PRs

1. **PR-α — Registry tagging.** Adds `formFactor` + `manufacturer` fields to `SystemTheme`, audits all 39 systems. Trivial.
2. **PR-β — Views infrastructure + `SidebarView` fold.** Schema, Rust commands, ViewsStore, default + legacy view construction, migration seeding logic, `SidebarView` discriminant rewrite, library filter extension. Sidebar still renders flat (DFS-flattened active view).
3. **PR-γ — Tree UI + migration banner.** New `SidebarTreeNode` component, recursive count badges, cascade auto-hide-empty, within-container + top-level drag-reorder, right-click menus, migration banner, Settings checkbox reconciliation.

Per-PR scope discipline: same as the polish-plan PRs. If you spot a related cleanup, file it as a follow-up note in the PR description rather than folding it in.

---

## 5. Out of scope (v1)

- **View Editor UI** — operator can't add/rename/delete categories or change filter rules. Default view is hand-coded; "Flat (Legacy)" view is auto-built from migration. CRUD lands in v3.
- **Multiple shipped default views** (e.g., a Manufacturer-grouped view as an alternative default). v2.
- **View picker UI** — switching between active views via the sidebar is right-click-only in v1 (or via a debug Settings entry); proper picker UI lands in v2 when there's a second view to switch to.
- **Cross-container drag-reorder** — operator can't drag NES from Consoles to Handhelds. v2.
- **Per-category art (banner/logo/clear logo)** — schema slot reserved; no UI in v1.
- **Per-category accent override** — neutral in v1; user-pickable in v3.
- **Auto-projection views** (`Decade → Year → Game`, `Genre → Game`) — v4 (depends on per-game metadata quality).
- **Playlists + Smart Views** — bays stay as placeholders; surfaces come in a separate future PR (not blocked by this plan).
- **Theme-shipped advisory views** — supported by the schema's `kind` field but no themes ship views in v1.

---

## 6. Risks & open considerations

Flagged for awareness; mitigation strategy noted where applicable.

1. **`{ kind: "system" }` fold is broad** — ~15-20 call sites rewrite to `view-node`. TypeScript catches misses since the variant is removed entirely. Risk: a bad rewrite changes navigation behavior subtly under deep-link conditions (search-jump, history nav). Mitigation: PR-β acceptance includes manually clicking each navigation entry to verify routing.

2. **Recursive count badges** rely on a per-container memo over the library entries list. With ~39 platforms × ~100s of games each, naive `entries.filter(...)` per render is fine but worth memoizing properly. Risk: scroll jitter if memo regenerates on every entry-list mutation. Mitigation: memo per node, depend on `entries` reference + node identity.

3. **Drag-reorder in a tree is delicate** — solid-dnd's nested-sortable pattern works but the UX has edge cases (drop on collapsed container, drop near container header vs first child). v1 ships within-container + top-level only; cross-container is v2. Risk: operators expect cross-container drag immediately; if not present, the "Move to category…" right-click submenu (v3) becomes the workaround. Mitigation: clear v1 visual cue that cross-container drag isn't accepted (e.g., drop indicator turns red).

4. **First-run-no-data state** — fresh install with no scanned ROMs and `autoHideEmptySystems: false` shows Console / Handheld / Computer / Arcade containers all with "(0 games)". Visually emptier than today's flat list. Mitigation: probably fine, but eyeball once running. If too sparse, consider auto-collapsing zero-game containers as a UX tweak.

5. **Synthesized-leaf fallback** for deep-links to systems not in the active view — when active view is a custom user-built tree that excludes NES, but a search-jump or recent-games entry deep-links to NES, the runtime synthesizes a virtual leaf node at the root level. Exact visual treatment of synthesized leaves (root-level vs "Outside this view" virtual container) is a PR-β implementation decision. Acceptance: deep-links never error; system is always reachable.

6. **Auto-extend on new-system-added** — when we add a new system to `systemThemes` (next core integration), the existing default view's tree doesn't know about it. A reconciler step on `views.json` load slots the new platform into the appropriate FormFactor container (or "Other" if untagged). Risk in v3+: operator manually deleted a leaf; reconciler shouldn't auto-re-add. Need an "explicitly removed" marker, designed in v3.

7. **Theming when active node is a container** — `data-system` attribute today cascades the accent color. For a container with no accent (neutral), `data-system` would be cleared / omitted; CSS falls back to default OA accent. Verify the cascade handles this cleanly. Tailwind variables don't break when undefined, but the visual transition (system accent → neutral when entering a container, back to system accent on a leaf) should be smooth, not jarring.

8. **Settings checkbox reconciliation** — `LibraryManagerPage.tsx`'s system-visibility checkboxes today write to `hiddenSystems`. PR-γ rewires them to `viewsStore.setNodeHidden(platform_node_id, ...)`. Container-level hide in Settings is deferred — operators hide containers via right-click only in v1. Risk: Settings checkbox state needs to mirror the per-view hide state, not a global flat `hiddenSystems` set. Mitigation: Settings shows the *active view's* leaf hide states; switching views updates the checkboxes.

---

## 7. Per-PR sign-off checklist

For each PR:
- `cd frontend && npm run typecheck` clean.
- `cargo test --workspace` clean.
- Manual visual check at two presentation modes (Desktop + Theater) and one window mode (two-window) — confirms no CSS-cascade regressions.
- For PR-γ specifically: confirm migration banner appears for upgrade installs, doesn't appear for fresh-install layouts.
- Append a SESSION_LOG entry: Shipped / Almost / Next.
- ROADMAP / PARKING_LOT hygiene per `CLAUDE.md`.

---

## 8. Post-v1 roadmap

How this v1 spec sets up the future work — sanity check that the data model holds up.

**v2 — Multiple default views + view picker.**
- Ship `default-manufacturer` and `default-formfactor` as shipped defaults; operator picks active via a small `<select>` in the sidebar header or under Library Manager → Views.
- Cross-container drag-reorder lands here (more complex solid-dnd patterns).
- Right-click leaf "Move to category…" submenu — adds a leaf to a different container; underlying mutation is `view.root` tree update.

**v3 — User-editable views (View Editor).**
- New `<Dialog>` for creating / renaming / deleting custom views.
- Operator can author new containers with their own filter rules (using the `ContainerRule` DSL).
- Per-container accent picker + art slot UI.
- "Explicitly removed" marker on nodes so the auto-extend reconciler doesn't re-add deleted platforms.

**v4 — Auto-projection views.**
- New rule kind: `{ kind: "groupBy"; axes: ("decade" | "genre" | "publisher")[] }` for projection-style hierarchies.
- Runtime auto-generates container nodes from the matching axis values across the library.
- Needs per-game metadata (year, genre, publisher) to be reliably populated — depends on completing the metadata sync work.

**vN — Kiosk shell view consumer.**
- The kiosk shell (Phase 1 of `KIOSK_PLAN.md`) reads the same `views.json` and renders its BigBox-class wheel/grid from the active view. Theme-shipped advisory views appear alongside user-built persistent views per kiosk plan §3.3.
- Per-container art slots become first-class (theme-painted banners, clear logos at each tree level).
- Breadcrumb UI for deep hierarchies.

---

That's the executable spec. Next session (or another engineer) can start with PR-α cold and have everything they need.
