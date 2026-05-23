# View Editor Plan — v3 of the Sidebar Tier Work

**Status:** Design locked 2026-05-22. Q1–Q3, Q5–Q7, Q9, Q11 answered by operator; Q4, Q8, Q10, Q12, Q13 defaulted to recommendations (operational details — operator can push back during implementation if a default feels wrong). Ready to execute.

**Locked decisions (operator-confirmed):**
- **Q1 — Editor home:** Library Manager → Views tab.
- **Q2 — Commit model:** Live editing; every change persists immediately.
- **Q3 — UI layout:** Two-pane (tree on the left, properties panel on the right).
- **Q5 — New-view templates:** Blank + 3 copy templates (Platforms / Manufacturers / Flat-Legacy).
- **Q6 — Shipped views:** Editable with warning banner; deletion still blocked.
- **Q7 — Accent picker (v3.3):** Native `<input type="color">` (operator overrode the curated-palette recommendation; full RGB picker ships).
- **Q9 — v3.5 timing:** Ship right after v3.2 so delete-leaf actually means delete.
- **Q11 — Rule overlap:** Allowed; consistent with view-as-projection model.

**Defaulted decisions (operator can override during impl):**
- **Q4 — Rule editing UI:** Properties panel renders per-kind editor (5-option select for formFactor, 15-option for manufacturer, multi-select checkbox list with search for systemIds).
- **Q8 — Per-container art (v3.4):** Deferred to vN; needs storage + format design.
- **Q10 — Reconciler scope:** Hands-off for user-built views; only touches shipped defaults.
- **Q12 — Container ordering in editor:** Drag-only; no alphabetical-sort button.
- **Q13 — Auto-extend on shipped defaults:** Reconciler adds newly-registered systems to shipped buckets on launch, skipping `explicitlyRemoved`. Behavior change for shipped views; user customization (reorder, hide flags) preserved.

**Purpose:** Lets operators create, rename, delete, and edit custom views in the sidebar tree. Today (post-v2) we ship two view defaults (Platforms, Manufacturers) plus the migration-seeded Flat-Legacy view; operators can re-order, hide containers/leaves, drag leaves between containers, and switch active view via the picker — but they can't add a new container, rename "Consoles" to "TVs", or build a Decade view from scratch. v3 closes that gap.

**Companion docs:**
- `docs/SIDEBAR_TIER_PLAN.md` §8 (v3 spec, one paragraph) — the parent contract this plan implements.
- `docs/KIOSK_PLAN.md` §3.3 — future kiosk consumer reads the same views model.
- `docs/DECISIONS.md` — design choices land here as they're made.

**Pre-v3 state (current main):**
- Two-level view trees only (root → form-factor/manufacturer container → platform leaves).
- Three view kinds in schema: `user-builtin` (Platforms/Manufacturers/Flat-Legacy), `theme-shipped` (unused), `user-built` (unused — no UI to create one).
- Container rule DSL supports three rule kinds: `formFactor`, `manufacturer`, `systemIds`. v3 unblocks operator authoring against this DSL.
- β.2's `ensureShippedDefaults` reconciler appends any missing shipped default view on hydrate (idempotent). v3.5 extends this with an "explicitly removed" marker.

---

## 0. Locked design contract (proposed — needs operator sign-off)

Each block below carries a **Recommendation** (my judgment based on the existing surface) and an **Alternative** (what to pick if the recommendation is wrong for you). Open questions are flagged **OPEN:** — please answer before I code.

### 0.1 Where the editor lives

**Recommendation:** New "Views" tab in `LibraryManagerPage`. Sister to the existing "Library" and "Game media" tabs. Consistent with where the operator already manages library state (folders, region priority, hidden systems); discovery is via the existing menu bar entry `Library ▾ → Library Manager…`.

**Alternative:** Standalone modal Dialog launched from the v2.1 view-picker dropdown ("+ New view…" / "Edit view…" items). Lighter weight but introduces a new modal surface.

**OPEN Q1:** Library Manager tab, or modal dialog?

### 0.2 Editor commit model

**Recommendation:** Live editing — every change applies immediately and persists via the existing ViewsStore write-through. Matches how every other settings surface in the app behaves (system order drag, hidden flags, region priority — all live). No Save/Cancel buttons.

**Alternative:** Draft editing — operator's changes accumulate in a local working copy, commit on Save / discard on Cancel. Standard for "schema-like" editing (e.g. editing a database table). Implements an undo-on-cancel UX but doubles the state machine.

**OPEN Q2:** Live or draft?

### 0.3 Tree-editing UX (per-node)

**Recommendation:** Inline tree on the LEFT, properties panel on the RIGHT (when a node is selected). Click a node → properties panel populates. Right-click a node → context menu with `Add child container`, `Add child leaf`, `Rename`, `Delete`. Drag-reorder reuses γ.2's solid-dnd machinery within the editor pane.

Layout sketch (editor pane):
```
┌─ [+ New view…] [Rename] [Delete view] ─────────────────────┐
│  My Custom View ▾                                          │
├────────────────────────────┬───────────────────────────────┤
│  Tree                      │  Properties — selected node   │
│                            │                               │
│  ▾ My Custom View          │  Label:  [Consoles_______]    │
│    ▾ Consoles              │                               │
│      • NES                 │  Rule kind:                   │
│      • SNES                │    ○ Form factor              │
│      • Genesis             │    ● Manufacturer             │
│    ▸ Handhelds             │    ○ Explicit system list     │
│    [+ Add container]       │                               │
│    [+ Add leaf]            │  Value: [Nintendo ▾]          │
│                            │                               │
│                            │  Accent: [● color picker]     │
│                            │                               │
│                            │  [Delete this container]      │
└────────────────────────────┴───────────────────────────────┘
```

**Alternative:** All-inline editing (no separate properties panel) — click a container's rule pill to edit it in-place; click the label to rename. Tighter but harder to fit color picker + dropdown UI.

**OPEN Q3:** Two-pane (tree + properties), or all-inline?

### 0.4 Container rule editing UI

For the three rule kinds in `ContainerRule`:

- **formFactor**: `<select>` with 5 options (Consoles / Handhelds / Computers / Arcade / Other).
- **manufacturer**: `<select>` with 15 options (Nintendo / Sega / Sony / NEC / Atari / SNK / Bandai / Microsoft / Coleco / Mattel / Magnavox / Fairchild / GCE / Panasonic / Other).
- **systemIds**: multi-select checkbox list of all SystemIds. ~41 systems today.

**Recommendation:** All three rendered in the properties panel based on the selected rule kind. The systemIds checkbox list gets a search/filter input on top since 41 entries is busy.

**OPEN Q4:** Acceptable, or push back?

### 0.5 New-view template picker

When the operator clicks **+ New view…**:

**Recommendation:** Inline name prompt + template picker:
```
┌─ New view ─────────────────────┐
│  Name:    [My Custom View___]  │
│  Start from:                   │
│    ● Blank tree                │
│    ○ Copy of Platforms         │
│    ○ Copy of Manufacturers     │
│    ○ Copy of Flat (Legacy)     │
│  [Cancel]      [Create]        │
└────────────────────────────────┘
```

Copy templates clone the source view's tree under a new id + new name. Subsequent edits don't affect the source.

**OPEN Q5:** Templates as listed (blank + 3 copies), or just "Blank"?

### 0.6 Editing user-builtin views

**Recommendation:** Operator CAN edit shipped defaults (Platforms / Manufacturers / Flat-Legacy) — rename / reorder / re-rule / etc. — with an inline warning banner: *"Editing a shipped view. Your changes survive across launches, but a future app update may reset them if the default view's structure changes substantially."* Operator CANNOT delete shipped defaults (delete button hidden/disabled for `kind: "user-builtin"`).

**Alternative:** Shipped views are read-only — operator must Duplicate to edit. Cleaner contract but more friction (operator who just wants to rename "Consoles" → "TVs" in Platforms has to clone the whole view).

**OPEN Q6:** Editable with warning, or read-only with required clone?

### 0.7 Accent picker (v3.3)

**Recommendation:** Curated palette of 16 preset colors (matching the existing system accent palette spread across the hue wheel) + a "None" option to clear. No freeform color picker — keeps the visual language coherent with shipped per-system accents.

Sketch:
```
Accent:  [⊘] [●] [●] [●] [●] [●] [●] [●] [●] [●] [●] [●] [●] [●] [●] [●] [●]
         off
```

**Alternative:** Native `<input type="color">` (full RGB). Maximum flexibility but operators can pick colors that clash with the OA dark surface.

**OPEN Q7:** Curated 16-swatch palette, or native color picker?

### 0.8 Art slot UI (v3.4 — deferred)

**Recommendation:** Defer to vN. Per-container art (banner / logo / clear-logo) needs design for:
- Storage location (separate folder under appData? per-view JSON blob? base64-embedded?)
- File format constraints (PNG/JPG/SVG?)
- Sizing / aspect-ratio expectations
- How the sidebar tree renders art (background image on row? icon swap?)

These are non-trivial decisions and v3.3's accent picker covers the immediate "make my containers visually distinct" need.

**OPEN Q8:** Defer art slots, or include in v3?

### 0.9 "Explicitly removed" marker (v3.5)

When an operator deletes a platform leaf from a view (whether shipped or user-built), β.2's reconciler shouldn't re-add it on next launch. New per-view field `explicitlyRemoved: SystemId[]` records the operator's intent. The reconciler consults this list before auto-extending newly-registered systems into the view.

**Schema bump:** `ViewsConfig.schemaVersion` 1 → 2. Migration: when loading v1 config, transcribe to v2 with `explicitlyRemoved: []` on every view. Idempotent.

**Recommendation:** Ship in v3.5 alongside the delete-leaf UX in v3.2 (delete UX shipped without the marker is broken — leaves you delete come back on next reconciler run).

**OPEN Q9:** Couple v3.5 to v3.2 (delete leaf → mark removed in same PR), or ship the marker as a separate v3.5 step?

### 0.10 Reconciler semantics for user-built views

**Recommendation:** β.2's `ensureShippedDefaults` continues to only touch the SHIPPED defaults (`DEFAULT_VIEW_ID`, `MANUFACTURER_VIEW_ID`). User-built views are NEVER auto-extended — the operator owns their content fully. New systems registered after a user-built view was created appear in shipped defaults but stay out of user-built views unless the operator manually adds them.

**Alternative:** Reconciler walks every view and adds new systems based on a per-view auto-extend toggle.

**OPEN Q10:** Hands-off for user-built (recommended), or per-view toggle?

### 0.11 Rule application + multi-membership

A given system can appear under MULTIPLE containers in the same view if the operator authors overlapping rules (e.g. "Consoles" with formFactor=console AND "Nintendo Consoles" with systemIds=[nes,snes,n64,gamecube]). Both containers' count badges would include the same SystemId; the leaf would render twice in the tree.

**Recommendation:** Allow it. The view-as-projection model from SIDEBAR_TIER_PLAN.md §0 explicitly says "games never belong to a category folder" — overlapping membership is consistent with that. Operator who creates overlapping rules sees the consequence and can adjust.

**Alternative:** Refuse overlapping rules with a validation error. Defensive but adds friction.

**OPEN Q11:** Allow overlap, or refuse?

### 0.12 Per-view ordering of containers

**Recommendation:** Operator-driven via γ.2's drag-reorder. The editor's tree pane uses the same SortableProvider scopes. No "Sort alphabetically" button (operator can manually reorder).

**OPEN Q12:** Drag-only, or add an alphabetical-sort button?

---

## 1. PR-v3.1 — View metadata CRUD

Smallest unit that delivers user-visible value: operator can create / rename / delete user-built views via Library Manager → Views tab. Tree editing comes in v3.2.

### 1.1 LibraryManagerPage gets a Views tab

- New tab named "Views" between existing "Library" and "Game media" tabs.
- Tab body: list of all views in `viewsStore.config().views` with per-row actions.

Row UI:
```
┌────────────────────────────────────────────────────────────┐
│  Platforms (shipped)              [Rename] [Set active]    │
│  Manufacturers (shipped)          [Rename] [Set active]    │
│  Flat (Legacy) (shipped)          [Rename] [Set active]    │
│  My Decade View (custom)          [Rename] [Delete] [Set…] │
└────────────────────────────────────────────────────────────┘
[+ New view…]
```

### 1.2 Store mutations

ViewsStore gains:

```ts
createView(name: string, template: "blank" | "copy-formfactor" | "copy-manufacturer" | "copy-legacy"): string
  // Returns the new view's id. Generates a stable id (e.g. `user-${counter}-${slug}`).
  // template="blank" → root container with no children.
  // template="copy-..." → deep-clone the source view's tree, fresh node ids.

renameView(viewId: string, name: string): void

deleteView(viewId: string): void
  // Refuses if view.kind === "user-builtin". Falls back active view to
  // DEFAULT_VIEW_ID if the deleted view was active.
```

### 1.3 New-view dialog

Inline (within the Views tab) per §0.5 mockup. Validation:
- Name non-empty, trimmed.
- Name unique among existing views (case-insensitive compare).
- If template is `copy-*`, source view must exist.

### 1.4 Acceptance

- `npm run typecheck` clean.
- Operator can create a new user-built view, see it appear in the picker (v2.1) and the editor list.
- Operator can rename any view (including shipped); name persists across launches.
- Operator can delete only user-built views; delete buttons hidden/disabled for shipped.
- Deleting the active view falls back to FormFactor.

---

## 2. PR-v3.2 — Tree structure editing

The bulk of the editor — operator can add / remove / configure containers and leaves within a selected view.

### 2.1 Editor pane (two-pane layout)

Selecting a view in the Views tab swaps the tab body for the editor pane (§0.3 layout). "← Back to views list" returns to v3.1's list.

### 2.2 Container CRUD

ViewsStore gains:

```ts
addContainer(parentId: string, label: string, rule: ContainerRule | null): string
  // Returns the new container's id. Appends to parent's children.
  // Generated id: `container:user-${counter}`.

setContainerLabel(nodeId: string, label: string): void
setContainerRule(nodeId: string, rule: ContainerRule | null): void
removeNode(nodeId: string): void
  // Removes the node (container OR leaf) from its parent. Refuses
  // on root (id === "root").
```

### 2.3 Leaf CRUD

```ts
addPlatformLeaf(parentId: string, systemId: SystemId): string
  // Returns the new leaf's id. Uses platformNodeIdFor(systemId).
  // Refuses if the parent already contains a leaf for systemId
  // (no duplicate leaves within the same parent — overlap across
  // parents in different containers is fine per §0.11).
```

(Existing `setNodeHidden`, `reorderChildren`, `moveNode` continue to work on user-built views — same API.)

### 2.4 Properties panel

When a node is selected in the tree:
- **Root container:** Label only (rule is always null for root).
- **Inner container:** Label, rule kind selector, rule value editor (per §0.4), [Delete container].
- **Platform leaf:** Read-only systemId + displayName, [Remove from view].

### 2.5 Drag-reorder within editor

Reuses γ.2 + v2.2.1's solid-dnd setup. Cross-container drag works (already in main); the editor pane just renders the same SortableContainerNode / SortableLeafNode.

### 2.6 Acceptance

- `npm run typecheck` clean.
- Operator can build a view from blank → add a container "Nintendo" with rule `manufacturer=nintendo` → see Nintendo systems appear under it when viewing that view in the sidebar.
- Operator can rename containers, change their rules, delete them.
- Operator can add specific platform leaves via the systemIds rule kind.
- Drag-reorder within editor mirrors sidebar drag.

---

## 3. PR-v3.3 — Per-container accent picker (optional)

Adds the accent picker per §0.7. Container rows in the sidebar tree pick up the accent color (`data-system-accent` or similar — TBD by the CSS cascade investigation in plan §6 risk 7).

### 3.1 Scope

- Properties panel for inner containers gains an "Accent" row with the 16-swatch palette + ⊘ off.
- ViewsStore gains `setContainerAccent(nodeId, accent | null)`.
- SidebarTreeNode's ContainerRow reads `container.accent` and applies as inline CSS variable override on the row.

### 3.2 Acceptance

- `npm run typecheck` clean.
- Setting accent on a container changes its row color in the sidebar.
- Clearing accent (⊘) reverts to neutral.

---

## 4. PR-v3.4 — Per-container art slots (deferred)

Design TBD per §0.8. Parked on `docs/PARKING_LOT.md` until storage + format decisions land.

---

## 5. PR-v3.5 — Explicitly-removed marker

Schema bump v1 → v2 per §0.9 + §0.10.

### 5.1 Schema change

```ts
// types.ts
export const CURRENT_SCHEMA_VERSION = 2;  // was 1

export type View = {
  // ... existing fields ...
  explicitlyRemoved?: SystemId[];  // new in v2; absent in v1 configs
};
```

### 5.2 Migration

```rust
// apps/oa-shell/src/views.rs migrate_inplace
if config.schema_version < 2 {
    for view in &mut config.views {
        if view.explicitly_removed.is_none() {
            view.explicitly_removed = Some(Vec::new());
        }
    }
    config.schema_version = 2;
}
```

Idempotent. Existing v1 configs hydrate cleanly with empty marker lists on every view.

### 5.3 Reconciler

`ensureShippedDefaults` consults `view.explicitlyRemoved` before any auto-extend (which currently doesn't exist for shipped defaults — they're rebuilt from registry on every hydrate via `buildDefaultFormFactorView` / `buildDefaultManufacturerView` and don't merge user state). v3.5 changes this: shipped defaults get an auto-extend pass that adds newly-registered systems to the appropriate bucket, SKIPPING any SystemIds in `explicitlyRemoved`.

This is a behavior change for shipped defaults — currently the operator's reorder + per-node hide flags survive because we never touch the shipped view after initial seed. v3.5 needs to preserve those too: the auto-extend ONLY adds new systems, never re-orders existing ones, never un-hides hidden ones.

**OPEN Q13:** Acceptable behavior change, or keep shipped defaults frozen-after-seed?

### 5.4 Delete-leaf UX hook

When operator deletes a platform leaf in v3.2's editor, push the systemId onto the parent view's `explicitlyRemoved`. ViewsStore's `removeNode` checks if the node is a platform leaf and updates the marker list.

### 5.5 Acceptance

- `cargo test --workspace` clean (Rust migration tested).
- `npm run typecheck` clean.
- Operator deletes "NES" from Platforms → leaf disappears → app restart → leaf stays gone.
- New system registered (e.g. a 42nd system in a future PR) → reconciler appends to Platforms' Consoles bucket on next launch unless operator had explicitly removed it.

---

## 6. Sequencing & PRs

1. **PR-v3.1** — View metadata CRUD (~1-2 commits).
2. **PR-v3.2** — Tree structure editing (~3-4 commits: container CRUD / leaf CRUD / properties panel / drag-reorder integration).
3. **PR-v3.3** (optional) — Accent picker (~1 commit).
4. **PR-v3.5** (recommended) — Explicitly-removed marker + schema v2 migration (~2 commits: schema bump + reconciler).
5. **PR-v3.4** — Art slots, deferred to vN.

Per-PR scope discipline: same as the polish/tier-plan PRs. If you spot a related cleanup, file it as a follow-up note rather than folding in.

---

## 7. Out of scope (v3)

- **View import / export** — share a custom view as JSON. v4.
- **View templates / community presets** — download a curated view set. v4+.
- **Rule kind expansion** — new rule kinds beyond formFactor/manufacturer/systemIds (e.g. `decade`, `genre`, `publisher`). v4 (depends on per-game metadata).
- **Nested user-built containers** — operator creates `Consoles → Nintendo`. v3 supports the data shape but the editor UI in v3.2 is two-level; deeper nesting is v3+.
- **Per-game tags / smart-view rules** — the "Smart Views" sidebar placeholder. Separate plan.
- **Undo / redo within the editor** — operator's mistake is recoverable by manually un-doing (e.g. re-add a deleted container). Generic undo is a cross-cutting concern, separate PR.

---

## 8. Risks

1. **Schema bump risk** — v3.5's schema v1 → v2 migration is straightforward but every operator's `views.json` gets rewritten on first v3.5 launch. Bug in migrate_inplace could corrupt persisted state. Mitigation: cargo unit tests for migration; back up `views.json` → `views.json.v1.bak` on first v3.5 launch.

2. **Tree-editor UX complexity** — two-pane layout with tree + properties + dialogs + drag-reorder is a substantial component. Likely 600-800 LOC across the editor pane. Mitigation: phase strictly (v3.1 then v3.2); push polish to follow-ups.

3. **Reconciler behavior change** — v3.5 introduces auto-extend on shipped defaults (adding new systems to buckets after they're registered). Operators who customized Platforms' bucket contents will see new systems appear in their customized view on app update. Mitigation: explicitly-removed marker covers the "I don't want this" case; new systems land in the correct bucket per formFactor tag.

4. **User-built view persistence across upgrades** — if a future app version changes ViewsConfig schema incompatibly, user-built views could be lost. Mitigation: migration tests, schema is JSON (forward-compat by design), backup file on first launch of new schema.

5. **Picker overflow** — v2.1's view picker dropdown could get long with many user-built views. v3 doesn't change the picker; operators with 20+ custom views might want a search input. Defer.

6. **Cross-view consistency** — operator hides a system in Platforms → does it stay visible in Manufacturers? Today yes (per-view hidden flag). v3 surfaces this clearly via the editor: each view has independent hide state. Mitigation: documentation + the per-view scope of the LibraryManagerPage's Hidden containers list (v2.3) is already correct.

7. **Active view deletion** — operator deletes the active view → fallback to FormFactor. If FormFactor was also somehow gone (shouldn't happen since reconciler ensures it), fallback to first view in list, else error toast. Mitigation: defensive guard in `setActiveView` and `deleteView`.

---

## 9. Per-PR sign-off checklist

For each PR:
- `cd frontend && npm run typecheck` clean.
- `cargo test --workspace` clean.
- Manual: editor surfaces work end-to-end on the active view.
- For PR-v3.5: migration verified by deleting `views.json`, manually crafting a v1 file, launching, confirming it upgrades to v2 with `explicitlyRemoved: []` on every view.
- Append SESSION_LOG entry: Shipped / Almost / Next.
- VIEW_EDITOR_PLAN.md updated if scope changes.

---

## 10. Post-v3 roadmap

How v3 sets up future work:

**v4 — Auto-projection views.** New rule kind `{ kind: "groupBy"; axes: ("decade" | "genre" | "publisher")[] }` for projection-style hierarchies. Runtime auto-generates container nodes from matching axis values across the library. Needs per-game metadata reliability.

**v5 — Per-container art slots.** v3.4 deferred. Storage design + theme integration.

**v6 — View import / export + community presets.** Share custom views as JSON. Curated preset catalog.

**vN — Kiosk shell view consumer.** KIOSK_PLAN.md §3.3. Kiosk shell reads the same `views.json`; user-built views appear as advisory tabs / wheels alongside theme-shipped views.

---

That's the proposed spec. Operator: please answer the OPEN Q1-Q13 callouts in §0 (and §5.3's Q13) before I start coding PR-v3.1.
