// External Emulators — the engine Settings landing that will be the single home
// for standalone-emulator setup (Dolphin / Cemu / RPCS3 / Lime3DS / …): binary
// paths, per-profile config, and the download/install pipeline.
//
// Slice 1 of the Settings IA redesign stands this up as a card SHELL only. The
// live configuration still lives under System Health → Cores ("External
// emulators" section, VL Phase C2). The consolidation — moving that section
// here + wiring the install pipeline — is Slice 4, which rides Virtual-Library
// Phase D. Kept as an honest placeholder rather than a duplicate surface so the
// IA is visible now without two sources of truth.

import { type Component } from "solid-js";
import { HubSection } from "./systemsHub/PanelScaffold";

export const ExternalEmulatorsLanding: Component = () => {
  return (
    <div class="flex flex-col gap-4">
      <HubSection title="Standalone emulators">
        <div class="space-y-3 text-sm text-(--color-oa-ink-dim)">
          <p>
            Some systems run best through a dedicated standalone emulator
            (Dolphin for GameCube/Wii, Cemu for Wii U, RPCS3 for PS3, Lime3DS for
            3DS) instead of an in-process libretro core. This is where their
            setup will live — binary paths, per-emulator options, and one-click
            installs.
          </p>
          <p class="rounded-lg border border-white/10 bg-white/[0.02] p-3 text-xs leading-relaxed">
            <span class="font-semibold uppercase tracking-wider text-(--color-system-accent)">
              Coming soon ·{" "}
            </span>
            For now, configure external-emulator binaries and per-system launcher
            choice under <span class="text-(--color-oa-ink)">System Health → Cores</span>
            {" "}and a system's <span class="text-(--color-oa-ink)">Launcher</span> card
            in Systems. The full consolidation here ships with the external-emulator
            install pipeline (Virtual-Library Phase D).
          </p>
        </div>
      </HubSection>
    </div>
  );
};

export default ExternalEmulatorsLanding;
