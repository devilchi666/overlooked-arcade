import { Show, type Component } from "solid-js";

import type { LayoutStore } from "../layout/state";
import { DEFAULT_VIEW_ID, LEGACY_VIEW_ID } from "../views/defaults";
import { reorderForFormFactor } from "../views/migration";
import type { ViewsStore } from "../views/store";

type Props = {
  views: ViewsStore;
  layout: LayoutStore;
  /// Forwarded so the banner can collapse to nothing when the sidebar
  /// collapses to its icon-only width — the banner copy wouldn't fit
  /// in the icon column.
  collapsed: boolean;
};

/// Upgrade-install migration banner per SIDEBAR_TIER_PLAN.md §3.7. Shown
/// at the top of the sidebar nav when:
///   - the active view is the seeded `flat-legacy` view (operator was on
///     a pre-PR-β build and got their `layout.systemOrder` migrated into
///     a Flat (Legacy) view), AND
///   - `bannerDismissed` is false in views.json.
///
/// Two buttons:
///   - Try Form Factor view → applies `reorderForFormFactor` to preserve
///     the operator's relative ordering within each form-factor bucket
///     (Option C), swaps the active view to `default-formfactor`,
///     dismisses the banner. Single batched mutation via
///     `viewsStore.commitTryFormFactor`.
///   - Stay on Flat (Legacy) → just dismisses the banner. The Flat-Legacy
///     view stays active; FormFactor view stays seeded and picker-
///     selectable in v2.
const SidebarMigrationBanner: Component<Props> = (props) => {
  const visible = () => {
    if (props.collapsed) return false;
    if (!props.views.hydrated()) return false;
    const cfg = props.views.config();
    if (cfg.bannerDismissed) return false;
    return cfg.activeViewId === LEGACY_VIEW_ID;
  };

  function handleTry() {
    const cfg = props.views.config();
    const defaultView = cfg.views.find((v) => v.id === DEFAULT_VIEW_ID);
    if (!defaultView) return;
    const reordered = reorderForFormFactor(defaultView, props.layout.systemOrder());
    props.views.commitTryFormFactor(reordered);
  }

  function handleStay() {
    props.views.setBannerDismissed(true);
  }

  return (
    <Show when={visible()}>
      <div class="mb-3 rounded-md border border-(--color-system-accent)/30 bg-(--color-system-accent)/[0.06] p-3 text-xs text-(--color-oa-ink)">
        <p class="font-semibold">We've reorganized your system list.</p>
        <p class="mt-1 text-(--color-oa-ink-dim)">
          Systems are now grouped by form factor (Consoles, Handhelds,
          Computers, Arcade). Your customized order is preserved as a
          "Flat (Legacy)" view if you'd rather not switch yet.
        </p>
        <div class="mt-3 flex flex-wrap gap-2">
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              handleTry();
            }}
            class="rounded-md bg-(--color-system-accent) px-3 py-1.5 text-[0.65rem] font-semibold uppercase tracking-wider text-(--color-oa-bg-deep) transition hover:brightness-110"
          >
            Try Form Factor view
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              handleStay();
            }}
            class="rounded-md border border-white/10 bg-white/[0.03] px-3 py-1.5 text-[0.65rem] font-semibold uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.07] hover:text-(--color-oa-ink)"
          >
            Stay on Flat (Legacy)
          </button>
        </div>
      </div>
    </Show>
  );
};

export default SidebarMigrationBanner;
