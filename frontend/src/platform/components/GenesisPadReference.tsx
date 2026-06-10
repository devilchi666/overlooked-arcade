// Visual Genesis / Mega Drive 6-button pad reference for the
// per-system Bindings dialog (and the per-game Input dialog).
//
// Companion to KeypadReference.tsx (Coleco) — same "physical
// controller render + current bindings labeled per face button" idea.
// The Genesis 6-button pad shipped in 1993 alongside Street Fighter
// II: SCE; arranged as two rows of three (X-Y-Z above A-B-C). The
// D-pad sits on the left, Start + Mode buttons in the centre. The
// row table SystemBindingsEditor renders alphabetically doesn't
// convey the spatial relationship — A is "next to B is next to C
// along the bottom row," X is "above A," etc. — that fighting-game
// muscle memory relies on. This component closes that gap.
//
// Shared across the Genesis family (genesis / segacd / sega32x /
// sega32xcd) — `apps/oa-shell/src/bindings.rs:1820` routes all four
// slugs to the same `GENESIS_BUTTONS` table.

import {
  createResource,
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
  /// One of the four Genesis-family slugs. The component only
  /// renders when systemId is in `GENESIS_SYSTEMS`; callers can
  /// mount unconditionally and let the Show gate handle the rest.
  systemId: SystemId;
};

/// The four OA slugs that share the Genesis controller binding
/// table. Sega CD and 32X both use Genesis pads; 32X-CD inherits
/// from segacd's stacked-override pattern.
export const GENESIS_SYSTEMS: ReadonlySet<SystemId> = new Set<SystemId>([
  "genesis",
  "segacd",
  "sega32x",
  "sega32xcd",
]);

/// Face-button layout — 6-button pad arrangement. Row 0 is X-Y-Z
/// (top), row 1 is A-B-C (bottom). Matches the physical 6-button
/// controller (1993 onward); 3-button games use only A-B-C and Start.
const FACE_LAYOUT: ReadonlyArray<{ name: string; row: 0 | 1; col: 0 | 1 | 2 }> = [
  { name: "X", row: 0, col: 0 },
  { name: "Y", row: 0, col: 1 },
  { name: "Z", row: 0, col: 2 },
  { name: "A", row: 1, col: 0 },
  { name: "B", row: 1, col: 1 },
  { name: "C", row: 1, col: 2 },
];

/// D-pad direction layout — center cell intentionally empty.
const DPAD_LAYOUT: ReadonlyArray<{ name: string | null; glyph: string; row: 0 | 1 | 2; col: 0 | 1 | 2 }> = [
  { name: null, glyph: "", row: 0, col: 0 },
  { name: "UP", glyph: "▲", row: 0, col: 1 },
  { name: null, glyph: "", row: 0, col: 2 },
  { name: "LEFT", glyph: "◄", row: 1, col: 0 },
  { name: null, glyph: "·", row: 1, col: 1 },
  { name: "RIGHT", glyph: "►", row: 1, col: 2 },
  { name: null, glyph: "", row: 2, col: 0 },
  { name: "DOWN", glyph: "▼", row: 2, col: 1 },
  { name: null, glyph: "", row: 2, col: 2 },
];

const GenesisPadReference: Component<Props> = (props) => {
  // Per-system bindings — same Tauri command SystemBindingsEditor
  // uses. Re-fetches when systemId switches (e.g. operator nav from
  // genesis to segacd within the same dialog session).
  const [bindings] = createResource<ButtonBinding[] | null, SystemId>(
    () => props.systemId,
    async (id) => {
      try {
        return await getBindings<ButtonBinding>(id);
      } catch (e) {
        console.warn("[oa-genesis-pad-ref] get_bindings failed:", e);
        return null;
      }
    },
  );

  const findBinding = (button: string): ButtonBinding | undefined =>
    bindings()?.find((b) => b.button === button);

  /// Mini-card for one face button or D-pad direction. Renders the
  /// physical label on top + the current keyboard / gamepad binding
  /// below. Used for both A-Z face buttons (centre-aligned, square)
  /// and D-pad directions (arrow glyph instead of letter).
  const ButtonCard: Component<{
    label: string;
    button: string | null;
    size?: "sm" | "md";
  }> = (cardProps) => {
    const b = () => (cardProps.button ? findBinding(cardProps.button) : undefined);
    const mapped = () => cardProps.button !== null;
    return (
      <div
        class="flex flex-col items-center justify-center rounded border bg-(--color-oa-bg-deep) px-1 py-1.5 text-center"
        classList={{
          "border-(--color-system-accent)/40": mapped(),
          "border-transparent": !mapped(),
          "min-h-[3.25rem] min-w-[2.5rem]": cardProps.size === "md",
          "min-h-[2.25rem] min-w-[2rem]": cardProps.size !== "md",
        }}
      >
        <div
          class="leading-none"
          classList={{
            "text-base font-bold text-(--color-system-accent)": cardProps.size === "md",
            "text-sm text-(--color-oa-ink-dim)": cardProps.size !== "md",
          }}
        >
          {cardProps.label}
        </div>
        <Show when={mapped()}>
          <div class="mt-1 flex flex-col items-center gap-0 text-[0.5rem] uppercase tracking-wider text-(--color-oa-ink-dim)">
            <Show
              when={b()?.keyboard || b()?.gamepad}
              fallback={<span>—</span>}
            >
              <Show when={b()?.keyboard}>
                {(k) => <span title={`Keyboard: ${k()}`}>⌨ {k()}</span>}
              </Show>
              <Show when={b()?.gamepad}>
                {(g) => <span title={`Gamepad: ${g()}`}>🎮 {g()}</span>}
              </Show>
            </Show>
          </div>
        </Show>
      </div>
    );
  };

  return (
    <Show when={GENESIS_SYSTEMS.has(props.systemId)}>
      <div class="rounded border border-(--color-oa-bg) bg-(--color-oa-bg)/40 p-3">
        <div class="mb-2 text-sm font-medium text-(--color-oa-ink)">
          Physical pad reference (6-button Mega Drive)
        </div>
        <div class="mb-3 text-xs leading-relaxed text-(--color-oa-ink-dim)">
          A / B / C live along the bottom row; X / Y / Z are the
          shoulder-row buttons added on the 1993 6-button pad
          (Street Fighter II: SCE launch). 3-button-era games use
          only A / B / C + Start. Edit bindings in
          <span class="text-(--color-oa-ink)"> System → Bindings</span>.
        </div>

        <div class="flex items-start justify-between gap-4">
          {/* D-pad (3×3 grid, centre empty) */}
          <div class="flex flex-col">
            <div class="mb-1 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              D-Pad
            </div>
            <div class="grid grid-cols-3 gap-1">
              {DPAD_LAYOUT.map((slot) => (
                <Show
                  when={slot.name !== null}
                  fallback={<div class="min-h-[2.25rem] min-w-[2rem]" />}
                >
                  <ButtonCard label={slot.glyph} button={slot.name} size="sm" />
                </Show>
              ))}
            </div>
          </div>

          {/* Centre: Mode + Start stack */}
          <div class="flex flex-col">
            <div class="mb-1 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Centre
            </div>
            <div class="flex flex-col gap-1">
              <ButtonCard label="MODE" button="MODE" size="sm" />
              <ButtonCard label="START" button="START" size="sm" />
            </div>
          </div>

          {/* Face buttons — 6-button arrangement (X-Y-Z above A-B-C). */}
          <div class="flex flex-col">
            <div class="mb-1 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Face buttons
            </div>
            <div class="grid grid-cols-3 gap-1">
              {FACE_LAYOUT.map((slot) => (
                <ButtonCard label={slot.name} button={slot.name} size="md" />
              ))}
            </div>
          </div>
        </div>

        <Show when={bindings.error}>
          <p class="mt-2 text-[0.65rem] text-red-300/80">
            Couldn't load per-system bindings.
          </p>
        </Show>
      </div>
    </Show>
  );
};

export default GenesisPadReference;
