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

type RawProfile = {
  deviceKey: string;
  name?: string;
  unverified?: boolean;
  buttons?: Record<string, number>;
  dpad?: Record<string, number>;
  hatAxis?: number;
};

const PROFILES: RawProfile[] = (db.profiles ?? []) as RawProfile[];

/// Resolve the layout for a pad. Returns `null` when the default standard
/// layout should be used — i.e. the browser already gave a standard mapping,
/// or no profile matches this device-key. A non-null result means "this
/// non-standard pad has a curated remap; use it instead of BUTTON_NAMES".
export function resolveLayout(
  deviceKey: string,
  mapping: GamepadMappingType | string,
): ResolvedLayout | null {
  // Trust the browser when it canonicalized the pad itself.
  if (mapping === "standard") return null;
  const profile = PROFILES.find((p) => p.deviceKey === deviceKey);
  if (!profile) return null;
  return buildLayout(profile);
}

/// Build a ResolvedLayout from a profile. Exported for tests.
export function buildLayout(profile: RawProfile): ResolvedLayout {
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
