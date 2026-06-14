// Responsive card grid for the Systems hub — same breakpoints as the Game-media
// grid (1 / 2 / 3 columns). No data-nav-region of its own: the hub renders
// inside SettingsPanel's `data-nav-region="settings-content"`, so the spatial
// engine flows UP/DOWN through the breadcrumb + grid and LEFT/RIGHT to the
// categories sidebar by geometry (Pillar-B byproduct).

import { type Component, type JSX } from "solid-js";

export const HubGrid: Component<{ children: JSX.Element }> = (props) => (
  <div class="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3">{props.children}</div>
);
