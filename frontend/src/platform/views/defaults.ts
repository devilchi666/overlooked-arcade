// Builds the shipped default views from the current systemThemes registry.
// Two are shipped in v2: the FormFactor view ("Platforms") and the
// Manufacturer view ("Manufacturers"). Both auto-bucket every registered
// system by its registry tag (formFactor / manufacturer) so a new system
// slots into the right bucket automatically on next launch — no touch to
// this file when integrating a new core. Per SIDEBAR_TIER_PLAN.md §2.1
// + §8 v2.

import {
  systemThemes,
  type FormFactorTag,
  type ManufacturerTag,
  type SystemId,
} from "@oa/platform/themes/registry";
import type { ContainerNode, View, ViewNode } from "./types";

export const DEFAULT_VIEW_ID = "default-formfactor";
export const MANUFACTURER_VIEW_ID = "default-manufacturer";
export const LEGACY_VIEW_ID = "flat-legacy";
export const ROOT_NODE_ID = "root";

const CONTAINER_ID_PREFIX = "container:";
const PLATFORM_ID_PREFIX = "platform:";

/// Stable encoding of a leaf node id. Mirrored by the resolver's
/// synthesized-leaf fallback and by App.tsx's viewForSystem helper —
/// keep all three in sync.
export function platformNodeIdFor(systemId: SystemId): string {
  return `${PLATFORM_ID_PREFIX}${systemId}`;
}

export function containerNodeIdFor(formFactor: FormFactorTag): string {
  return `${CONTAINER_ID_PREFIX}${formFactor}`;
}

/// Manufacturer-bucket container id encoding. Held distinct from the
/// formFactor variant so the two views' container ids never collide,
/// even though they share the "container:" prefix (the underlying
/// tag string differs — e.g. `container:console` vs `container:sony`).
export function manufacturerNodeIdFor(manufacturer: ManufacturerTag): string {
  return `${CONTAINER_ID_PREFIX}${manufacturer}`;
}

/// Returns true if `nodeId` was produced by `platformNodeIdFor` — used
/// by the resolver to fall back to a synthesized leaf when a deep-link
/// targets a system not present in the active view's tree.
export function parsePlatformNodeId(nodeId: string): SystemId | null {
  if (!nodeId.startsWith(PLATFORM_ID_PREFIX)) return null;
  return nodeId.slice(PLATFORM_ID_PREFIX.length) as SystemId;
}

const BUCKETS: { tag: FormFactorTag; label: string }[] = [
  { tag: "console", label: "Consoles" },
  { tag: "handheld", label: "Handhelds" },
  { tag: "computer", label: "Computers" },
  { tag: "arcade", label: "Arcade" },
  { tag: "other", label: "Other" },
];

/// Build the default Platforms view from the live registry. Leaves
/// within each bucket are sorted by `displayName` for predictability —
/// the registry's insertion order is just whatever PRs happened to land
/// first and isn't meaningfully ordered. Operators can re-order via
/// PR-γ's drag-handle later.
export function buildDefaultFormFactorView(): View {
  const allSystems = Object.values(systemThemes);
  const containers: ViewNode[] = BUCKETS.map(({ tag, label }) => {
    const members = allSystems
      .filter((s) => s.formFactor === tag)
      .sort((a, b) => a.displayName.localeCompare(b.displayName));
    const container: ContainerNode = {
      id: containerNodeIdFor(tag),
      label,
      rule: { kind: "formFactor", value: tag },
      accent: null,
      art: null,
      hidden: false,
      children: members.map((s): ViewNode => ({
        kind: "platform",
        id: platformNodeIdFor(s.id),
        systemId: s.id,
        hidden: false,
      })),
    };
    return { kind: "container", ...container };
  });

  const root: ContainerNode = {
    id: ROOT_NODE_ID,
    label: "Platforms",
    rule: null,
    accent: null,
    art: null,
    hidden: false,
    children: containers,
  };

  return {
    id: DEFAULT_VIEW_ID,
    name: "Platforms",
    kind: "user-builtin",
    /// Expand root + every non-empty bucket on first launch. "Other"
    /// stays collapsed since it's empty in v1 (auto-fallback for any
    /// future system that lands untagged).
    expandedNodes: defaultExpandedNodes(),
    root,
  };
}

export function defaultExpandedNodes(): string[] {
  return [
    ROOT_NODE_ID,
    containerNodeIdFor("console"),
    containerNodeIdFor("handheld"),
    containerNodeIdFor("computer"),
    containerNodeIdFor("arcade"),
  ];
}

// ── Manufacturer view ────────────────────────────────────────────────

/// Display order for the Manufacturer view's top-level containers.
/// Roughly chronological by hardware lineage (Nintendo + Sega at the
/// top since they're the deepest libraries; the long tail of single-
/// platform vendors sinks toward the bottom; `other` last as the
/// auto-fallback bucket).
const MANUFACTURER_BUCKETS: { tag: ManufacturerTag; label: string }[] = [
  { tag: "nintendo", label: "Nintendo" },
  { tag: "sega", label: "Sega" },
  { tag: "sony", label: "Sony" },
  { tag: "nec", label: "NEC" },
  { tag: "atari", label: "Atari" },
  { tag: "snk", label: "SNK" },
  { tag: "bandai", label: "Bandai" },
  { tag: "microsoft", label: "Microsoft" },
  { tag: "coleco", label: "Coleco" },
  { tag: "mattel", label: "Mattel" },
  { tag: "magnavox", label: "Magnavox" },
  { tag: "fairchild", label: "Fairchild" },
  { tag: "gce", label: "GCE" },
  { tag: "panasonic", label: "Panasonic" },
  { tag: "other", label: "Other" },
];

/// Build the shipped Manufacturer view from the live registry. Same
/// auto-bucket pattern as `buildDefaultFormFactorView` but keyed on
/// `manufacturer` instead of `formFactor`. PR-α tagged every system
/// with its manufacturer back in May 2026, so this is a pure-render
/// operation — no per-system additions needed.
export function buildDefaultManufacturerView(): View {
  const allSystems = Object.values(systemThemes);
  const containers: ViewNode[] = MANUFACTURER_BUCKETS.map(({ tag, label }) => {
    const members = allSystems
      .filter((s) => s.manufacturer === tag)
      .sort((a, b) => a.displayName.localeCompare(b.displayName));
    const container: ContainerNode = {
      id: manufacturerNodeIdFor(tag),
      label,
      rule: { kind: "manufacturer", value: tag },
      accent: null,
      art: null,
      hidden: false,
      children: members.map((s): ViewNode => ({
        kind: "platform",
        id: platformNodeIdFor(s.id),
        systemId: s.id,
        hidden: false,
      })),
    };
    return { kind: "container", ...container };
  });

  const root: ContainerNode = {
    id: ROOT_NODE_ID,
    label: "Manufacturers",
    rule: null,
    accent: null,
    art: null,
    hidden: false,
    children: containers,
  };

  return {
    id: MANUFACTURER_VIEW_ID,
    name: "Manufacturers",
    kind: "user-builtin",
    /// Expand the deep-library vendors by default — Nintendo / Sega /
    /// Sony cover the bulk of what most operators are looking for.
    /// The long-tail single-platform vendors stay collapsed so the
    /// tree's height isn't dominated by 10 one-item buckets.
    expandedNodes: defaultManufacturerExpandedNodes(),
    root,
  };
}

export function defaultManufacturerExpandedNodes(): string[] {
  return [
    ROOT_NODE_ID,
    manufacturerNodeIdFor("nintendo"),
    manufacturerNodeIdFor("sega"),
    manufacturerNodeIdFor("sony"),
    manufacturerNodeIdFor("atari"),
  ];
}
