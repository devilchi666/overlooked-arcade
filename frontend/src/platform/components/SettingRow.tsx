import { For, Show, type Component, type JSX } from "solid-js";

// Three-tier inheritance row primitive (OA-wide / per-system / per-game).
//
// Renders: label · optional description · input · optional Reset button ·
// optional inheritance chip. The chip shows what the setting WOULD be if
// not overridden — typically the OA-wide default for a per-system row, or
// the per-system value for a per-game row. VS Code's workspace-vs-user
// settings pattern.
//
// The component is presentation-only: parent controls the value + the
// override state. Every override surface has its own persistence shape, so
// SettingRow doesn't try to be a smart controlled/uncontrolled wrapper —
// it just renders.
//
// Built-in controls (mutually exclusive with `children`):
//   • `select` — drop-down with options
//   • `slider` — <input type="range"> + value readout
//   • `toggle` — checkbox with label/description
// Pass `children` instead for any custom widget the built-ins don't cover.

export type InheritScope = "oa-default" | "per-system" | "per-game" | (string & {});

export type SelectOption = {
  value: string;
  label: string;
  disabled?: boolean;
};

type SelectControl = {
  value: string;
  options: ReadonlyArray<SelectOption>;
  onChange: (v: string) => void;
  placeholder?: string;
  /// "oa" → ink-dim focus ring (OA-wide dialogs).
  /// "system" → system-accent focus ring (per-system / per-game dialogs).
  /// Default "oa".
  tone?: "oa" | "system";
};

type SliderControl = {
  min: number;
  max: number;
  step: number;
  value: number;
  /// How to render the value readout to the right of the slider.
  /// Default: `v.toFixed(2)`.
  format?: (v: number) => string;
  onInput: (v: number) => void;
};

type ToggleControl = {
  checked: boolean;
  onChange: (v: boolean) => void;
};

type Props = {
  /// Short identifier shown left of the input. ~2-4 words.
  label: string;
  /// Optional short hint shown under the label.
  hint?: string;
  /// Longer prose below the control. Use this for explanations the old API
  /// crammed into all-caps `hint` text. Plain string or JSX.
  description?: string | JSX.Element;

  /// Typed inheritance descriptor. Pass `null` to say "no upstream tier"
  /// (OA-wide row), an object to render the chip. Preferred over the
  /// legacy inheritedValue/inheritedFrom pair below.
  inherited?: { value: string; from: InheritScope } | null;
  /// LEGACY — string-pair API kept as a passthrough during migration to
  /// `inherited`. Remove once every call site has migrated.
  inheritedValue?: string | null;
  inheritedFrom?: string;

  /// True when the user has set an override (parent value differs from the
  /// inherited one). When false and inheritance exists, the input dims to
  /// suggest "this is inherited, not active here."
  overridden: boolean;

  /// Dim the whole row + disable built-in controls. Used by inheritance-
  /// gating ("this field is only meaningful when X is on").
  disabled?: boolean;

  /// Renders a Reset chip next to the inheritance chip when `overridden`
  /// is true. Click clears the override (parent decides what that means).
  onReset?: () => void;

  // ---- Built-in controls — mutually exclusive with each other and with
  // `children`. If none are passed, `children` is rendered instead.
  select?: SelectControl;
  slider?: SliderControl;
  toggle?: ToggleControl;
  children?: JSX.Element;
};

/// Canonical `<select>` class — single source of truth for select styling
/// across every dialog/page. `tone` picks the focus-ring color:
///   • "oa"     → ink-dim ring (OA-wide dialogs)
///   • "system" → system-accent ring (per-system / per-game dialogs)
/// Defaults to "oa".
///
/// Non-SettingRow callers (table cells, list rows) import this directly
/// so the styling stays single-source even when the row isn't going
/// through SettingRow's `select={…}` built-in.
export function selectClass(tone?: "oa" | "system"): string {
  const ring =
    tone === "system"
      ? "focus-visible:outline-(--color-system-accent)"
      : "focus-visible:outline-(--color-oa-ink-dim)";
  return (
    "w-full rounded-md border border-white/10 bg-white/[0.04] px-3 py-2 text-sm font-medium text-(--color-oa-ink) transition hover:bg-white/[0.08] focus-visible:outline focus-visible:outline-2 disabled:opacity-50 " +
    ring
  );
}

const SettingRow: Component<Props> = (props) => {
  const inheritedInfo = (): { value: string; from: string } | null => {
    if (props.inherited !== undefined) {
      return props.inherited === null
        ? null
        : { value: props.inherited.value, from: props.inherited.from };
    }
    if (props.inheritedValue === undefined || props.inheritedValue === null) {
      return null;
    }
    return {
      value: props.inheritedValue,
      from: props.inheritedFrom ?? "OA default",
    };
  };

  const hasInheritance = () => inheritedInfo() !== null;

  return (
    <div
      // `data-setting-row` opts this row into the Settings center pane's
      // controller-nav focus group (platform/nav/settingsRowNav). `tabindex=-1`
      // lets the focus manager move DOM focus to the row for Tab / screen-reader
      // continuity; inner controls keep their own native tab order. No effect on
      // the legacy SettingsDialogs.tsx callers — they don't query for the marker.
      data-setting-row
      tabindex="-1"
      class="flex flex-wrap items-start gap-4 rounded-md border border-white/5 bg-white/[0.02] px-4 py-3"
      classList={{ "pointer-events-none opacity-50": !!props.disabled }}
    >
      <div class="flex w-48 shrink-0 flex-col gap-0.5">
        <p class="text-sm font-medium text-(--color-oa-ink)">{props.label}</p>
        <Show when={props.hint}>
          <p class="text-xs text-(--color-oa-ink-dim)">{props.hint}</p>
        </Show>
      </div>
      <div
        class="flex min-w-[12rem] flex-1 flex-col gap-1.5"
        classList={{ "opacity-70": !props.overridden && hasInheritance() }}
      >
        <div class="flex items-center gap-2">
          <Show
            when={props.select}
            fallback={
              <Show
                when={props.slider}
                fallback={
                  <Show when={props.toggle} fallback={props.children}>
                    {(t) => (
                      <label class="flex flex-1 cursor-pointer items-center gap-2 text-sm text-(--color-oa-ink-dim)">
                        <input
                          type="checkbox"
                          checked={t().checked}
                          onChange={(e) => t().onChange(e.currentTarget.checked)}
                          disabled={!!props.disabled}
                          class="size-4 cursor-pointer accent-(--color-system-accent)"
                        />
                        <span>{t().checked ? "On" : "Off"}</span>
                      </label>
                    )}
                  </Show>
                }
              >
                {(s) => (
                  <>
                    <input
                      type="range"
                      min={s().min}
                      max={s().max}
                      step={s().step}
                      value={s().value}
                      disabled={!!props.disabled}
                      onInput={(e) => {
                        const v = Number(e.currentTarget.value);
                        if (Number.isFinite(v)) s().onInput(v);
                      }}
                      class="flex-1"
                    />
                    <span class="w-12 text-right font-mono text-sm tabular-nums">
                      {(s().format ?? ((v) => v.toFixed(2)))(s().value)}
                    </span>
                  </>
                )}
              </Show>
            }
          >
            {(sel) => (
              <select
                value={sel().value}
                onChange={(e) => sel().onChange(e.currentTarget.value)}
                disabled={!!props.disabled}
                class={selectClass(sel().tone)}
              >
                <Show when={sel().placeholder}>
                  <option value="" disabled>
                    {sel().placeholder}
                  </option>
                </Show>
                <For each={sel().options}>
                  {(opt) => (
                    <option value={opt.value} disabled={opt.disabled}>
                      {opt.label}
                    </option>
                  )}
                </For>
              </select>
            )}
          </Show>
        </div>
        <Show when={props.description}>
          <div class="text-xs leading-relaxed text-(--color-oa-ink-dim)">
            {props.description}
          </div>
        </Show>
      </div>
      <Show when={hasInheritance() || (props.overridden && props.onReset)}>
        <div class="flex flex-col items-end gap-1">
          <Show when={hasInheritance()}>
            {(_) => {
              const info = inheritedInfo()!;
              return (
                <>
                  <p class="text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                    {info.from === "oa-default"
                      ? "OA default"
                      : info.from === "per-system"
                        ? "Per-system"
                        : info.from === "per-game"
                          ? "Per-game"
                          : info.from}
                  </p>
                  <span
                    class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-xs"
                    classList={{
                      "text-(--color-oa-ink)": !props.overridden,
                      "text-(--color-oa-ink-dim) line-through": props.overridden,
                    }}
                    title={
                      props.overridden
                        ? `Overridden — inherited value was ${info.value}`
                        : "Inherited value (in effect)"
                    }
                  >
                    {info.value}
                  </span>
                </>
              );
            }}
          </Show>
          <Show when={props.overridden && props.onReset}>
            <button
              type="button"
              onClick={(e) => {
                e.currentTarget.blur();
                props.onReset?.();
              }}
              class="rounded-md border border-white/10 bg-white/[0.03] px-2 py-0.5 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim) transition hover:bg-white/[0.07] hover:text-(--color-oa-ink)"
              title="Reset to inherited"
            >
              Reset
            </button>
          </Show>
        </div>
      </Show>
    </div>
  );
};

export default SettingRow;
