// DragDropProbe — DEV-ONLY external drag-drop diagnostic (retry of the parked
// 2026-05-19 "external drag-drop file import" investigation; PARKING_LOT.md).
//
// WHY THIS EXISTS
// External OS→window drag-drop was marked "Won't fix" 2026-05-20 because neither
// shell mode delivered file paths and the root cause was "unclear without
// bisecting commits + WebView2 runtime + Tauri/wry releases." The 2026-06 research
// pass found the likely culprit: wry/tauri ship a documented bug where WebView2's
// external drop target is registered at window-creation time and SKIPPED when the
// window is built hidden (`.visible(false)`) — and showing it later does NOT
// re-register it. Our single-window builder is exactly that case
// (apps/oa-shell/src/main.rs `setup_single_window`: `.transparent(true)` +
// `.visible(false)`, shown later via the M1 `oa://window-shown` handshake).
//   - wry  #1639  "drag & drop doesn't work for windows created hidden until shown"
//   - tauri #14643 "Drag & Drop does not work for dynamically created windows when visible=false"
//   - fix is wry PR #1638 (touches SetAllowExternalDrop) — NOT in our wry 0.55.1.
//
// WHAT THIS PROBE DOES
// It does NOT try to fix anything. It instruments every layer that *could* carry a
// drop, live, so the operator can drag real files AND folders onto the window and
// read off exactly which layer fired and what data each delivered:
//   Layer 1 — DOM HTML5 (window dragenter/over/leave/drop). Fires inside Chromium.
//             dataTransfer.files never exposes a real OS path on WebView2, but the
//             event firing at all + file COUNT + webkitGetAsEntry().isDirectory tell
//             us whether the WebView even sees the drag.
//   Layer 2 — Tauri OS handler (getCurrentWebview().onDragDropEvent). THIS is the
//             one that carries real OS paths. If it stays silent / paths empty,
//             the hidden-window registration bug is confirmed on this machine.
//
// It also exposes a live show()/hide() cycle button to empirically test the
// "doesn't work until shown" hypothesis (reporters say a late show() doesn't help —
// confirm it here on our exact stack).
//
// Frontend-only, gated behind import.meta.env.DEV by the F10 host — never ships.
import { createSignal, For, onCleanup, onMount, Show } from "solid-js";
import type { JSX } from "solid-js";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";

type LogSource = "dom" | "tauri" | "ctl";
interface LogLine {
  n: number;
  t: number; // ms since probe mount
  source: LogSource;
  type: string;
  detail: string;
}

interface DomDrop {
  fileCount: number;
  names: string[];
  dirNames: string[]; // entries webkitGetAsEntry() reports as directories
  itemKinds: string[];
}

interface TauriDrop {
  paths: string[];
}

export default function DragDropProbe(): JSX.Element {
  const mountT = performance.now();
  let seq = 0;
  const [log, setLog] = createSignal<LogLine[]>([]);

  // Layer counters.
  const [domEnter, setDomEnter] = createSignal(0);
  const [domOver, setDomOver] = createSignal(0);
  const [domLeave, setDomLeave] = createSignal(0);
  const [domDropCount, setDomDropCount] = createSignal(0);
  const [tauriEnter, setTauriEnter] = createSignal(0);
  const [tauriOver, setTauriOver] = createSignal(0);
  const [tauriLeave, setTauriLeave] = createSignal(0);
  const [tauriDropCount, setTauriDropCount] = createSignal(0);

  // Last-drop payloads.
  const [lastDom, setLastDom] = createSignal<DomDrop | null>(null);
  const [lastTauri, setLastTauri] = createSignal<TauriDrop | null>(null);

  // Listener registration status.
  const [tauriListenerOk, setTauriListenerOk] = createSignal<boolean | null>(null);
  const [tauriListenerErr, setTauriListenerErr] = createSignal<string>("");
  const [visible, setVisible] = createSignal<boolean | null>(null);

  const push = (source: LogSource, type: string, detail: string): void => {
    seq += 1;
    const line: LogLine = {
      n: seq,
      t: Math.round(performance.now() - mountT),
      source,
      type,
      detail,
    };
    // Keep the last 40, newest first.
    setLog((prev) => [line, ...prev].slice(0, 40));
  };

  // ---- Layer 1: DOM HTML5 drag-drop -------------------------------------
  const onDragEnter = (e: DragEvent): void => {
    e.preventDefault();
    setDomEnter((n) => n + 1);
    push("dom", "dragenter", `items=${e.dataTransfer?.items?.length ?? 0}`);
  };
  const onDragOver = (e: DragEvent): void => {
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
    setDomOver((n) => n + 1);
  };
  const onDragLeave = (): void => {
    setDomLeave((n) => n + 1);
    push("dom", "dragleave", "");
  };
  const onDrop = (e: DragEvent): void => {
    e.preventDefault();
    setDomDropCount((n) => n + 1);
    const dt = e.dataTransfer;
    const files = dt?.files ? Array.from(dt.files) : [];
    const names = files.map((f) => `${f.name} (${f.size}B, "${f.type || "no-type"}")`);
    const itemKinds: string[] = [];
    const dirNames: string[] = [];
    if (dt?.items) {
      for (const item of Array.from(dt.items)) {
        itemKinds.push(`${item.kind}/${item.type || "?"}`);
        // webkitGetAsEntry is the only DOM-side way to tell file vs directory.
        const entry = (item as DataTransferItem & {
          webkitGetAsEntry?: () => { isDirectory?: boolean; name?: string } | null;
        }).webkitGetAsEntry?.();
        if (entry?.isDirectory) dirNames.push(entry.name ?? "(dir)");
      }
    }
    setLastDom({ fileCount: files.length, names, dirNames, itemKinds });
    push(
      "dom",
      "drop",
      `files=${files.length} dirs=${dirNames.length} items=[${itemKinds.join(", ")}]`,
    );
  };

  let unlistenTauri: (() => void) | undefined;

  onMount(async () => {
    window.addEventListener("dragenter", onDragEnter);
    window.addEventListener("dragover", onDragOver);
    window.addEventListener("dragleave", onDragLeave);
    window.addEventListener("drop", onDrop);

    // ---- Layer 2: Tauri OS-level drag-drop ------------------------------
    try {
      unlistenTauri = await getCurrentWebview().onDragDropEvent((event) => {
        const p = event.payload;
        switch (p.type) {
          case "enter":
            setTauriEnter((n) => n + 1);
            push("tauri", "enter", `paths=${(p.paths ?? []).length}`);
            break;
          case "over":
            setTauriOver((n) => n + 1);
            break;
          case "leave":
            setTauriLeave((n) => n + 1);
            push("tauri", "leave", "");
            break;
          case "drop": {
            setTauriDropCount((n) => n + 1);
            const paths = p.paths ?? [];
            setLastTauri({ paths });
            push("tauri", "drop", paths.length ? paths.join(" | ") : "EMPTY paths[]");
            break;
          }
        }
      });
      setTauriListenerOk(true);
      push("ctl", "listener", "onDragDropEvent registered OK");
    } catch (err) {
      setTauriListenerOk(false);
      setTauriListenerErr(String(err));
      push("ctl", "listener", `onDragDropEvent FAILED: ${String(err)}`);
    }

    try {
      setVisible(await getCurrentWindow().isVisible());
    } catch {
      /* ignore */
    }

    onCleanup(() => {
      window.removeEventListener("dragenter", onDragEnter);
      window.removeEventListener("dragover", onDragOver);
      window.removeEventListener("dragleave", onDragLeave);
      window.removeEventListener("drop", onDrop);
      unlistenTauri?.();
    });
  });

  const clear = (): void => {
    setLog([]);
    setDomEnter(0); setDomOver(0); setDomLeave(0); setDomDropCount(0);
    setTauriEnter(0); setTauriOver(0); setTauriLeave(0); setTauriDropCount(0);
    setLastDom(null); setLastTauri(null);
  };

  const tauriVerdict = (): string => {
    if (tauriDropCount() === 0) return "no drop seen yet";
    return (lastTauri()?.paths.length ?? 0) > 0 ? "PATHS DELIVERED ✓" : "fired but EMPTY paths ✗";
  };

  return (
    <div class="min-h-0 flex-1 overflow-y-auto pr-1">
      <p class="mb-4 max-w-3xl text-xs leading-relaxed text-white/55">
        Drag real <b class="text-white">files</b> and a <b class="text-white">folder</b> from
        Explorer onto this window. Two layers are instrumented: <b>DOM</b> (HTML5, fires inside
        Chromium — never exposes a real OS path on WebView2) and <b>Tauri</b>{" "}
        (<span class="font-mono">onDragDropEvent</span>, the OS handler that carries real paths).
        Expected on the current build: the Tauri card lights up{" "}
        <span class="font-mono">PATHS DELIVERED ✓</span> for both files and folders (the window is
        shown before drops arrive, so WebView2's drop target is registered). If it ever goes silent
        / empty again, suspect the wry hidden-window registration bug (#1639 / tauri#14643).
      </p>

      <div class="mb-4 flex flex-wrap items-center gap-2">
        <button class="rounded border border-white/20 px-3 py-1 text-xs hover:bg-white/10" onClick={clear}>
          Clear
        </button>
        <span class="font-mono text-[0.65rem] text-white/40">
          window.visible={visible() === null ? "?" : String(visible())} · listener=
          <Show when={tauriListenerOk() !== null} fallback="…">
            <span classList={{ "text-emerald-300": tauriListenerOk() === true, "text-rose-300": tauriListenerOk() === false }}>
              {tauriListenerOk() ? "ok" : "FAILED"}
            </span>
          </Show>
        </span>
      </div>
      <Show when={tauriListenerErr()}>
        <div class="mb-3 rounded border border-rose-500/40 bg-rose-500/10 px-3 py-2 font-mono text-[0.65rem] text-rose-200">
          {tauriListenerErr()}
        </div>
      </Show>

      {/* Layer verdict cards */}
      <div class="mb-4 grid grid-cols-1 gap-3 md:grid-cols-2">
        <div class="rounded-lg border border-white/10 bg-white/[0.03] p-3">
          <div class="mb-2 text-xs font-semibold text-white/80">Layer 1 — DOM HTML5</div>
          <div class="grid grid-cols-4 gap-1 font-mono text-[0.65rem] text-white/60">
            <span>enter {domEnter()}</span><span>over {domOver()}</span>
            <span>leave {domLeave()}</span><span class="text-cyan-300">drop {domDropCount()}</span>
          </div>
          <Show when={lastDom()} fallback={<div class="mt-2 text-[0.65rem] text-white/30">no drop yet</div>}>
            {(d) => (
              <div class="mt-2 space-y-0.5 font-mono text-[0.65rem] text-white/70">
                <div>files={d().fileCount} · dirs={d().dirNames.length}</div>
                <For each={d().names}>{(n) => <div class="truncate text-white/50">📄 {n}</div>}</For>
                <For each={d().dirNames}>{(n) => <div class="truncate text-amber-300/70">📁 {n}</div>}</For>
                <div class="text-white/30">items: {d().itemKinds.join(", ") || "—"}</div>
              </div>
            )}
          </Show>
        </div>

        <div class="rounded-lg border border-white/10 bg-white/[0.03] p-3">
          <div class="mb-2 text-xs font-semibold text-white/80">
            Layer 2 — Tauri OS handler{" "}
            <span class="font-mono text-[0.6rem] font-normal text-cyan-300">{tauriVerdict()}</span>
          </div>
          <div class="grid grid-cols-4 gap-1 font-mono text-[0.65rem] text-white/60">
            <span>enter {tauriEnter()}</span><span>over {tauriOver()}</span>
            <span>leave {tauriLeave()}</span><span class="text-cyan-300">drop {tauriDropCount()}</span>
          </div>
          <Show when={lastTauri()} fallback={<div class="mt-2 text-[0.65rem] text-white/30">no drop yet</div>}>
            {(d) => (
              <div class="mt-2 space-y-0.5 font-mono text-[0.65rem]">
                <Show when={d().paths.length} fallback={<div class="text-rose-300">drop fired but paths[] was EMPTY</div>}>
                  <For each={d().paths}>{(p) => <div class="truncate text-emerald-300/80">{p}</div>}</For>
                </Show>
              </div>
            )}
          </Show>
        </div>
      </div>

      {/* Raw event log */}
      <div class="rounded-lg border border-white/10 bg-black/40 p-2">
        <div class="mb-1 px-1 text-[0.6rem] font-semibold uppercase tracking-wide text-white/40">Event log (newest first)</div>
        <div class="max-h-64 overflow-y-auto font-mono text-[0.65rem] leading-relaxed">
          <For each={log()} fallback={<div class="px-1 text-white/30">— drag something onto the window —</div>}>
            {(l) => (
              <div class="flex gap-2 px-1">
                <span class="w-12 shrink-0 text-white/30">{l.t}ms</span>
                <span
                  class="w-12 shrink-0"
                  classList={{ "text-sky-300": l.source === "dom", "text-emerald-300": l.source === "tauri", "text-amber-300": l.source === "ctl" }}
                >
                  {l.source}
                </span>
                <span class="w-16 shrink-0 text-white/70">{l.type}</span>
                <span class="truncate text-white/50">{l.detail}</span>
              </div>
            )}
          </For>
        </div>
      </div>
    </div>
  );
}
