// TypeScript mirror of the Rust ViewsConfig schema in apps/oa-shell/src/views.rs.
//
// Source of truth: SIDEBAR_TIER_PLAN.md §0 + §2.1. Any field name change
// here MUST land in lockstep with the Rust struct — serde's
// `rename_all = "camelCase"` makes the JSON shape symmetric so a mismatch
// would silently drop fields on hydrate.
//
// ViewNode is internally-tagged on the Rust side (`#[serde(tag = "kind")]`)
// — the discriminant lives at the same level as the struct fields, not
// wrapped in a payload object. So a container serializes as
// `{ "kind": "container", "id": ..., "label": ..., ... }`, not
// `{ "kind": "container", "value": { "id": ..., ... } }`.

import type { FormFactorTag, ManufacturerTag, SystemId } from "@oa/platform/themes/registry";

export const CURRENT_SCHEMA_VERSION = 2;

export type ViewsConfig = {
  schemaVersion: number;
  activeViewId: string;
  views: View[];
  /// PR-γ migration-banner dismissal flag. Absent in older saves —
  /// hydrate logic treats undefined as false.
  bannerDismissed?: boolean;
};

export type View = {
  id: string;
  name: string;
  kind: ViewKind;
  /// Node ids the operator has expanded in the sidebar tree. Held
  /// separately from the inline `hidden` flag so a single expand-toggle
  /// produces a tiny diff rather than rewriting the whole tree.
  expandedNodes: string[];
  root: ContainerNode;
  /// SystemIds the operator deleted from this view via the v3.2 editor.
  /// v3.5.3's reconciler consults this list when auto-extending shipped
  /// views with newly-registered systems — operator-deleted systems
  /// stay deleted across launches instead of coming back. Optional for
  /// forward/back compat (v1 configs lack the field).
  explicitlyRemoved?: SystemId[];
};

/// `user-builtin` = shipped default (Platforms form-factor view) or
/// migration-seeded Flat-Legacy view. Operator can hide / reorder but
/// can't delete it.
///
/// `theme-shipped` = advisory view tied to the active theme. Reserved
/// for kiosk theme work; no themes ship views in v1.
///
/// `user-built` = custom view created via the v3 View Editor.
export type ViewKind = "user-builtin" | "theme-shipped" | "user-built";

export type ViewNode = ContainerNode & { kind: "container" }
  | PlatformNode & { kind: "platform" };

export type ContainerNode = {
  id: string;
  label: string;
  /// `null` on the root container — runtime treats it as "match
  /// everything." Leaf-level containers carry a rule that drives the
  /// runtime filter when the operator clicks the container row.
  rule: ContainerRule | null;
  /// Per-container accent override. `null` in v1 (neutral); v3's
  /// category-editor UI populates this.
  accent: string | null;
  /// Reserved slot for per-container art (banner / logo / clear-logo).
  /// Held as a free-form unknown so the schema doesn't bump when the
  /// art shape lands in vN.
  art: unknown | null;
  hidden: boolean;
  children: ViewNode[];
};

export type PlatformNode = {
  id: string;
  systemId: SystemId;
  hidden: boolean;
};

export type ContainerRule =
  | { kind: "formFactor"; value: FormFactorTag }
  | { kind: "manufacturer"; value: ManufacturerTag }
  | { kind: "systemIds"; values: SystemId[] }
  /// Unified Navigation Tree Slice 1 — a container whose membership is an
  /// explicit set of games rather than a set of systems. `collectionId`
  /// keys into the `customCollections` store's `members` map; the resolver
  /// returns `{ kind: "games", romIds }` for such a node so the library
  /// filter slices by rom id instead of by system. This is what makes a
  /// collection a first-class navigation node (NT1) rather than a separate
  /// tab.
  | { kind: "collection"; collectionId: string }
  /// Reserved (D59) — a `filter` node resolves to a predicate over games
  /// (Favorites / Recently Played / Multi-Player / dump-quality). Declared
  /// now so the schema + Rust mirror don't bump when Slice 2 wires it; the
  /// resolver + library filter treat it as inert (empty membership) until
  /// then. `spec` is a free-form record so the predicate AST can settle in
  /// Slice 2 without a schema migration.
  | { kind: "filter"; spec: FilterRuleSpec };

/// Reserved free-form predicate payload for `filter`-kind container rules.
/// Intentionally untyped in Slice 1 (the shape lands in Slice 2 alongside
/// the smart-list predicate evaluator). Held as a record so it round-trips
/// through `get_views`/`set_views` as opaque JSON.
export type FilterRuleSpec = Record<string, unknown>;
