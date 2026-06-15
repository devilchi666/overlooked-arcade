// External Emulators — the single home for standalone-emulator setup (Dolphin /
// Cemu / RPCS3 / Lime3DS / …): binary paths + per-system default launcher.
//
// Settings IA Slice 4 made this real: the config moved here from System Health →
// Cores (the duplicate section there is retired), via the self-contained
// `ExternalEmulatorsSection`. The one-click download/install pipeline still
// rides Virtual-Library Phase D (unbuilt) — this is bring-your-own-binary today.
// The research roster driving which emulators we profile lives at
// docs/RESEARCH/external-emulators.md.

import { type Component } from "solid-js";
import { HubSection } from "./systemsHub/PanelScaffold";
import ExternalEmulatorsSection from "./ExternalEmulatorsSection";

export const ExternalEmulatorsLanding: Component = () => {
  return (
    <div class="flex flex-col gap-4">
      <HubSection title="Standalone emulators">
        <div class="space-y-4">
          <p class="text-sm text-(--color-oa-ink-dim)">
            Some systems run best through a dedicated standalone emulator
            (Dolphin for GameCube/Wii, and more as support grows) instead of an
            in-process libretro core. Point OA at your installed binary and pick
            it as the default launcher per system. The per-system launcher choice
            also lives on each system's card in Systems.
          </p>
          <ExternalEmulatorsSection />
          <p class="rounded-lg border border-white/10 bg-white/[0.02] p-3 text-xs leading-relaxed text-(--color-oa-ink-dim)">
            <span class="font-semibold uppercase tracking-wider text-(--color-system-accent)">
              Coming ·{" "}
            </span>
            One-click download + setup of these emulators (and more systems we
            can't run in-process) ships with the external-emulator install
            pipeline (Virtual-Library Phase D).
          </p>
        </div>
      </HubSection>
    </div>
  );
};

export default ExternalEmulatorsLanding;
