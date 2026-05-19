// Modal dialog primitive — backdrop, centered card, Esc + click-outside to
// close. Used by the Settings / System / Game / Tools menus when a menu
// item launches a focused configuration surface that wants more room than
// a popover affords.
//
// Width sizes: "sm" ~ 24rem (single-field dialogs), "md" ~ 32rem (default,
// most settings), "lg" ~ 48rem (denser forms or two-column layouts).

import {
  Show,
  onCleanup,
  onMount,
  type Component,
  type JSX,
} from "solid-js";

export type DialogSize = "sm" | "md" | "lg";

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
  /// Body content.
  children: JSX.Element;
};

const WIDTH_CLASS: Record<DialogSize, string> = {
  sm: "max-w-sm",
  md: "max-w-md",
  lg: "max-w-2xl",
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

  return (
    <Show when={props.open}>
      <div
        class="fixed inset-0 z-[55] grid place-items-center bg-black/55 backdrop-blur-sm"
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
          <header class="flex items-start justify-between border-b border-white/5 bg-(--color-system-accent)/10 px-5 py-3">
            <div class="min-w-0">
              <h2 class="truncate text-base font-semibold text-(--color-oa-ink)">
                {props.title}
              </h2>
              <Show when={props.subtitle}>
                <p class="mt-0.5 text-[0.6rem] uppercase tracking-[0.3em] text-(--color-oa-ink-dim)">
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
              class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-0.5 text-xs text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
              title="Close (Esc)"
              aria-label="Close"
            >
              ✕
            </button>
          </header>
          <div class="max-h-[70vh] overflow-y-auto px-5 py-4">{props.children}</div>
        </div>
      </div>
    </Show>
  );
};
