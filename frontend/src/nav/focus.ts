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

import { createEffect, createSignal, onCleanup, onMount, type Accessor, type Setter } from "solid-js";
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
  /** Neighbour group ids per direction. Drives DPad inter-region
   *  transfer: pressing DPad <direction> activates `neighbours[direction]`
   *  if declared. Left-stick (and any other "stick" source) walks
   *  within the group — it never spills to a neighbour. Undeclared
   *  directions fall back to within-group walking even on DPad, so
   *  the operator on a controller without a stick can still navigate
   *  vertically within a sidebar by pressing DPad UP/DOWN. */
  neighbours?: {
    left?: string;
    right?: string;
    up?: string;
    down?: string;
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
  /** When false, the group registers without auto-claiming the active
   *  slot on mount (even if `activeGroupId` is null). Use for sibling
   *  region groups where one specific group should be the initial
   *  landing surface — the others register with `autoClaim: false` so
   *  registration order doesn't pick the active group arbitrarily.
   *  Defaults true. */
  autoClaim?: boolean;
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
  /** Called from useFocusGroup's onCleanup when the active group is
   *  unmounting. Picks the most-recently-active still-registered
   *  successor; falls back to the first registered group otherwise. */
  demote: (unregisteringId: string) => void;
};

/// Bumps every time a group is registered or unregistered. Reactive —
/// consumers that depend on "is group X registered yet?" can track this
/// and re-evaluate their state. Used by LibraryPage's delegating effect
/// so it re-fires when "library-grid" mounts later (empty library →
/// imported games OR list-view → capsule-view).
const [groupsVersionSig, setGroupsVersionSig] = createSignal(0);
export const groupsVersion: Accessor<number> = groupsVersionSig;

function createManager(): Manager {
  const [activeGroupId, setActiveGroupId] = createSignal<string | null>(null);
  const groups = new Map<string, FocusGroupHandle>();
  // Most-recently-active history. When the active group unmounts, the
  // demote logic walks this stack to find the youngest still-registered
  // group instead of falling back to arbitrary Map insertion order
  // (which used to teleport focus to whichever group happened to
  // register first this session).
  const activationHistory: string[] = [];
  function activate(id: string): void {
    if (!groups.has(id)) return;
    const current = activeGroupId();
    if (current !== null && current !== id) {
      // Push the previous active onto the history so we can fall back
      // to it later. Dedupe consecutive entries.
      if (activationHistory[activationHistory.length - 1] !== current) {
        activationHistory.push(current);
        // Cap to prevent unbounded growth on long sessions — 32 is
        // generous (operators rarely have nav chains > 4 deep).
        if (activationHistory.length > 32) activationHistory.shift();
      }
    }
    setActiveGroupId(id);
  }
  function demote(unregisteringId: string): void {
    // Walk history newest-first; pick the first still-registered id.
    for (let i = activationHistory.length - 1; i >= 0; i--) {
      const candidate = activationHistory[i];
      if (candidate !== unregisteringId && groups.has(candidate)) {
        activationHistory.length = i; // trim consumed entries
        setActiveGroupId(candidate);
        return;
      }
    }
    // Nothing in history. Pick the MOST recently inserted registered
    // group — that's typically the youngest mount and the one the
    // operator most plausibly wants. Map iteration preserves insertion
    // order, so the last key produced by the iterator is the newest.
    // Setting null is a valid fallback if no groups are registered;
    // the next `useFocusGroup.onMount` will then auto-claim because
    // its "active is stale" check sees no active group.
    let newest: string | null = null;
    for (const key of groups.keys()) newest = key;
    setActiveGroupId(newest);
  }
  return { groups, activeGroupId, setActiveGroupId, activate, demote };
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

// Diagnostic flag — flip via window.__oaFocusDebug = false in DevTools
// to silence. Default ON while we hunt the DPad regression so the
// operator's session log captures every event-routing decision.
(globalThis as { __oaFocusDebug?: boolean }).__oaFocusDebug ??= true;
const FOCUS_DEBUG = () =>
  (globalThis as { __oaFocusDebug?: boolean }).__oaFocusDebug === true;

function focusLog(...args: unknown[]): void {
  if (FOCUS_DEBUG()) console.log("[oa-focus]", ...args);
}

// Global event subscription — once, at module load. Routes every
// NavEvent to whichever group is active. No-op if no group is active.
onNavEvent((event) => {
  const id = manager.activeGroupId();
  if (id === null) {
    focusLog("event dropped — no active group", event);
    return;
  }
  const handle = manager.groups.get(id);
  if (!handle) {
    focusLog("event dropped — active group not in map", id, event);
    return;
  }
  focusLog("event", { id, event });
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
      case "l1":
        // Shoulder bumpers are EXPLICIT-OPT-IN now. The previous
        // neighbours-based fallback collided with the new DPad =
        // region-transfer model: DPad LEFT/RIGHT now does what L1/R1
        // used to do for legacy sidebar↔grid jumps. L1/R1 is reserved
        // for top-bar tab cycling (RetroverseShell intercepts
        // globally) and for menus that want shoulder-cycle behaviour
        // (MenuBar.cycleMenu, GameInfoModal.cycleTab — both wired via
        // onShoulderL/R explicitly).
        handle.options.onShoulderL?.();
        return;
      case "r1":
        handle.options.onShoulderR?.();
        return;
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
  const cur = count > 0 ? clamp(o.focusedIndex(), 0, count - 1) : 0;

  // Custom direction handler runs first (used by sidebar tree
  // expand/collapse, rewind scrubber, etc.). Return true to consume.
  if (o.onDirection?.(event.direction, cur)) {
    focusLog("direction consumed by onDirection", {
      id: o.id,
      direction: event.direction,
    });
    return;
  }

  // DPad = inter-region transfer. Press DPad <dir> and activate the
  // neighbour group declared for that direction. If no neighbour is
  // declared on that direction the DPad press falls through to within-
  // group walking — keeps a stickless controller usable: DPad UP/DOWN
  // in a vertical sidebar walks rows even though no `up`/`down`
  // neighbour is declared.
  if (event.source === "dpad") {
    const neighbour = o.neighbours?.[event.direction];
    if (neighbour) {
      focusLog("dpad transfer", {
        from: o.id,
        direction: event.direction,
        to: neighbour,
        registered: manager.groups.has(neighbour),
      });
      manager.activate(neighbour);
      return;
    }
    focusLog("dpad fallback to walk — no neighbour", {
      id: o.id,
      direction: event.direction,
      neighbours: o.neighbours,
    });
    // No neighbour in that direction → fall through to walk.
  }

  // Stick (any source other than DPad) walks within the group only —
  // never spills. The grid's flat-list left/right behaviour stays so
  // stick LEFT at column 0 of row N still wraps to column LAST of row
  // N-1; only DPad LEFT at the absolute first tile triggers transfer
  // (via the neighbour activation above).
  if (count <= 0) return;
  let next = cur;

  if (o.orientation === "vertical") {
    if (event.direction === "up") next = Math.max(0, cur - 1);
    else if (event.direction === "down") next = Math.min(count - 1, cur + 1);
    // left/right have no movement in a vertical group — bounded no-op.
  } else if (o.orientation === "horizontal") {
    if (event.direction === "left") next = Math.max(0, cur - 1);
    else if (event.direction === "right") next = Math.min(count - 1, cur + 1);
  } else {
    // grid — flat linear list visually wrapped into columns. Left/right
    // walk the list linearly (left at column 0 of row N lands on
    // column LAST of row N-1); up/down jump by `cols`. Matches Steam
    // Big Picture / Xbox dashboard semantics.
    const cols = Math.max(1, o.columns?.() ?? 1);
    const row = Math.floor(cur / cols);
    const lastRow = Math.floor((count - 1) / cols);
    if (event.direction === "left") next = Math.max(0, cur - 1);
    else if (event.direction === "right") next = Math.min(count - 1, cur + 1);
    else if (event.direction === "up") next = row > 0 ? cur - cols : cur;
    else if (event.direction === "down") {
      if (row < lastRow) next = Math.min(cur + cols, count - 1);
    }
  }

  if (next !== cur) {
    focusLog("walk", {
      id: o.id,
      direction: event.direction,
      from: cur,
      to: next,
    });
    o.setFocusedIndex(next);
    focusDomFor(handle, next);
  } else {
    focusLog("walk no-op", {
      id: o.id,
      direction: event.direction,
      cur,
      count,
    });
  }
}

function focusDomFor(handle: FocusGroupHandle, index: number): void {
  const el = handle.binds.get(index);
  if (!el) return;
  // Move browser focus so screen readers + Tab continuity work too.
  // preventScroll:true so we don't trigger a browser scroll that fights
  // a virtualizer's scrollToIndex (the grid drives its own scroll via
  // createEffect on focusedIndex). For non-virtualized lists, the
  // consuming surface usually has `overflow-y:auto` on the row's
  // ancestor — focus + the row's css transitions cover the rest. The
  // previous unconditional `scrollIntoView` fought the virtualizer and
  // is dropped.
  el.focus({ preventScroll: true });
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
    setGroupsVersionSig((v) => v + 1);
    if (options.autoClaim !== false) {
      const currentActive = manager.activeGroupId();
      // Auto-claim when either (a) nothing is active, OR (b) the
      // current active id is stale — set to a group that's no longer
      // registered. Stale ids accumulate when a page unmounts and the
      // demote-history fallback picks a sibling that's also currently
      // unmounting; the active id stays pointing at it even though
      // the handle was deleted. Without this check, the next page's
      // sidebar never wins activeGroupId on its `useFocusGroup`
      // onMount (the `=== null` check failed because of the stale id),
      // and the operator lands on a page where no group consumes
      // their input.
      const currentIsRegistered =
        currentActive !== null && manager.groups.has(currentActive);
      if (!currentIsRegistered) {
        manager.setActiveGroupId(options.id);
      }
    }
  });
  onCleanup(() => {
    const wasActive = manager.activeGroupId() === options.id;
    manager.groups.delete(options.id);
    setGroupsVersionSig((v) => v + 1);
    if (wasActive) {
      // Demote to the most-recently-active still-registered group.
      manager.demote(options.id);
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

/// Snapshot the current active focus group id and return a function
/// that restores it. Used by menus / modals to remember where the
/// operator was BEFORE the overlay activated its own group, so
/// closing the overlay returns focus to the original surface (works
/// across Retroverse pages, not just LIBRARY).
///
/// Usage pattern (inside a menu / modal component):
///   const restore = captureFocusReturn();
///   onMount(() => focusGroup.activate());
///   onCleanup(() => restore());
export function captureFocusReturn(): () => void {
  const saved = manager.activeGroupId();
  return () => {
    if (saved !== null && manager.groups.has(saved)) {
      manager.setActiveGroupId(saved);
    }
  };
}

/// Imperatively change the active group from outside a useFocusGroup
/// scope. Useful for "mouse click on a tile inside group X" → focus
/// transfers to X.
export function activateFocusGroup(id: string): void {
  manager.activate(id);
}

// --- DOM-query helper -------------------------------------------------
//
// Convenience wrapper around useFocusGroup for surfaces where the set of
// focusable rows is dynamic (Show/For branches that mount + unmount
// rows, disabled-attr flips from background work) and explicit
// bind(i, el) plumbing would be awkward. Discovers buttons via a CSS
// selector inside a container ref, watches the container with a
// MutationObserver, and tracks the focused button by IDENTITY so a
// later insertion before the focused row doesn't shift the visual ring
// to a different button.

export type DomQueryFocusGroupOptions = {
  id: string;
  /// Returns the container element to query inside. Called after mount;
  /// safe to return undefined while the ref hasn't been bound yet.
  containerRef: () => HTMLElement | undefined;
  /// CSS selector for focusable rows. Defaults to all buttons inside the
  /// container. The matched element type is HTMLElement (not just
  /// buttons) so callers can use data-attribute selectors for mixed
  /// surfaces (e.g. read-only widget rows + action buttons). Elements
  /// with a truthy `.disabled` (i.e. real disabled buttons) are skipped
  /// automatically.
  selector?: string;
  orientation?: FocusOrientation;
  /// Fires when A is pressed. `el` is the live DOM node for the focused
  /// row — callers usually call `el.click()` to keep the mouse + gamepad
  /// paths identical.
  onActivate?: (index: number, el: HTMLElement) => void;
  onCancel?: () => void;
  onShoulderL?: () => void;
  onShoulderR?: () => void;
  neighbours?: { left?: string; right?: string };
  /// Pre-handler for direction events. Return true to consume the event
  /// (default movement skipped). Mirrors useFocusGroup's onDirection —
  /// useful for Retroverse pages where DPad LEFT/RIGHT should transfer
  /// to a neighbour region group instead of being ignored (vertical
  /// orientation) or moving (horizontal).
  onDirection?: (direction: NavDirection, currentIndex: number) => boolean;
  /// When false the group registers but does not auto-activate on mount.
  /// Useful when multiple sibling groups co-exist on one page and only
  /// one should be the landing surface; the others activate via DPad /
  /// onDirection transfer. Defaults `true`.
  autoActivate?: boolean;
};

export type DomQueryFocusGroupApi = FocusGroupApi & {
  focusedIndex: Accessor<number>;
  setFocusedIndex: (next: number) => void;
};

export function useDomQueryFocusGroup(opts: DomQueryFocusGroupOptions): DomQueryFocusGroupApi {
  const selector = opts.selector ?? "button";
  const [focusedIndex, setFocusedIndex] = createSignal(0);
  const [itemCount, setItemCount] = createSignal(0);
  // Bumped on every MutationObserver-driven rebind so the data-oa-focus
  // mirror effect re-paints onto whichever rows are now mounted.
  const [domRev, setDomRev] = createSignal(0);
  // Identity-tracked focused row. Captured by the mirror effect after
  // each focusedIndex change; consulted on rebind so a row inserted
  // before the focused one doesn't visually shift the ring.
  let lastFocusedEl: HTMLElement | null = null;
  let observer: MutationObserver | null = null;

  const queryItems = (): HTMLElement[] => {
    const root = opts.containerRef();
    if (!root) return [];
    return Array.from(root.querySelectorAll<HTMLElement>(selector)).filter(
      (el) => !(el as Partial<HTMLButtonElement>).disabled,
    );
  };

  const group = useFocusGroup({
    id: opts.id,
    orientation: opts.orientation ?? "vertical",
    itemCount,
    focusedIndex,
    setFocusedIndex,
    onActivate: (i) => {
      const el = queryItems()[i];
      if (el) opts.onActivate?.(i, el);
    },
    onCancel: opts.onCancel,
    onShoulderL: opts.onShoulderL,
    onShoulderR: opts.onShoulderR,
    neighbours: opts.neighbours,
    onDirection: opts.onDirection,
    // Forward autoActivate to the inner useFocusGroup's autoClaim so
    // sibling region groups with autoActivate:false don't accidentally
    // win the "first registered claims active" race during mount.
    autoClaim: opts.autoActivate !== false,
  });

  const rebind = (): void => {
    const root = opts.containerRef();
    if (!root || !root.isConnected) return;
    const items = queryItems();
    // Identity tracking — if the previously-focused element is still
    // present, keep focus on it (its index may have shifted because an
    // item was inserted before or after it).
    if (lastFocusedEl) {
      const newIdx = items.indexOf(lastFocusedEl);
      if (newIdx >= 0 && newIdx !== focusedIndex()) {
        setFocusedIndex(newIdx);
      }
    }
    setItemCount(items.length);
    items.forEach((el, i) => group.bind(i, el));
    setDomRev((r) => r + 1);
  };

  onMount(() => {
    queueMicrotask(() => {
      const root = opts.containerRef();
      if (!root || !root.isConnected) return;
      rebind();
      // autoActivate: omit / true → claim active on mount. Pass false to
      // register the group without stealing focus (e.g. sibling region
      // groups where only one is the landing surface).
      if (opts.autoActivate !== false) {
        group.activate();
      }
      observer = new MutationObserver(() => rebind());
      observer.observe(root, {
        childList: true,
        subtree: true,
        attributes: true,
        attributeFilter: ["disabled"],
      });
    });
  });
  onCleanup(() => {
    observer?.disconnect();
    observer = null;
    lastFocusedEl = null;
  });

  // Mirror focusedIndex → data-oa-focus attributes. Re-runs on cursor
  // moves and on domRev bumps (content changed mid-mount). Captures the
  // current focused element into lastFocusedEl so the next rebind can
  // track it by identity.
  createEffect(() => {
    const idx = focusedIndex();
    const active = group.isActive();
    void domRev();
    queueMicrotask(() => {
      const items = queryItems();
      const targetIdx = items.length === 0 ? -1 : Math.min(Math.max(0, idx), items.length - 1);
      lastFocusedEl = targetIdx >= 0 ? items[targetIdx] : null;
      items.forEach((el, i) => {
        if (i === targetIdx) {
          el.setAttribute("data-oa-focus", "true");
          el.setAttribute("data-oa-focus-active", active ? "true" : "false");
        } else {
          el.removeAttribute("data-oa-focus");
          el.removeAttribute("data-oa-focus-active");
        }
      });
    });
  });

  return {
    isActive: group.isActive,
    activate: group.activate,
    bind: group.bind,
    focusedIndex,
    setFocusedIndex,
  };
}

/// Read the currently active group id. Reactive — components that
/// render based on which group is active (e.g. the hint bar) can
/// derive from this.
export const activeFocusGroupId: Accessor<string | null> = manager.activeGroupId;

function clamp(v: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, v));
}
