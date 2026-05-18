import { Show, type Component, type JSX } from "solid-js";

// Phase 2.8 slice C — the three-tier inheritance UI primitive.
//
// Renders a label + child input + an inherited-value chip. The chip shows
// what the setting WOULD be if not overridden — typically the OA-wide
// default for a per-system row, or the per-system value for a per-game row.
// VS Code's workspace-vs-user settings pattern.
//
// The component is presentation-only: parent controls the input + the
// override state. SettingRow doesn't try to be a smart "controlled / un-
// controlled" wrapper because every override surface has its own persistence
// shape. Keeping it dumb means it slots into both per-system (slice C) and
// per-game (slice D) without conditionalizing.
//
// Layout: three-column flex row. Label left, input center (flex-1), chip
// right. On narrow screens or when overridden, the chip drops below the
// input (responsive via flex-wrap). Reset-to-default lives on the parent's
// input control (e.g. a "Use default" option in a select), NOT on SettingRow
// itself — different inputs handle reset differently and pushing it into
// the wrapper would force a Cancel-button shape that doesn't fit every case.

type Props = {
  /// Short identifier shown left of the input. ~2-4 words.
  label: string;
  /// Optional secondary hint shown under the label in dim text.
  hint?: string;
  /// What this setting WOULD resolve to if not overridden. Rendered as a
  /// greyed chip on the right. Pass null when the row has no upstream
  /// inheritance (e.g. an OA-wide setting on the root Settings page).
  inheritedValue?: string | null;
  /// Where the inherited value comes from. Default "OA default" for per-
  /// system rows; per-game rows should pass "Per-system" when the per-system
  /// override is the inherited source. Shown as a small tag above the chip.
  inheritedFrom?: string;
  /// True when the user has set an override (i.e., the parent's value
  /// differs from the inherited one). When false, the input renders dimmed
  /// to suggest "this is inherited, not active here."
  overridden: boolean;
  /// The input control. SettingRow doesn't dictate shape — pass a `<select>`,
  /// `<input>`, button group, whatever fits.
  children: JSX.Element;
};

const SettingRow: Component<Props> = (props) => {
  return (
    <div class="flex flex-wrap items-start gap-4 rounded-md border border-white/5 bg-white/[0.02] px-4 py-3">
      <div class="flex w-48 shrink-0 flex-col gap-0.5">
        <p class="text-sm font-medium text-(--color-oa-ink)">{props.label}</p>
        <Show when={props.hint}>
          <p class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            {props.hint}
          </p>
        </Show>
      </div>
      <div
        class="flex min-w-[12rem] flex-1 items-center gap-2"
        classList={{ "opacity-70": !props.overridden && props.inheritedValue !== null }}
      >
        {props.children}
      </div>
      <Show when={props.inheritedValue !== null && props.inheritedValue !== undefined}>
        <div class="flex flex-col items-end gap-0.5">
          <p class="text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            {props.inheritedFrom ?? "OA default"}
          </p>
          <span
            class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-xs"
            classList={{
              "text-(--color-oa-ink)": !props.overridden,
              "text-(--color-oa-ink-dim) line-through": props.overridden,
            }}
            title={
              props.overridden
                ? `Overridden — inherited value was ${props.inheritedValue}`
                : "Inherited value (in effect)"
            }
          >
            {props.inheritedValue}
          </span>
        </div>
      </Show>
    </div>
  );
};

export default SettingRow;
