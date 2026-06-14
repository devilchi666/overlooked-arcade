// Import & Setup — the engine Settings landing where a new operator gets ROMs
// into OA. New top-level category in the Settings IA redesign (Slice 1): pulls
// onboarding OUT of Library (which is now pure directory management) into its
// own home. Card shell for Slice 1 — the guided first-run depth fills in
// Slice 5 (ties to docs/features/guided-setup/).

import { type Component } from "solid-js";
import { HubGrid } from "./systemsHub/HubGrid";
import { HubCard } from "./systemsHub/HubCard";
import { setWizardOpen } from "@oa/platform/dialogs";
import { addLibraryFolder, rescanLibraryFolders } from "@oa/platform/libraryAdmin";

export const ImportSetupLanding: Component = () => {
  return (
    <div class="flex flex-col gap-4">
      <HubGrid>
        <HubCard
          title="Set up your library"
          subtitle="Guided — recommended for first-time setup"
          onActivate={() => setWizardOpen(true)}
        >
          <p class="text-[0.7rem] leading-relaxed text-(--color-oa-ink-dim)">
            Drop in your ROMs and OA will get them ready: detect systems, suggest
            canonical titles via hash matching, and walk you through anything that
            needs your input.
          </p>
        </HubCard>

        <HubCard
          title="Add a folder"
          subtitle="Quick — pick a directory and scan it now"
          onActivate={() => addLibraryFolder()}
        >
          <p class="text-[0.7rem] leading-relaxed text-(--color-oa-ink-dim)">
            Already know your layout? Point OA at a folder and it scans
            immediately, no wizard. Manage tracked folders under Library.
          </p>
        </HubCard>

        <HubCard
          title="Rescan all folders"
          subtitle="Pick up new or removed ROMs"
          onActivate={() => rescanLibraryFolders()}
        >
          <p class="text-[0.7rem] leading-relaxed text-(--color-oa-ink-dim)">
            Re-walks every tracked library folder and reconciles the database
            with what's on disk.
          </p>
        </HubCard>
      </HubGrid>
    </div>
  );
};

export default ImportSetupLanding;
