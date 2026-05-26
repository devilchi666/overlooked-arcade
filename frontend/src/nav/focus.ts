// Focus manager + group primitives.
//
// Model: focus groups are index-based. Each group owns:
//   - an itemCount accessor (Solid-style getter so it tracks)
//   - an orientation (vertical / horizontal / grid)
//   - columns (grid only)
//   - a focused-index accessor + setter (parent-owned state)
//   - callbacks for activate / cancel / secondary / tertiary
//
// Only one group is "active" at a time. Active = consumes NavEvents
// from the gamepad bus. Activation transfers via `activate(id)` or
// implicit "most-recently-mounted group" if no explicit activation
// has happened yet.
//
// Virtual lists work fine: focus is index-based, not DOM-based. The
// optional `bind(i, el)` registration lets the manager call
// `.focus()` + `.scrollIntoView()` on the element when it exists.
// For an index whose row isn't rendered, focus state still tracks
// correctly; once the row scrolls into view + binds, the manager
// catches up.

import { createSignal, onCleanup, onMount, type Accessor, type Setter } from "solid-js";
import { popBack } from "./back";

/// Either a Solid Setter (from createSignal) or a plain `next => void`
/// callback. Both work — Solid's Setter is structurally assignable to
/// the callback form, and the manager only needs to push values, never
/// read return values.
export type IndexSink = (next: number) => void;
import type { NavDirection, NavDirectionEvent, NavEvent } from "./types";
import { onNavEvent } from "./gamepad";

export type FocusOrientation = "vertical" | "horizontal" | "grid";

export type FocusGroupOptions = {
  /** Stable id. Used by `activate(id)` to transfer focus across groups. */
  id: string;
  /** How DPad/stick directions move within the group. */
  orientation: FocusOrientation;
  /** Live item count. Required — the group never assumes a static size. */
  itemCount: Accessor<number>;
  /** Required when orientation is `"grid"`. Live columns count. */
  columns?: Accessor<number>;
  /** Parent-owned focused-index accessor. Use `createSignal(0)` typically. */
  focusedIndex: Accessor<number>;
  setFocusedIndex: IndexSink;
  /** A button on a focused row. */
  onActivate?: (index: number) => void;
  /** B button anywhere in the group. */
  onCancel?: () => void;
  /** X button on a focused row. */
  onSecondary?: (index: number) => void;
  /** Y button on a focused row. */
  onTertiary?: (index: number) => void;
  /** Start button anywhere in the group. */
  onStart?: () => void;
  /** Neighbour group ids for shoulder-bumper transfer. `null` = nothing
   *  there; navigation ignored. */
  neighbours?: {
    left?: string;
    right?: string;
  };
  /** Pre-handler for direction events. Return true to consume the event
   *  (default movement skipped). Use this for orientation-specific
   *  behaviour like "left collapses a sidebar container" or "right
   *  expands a tree node before descending." */
  onDirection?: (direction: NavDirection, currentIndex: number) => boolean;
  /** Overrides for the shoulder bumpers. When defined, the handler runs
   *  instead of jumping to a neighbour group — useful for modals that
   *  want L1/R1 to cycle tabs rather than transfer focus. */
  onShoulderL?: () => void;
  onShoulderR?: () => void;
};

type FocusGroupHandle = {
  options: FocusGroupOptions;
  binds: Map<number, HTMLElement>;
};

type Manager = {
  groups: Map<string, FocusGroupHandle>;
  activeGroupId: Accessor<string | null>;
  setActiveGroupId: Setter<string | null>;
  /** Set the active group. No-op if `id` isn't registered. */
  activate: (id: string) => void;
};

function createManager(): Manager {
  const [activeGroupId, setActiveGroupId] = createSignal<string | null>(null);
  const groups = new Map<string, FocusGroupHandle>();
  function activate(id: string): void {
    if (!groups.has(id)) return;
    setActiveGroupId(id);
  }
  return { groups, activeGroupId, setActiveGroupId, activate };
}

const manager = createManager();

const [swapABSig, setSwapABSig] = createSignal(false);

/// When true, A and B button events swap before dispatch — Nintendo-
/// convention layout (B = confirm, A = back). Settings calls this when
/// the operator flips the swap toggle. Reactive accessor exposed for
/// the hint bar (renames glyph labels) + any consumer that wants to
/// adapt copy ("Press A" vs "Press B").
export function setSwapAB(on: boolean): void {
  setSwapABSig(on);
}
export const isSwapAB: Accessor<boolean> = swapABSig;

// Global event subscription — once, at module load. Routes every
// NavEvent to whichever group is active. No-op if no group is active.
onNavEvent((event) => {
  const id = manager.activeGroupId();
  if (id === null) return;
  const handle = manager.groups.get(id);
  if (!handle) return;
  routeEvent(handle, event);
});

function routeEvent(handle: FocusGroupHandle, event: NavEvent): void {
  if (event.kind === "button") {
    if (event.phase !== "down") return;
    const idx = handle.options.focusedIndex();
    // Nintendo-layout swap: rename A→B and B→A before semantic dispatch.
    const swap = swapABSig();
    const button = swap && (event.button === "a" || event.button === "b")
      ? (event.button === "a" ? "b" : "a")
      : event.button;
    switch (button) {
      case "a":
        handle.options.onActivate?.(idx);
        return;
      case "b":
        // Global back-stack consumes first; the active group's onCancel
        // is the fallback when no overlay / menu is open.
        if (popBack()) return;
        handle.options.onCancel?.();
        return;
      case "x":
        handle.options.onSecondary?.(idx);
        return;
      case "y":
        handle.options.onTertiary?.(idx);
        return;
      case "start":
        handle.options.onStart?.();
        return;
      case "l1": {
        if (handle.options.onShoulderL) {
          handle.options.onShoulderL();
          return;
        }
        const left = handle.options.neighbours?.left;
        if (left) manager.activate(left);
        return;
      }
      case "r1": {
        if (handle.options.onShoulderR) {
          handle.options.onShoulderR();
          return;
        }
        const right = handle.options.neighbours?.right;
        if (right) manager.activate(right);
        return;
      }
      default:
        return;
    }
  }
  // Direction events: react on down + repeat, ignore up.
  if (event.phase === "up") return;
  applyDirection(handle, event);
}

function applyDirection(handle: FocusGroupHandle, event: NavDirectionEvent): void {
  const o = handle.options;
  const count = o.itemCount();
  if (count <= 0) return;
  const cur = clamp(o.focusedIndex(), 0, count - 1);
  if (o.onDirection?.(event.direction, cur)) return;
  let next = cur;

  if (o.orientation === "vertical") {
    if (event.direction === "up") next = Math.max(0, cur - 1);
    else if (event.direction === "down") next = Math.min(count - 1, cur + 1);
    // left/right ignored — vertical groups can opt to transfer focus
    // via neighbours (handled by shoulder bumpers, not edge-of-list).
  } else if (o.orientation === "horizontal") {
    if (event.direction === "left") next = Math.max(0, cur - 1);
    else if (event.direction === "right") next = Math.min(count - 1, cur + 1);
  } else {
    // grid
    const cols = Math.max(1, o.columns?.() ?? 1);
    const col = cur % cols;
    const row = Math.floor(cur / cols);
    const lastRow = Math.floor((count - 1) / cols);
    if (event.direction === "left") next = col > 0 ? cur - 1 : cur;
    else if (event.direction === "right") next = col < cols - 1 && cur + 1 < count ? cur + 1 : cur;
    else if (event.direction === "up") next = row > 0 ? cur - cols : cur;
    else if (event.direction === "down") {
      if (row < lastRow) next = Math.min(cur + cols, count - 1);
    }
  }

  if (next !== cur) {
    o.setFocusedIndex(next);
    focusDomFor(handle, next);
  }
}

function focusDomFor(handle: FocusGroupHandle, index: number): void {
  const el = handle.binds.get(index);
  if (!el) return;
  // Move browser focus so screen readers + Tab continuity work too.
  el.focus({ preventScroll: true });
  el.scrollIntoView({ block: "nearest", inline: "nearest" });
}

/// Register a focus group. Call inside a Solid component / reactive
/// scope. Returns helpers the component uses to wire children. Auto-
/// unregisters on dispose. First registered group activates by default
/// (so a fresh app session has SOMETHING listening).
export type FocusGroupApi = {
  /** True when this group is currently consuming NavEvents. */
  isActive: Accessor<boolean>;
  /** Imperatively transfer activation to this group. */
  activate: () => void;
  /** Bind a DOM element to an index so the manager can `.focus()` it. */
  bind: (index: number, el: HTMLElement | null) => void;
};

export function useFocusGroup(options: FocusGroupOptions): FocusGroupApi {
  const handle: FocusGroupHandle = { options, binds: new Map() };

  onMount(() => {
    manager.groups.set(options.id, handle);
    if (manager.activeGroupId() === null) {
      manager.setActiveGroupId(options.id);
    }
  });
  onCleanup(() => {
    manager.groups.delete(options.id);
    if (manager.activeGroupId() === options.id) {
      // Demote — next interaction picks a new active group.
      const next = manager.groups.keys().next().value ?? null;
      manager.setActiveGroupId(next);
    }
  });

  return {
    isActive: () => manager.activeGroupId() === options.id,
    activate: () => manager.activate(options.id),
    bind: (index, el) => {
      if (el) handle.binds.set(index, el);
      else handle.binds.delete(index);
    },
  };
}

/// Imperatively change the active group from outside a useFocusGroup
/// scope. Useful for "mouse click on a tile inside group X" → focus
/// transfers to X.
export function activateFocusGroup(id: string): void {
  manager.activate(id);
}

/// Read the currently active group id. Reactive — components that
/// render based on which group is active (e.g. the hint bar) can
/// derive from this.
export const activeFocusGroupId: Accessor<string | null> = manager.activeGroupId;

function clamp(v: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, v));
}
