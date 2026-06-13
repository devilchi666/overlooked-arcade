// Controller nav for settings-form bodies (Controller-Nav Coverage, Slice 1).
//
// The Settings center pane already had a `useDomQueryFocusGroup` with the
// default `"button"` selector + the sidebar hand-off wired — but the category
// bodies are almost entirely `SettingRow`s rendering <select>/<input
// type=range>/<input type=checkbox>, none of which are <button>s, so the group
// matched ~0 rows. This hook widens that group to match the rows themselves and
// teaches Confirm (A) to do the right thing per control type:
//
//   • toggle  → click the checkbox (flip)
//   • button  → click it
//   • select  → open a controller-navigable overlay picker (the proven
//               context-menu vertical-list pattern; up/down + Confirm)
//   • slider  → enter "adjust mode": up/right increment, down/left decrement,
//               Confirm/Back exit. Adjust mode is an explicitly-entered state,
//               so repurposing LEFT/RIGHT here does NOT violate the locked
//               nav spec (LEFT/RIGHT = region transfer in *normal* nav).
//
// Rows opt in via the `data-setting-row` marker on `SettingRow`'s root; stray
// action buttons that aren't `SettingRow`s opt in via `data-setting-action`.
// Embedded sub-pages (Library / Media / System Health) carry neither marker, so
// they contribute nothing here — their nav is a separate slice.

import {
  createSignal,
  For,
  onCleanup,
  onMount,
  Show,
  type Component,
  type JSX,
} from "solid-js";
import { Portal } from "solid-js/web";
import {
  captureFocusReturn,
  useDomQueryFocusGroup,
  useFocusGroup,
  type DomQueryFocusGroupApi,
} from "./focus";
import { pushBackHandler } from "./back";
import { HintRegion } from "./HintBar";

/// Selector for the rows + standalone action buttons the center group walks.
/// Explicit opt-in (no bare `button`) so embedded sub-pages aren't half-wired.
export const SETTING_ROW_SELECTOR = "[data-setting-row], [data-setting-action]";

/// One slider step in `dir` (+1 = increase) from `value`, clamped to
/// [min, max] and snapped to the step grid relative to `min` so repeated
/// gamepad steps don't accumulate float drift (e.g. 0.05-step bloom). `min`
/// / `max` may be ±Infinity when the input declares no bound. Pure — unit-
/// tested in settingsRowNav.test.ts.
export function nextSliderValue(
  value: number,
  step: number,
  min: number,
  max: number,
  dir: 1 | -1,
): number {
  let next = value + dir * step;
  if (Number.isFinite(min)) next = Math.max(min, next);
  if (Number.isFinite(max)) next = Math.min(max, next);
  if (Number.isFinite(min) && step > 0) {
    next = min + Math.round((next - min) / step) * step;
    if (Number.isFinite(max)) next = Math.min(max, next);
    next = Math.max(min, next);
  }
  return next;
}

export type SettingsRowFocusGroupOptions = {
  id: string;
  containerRef: () => HTMLElement | undefined;
  neighbours?: { left?: string; right?: string };
  /// When false the group registers without auto-claiming active on mount —
  /// for sibling-region pages where the sidebar is the landing surface.
  autoActivate?: boolean;
  /// Fires for B when not in adjust mode + no overlay open.
  onCancel?: () => void;
};

export type SettingsRowFocusGroupApi = DomQueryFocusGroupApi & {
  /// The select-overlay JSX. Render it once inside the consuming component.
  overlay: JSX.Element;
  /// True while a slider is in adjust mode (consuming directions).
  adjusting: () => boolean;
};

export function useSettingsRowFocusGroup(
  opts: SettingsRowFocusGroupOptions,
): SettingsRowFocusGroupApi {
  // The <select> currently driving the overlay picker (null = closed).
  const [picker, setPicker] = createSignal<HTMLSelectElement | null>(null);
  // The <input type=range> currently in adjust mode (null = not adjusting).
  const [adjustEl, setAdjustEl] = createSignal<HTMLInputElement | null>(null);

  let disposeAdjustBack: (() => void) | null = null;

  const exitAdjust = (): void => {
    const el = adjustEl();
    if (el) el.removeAttribute("data-setting-adjusting");
    setAdjustEl(null);
    disposeAdjustBack?.();
    disposeAdjustBack = null;
  };

  const enterAdjust = (el: HTMLInputElement): void => {
    el.setAttribute("data-setting-adjusting", "true");
    setAdjustEl(el);
    // Push BEFORE any outer Settings-close handler so B exits adjust first.
    disposeAdjustBack = pushBackHandler(exitAdjust);
  };

  const stepSlider = (el: HTMLInputElement, dir: 1 | -1): void => {
    const step = Number(el.step) || 1;
    const min = el.min === "" ? -Infinity : Number(el.min);
    const max = el.max === "" ? Infinity : Number(el.max);
    el.value = String(nextSliderValue(Number(el.value), step, min, max, dir));
    // SettingRow binds onInput; bubble a native input event so it fires.
    el.dispatchEvent(new Event("input", { bubbles: true }));
  };

  const group = useDomQueryFocusGroup({
    id: opts.id,
    containerRef: opts.containerRef,
    selector: SETTING_ROW_SELECTOR,
    orientation: "vertical",
    autoActivate: opts.autoActivate,
    neighbours: opts.neighbours,
    onActivate: (_i, el) => {
      // Confirm while adjusting commits + exits.
      if (adjustEl()) {
        exitAdjust();
        return;
      }
      if (el.matches("[data-setting-action]")) {
        el.click();
        return;
      }
      const checkbox = el.querySelector<HTMLInputElement>('input[type="checkbox"]');
      if (checkbox && !checkbox.disabled) {
        checkbox.click();
        return;
      }
      const select = el.querySelector<HTMLSelectElement>("select");
      if (select && !select.disabled) {
        setPicker(select);
        return;
      }
      const range = el.querySelector<HTMLInputElement>('input[type="range"]');
      if (range && !range.disabled) {
        enterAdjust(range);
        return;
      }
      // Fallback: a custom-widget row with its own button / link.
      const btn = el.querySelector<HTMLElement>("button, a[href]");
      btn?.click();
    },
    onTertiary: (_i, el) => {
      // Y resets the focused row to its inherited value — matches the panel's
      // `Tertiary: "Reset"` hint. Only SettingRows with an active override
      // render `[data-setting-reset]`, so this is a no-op elsewhere. Ignored
      // mid-adjust so Y can't reset a value the operator is actively editing.
      if (adjustEl()) return;
      el.querySelector<HTMLElement>("[data-setting-reset]")?.click();
    },
    onCancel: () => {
      // Adjust-mode B is caught by the pushed back handler before this; the
      // guard is belt-and-suspenders in case the stack ordering ever changes.
      if (adjustEl()) {
        exitAdjust();
        return;
      }
      opts.onCancel?.();
    },
    onDirection: (direction) => {
      const el = adjustEl();
      if (!el) return false; // normal nav: let the manager walk / transfer.
      if (direction === "up" || direction === "right") stepSlider(el, 1);
      else if (direction === "down" || direction === "left") stepSlider(el, -1);
      return true; // consume every direction while adjusting.
    },
  });

  onCleanup(exitAdjust);

  const overlay: JSX.Element = (
    <Show when={picker()}>
      {(select) => (
        <SettingSelectOverlay selectEl={select()} onClose={() => setPicker(null)} />
      )}
    </Show>
  );

  return {
    ...group,
    overlay,
    adjusting: () => adjustEl() !== null,
  };
}

// --- Select overlay -----------------------------------------------------
//
// A controller-navigable popup listbox mirroring the <select>'s options.
// Mirrors CorePickerMenu's structure (own focus group + captureFocusReturn +
// back handler + Escape / click-outside). Portaled to <body> so the panel's
// overflow / stacking context can't clip it.

type SelectOption = { value: string; label: string; disabled: boolean };

const SettingSelectOverlay: Component<{
  selectEl: HTMLSelectElement;
  onClose: () => void;
}> = (props) => {
  // Snapshot options + current value at open. A native <select> can't change
  // while the overlay owns input, so a snapshot is safe and avoids re-querying.
  const options: SelectOption[] = Array.from(props.selectEl.options).map((o) => ({
    value: o.value,
    label: o.label || o.textContent || o.value,
    disabled: o.disabled,
  }));
  const currentValue = props.selectEl.value;
  const initialIdx = Math.max(
    0,
    options.findIndex((o) => o.value === currentValue),
  );
  const [focusedIndex, setFocusedIndex] = createSignal(initialIdx);

  const pick = (i: number): void => {
    const o = options[i];
    if (!o || o.disabled) return;
    props.selectEl.value = o.value;
    // SettingRow binds onChange; bubble a native change event so it fires.
    props.selectEl.dispatchEvent(new Event("change", { bubbles: true }));
    props.onClose();
  };

  const group = useFocusGroup({
    id: "setting-select-overlay",
    orientation: "vertical",
    itemCount: () => options.length,
    focusedIndex,
    setFocusedIndex,
    onActivate: (i) => pick(i),
    onCancel: () => props.onClose(),
  });

  const rect = props.selectEl.getBoundingClientRect();
  // Clamp the top so a row near the viewport bottom doesn't push the menu
  // off-screen; the max-height + overflow handle the rest.
  const top = Math.min(rect.bottom + 4, window.innerHeight - 80);
  const style: JSX.CSSProperties = {
    left: `${rect.left}px`,
    top: `${top}px`,
    "min-width": `${Math.max(rect.width, 180)}px`,
  };

  const onWindowKey = (e: KeyboardEvent): void => {
    if (e.key === "Escape") props.onClose();
  };
  const onWindowDown = (e: MouseEvent): void => {
    const t = e.target as HTMLElement | null;
    if (!t || !t.closest("[data-setting-select-root]")) props.onClose();
  };

  const restore = captureFocusReturn();
  const disposeBack = pushBackHandler(() => props.onClose());
  onMount(() => {
    group.activate();
    window.addEventListener("keydown", onWindowKey, true);
    window.addEventListener("mousedown", onWindowDown, true);
  });
  onCleanup(() => {
    window.removeEventListener("keydown", onWindowKey, true);
    window.removeEventListener("mousedown", onWindowDown, true);
    disposeBack();
    restore();
  });

  return (
    <Portal>
      <div
        data-setting-select-root
        class="fixed z-[60] max-h-72 overflow-y-auto rounded-md border border-white/10 bg-(--color-oa-bg-deep)/95 text-sm shadow-2xl shadow-black/60 backdrop-blur"
        style={style}
        onClick={(e) => e.stopPropagation()}
      >
        <HintRegion hints={{ Confirm: "Pick", Back: "Close" }} />
        <ul class="py-1">
          <For each={options}>
            {(o, index) => (
              <li
                ref={(el) => group.bind(index(), el)}
                data-oa-focus={focusedIndex() === index() ? "true" : undefined}
                data-oa-focus-active={group.isActive() ? "true" : undefined}
              >
                <button
                  type="button"
                  disabled={o.disabled}
                  class="flex w-full items-center justify-between gap-4 px-3 py-1.5 text-left transition hover:bg-white/[0.06] disabled:opacity-40"
                  onMouseEnter={() => setFocusedIndex(index())}
                  onClick={() => pick(index())}
                >
                  <span class="text-(--color-oa-ink)">{o.label}</span>
                  <Show when={o.value === currentValue}>
                    <span class="shrink-0 text-[0.6rem] uppercase tracking-widest text-(--color-system-accent)">
                      current
                    </span>
                  </Show>
                </button>
              </li>
            )}
          </For>
        </ul>
      </div>
    </Portal>
  );
};
