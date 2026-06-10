import { createSignal, For, onCleanup, type Component } from "solid-js";
import { listenScoped, OA_EVENTS } from "@oa/platform/api/eventsApi";

// Mirror of the Rust ToastPayload struct in oa-shell/src/main.rs. `system`
// is optional — when present it picks up that system's CSS cascade colors
// via [data-system="<id>"] in themes/systems.css.
type ToastLevel = "info" | "success" | "warn" | "error";
type ToastPayload = {
  level: ToastLevel;
  message: string;
  system?: string;
};

type Toast = { id: number; payload: ToastPayload };

// Errors stay around longer than info/success/warn — the operator needs more
// time to read "ROM rejected: …" than "Saved slot 0".
const DISMISS_MS: Record<ToastLevel, number> = {
  info:    2500,
  success: 2500,
  warn:    2500,
  error:   4000,
};

const STACK_LIMIT = 4;

const GLYPH: Record<ToastLevel, string> = {
  info:    "•", // •
  success: "✓", // ✓
  warn:    "⚠", // ⚠
  error:   "✕", // ✕
};

const ToastStack: Component = () => {
  const [toasts, setToasts] = createSignal<Toast[]>([]);
  let nextId = 1;
  const timers = new Map<number, number>();

  const cancelTimer = (id: number) => {
    const h = timers.get(id);
    if (h !== undefined) {
      window.clearTimeout(h);
      timers.delete(id);
    }
  };

  const dismiss = (id: number) => {
    cancelTimer(id);
    setToasts((prev) => prev.filter((t) => t.id !== id));
  };

  const push = (payload: ToastPayload) => {
    const id = nextId++;
    setToasts((prev) => {
      const next = [...prev, { id, payload }];
      // Evict oldest if we're over the visible cap. We cancel their dismissal
      // timer so it doesn't fire on a stale id.
      while (next.length > STACK_LIMIT) {
        const evicted = next.shift()!;
        cancelTimer(evicted.id);
      }
      return next;
    });
    const handle = window.setTimeout(() => dismiss(id), DISMISS_MS[payload.level]);
    timers.set(id, handle);
  };

  listenScoped<ToastPayload>(OA_EVENTS.toast, (e) => push(e.payload));

  onCleanup(() => {
    timers.forEach((h) => window.clearTimeout(h));
    timers.clear();
  });

  return (
    // pointer-events-none on the stack root + each toast so clicks pass
    // straight through to the library / game underneath. No close X — toasts
    // are auto-dismiss only, by design (see ToastStack architecture notes).
    <div class="pointer-events-none fixed bottom-3 right-4 z-30 flex flex-col-reverse items-end gap-2">
      <For each={toasts()}>
        {(t) => (
          <div
            class="oa-toast pointer-events-none"
            classList={{
              "oa-toast-info":    t.payload.level === "info",
              "oa-toast-success": t.payload.level === "success",
              "oa-toast-warn":    t.payload.level === "warn",
              "oa-toast-error":   t.payload.level === "error",
            }}
            data-system={t.payload.system}
            role="status"
            aria-live="polite"
          >
            <span class="oa-toast-glyph" aria-hidden="true">
              {GLYPH[t.payload.level]}
            </span>
            <span class="oa-toast-msg">{t.payload.message}</span>
          </div>
        )}
      </For>
    </div>
  );
};

export default ToastStack;
