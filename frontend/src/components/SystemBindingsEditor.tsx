import {
  createEffect,
  createResource,
  createSignal,
  For,
  onCleanup,
  onMount,
  Show,
  type Component,
} from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import {
  eventCodeToRustKey,
  firstPressedGamepadIndex,
  formatGamepadButton,
  formatKey,
  gamepadIndexToRustButton,
} from "../systems/keymap";
import type { SystemId } from "../themes/registry";
import AnalogBindingsSection from "./AnalogBindingsSection";

// Per-system input bindings editor.
//
// Extracted from the original SystemPage (which has been retired in favor
// of sidebar → filtered library) so the bindings UI can live on the
// PerSystemSettingsPage Input tab alongside the rest of the per-system
// settings. Same capture flow as before: click a cell, press a key (or
// gamepad button), the binding is saved + pushed to the running emu's
// poller. Right-click clears. Escape cancels capture.
//
// The component is system-agnostic — every state-tracking signal keys
// off `props.systemId` so the rest of the per-system page can stay flat.

type ButtonBinding = {
  button: string;
  keyboard: string | null;
  gamepad: string | null;
};

type CaptureKind = "keyboard" | "gamepad";
type Capture = { button: string; kind: CaptureKind };

type Props = {
  systemId: SystemId;
};

const SystemBindingsEditor: Component<Props> = (props) => {
  const [refreshKey, setRefreshKey] = createSignal(0);
  const [bindings] = createResource(
    () => ({ id: props.systemId, _: refreshKey() }),
    async (input): Promise<ButtonBinding[] | null> => {
      try {
        return await invoke<ButtonBinding[]>("get_bindings", { systemId: input.id });
      } catch (e) {
        console.warn("get_bindings failed:", e);
        return null;
      }
    },
  );

  const [capture, setCapture] = createSignal<Capture | null>(null);
  const [error, setError] = createSignal<string | null>(null);

  // Tell the emu thread to gate input + hotkeys while a capture is active.
  // Otherwise the user pressing F5 to bind it (or any direction key) would
  // also fire the hotkey on the running game underneath.
  createEffect(() => {
    const c = capture();
    invoke("set_ui_intercepting", { intercepting: c !== null }).catch((e) =>
      console.warn("set_ui_intercepting failed:", e),
    );
  });
  onCleanup(() => {
    // Belt-and-braces: release the intercept if the component unmounts mid-capture.
    invoke("set_ui_intercepting", { intercepting: false }).catch(() => {});
  });

  async function applyBinding(button: string, kind: CaptureKind, value: string | null) {
    try {
      const updated = await invoke<ButtonBinding[]>("set_binding", {
        systemId: props.systemId,
        button,
        kind,
        value,
      });
      setError(null);
      setRefreshKey((k) => k + 1);
      return updated;
    } catch (e) {
      const msg = String(e);
      console.warn("set_binding failed:", e);
      setError(msg);
      return null;
    }
  }

  async function handleReset() {
    try {
      await invoke("reset_bindings", { systemId: props.systemId });
      setError(null);
      setRefreshKey((k) => k + 1);
    } catch (e) {
      console.warn("reset_bindings failed:", e);
      setError(String(e));
    }
  }

  // Keyboard capture: listen for any keydown while a keyboard capture is active.
  // Escape cancels; Backspace/Delete unbinds; anything else binds (if the code
  // maps to a Rust Keycode).
  const captureKeyHandler = (e: KeyboardEvent) => {
    const c = capture();
    if (!c || c.kind !== "keyboard") return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      setCapture(null);
      return;
    }
    if (e.key === "Backspace" || e.key === "Delete") {
      void applyBinding(c.button, "keyboard", null).then(() => setCapture(null));
      return;
    }
    const rustName = eventCodeToRustKey(e.code);
    if (!rustName) {
      setError(`Key ${e.code} can't be mapped — try a different key.`);
      return;
    }
    void applyBinding(c.button, "keyboard", rustName).then(() => setCapture(null));
  };
  onMount(() => window.addEventListener("keydown", captureKeyHandler, { capture: true }));
  onCleanup(() => window.removeEventListener("keydown", captureKeyHandler, { capture: true }));

  // Gamepad capture loop. Esc still cancels — that's handled in captureKeyHandler.
  // Polls navigator.getGamepads() at ~60Hz while a gamepad capture is active.
  createEffect(() => {
    const c = capture();
    if (!c || c.kind !== "gamepad") return;
    let raf = 0;
    let cancelled = false;
    const tick = () => {
      if (cancelled) return;
      const idx = firstPressedGamepadIndex();
      if (idx !== null) {
        const rustName = gamepadIndexToRustButton(idx);
        if (!rustName) {
          setError(`Gamepad button ${idx} can't be mapped — try a different one.`);
        } else {
          void applyBinding(c.button, "gamepad", rustName).then(() => setCapture(null));
        }
        return;
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    onCleanup(() => {
      cancelled = true;
      if (raf) cancelAnimationFrame(raf);
    });
  });

  function cellLabel(c: Capture | null, button: string, kind: CaptureKind, current: string | null): string {
    if (c && c.button === button && c.kind === kind) {
      return kind === "keyboard" ? "press a key…" : "press a button…";
    }
    return kind === "keyboard" ? formatKey(current) : formatGamepadButton(current);
  }

  return (
    <article class="rounded-lg border border-white/5 bg-white/[0.03] p-5">
      <div class="flex items-center justify-between">
        <h2 class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
          Input mapping
        </h2>
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            void handleReset();
          }}
          class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
        >
          Reset to defaults
        </button>
      </div>
      <Show
        when={bindings()}
        fallback={
          <p class="mt-3 text-sm text-(--color-oa-ink-dim)">
            Loading bindings…
          </p>
        }
      >
        {(rows) => (
          <table class="mt-3 w-full text-left text-sm">
            <thead>
              <tr class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                <th class="py-1 font-medium">Button</th>
                <th class="py-1 font-medium">Keyboard</th>
                <th class="py-1 font-medium">Gamepad</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-white/5 text-(--color-oa-ink)">
              <For each={rows()}>
                {(r) => {
                  const isCapturing = (kind: CaptureKind) => {
                    const c = capture();
                    return c !== null && c.button === r.button && c.kind === kind;
                  };
                  const cellClass = (kind: CaptureKind) =>
                    [
                      "inline-flex min-w-[8rem] items-center justify-between gap-2 rounded border px-2 py-1 font-mono text-xs transition",
                      isCapturing(kind)
                        ? "border-(--color-system-accent) bg-(--color-system-accent)/10 text-(--color-system-accent)"
                        : "border-white/10 bg-white/[0.04] text-(--color-oa-ink) hover:bg-white/[0.08]",
                    ].join(" ");
                  return (
                    <tr>
                      <td class="py-1.5">
                        {/* Slice-polish — render the button name as a styled
                            chip using the system's accent color. Same visual
                            shape across PCE I/II, NES A/B, SNES A/B/X/Y,
                            Lynx Opt1/Opt2/Pause, etc. — the chip's accent
                            cascades from the per-system data-system on the
                            settings page wrapper. */}
                        <span class="inline-block rounded border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 px-2 py-0.5 font-mono text-xs uppercase tracking-wide text-(--color-system-accent-soft) min-w-[3rem] text-center">
                          {r.button}
                        </span>
                      </td>
                      <td class="py-1.5">
                        <button
                          type="button"
                          class={cellClass("keyboard")}
                          onClick={(e) => {
                            e.currentTarget.blur();
                            setCapture({ button: r.button, kind: "keyboard" });
                            setError(null);
                          }}
                          onContextMenu={(e) => {
                            e.preventDefault();
                            void applyBinding(r.button, "keyboard", null);
                          }}
                          title="Click to rebind. Right-click to clear."
                        >
                          <span>{cellLabel(capture(), r.button, "keyboard", r.keyboard)}</span>
                          <Show when={r.keyboard && !isCapturing("keyboard")}>
                            <span
                              role="button"
                              aria-label="Clear binding"
                              onClick={(e) => {
                                e.stopPropagation();
                                void applyBinding(r.button, "keyboard", null);
                              }}
                              class="text-[0.6rem] text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)"
                            >
                              ✕
                            </span>
                          </Show>
                        </button>
                      </td>
                      <td class="py-1.5">
                        <button
                          type="button"
                          class={cellClass("gamepad")}
                          onClick={(e) => {
                            e.currentTarget.blur();
                            setCapture({ button: r.button, kind: "gamepad" });
                            setError(null);
                          }}
                          onContextMenu={(e) => {
                            e.preventDefault();
                            void applyBinding(r.button, "gamepad", null);
                          }}
                          title="Click then press a gamepad button. Right-click to clear."
                        >
                          <span>{cellLabel(capture(), r.button, "gamepad", r.gamepad)}</span>
                          <Show when={r.gamepad && !isCapturing("gamepad")}>
                            <span
                              role="button"
                              aria-label="Clear binding"
                              onClick={(e) => {
                                e.stopPropagation();
                                void applyBinding(r.button, "gamepad", null);
                              }}
                              class="text-[0.6rem] text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)"
                            >
                              ✕
                            </span>
                          </Show>
                        </button>
                      </td>
                    </tr>
                  );
                }}
              </For>
            </tbody>
          </table>
        )}
      </Show>
      <p class="mt-4 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
        Click a cell, then press a key or gamepad button. Esc cancels; right-click or
        Backspace clears.
      </p>
      <Show when={error()}>
        <p class="mt-2 text-[0.7rem] text-red-300/80">{error()}</p>
      </Show>
      <AnalogBindingsSection systemId={props.systemId} mode="system" />
    </article>
  );
};

export default SystemBindingsEditor;
