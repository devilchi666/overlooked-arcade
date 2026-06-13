// Controller LABEL family — Controller Identity arc (Phase 2.5 follow-up).
// See docs/PLANS/controller-identity-substrate.md + DECISIONS D15/D16.
//
// Distinct from button MAPPING (raw→canonical, in controllerProfiles.ts). The
// canonical model is positional (south/east/west/north). The FAMILY is which
// label set a pad's face buttons carry — Nintendo's labels sit in swapped
// positions vs Xbox, so a positionally-correct map still *reads* swapped on a
// Nintendo pad ("press Y, see X"). Family resolves that: it picks the label
// set, so the test window (and later, glyphs) show the pad's REAL labels.
//
// The collapse that makes this scale: thousands of controllers → ~4 families →
// one label set each. We DON'T tag every pad; we detect family. Detection is
// authoritative-first: SDL's VID/PID→type table (controllerTypes.json, ported
// from SDL controller_list.h) → name-keyword heuristic → generic fallback.

import typeDb from "./controllerTypes.json";

export type ControllerFamily = "nintendo" | "xbox" | "playstation" | "generic";

/// Face + shoulder labels per family. Canonical positions on the left; the
/// family's printed label on the right. (DPad / sticks / start-select are
/// family-neutral enough to skip here for the MVP label read-out.)
type LabelKey = "south" | "east" | "west" | "north" | "l1" | "r1" | "l2" | "r2";

const FAMILY_LABELS: Record<ControllerFamily, Record<LabelKey, string>> = {
  nintendo: { south: "B", east: "A", west: "Y", north: "X", l1: "L", r1: "R", l2: "ZL", r2: "ZR" },
  xbox: { south: "A", east: "B", west: "X", north: "Y", l1: "LB", r1: "RB", l2: "LT", r2: "RT" },
  playstation: { south: "✕", east: "○", west: "□", north: "△", l1: "L1", r1: "R1", l2: "L2", r2: "R2" },
  generic: { south: "A", east: "B", west: "X", north: "Y", l1: "L1", r1: "R1", l2: "L2", r2: "R2" },
};

const FAMILIES: Record<string, ControllerFamily> = (typeDb.families ?? {}) as Record<
  string,
  ControllerFamily
>;

/// NavButton → the canonical label key (face buttons are positional: a=south).
const NAV_TO_LABEL_KEY: Record<string, LabelKey> = {
  a: "south",
  b: "east",
  x: "west",
  y: "north",
  l1: "l1",
  r1: "r1",
  l2: "l2",
  r2: "r2",
};

/// Resolve a pad's label family. Authoritative SDL table first (keyed by the
/// VID:PID inside the device-key), then a defensive name-keyword heuristic for
/// pads not in the table, then generic. (An OA per-pad override could slot in
/// ahead of the table later if SDL ever misclassifies a pad.)
export function resolveFamily(deviceKey: string, name?: string): ControllerFamily {
  const m = deviceKey.match(/^vidpid:([0-9a-f]{4}):([0-9a-f]{4})$/);
  if (m) {
    const fam = FAMILIES[`${m[1]}:${m[2]}`];
    if (fam) return fam;
  }
  if (name) {
    const n = name.toLowerCase();
    if (/switch|nintendo|joy-?con|wii|gamecube/.test(n)) return "nintendo";
    if (/dualsense|dualshock|playstation|\bps[345]\b/.test(n)) return "playstation";
    if (/xbox|xinput|x-?box/.test(n)) return "xbox";
  }
  return "generic";
}

/// The family's printed label for a given NavButton (e.g. nintendo + "a" →
/// "B"). Non-face/shoulder buttons fall back to the uppercased NavButton name.
export function familyLabel(family: ControllerFamily, navButton: string): string {
  const key = NAV_TO_LABEL_KEY[navButton];
  if (key) return FAMILY_LABELS[family][key];
  return navButton.toUpperCase();
}
