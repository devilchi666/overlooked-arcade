// Retroverse-UI Phase A Slice 3 — persistent right-side detail pane.
//
// Thin wrapper over GameInfoModal that locks variant="panel" so callers
// don't have to remember the prop. Mounts inside a parent that owns
// layout (e.g. the LIBRARY tab's right column in the Retroverse 3-pane
// shell). No backdrop, no Close button, no modal-scoped HintRegion —
// the parent tab provides the hint bar and back-stack semantics.
//
// `onClose` still required so the parent can react to a "deselect"
// gesture (e.g. clearing the focused tile). Pass a no-op when nothing
// to do.
//
// See docs/PLANS/retroverse-ui-rollout.md (Phase B will import this
// from frontend/src/routes/retroverse/LibraryPage.tsx).

import type { Component } from "solid-js";
import type { RomEntry } from "../library/types";
import GameInfoModal from "./GameInfoModal";

type Props = {
  entry: RomEntry | null;
  onClose: () => void;
  onLaunched?: (entry: RomEntry, slot?: number) => void;
};

const RightDetailPanel: Component<Props> = (props) => {
  return (
    <GameInfoModal
      entry={props.entry}
      onClose={props.onClose}
      onLaunched={props.onLaunched}
      variant="panel"
    />
  );
};

export default RightDetailPanel;
