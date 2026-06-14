// Modal dialog primitive — backdrop, centered card, Esc + click-outside to
// close. Used by the Settings / System / Game / Tools menus when a menu
// item launches a focused configuration surface that wants more room than
// a popover affords.
//
// Width sizes (UI_POLISH_PLAN.md §B.1):
//   sm  ~448px — single-field / confirm dialogs
//   md  ~576px — default; most settings dialogs
//   lg  ~768px — sectioned forms
//   xl  ~896px — content-rich (Display, Bindings, Properties)
//   2xl ~1024px — multi-column Properties surface

import {
  Show,
  createSignal,
  onCleanup,
  onMount,
  type Component,
  type JSX,
} from "solid-js";
import { useBackHandler } from "@oa/platform/nav";
import { captureFocusReturn, useFocusGroup, useSettingsRowFocusGroup } from "@oa/platform/nav";
import { HintRegion } from "@oa/platform/nav";

export type DialogSize = "sm" | "md" | "lg" | "xl" | "2xl";

// Monotonic counter so stacked dialogs each get a unique focus-group id
// in the manager's map. Solid's reactive lifecycle on a same-id remount
// would otherwise see two handles for one key.
let dialogIdCounter = 0;

/// Tiny helper component — for the lifetime of the open dialog:
///   - pushes the dialog's onClose onto the back stack (B closes)
///   - captures the previously active focus group so the cleanup
///     restores it (works across legacy + Retroverse without per-
///     consumer wiring)
///   - owns an inert focus group that CLAIMS active while the dialog is
///     up, so controller A/X/Y/Start presses don't leak through to
///     whichever surface was active before (e.g. the library grid
///     behind a Settings dialog would otherwise launch a tile on A).
///     itemCount stays 0 so direction events bail in routeEvent's count
///     guard; the button handlers are no-ops. B is consumed by the
///     back stack before reaching onCancel.
const DialogBackHandler: Component<{ onClose: () => void }> = (props) => {
  useBackHandler(() => props.onClose());
  const restoreFocus = captureFocusReturn();
  const [focusedIndex, setFocusedIndex] = createSignal(0);
  const dialogGroupId = `dialog-modal-${++dialogIdCounter}`;
  const group = useFocusGroup({
    id: dialogGroupId,
    orientation: "vertical",
    itemCount: () => 0,
    focusedIndex,
    setFocusedIndex,
    onActivate: () => {},
    onSecondary: () => {},
    onTertiary: () => {},
    onStart: () => {},
    onCancel: () => props.onClose(),
  });
  // Explicit activate — autoClaim only fires when the current active is
  // null or unregistered. Library-grid (or whichever surface launched
  // the dialog) is typically still registered, so we transfer
  // imperatively and let the cleanup restoreFocus put it back.
  onMount(() => group.activate());
  onCleanup(restoreFocus);
  return null;
};

/// Navigate variant of DialogBackHandler — for `<Dialog navigate>`. Instead of
/// the inert 0-item group, it mounts a real `useSettingsRowFocusGroup` over the
/// dialog body so a gamepad walks the body's SettingRows + `[data-setting-
/// action]` buttons with the Slice-1 vocabulary (toggle-flip / select-overlay /
/// slider-adjust / Y-reset). Same lifecycle guarantees as the inert handler:
/// captures + restores focus, B closes via the back stack (fires before the
/// group's onCancel). Free-text fields stay keyboard-only until the OSK lands
/// (docs/features/nav-coverage/OSK_PLAN.md). Renders the select-overlay host.
const DialogNavHandler: Component<{
  bodyRef: () => HTMLElement | undefined;
  onClose: () => void;
}> = (props) => {
  useBackHandler(() => props.onClose());
  const restoreFocus = captureFocusReturn();
  const groupId = `dialog-nav-${++dialogIdCounter}`;
  const nav = useSettingsRowFocusGroup({
    id: groupId,
    containerRef: props.bodyRef,
    autoActivate: false,
    onCancel: () => props.onClose(),
  });
  onMount(() => nav.activate());
  onCleanup(restoreFocus);
  return <>{nav.overlay}</>;
};

type Props = {
  open: boolean;
  onClose: () => void;
  title: string;
  /// Optional subtitle / hint shown under the title.
  subtitle?: string;
  /// Optional data-system attribute on the dialog root — drives the
  /// accent-color cascade for theme-aware styling.
  system?: string;
  /// Width preset. Default "md".
  size?: DialogSize;
  /// Opt into controller body navigation. When set, the dialog walks its body
  /// rows ([data-setting-row]) + action buttons ([data-setting-action]) with a
  /// real focus group instead of the inert input-trap. Default (unset) keeps
  /// the trap — correct for read-only dialogs (Help/About). See
  /// docs/features/nav-coverage/SLICE_2_PLAN.md.
  navigate?: boolean;
  /// Body content.
  children: JSX.Element;
};

const WIDTH_CLASS: Record<DialogSize, string> = {
  sm: "max-w-md",
  md: "max-w-xl",
  lg: "max-w-3xl",
  xl: "max-w-4xl",
  "2xl": "max-w-5xl",
};

export const Dialog: Component<Props> = (props) => {
  // Esc to close. Capture-phase so we win against page-level Esc handlers
  // (e.g. the App-level Quick Settings opener).
  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!props.open) return;
      if (e.key !== "Escape") return;
      e.preventDefault();
      e.stopPropagation();
      props.onClose();
    };
    window.addEventListener("keydown", onKey, { capture: true });
    onCleanup(() => window.removeEventListener("keydown", onKey, { capture: true }));
  });

  let bodyEl: HTMLElement | undefined;

  return (
    <Show when={props.open}>
      <Show
        when={props.navigate}
        fallback={<DialogBackHandler onClose={props.onClose} />}
      >
        <DialogNavHandler bodyRef={() => bodyEl} onClose={props.onClose} />
      </Show>
      <HintRegion
        hints={
          props.navigate
            ? { Confirm: "Select", Back: "Close", Tertiary: "Reset" }
            : { Back: "Close" }
        }
      />
      <div
        // z-[70]: platform-owned modal dialogs render ABOVE the engine
        // manager takeover (EngineManagerSurface, z-[60]) — Settings →
        // Help summons KeyboardShortcutsDialog / DebugLogDialog from
        // INSIDE the takeover, and at the pre-theming z-[55] they
        // opened invisibly behind the opaque surface. Matches
        // ResumePromptDialog + DiscPickerDialog (both already z-[70]).
        class="fixed inset-0 z-[70] grid place-items-center bg-black/55 backdrop-blur-sm"
        onClick={(e) => {
          // Only the backdrop closes — clicks inside the card stop here.
          if (e.currentTarget === e.target) props.onClose();
        }}
        role="dialog"
        aria-modal="true"
        aria-label={props.title}
        data-system={props.system}
      >
        <div
          class={
            "w-full overflow-hidden rounded-xl border border-white/10 bg-(--color-oa-bg-deep)/95 shadow-2xl shadow-black/60 " +
            WIDTH_CLASS[props.size ?? "md"]
          }
        >
          <header class="flex items-start justify-between border-b border-white/10 bg-(--color-system-accent)/15 px-6 py-4">
            <div class="min-w-0">
              <h2 class="truncate text-lg font-semibold text-(--color-oa-ink)">
                {props.title}
              </h2>
              <Show when={props.subtitle}>
                <p class="mt-0.5 text-xs text-(--color-oa-ink-dim)">
                  {props.subtitle}
                </p>
              </Show>
            </div>
            <button
              type="button"
              onClick={(e) => {
                e.currentTarget.blur();
                props.onClose();
              }}
              class="rounded-md border border-white/10 bg-white/[0.04] p-1.5 text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
              title="Close (Esc)"
              aria-label="Close"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="14"
                height="14"
                viewBox="0 0 14 14"
                fill="none"
                stroke="currentColor"
                stroke-width="1.6"
                stroke-linecap="round"
                aria-hidden="true"
              >
                <path d="M3 3l8 8M11 3l-8 8" />
              </svg>
            </button>
          </header>
          <div
            ref={(el) => (bodyEl = el)}
            class="max-h-[70vh] overflow-y-auto px-6 py-5"
          >
            {props.children}
          </div>
        </div>
      </div>
    </Show>
  );
};

// --- DialogSection ------------------------------------------------------
//
// Groups related rows inside a dialog body. Sibling sections get a top
// border separator; the first one has no border (first:border-0). Optional
// collapsibility via the `collapsible` prop; `id` is the anchor target for
// the future <DialogNavRail> (UI_POLISH_PLAN.md §B.4 — deferred).

type DialogSectionProps = {
  title: string;
  description?: string;
  collapsible?: boolean;
  defaultCollapsed?: boolean;
  /// Anchor id for future nav-rail scroll-spy.
  id?: string;
  children: JSX.Element;
};

export const DialogSection: Component<DialogSectionProps> = (props) => {
  const [collapsed, setCollapsed] = createSignal(props.defaultCollapsed ?? false);
  return (
    <section
      id={props.id}
      class="border-t border-white/5 pt-5 first:border-0 first:pt-0"
    >
      <header class="mb-3 flex items-baseline justify-between gap-3">
        <div class="min-w-0">
          <h3 class="text-sm font-semibold text-(--color-oa-ink)">
            {props.title}
          </h3>
          <Show when={props.description}>
            <p class="mt-1 text-xs leading-relaxed text-(--color-oa-ink-dim)">
              {props.description}
            </p>
          </Show>
        </div>
        <Show when={props.collapsible}>
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              setCollapsed((v) => !v);
            }}
            class="shrink-0 rounded-md border border-white/10 bg-white/[0.03] px-2 py-0.5 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim) transition hover:bg-white/[0.07] hover:text-(--color-oa-ink)"
            aria-expanded={!collapsed()}
          >
            {collapsed() ? "Show" : "Hide"}
          </button>
        </Show>
      </header>
      <Show when={!collapsed()}>
        <div class="flex flex-col gap-4">{props.children}</div>
      </Show>
    </section>
  );
};
