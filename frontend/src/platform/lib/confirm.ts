// In-app confirm dialog — imperative `confirm(message): Promise<boolean>`.
//
// Replaces native `window.confirm`, which is wrong here on two counts under
// Tauri 2:
//   1. It's intercepted + routed to `plugin:dialog|confirm`, gated by an ACL
//      (`dialog:allow-confirm`) — fragile, and throws an unhandled rejection
//      when a window's capability doesn't carry it.
//   2. It's ASYNC (returns a Promise), but every call site treated it as a
//      synchronous boolean (`if (!window.confirm(x))`). A Promise is truthy, so
//      the guard never fired → destructive actions ran WITHOUT waiting for the
//      operator's answer. This `confirm()` is awaitable and returns a real
//      boolean.
//
// Bonus: it's an in-app modal (the Dialog primitive), so it's themeable +
// controller-navigable — native OS dialogs can't be driven by a gamepad, which
// matters for kiosk / controller use. Mirrors the toast pattern: helper here,
// a single `<ConfirmHost />` mounted once at the app root renders the queue.

import { createSignal } from "solid-js";

export type ConfirmOptions = {
  /// Dialog title. Default "Confirm".
  title?: string;
  /// Confirm button label. Default "Confirm".
  confirmLabel?: string;
  /// Cancel button label. Default "Cancel".
  cancelLabel?: string;
  /// Style the confirm button as destructive (red) — for deletes / cancels.
  danger?: boolean;
  /// `data-system` for the accent-color cascade (theme-aware styling).
  system?: string;
};

export type ConfirmRequest = ConfirmOptions & {
  id: number;
  message: string;
  resolve: (ok: boolean) => void;
};

const [queue, setQueue] = createSignal<ConfirmRequest[]>([]);
let nextId = 1;

/// The confirm at the head of the queue (what `<ConfirmHost />` renders), or
/// null when nothing is pending.
export const currentConfirm = (): ConfirmRequest | null => queue()[0] ?? null;

/// Ask the operator to confirm an action. Resolves `true` on confirm, `false`
/// on cancel / dismiss (Esc, B, backdrop, the X). Await it:
///   if (!(await confirm("Delete this?"))) return;
export function confirm(message: string, opts: ConfirmOptions = {}): Promise<boolean> {
  return new Promise<boolean>((resolve) => {
    setQueue((q) => [...q, { id: nextId++, message, resolve, ...opts }]);
  });
}

/// Resolve the head request + advance the queue. Called by `<ConfirmHost />`.
export function settleConfirm(ok: boolean): void {
  const head = queue()[0];
  if (!head) return;
  setQueue((q) => q.slice(1));
  head.resolve(ok);
}
