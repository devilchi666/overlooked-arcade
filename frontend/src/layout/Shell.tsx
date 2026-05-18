import { Show, type Component, type JSX } from "solid-js";
import type { LayoutStore } from "./state";

type Props = {
  layout: LayoutStore;
  toolbar: JSX.Element;
  leftSidebar: JSX.Element;
  rightSidebar: JSX.Element;
  /** Main content slot — library grid, system page, game detail, etc. */
  children: JSX.Element;
  /** When true the main content area is rendered full-bleed: no sidebars,
   *  no toolbar gap. Used when a game is running in single-window mode and
   *  the WebView UI should "melt away" so wgpu pixels show through. */
  fullBleed?: boolean;
};

/**
 * Top-level layout primitive. Renders the three-region grid (toolbar /
 * left+main+right / optional status bar) using CSS-variable-driven geometry
 * tokens declared in index.css. Sidebar widths come from the LayoutStore so
 * user resize gestures + presentation-mode overrides both flow through one
 * source of truth.
 *
 * Presentation-mode rules are applied via `body[data-presentation="..."]`
 * (see index.css `@layer base`). This component only needs to honor the
 * collapsed/visible booleans the user controls; the actual pixel widths
 * cascade from CSS.
 */
const Shell: Component<Props> = (props) => {
  const leftWidthPx = () => {
    if (props.layout.presentationMode() === "cabinet") return 0;
    if (props.layout.leftSidebarCollapsed()) {
      return parseInt(
        getComputedStyle(document.documentElement)
          .getPropertyValue("--layout-left-sidebar-collapsed-width") || "64",
        10,
      );
    }
    return props.layout.leftSidebarWidth();
  };

  const rightWidthPx = () => {
    if (props.layout.presentationMode() === "cabinet") return 0;
    if (!props.layout.rightSidebarVisible()) return 0;
    return props.layout.rightSidebarWidth();
  };

  return (
    <Show
      when={!props.fullBleed}
      fallback={<div class="h-full w-full">{props.children}</div>}
    >
      <div
        class="grid h-full w-full overflow-hidden"
        style={{
          "grid-template-columns": `${leftWidthPx()}px minmax(0,1fr) ${rightWidthPx()}px`,
          "grid-template-rows": "var(--layout-top-toolbar-height) minmax(0,1fr)",
          "grid-template-areas": `"toolbar toolbar toolbar" "left main right"`,
        }}
      >
        <div
          style={{ "grid-area": "toolbar", position: "relative", "z-index": 40 }}
          class="min-w-0"
        >
          {props.toolbar}
        </div>
        <Show when={leftWidthPx() > 0}>
          <div style={{ "grid-area": "left" }} class="min-w-0 overflow-hidden">
            {props.leftSidebar}
          </div>
        </Show>
        <div style={{ "grid-area": "main" }} class="min-w-0 overflow-hidden">
          {props.children}
        </div>
        <Show when={rightWidthPx() > 0}>
          <div style={{ "grid-area": "right" }} class="min-w-0 overflow-hidden">
            {props.rightSidebar}
          </div>
        </Show>
      </div>
    </Show>
  );
};

export default Shell;
