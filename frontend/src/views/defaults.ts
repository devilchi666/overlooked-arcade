// Builds the shipped default view ("Platforms" — form-factor grouped) from
// the current systemThemes registry. Bucketing is auto — every registered
// system slots into its `formFactor` tag's container, so adding a new
// system later just appears under the right bucket on next launch without
// touching this file. Per SIDEBAR_TIER_PLAN.md §2.1.

import { systemThemes, type FormFactorTag, type SystemId } from "../themes/registry";
import type { ContainerNode, View, ViewNode } from "./types";

export const DEFAULT_VIEW_ID = "default-formfactor";
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
