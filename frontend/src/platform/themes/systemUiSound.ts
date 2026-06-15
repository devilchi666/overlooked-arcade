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
import { dispatchUiSound, type UiSoundEvent } from "@oa/platform/lib/audio";
import type { NavSoundEvent } from "@oa/platform/nav";
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
/// reactive view rather than a stale snapshot. This is the USER master
/// toggle only — for the shared-grid tile/SFX consumption gate use
/// `consumesPerSystemTiles` / `consumesPerSystemSfx` (they fold in the
/// active theme's opt-in, D33).
export const isPerSystemUiEnabled: Accessor<boolean> = enabledSig;

/// Per-theme consumption of the shared-grid per-system UI surfaces (Theming
/// ARC 2 L1; DECISIONS D33/D34). Per-system UI is a platform *capability*;
/// whether a theme *consumes* it on the shared `LibraryTile` / grid is the
/// theme's opt-in, bridged from the active theme's manifest `per_system_ui` by
/// an App.tsx createEffect (mirrors the glyph-set bridge). Defaults OFF for both
/// — a theme that declares nothing (CoverFlow / bare) gets a uniform grid, the
/// D33 "uniformly theme-opt-in" rule. (Backgrounds + boot stay opt-in by
/// component mount, so they need no flag here.)
const [themeUiSig, setThemeUiSig] = createSignal<{ tiles: boolean; sfx: boolean }>({
  tiles: false,
  sfx: false,
});

/// Bridge for the App.tsx createEffect watching
/// `activeTheme()?.manifest.per_system_ui`. Coerces missing / partial configs to
/// explicit booleans so the gate accessors never see `undefined`.
export function setThemePerSystemUi(
  cfg: { tiles?: boolean; sfx?: boolean } | undefined,
): void {
  setThemeUiSig({ tiles: cfg?.tiles === true, sfx: cfg?.sfx === true });
}

/// Effective gate for per-system TILE flourishes (tileShape + interactionStyle):
/// the user master toggle AND the active theme opting into `tiles`. Consumed by
/// `LibraryTile` and the grid's column-fitting estimate.
export const consumesPerSystemTiles: Accessor<boolean> = () =>
  enabledSig() && themeUiSig().tiles;

/// Effective gate for per-system nav SFX: user master AND the active theme
/// opting into `sfx`. `playSystemUiSound` consumes this.
export const consumesPerSystemSfx: Accessor<boolean> = () =>
  enabledSig() && themeUiSig().sfx;

/// Fire a per-system UI sound for an event on a specific system. Drops
/// the dispatch silently when:
///
/// - Per-system SFX aren't consumed here — either the user master toggle is
///   off (uniform plain library) OR the active theme didn't opt into `sfx`
///   (`consumesPerSystemSfx()`, D33).
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
  if (!consumesPerSystemSfx()) return;
  if (uiConfigFor(systemId).audioProfile === "none") return;
  void dispatchUiSound(systemId, event);
}

/// Maps a primitive's coarse `NavSoundEvent` to a per-system `UiSoundEvent`.
const NAV_SOUND_EVENT: Record<NavSoundEvent, UiSoundEvent> = {
  move: "navigate",
  confirm: "launch",
  back: "back",
  secondary: "click",
};

/// Engine-default `onNavSound` handler for the nav primitives (scope-call #6). A
/// theme wires `onNavSound={navSoundDispatcher((item) => item?.systemId)}`; the
/// primitive's move/confirm/back/secondary events then play that item's
/// per-system UI sound (through the master-toggle + audioProfile gating +
/// theme→platform resolver cascade — same path as the library grid). Generic on
/// the item type via a `systemIdOf` selector, so the nav layer never needs to
/// know an item carries a system.
export function navSoundDispatcher<T>(
  systemIdOf: (item: T | undefined) => SystemId | undefined,
): (event: NavSoundEvent, item: T | undefined) => void {
  return (event, item) => {
    const systemId = systemIdOf(item);
    if (systemId) playSystemUiSound(systemId, NAV_SOUND_EVENT[event]);
  };
}
