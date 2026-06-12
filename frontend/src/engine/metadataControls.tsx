// Engine territory — typed field controls for the Settings → Metadata
// editor (Metadata Curation arc S3). The premium half of the UX pillar:
// never bare text boxes for typed data. Shared by the game editor (and
// reusable by the system editor / future surfaces).
//
//   • ProvenanceField — the quiet-provenance row wrapper (D7): label +
//     control + an edited accent bar, with the "Default: <value> ·
//     Reset" affordance revealed only on hover / focus-within.
//   • NumberStepper   — − [n] + for year / players.
//   • StarRating      — 0–5 star widget for rating.
//   • SegmentedPills  — single-select pill row (region / release type).
//   • ChipInput       — token multi-select with datalist typeahead from
//     the library corpus (genres), keeps values consistent.

import { createSignal, For, Show, type Component, type JSX } from "solid-js";

const ACCENT_INPUT =
  "rounded-md border border-white/10 bg-black/30 px-3 py-1.5 text-sm text-(--color-oa-ink) placeholder:text-(--color-oa-ink-dim)/45 transition focus:border-(--color-system-accent)/60 focus:bg-black/40 focus:outline-none";

const STEPPER_BTN =
  "flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-white/10 bg-white/[0.04] text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)";

// --- Provenance row wrapper (D7) -----------------------------------------

export const ProvenanceField: Component<{
  label: string;
  overridden: boolean;
  /// The value the field falls back to with no override. When present,
  /// shown as "Default: <text>" on hover/focus.
  defaultText?: string;
  /// Tooltip on the Default chip (the precise source).
  defaultTitle?: string;
  onReset?: () => void;
  children: JSX.Element;
}> = (props) => (
  <div class="group relative flex items-start gap-3 rounded-md py-1.5 pl-3 pr-2 transition hover:bg-white/[0.03]">
    <span
      class="absolute inset-y-2 left-0 w-0.5 rounded-full bg-(--color-system-accent) transition-opacity"
      classList={{ "opacity-0": !props.overridden, "opacity-100": props.overridden }}
      aria-hidden="true"
    />
    <label
      class="mt-1.5 w-36 shrink-0 text-sm transition-colors"
      classList={{
        "text-(--color-oa-ink)": props.overridden,
        "text-(--color-oa-ink-dim)": !props.overridden,
      }}
    >
      {props.label}
    </label>
    <div class="min-w-0 flex-1">{props.children}</div>
    <div class="mt-1.5 flex w-44 shrink-0 items-center justify-end gap-2 text-[0.65rem] opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100">
      <Show when={props.defaultText}>
        <span class="truncate text-(--color-oa-ink-dim)/70" title={props.defaultTitle}>
          Default: {props.defaultText}
        </span>
      </Show>
      <Show when={props.overridden && props.onReset}>
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            props.onReset?.();
          }}
          class="shrink-0 rounded border border-white/10 bg-white/[0.04] px-1.5 py-0.5 uppercase tracking-widest text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
          title="Reset this field to its default"
        >
          Reset
        </button>
      </Show>
    </div>
  </div>
);

// --- Number stepper ------------------------------------------------------

export const NumberStepper: Component<{
  value?: number;
  placeholder?: string;
  min?: number;
  max?: number;
  step?: number;
  width?: string;
  onChange: (n: number | undefined) => void;
}> = (props) => {
  const step = () => props.step ?? 1;
  const clamp = (n: number) => {
    let v = n;
    if (props.min !== undefined) v = Math.max(props.min, v);
    if (props.max !== undefined) v = Math.min(props.max, v);
    return v;
  };
  const bump = (dir: number) => {
    const base = props.value ?? props.min ?? 0;
    props.onChange(clamp(base + dir * step()));
  };
  return (
    <div class="flex items-center gap-1.5">
      <button type="button" class={STEPPER_BTN} onClick={() => bump(-1)} aria-label="Decrease">
        −
      </button>
      <input
        type="number"
        value={props.value ?? ""}
        placeholder={props.placeholder}
        min={props.min}
        max={props.max}
        step={step()}
        onInput={(e) => {
          const raw = e.currentTarget.value;
          if (raw === "") return props.onChange(undefined);
          const n = Number(raw);
          if (Number.isFinite(n)) props.onChange(n);
        }}
        class={`${ACCENT_INPUT} text-center tabular-nums ${props.width ?? "w-24"}`}
      />
      <button type="button" class={STEPPER_BTN} onClick={() => bump(1)} aria-label="Increase">
        +
      </button>
    </div>
  );
};

// --- Star rating (0–5) ---------------------------------------------------

export const StarRating: Component<{
  value?: number;
  onChange: (n: number | undefined) => void;
}> = (props) => {
  const [hover, setHover] = createSignal<number | null>(null);
  const lit = (i: number) => {
    const h = hover();
    const v = h ?? props.value ?? 0;
    return i <= Math.round(v);
  };
  return (
    <div class="flex items-center gap-1" onMouseLeave={() => setHover(null)}>
      <For each={[1, 2, 3, 4, 5]}>
        {(i) => (
          <button
            type="button"
            onMouseEnter={() => setHover(i)}
            onClick={(e) => {
              e.currentTarget.blur();
              // Click the current value to clear it.
              props.onChange(props.value === i ? undefined : i);
            }}
            class="text-lg leading-none transition-colors"
            classList={{
              "text-(--color-system-accent)": lit(i),
              "text-(--color-oa-ink-dim)/30": !lit(i),
            }}
            title={`${i} star${i > 1 ? "s" : ""}`}
            aria-label={`${i} star${i > 1 ? "s" : ""}`}
          >
            ★
          </button>
        )}
      </For>
      <Show when={props.value !== undefined}>
        <span class="ml-1 text-[0.65rem] text-(--color-oa-ink-dim)">{props.value}/5</span>
      </Show>
    </div>
  );
};

// --- Segmented pills (single-select, clearable) --------------------------

export const SegmentedPills: Component<{
  value?: string;
  options: readonly string[];
  onChange: (v: string | undefined) => void;
}> = (props) => (
  <div class="flex flex-wrap gap-1.5">
    <For each={props.options}>
      {(opt) => {
        const selected = () => props.value === opt;
        return (
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              props.onChange(selected() ? undefined : opt);
            }}
            class="rounded-full border px-2.5 py-1 text-[0.7rem] transition"
            classList={{
              "border-(--color-system-accent)/50 bg-(--color-system-accent)/20 text-(--color-system-accent-soft)":
                selected(),
              "border-white/10 bg-white/[0.03] text-(--color-oa-ink-dim) hover:bg-white/[0.06]":
                !selected(),
            }}
            aria-pressed={selected()}
          >
            {opt}
          </button>
        );
      }}
    </For>
  </div>
);

// --- Chip input (token multi-select with typeahead) ----------------------

export const ChipInput: Component<{
  values: string[];
  suggestions: readonly string[];
  placeholder?: string;
  /// Unique id for the backing <datalist> (typeahead source).
  listId: string;
  onChange: (next: string[]) => void;
}> = (props) => {
  const [text, setText] = createSignal("");
  const add = (raw: string) => {
    const v = raw.trim();
    if (!v) return;
    if (props.values.some((x) => x.toLowerCase() === v.toLowerCase())) {
      setText("");
      return;
    }
    props.onChange([...props.values, v]);
    setText("");
  };
  const removeAt = (i: number) => {
    const next = props.values.slice();
    next.splice(i, 1);
    props.onChange(next);
  };
  return (
    <div class="flex flex-wrap items-center gap-1.5 rounded-md border border-white/10 bg-black/30 px-2 py-1.5 transition focus-within:border-(--color-system-accent)/60">
      <For each={props.values}>
        {(chip, i) => (
          <span class="flex items-center gap-1 rounded-full border border-(--color-system-accent)/30 bg-(--color-system-accent)/15 px-2 py-0.5 text-[0.7rem] text-(--color-oa-ink)">
            {chip}
            <button
              type="button"
              onClick={(e) => {
                e.currentTarget.blur();
                removeAt(i());
              }}
              class="text-(--color-oa-ink-dim) transition hover:text-red-200"
              aria-label={`Remove ${chip}`}
            >
              ×
            </button>
          </span>
        )}
      </For>
      <input
        list={props.listId}
        value={text()}
        placeholder={props.values.length ? "" : props.placeholder}
        onInput={(e) => setText(e.currentTarget.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === ",") {
            e.preventDefault();
            add(text());
          } else if (e.key === "Backspace" && text() === "" && props.values.length) {
            removeAt(props.values.length - 1);
          }
        }}
        class="min-w-[6rem] flex-1 bg-transparent text-sm text-(--color-oa-ink) placeholder:text-(--color-oa-ink-dim)/45 outline-none"
      />
      <datalist id={props.listId}>
        <For each={props.suggestions}>{(s) => <option value={s} />}</For>
      </datalist>
    </div>
  );
};

/// Shared accent-bordered single-line text input (title / developer /
/// publisher / series) — keeps the editor's free-text fields visually
/// consistent with the typed controls. Pass `list` to attach a
/// <datalist> (rendered by the caller) for free-text autocomplete.
export const TextField: Component<{
  value?: string;
  placeholder?: string;
  list?: string;
  onInput: (raw: string) => void;
}> = (props) => (
  <input
    type="text"
    value={props.value ?? ""}
    placeholder={props.placeholder}
    list={props.list}
    onInput={(e) => props.onInput(e.currentTarget.value)}
    class={`${ACCENT_INPUT} w-full`}
  />
);

/// Auto-sizing-ish textarea for the description / overview field.
export const TextArea: Component<{
  value?: string;
  placeholder?: string;
  onInput: (raw: string) => void;
}> = (props) => (
  <textarea
    value={props.value ?? ""}
    placeholder={props.placeholder}
    rows={4}
    onInput={(e) => props.onInput(e.currentTarget.value)}
    class={`${ACCENT_INPUT} w-full resize-y leading-relaxed`}
  />
);
