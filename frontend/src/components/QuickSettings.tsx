import {
  createEffect,
  createSignal,
  For,
  onCleanup,
  onMount,
  Show,
  type Component,
  type JSX,
} from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import type { RomEntry } from "../library/types";
import { systemThemes } from "../themes/registry";
import type { SettingsStore } from "../settings/store";

// Phase 2.8 slice B (Quick Settings overlay) + Phase 4 slice B (rewind
// scrubbing). Triggered by Escape during gameplay. Two-mode card:
//
//   1. "actions" — the original Resume / Save&Load / Game info / Rewind /
//      Scaling / Shader / All settings / Exit grid.
//   2. "rewind" — swaps the action list for a timeline strip + commit /
//      cancel buttons. Entered via clicking the Rewind action; exited via
//      Commit (truncates ring above scrub position) or Cancel (restores
//      the live edge). The Rust emu thread is in scrub mode for the whole
//      lifetime of view === "rewind".
//
// Closing the overlay (Esc, backdrop) while in rewind view auto-cancels
// the scrub before the overlay tears down so we never leave the emu
// thread frozen in scrub mode with no UI driving it.

// Public view ids — exported so the parent can request the overlay to
// open pre-set on a specific drill-in (Tools ▾ menu items).
export type QuickSettingsView = "actions" | "rewind" | "tas" | "video" | "memory" | "disc";

type Props = {
  open: boolean;
  onClose: () => void;
  entry: RomEntry | null;
  onShowSaves: (entry: RomEntry) => void;
  onShowInfo: (entry: RomEntry) => void;
  onExitToLibrary: () => void;
  settings: SettingsStore;
  /// Optional landing view on open. Defaults to "actions". Set by the
  /// Tools menu items to drop the user directly on the panel they asked
  /// for instead of forcing them through the action grid.
  requestedView?: QuickSettingsView | null;
};

const BTN_BASE =
  "flex w-full items-center gap-3 rounded-md border border-white/10 bg-white/[0.04] px-4 py-2.5 text-left text-sm font-medium text-(--color-oa-ink) transition hover:bg-white/[0.08] focus-visible:border-(--color-system-accent) focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-40";

// Shape returned by the Rust `get_rewind_state` Tauri command. Matches
// `SharedRewindState` in apps/oa-shell/src/main.rs (camelCase serde).
type RewindState = {
  enabled: boolean;
  snapshotCount: number;
  byteSize: number;
  captureIntervalFrames: number;
  fps: number;
  scrubbing: boolean;
  scrubPosition: number;
};

// Phase 4 slice C — TAS state shape returned by `get_tas_state`.
// `mode` is the Rust enum's lowercase serde-rename.
type TasMode = "idle" | "recording" | "replaying";
type TasState = {
  mode: TasMode;
  frame: number;
  totalFrames: number;
  displayName: string;
};

type TasListEntry = {
  filePath: string;
  displayName: string;
  recordedAtUnixMs: number;
  frameCount: number;
  fps: number;
  durationSeconds: number;
};

// Phase 4 slice D — video capture state.
type VideoState = {
  capturing: boolean;
  frameCount: number;
  droppedFrameCount: number;
  displayName: string;
  clipDir: string;
};

type VideoClipEntry = {
  clipDir: string;
  displayName: string;
  recordedAtUnixMs: number;
  frameCount: number;
  droppedFrameCount: number;
  fps: number;
  width: number;
  height: number;
  durationSeconds: number;
};

// Phase 4 slice E — memory inspector.
type MemoryRegionId = "save_ram" | "rtc" | "system_ram" | "video_ram";
type MemoryRegionInfo = {
  region: string;
  available: boolean;
  totalSize: number;
  offset: number;
  bytes: number[];
};

type QuickView = "actions" | "rewind" | "tas" | "video" | "memory" | "disc";

type DiscInfo = {
  numDiscs: number;
  currentIndex: number;
  ejected: boolean;
  labels: string[];
};

const MEMORY_WINDOW_BYTES = 256;

const QuickSettings: Component<Props> = (props) => {
  let cardRef: HTMLDivElement | undefined;
  const [view, setView] = createSignal<QuickView>("actions");
  const [rewindState, setRewindState] = createSignal<RewindState | null>(null);
  // scrubPosition is OUR view of where the head is. The Rust side clamps
  // and confirms via the next get_rewind_state read, but we update the UI
  // synchronously off the pointer events so dragging feels live.
  const [scrubPosition, setScrubPosition] = createSignal(0);
  const [tasState, setTasState] = createSignal<TasState | null>(null);
  const [tasRecordings, setTasRecordings] = createSignal<TasListEntry[]>([]);
  const [tasRecordingName, setTasRecordingName] = createSignal("");
  // Poll TAS state while the TAS view is open + during gameplay so the
  // "frame X / Y" status row stays live. setInterval ID stored so we can
  // clear on view change / overlay close.
  let tasPollId: number | undefined;

  // Phase 4 slice D — video capture.
  const [videoState, setVideoState] = createSignal<VideoState | null>(null);
  const [videoClips, setVideoClips] = createSignal<VideoClipEntry[]>([]);
  const [videoCaptureName, setVideoCaptureName] = createSignal("");
  let videoPollId: number | undefined;

  // Phase 4 slice E — memory inspector.
  const [memoryRegion, setMemoryRegion] = createSignal<MemoryRegionId>("system_ram");
  const [memoryOffset, setMemoryOffset] = createSignal(0);
  const [memoryView, setMemoryView] = createSignal<MemoryRegionInfo | null>(null);
  let memoryPollId: number | undefined;

  // RetroArch parity slice — disc control.
  const [discInfo, setDiscInfo] = createSignal<DiscInfo | null>(null);
  const [discSwapping, setDiscSwapping] = createSignal(false);
  async function refreshDiscState() {
    try {
      const s = await invoke<DiscInfo | null>("get_disc_state");
      setDiscInfo(s);
    } catch (e) {
      console.warn("[oa-quick] get_disc_state failed:", e);
    }
  }
  /// Run the canonical eject → set_image → close sequence. Cores resume
  /// reading from the new disc on the next frame. Disabled while another
  /// swap is in flight so we don't interleave protocol steps.
  async function swapToDisc(index: number) {
    if (discSwapping()) return;
    setDiscSwapping(true);
    try {
      await invoke("set_disc_eject", { ejected: true });
      await invoke("set_disc_image", { index });
      await invoke("set_disc_eject", { ejected: false });
      await refreshDiscState();
    } catch (e) {
      console.warn("[oa-quick] swapToDisc failed:", e);
    } finally {
      setDiscSwapping(false);
    }
  }

  // Esc closes. Capture-phase so we win against the App.tsx-level Esc
  // handler. While in rewind view, Esc cancels the scrub first.
  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!props.open) return;
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        if (view() === "rewind") {
          // Cancel restores history; close overlay too.
          void invoke("end_rewind_scrub", { commit: false }).catch(() => {});
          setView("actions");
        }
        props.onClose();
      }
    };
    window.addEventListener("keydown", onKey, { capture: true });
    onCleanup(() => window.removeEventListener("keydown", onKey, { capture: true }));
  });

  // ui_intercepting gates gameplay input + the F5/F8/digit hotkeys for the
  // overlay's lifetime. Also reset view to "actions" + clear scrub state
  // when the overlay closes, so a re-open doesn't land in rewind by accident.
  createEffect(() => {
    if (props.open) {
      void invoke("set_ui_intercepting", { intercepting: true }).catch(() => {});
      // Hydrate ring stats + TAS state so the action-row hints reflect
      // current Rust-side state. Both are non-blocking.
      void invoke<RewindState>("get_rewind_state")
        .then((s) => setRewindState(s ?? null))
        .catch(() => setRewindState(null));
      void invoke<TasState>("get_tas_state")
        .then((s) => setTasState(s ?? null))
        .catch(() => setTasState(null));
      void invoke<VideoState>("get_video_state")
        .then((s) => setVideoState(s ?? null))
        .catch(() => setVideoState(null));
      void refreshDiscState();
      // Tools-menu deep-link: open straight on the requested panel. Each
      // view-specific enter helper does its own hydration (recordings,
      // clip list, scrub start, …) — calling them here keeps the
      // landing-by-link path equivalent to the action-row click path.
      const req = props.requestedView ?? null;
      if (req === "rewind") void enterRewindView();
      else if (req === "tas") void enterTasView();
      else if (req === "video") void enterVideoView();
      else if (req === "memory") setView("memory");
      else if (req === "disc") setView("disc");
      requestAnimationFrame(() => {
        if (!cardRef) return;
        const firstBtn = cardRef.querySelector<HTMLButtonElement>(
          "button[data-quick-action]:not(:disabled)",
        );
        firstBtn?.focus();
      });
    } else {
      void invoke("set_ui_intercepting", { intercepting: false }).catch(() => {});
      // If we got closed while in rewind view (overlay close path that
      // doesn't go through Esc), cancel the scrub server-side. Safe to
      // call when not scrubbing — the Rust side ignores the message.
      if (view() === "rewind") {
        void invoke("end_rewind_scrub", { commit: false }).catch(() => {});
      }
      // Stop polling TAS state when the overlay closes. Recording /
      // replay themselves keep going server-side — only the UI poll
      // pauses.
      if (tasPollId !== undefined) {
        window.clearInterval(tasPollId);
        tasPollId = undefined;
      }
      if (videoPollId !== undefined) {
        window.clearInterval(videoPollId);
        videoPollId = undefined;
      }
      if (memoryPollId !== undefined) {
        window.clearInterval(memoryPollId);
        memoryPollId = undefined;
      }
      setView("actions");
      setScrubPosition(0);
    }
  });

  // Memory inspector: poll the active region + offset window at 4 Hz
  // while the view is open. The Rust side serves from a snapshot
  // Mutex, so polling is cheap.
  createEffect(() => {
    const v = view();
    if (v === "memory" && props.open) {
      const fetchMemory = () => {
        void invoke<MemoryRegionInfo>("read_memory_region", {
          region: memoryRegion(),
          offset: memoryOffset(),
          length: MEMORY_WINDOW_BYTES,
        })
          .then((info) => setMemoryView(info ?? null))
          .catch(() => setMemoryView(null));
      };
      fetchMemory();
      if (memoryPollId === undefined) {
        memoryPollId = window.setInterval(fetchMemory, 250);
      }
    } else if (memoryPollId !== undefined) {
      window.clearInterval(memoryPollId);
      memoryPollId = undefined;
    }
  });

  // While the TAS view is open, refresh stats at 4 Hz so the "frame X
  // of Y" status row updates without busy-polling 60 Hz. Refreshes also
  // happen on explicit user actions (record/stop) via direct invoke
  // results; this is just the steady-state heartbeat.
  createEffect(() => {
    const v = view();
    if (v === "tas" && props.open) {
      if (tasPollId === undefined) {
        tasPollId = window.setInterval(() => {
          void invoke<TasState>("get_tas_state")
            .then((s) => setTasState(s ?? null))
            .catch(() => {});
        }, 250);
      }
    } else if (tasPollId !== undefined) {
      window.clearInterval(tasPollId);
      tasPollId = undefined;
    }
  });

  createEffect(() => {
    const v = view();
    if (v === "video" && props.open) {
      if (videoPollId === undefined) {
        videoPollId = window.setInterval(() => {
          void invoke<VideoState>("get_video_state")
            .then((s) => setVideoState(s ?? null))
            .catch(() => {});
        }, 250);
      }
    } else if (videoPollId !== undefined) {
      window.clearInterval(videoPollId);
      videoPollId = undefined;
    }
  });

  const systemName = (): string => {
    const e = props.entry;
    if (!e) return "";
    return systemThemes[e.systemId]?.shortName ?? e.systemId;
  };

  // Can the user enter the scrubber right now? Needs rewind enabled +
  // a non-empty ring.
  const rewindAvailable = (): boolean => {
    const s = rewindState();
    return s !== null && s.enabled && s.snapshotCount > 0;
  };

  async function enterRewindView() {
    if (!rewindAvailable()) return;
    try {
      await invoke("start_rewind_scrub");
      setScrubPosition(0);
      setView("rewind");
    } catch (e) {
      console.warn("[oa-quick] start_rewind_scrub failed:", e);
    }
  }

  async function commitRewind() {
    try {
      await invoke("end_rewind_scrub", { commit: true });
    } catch (e) {
      console.warn("[oa-quick] commit failed:", e);
    }
    setView("actions");
    props.onClose();
  }

  async function cancelRewind() {
    try {
      await invoke("end_rewind_scrub", { commit: false });
    } catch (e) {
      console.warn("[oa-quick] cancel failed:", e);
    }
    setView("actions");
  }

  // --- TAS helpers ------------------------------------------------------

  async function enterTasView() {
    setView("tas");
    // Hydrate recordings list + tas state on view entry.
    void invoke<TasState>("get_tas_state")
      .then((s) => setTasState(s ?? null))
      .catch(() => {});
    const entry = props.entry;
    if (entry) {
      void invoke<TasListEntry[]>("list_tas_recordings", { romPath: entry.filePath })
        .then((list) => setTasRecordings(list ?? []))
        .catch((e) => {
          console.warn("[oa-quick] list_tas_recordings failed:", e);
          setTasRecordings([]);
        });
    }
  }

  async function refreshTasState() {
    try {
      const s = await invoke<TasState>("get_tas_state");
      setTasState(s ?? null);
    } catch {}
  }

  async function refreshRecordingsList() {
    const entry = props.entry;
    if (!entry) return;
    try {
      const list = await invoke<TasListEntry[]>("list_tas_recordings", { romPath: entry.filePath });
      setTasRecordings(list ?? []);
    } catch (e) {
      console.warn("[oa-quick] refresh recordings failed:", e);
    }
  }

  async function startTasRecording() {
    try {
      await invoke("start_tas_recording", { displayName: tasRecordingName().trim() });
      setTasRecordingName("");
      await refreshTasState();
    } catch (e) {
      console.warn("[oa-quick] start_tas_recording failed:", e);
    }
  }

  async function stopTasRecording(discard: boolean) {
    try {
      await invoke("stop_tas_recording", { discard });
      await refreshTasState();
      await refreshRecordingsList();
    } catch (e) {
      console.warn("[oa-quick] stop_tas_recording failed:", e);
    }
  }

  async function startTasReplay(filePath: string) {
    try {
      await invoke("start_tas_replay", { filePath });
      await refreshTasState();
    } catch (e) {
      console.warn("[oa-quick] start_tas_replay failed:", e);
    }
  }

  async function stopTasReplay() {
    try {
      await invoke("stop_tas_replay");
      await refreshTasState();
    } catch (e) {
      console.warn("[oa-quick] stop_tas_replay failed:", e);
    }
  }

  async function deleteTasRecording(filePath: string) {
    try {
      await invoke("delete_tas_recording", { filePath });
      await refreshRecordingsList();
    } catch (e) {
      console.warn("[oa-quick] delete_tas_recording failed:", e);
    }
  }

  // --- Video capture helpers --------------------------------------------

  async function enterVideoView() {
    setView("video");
    void invoke<VideoState>("get_video_state")
      .then((s) => setVideoState(s ?? null))
      .catch(() => {});
    const entry = props.entry;
    if (entry) {
      void invoke<VideoClipEntry[]>("list_video_clips", { romPath: entry.filePath })
        .then((list) => setVideoClips(list ?? []))
        .catch((e) => {
          console.warn("[oa-quick] list_video_clips failed:", e);
          setVideoClips([]);
        });
    }
  }

  async function refreshVideoState() {
    try {
      const s = await invoke<VideoState>("get_video_state");
      setVideoState(s ?? null);
    } catch {}
  }

  async function refreshClipsList() {
    const entry = props.entry;
    if (!entry) return;
    try {
      const list = await invoke<VideoClipEntry[]>("list_video_clips", { romPath: entry.filePath });
      setVideoClips(list ?? []);
    } catch (e) {
      console.warn("[oa-quick] refresh clips failed:", e);
    }
  }

  async function startVideoCapture() {
    try {
      await invoke("start_video_capture", { displayName: videoCaptureName().trim() });
      setVideoCaptureName("");
      await refreshVideoState();
    } catch (e) {
      console.warn("[oa-quick] start_video_capture failed:", e);
    }
  }

  async function stopVideoCapture(discard: boolean) {
    try {
      await invoke("stop_video_capture", { discard });
      await refreshVideoState();
      await refreshClipsList();
    } catch (e) {
      console.warn("[oa-quick] stop_video_capture failed:", e);
    }
  }

  async function deleteVideoClip(clipDir: string) {
    try {
      await invoke("delete_video_clip", { clipDir });
      await refreshClipsList();
    } catch (e) {
      console.warn("[oa-quick] delete_video_clip failed:", e);
    }
  }

  async function openVideoClipFolder(clipDir: string) {
    try {
      await invoke("open_video_clip_folder", { clipDir });
    } catch (e) {
      console.warn("[oa-quick] open_video_clip_folder failed:", e);
    }
  }

  // Slice D-2 — shell out to ffmpeg to convert a PNG sequence to WebM.
  // The Rust side blocks on ffmpeg, so we show a per-clip "converting…"
  // state to avoid the impression that the click did nothing.
  const [convertingClip, setConvertingClip] = createSignal<string | null>(null);
  const [conversionResult, setConversionResult] = createSignal<{
    clipDir: string;
    ok: boolean;
    message: string;
  } | null>(null);
  async function convertVideoClip(clipDir: string) {
    if (convertingClip()) return;
    setConvertingClip(clipDir);
    setConversionResult(null);
    try {
      const out = await invoke<string>("convert_video_clip_to_webm", { clipDir });
      setConversionResult({ clipDir, ok: true, message: out });
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as Error)?.message ?? String(e);
      console.warn("[oa-quick] convert_video_clip_to_webm failed:", e);
      setConversionResult({ clipDir, ok: false, message: msg });
    } finally {
      setConvertingClip(null);
    }
  }

  return (
    <Show when={props.open && props.entry !== null}>
      <div
        class="fixed inset-0 z-[60] grid place-items-center bg-black/55 backdrop-blur-sm"
        onClick={(e) => {
          if (e.currentTarget === e.target) {
            // If we're in rewind view, the parent overlay click should
            // cancel the scrub first.
            if (view() === "rewind") {
              void cancelRewind();
            }
            props.onClose();
          }
        }}
        data-system={props.entry?.systemId}
        role="dialog"
        aria-modal="true"
        aria-labelledby="quick-settings-title"
      >
        <div
          ref={cardRef}
          class="w-full max-w-md overflow-hidden rounded-xl border border-white/10 bg-(--color-oa-bg-deep)/95 shadow-2xl shadow-black/60"
        >
          <header class="border-b border-white/5 bg-(--color-system-accent)/10 px-5 py-3">
            <p class="text-[0.6rem] uppercase tracking-[0.3em] text-(--color-oa-ink-dim)">
              {systemName()} · {view() === "rewind" ? "Rewinding" : "Paused"}
            </p>
            <h2
              id="quick-settings-title"
              class="mt-0.5 truncate text-base font-semibold text-(--color-oa-ink)"
              title={props.entry?.title}
            >
              {props.entry?.title}
            </h2>
          </header>

          <Show when={view() === "actions"}>
            <div class="flex flex-col gap-1.5 p-3">
              <ActionRow icon="▶" label="Resume" hint="Esc" onClick={props.onClose} autoFocus />
              <ActionRow
                icon="⏱"
                label="Save / Load states"
                onClick={() => {
                  if (props.entry) {
                    props.onClose();
                    props.onShowSaves(props.entry);
                  }
                }}
              />
              <ActionRow
                icon="ⓘ"
                label="Game info"
                onClick={() => {
                  if (props.entry) {
                    props.onClose();
                    props.onShowInfo(props.entry);
                  }
                }}
              />
              <div class="my-1 border-t border-white/5" />
              <ActionRow
                icon="🚪"
                label="Exit to library"
                hint="Ctrl+W"
                destructive
                onClick={() => {
                  props.onClose();
                  props.onExitToLibrary();
                }}
              />
              <p class="px-2 pt-1 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                Rewind · TAS · Video · Memory → top bar <span class="text-(--color-oa-ink)">Tools</span> menu
              </p>
            </div>
          </Show>

          <Show when={view() === "rewind" && rewindState() !== null}>
            <RewindScrubber
              state={rewindState()!}
              scrubPosition={scrubPosition()}
              onSetPosition={(pos) => {
                // Clamp client-side too so the thumb doesn't jitter past
                // the bounds during fast drags.
                const s = rewindState();
                const max = s ? Math.max(s.snapshotCount - 1, 0) : 0;
                const clamped = Math.max(0, Math.min(max, pos));
                setScrubPosition(clamped);
                void invoke("set_rewind_scrub_position", { stepsBack: clamped }).catch(() => {});
              }}
              onCommit={() => void commitRewind()}
              onCancel={() => void cancelRewind()}
            />
          </Show>

          <Show when={view() === "tas"}>
            <TasPanel
              tas={tasState()}
              recordings={tasRecordings()}
              recordingName={tasRecordingName()}
              onNameChange={(n) => setTasRecordingName(n)}
              onStartRecording={() => void startTasRecording()}
              onStopRecording={() => void stopTasRecording(false)}
              onDiscardRecording={() => void stopTasRecording(true)}
              onReplay={(file) => void startTasReplay(file)}
              onStopReplay={() => void stopTasReplay()}
              onDelete={(file) => void deleteTasRecording(file)}
              onBack={() => setView("actions")}
            />
          </Show>

          <Show when={view() === "memory"}>
            <MemoryInspectorPanel
              info={memoryView()}
              region={memoryRegion()}
              offset={memoryOffset()}
              onRegionChange={(r) => {
                setMemoryRegion(r);
                setMemoryOffset(0);
              }}
              onOffsetChange={(off) => setMemoryOffset(off)}
              onBack={() => setView("actions")}
            />
          </Show>

          <Show when={view() === "video"}>
            <VideoPanel
              video={videoState()}
              clips={videoClips()}
              captureName={videoCaptureName()}
              onNameChange={(n) => setVideoCaptureName(n)}
              onStartCapture={() => void startVideoCapture()}
              onStopCapture={() => void stopVideoCapture(false)}
              onDiscardCapture={() => void stopVideoCapture(true)}
              onOpenFolder={(dir) => void openVideoClipFolder(dir)}
              onDelete={(dir) => void deleteVideoClip(dir)}
              onConvertWebM={(dir) => void convertVideoClip(dir)}
              convertingClip={convertingClip()}
              conversionResult={conversionResult()}
              onBack={() => setView("actions")}
            />
          </Show>

          <Show when={view() === "disc" && discInfo() !== null}>
            <DiscPanel
              info={discInfo()!}
              swapping={discSwapping()}
              onSwap={(idx) => void swapToDisc(idx)}
              onBack={() => setView("actions")}
            />
          </Show>
        </div>
      </div>
    </Show>
  );
};

// --- Disc control panel ----------------------------------------------

type DiscPanelProps = {
  info: DiscInfo;
  swapping: boolean;
  onSwap: (index: number) => void;
  onBack: () => void;
};

/// RetroArch-parity slice — multi-disc swap UI. Listed under
/// QuickSettings when the active core registered a disc-control
/// interface AND has more than one disc image (single-disc games hide
/// the row entirely). Clicking "Insert" runs the canonical
/// eject → set_image_index → close sequence; the core resumes from the
/// new disc on the next frame.
const DiscPanel: Component<DiscPanelProps> = (props) => {
  const labelFor = (idx: number): string => {
    const fromCore = props.info.labels[idx];
    if (fromCore && fromCore.trim().length > 0) return fromCore;
    return `Disc ${idx + 1}`;
  };
  return (
    <div class="flex flex-col gap-3 p-4">
      <div class="flex items-center justify-between text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
        <span>Disc control</span>
        <span>
          {props.info.numDiscs} {props.info.numDiscs === 1 ? "disc" : "discs"}
          {props.info.ejected ? " · tray open" : ""}
        </span>
      </div>
      <Show when={props.info.numDiscs <= 1}>
        <p class="rounded-md border border-white/5 bg-white/[0.02] px-3 py-2 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          Single-disc game — disc swap only meaningful for multi-disc images
          (e.g. <code class="font-mono normal-case">.m3u</code> playlists).
        </p>
      </Show>
      <div class="flex flex-col gap-1.5">
        <For each={Array.from({ length: props.info.numDiscs }, (_, i) => i)}>
          {(idx) => {
            const isActive = () => idx === props.info.currentIndex;
            return (
              <div
                class="flex items-center gap-3 rounded-md border p-2.5"
                classList={{
                  "border-(--color-system-accent)/40 bg-(--color-system-accent)/10": isActive(),
                  "border-white/10 bg-white/[0.03]": !isActive(),
                }}
              >
                <div class="min-w-0 flex-1">
                  <p class="truncate text-sm text-(--color-oa-ink)">{labelFor(idx)}</p>
                  <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                    {isActive() ? "loaded" : "available"}
                  </p>
                </div>
                <button
                  type="button"
                  onClick={() => props.onSwap(idx)}
                  disabled={isActive() || props.swapping}
                  class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) hover:bg-white/[0.08] disabled:opacity-40 disabled:cursor-not-allowed"
                >
                  {props.swapping ? "…" : isActive() ? "Loaded" : "Insert"}
                </button>
              </div>
            );
          }}
        </For>
      </div>
      <button
        type="button"
        onClick={props.onBack}
        class="self-start rounded border border-white/10 bg-white/[0.04] px-3 py-1 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)"
      >
        ← Back
      </button>
    </div>
  );
};

// --- Rewind timeline strip ---------------------------------------------

type ScrubberProps = {
  state: RewindState;
  scrubPosition: number;
  onSetPosition: (stepsBack: number) => void;
  onCommit: () => void;
  onCancel: () => void;
};

const RewindScrubber: Component<ScrubberProps> = (props) => {
  let stripRef: HTMLDivElement | undefined;
  const [dragging, setDragging] = createSignal(false);

  // x-coordinate to steps_back. Left edge = oldest snapshot
  // (count - 1 steps back); right edge = newest (0 steps back).
  function xToStepsBack(clientX: number): number {
    if (!stripRef) return 0;
    const rect = stripRef.getBoundingClientRect();
    if (rect.width <= 0) return 0;
    const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
    const max = Math.max(props.state.snapshotCount - 1, 0);
    return Math.round((1 - ratio) * max);
  }

  function onPointerDown(e: PointerEvent) {
    if (!stripRef) return;
    stripRef.setPointerCapture(e.pointerId);
    setDragging(true);
    props.onSetPosition(xToStepsBack(e.clientX));
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging()) return;
    props.onSetPosition(xToStepsBack(e.clientX));
  }

  function onPointerUp(e: PointerEvent) {
    if (!stripRef) return;
    try {
      stripRef.releasePointerCapture(e.pointerId);
    } catch {
      // No capture to release (pointer left + re-entered the window).
    }
    setDragging(false);
  }

  // Visual: thumb x-position derived from scrubPosition. 0 steps back =
  // rightmost (live edge); count-1 = leftmost.
  function thumbLeftPercent(): number {
    const max = Math.max(props.state.snapshotCount - 1, 0);
    if (max <= 0) return 100;
    const ratio = 1 - props.state.scrubPosition / max;
    return Math.max(0, Math.min(100, ratio * 100));
  }

  const totalSeconds = (): number => {
    return (props.state.snapshotCount * props.state.captureIntervalFrames)
      / Math.max(props.state.fps, 1);
  };

  const positionSeconds = (): number => {
    // Seconds-into-the-past corresponding to the current scrub position.
    return (props.scrubPosition * props.state.captureIntervalFrames)
      / Math.max(props.state.fps, 1);
  };

  return (
    <div class="flex flex-col gap-3 p-4">
      <div class="flex items-baseline justify-between text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
        <span>
          {props.scrubPosition === 0 ? "Live edge" : `-${positionSeconds().toFixed(2)}s`}
        </span>
        <span>{props.state.snapshotCount} snapshots · {totalSeconds().toFixed(1)}s buffered</span>
      </div>

      {/* Timeline strip. Left = oldest, right = newest. */}
      <div
        ref={stripRef}
        class="relative h-12 touch-none select-none rounded-md border border-white/10 bg-white/[0.03]"
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
        role="slider"
        aria-label="Rewind position"
        aria-valuemin={0}
        aria-valuemax={Math.max(props.state.snapshotCount - 1, 0)}
        aria-valuenow={props.scrubPosition}
        tabindex="0"
        onKeyDown={(e) => {
          // Arrow keys: ←/→ shift by 1, Shift+arrow shifts by 10.
          const step = e.shiftKey ? 10 : 1;
          if (e.key === "ArrowLeft") {
            e.preventDefault();
            props.onSetPosition(props.scrubPosition + step);
          } else if (e.key === "ArrowRight") {
            e.preventDefault();
            props.onSetPosition(props.scrubPosition - step);
          } else if (e.key === "Home") {
            e.preventDefault();
            // Jump to oldest.
            props.onSetPosition(props.state.snapshotCount - 1);
          } else if (e.key === "End") {
            e.preventDefault();
            // Jump to newest (live edge).
            props.onSetPosition(0);
          }
        }}
      >
        {/* Filled region from left edge to current thumb position = the
            "rewound-into-the-past" span. */}
        <div
          class="absolute inset-y-0 left-0 rounded-l-md bg-(--color-system-accent)/30"
          style={{ width: `${thumbLeftPercent()}%` }}
        />
        {/* Vertical thumb. */}
        <div
          class="absolute top-0 bottom-0 w-0.5 -translate-x-1/2 rounded-full bg-(--color-system-accent) shadow-[0_0_8px_var(--color-system-accent)]"
          style={{ left: `${thumbLeftPercent()}%` }}
        />
        {/* Endpoint labels. */}
        <div class="pointer-events-none absolute inset-y-0 left-2 flex items-center text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          oldest
        </div>
        <div class="pointer-events-none absolute inset-y-0 right-2 flex items-center text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          live
        </div>
      </div>

      <p class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
        Drag the strip, click to jump, or use ←/→ (Shift = 10× step) · Home / End jump to ends.
      </p>

      <div class="mt-1 flex gap-2">
        <button
          type="button"
          onClick={props.onCancel}
          class={BTN_BASE + " flex-1 justify-center"}
        >
          <span aria-hidden="true">↩</span>
          <span>Cancel (back to live)</span>
        </button>
        <button
          type="button"
          onClick={props.onCommit}
          disabled={props.scrubPosition === 0}
          class={BTN_BASE + " flex-1 justify-center border-(--color-system-accent)/40 bg-(--color-system-accent)/15 text-(--color-oa-ink) hover:bg-(--color-system-accent)/25"}
        >
          <span aria-hidden="true">⏯</span>
          <span>Resume from here</span>
        </button>
      </div>
    </div>
  );
};

// --- TAS panel ----------------------------------------------------------

type TasPanelProps = {
  tas: TasState | null;
  recordings: TasListEntry[];
  recordingName: string;
  onNameChange: (n: string) => void;
  onStartRecording: () => void;
  onStopRecording: () => void;
  onDiscardRecording: () => void;
  onReplay: (filePath: string) => void;
  onStopReplay: () => void;
  onDelete: (filePath: string) => void;
  onBack: () => void;
};

function formatDuration(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "—";
  const m = Math.floor(seconds / 60);
  const s = seconds - m * 60;
  if (m > 0) return `${m}:${s.toFixed(1).padStart(4, "0")}`;
  return `${s.toFixed(1)}s`;
}

function formatTimestamp(unixMs: number): string {
  if (!Number.isFinite(unixMs) || unixMs <= 0) return "";
  try {
    const d = new Date(unixMs);
    return d.toLocaleString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return "";
  }
}

const TasPanel: Component<TasPanelProps> = (props) => {
  const mode = (): TasMode => props.tas?.mode ?? "idle";
  const frame = (): number => props.tas?.frame ?? 0;
  const totalFrames = (): number => props.tas?.totalFrames ?? 0;
  const displayName = (): string => props.tas?.displayName ?? "";

  return (
    <div class="flex flex-col gap-3 p-4">
      <div class="flex items-center justify-between text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
        <span>
          {mode() === "idle" && "TAS"}
          {mode() === "recording" && `Recording${displayName() ? ` · ${displayName()}` : ""}`}
          {mode() === "replaying" && `Replaying${displayName() ? ` · ${displayName()}` : ""}`}
        </span>
        <span>
          {mode() === "recording" && `${frame()} frames`}
          {mode() === "replaying" && `${frame()} / ${totalFrames()}`}
        </span>
      </div>

      <Show when={mode() === "idle"}>
        {/* Start a new recording */}
        <div class="rounded-md border border-white/10 bg-white/[0.03] p-3">
          <label class="block text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            New recording — optional label
          </label>
          <input
            type="text"
            value={props.recordingName}
            placeholder="e.g. World 1-1 any%"
            onInput={(e) => props.onNameChange(e.currentTarget.value)}
            class="mt-2 w-full rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-sm text-(--color-oa-ink) focus-visible:border-(--color-system-accent) focus-visible:outline-none"
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                props.onStartRecording();
              }
            }}
          />
          <button
            type="button"
            onClick={props.onStartRecording}
            class={BTN_BASE + " mt-2 justify-center border-(--color-system-accent)/40 bg-(--color-system-accent)/15 hover:bg-(--color-system-accent)/25"}
          >
            <span aria-hidden="true">⏺</span>
            <span>Start recording</span>
          </button>
        </div>

        {/* Existing recordings list */}
        <Show when={props.recordings.length > 0} fallback={
          <p class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            No recordings yet for this game.
          </p>
        }>
          <div class="flex max-h-64 flex-col gap-1.5 overflow-y-auto">
            {props.recordings.map((r) => (
              <div class="flex items-center gap-2 rounded-md border border-white/10 bg-white/[0.03] p-2.5">
                <div class="min-w-0 flex-1">
                  <p class="truncate text-sm text-(--color-oa-ink)">
                    {r.displayName.trim() || "(unnamed)"}
                  </p>
                  <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                    {formatDuration(r.durationSeconds)} · {r.frameCount} frames · {formatTimestamp(r.recordedAtUnixMs)}
                  </p>
                </div>
                <button
                  type="button"
                  onClick={() => props.onReplay(r.filePath)}
                  title="Replay this recording"
                  class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) hover:bg-white/[0.08]"
                >
                  ▶ Replay
                </button>
                <button
                  type="button"
                  onClick={() => props.onDelete(r.filePath)}
                  title="Delete this recording"
                  class="rounded border border-red-500/30 px-2 py-1 text-xs text-red-300 hover:bg-red-500/10"
                >
                  ✕
                </button>
              </div>
            ))}
          </div>
        </Show>
      </Show>

      <Show when={mode() === "recording"}>
        <div class="rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/10 p-3 text-sm text-(--color-oa-ink)">
          Recording inputs frame-by-frame.
          Hold-Backspace rewind is paused while recording so the input log
          stays clean.
        </div>
        <div class="flex gap-2">
          <button
            type="button"
            onClick={props.onDiscardRecording}
            class={BTN_BASE + " flex-1 justify-center text-red-300 hover:bg-red-500/10"}
          >
            <span aria-hidden="true">🗑</span>
            <span>Discard</span>
          </button>
          <button
            type="button"
            onClick={props.onStopRecording}
            class={BTN_BASE + " flex-1 justify-center border-(--color-system-accent)/40 bg-(--color-system-accent)/15 hover:bg-(--color-system-accent)/25"}
          >
            <span aria-hidden="true">⏹</span>
            <span>Stop & save</span>
          </button>
        </div>
      </Show>

      <Show when={mode() === "replaying"}>
        <div class="rounded-md border border-white/10 bg-white/[0.03] p-3">
          {/* Progress bar */}
          <div class="h-2 overflow-hidden rounded bg-white/5">
            <div
              class="h-full bg-(--color-system-accent) transition-[width]"
              style={{
                width: `${Math.min(100, (frame() / Math.max(totalFrames(), 1)) * 100)}%`,
              }}
            />
          </div>
          <p class="mt-2 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            User input is suppressed during replay. Stop to take control.
          </p>
        </div>
        <button
          type="button"
          onClick={props.onStopReplay}
          class={BTN_BASE + " justify-center"}
        >
          <span aria-hidden="true">⏹</span>
          <span>Stop replay</span>
        </button>
      </Show>

      <button
        type="button"
        onClick={props.onBack}
        class={BTN_BASE + " justify-center"}
        disabled={mode() !== "idle"}
        title={mode() !== "idle" ? "Stop the current TAS operation first" : ""}
      >
        <span aria-hidden="true">↩</span>
        <span>Back</span>
      </button>
    </div>
  );
};

// --- Video panel --------------------------------------------------------

type VideoPanelProps = {
  video: VideoState | null;
  clips: VideoClipEntry[];
  captureName: string;
  onNameChange: (n: string) => void;
  onStartCapture: () => void;
  onStopCapture: () => void;
  onDiscardCapture: () => void;
  onOpenFolder: (clipDir: string) => void;
  onDelete: (clipDir: string) => void;
  onConvertWebM: (clipDir: string) => void;
  /// Slice D-2 — non-null while a conversion is in flight; the matching
  /// clip row replaces its "Convert" button with a spinner-style label.
  convertingClip: string | null;
  /// Slice D-2 — last conversion's outcome. Sticks until the next conversion
  /// starts; the matching clip row shows a small ok/error indicator.
  conversionResult: { clipDir: string; ok: boolean; message: string } | null;
  onBack: () => void;
};

const VideoPanel: Component<VideoPanelProps> = (props) => {
  const capturing = (): boolean => props.video?.capturing ?? false;
  const frameCount = (): number => props.video?.frameCount ?? 0;
  const droppedCount = (): number => props.video?.droppedFrameCount ?? 0;
  const displayName = (): string => props.video?.displayName ?? "";

  return (
    <div class="flex flex-col gap-3 p-4">
      <div class="flex items-center justify-between text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
        <span>
          {capturing()
            ? `Capturing${displayName() ? ` · ${displayName()}` : ""}`
            : "Video capture"}
        </span>
        <span>
          {capturing() && `${frameCount()} frames${droppedCount() > 0 ? ` · ${droppedCount()} dropped` : ""}`}
        </span>
      </div>

      <Show when={!capturing()}>
        <div class="rounded-md border border-white/10 bg-white/[0.03] p-3">
          <label class="block text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            New capture — optional label
          </label>
          <input
            type="text"
            value={props.captureName}
            placeholder="e.g. boss-fight"
            onInput={(e) => props.onNameChange(e.currentTarget.value)}
            class="mt-2 w-full rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-sm text-(--color-oa-ink) focus-visible:border-(--color-system-accent) focus-visible:outline-none"
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                props.onStartCapture();
              }
            }}
          />
          <button
            type="button"
            onClick={props.onStartCapture}
            class={BTN_BASE + " mt-2 justify-center border-(--color-system-accent)/40 bg-(--color-system-accent)/15 hover:bg-(--color-system-accent)/25"}
          >
            <span aria-hidden="true">🎥</span>
            <span>Start capture</span>
          </button>
          <p class="mt-2 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            One PNG per frame is written under{" "}
            <code class="font-mono normal-case">appData/clips/&lt;rom&gt;/&lt;timestamp&gt;/</code>
            {" "}plus a <code class="font-mono normal-case">manifest.json</code>. Convert offline
            with ffmpeg.
          </p>
        </div>

        <Show when={props.clips.length > 0} fallback={
          <p class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            No clips captured for this game yet.
          </p>
        }>
          <div class="flex max-h-64 flex-col gap-1.5 overflow-y-auto">
            {props.clips.map((c) => (
              <div class="flex items-center gap-2 rounded-md border border-white/10 bg-white/[0.03] p-2.5">
                <div class="min-w-0 flex-1">
                  <p class="truncate text-sm text-(--color-oa-ink)">
                    {c.displayName.trim() || "(unnamed)"}
                  </p>
                  <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                    {formatDuration(c.durationSeconds)} · {c.frameCount} frames · {c.width}×{c.height}
                    {c.droppedFrameCount > 0 ? ` · ${c.droppedFrameCount} dropped` : ""}
                    {" · "}{formatTimestamp(c.recordedAtUnixMs)}
                  </p>
                </div>
                <button
                  type="button"
                  onClick={() => props.onConvertWebM(c.clipDir)}
                  disabled={props.convertingClip !== null}
                  title={
                    props.convertingClip === c.clipDir
                      ? "Running ffmpeg…"
                      : props.conversionResult?.clipDir === c.clipDir
                      ? props.conversionResult.message
                      : "Convert PNG frames to a single WebM via ffmpeg"
                  }
                  class={
                    "rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) hover:bg-white/[0.08] disabled:opacity-50 disabled:cursor-wait"
                  }
                >
                  {props.convertingClip === c.clipDir
                    ? "…WebM"
                    : props.conversionResult?.clipDir === c.clipDir && props.conversionResult.ok
                    ? "✓ WebM"
                    : props.conversionResult?.clipDir === c.clipDir && !props.conversionResult.ok
                    ? "⚠ WebM"
                    : "WebM"}
                </button>
                <button
                  type="button"
                  onClick={() => props.onOpenFolder(c.clipDir)}
                  title="Open the clip's PNG folder"
                  class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) hover:bg-white/[0.08]"
                >
                  📁
                </button>
                <button
                  type="button"
                  onClick={() => props.onDelete(c.clipDir)}
                  title="Delete this clip"
                  class="rounded border border-red-500/30 px-2 py-1 text-xs text-red-300 hover:bg-red-500/10"
                >
                  ✕
                </button>
              </div>
            ))}
          </div>
        </Show>
      </Show>

      <Show when={capturing()}>
        <div class="rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/10 p-3 text-sm text-(--color-oa-ink)">
          Writing one PNG per emulator frame to disk.
          Drops happen when the encoder can't keep up — they're recorded in
          the manifest so you know where the gaps are.
        </div>
        <div class="flex gap-2">
          <button
            type="button"
            onClick={props.onDiscardCapture}
            class={BTN_BASE + " flex-1 justify-center text-red-300 hover:bg-red-500/10"}
          >
            <span aria-hidden="true">🗑</span>
            <span>Discard</span>
          </button>
          <button
            type="button"
            onClick={props.onStopCapture}
            class={BTN_BASE + " flex-1 justify-center border-(--color-system-accent)/40 bg-(--color-system-accent)/15 hover:bg-(--color-system-accent)/25"}
          >
            <span aria-hidden="true">⏹</span>
            <span>Stop & save</span>
          </button>
        </div>
      </Show>

      <button
        type="button"
        onClick={props.onBack}
        class={BTN_BASE + " justify-center"}
        disabled={capturing()}
        title={capturing() ? "Stop or discard the capture first" : ""}
      >
        <span aria-hidden="true">↩</span>
        <span>Back</span>
      </button>
    </div>
  );
};

// --- Memory inspector panel --------------------------------------------

type MemoryInspectorProps = {
  info: MemoryRegionInfo | null;
  region: MemoryRegionId;
  offset: number;
  onRegionChange: (r: MemoryRegionId) => void;
  onOffsetChange: (offset: number) => void;
  onBack: () => void;
};

const MEMORY_REGION_LABELS: Record<MemoryRegionId, string> = {
  save_ram: "Save RAM",
  rtc: "RTC",
  system_ram: "System RAM",
  video_ram: "Video RAM",
};

const MEMORY_REGION_ORDER: readonly MemoryRegionId[] = ["system_ram", "save_ram", "video_ram", "rtc"];

function formatHex(n: number, digits: number): string {
  return n.toString(16).toUpperCase().padStart(digits, "0");
}

function parseOffsetInput(raw: string): number | null {
  const trimmed = raw.trim();
  if (trimmed.length === 0) return 0;
  // Accept "0x..." or plain hex / decimal. Decimal-looking inputs (no
  // letters, no 0x prefix) parse as decimal so "1234" means byte 1234,
  // not byte 0x1234. Hex needs an explicit 0x or contains a-f.
  const hex = trimmed.toLowerCase();
  if (hex.startsWith("0x")) {
    const n = parseInt(hex.slice(2), 16);
    return Number.isFinite(n) && n >= 0 ? n : null;
  }
  if (/[a-f]/.test(hex)) {
    const n = parseInt(hex, 16);
    return Number.isFinite(n) && n >= 0 ? n : null;
  }
  const n = parseInt(trimmed, 10);
  return Number.isFinite(n) && n >= 0 ? n : null;
}

const MemoryInspectorPanel: Component<MemoryInspectorProps> = (props) => {
  const [offsetInput, setOffsetInput] = createSignal(formatHex(props.offset, 4));
  // Keep the input in sync if the offset changes via paging buttons.
  createEffect(() => setOffsetInput(formatHex(props.offset, 4)));

  const rows = (): { offset: number; bytes: number[] }[] => {
    const info = props.info;
    if (!info || !info.available || info.bytes.length === 0) return [];
    // 8 bytes per row keeps the layout fitting inside max-w-md.
    const out: { offset: number; bytes: number[] }[] = [];
    for (let i = 0; i < info.bytes.length; i += 8) {
      out.push({
        offset: info.offset + i,
        bytes: info.bytes.slice(i, i + 8),
      });
    }
    return out;
  };

  function applyOffset() {
    const parsed = parseOffsetInput(offsetInput());
    if (parsed === null) return;
    const max = Math.max(0, (props.info?.totalSize ?? 0) - 1);
    props.onOffsetChange(Math.min(parsed, max));
  }

  function nudge(delta: number) {
    const max = Math.max(0, (props.info?.totalSize ?? 0) - 1);
    props.onOffsetChange(Math.max(0, Math.min(props.offset + delta, max)));
  }

  return (
    <div class="flex flex-col gap-3 p-4">
      <div class="flex items-center justify-between text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
        <span>Memory inspector</span>
        <span>{props.info?.available ? `${props.info.totalSize.toLocaleString()} bytes` : "—"}</span>
      </div>

      {/* Region picker + offset input on one row */}
      <div class="flex gap-2">
        <select
          value={props.region}
          onChange={(e) => props.onRegionChange(e.currentTarget.value as MemoryRegionId)}
          class="flex-1 rounded-md border border-white/10 bg-white/[0.04] px-2 py-1.5 text-sm text-(--color-oa-ink) focus-visible:border-(--color-system-accent) focus-visible:outline-none"
        >
          <For each={MEMORY_REGION_ORDER}>
            {(r) => <option value={r}>{MEMORY_REGION_LABELS[r]}</option>}
          </For>
        </select>
        <div class="flex items-center gap-1 rounded-md border border-white/10 bg-white/[0.04] px-2">
          <span class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">0x</span>
          <input
            type="text"
            value={offsetInput()}
            onInput={(e) => setOffsetInput(e.currentTarget.value)}
            onBlur={applyOffset}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                applyOffset();
              }
            }}
            class="w-20 bg-transparent py-1.5 font-mono text-sm text-(--color-oa-ink) focus-visible:outline-none"
            placeholder="0000"
          />
        </div>
      </div>

      {/* Hex view */}
      <Show
        when={props.info?.available && (props.info?.bytes?.length ?? 0) > 0}
        fallback={
          <div class="rounded-md border border-white/10 bg-white/[0.03] p-3 text-xs text-(--color-oa-ink-dim)">
            {props.info?.available === false
              ? "Region unavailable for this core."
              : "Waiting for first frame…"}
          </div>
        }
      >
        <div class="overflow-y-auto rounded-md border border-white/10 bg-black/40 p-2 font-mono text-[0.7rem] leading-tight" style={{ "max-height": "20rem" }}>
          <For each={rows()}>
            {(row) => (
              <div class="flex gap-2 text-(--color-oa-ink)">
                <span class="text-(--color-oa-ink-dim) select-none">{formatHex(row.offset, 6)}</span>
                <span class="flex-1">
                  <For each={row.bytes}>
                    {(b, i) => (
                      <span class={i() % 2 === 0 ? "" : "text-(--color-oa-ink-dim)"}>
                        {formatHex(b, 2)}{i() < row.bytes.length - 1 ? " " : ""}
                      </span>
                    )}
                  </For>
                </span>
              </div>
            )}
          </For>
        </div>
      </Show>

      {/* Paging buttons */}
      <div class="flex gap-2">
        <button
          type="button"
          onClick={() => nudge(-MEMORY_WINDOW_BYTES)}
          class={BTN_BASE + " flex-1 justify-center"}
          disabled={props.offset === 0}
        >
          ◀ Prev page
        </button>
        <button
          type="button"
          onClick={() => nudge(MEMORY_WINDOW_BYTES)}
          class={BTN_BASE + " flex-1 justify-center"}
          disabled={!props.info || props.offset + MEMORY_WINDOW_BYTES >= props.info.totalSize}
        >
          Next page ▶
        </button>
      </div>

      <button
        type="button"
        onClick={props.onBack}
        class={BTN_BASE + " justify-center"}
      >
        <span aria-hidden="true">↩</span>
        <span>Back</span>
      </button>
    </div>
  );
};

const ActionRow: Component<{
  icon: string;
  label: string;
  hint?: string;
  disabled?: boolean;
  destructive?: boolean;
  autoFocus?: boolean;
  onClick: () => void;
}> = (props) => {
  let ref: HTMLButtonElement | undefined;
  onMount(() => {
    if (props.autoFocus && ref && !props.disabled) {
      // The createEffect in the parent sets focus too, but as a backstop
      // for cases where the first non-disabled child reorders.
    }
  });
  const handler: JSX.EventHandler<HTMLButtonElement, MouseEvent> = (e) => {
    e.currentTarget.blur();
    if (!props.disabled) props.onClick();
  };
  return (
    <button
      ref={ref}
      type="button"
      data-quick-action
      disabled={props.disabled === true}
      onClick={handler}
      class={BTN_BASE}
      classList={{
        "text-red-300 hover:bg-red-500/10 hover:text-red-200": props.destructive === true,
      }}
    >
      <span class="text-base" aria-hidden="true">
        {props.icon}
      </span>
      <span class="flex-1">{props.label}</span>
      <Show when={props.hint}>
        <span class="rounded border border-white/10 bg-white/[0.04] px-1.5 py-0.5 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          {props.hint}
        </span>
      </Show>
    </button>
  );
};

export default QuickSettings;
