// The always-visible top-right corner icon themes mount to give the
// operator a visual summon affordance for the engine surface.
//
// Engine-OWNED by convention (its look stays consistent across themes so
// the operator's muscle memory doesn't change on a theme swap), but it
// physically lives in platform/components/ because THEMES must mount it
// (plan D3) and a theme can't import engine/ (theme ↛ engine boundary). It
// depends only on platform/engineSurface, so per DECISIONS D12 (a leaf
// consumed by the lowest layer belongs to that layer) platform is its home.
// Relocated here from engine/ in Theming Substrate ARC 1 Phase 3 S2 so the
// new Wheel theme + the Retroverse wrapper both mount it without crossing
// the boundary.
//
// Per docs/features/theming-substrate/SURFACES.md + plan D3: themes MUST
// reserve a top-right slot for this icon. In ARC 1 the corner is fixed;
// future relaxations (themes pick a corner) deferred. The icon is rendered
// inside whatever top-right cluster the theme provides.

import type { Component } from "solid-js";
import { openEngineSurface } from "@oa/platform/engineSurface";

const EngineSummonIcon: Component = () => {
  return (
    <button
      type="button"
      onClick={(e) => {
        e.currentTarget.blur();
        openEngineSurface();
      }}
      class="grid h-9 w-9 shrink-0 place-items-center rounded-md border border-white/10 bg-white/[0.04] text-sm text-(--color-oa-ink-dim) transition hover:border-(--color-oa-ink-dim)/50 hover:bg-white/[0.08] hover:text-(--color-oa-ink) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
      title="Open OA Settings (F12 · Select+Start)"
      aria-label="Open OA Settings"
    >
      ⚙
    </button>
  );
};

export default EngineSummonIcon;
