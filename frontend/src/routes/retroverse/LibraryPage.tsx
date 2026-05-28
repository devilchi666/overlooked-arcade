// Retroverse-UI Phase B Slice 5/6 — LIBRARY tab.
//
// Slice 5 (this commit) ships a placeholder so the routing in
// RetroverseShell is observably correct end-to-end (operator toggles
// the flag → top-tab strip renders → clicking LIBRARY lands here vs
// the StubPages on other tabs).
//
// Slice 6 will fill in the real implementation:
//   - Left pane: system filter list (sourced from existing LeftSidebar
//     content or a fresh systems-filter component).
//   - Center pane: header card + filtered VirtualLibraryGrid using the
//     same library state signals App.tsx already owns.
//   - Right pane: <RightDetailPanel entry={focusedEntry()} ... />
//     always-visible — no more modal in this code path.
//
// See docs/PLANS/retroverse-ui-rollout.md Phase B + the operator-
// supplied mockup at docs/features/per-system-ui/assets/library-default-mockup.png.

import type { Component } from "solid-js";

const LibraryPage: Component = () => {
  return (
    <div class="flex h-full w-full items-center justify-center p-12">
      <div class="max-w-md rounded-xl border border-(--color-system-accent)/40 bg-(--color-system-accent)/[0.04] px-10 py-12 text-center">
        <p class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-system-accent)">
          Retroverse UI · Phase B Slice 5
        </p>
        <h1 class="mt-3 text-2xl font-semibold uppercase tracking-widest text-(--color-oa-ink)">
          LIBRARY
        </h1>
        <p class="mt-4 text-sm text-(--color-oa-ink-dim)">
          Routing reached. Slice 6 wires the real header card + tile
          grid + persistent right-side detail pane here.
        </p>
      </div>
    </div>
  );
};

export default LibraryPage;
