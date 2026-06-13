// Controller profile resolution for the MENU poller — Controller Identity arc
// Phase 2 (the non-standard-pad normalization that fixes the Switch Pro in
// menus). See docs/PLANS/controller-identity-substrate.md + DECISIONS D12.
//
// The Web Gamepad API gives us a clean "standard layout" ONLY when
// `gamepad.mapping === "standard"`. For non-standard pads (mapping === "")
// the raw button/axis indices are arbitrary, so BUTTON_NAMES mis-maps them
// (the Switch Pro "Y selects, nothing else does" bug). This module resolves a
// per-pad layout from controllers.json that remaps RAW Web indices onto the
// canonical model, then onto NavButton/NavDirection.
//
// Scope (D12): the frontend DB is OA-curated in Web-Gamepad-index space, NOT a
// raw SDL gamecontrollerdb import (SDL numbers buttons in joystick-index space,
// which doesn't align with Web indices). The Rust gameplay poller already
// normalizes via gilrs' native SDL mappings, so this layer is menus-only.

import db from "./controllers.json";
import mappingDb from "./controllerMappings.json";
import type { CanonicalButton } from "./canonical";
import type { NavButton, NavDirection } from "./types";

/// Canonical (non-direction) button → NavButton. The face buttons map by
/// POSITION (south = bottom = the Confirm-default "a"); Nintendo's "B confirms"
/// preference is handled downstream by the navBindings A/B-swap, not here.
const CANONICAL_TO_NAV: Record<string, NavButton> = {
  south: "a",
  east: "b",
  west: "x",
  north: "y",
  l1: "l1",
  r1: "r1",
  l2: "l2",
  r2: "r2",
  l3: "l3",
  r3: "r3",
  start: "start",
  select: "select",
  guide: "home",
};

const DPAD_NAMES = ["up", "down", "left", "right"] as const;

/// A pad's resolved button/axis layout in RAW Web-index space. `null` hatAxis
/// means "no profile-declared HAT" (auto-detection still applies).
export type ResolvedLayout = {
  /** raw button index → NavButton (face / shoulder / system). */
  buttons: Record<number, NavButton>;
  /** raw button index → NavDirection (DPad reported as buttons). */
  dpad: Record<number, NavDirection>;
  /** axis index the pad reports its DPad on as a HAT, or null. */
  hatAxis: number | null;
};

/// The raw-index layout fields shared by curated profiles and bulk DB entries.
type LayoutSpec = {
  buttons?: Record<string, number>;
  dpad?: Record<string, number>;
  hatAxis?: number;
};

type RawProfile = LayoutSpec & {
  deviceKey: string;
  name?: string;
  unverified?: boolean;
};

const PROFILES: RawProfile[] = (db.profiles ?? []) as RawProfile[];

/// Bulk SDL gamecontrollerdb mappings, keyed by `vid:pid` (Web-index space,
/// digital buttons; hat-DPads handled by the runtime HAT detector). The
/// curated `controllers.json` overrides win over this layer.
const BULK: Record<string, LayoutSpec> = (mappingDb.mappings ?? {}) as Record<string, LayoutSpec>;

/// Where a resolved layout came from. `null` = no match (use standard layout).
export type LayoutSource = "curated" | "sdl-db" | null;

function bulkKey(deviceKey: string): string | null {
  const m = deviceKey.match(/^vidpid:([0-9a-f]{4}):([0-9a-f]{4})$/);
  return m ? `${m[1]}:${m[2]}` : null;
}

/// Resolve the layout for a pad. Returns `null` when the default standard
/// layout should be used — the browser already canonicalized the pad, or no
/// curated/bulk profile matches. A non-null result is a remap to use instead
/// of the blind standard `BUTTON_NAMES`. Lookup order: curated override
/// (`controllers.json`) → bulk SDL DB (`controllerMappings.json`).
export function resolveLayout(
  deviceKey: string,
  mapping: GamepadMappingType | string,
): ResolvedLayout | null {
  // Trust the browser when it canonicalized the pad itself.
  if (mapping === "standard") return null;
  const curated = PROFILES.find((p) => p.deviceKey === deviceKey);
  if (curated) return buildLayout(curated);
  const key = bulkKey(deviceKey);
  if (key && BULK[key]) return buildLayout(BULK[key]);
  return null;
}

/// Which layer (if any) supplies this pad's layout — for the test window.
export function layoutSource(
  deviceKey: string,
  mapping: GamepadMappingType | string,
): LayoutSource {
  if (mapping === "standard") return null;
  if (PROFILES.some((p) => p.deviceKey === deviceKey)) return "curated";
  const key = bulkKey(deviceKey);
  if (key && BULK[key]) return "sdl-db";
  return null;
}

/// Build a ResolvedLayout from a curated profile or a bulk DB entry. Exported
/// for tests.
export function buildLayout(profile: LayoutSpec): ResolvedLayout {
  const buttons: Record<number, NavButton> = {};
  for (const [canon, idx] of Object.entries(profile.buttons ?? {})) {
    const nav = CANONICAL_TO_NAV[canon as CanonicalButton];
    // Silently skip unknown canonical names so a forward-compatible DB entry
    // (e.g. a future canonical button) doesn't break older builds.
    if (nav !== undefined) buttons[idx] = nav;
  }
  const dpad: Record<number, NavDirection> = {};
  for (const [dir, idx] of Object.entries(profile.dpad ?? {})) {
    if ((DPAD_NAMES as readonly string[]).includes(dir)) {
      dpad[idx] = dir as NavDirection;
    }
  }
  return { buttons, dpad, hatAxis: profile.hatAxis ?? null };
}
