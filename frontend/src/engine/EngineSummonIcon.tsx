// Engine territory — the always-visible top-right corner icon themes
// mount to give the operator a visual summon affordance for the engine
// surface.
//
// Per docs/features/theming-substrate/SURFACES.md + plan D3: themes
// MUST reserve a top-right slot for this icon. In ARC 1 the corner is
// fixed; future relaxations (themes pick a corner) deferred. The icon
// itself is engine-owned — themes can't restyle it beyond mounting
// it. That keeps the affordance consistent across all themes so the
// operator's muscle memory doesn't change when they swap themes.
//
// Phase 1 ships the icon as a small button rendered inside whatever
// the theme provides as its top-right cluster. Today only Retroverse
// exists; RetroverseShell's header mounts this in its top-right area
// alongside the clock + quit + profile chip.

import type { Component } from "solid-js";
import { openEngineSurface } from "../platform/engineSurface";

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
