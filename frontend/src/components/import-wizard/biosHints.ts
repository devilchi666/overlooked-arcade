// Phase 1B Slice 5 — per-system stub map for the "Where to get it"
// hint surfaced by BiosResolutionDetail when a row has ⚠ BIOS state.
//
// The stub points the operator at the per-core README in the
// repository, where sourcing notes can be added per-system over time
// (operator-driven content; OA doesn't curate BIOS sources because
// of the legal-sketchy nature, per `docs/PLANS/guided-setup.md` §5
// Step 6). Operator-specific hints live in the per-core README at
// `docs/cores/<systemId>/README.md`; this map gives the operator a
// pointer when the readiness checklist surfaces a missing BIOS.

import type { SystemId } from "../../themes/registry";

/// Default hint shown for any system that doesn't have an explicit
/// per-system entry below. Keeps the affordance discoverable while
/// leaving real sourcing guidance up to per-core curation.
const DEFAULT_HINT =
  "Drop the matching files into the BIOS folder shown above. " +
  "See `docs/cores/<systemId>/README.md` in the OA repo for per-system " +
  "sourcing notes.";

/// Per-system override copy. Operator can flesh these out per system
/// over time — entries here override the DEFAULT_HINT for the matching
/// systemId. Keep entries terse; deep guidance belongs in the per-core
/// README, not here.
const HINTS: Partial<Record<SystemId, string>> = {
  // Intentionally sparse for Slice 5 — operator fills in per-system
  // copy at their own pace. Examples of what could land here:
  //   psx: "PlayStation BIOS comes from a Sony PS1 console you own — …"
  //   nds: "DS BIOS files extract from a real DS console's firmware — …"
  //   neogeo: "Neo Geo system BIOS zip — see MAME documentation …"
};

export function biosHintFor(systemId: SystemId): string {
  return HINTS[systemId] ?? DEFAULT_HINT;
}
