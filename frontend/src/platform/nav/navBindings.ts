// The physical-input → semantic-verb indirection layer.
//
// This is the keystone of the verb-native nav model (DECISIONS D18): a single
// OA-wide map translating raw gamepad buttons + keyboard keys into the
// `NavVerb` vocabulary. The focus framework and HintBar resolve through this
// map, so remapping an input updates dispatch AND every on-screen glyph for
// free.
//
// Tier: OA-wide (not per-theme, not per-game) — one config applied identically
// across all themes, giving full user control AND cross-theme muscle-memory
// consistency. Persisted in appData (`nav_bindings.json`) via
// `platform/api/navBindingsApi`. Defaults = the operator-locked controller-nav
// spec, so out-of-the-box behaviour is unchanged; remapping is opt-in and
// arrives with the remap Settings UI (the Phase-3 follow-on after the S2 swap
// gate).

import { createSignal, type Accessor } from "solid-js";
import type { NavButton } from "./types";
import type { NavVerb } from "./verbs";
import { getNavBindings, saveNavBindings } from "@oa/platform/api/navBindingsApi";

/// The OA-wide input→verb map. Both channels are partial: an input absent from
/// the map produces no verb (inert). Directional verbs are resolved 1:1 from
/// raw D-pad/stick directions by the focus framework (identity in S1), so they
/// don't appear in the gamepad button map; the keyboard map DOES carry arrows
/// (keyboard has no separate "stick" channel).
export type NavBindings = {
  /** Web Gamepad "standard layout" button name → verb. */
  gamepad: Partial<Record<NavButton, NavVerb>>;
  /** KeyboardEvent.key → verb. */
  keyboard: Partial<Record<string, NavVerb>>;
};

/// Default bindings = the operator-locked controller-nav spec verbatim:
/// A confirm / B back / X·Y secondary·tertiary / L1·R1 cycle sections /
/// Start = menu. Select stays unbound (reserved for the Select+Start
/// engine-summon chord). The keyboard channel ships arrow-key directional nav
/// in S1; Confirm already works natively on focusable buttons (Enter/Space),
/// and Enter/Back/Esc keyboard verbs are added with the remap follow-on (they
/// need a native-control coexistence audit).
export const DEFAULT_BINDINGS: NavBindings = {
  gamepad: {
    a: "Confirm",
    b: "Back",
    x: "Secondary",
    y: "Tertiary",
    l1: "PrevSection",
    r1: "NextSection",
    start: "Menu",
  },
  keyboard: {
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
  },
};

// --- live bindings signal ---------------------------------------------------

const [bindingsSig, setBindingsSig] = createSignal<NavBindings>(DEFAULT_BINDINGS);

/// Reactive accessor for the live bindings. The HintBar derives glyphs from
/// this, so a remap re-paints hints automatically.
export const navBindings: Accessor<NavBindings> = bindingsSig;

/// Replace the live bindings and persist to appData. Used by the remap
/// Settings UI (the Phase-3 follow-on). Persistence failures are non-fatal —
/// the in-memory map still updates so the session reflects the change.
export async function setNavBindings(next: NavBindings): Promise<void> {
  setBindingsSig(next);
  await saveNavBindings(next).catch((e) => {
    console.error("[oa-nav] failed to persist nav bindings", e);
  });
}

/// Load persisted bindings from appData, falling back to defaults. Call once at
/// app mount (mirrors `startGamepadInput`). Missing / malformed file → defaults.
export async function loadNavBindings(): Promise<void> {
  try {
    const stored = await getNavBindings();
    if (stored) setBindingsSig(normalize(stored));
  } catch (e) {
    console.error("[oa-nav] failed to load nav bindings; using defaults", e);
  }
}

/// Merge a stored (possibly partial / older-schema) map over the defaults so a
/// hand-edited or future-version file never drops a channel.
function normalize(stored: Partial<NavBindings>): NavBindings {
  return {
    gamepad: { ...DEFAULT_BINDINGS.gamepad, ...(stored.gamepad ?? {}) },
    keyboard: { ...DEFAULT_BINDINGS.keyboard, ...(stored.keyboard ?? {}) },
  };
}

// --- A/B swap overlay -------------------------------------------------------
//
// "Nintendo-convention layout" (B confirms, A backs) is expressed as a
// resolve-time overlay rather than mutating the stored map: physical A and B
// trade roles at lookup. This keeps the swap orthogonal to (future) remapped
// bindings and matches today's behaviour, just relocated into the resolver.
// The setting itself stays persisted in the settings store
// (`controllerNavSwapAB`) and is re-applied on load via a createEffect in
// App.tsx — so this overlay is not separately persisted.

const [swapABSig, setSwapABSig] = createSignal(false);

/// When true, physical A and B swap roles before verb resolution. Settings
/// calls this when the operator flips the "Swap A/B" toggle.
export function setSwapAB(on: boolean): void {
  setSwapABSig(on);
}

/// Reactive accessor — the HintBar reads it to render the swapped glyph.
export const isSwapAB: Accessor<boolean> = swapABSig;

// --- resolution -------------------------------------------------------------

/// Apply the A/B swap overlay to a physical button name.
function applySwap(button: NavButton, swap: boolean): NavButton {
  if (!swap) return button;
  if (button === "a") return "b";
  if (button === "b") return "a";
  return button;
}

/// Resolve a raw gamepad button to its verb under the current bindings (+ swap
/// overlay). Returns null when the button is unbound.
export function resolveButtonVerb(
  button: NavButton,
  bindings: NavBindings,
  swap: boolean,
): NavVerb | null {
  return bindings.gamepad[applySwap(button, swap)] ?? null;
}

/// Resolve a KeyboardEvent.key to its verb under the current bindings. Returns
/// null when the key is unbound.
export function resolveKeyVerb(key: string, bindings: NavBindings): NavVerb | null {
  return bindings.keyboard[key] ?? null;
}

/// The physical button that currently triggers `verb` on the gamepad, with the
/// swap overlay applied — i.e. the button whose GLYPH the HintBar should show.
/// Returns null when no button maps to the verb. (Reverse of
/// `resolveButtonVerb`.)
export function buttonForVerb(
  verb: NavVerb,
  bindings: NavBindings,
  swap: boolean,
): NavButton | null {
  let found: NavButton | null = null;
  for (const [btn, v] of Object.entries(bindings.gamepad)) {
    if (v === verb) {
      found = btn as NavButton;
      break;
    }
  }
  if (found === null) return null;
  return applySwap(found, swap);
}
