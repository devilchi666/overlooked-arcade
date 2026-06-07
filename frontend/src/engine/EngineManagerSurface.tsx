// Engine territory — fullscreen takeover that hosts every engine-
// rendered surface (Settings, Library Manager, Import Wizard, BIOS,
// Cores, System Health, Background Jobs). Mounted from App.tsx;
// visibility driven by platform/engineSurface.ts.
//
// Per docs/features/theming-substrate/SURFACES.md (D2 + D3):
// - Engine surface is fullscreen takeover — not a modal, not a drawer.
// - All 3 summon affordances (F12 / Select+Start / corner icon) reach
//   the same surface. Wire-up of those affordances lives in App.tsx
//   (hotkey + chord) and the active theme's reserved top-right slot
//   (corner icon — see EngineSummonIcon).
// - Escape / B button / click outside the panel area closes it.
// - The takeover preserves the theme's underlying view; on close the
//   operator returns to whatever they were doing.
//
// Phase 1 ships only the Settings panel. Library Manager, Import
// Wizard, BIOS pre-checks, Core installer, System Health, Background
// Jobs are reached today THROUGH the Settings panel (via SETTINGS →
// Library / System Health internal tabs / etc.) — those entries
// remain unchanged; only their host moved from the SETTINGS tab to
// this engine surface. The map is enumerated in SURFACES.md.

import { createEffect, onCleanup, Show, type Component } from "solid-js";
import { closeEngineSurface, engineSurfaceOpen } from "../platform/engineSurface";
import SettingsPanel from "./SettingsPanel";

const EngineManagerSurface: Component = () => {
  // Escape closes the surface. Document-level capture so it fires
  // regardless of which inner control has focus.
  createEffect(() => {
    if (!engineSurfaceOpen()) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        closeEngineSurface();
      }
    };
    window.addEventListener("keydown", handler, true);
    onCleanup(() => window.removeEventListener("keydown", handler, true));
  });

  return (
    <Show when={engineSurfaceOpen()}>
      <div
        // Fullscreen, opaque, above ALL other UI (ToastStack uses z-50;
        // drop overlay uses z-50; QuickSettings uses z-40-ish). z-[60]
        // sits cleanly above. Backdrop is fully opaque per D3 ("not a
        // modal overlay") — operator is fully in engine territory while
        // the surface is open.
        class="fixed inset-0 z-[60] flex flex-col bg-(--color-oa-bg-deep) text-(--color-oa-ink)"
        role="dialog"
        aria-modal="true"
        aria-label="OA engine settings"
      >
        {/* Header — back button + label. Mirrors the affordance density
            of RetroverseShell's top bar so the operator's visual rhythm
            doesn't break on summon. */}
        <header class="flex h-16 shrink-0 items-center gap-4 border-b border-white/5 bg-(--color-oa-bg-deep)/95 px-6 backdrop-blur">
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              closeEngineSurface();
            }}
            class="grid h-9 w-9 shrink-0 place-items-center rounded-md border border-white/10 bg-white/[0.04] text-sm text-(--color-oa-ink-dim) transition hover:border-(--color-oa-ink-dim)/50 hover:bg-white/[0.08] hover:text-(--color-oa-ink) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
            title="Back to library (Esc)"
            aria-label="Back to library"
          >
            ←
          </button>
          <div class="leading-tight">
            <p class="text-sm font-semibold uppercase tracking-[0.2em]">
              OA Settings
            </p>
            <p class="text-[0.55rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
              Engine surface · Press Esc to return
            </p>
          </div>
        </header>

        {/* Body — Phase 1 mounts the SettingsPanel only. Phase 2+ may
            introduce other top-level surfaces (e.g. a dedicated
            Appearance picker once Phase 5's theme loader lands), at
            which point this becomes a small router. */}
        <main class="min-h-0 min-w-0 flex-1 overflow-hidden">
          <SettingsPanel />
        </main>
      </div>
    </Show>
  );
};

export default EngineManagerSurface;
