// Global "back" handler stack.
//
// Components (modals, menus, overlays, the per-group focus model)
// register a close / cancel function. The B button pops the most
// recently registered handler. The focus group's `onCancel` only fires
// when the stack is empty — so a tile context menu opened from the
// library grid closes on B without disturbing the underlying group.
//
// Stack order = mount order. Solid mounts children after parents and
// disposes children before parents, so an opened dialog naturally
// stacks above the underlying screen and pops first.

import { onCleanup } from "solid-js";

const stack: Array<() => void> = [];

/// Push a back handler imperatively. Returns a disposer that removes it.
/// Use when the handler's lifetime isn't tied to a component mount — e.g.
/// a transient "adjust mode" that the operator enters/exits with Confirm,
/// and that must intercept B BEFORE any outer close handler already on the
/// stack (popBack fires the most-recently-pushed handler first).
export function pushBackHandler(fn: () => void): () => void {
  stack.push(fn);
  return () => {
    const i = stack.lastIndexOf(fn);
    if (i >= 0) stack.splice(i, 1);
  };
}

/// Push a back handler for the lifetime of this component. Use
/// inside a Solid reactive scope; auto-pops on cleanup.
export function useBackHandler(fn: () => void): void {
  const dispose = pushBackHandler(fn);
  onCleanup(dispose);
}

/// Pop and invoke the most-recently registered handler. Returns true
/// if a handler fired; false if the stack was empty (caller may fall
/// back to per-group `onCancel`).
export function popBack(): boolean {
  const fn = stack[stack.length - 1];
  if (!fn) return false;
  fn();
  return true;
}

/// Does the stack currently hold any handlers? Useful for the hint bar
/// to label B as "Close menu" vs "Back to library."
export function hasBack(): boolean {
  return stack.length > 0;
}
