// Organize My Collection — the engine Settings landing for library
// *organization* (navigation structure), new in the Settings IA redesign
// (Slice 1). Three sections, all relocated out of the old Library Manager:
//   1. Sidebar layouts — custom sidebar trees (the surface formerly called
//      "Views"; renamed user-facing to kill the "Views = appearance?" trap).
//   2. Collections — flat curated member sets.
//   3. Sidebar systems — hide/show systems in the left sidebar.
//
// Renders inside SettingsPanel's `data-nav-region="settings-content"`, so the
// spatial engine drives every control by geometry (zero per-control wiring).

import { type Component } from "solid-js";
import { usePlatform } from "@oa/platform/platformContext";
import ViewsManagerTab from "./ViewsManagerTab";
import CollectionsManager from "./CollectionsManager";
import SidebarSystemsCard from "./SidebarSystemsCard";

const SectionHeader: Component<{ title: string; hint?: string }> = (props) => (
  <div class="space-y-1">
    <h2 class="text-xs font-semibold uppercase tracking-[0.3em] text-(--color-system-accent)">
      {props.title}
    </h2>
    {props.hint ? (
      <p class="text-[0.65rem] text-(--color-oa-ink-dim)">{props.hint}</p>
    ) : null}
  </div>
);

export const OrganizeLanding: Component = () => {
  const platform = usePlatform();

  return (
    <div class="flex flex-col gap-8">
      <section class="space-y-3">
        <SectionHeader
          title="Sidebar layouts"
          hint="Group your systems into a custom left-sidebar tree (by manufacturer, form factor, era, or your own hierarchy). This is navigation structure — it doesn't change how tiles look."
        />
        <ViewsManagerTab
          views={platform.views}
          library={platform.library}
          customCollections={platform.customCollections}
        />
      </section>

      <section class="space-y-3 border-t border-white/5 pt-6">
        <CollectionsManager />
      </section>

      <section class="space-y-3 border-t border-white/5 pt-6">
        <SidebarSystemsCard />
      </section>
    </div>
  );
};

export default OrganizeLanding;
