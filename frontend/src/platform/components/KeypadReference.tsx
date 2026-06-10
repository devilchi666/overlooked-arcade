// Visual keypad reference for the per-game Input dialog.
//
// Coleco / Intellivision / Odyssey²-era controllers shipped with
// 12-button keypads (3×4 grid) where each game defined its own paper
// overlay assigning meaning to KP1-KP9 (+ * and # for Coleco / CLEAR
// and ENTER for Intellivision). The keypad_layout_note text-area
// already exists in the per-game Input dialog and lets operators
// document "KP1=climb-up, KP2=climb-down" per game, but operators
// still have to mentally translate between the note ("KP1") and the
// physical keyboard key that triggers it (the per-system binding).
//
// This component bridges that gap: renders the physical 3×4 keypad
// layout with each button labeled with its current keyboard /
// gamepad mapping from per-system bindings. The note sits next to
// it; together they answer "which key on my keyboard makes the
// game do X?" at a glance.
//
// Coleco-only for now. Intv shares the 3×4 shape but with different
// button names (CLEAR / ENTER instead of * / #) — easy follow-on
// when its ROADMAP picks this up.

import {
  createResource,
  For,
  Show,
  type Component,
} from "solid-js";
import { getBindings } from "@oa/platform/api/inputApi";
import type { SystemId } from "@oa/platform/themes/registry";

type ButtonBinding = {
  button: string;
  keyboard: string | null;
  gamepad: string | null;
};

type Props = {
  /// Either "coleco" or any other keypad-using system. Component
  /// renders the Coleco-specific layout for now; extend with a
  /// switch when more systems are wired in.
  systemId: SystemId;
};

/// Visual layout of the physical 12-button Coleco keypad. Top row
/// is 1-2-3, bottom row is *-0-#. The `name` field maps to the
/// `COLECO_BUTTONS` table in `apps/oa-shell/src/bindings.rs`; null
/// means the physical button exists on the controller but isn't
/// wired to libretro today (the * and # keys land here — they'd
/// need RETRO_DEVICE_KEYBOARD passthrough that the bluemsx core
/// doesn't surface).
const COLECO_KEYPAD_LAYOUT: ReadonlyArray<{ name: string | null; label: string }> = [
  { name: "KP1", label: "1" }, { name: "KP2", label: "2" }, { name: "KP3", label: "3" },
  { name: "KP4", label: "4" }, { name: "KP5", label: "5" }, { name: "KP6", label: "6" },
  { name: "KP7", label: "7" }, { name: "KP8", label: "8" }, { name: "KP9", label: "9" },
  { name: null,  label: "*" }, { name: "KP0", label: "0" }, { name: null,  label: "#" },
];

const KeypadReference: Component<Props> = (props) => {
  // Per-system bindings — same Tauri command SystemBindingsEditor
  // uses. Re-fetches when the focused game switches to a different
  // system (rare in the same dialog session but cheap to handle).
  const [bindings] = createResource<ButtonBinding[] | null, SystemId>(
    () => props.systemId,
    async (id) => {
      try {
        return await getBindings<ButtonBinding>(id);
      } catch (e) {
        console.warn("[oa-keypad-ref] get_bindings failed:", e);
        return null;
      }
    },
  );

  const findBinding = (button: string): ButtonBinding | undefined =>
    bindings()?.find((b) => b.button === button);

  return (
    <div class="rounded border border-(--color-oa-bg) bg-(--color-oa-bg)/40 p-3">
      <div class="mb-2 text-sm font-medium text-(--color-oa-ink)">
        Physical keypad reference
      </div>
      <div class="mb-3 text-xs leading-relaxed text-(--color-oa-ink-dim)">
        The Coleco controller had this 12-button keypad on the front.
        Each game shipped a paper overlay telling the player what
        each number meant (Donkey Kong: 1=jump, 2=climb-up, …).
        Current per-system keyboard / gamepad mappings are shown
        below each button. Edit them in
        <span class="text-(--color-oa-ink)"> System → Bindings</span>.
      </div>
      <div class="grid grid-cols-3 gap-1.5">
        <For each={COLECO_KEYPAD_LAYOUT}>
          {(slot) => {
            const b = slot.name ? findBinding(slot.name) : undefined;
            const mapped = slot.name !== null;
            return (
              <div
                class="rounded border bg-(--color-oa-bg-deep) p-2 text-center"
                classList={{
                  "border-(--color-system-accent)/40": mapped,
                  "border-(--color-oa-bg)/40 opacity-50": !mapped,
                }}
              >
                <div
                  class="text-xl font-semibold leading-none"
                  classList={{
                    "text-(--color-oa-ink)": mapped,
                    "text-(--color-oa-ink-dim)": !mapped,
                  }}
                >
                  {slot.label}
                </div>
                <div class="mt-1 text-[0.55rem] uppercase tracking-wider text-(--color-oa-ink-dim)">
                  <Show
                    when={mapped}
                    fallback={<span>unmapped</span>}
                  >
                    <Show
                      when={b?.keyboard || b?.gamepad}
                      fallback={<span>—</span>}
                    >
                      <div class="flex flex-col items-center gap-0.5">
                        <Show when={b?.keyboard}>
                          {(k) => <span title={`Keyboard: ${k()}`}>⌨ {k()}</span>}
                        </Show>
                        <Show when={b?.gamepad}>
                          {(g) => <span title={`Gamepad: ${g()}`}>🎮 {g()}</span>}
                        </Show>
                      </div>
                    </Show>
                  </Show>
                </div>
              </div>
            );
          }}
        </For>
      </div>
      <Show when={bindings.error}>
        <p class="mt-2 text-[0.65rem] text-red-300/80">
          Couldn't load per-system bindings.
        </p>
      </Show>
    </div>
  );
};

export default KeypadReference;
