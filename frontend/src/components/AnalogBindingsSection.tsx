import {
  createEffect,
  createResource,
  createSignal,
  onCleanup,
  onMount,
  Show,
  type Component,
} from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { eventCodeToRustKey, formatKey } from "../systems/keymap";
import type { SystemId } from "../themes/registry";

// Per-system / per-game analog Bindings UI (Phase 2.5).
//
// Renders one panel per analog stick the system uses (n64 has 1,
// gamecube has 2, dreamcast / psp / saturn have 1, psx / ps2 have 2,
// everything else has 0). Each panel exposes:
//   - Gamepad source dropdown (Left / Right / None)
//   - 4 keyboard binding cells (Up / Down / Left / Right of the stick)
//   - Deadzone slider (0-50%)
//   - Sensitivity slider (0.5x-2.0x)
//   - Invert X / Invert Y toggles
//
// Plus, at section level: port selector (P1-P5) + stick-swap toggle
// (dual-stick systems) + Reset.
//
// `mode: "system"` writes to the per-system settings file. `mode: "game"`
// writes to GameOverrides — per-game changes layer on top of per-system
// at launch via `arm_analog_routing`. All changes push to the running emu
// thread so tuning takes effect mid-game.

type AnalogSticksInfo = {
  kind: "none" | "single" | "dual";
  leftLabel: string | null;
  rightLabel: string | null;
};

type AnalogStickPrefs = {
  gamepadSource: "left" | "right" | "none";
  keyboard: [string | null, string | null, string | null, string | null];
  deadzone: number;
  sensitivity: number;
  invertX: boolean;
  invertY: boolean;
};

type AnalogPortRouting = {
  left: AnalogStickPrefs;
  right: AnalogStickPrefs;
  stickSwap: boolean;
};

type AnalogRoutingPrefs = {
  ports: AnalogPortRouting[];
};

type SystemSettings = { analogRouting?: AnalogRoutingPrefs };
type GameOverrides = { analogRouting?: AnalogRoutingPrefs };

function defaultStick(source: "left" | "right"): AnalogStickPrefs {
  return {
    gamepadSource: source,
    keyboard: [null, null, null, null],
    deadzone: 0,
    sensitivity: 1.0,
    invertX: false,
    invertY: false,
  };
}

function identityPort(): AnalogPortRouting {
  return { left: defaultStick("left"), right: defaultStick("right"), stickSwap: false };
}

function portRouting(prefs: AnalogRoutingPrefs | undefined, port: number): AnalogPortRouting {
  if (!prefs) return identityPort();
  return prefs.ports[port] ?? identityPort();
}

type Props = {
  systemId: SystemId;
  /// "system" writes to per-system settings (settings page).
  /// "game" writes to GameOverrides (per-game drawer); requires `gameId`.
  mode: "system" | "game";
  gameId?: string;
};

const AnalogBindingsSection: Component<Props> = (props) => {
  const [info] = createResource(
    () => props.systemId,
    async (id): Promise<AnalogSticksInfo | null> => {
      try {
        return await invoke<AnalogSticksInfo>("analog_sticks_for_system", { systemId: id });
      } catch (e) {
        console.warn("analog_sticks_for_system failed:", e);
        return null;
      }
    },
  );

  const [activePort, setActivePort] = createSignal(0);
  const [prefs, setPrefs] = createSignal<AnalogRoutingPrefs>({ ports: [] });
  const [refreshKey, setRefreshKey] = createSignal(0);
  const [error, setError] = createSignal<string | null>(null);

  // Pull current state per mode. Game mode: get_game_overrides → analogRouting.
  // System mode: get_system_settings → analogRouting. Either way, store the
  // full Prefs blob; per-port resolution happens at render time.
  createResource(
    () => ({ id: props.systemId, gameId: props.gameId, mode: props.mode, _: refreshKey() }),
    async (input): Promise<void> => {
      try {
        if (input.mode === "game" && input.gameId) {
          const overrides = await invoke<GameOverrides>("get_game_overrides", { gameId: input.gameId });
          setPrefs(overrides.analogRouting ?? { ports: [] });
        } else {
          const settings = await invoke<SystemSettings>("get_system_settings", { systemId: input.id });
          setPrefs(settings.analogRouting ?? { ports: [] });
        }
      } catch (e) {
        console.warn("analog prefs fetch failed:", e);
      }
    },
  );

  // Keyboard-capture state for analog direction cells. Slot format:
  // "<port>_<side>_<dir>" — e.g. "0_left_up".
  const [capture, setCapture] = createSignal<string | null>(null);
  createEffect(() => {
    const c = capture();
    void invoke("set_ui_intercepting", { intercepting: c !== null }).catch(() => {});
  });
  onCleanup(() => {
    void invoke("set_ui_intercepting", { intercepting: false }).catch(() => {});
  });

  async function pushPort(port: number, routing: AnalogPortRouting) {
    // Update the local cache first for snappy UI.
    const next = { ports: [...prefs().ports] };
    while (next.ports.length <= port) next.ports.push(identityPort());
    next.ports[port] = routing;
    setPrefs(next);
    // Persist + push.
    try {
      if (props.mode === "game" && props.gameId) {
        await invoke("set_analog_routing_for_game", {
          gameId: props.gameId,
          port,
          routing,
        });
      } else {
        await invoke("set_analog_routing", {
          systemId: props.systemId,
          port,
          routing,
        });
      }
      setError(null);
    } catch (e) {
      console.warn("set_analog_routing failed:", e);
      setError(String(e));
    }
  }

  function updateStick(port: number, side: "left" | "right", patch: Partial<AnalogStickPrefs>) {
    const cur = portRouting(prefs(), port);
    const next: AnalogPortRouting = { ...cur, [side]: { ...cur[side], ...patch } };
    void pushPort(port, next);
  }

  function updatePort(port: number, patch: Partial<AnalogPortRouting>) {
    const cur = portRouting(prefs(), port);
    const next: AnalogPortRouting = { ...cur, ...patch };
    void pushPort(port, next);
  }

  const captureKeyHandler = (e: KeyboardEvent) => {
    const slot = capture();
    if (!slot) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      setCapture(null);
      return;
    }
    const isClear = e.key === "Backspace" || e.key === "Delete";
    const rustName = isClear ? null : eventCodeToRustKey(e.code);
    if (!isClear && !rustName) {
      setError(`Key ${e.code} can't be mapped — try a different key.`);
      return;
    }
    applyKeyboardSlot(slot, rustName);
    setCapture(null);
  };
  onMount(() => window.addEventListener("keydown", captureKeyHandler, { capture: true }));
  onCleanup(() => window.removeEventListener("keydown", captureKeyHandler, { capture: true }));

  function applyKeyboardSlot(slot: string, value: string | null) {
    const parts = slot.split("_");
    const port = parseInt(parts[0], 10);
    const side = parts[1] as "left" | "right";
    const dir = parts[2] as "up" | "down" | "left" | "right";
    const dirIndex = { up: 0, down: 1, left: 2, right: 3 }[dir];
    const cur = portRouting(prefs(), port);
    const stick = cur[side];
    const nextKeyboard: [string | null, string | null, string | null, string | null] = [
      stick.keyboard[0],
      stick.keyboard[1],
      stick.keyboard[2],
      stick.keyboard[3],
    ];
    nextKeyboard[dirIndex] = value;
    updateStick(port, side, { keyboard: nextKeyboard });
  }

  async function resetActivePort() {
    await pushPort(activePort(), identityPort());
    setRefreshKey((k) => k + 1);
  }

  const portLabel = (p: number) => `P${p + 1}`;

  return (
    <Show when={info() && info()!.kind !== "none"}>
      {(_) => {
        const currentPort = () => portRouting(prefs(), activePort());
        return (
          <article class="mt-4 rounded-lg border border-white/5 bg-white/[0.03] p-5">
            <div class="flex items-center justify-between flex-wrap gap-2">
              <h2 class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                Analog input
                <Show when={props.mode === "game"}>
                  {" "}<span class="text-(--color-system-accent)">· per-game</span>
                </Show>
              </h2>
              <div class="flex items-center gap-2">
                {/* Port selector */}
                <div class="flex items-center gap-1 rounded-md border border-white/10 bg-white/[0.04] p-0.5">
                  {[0, 1, 2, 3, 4].map((p) => (
                    <button
                      type="button"
                      onClick={(e) => {
                        e.currentTarget.blur();
                        setActivePort(p);
                      }}
                      class="rounded px-2 py-0.5 text-[0.6rem] uppercase tracking-wider transition"
                      classList={{
                        "bg-(--color-system-accent)/15 text-(--color-system-accent)": activePort() === p,
                        "text-(--color-oa-ink-dim) hover:bg-white/[0.06] hover:text-(--color-oa-ink)": activePort() !== p,
                      }}
                    >
                      {portLabel(p)}
                    </button>
                  ))}
                </div>
                <Show when={info()!.kind === "dual"}>
                  <label class="flex cursor-pointer items-center gap-2 rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)">
                    <input
                      type="checkbox"
                      checked={currentPort().stickSwap}
                      onChange={(e) => updatePort(activePort(), { stickSwap: e.currentTarget.checked })}
                      class="h-3 w-3"
                    />
                    Swap sticks
                  </label>
                </Show>
                <button
                  type="button"
                  onClick={(e) => {
                    e.currentTarget.blur();
                    void resetActivePort();
                  }}
                  class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                >
                  Reset {portLabel(activePort())}
                </button>
              </div>
            </div>

            <div class="mt-3 grid gap-4 sm:grid-cols-1 lg:grid-cols-2">
              <StickPanel
                port={activePort()}
                side="left"
                label={info()!.leftLabel ?? "Left Stick"}
                stick={currentPort().left}
                capture={capture}
                setCapture={setCapture}
                onChange={(patch) => updateStick(activePort(), "left", patch)}
              />
              <Show when={info()!.kind === "dual"}>
                <StickPanel
                  port={activePort()}
                  side="right"
                  label={info()!.rightLabel ?? "Right Stick"}
                  stick={currentPort().right}
                  capture={capture}
                  setCapture={setCapture}
                  onChange={(patch) => updateStick(activePort(), "right", patch)}
                />
              </Show>
            </div>

            <p class="mt-4 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Click a keyboard cell + press a key. Esc cancels; Backspace clears.
              Deadzone is radial — magnitudes below the threshold clamp to zero.
              Each port has its own routing — most operators only configure P1.
            </p>
            <Show when={error()}>
              <p class="mt-2 text-[0.7rem] text-red-300/80">{error()}</p>
            </Show>
          </article>
        );
      }}
    </Show>
  );
};

// One stick panel — keyboard cells, sliders, source dropdown, inverts.
const StickPanel: Component<{
  port: number;
  side: "left" | "right";
  label: string;
  stick: AnalogStickPrefs;
  capture: () => string | null;
  setCapture: (s: string | null) => void;
  onChange: (patch: Partial<AnalogStickPrefs>) => void;
}> = (props) => {
  const slotFor = (dir: "up" | "down" | "left" | "right") =>
    `${props.port}_${props.side}_${dir}`;
  const isCapturing = (dir: "up" | "down" | "left" | "right") =>
    props.capture() === slotFor(dir);
  const cellClass = (capturing: boolean) =>
    [
      "inline-flex min-w-[6rem] items-center justify-between gap-1 rounded border px-2 py-1 font-mono text-xs transition",
      capturing
        ? "border-(--color-system-accent) bg-(--color-system-accent)/10 text-(--color-system-accent)"
        : "border-white/10 bg-white/[0.04] text-(--color-oa-ink) hover:bg-white/[0.08]",
    ].join(" ");
  const KeyCell: Component<{ dir: "up" | "down" | "left" | "right"; idx: number }> = (k) => (
    <button
      type="button"
      class={cellClass(isCapturing(k.dir))}
      onClick={(e) => {
        e.currentTarget.blur();
        props.setCapture(slotFor(k.dir));
      }}
      onContextMenu={(e) => {
        e.preventDefault();
        const next: [string | null, string | null, string | null, string | null] = [
          props.stick.keyboard[0],
          props.stick.keyboard[1],
          props.stick.keyboard[2],
          props.stick.keyboard[3],
        ];
        next[k.idx] = null;
        props.onChange({ keyboard: next });
      }}
      title="Click to rebind. Right-click to clear."
    >
      <span class="uppercase tracking-wide text-[0.55rem] text-(--color-oa-ink-dim)">
        {k.dir}
      </span>
      <span>
        {isCapturing(k.dir) ? "press a key…" : formatKey(props.stick.keyboard[k.idx])}
      </span>
    </button>
  );

  return (
    <div class="rounded-md border border-white/5 bg-(--color-oa-bg-deep)/40 p-3">
      <h3 class="text-[0.6rem] font-semibold uppercase tracking-[0.3em] text-(--color-system-accent-soft)">
        {props.label}
      </h3>

      <div class="mt-2 flex items-center gap-2">
        <label class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim) min-w-[5rem]">
          Gamepad
        </label>
        <select
          value={props.stick.gamepadSource}
          onChange={(e) =>
            props.onChange({
              gamepadSource: e.currentTarget.value as "left" | "right" | "none",
            })
          }
          class="flex-1 rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) focus:border-(--color-system-accent) focus:outline-none"
        >
          <option value="left">Left stick</option>
          <option value="right">Right stick</option>
          <option value="none">None (keyboard only)</option>
        </select>
      </div>

      <div class="mt-3 grid grid-cols-3 gap-1 text-center">
        <div />
        <KeyCell dir="up" idx={0} />
        <div />
        <KeyCell dir="left" idx={2} />
        <div />
        <KeyCell dir="right" idx={3} />
        <div />
        <KeyCell dir="down" idx={1} />
        <div />
      </div>

      <div class="mt-3 flex items-center gap-2">
        <label class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim) min-w-[5rem]">
          Deadzone
        </label>
        <input
          type="range"
          min="0"
          max="0.5"
          step="0.01"
          value={props.stick.deadzone}
          onInput={(e) => props.onChange({ deadzone: parseFloat(e.currentTarget.value) })}
          class="flex-1 accent-(--color-system-accent)"
        />
        <span class="min-w-[2.5rem] text-right text-xs tabular-nums text-(--color-oa-ink-dim)">
          {Math.round(props.stick.deadzone * 100)}%
        </span>
      </div>

      <div class="mt-2 flex items-center gap-2">
        <label class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim) min-w-[5rem]">
          Sensitivity
        </label>
        <input
          type="range"
          min="0.5"
          max="2.0"
          step="0.05"
          value={props.stick.sensitivity}
          onInput={(e) => props.onChange({ sensitivity: parseFloat(e.currentTarget.value) })}
          class="flex-1 accent-(--color-system-accent)"
        />
        <span class="min-w-[2.5rem] text-right text-xs tabular-nums text-(--color-oa-ink-dim)">
          {props.stick.sensitivity.toFixed(2)}×
        </span>
      </div>

      <div class="mt-2 flex items-center gap-4">
        <label class="flex cursor-pointer items-center gap-2 text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)">
          <input
            type="checkbox"
            checked={props.stick.invertX}
            onChange={(e) => props.onChange({ invertX: e.currentTarget.checked })}
            class="h-3 w-3"
          />
          Invert X
        </label>
        <label class="flex cursor-pointer items-center gap-2 text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)">
          <input
            type="checkbox"
            checked={props.stick.invertY}
            onChange={(e) => props.onChange({ invertY: e.currentTarget.checked })}
            class="h-3 w-3"
          />
          Invert Y
        </label>
      </div>
    </div>
  );
};

export default AnalogBindingsSection;
