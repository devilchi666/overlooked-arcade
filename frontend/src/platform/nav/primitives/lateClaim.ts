// Late-claim helper for the nav primitives.
//
// A whole-shell primitive (a theme's root list/grid/carousel) often mounts AFTER
// the async active-theme seed resolves, so `useFocusGroup`'s on-mount auto-claim
// can miss — a stray earlier group may still hold the active input slot, or the
// group mounts before its items load. The result is a surface you can't move
// with the controller/keyboard (it's not the active group), where the only
// interaction is a mouse click (which then launches).
//
// This claims the active slot once the group's items first appear (unless the
// theme opted out via `autoActivate === false`). Generalizes the manual
// force-claim CoverFlow hand-rolled in S2 — every list-like primitive needs it,
// not just the carousel.

import { createEffect } from "solid-js";

export function useLateClaim(
  group: { activate: () => void },
  count: () => number,
  autoActivate: boolean | undefined,
): void {
  let claimed = false;
  createEffect(() => {
    if (autoActivate === false) return;
    if (!claimed && count() > 0) {
      claimed = true;
      group.activate();
    }
  });
}
