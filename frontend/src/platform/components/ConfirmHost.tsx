// Single mount point for the in-app confirm dialog. Reads the head of the
// confirm queue (platform/lib/confirm) and renders it via the Dialog primitive
// (z-[70], above the engine surface; themeable; controller-navigable). Mounted
// once at the app root, next to ToastStack.
//
// Esc / B / backdrop / the X all cancel (Dialog's onClose → settleConfirm(false));
// Enter confirms; the confirm button auto-focuses so a gamepad A press confirms.

import { Show, createEffect, onCleanup, type Component } from "solid-js";
import { Dialog } from "./Dialog";
import { currentConfirm, settleConfirm } from "@oa/platform/lib/confirm";

const ConfirmHost: Component = () => {
  // Enter confirms the active request. Capture-phase so we win against any
  // page-level Enter handlers. Only armed while a request is pending.
  createEffect(() => {
    if (!currentConfirm()) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Enter") return;
      e.preventDefault();
      e.stopPropagation();
      settleConfirm(true);
    };
    window.addEventListener("keydown", onKey, { capture: true });
    onCleanup(() => window.removeEventListener("keydown", onKey, { capture: true }));
  });

  return (
    <Show when={currentConfirm()} keyed>
      {(req) => (
        <Dialog
          open={true}
          onClose={() => settleConfirm(false)}
          title={req.title ?? "Confirm"}
          system={req.system}
          size="sm"
        >
          <div class="flex flex-col gap-5">
            <p class="whitespace-pre-line text-sm leading-relaxed text-(--color-oa-ink)">
              {req.message}
            </p>
            <div class="flex justify-end gap-2">
              <button
                type="button"
                onClick={() => settleConfirm(false)}
                class="rounded-md border border-white/10 bg-white/[0.04] px-4 py-2 text-sm text-(--color-oa-ink) transition hover:bg-white/[0.08]"
              >
                {req.cancelLabel ?? "Cancel"}
              </button>
              <button
                type="button"
                ref={(el) => requestAnimationFrame(() => el.focus())}
                onClick={() => settleConfirm(true)}
                class={
                  "rounded-md px-4 py-2 text-sm font-semibold transition " +
                  (req.danger
                    ? "border border-red-400/40 bg-red-500/15 text-red-200 hover:bg-red-500/25"
                    : "border border-(--color-system-accent)/40 bg-(--color-system-accent)/20 text-(--color-oa-ink) hover:bg-(--color-system-accent)/30")
                }
              >
                {req.confirmLabel ?? "Confirm"}
              </button>
            </div>
          </div>
        </Dialog>
      )}
    </Show>
  );
};

export default ConfirmHost;
