import { listen, type EventCallback } from "@tauri-apps/api/event";
import { onCleanup } from "solid-js";

// Subscribe to a Tauri event for the lifetime of the current Solid
// tracking scope. Bakes in onCleanup so callers don't need to track
// UnlistenFn by hand.
//
// Call site shape collapses from:
//   let un: UnlistenFn | undefined;
//   void listen(ch, h).then(u => (un = u)).catch(e => console.warn(...));
//   onCleanup(() => un?.());
// to:
//   listenScoped(ch, h);
//
// Race safety: if the scope tears down before listen() resolves, we
// flip `cancelled` and immediately unlisten as soon as the Promise
// completes — no leak.
//
// Must be called SYNCHRONOUSLY inside a Solid scope (onMount body, top
// level of a component setup, createEffect, etc.). Calling after an
// `await` loses the tracking owner and onCleanup won't bind correctly.
export function listenScoped<T>(channel: string, handler: EventCallback<T>): void {
  let cancelled = false;
  let unlisten: (() => void) | undefined;
  void listen<T>(channel, handler)
    .then((un) => {
      if (cancelled) {
        un();
      } else {
        unlisten = un;
      }
    })
    .catch((e) => {
      console.warn(`[listenScoped] subscribe to ${channel} failed:`, e);
    });
  onCleanup(() => {
    cancelled = true;
    unlisten?.();
  });
}
