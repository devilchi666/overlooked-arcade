// Developer tools — a dev-only panel in Settings → About. Toggle the various
// debug-logging streams on/off (they write to oa-current.log + the in-app Debug
// log viewer), open the WebView inspector, and reach the log file. Gated to
// dev/debug builds by the caller (import.meta.env.DEV).
//
// The logging flags are plain globals read live by their owners
// (platform/nav/spatial.tsx, platform/nav/focus.ts). Toggling writes the global
// and mirrors it into a local signal for the button state. Add a new entry to
// DEBUG_FLAGS to surface another stream — no other wiring needed.

import { createSignal, For, onMount, Show, type Component } from "solid-js";
import { spawnTestJob } from "@oa/platform/api/jobsApi";
import {
  getLogFilePath,
  getOaDataDir,
  openDevtools,
  restartApp,
  revealLogsFolder,
  setLogStreams,
} from "@oa/platform/api/shellApi";

type FlagDef = { key: string; label: string; hint: string };

/// Backend (Rust) log streams — toggled via a Tauri command that raises the
/// log level for the matching target prefixes on demand (the noisy logging in
/// these areas is Rust-side, so a JS global can't reach it).
type StreamDef = { id: string; label: string; hint: string };
const BACKEND_STREAMS: readonly StreamDef[] = [
  { id: "media", label: "Media sync", hint: "Art / metadata / hash sync + identify (oa_shell::media)." },
  { id: "audio", label: "Audio", hint: "Mixer / cpal sink / UI-sound playback (audio_player, oa_audio)." },
  { id: "render", label: "Renderer", hint: "wgpu pipeline / framebuffer (oa_render)." },
];

const DEBUG_FLAGS: readonly FlagDef[] = [
  {
    key: "__oaSpatialDebug",
    label: "Spatial nav",
    hint: "Log every directional move decision (region, target).",
  },
  {
    key: "__oaFocusDebug",
    label: "Focus (index) nav",
    hint: "Log the legacy index focus-group routing.",
  },
  {
    key: "__oaGamepadDebug",
    label: "Gamepad raw input",
    hint: "Log raw button / axis / HAT presses — controller-mapping work.",
  },
];

function readFlag(key: string): boolean {
  return (globalThis as Record<string, unknown>)[key] === true;
}

const DevToolsPanel: Component = () => {
  // One signal mirroring all flags; re-read on each toggle so the buttons
  // reflect external changes (e.g. set from the console) on next interaction.
  const [, setTick] = createSignal(0);
  const isOn = (key: string): boolean => readFlag(key);
  const toggle = (key: string): void => {
    (globalThis as Record<string, unknown>)[key] = !readFlag(key);
    setTick((n) => n + 1);
  };

  // Backend log streams — the active set is sent to Rust on every change.
  const [streams, setStreams] = createSignal<string[]>([]);
  const streamOn = (id: string): boolean => streams().includes(id);
  const toggleStream = (id: string): void => {
    const next = streamOn(id) ? streams().filter((s) => s !== id) : [...streams(), id];
    setStreams(next);
    void setLogStreams(next).catch((e) => setLogMsg(String(e)));
  };

  const [logMsg, setLogMsg] = createSignal<string>("");
  const [dataDir, setDataDir] = createSignal<string>("");
  onMount(() => {
    void getOaDataDir()
      .then(setDataDir)
      .catch(() => {});
  });

  const spawnJob = async (): Promise<void> => {
    try {
      await spawnTestJob(4);
      setLogMsg("Spawned a 4s test job — watch the background-jobs bar.");
    } catch (e) {
      setLogMsg(String(e));
    }
  };
  const reloadUi = (): void => location.reload();
  const restart = async (): Promise<void> => {
    try {
      await restartApp();
    } catch (e) {
      setLogMsg(String(e));
    }
  };

  const copyLogPath = async (): Promise<void> => {
    try {
      const p = await getLogFilePath();
      if (p) {
        await navigator.clipboard.writeText(p);
        setLogMsg(`Copied: ${p}`);
      } else {
        setLogMsg("No log file yet.");
      }
    } catch (e) {
      setLogMsg(String(e));
    }
  };
  const openLogs = async (): Promise<void> => {
    try {
      await revealLogsFolder();
    } catch (e) {
      setLogMsg(String(e));
    }
  };
  const openInspector = async (): Promise<void> => {
    try {
      await openDevtools();
    } catch (e) {
      setLogMsg(String(e));
    }
  };

  const actionBtn =
    "rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-[0.65rem] font-semibold uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)";

  return (
    <section class="rounded-xl border border-amber-400/20 bg-amber-400/[0.03] p-5">
      <div class="mb-3 flex items-center gap-2">
        <span class="text-[0.6rem] font-semibold uppercase tracking-[0.4em] text-amber-300/90">
          ⚒ Developer tools
        </span>
        <span class="rounded-full border border-amber-400/30 bg-amber-400/10 px-1.5 py-0.5 text-[0.5rem] uppercase tracking-widest text-amber-300/80">
          dev build
        </span>
      </div>

      {/* Logging toggles */}
      <p class="mb-2 text-[0.7rem] text-(--color-oa-ink-dim)">
        Logging streams (write to the debug log + <code class="font-mono">oa-current.log</code>):
      </p>
      <div class="flex flex-col gap-2">
        <For each={DEBUG_FLAGS}>
          {(flag) => (
            <div class="flex items-center justify-between gap-3 rounded-md border border-white/10 bg-white/[0.02] px-3 py-2">
              <div class="min-w-0">
                <p class="text-[0.8rem] font-semibold text-(--color-oa-ink)">{flag.label}</p>
                <p class="text-[0.65rem] text-(--color-oa-ink-dim)">{flag.hint}</p>
              </div>
              <button
                type="button"
                onClick={(e) => {
                  e.currentTarget.blur();
                  toggle(flag.key);
                }}
                aria-pressed={isOn(flag.key)}
                class="shrink-0 rounded-md border px-3 py-1.5 text-[0.65rem] font-semibold uppercase tracking-wider transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                classList={{
                  "border-emerald-400/40 bg-emerald-500/15 text-emerald-200": isOn(flag.key),
                  "border-white/10 bg-white/[0.04] text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)":
                    !isOn(flag.key),
                }}
              >
                {isOn(flag.key) ? "On" : "Off"}
              </button>
            </div>
          )}
        </For>
      </div>

      {/* Backend (Rust) log streams */}
      <p class="mb-2 mt-4 text-[0.7rem] text-(--color-oa-ink-dim)">
        Backend log streams (raise the Rust log level for that subsystem on demand):
      </p>
      <div class="flex flex-col gap-2">
        <For each={BACKEND_STREAMS}>
          {(s) => (
            <div class="flex items-center justify-between gap-3 rounded-md border border-white/10 bg-white/[0.02] px-3 py-2">
              <div class="min-w-0">
                <p class="text-[0.8rem] font-semibold text-(--color-oa-ink)">{s.label}</p>
                <p class="text-[0.65rem] text-(--color-oa-ink-dim)">{s.hint}</p>
              </div>
              <button
                type="button"
                onClick={(e) => {
                  e.currentTarget.blur();
                  toggleStream(s.id);
                }}
                aria-pressed={streamOn(s.id)}
                class="shrink-0 rounded-md border px-3 py-1.5 text-[0.65rem] font-semibold uppercase tracking-wider transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                classList={{
                  "border-emerald-400/40 bg-emerald-500/15 text-emerald-200": streamOn(s.id),
                  "border-white/10 bg-white/[0.04] text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)":
                    !streamOn(s.id),
                }}
              >
                {streamOn(s.id) ? "On" : "Off"}
              </button>
            </div>
          )}
        </For>
      </div>

      {/* Actions */}
      <div class="mt-4 flex flex-wrap gap-2">
        <button type="button" class={actionBtn} onClick={(e) => { e.currentTarget.blur(); void openInspector(); }}>
          Open inspector
        </button>
        <button type="button" class={actionBtn} onClick={(e) => { e.currentTarget.blur(); void spawnJob(); }}>
          Spawn test job
        </button>
        <button type="button" class={actionBtn} onClick={(e) => { e.currentTarget.blur(); void copyLogPath(); }}>
          Copy log path
        </button>
        <button type="button" class={actionBtn} onClick={(e) => { e.currentTarget.blur(); void openLogs(); }}>
          Open logs folder
        </button>
        <button type="button" class={actionBtn} onClick={(e) => { e.currentTarget.blur(); reloadUi(); }}>
          Reload UI
        </button>
        <button type="button" class={actionBtn} onClick={(e) => { e.currentTarget.blur(); void restart(); }}>
          Restart app
        </button>
      </div>
      <Show when={logMsg()}>
        <p class="mt-2 break-all text-[0.6rem] text-(--color-oa-ink-dim)">{logMsg()}</p>
      </Show>
      <Show when={dataDir()}>
        <p class="mt-2 break-all text-[0.6rem] text-(--color-oa-ink-dim)/70">
          Data dir: <code class="font-mono text-(--color-oa-ink-dim)">{dataDir()}</code>
        </p>
      </Show>
    </section>
  );
};

export default DevToolsPanel;
