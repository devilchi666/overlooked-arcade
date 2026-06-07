// Retroverse UI experimental flag bridge.
//
// Mirrors the per-system-ui master toggle shape (themes/systemUiSound.ts
// :: setPerSystemUiEnabled / isPerSystemUiEnabled). App.tsx bridges the
// Settings store's signal to setRetroverseUiEnabled via a createEffect;
// downstream code reads isRetroverseUiEnabled() reactively.
//
// Phase A wires the flag without consumers — accessor is exported so
// Phase B's RetroverseShell + top-tab strip can gate on it as the
// first surface. Default OFF: existing UI stays byte-identical until
// the operator flips Settings → Display → Experimental → Retroverse UI.
//
// See docs/PLANS/retroverse-ui-rollout.md for the rollout plan.

import { createSignal, type Accessor } from "solid-js";

const [enabledSig, setEnabledSig] = createSignal(false);

/// Bridge for the App.tsx createEffect that watches
/// `settings.experimentalRetroverseUi()`. When the operator flips the
/// toggle in Settings → Display → Experimental, downstream gates pick
/// the new value up on next render.
export function setRetroverseUiEnabled(on: boolean): void {
  setEnabledSig(on);
}

/// Reactive accessor for code that wants to know whether Retroverse UI
/// is active for this session. Exported alongside the setter so call
/// sites get a reactive view rather than a stale snapshot.
export const isRetroverseUiEnabled: Accessor<boolean> = enabledSig;
