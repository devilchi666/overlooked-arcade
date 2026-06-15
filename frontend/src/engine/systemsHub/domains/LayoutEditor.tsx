// Layout domain editor — the L5 end-user "pick your view per system" surface
// (Theming ARC 2 / D32). For the selected system it lets the user override which
// layout primitive each library-journey view mounts, writing the L3 override
// store (`platform/theme/layoutOverrides`). The override is the TOP tier of the
// resolver cascade, so a pick takes effect immediately (LibraryView consumes
// `useResolvedLayout`) and persists across the restart-based theme swap.
//
// Overrides are keyed by the ACTIVE theme id, so what's edited here applies to
// the current theme only — the subtitle says so. Per-row Reset clears the
// override and falls back to the theme / engine tiers (D30 reset discipline,
// mirroring the NavRemapCard); the inheritance chip shows what that fallback
// resolves to and which tier supplies it.

import { For, type Accessor, type Component } from "solid-js";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import SettingRow from "@oa/platform/components/SettingRow";
import { activeTheme, activeThemeId } from "@oa/platform/theme/registry";
import {
  clearLayoutOverride,
  getLayoutOverride,
  setLayoutOverride,
} from "@oa/platform/theme/layoutOverrides";
import { resolveLayout } from "@oa/platform/theme/layoutResolver";
import {
  LAYOUT_PRIMITIVES,
  type LayoutPrimitive,
  type ThemeViews,
  type ViewType,
} from "@oa/platform/theme/manifest";
import { HubSection, PanelScaffold } from "../PanelScaffold";

/// Views shown first (the live ones), then the contract-reserved ones. Only
/// `game-browse` currently has a renderer (LibraryView); the other three views
/// are honored by the resolver but nothing mounts them yet — the editor labels
/// that honestly rather than implying a pick takes effect.
const VIEW_ORDER: readonly ViewType[] = [
  "game-browse",
  "system-browse",
  "manufacturer-browse",
  "game-details",
];

/// Views with a live consumer today — drives the per-row "shown now" vs
/// "reserved" hint. Grow this as more views gain a renderer.
const HONORED_VIEWS: ReadonlySet<ViewType> = new Set<ViewType>(["game-browse"]);

const VIEW_LABEL: Record<ViewType, string> = {
  "manufacturer-browse": "Manufacturer browse",
  "system-browse": "System browse",
  "game-browse": "Game browse",
  "game-details": "Game details",
};

const PRIMITIVE_LABEL: Record<LayoutPrimitive, string> = {
  list: "List",
  grid: "Grid",
  carousel: "Carousel",
  wheel: "Wheel",
  custom: "Custom (theme-drawn)",
};

/// User-selectable primitives — everything the contract names except `custom`,
/// which means "the theme draws this view" and isn't authorable from a dropdown
/// (operator call: curate to list / grid / carousel / wheel). It still appears
/// in the inheritance chip when a view's default resolves to `custom`.
const SELECTABLE: readonly LayoutPrimitive[] = LAYOUT_PRIMITIVES.filter(
  (p) => p !== "custom",
);

/// Which cascade tier supplies the no-override fallback — for the chip's "from".
function fallbackScope(
  themeViews: ThemeViews | undefined,
  view: ViewType,
  systemId: SystemId,
): string {
  const v = themeViews?.[view];
  if (v) {
    if (v.per_system?.[systemId]) return "this theme · per-system";
    if (v.layout) return "this theme";
  }
  return "engine default";
}

export const LayoutEditor: Component<{ systemId: Accessor<SystemId> }> = (
  props,
) => {
  const themeName = (): string => activeTheme()?.manifest.name ?? "current theme";

  return (
    <PanelScaffold
      system={props.systemId()}
      title={systemThemes[props.systemId()]?.displayName ?? props.systemId()}
      subtitle={`Library layout · overrides apply to the ${themeName()} theme`}
    >
      <HubSection title="Browse layout">
        <For each={VIEW_ORDER}>
          {(view) => {
            const themeViews = (): ThemeViews | undefined =>
              activeTheme()?.manifest.views;
            // The user's override for (active theme, this system, this view).
            const override = (): LayoutPrimitive | undefined => {
              const t = activeThemeId();
              return t ? getLayoutOverride(t, props.systemId(), view) : undefined;
            };
            // What this view resolves to WITHOUT the user override — the chip's
            // value (theme per_system → theme default → engine default).
            const fallback = (): LayoutPrimitive =>
              resolveLayout({
                override: undefined,
                themeViews: themeViews(),
                view,
                systemId: props.systemId(),
              });
            const set = (v: string): void => {
              const t = activeThemeId();
              if (!t) return;
              if (v === "") clearLayoutOverride(t, props.systemId(), view);
              else
                setLayoutOverride(
                  t,
                  props.systemId(),
                  view,
                  v as LayoutPrimitive,
                );
            };
            return (
              <SettingRow
                label={VIEW_LABEL[view]}
                hint={
                  HONORED_VIEWS.has(view)
                    ? "Shown in the library now"
                    : "Reserved — no renderer yet"
                }
                inherited={{ value: PRIMITIVE_LABEL[fallback()], from: fallbackScope(themeViews(), view, props.systemId()) }}
                overridden={override() !== undefined}
                onReset={() => set("")}
                select={{
                  tone: "system",
                  value: override() ?? "",
                  options: [
                    {
                      value: "",
                      label: `Theme default — ${PRIMITIVE_LABEL[fallback()]}`,
                    },
                    ...SELECTABLE.map((p) => ({
                      value: p,
                      label: PRIMITIVE_LABEL[p],
                    })),
                  ],
                  onChange: set,
                }}
              />
            );
          }}
        </For>
      </HubSection>
    </PanelScaffold>
  );
};
