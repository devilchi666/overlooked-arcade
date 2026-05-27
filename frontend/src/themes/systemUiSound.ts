// Per-System UI sound dispatcher.
//
// Thin gating layer on top of `lib/audio.ts::dispatchUiSound` that
// respects the two Stage 1 Settings-driven opt-outs:
//
//   1. `perSystemUiEnabled` master toggle in Settings → Display. When
//      off, every per-system UI event suppresses silently. App.tsx
//      bridges the Settings store's signal to `setPerSystemUiEnabled`
//      via a createEffect; defaults to ON so a fresh session is
//      audible.
//   2. `audioProfile === "none"` in `systemUIConfigs[systemId]`. A
//      system can opt itself out without the operator touching
//      Settings — useful for noisy / inappropriate systems (none in
//      Stage 1 baseline, but the option lives in the schema).
//
// The underlying `dispatchUiSound` is "best-effort, silent on miss"
// — when no operator override + no bundled asset exists for the
// (systemId, event) pair, playback no-ops. Slice 2 wires this helper
// into the library-grid focus-group navigation + activation paths;
// later slices add call sites for the sidebar, context menus, etc.

import { createSignal, type Accessor } from "solid-js";
import { dispatchUiSound, type UiSoundEvent } from "../lib/audio";
import type { SystemId } from "./registry";
import { uiConfigFor } from "./systemUIConfigs";

const [enabledSig, setEnabledSig] = createSignal(true);

/// Bridge for the App.tsx createEffect that watches
/// `settings.perSystemUiEnabled()`. When the operator flips the master
/// toggle in Settings → Display, this dispatcher honors it on the next
/// nav / activation event.
export function setPerSystemUiEnabled(on: boolean): void {
  setEnabledSig(on);
}

/// Reactive accessor for downstream code that wants to know whether
/// the master toggle is on (e.g. boot-animation framework gating in
/// Slice 4). Exported alongside the setter so call sites have a
/// reactive view rather than a stale snapshot.
export const isPerSystemUiEnabled: Accessor<boolean> = enabledSig;

/// Fire a per-system UI sound for an event on a specific system. Drops
/// the dispatch silently when:
///
/// - The master toggle is off (operator wants a uniform plain library).
/// - The system's `audioProfile` is `"none"` (system opted out).
///
/// Otherwise hands off to `dispatchUiSound(systemId, event)`, which
/// runs the Rust resolver cascade (operator override → per-system
/// bundle → universal baseline → silence) and plays through the
/// `ui-sounds` mixer bus on the Rust side.
///
/// Fire-and-forget — the dispatch is async on the Rust side but call
/// sites in nav / activation handlers shouldn't await it.
export function playSystemUiSound(systemId: SystemId, event: UiSoundEvent): void {
  if (!enabledSig()) return;
  if (uiConfigFor(systemId).audioProfile === "none") return;
  void dispatchUiSound(systemId, event);
}
