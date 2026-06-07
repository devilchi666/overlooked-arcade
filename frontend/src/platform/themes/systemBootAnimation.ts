// Per-System UI boot-animation bridge.
//
// Module-level signal mirroring `Settings.bootAnimationsEnabled` so the
// SystemBootAnimation component reads via `isBootAnimationsEnabled()`
// without prop-drilling through the layout grid. App.tsx wires the
// bridge via a createEffect — same pattern as the controller-nav
// primitives in `nav/focus.ts` (`setSwapAB`) and the per-system SFX
// dispatcher in `themes/systemUiSound.ts` (`setPerSystemUiEnabled`).
//
// Default state is ON to match the persisted Settings default; the
// App.tsx createEffect overrides on hydration.

import { createSignal, type Accessor } from "solid-js";

const [enabledSig, setEnabledSig] = createSignal(true);

/// Bridge for the App.tsx createEffect that watches
/// `settings.bootAnimationsEnabled()`. When the operator flips the
/// sub-toggle in Settings → Display, the SystemBootAnimation honors
/// it on the next system-entry transition. Reduced-motion is
/// orthogonal — that path always shortens to 200 ms regardless of
/// this flag (accessibility floor).
export function setBootAnimationsEnabled(on: boolean): void {
  setEnabledSig(on);
}

/// Reactive accessor used by SystemBootAnimation's trigger effect.
export const isBootAnimationsEnabled: Accessor<boolean> = enabledSig;
