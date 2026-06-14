// Spatial navigation engine — Pillar A of the Unified Navigation & Panel arc
// (docs/PLANS/unified-navigation-and-panels.md). ONE global navigator that:
//
//   1. Auto-discovers focusables via the NATIVE focusable set (no markers),
//      scoped to the top active "layer" container + visibility-filtered.
//   2. Moves by GEOMETRY — a direction press jumps to the nearest focusable in
//      that direction (spatialGeometry.ts scoring). Columns, rows, grids, tab
//      strips, mixed forms all work with zero per-surface config.
//   3. Acts by CONTROL TYPE on Confirm — reusing the Slice-1/2/3 dispatch:
//      checkbox/radio → flip, select → overlay picker, slider → adjust mode,
//      button/link/role=button → click. (Text entry → OSK, deferred.)
//   4. Scopes to the TOP LAYER — a stack of layers (the engine surface, then
//      any dialog / select overlay pushed on top) traps focus; popping restores.
//
// Coexistence: while a spatial layer is active, the legacy index-based focus
// manager (./focus) bypasses every event (it early-returns on isSpatialActive),
// so exactly ONE model handles each input. Surfaces not yet migrated keep using
// the index manager. The activate layer (overlay picker, slider adjust, Y-reset)
// + the input bus + back stack + HintBar + focus-ring CSS are all reused.
//
// The "minimum control set" invariant (plan §"Minimum control set") holds by
// construction here: every actionable element is a discovered focusable, so
// Direction→Confirm reaches it. X/Y/shoulders stay pure accelerators.

import {
  For,
  Show,
  createSignal,
  onCleanup,
  onMount,
  type Accessor,
  type Component,
  type JSX,
} from "solid-js";
import { Portal } from "solid-js/web";
import { hasBack, popBack, pushBackHandler } from "./back";
import { isNavEnabled, onNavEvent } from "./gamepad";
import { HintRegion } from "./HintBar";
import { isSwapAB, navBindings, resolveButtonVerb, resolveKeyVerb } from "./navBindings";
import { nextSliderValue } from "./sliderStep";
import {
  nearestTo,
  pickInDirection,
  toNavRect,
  type NavRect,
} from "./spatialGeometry";
import type { NavDirection } from "./types";
import { isDirectionVerb, type NavVerb } from "./verbs";

// --- Layer stack --------------------------------------------------------

type SpatialLayer = {
  id: string;
  containerRef: () => HTMLElement | undefined;
  /** Back/cancel for this layer (close the dialog, return to library, …). */
  onCancel?: () => void;
  /** Element to re-focus when this layer pops (the one focused below it). */
  restoreEl: HTMLElement | null;
};

const layers: SpatialLayer[] = [];
// Reactive layer-depth signal. isSpatialActive() reads it so both the
// event-handler bypass in ./focus AND reactive consumers (Dialog's branch)
// stay in sync.
const [layerCount, setLayerCount] = createSignal(0);

/// True when the spatial engine owns input (≥1 layer pushed). The legacy
/// index manager early-returns on this so the two never double-handle.
export function isSpatialActive(): boolean {
  return layerCount() > 0;
}

/// Reactive depth accessor (for consumers that want to track nesting).
export const spatialDepth: Accessor<number> = layerCount;

function topLayer(): SpatialLayer | null {
  return layers.length > 0 ? layers[layers.length - 1]! : null;
}
function topContainer(): HTMLElement | null {
  return topLayer()?.containerRef() ?? null;
}

// --- Focus tracking + adjust/picker state -------------------------------

let lastFocused: HTMLElement | null = null;
let lastRect: NavRect | null = null;

const [adjustEl, setAdjustEl] = createSignal<HTMLInputElement | null>(null);
const [pickerSelect, setPickerSelect] = createSignal<HTMLSelectElement | null>(null);

let disposeAdjustBack: (() => void) | null = null;

// --- Discovery ----------------------------------------------------------

const FOCUSABLE_SELECTOR = [
  "button",
  "a[href]",
  'input:not([type="hidden"])',
  "select",
  "textarea",
  '[tabindex]:not([tabindex="-1"])',
  '[role="button"]',
  '[role="tab"]',
].join(",");

function isNavigable(el: HTMLElement): boolean {
  if ((el as Partial<HTMLButtonElement>).disabled) return false;
  if (el.getAttribute("aria-disabled") === "true") return false;
  if (el.hasAttribute("data-nav-skip") || el.closest("[data-nav-skip]")) return false;
  // Hidden inside a collapsed <details>: the browser renders a layout box for
  // the content in some engines but refuses focus, which would trap movement
  // (a Down lands on an element that can't take focus, so the next Down
  // recomputes from the same origin forever). The summary itself stays
  // reachable. Open <details> content is fine.
  const closedDetails = el.closest("details:not([open])");
  if (closedDetails && !el.closest("summary")) return false;
  const rect = el.getBoundingClientRect();
  if (rect.width <= 0 || rect.height <= 0) return false;
  const style = getComputedStyle(el);
  if (style.visibility === "hidden" || style.display === "none") return false;
  return true;
}

/// Discover navigable focusables inside the top layer's container, in document
/// order. Scoped + filtered, so it's cheap to run on each direction press.
function discover(container: HTMLElement): HTMLElement[] {
  return Array.from(
    container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR),
  ).filter(isNavigable);
}

function rectItems(els: HTMLElement[]): Array<{ rect: NavRect; item: HTMLElement }> {
  return els.map((el) => ({ rect: toNavRect(el.getBoundingClientRect()), item: el }));
}

/// The navigation REGION an element belongs to — its nearest explicit
/// `[data-nav-region]` or semantic landmark (`aside`/`nav`/role), else the
/// layer container itself (the catch-all region). Regions implement the locked
/// nav model: UP/DOWN move WITHIN a region; LEFT/RIGHT cross BETWEEN regions at
/// an edge. Derived from existing markup (sidebars are already `<aside>`/`<nav>`)
/// so it needs no per-control wiring; `data-nav-region` is the override hook for
/// surfaces whose columns aren't landmarks (Pillar B's PanelScaffold will set it).
function regionOf(el: HTMLElement, container: HTMLElement): HTMLElement {
  const r = el.closest<HTMLElement>(
    '[data-nav-region], aside, nav, [role="navigation"], [role="tablist"]',
  );
  if (r && r !== container && container.contains(r)) return r;
  return container;
}

// --- Focus application --------------------------------------------------

/// Paint the engine's focus ring on `el` and record it as the current element.
/// Does NOT call the browser's `.focus()` — that's `setFocus`. Split out so the
/// `focusin` listener can sync the ring to a mouse/Tab focus change without
/// re-driving focus (which would recurse).
function paintFocus(el: HTMLElement): void {
  if (lastFocused && lastFocused !== el) {
    lastFocused.removeAttribute("data-oa-focus");
    lastFocused.removeAttribute("data-oa-focus-active");
  }
  el.setAttribute("data-oa-focus", "true");
  el.setAttribute("data-oa-focus-active", "true");
  lastFocused = el;
  lastRect = toNavRect(el.getBoundingClientRect());
}

function setFocus(el: HTMLElement): void {
  paintFocus(el);
  try {
    el.focus({ preventScroll: true });
  } catch {
    /* element may not be focusable in jsdom; attrs still drive the ring */
  }
  el.scrollIntoView({ block: "nearest", inline: "nearest" });
}

/// Focus the most sensible landing element in a freshly-activated layer:
/// an explicit `[data-nav-autofocus]`, else the first discovered focusable.
function focusFirst(layer: SpatialLayer): void {
  const c = layer.containerRef();
  if (!c) return;
  const items = discover(c);
  if (items.length === 0) return;
  const preferred = items.find((el) => el.hasAttribute("data-nav-autofocus"));
  setFocus(preferred ?? items[0]!);
}

/// The element the engine considers focused right now. The engine's own
/// `lastFocused` is the source of truth (kept in sync with mouse/Tab via the
/// `focusin` listener) — preferring it over `document.activeElement` is what
/// stops movement from getting pinned to a control that refused `.focus()`
/// (e.g. a hidden one), where the live activeElement would stay on the prior
/// element and every press would recompute from the same origin.
function focusedTarget(): HTMLElement | null {
  const c = topContainer();
  if (!c) return null;
  if (
    lastFocused &&
    c.contains(lastFocused) &&
    document.contains(lastFocused) &&
    isNavigable(lastFocused)
  ) {
    return lastFocused;
  }
  const active = document.activeElement as HTMLElement | null;
  if (active && active !== document.body && c.contains(active) && isNavigable(active)) {
    return active;
  }
  return null;
}

// --- Movement -----------------------------------------------------------

// Diagnostic flag — defaults OFF. Flip via `window.__oaSpatialDebug = true` in
// DevTools to log every move decision (direction, region, chosen target) to the
// unified log (console.* is captured into oa-current.log as
// frontend::oa-spatial). Kept compiled in — mirrors ./focus's FOCUS_DEBUG — so a
// future movement regression is diagnosable by flipping one flag.
const SPATIAL_DEBUG = (): boolean =>
  (globalThis as { __oaSpatialDebug?: boolean }).__oaSpatialDebug === true;

function describe(el: HTMLElement | null): string {
  if (!el) return "∅";
  const t = (el.textContent ?? "").trim().replace(/\s+/g, " ").slice(0, 28);
  const type = el instanceof HTMLInputElement ? `:${el.type}` : "";
  return `${el.tagName.toLowerCase()}${type}[${t}]`;
}

function move(dir: NavDirection): void {
  // Slider adjust mode repurposes directions to step the value.
  const adj = adjustEl();
  if (adj) {
    stepSlider(adj, dir);
    return;
  }
  const c = topContainer();
  if (!c) {
    if (SPATIAL_DEBUG()) console.log("[oa-spatial] move: no container", { dir });
    return;
  }
  const items = discover(c);
  if (items.length === 0) {
    if (SPATIAL_DEBUG()) console.log("[oa-spatial] move: 0 items", { dir });
    return;
  }

  let cur = focusedTarget();
  if (cur && !items.includes(cur)) cur = null;
  const all = rectItems(items);
  if (!cur) {
    // Lost focus (content swapped / detached). Keep going in the pressed
    // direction from where we were; fall back to the nearest focusable.
    const next =
      (lastRect ? pickInDirection(lastRect, all, dir) : null) ??
      (lastRect ? nearestTo(lastRect, all) : items[0]!);
    if (SPATIAL_DEBUG()) {
      console.log("[oa-spatial] move(recover)", {
        dir,
        items: items.length,
        next: describe(next),
      });
    }
    if (next) setFocus(next);
    return;
  }
  const curRect = toNavRect(cur.getBoundingClientRect());
  const region = regionOf(cur, c);
  const others = all.filter((x) => x.item !== cur);
  // Region-bias: candidates in the SAME region first.
  const inRegion = others.filter((x) => regionOf(x.item, c) === region);
  let next = pickInDirection(curRect, inRegion, dir);
  let crossed = false;
  // LEFT/RIGHT cross to an adjacent region when there's no in-region neighbour.
  // UP/DOWN deliberately do NOT cross (locked nav model) — that's what stopped
  // the dart from the Library sub-nav to the far-left settings categories.
  if (!next && (dir === "left" || dir === "right")) {
    const cross = others.filter((x) => regionOf(x.item, c) !== region);
    next = pickInDirection(curRect, cross, dir);
    crossed = next !== null;
  }
  if (SPATIAL_DEBUG()) {
    console.log("[oa-spatial] move", {
      dir,
      items: items.length,
      region: region === c ? "(layer)" : describe(region as HTMLElement),
      cur: describe(cur),
      next: describe(next),
      crossed,
    });
  }
  if (next) setFocus(next);
  // else: edge — no-op.
}

// --- Activation by control type -----------------------------------------

const TEXT_INPUT_TYPES = new Set([
  "text",
  "search",
  "email",
  "url",
  "tel",
  "password",
  "number",
  "",
]);
// Plain boolean (not a type predicate) — a predicate would narrow the FALSE
// branch of the `instanceof HTMLInputElement` block to `never`.
function isTextInput(el: HTMLElement | null): boolean {
  return el instanceof HTMLInputElement && TEXT_INPUT_TYPES.has(el.type);
}

function activate(): void {
  if (adjustEl()) {
    exitAdjust();
    return;
  }
  const el = focusedTarget();
  if (!el) return;
  if (el instanceof HTMLInputElement) {
    if (el.type === "checkbox" || el.type === "radio") {
      el.click();
      return;
    }
    if (el.type === "range") {
      enterAdjust(el);
      return;
    }
    if (isTextInput(el)) {
      // Text entry → the on-screen keyboard (deferred; see
      // docs/features/nav-coverage/OSK_PLAN.md). Focus it so a physical
      // keyboard can type in the meantime.
      el.focus();
      return;
    }
    el.click();
    return;
  }
  if (el instanceof HTMLSelectElement) {
    setPickerSelect(el);
    return;
  }
  el.click();
}

function cancel(): void {
  if (adjustEl()) {
    exitAdjust();
    return;
  }
  // Any component-pushed back handler (overlays/menus) wins first.
  if (popBack()) return;
  topLayer()?.onCancel?.();
}

/// Y / Tertiary accelerator — reset the focused row's overridden value. The
/// reset chip is itself a discovered focusable (so Direction→Confirm reaches it
/// too, satisfying the control-floor invariant); this is purely the shortcut.
function resetFocused(): void {
  const el = focusedTarget();
  if (!el) return;
  const row = el.closest("[data-setting-row]") ?? el;
  row.querySelector<HTMLElement>("[data-setting-reset]")?.click();
}

// --- Slider adjust mode -------------------------------------------------

function enterAdjust(el: HTMLInputElement): void {
  el.setAttribute("data-setting-adjusting", "true");
  setAdjustEl(el);
  disposeAdjustBack = pushBackHandler(exitAdjust);
}

function exitAdjust(): void {
  const el = adjustEl();
  if (el) el.removeAttribute("data-setting-adjusting");
  setAdjustEl(null);
  disposeAdjustBack?.();
  disposeAdjustBack = null;
}

function stepSlider(el: HTMLInputElement, dir: NavDirection): void {
  const d: 1 | -1 = dir === "up" || dir === "right" ? 1 : -1;
  const step = Number(el.step) || 1;
  const min = el.min === "" ? -Infinity : Number(el.min);
  const max = el.max === "" ? Infinity : Number(el.max);
  el.value = String(nextSliderValue(Number(el.value), step, min, max, d));
  el.dispatchEvent(new Event("input", { bubbles: true }));
}

// --- Layer push / pop ---------------------------------------------------

/// Push a spatial layer. Returns a disposer that pops it. The newly-pushed
/// layer becomes the active scope (focus traps inside its container); popping
/// restores focus to wherever it was below. Low-level — components use
/// `useSpatialLayer` (auto-pops on cleanup).
export function pushSpatialLayer(opts: {
  id: string;
  containerRef: () => HTMLElement | undefined;
  onCancel?: () => void;
}): () => void {
  const layer: SpatialLayer = { ...opts, restoreEl: lastFocused };
  layers.push(layer);
  setLayerCount(layers.length);
  // Defer initial focus to the next frame so the container's children have
  // mounted + laid out (covers portaled overlays whose DOM lands post-render).
  requestAnimationFrame(() => {
    if (topLayer() === layer) focusFirst(layer);
  });
  return () => {
    const i = layers.indexOf(layer);
    if (i < 0) return;
    const wasTop = i === layers.length - 1;
    layers.splice(i, 1);
    setLayerCount(layers.length);
    if (!wasTop) return;
    const prev = layer.restoreEl;
    requestAnimationFrame(() => {
      const t = topLayer();
      if (!t) return;
      const c = t.containerRef();
      if (prev && c && c.contains(prev) && document.contains(prev) && isNavigable(prev)) {
        setFocus(prev);
      } else {
        focusFirst(t);
      }
    });
  };
}

/// Solid hook for a top-level navigable surface (the engine takeover, a panel).
/// Pushes a layer for the component's lifetime and returns the overlay host JSX
/// (the select-picker portal) to render once inside the surface.
export function useSpatialLayer(opts: {
  id: string;
  containerRef: () => HTMLElement | undefined;
  onCancel?: () => void;
}): { overlay: JSX.Element } {
  const dispose = pushSpatialLayer(opts);
  onCleanup(dispose);
  return { overlay: <SpatialOverlays /> };
}

// --- Event routing (module-load, gated to isSpatialActive) ---------------

onNavEvent((event) => {
  if (!isSpatialActive()) return;
  if (event.kind === "button") {
    if (event.phase !== "down") return;
    const verb = resolveButtonVerb(event.button, navBindings(), isSwapAB());
    if (verb !== null) dispatchVerb(verb);
    return;
  }
  if (event.phase === "up") return;
  move(event.direction);
});

function dispatchVerb(verb: NavVerb): void {
  switch (verb) {
    case "Confirm":
      activate();
      return;
    case "Back":
      cancel();
      return;
    case "Tertiary":
      resetFocused();
      return;
    // Secondary / Menu / PrevSection / NextSection: no spatial action in
    // Phase 1 (pure accelerators — their effects route through focusable
    // affordances per the control-floor invariant).
    default:
      return;
  }
}

if (typeof window !== "undefined") {
  // Keep the engine's ring synced when focus moves by mouse / Tab (not via the
  // engine itself), so `focusedTarget` (which trusts `lastFocused`) follows.
  window.addEventListener("focusin", (e: FocusEvent) => {
    if (!isSpatialActive()) return;
    const t = e.target as HTMLElement | null;
    const c = topContainer();
    if (!t || !c || !c.contains(t) || t === lastFocused) return;
    if (!isNavigable(t)) return;
    paintFocus(t);
  });

  window.addEventListener("keydown", (e: KeyboardEvent) => {
    if (!isSpatialActive() || !isNavEnabled()) return;
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    const t = e.target as HTMLElement | null;
    const editable =
      !!t && (t.isContentEditable || t.tagName === "TEXTAREA" || isTextInput(t));

    // Escape: close the top overlay/adjust/dialog. When nothing nested is open
    // we let it bubble so EngineManagerSurface closes the whole takeover.
    if (e.key === "Escape") {
      if (adjustEl() || layers.length > 1 || pickerSelect() || hasBack()) {
        e.preventDefault();
        e.stopPropagation();
        cancel();
      }
      return;
    }

    // In a text field, leave chars + Enter to the field. Up/Down navigate OUT
    // (single-line fields rarely need them). Left/Right edit the caret UNLESS
    // it's already at the edge — then they navigate out (caret-edge escape), so
    // a focusable sitting beside an input (e.g. a per-field Reset button) is
    // reachable by keyboard too. Inputs that don't support selection (number)
    // report null → treated as at-edge so escape still works.
    if (editable) {
      if (e.key === "ArrowUp" || e.key === "ArrowDown") {
        e.preventDefault();
        move(e.key === "ArrowUp" ? "up" : "down");
        return;
      }
      if ((e.key === "ArrowLeft" || e.key === "ArrowRight") && t instanceof HTMLInputElement) {
        const len = t.value.length;
        const start = t.selectionStart;
        const end = t.selectionEnd;
        const atStart = start === null || (start === 0 && end === 0);
        const atEnd = start === null || (start === len && end === len);
        if ((e.key === "ArrowLeft" && atStart) || (e.key === "ArrowRight" && atEnd)) {
          e.preventDefault();
          move(e.key === "ArrowLeft" ? "left" : "right");
        }
      }
      return;
    }

    const verb = resolveKeyVerb(e.key, navBindings());
    if (verb && isDirectionVerb(verb)) {
      e.preventDefault();
      move(verb.toLowerCase() as NavDirection);
      return;
    }
    if (e.key.startsWith("Arrow")) {
      e.preventDefault();
      move(e.key.slice(5).toLowerCase() as NavDirection);
      return;
    }
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      activate();
      return;
    }
    if (e.key === "Backspace") {
      e.preventDefault();
      cancel();
    }
  });
}

// --- Overlay host + select picker ---------------------------------------

/// Rendered once per top-level surface (via useSpatialLayer). Shows the select
/// picker when a <select> Confirm opens one. Module-global state, so only the
/// root surface needs to host it.
const SpatialOverlays: Component = () => (
  <Show when={pickerSelect()}>
    {(sel) => (
      <SpatialSelectOverlay selectEl={sel()} onClose={() => setPickerSelect(null)} />
    )}
  </Show>
);

type SelectOption = { value: string; label: string; disabled: boolean };

/// Controller-navigable popup listbox mirroring a native <select>. It pushes a
/// NESTED spatial layer scoped to its own option buttons — so the SAME engine
/// navigates the options (up/down geometry, Confirm clicks). No bespoke nav.
const SpatialSelectOverlay: Component<{
  selectEl: HTMLSelectElement;
  onClose: () => void;
}> = (props) => {
  const options: SelectOption[] = Array.from(props.selectEl.options).map((o) => ({
    value: o.value,
    label: o.label || o.textContent || o.value,
    disabled: o.disabled,
  }));
  const currentValue = props.selectEl.value;
  let rootEl: HTMLElement | undefined;

  const pick = (o: SelectOption): void => {
    if (o.disabled) return;
    props.selectEl.value = o.value;
    props.selectEl.dispatchEvent(new Event("change", { bubbles: true }));
    props.onClose();
  };

  const rect = props.selectEl.getBoundingClientRect();
  const top = Math.min(rect.bottom + 4, window.innerHeight - 80);
  const style: JSX.CSSProperties = {
    left: `${rect.left}px`,
    top: `${top}px`,
    "min-width": `${Math.max(rect.width, 180)}px`,
  };

  const disposeLayer = pushSpatialLayer({
    id: "spatial-select-overlay",
    containerRef: () => rootEl,
    onCancel: () => props.onClose(),
  });
  const disposeBack = pushBackHandler(() => props.onClose());
  const onWindowDown = (e: MouseEvent): void => {
    const tgt = e.target as HTMLElement | null;
    if (!tgt || !tgt.closest("[data-spatial-select-root]")) props.onClose();
  };
  onMount(() => window.addEventListener("mousedown", onWindowDown, true));
  onCleanup(() => {
    window.removeEventListener("mousedown", onWindowDown, true);
    disposeBack();
    disposeLayer();
  });

  return (
    <Portal>
      <div
        data-spatial-select-root
        // z-[80]: above the engine surface (z-[60]) and platform dialogs
        // (z-[70]) — a select inside a dialog must open ON TOP of it.
        class="fixed z-[80] max-h-72 overflow-y-auto rounded-md border border-white/10 bg-(--color-oa-bg-deep)/95 text-sm shadow-2xl shadow-black/60 backdrop-blur"
        style={style}
        onClick={(e) => e.stopPropagation()}
      >
        <HintRegion hints={{ Confirm: "Pick", Back: "Close" }} />
        <ul class="py-1">
          <For each={options}>
            {(o) => (
              <li>
                <button
                  type="button"
                  disabled={o.disabled}
                  data-nav-autofocus={o.value === currentValue ? "true" : undefined}
                  class="flex w-full items-center justify-between gap-4 px-3 py-1.5 text-left transition hover:bg-white/[0.06] disabled:opacity-40"
                  onMouseEnter={(e) => setFocus(e.currentTarget)}
                  onClick={() => pick(o)}
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

// --- Dialog integration -------------------------------------------------

/// Mounted by the platform Dialog when the spatial engine is active (i.e. the
/// dialog opened from inside an engine takeover). Pushes a spatial layer scoped
/// to the dialog body so the SAME engine navigates it; B / Esc close. Replaces
/// the index-based DialogNavHandler in the spatial world.
export const SpatialDialogLayer: Component<{
  bodyRef: () => HTMLElement | undefined;
  onClose: () => void;
}> = (props) => {
  const dispose = pushSpatialLayer({
    id: "spatial-dialog",
    containerRef: props.bodyRef,
    onCancel: () => props.onClose(),
  });
  onCleanup(dispose);
  return <SpatialOverlays />;
};

/// Wraps a CUSTOM modal (one not built on the platform Dialog primitive — e.g.
/// the Import Wizard, the Game-media panel) in a spatial layer so the engine
/// navigates it. "Wire the scope ONCE into the container" — the children's
/// native controls are all discovered with no per-control markers. Mount it
/// INSIDE the modal's `<Show when={open}>` so the layer's lifetime matches the
/// modal's. The wrapper is `display:contents` (layout-neutral); discovery scopes
/// to it via DOM containment, so the modal's own `fixed`/`portal`-free overlay
/// is found. `onCancel` runs on B / Esc-when-nested.
export const SpatialModalScope: Component<{
  id: string;
  onCancel?: () => void;
  children: JSX.Element;
}> = (props) => {
  let ref: HTMLDivElement | undefined;
  const nav = useSpatialLayer({
    id: props.id,
    containerRef: () => ref,
    onCancel: props.onCancel,
  });
  return (
    <div ref={(el) => (ref = el)} style={{ display: "contents" }}>
      {props.children}
      {nav.overlay}
    </div>
  );
};
