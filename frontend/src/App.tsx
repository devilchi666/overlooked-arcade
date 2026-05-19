import { createEffect, createMemo, createSignal, Match, onCleanup, onMount, Show, Switch, type Component } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open as pickDirectory } from "@tauri-apps/plugin-dialog";
import CorePickerMenu from "./components/CorePickerMenu";
import GameInfoModal from "./components/GameInfoModal";
import ImportWizard from "./components/ImportWizard";
import LibraryView from "./components/LibraryView";
import PerGameSettingsDrawer, { type GameDrawerTab } from "./components/PerGameSettingsDrawer";
import CoresPage from "./components/CoresPage";
import QuickSettings, { type QuickSettingsView } from "./components/QuickSettings";
import SaveSlotsModal from "./components/SaveSlotsModal";
import SettingsPage from "./components/SettingsPage";
import SystemContextMenu from "./components/SystemContextMenu";
import RegionPicker from "./components/RegionPicker";
import TileContextMenu from "./components/TileContextMenu";
import ToastStack from "./components/ToastStack";
import Shell from "./layout/Shell";
import TopToolbar from "./layout/TopToolbar";
import LeftSidebar, { type SidebarView } from "./layout/LeftSidebar";
import { MenuBar, Menu, MenuItem, MenuLabel, MenuDivider, MenuRadio, MenuCheckbox } from "./layout/MenuBar";
import {
  AudioDialog,
  DisplayDialog,
  GameplayDialog,
  ShadersDialog,
  SHELL_MODE_LABELS,
  SHELL_OPTIONS,
  type ShellMode,
} from "./components/SettingsDialogs";
import {
  SystemBindingsDialog,
  SystemCoreOptionsDialog,
  SystemSettingsDialog,
  type SystemDialogSection,
} from "./components/SystemDialogs";
import { AboutDialog, KeyboardShortcutsDialog } from "./components/HelpDialogs";
import { DebugLogDialog } from "./components/DebugLogDialog";
import { WidgetCustomizerDialog } from "./components/WidgetCustomizerDialog";
import { ScreenshotGalleryDialog } from "./components/ScreenshotGalleryDialog";
import { PerformanceHud } from "./components/PerformanceHud";
import RightSidebar from "./layout/RightSidebar";
import {
  createLayoutStore,
  PRESENTATION_LABELS,
  PRESENTATION_OPTIONS,
  VIEW_MODE_LABELS,
  VIEW_MODE_OPTIONS,
  SORT_KEY_LABELS,
  SORT_KEY_OPTIONS,
  GROUP_BY_LABELS,
  GROUP_BY_OPTIONS,
  type PresentationMode,
  type ViewMode,
  type SortKey,
  type GroupBy,
} from "./layout/state";
import {
  ingestFolderPath,
  pickFolderAndIngest,
  rescanFolders,
  romIdFromPath,
  titleFromFileName,
  type IngestResult,
  type ScanProgress,
} from "./library/ingest";
import { listen } from "@tauri-apps/api/event";
import { allSupportedExtensions, resolveShaderPreset, systemForExtension } from "./themes/registry";
import { launchRom, type LaunchResult } from "./library/launch";
import { MediaProvider } from "./library/media";
import { createLibraryStore } from "./library/store";
import type { RomEntry } from "./library/types";
import { createSettingsStore } from "./settings/store";
import { loadShaderPresets, applyShaderPresetsUpdate, type ShaderPresetEntry } from "./settings/shader_presets";
import type { SystemId } from "./themes/registry";

type Busy = "idle" | "scanning" | "launching";

function ingestStatus(result: IngestResult): string {
  switch (result.kind) {
    case "cancelled": return "Pick cancelled.";
    case "empty":     return `No supported ROMs in ${result.folder}.`;
    case "error":     return result.message;
    case "ingested":  return `Added ${result.added} of ${result.total} from ${result.folder}.`;
  }
}

function launchStatus(result: LaunchResult): string {
  switch (result.kind) {
    case "skipped-seed": return "Placeholder tile — pick a folder to load real ROMs.";
    case "error":        return `Launch failed: ${result.message}`;
    case "launched":     return `Launched ${result.entry.title}.`;
  }
}

const TOOLBAR_BTN =
  "rounded-md border border-white/10 bg-white/[0.04] px-2.5 py-1.5 text-[0.65rem] font-medium uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:cursor-wait disabled:opacity-60";

const App: Component = () => {
  const library = createLibraryStore();
  const settings = createSettingsStore();
  const layout = createLayoutStore();
  const [busy, setBusy] = createSignal<Busy>("idle");
  const [status, setStatus] = createSignal<string>("");
  const [shellMode, setShellMode] = createSignal<ShellMode>("two-window");
  const [libraryVisible, setLibraryVisible] = createSignal(true);
  // OA-wide Settings dialogs (Step 4). Single open-dialog signal — opening
  // one closes the others. `null` means no dialog open.
  const [settingsDialog, setSettingsDialog] = createSignal<
    "display" | "audio" | "gameplay" | "shaders" | null
  >(null);
  // Per-system dialogs (Step 5). The target system + which section is
  // open. Opening from the System ▾ menu uses the actively-viewed system;
  // opening from the SystemContextMenu (right-click in sidebar) uses the
  // right-clicked system, which may differ from what's on screen.
  const [systemDialog, setSystemDialog] = createSignal<{
    section: SystemDialogSection;
    target: SystemId;
  } | null>(null);
  function openSystemDialog(section: SystemDialogSection, target: SystemId) {
    setSystemDialog({ section, target });
  }
  // Current navigation target — replaces the old systemPage signal. The
  // sidebar drives this, App routes content from it.
  const [currentView, setCurrentView] = createSignal<SidebarView>({ kind: "all" });
  const [savesEntry, setSavesEntry] = createSignal<RomEntry | null>(null);
  // Right-click target for the unified tile context menu.
  const [contextMenuFor, setContextMenuFor] = createSignal<{ entry: RomEntry; position: { x: number; y: number } } | null>(null);
  const [coreMenuFor, setCoreMenuFor] = createSignal<{ entry: RomEntry; position: { x: number; y: number } } | null>(null);
  const [regionPickerFor, setRegionPickerFor] = createSignal<RomEntry | null>(null);
  const [gameInfoFor, setGameInfoFor] = createSignal<RomEntry | null>(null);
  // Phase 2.8 slice D — per-game settings drawer. Triggered from the tile
  // context menu's Game properties… item; null when closed.
  const [propertiesFor, setPropertiesFor] = createSignal<RomEntry | null>(null);
  // Game ▾ menu items that conceptually map to a drawer tab use this
  // signal to land on the right tab when the drawer opens. Cleared when
  // the drawer closes so the next "Properties…" click opens on Overview.
  const [propertiesInitialTab, setPropertiesInitialTab] = createSignal<GameDrawerTab | undefined>(undefined);
  function openGameDrawer(entry: RomEntry, tab?: GameDrawerTab) {
    setPropertiesInitialTab(tab);
    setPropertiesFor(entry);
  }
  // Game-running state — drives game-mode chrome behavior.
  const [gameRunning, setGameRunning] = createSignal(false);
  const [currentRomTitle, setCurrentRomTitle] = createSignal<string | null>(null);
  // Full RomEntry for the running game. Used by Quick Settings (slice 2.8.B)
  // to surface Saves / Game info / Exit-to-library actions against the live
  // entry. Cleared on unload.
  const [runningEntry, setRunningEntry] = createSignal<RomEntry | null>(null);
  // Quick Settings overlay (slice 2.8.B). Replaces the slice-A Esc → library
  // toggle behavior during single-window gameplay.
  const [quickSettingsOpen, setQuickSettingsOpen] = createSignal(false);
  // Help menu dialogs.
  const [helpDialog, setHelpDialog] = createSignal<"shortcuts" | "about" | "debug-log" | null>(null);
  // View → Customize widgets… dialog.
  const [widgetCustomizerOpen, setWidgetCustomizerOpen] = createSignal(false);
  // Tools → Screenshot gallery. Targets the active game (running or
  // focused) at the time of opening; entry stays bound until the dialog
  // closes.
  const [screenshotGalleryFor, setScreenshotGalleryFor] = createSignal<RomEntry | null>(null);
  // Tools → Performance HUD toggle. UI-side render-loop FPS only (v1);
  // emulator-side telemetry will plug into the same overlay when wired.
  const [perfHudVisible, setPerfHudVisible] = createSignal(false);
  // Phase 6 Cross-system slice 3 — Game focus toggle. When true, OA hotkeys
  // (F1/F2/F5/F8/Esc/digits/Backspace) stop firing inside the emu thread
  // so the keyboard-passthrough pump can deliver those keys to the core
  // unchallenged. Hydrated from `get_game_focus` at mount; pushed to Rust
  // via `set_game_focus` on user change; updated reactively when the Rust
  // side toggles via the Ctrl+G hotkey by listening to the
  // `oa://game-focus-changed` event.
  const [gameFocus, setGameFocusSignal] = createSignal(false);
  function toggleGameFocus(next: boolean) {
    setGameFocusSignal(next);
    void invoke("set_game_focus", { active: next });
  }
  onMount(() => {
    void invoke<boolean>("get_game_focus").then((on) => setGameFocusSignal(on));
    let unlisten: (() => void) | undefined;
    void listen<boolean>("oa://game-focus-changed", (e) => {
      setGameFocusSignal(!!e.payload);
    }).then((u) => { unlisten = u; });
    onCleanup(() => unlisten?.());
  });
  // Tools ▾ menu items request the overlay to land on a specific panel.
  // Cleared on close so a subsequent Esc-open lands on the action grid.
  const [quickSettingsRequestedView, setQuickSettingsRequestedView] = createSignal<QuickSettingsView | null>(null);
  function openQuickSettings(view: QuickSettingsView) {
    setQuickSettingsRequestedView(view);
    setQuickSettingsOpen(true);
  }
  // Toolbar idle-hide flag (single-window gameplay).
  const [headerHidden, setHeaderHidden] = createSignal(false);
  // Last-focused library tile — drives the right sidebar widgets when nothing
  // is pinned. Sticky: cleared on library reload, not on tile leave.
  const [focusedEntry, setFocusedEntry] = createSignal<RomEntry | null>(null);
  // Overflow menu state (toolbar … button).
  const [overflowOpen, setOverflowOpen] = createSignal(false);
  // Library menu deep-links into the Library Manager page. The page hosts
  // two tabs (library / media); these menu items pick which one to land on.
  const [libraryManagerInitialTab, setLibraryManagerInitialTab] =
    createSignal<"library" | "media" | undefined>(undefined);
  function openLibraryManager(tab?: "library" | "media") {
    setLibraryManagerInitialTab(tab);
    setCurrentView({ kind: "settings" });
  }
  // Right-click context menu over a system entry in the left sidebar.
  // Open when the user right-clicks a SystemItem; null when closed.
  const [systemContextFor, setSystemContextFor] = createSignal<{
    id: SystemId;
    position: { x: number; y: number };
  } | null>(null);
  // Import wizard modal (Phase 2.7 slice C). The legacy `handlePickFolder`
  // path stays as a single-shot fallback (kept on the Rescan menu item +
  // the drag-drop commit) so users with simple needs aren't forced through
  // the 4-step flow.
  const [wizardOpen, setWizardOpen] = createSignal(false);
  // Search-as-you-type. Filters the active library view by title (in-memory
  // includes match). FTS5 in Rust is wired and ready for >100K libraries;
  // 2.6 ships with the simpler path.
  const [searchQuery, setSearchQuery] = createSignal("");
  // True while a folder is being dragged over the window — drives the
  // drop-overlay UI. Cleared on drop / leave / cancel by the Tauri event.
  const [dropOverlayVisible, setDropOverlayVisible] = createSignal(false);

  // Right-sidebar pinned entry (lookup from id in layout store).
  const pinnedEntry = createMemo<RomEntry | null>(() => {
    const id = layout.rightSidebarPinnedGameId();
    if (!id) return null;
    return library.state.entries.find((e) => e.id === id) ?? null;
  });

  // Game mode = single-window shell + a ROM is running + the library is hidden.
  const gameMode = () =>
    shellMode() === "single-window" && gameRunning() && !libraryVisible();

  onMount(async () => {
    try {
      const mode = (await invoke<string>("get_shell_mode")) as ShellMode;
      document.body.dataset.shell = mode;
      setShellMode(mode);
    } catch (e) {
      console.warn("get_shell_mode failed (assuming two-window):", e);
      document.body.dataset.shell = "two-window";
    }
  });

  // Slice C — populate the shader preset registry signal once, on app
  // mount. Dropdowns in PerSystem/PerGame settings pages render from
  // `shaderPresets()` (live signal) and start with the hardcoded
  // fallback list; this swaps in the Rust-backed registry once it loads.
  // Slice D — also subscribe to `oa://shader-presets-changed` for the
  // hot-reload path. The Rust watcher emits this on any
  // `<exe_dir>/shaders/presets/*.preset.toml` change with the fresh
  // summary list as payload; we update the signal so open dropdowns
  // pick it up immediately. The watcher ALSO re-applies the active
  // preset internally, so a TOML edit takes effect on the next frame
  // without any frontend action.
  onMount(() => {
    void loadShaderPresets();
    const unlisten = listen<ShaderPresetEntry[]>("oa://shader-presets-changed", (e) => {
      applyShaderPresetsUpdate(e.payload);
    });
    onCleanup(() => {
      void unlisten.then((fn) => fn());
    });
  });

  // Keyboard handling — same gate-by-focus rules as before.
  // F1 reset, F2 pause, F3 frame-advance, F5 save, F6 fast-forward,
  // F7 slow-motion, F8 load, F12 screenshot — all consumed by the emu
  // thread; preventDefault here so the browser doesn't open Help / open
  // dev tools / etc.
  const SUPPRESS_DEFAULT = new Set([
    "F1", "F2", "F3", "F5", "F6", "F7", "F8", "F12",
    "Enter", "z", "Z", "x", "X", "Shift",
    "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight",
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9",
  ]);
  const keydownHandler = (e: KeyboardEvent) => {
    if (SUPPRESS_DEFAULT.has(e.key)) {
      const tag = (document.activeElement as HTMLElement | null)?.tagName;
      // BUTTON + SELECT are interactive — Enter on a focused button is THE
      // browser default we want to keep (it activates the button). Without
      // this guard, the Quick Settings overlay's Enter-to-activate behavior
      // breaks because we'd preventDefault the click.
      if (tag !== "INPUT" && tag !== "TEXTAREA" && tag !== "BUTTON" && tag !== "SELECT") {
        e.preventDefault();
      }
    }
    if (
      e.key === "Escape" &&
      shellMode() === "single-window" &&
      gameRunning() &&
      currentView().kind !== "settings"
    ) {
      const tag = (document.activeElement as HTMLElement | null)?.tagName;
      // The QuickSettings component has its own capture-phase Esc listener
      // that handles closing — so when the overlay is open, this branch
      // never runs. When it's closed, this branch opens it.
      if (tag !== "INPUT" && tag !== "TEXTAREA" && !quickSettingsOpen()) {
        e.preventDefault();
        setQuickSettingsOpen(true);
        (document.activeElement as HTMLElement | null)?.blur();
      }
    }
    if ((e.ctrlKey || e.metaKey) && (e.key === "w" || e.key === "W") && gameRunning()) {
      const tag = (document.activeElement as HTMLElement | null)?.tagName;
      if (tag !== "INPUT" && tag !== "TEXTAREA") {
        e.preventDefault();
        void handleUnload();
      }
    }
    // Ctrl+B / Cmd+B toggles left sidebar collapse (VS Code convention).
    if ((e.ctrlKey || e.metaKey) && (e.key === "b" || e.key === "B")) {
      const tag = (document.activeElement as HTMLElement | null)?.tagName;
      if (tag !== "INPUT" && tag !== "TEXTAREA") {
        e.preventDefault();
        layout.setLeftSidebarCollapsed(!layout.leftSidebarCollapsed());
      }
    }
  };
  onMount(() => window.addEventListener("keydown", keydownHandler, { capture: true }));
  onCleanup(() => window.removeEventListener("keydown", keydownHandler, { capture: true }));

  // Rust-emitted Esc → Quick Settings open. The emu thread polls keys
  // globally via device_query and emits this event on Esc rising edge so
  // two-window mode (where the native game window has no WebView to
  // receive keydown events) gets the same overlay affordance that
  // single-window mode has via the keydown handler above. Idempotent —
  // if the overlay is already open or the user is on the Settings page,
  // we silently no-op.
  onMount(() => {
    let unlisten: (() => void) | undefined;
    void listen("oa://request-quick-settings", () => {
      if (!gameRunning()) return;
      if (currentView().kind === "settings") return;
      if (quickSettingsOpen()) return;
      setQuickSettingsOpen(true);
      (document.activeElement as HTMLElement | null)?.blur();
    }).then((un) => (unlisten = un));
    onCleanup(() => unlisten?.());
  });

  // Phase 4 slice F — milestone-triggered events. The Rust side
  // already emits a toast via the in-game toast channel; this listener
  // is for any in-shell consumer that wants to react to unlocks (e.g.
  // re-fetching the per-game milestone list to update its "triggered"
  // badge). For v1 we just log; the PerGameSettingsDrawer's Milestones
  // tab re-fetches on every re-open which is plenty.
  onMount(() => {
    let unlisten: (() => void) | undefined;
    void listen<{ id: number; name: string; triggeredAtUnixMs: number }>("oa://milestone-triggered", (e) => {
      console.log("[oa-milestone] triggered:", e.payload);
    }).then((un) => (unlisten = un));
    onCleanup(() => unlisten?.());
  });

  // Game-mode idle timer — same as before. Only applies in single-window
  // gameplay since two-window keeps the chrome on its own window.
  const IDLE_MS = 3000;
  let idleTimer: number | undefined;
  const armIdle = () => {
    if (idleTimer) {
      window.clearTimeout(idleTimer);
      idleTimer = undefined;
    }
    if (gameMode()) {
      idleTimer = window.setTimeout(() => setHeaderHidden(true), IDLE_MS);
    }
  };
  const handleActivity = () => {
    if (headerHidden()) setHeaderHidden(false);
    armIdle();
  };
  onMount(() => {
    window.addEventListener("mousemove", handleActivity);
    window.addEventListener("keydown", handleActivity);
  });
  onCleanup(() => {
    window.removeEventListener("mousemove", handleActivity);
    window.removeEventListener("keydown", handleActivity);
    if (idleTimer) window.clearTimeout(idleTimer);
  });

  createEffect(() => {
    if (gameMode()) {
      armIdle();
    } else {
      setHeaderHidden(false);
      if (idleTimer) {
        window.clearTimeout(idleTimer);
        idleTimer = undefined;
      }
    }
  });

  createEffect(() => {
    document.body.classList.toggle("oa-game-idle", gameMode() && headerHidden());
  });

  // Pick a default focused entry on first library load so the right sidebar
  // has content to display before the user has hovered anything.
  createEffect(() => {
    if (focusedEntry() === null) {
      const firstReal = library.state.entries.find((e) => !e.seed);
      if (firstReal) setFocusedEntry(firstReal);
    }
  });

  /// Throttled status-bar reporter for in-flight scans. `currentFile` is
  /// short-cut for "show the user the scanner is alive without overwhelming
  /// the bar with paths."
  function scanProgressReporter(p: ScanProgress) {
    const tail = p.currentFile.length > 48 ? `…${p.currentFile.slice(-48)}` : p.currentFile;
    setStatus(`Scanning ${p.folder}: ${p.matches} matched (${p.archived} archived) · ${tail}`);
  }

  async function handlePickFolder() {
    setBusy("scanning");
    setStatus("Scanning folder…");
    const result = await pickFolderAndIngest(library, scanProgressReporter);
    setStatus(ingestStatus(result));
    if (result.kind === "ingested" || result.kind === "empty") {
      const existing = settings.libraryFolders();
      if (!existing.includes(result.folder)) {
        settings.setLibraryFolders([...existing, result.folder]);
      }
    }
    setBusy("idle");
    setOverflowOpen(false);
  }

  async function handleAddLibraryFolder() {
    const picked = await pickDirectory({ directory: true, multiple: false }).catch(() => null);
    if (!picked || Array.isArray(picked)) return;
    const existing = settings.libraryFolders();
    if (!existing.includes(picked)) {
      settings.setLibraryFolders([...existing, picked]);
    }
    setBusy("scanning");
    setStatus(`Scanning ${picked}…`);
    const summary = await rescanFolders(library, [picked], scanProgressReporter);
    setStatus(`Added ${summary.totalAdded} from ${picked}.`);
    setBusy("idle");
  }

  async function handleRescanLibraryFolders() {
    const folders = settings.libraryFolders();
    if (folders.length === 0) return;
    setBusy("scanning");
    setStatus(`Rescanning ${folders.length} folder${folders.length === 1 ? "" : "s"}…`);
    const summary = await rescanFolders(library, folders, scanProgressReporter);
    const errSuffix = summary.errors.length > 0 ? ` (${summary.errors.length} errored)` : "";
    setStatus(`Rescan: added ${summary.totalAdded} new ROMs across ${summary.folders} folders${errSuffix}.`);
    if (summary.errors.length > 0) console.warn("rescan errors:", summary.errors);
    setBusy("idle");
  }

  async function handleLaunch(entry: RomEntry) {
    // Diagnostic — verify the click reached us + log the full entry shape
    // so we can confirm archiveInnerPath/coreOverride/etc. are populated.
    console.log("[oa-launch] handleLaunch called", {
      id: entry.id,
      title: entry.title,
      systemId: entry.systemId,
      filePath: entry.filePath,
      archiveInnerPath: entry.archiveInnerPath,
      coreOverride: entry.coreOverride,
      seed: entry.seed,
    });
    if (entry.seed) {
      console.log("[oa-launch] entry.seed=true → skipping");
      setStatus("Placeholder tile — pick a folder to load real ROMs.");
      return;
    }
    setBusy("launching");
    setStatus(`Launching ${entry.title}…`);

    // Phase 3 slice A + B — resolve the effective Display + shader values
    // from the per-game → per-system → OA-wide inheritance chain and push
    // them to the renderer / shell window before the ROM loads. Soft-
    // failures don't block launch — worst case the renderer keeps its
    // previous values. Region override stays scaffold-only (per-core BIOS
    // region wiring is per-core work; the override persists and is
    // available for individual core loaders once they consume it).
    try {
      type SysSettings = {
        shaderPreset?: string | null;
        bloomAmount?: number | null;
        scalingOverride?: string | null;
        windowModeOverride?: string | null;
        monitorIndexOverride?: number | null;
        rewindEnabled?: boolean | null;
        rewindCaptureIntervalFrames?: number | null;
        rewindBufferMegabytes?: number | null;
      };
      type GameOver = SysSettings & {
        regionOverride?: string | null;
      };
      const [sys, game] = await Promise.all([
        invoke<SysSettings>("get_system_settings", { systemId: entry.systemId }).catch(() => ({} as SysSettings)),
        invoke<GameOver>("get_game_overrides", { id: entry.id }).catch(() => ({} as GameOver)),
      ]);
      const effective = {
        // Per-system shaderPreset chain (Phase 3 slice C polish): per-game
        // override → per-system override → OA-wide (which may be the
        // `"system-default"` sentinel → registry per-system default).
        shaderPreset:
          game?.shaderPreset
          ?? sys?.shaderPreset
          ?? resolveShaderPreset(settings.shaderPreset(), entry.systemId as SystemId),
        // Slice C polish — bloom_amount inheritance: per-game → per-system →
        // OA-wide → preset TOML default. OA-wide value is the user-controllable
        // root; the TOML default is the floor under that. settings.bloomAmount()
        // always returns a number, so this branch always sends.
        bloomAmount: game?.bloomAmount ?? sys?.bloomAmount ?? settings.bloomAmount(),
        scaling: game?.scalingOverride ?? sys?.scalingOverride ?? settings.scalingMode(),
        windowMode: game?.windowModeOverride ?? sys?.windowModeOverride ?? settings.windowMode(),
        monitor: game?.monitorIndexOverride ?? sys?.monitorIndexOverride ?? settings.monitorIndex(),
        rewindEnabled: game?.rewindEnabled ?? sys?.rewindEnabled ?? settings.rewindEnabled(),
        rewindCaptureIntervalFrames:
          game?.rewindCaptureIntervalFrames
          ?? sys?.rewindCaptureIntervalFrames
          ?? settings.rewindCaptureIntervalFrames(),
        rewindBufferMegabytes:
          game?.rewindBufferMegabytes
          ?? sys?.rewindBufferMegabytes
          ?? settings.rewindBufferMegabytes(),
      };
      console.log("[oa-launch] resolved overrides:", effective, "region:", game?.regionOverride ?? "inherit");
      // Order matters between set_shader_preset + set_bloom_amount: the
      // preset apply on the emu thread writes the TOML's default
      // bloom_amount; the override has to land AFTER so it wins. Awaiting
      // shader preset before queuing the override keeps the EmuCommand
      // channel ordered the way the emu loop sees it.
      await invoke("set_shader_preset", { preset: effective.shaderPreset }).catch((e) =>
        console.warn("[oa-launch] set_shader_preset failed:", e),
      );
      await invoke("set_bloom_amount", { amount: effective.bloomAmount }).catch((e) =>
        console.warn("[oa-launch] set_bloom_amount failed:", e),
      );
      // RetroArch-parity slice — push the merged per-system + per-game
      // core-option overrides to the running core. The emu thread's
      // LoadRom handler already applied per-system defaults; this overlays
      // any per-game values on top. No-op if the core hasn't registered
      // any options yet (first launch ever for this system).
      await invoke("apply_game_core_options", { gameId: entry.id }).catch((e) =>
        console.warn("[oa-launch] apply_game_core_options failed:", e),
      );
      await Promise.all([
        invoke("set_scaling_mode", { mode: effective.scaling }).catch((e) =>
          console.warn("[oa-launch] set_scaling_mode failed:", e),
        ),
        invoke("set_window_mode", { mode: effective.windowMode, monitorIndex: effective.monitor }).catch((e) =>
          console.warn("[oa-launch] set_window_mode failed:", e),
        ),
        invoke("set_rewind_config", {
          enabled: effective.rewindEnabled,
          captureIntervalFrames: effective.rewindCaptureIntervalFrames,
          maxMegabytes: effective.rewindBufferMegabytes,
        }).catch((e) => console.warn("[oa-launch] set_rewind_config failed:", e)),
      ]);
    } catch (e) {
      console.warn("[oa-launch] override resolution failed:", e);
    }

    const result = await launchRom(entry);
    console.log("[oa-launch] launchRom result:", result);
    setStatus(launchStatus(result));
    setBusy("idle");
    if (result.kind === "launched") {
      // Phase 4 slice F — arm any per-game milestones in the emu
      // thread's runtime evaluator. Soft failure — recording the
      // milestones in SQLite is the source of truth; the live
      // evaluator just doesn't fire toasts until a re-launch.
      void invoke<number>("arm_milestones", { gameId: entry.id })
        .then((n) => {
          if (n > 0) console.log(`[oa-launch] armed ${n} milestone(s)`);
        })
        .catch((e) => console.warn("[oa-launch] arm_milestones failed:", e));
      // RetroArch parity slice 5 — arm per-game cheats. Same soft-failure
      // story as milestones (SQLite is source of truth; emu-thread
      // runtime is the live evaluator that runs on next launch otherwise).
      void invoke<number>("arm_cheats", { gameId: entry.id })
        .then((n) => {
          if (n > 0) console.log(`[oa-launch] armed ${n} cheat(s)`);
        })
        .catch((e) => console.warn("[oa-launch] arm_cheats failed:", e));
      setGameRunning(true);
      setCurrentRomTitle(entry.title);
      setRunningEntry(entry);
      if (shellMode() === "single-window") {
        setLibraryVisible(false);
        (document.activeElement as HTMLElement | null)?.blur();
      }
    }
  }

  async function handleUnload() {
    if (!gameRunning()) return;
    const title = currentRomTitle();
    try {
      await invoke("unload_rom", { title });
      setGameRunning(false);
      setCurrentRomTitle(null);
      setRunningEntry(null);
      setQuickSettingsOpen(false);
      if (shellMode() === "single-window") {
        setLibraryVisible(true);
      }
      // Phase 3 slice B — revert renderer + window state to OA-wide defaults
      // so the NEXT launch (which may have no per-game override) doesn't
      // inherit stale state from the game that just unloaded. The settings
      // store's createEffect would catch some of these on subsequent signal
      // changes, but pushing them explicitly here is the safer guarantee.
      void Promise.all([
        invoke("set_shader_preset", { preset: settings.shaderPreset() }).catch(() => {}),
        invoke("set_scaling_mode", { mode: settings.scalingMode() }).catch(() => {}),
        invoke("set_window_mode", { mode: settings.windowMode(), monitorIndex: settings.monitorIndex() }).catch(() => {}),
        invoke("set_rewind_config", {
          enabled: settings.rewindEnabled(),
          captureIntervalFrames: settings.rewindCaptureIntervalFrames(),
          maxMegabytes: settings.rewindBufferMegabytes(),
        }).catch(() => {}),
      ]);
    } catch (e) {
      console.warn("unload_rom failed:", e);
      setStatus(`Unload failed: ${e}`);
    }
  }

  // Context for the contextual menus: System ▾ is disabled unless the user
  // is viewing a system-filtered library or a per-system settings page;
  // Game ▾ is disabled unless a tile is focused, a ROM is running, or one
  // is pinned in the right sidebar.
  const activeSystemId = createMemo<SystemId | null>(() => {
    const cv = currentView();
    if (cv.kind === "system") return cv.id;
    return null;
  });
  const activeGameEntry = createMemo<RomEntry | null>(() => {
    return runningEntry() ?? focusedEntry() ?? pinnedEntry();
  });

  const toolbarLeft = (
    <>
      <button
        type="button"
        onClick={(e) => {
          e.currentTarget.blur();
          setCurrentView({ kind: "all" });
        }}
        class="rounded px-2 py-1 text-base font-bold text-(--color-system-accent) transition hover:bg-white/5"
        title="Home"
      >
        ◐
      </button>
      <MenuBar>
        <Menu label="Library">
          <MenuItem
            label={busy() === "scanning" ? "Scanning…" : "Import folder…"}
            disabled={busy() !== "idle"}
            onClick={() => setWizardOpen(true)}
          />
          <MenuItem
            label="Rescan tracked folders"
            disabled={settings.libraryFolders().length === 0 || busy() !== "idle"}
            onClick={() => void handleRescanLibraryFolders()}
          />
          <MenuDivider />
          <MenuItem label="Library Manager…" onClick={() => openLibraryManager("library")} />
          <MenuItem label="Sync media…" onClick={() => openLibraryManager("media")} />
          <MenuItem label="Cores Manager…" onClick={() => setCurrentView({ kind: "cores" })} />
          <MenuDivider />
          <MenuCheckbox
            label="Auto-hide empty systems"
            checked={layout.autoHideEmptySystems()}
            onChange={(next) => layout.setAutoHideEmptySystems(next)}
          />
          <MenuCheckbox
            label="Auto-remove on file delete"
            checked={settings.autoRemoveOnDelete()}
            onChange={(next) => settings.setAutoRemoveOnDelete(next)}
          />
        </Menu>
        <Menu label="View">
          <MenuRadio<ViewMode>
            label="View mode"
            value={layout.viewMode()}
            onChange={(v) => layout.setViewMode(v)}
            options={VIEW_MODE_OPTIONS.map((m) => ({ value: m, label: VIEW_MODE_LABELS[m] }))}
          />
          <MenuDivider />
          <MenuRadio<SortKey>
            label="Sort by"
            value={layout.sortKey()}
            onChange={(v) => layout.setSortKey(v)}
            options={SORT_KEY_OPTIONS.map((k) => ({ value: k, label: SORT_KEY_LABELS[k] }))}
          />
          <MenuDivider />
          <MenuRadio<GroupBy>
            label="Group by"
            value={layout.groupBy()}
            onChange={(v) => layout.setGroupBy(v)}
            options={GROUP_BY_OPTIONS.map((g) => ({ value: g, label: GROUP_BY_LABELS[g] }))}
          />
          <MenuDivider />
          <MenuCheckbox
            label="Left sidebar"
            hint="Ctrl+B"
            checked={!layout.leftSidebarCollapsed()}
            onChange={(next) => layout.setLeftSidebarCollapsed(!next)}
          />
          <MenuCheckbox
            label="Right sidebar"
            checked={layout.rightSidebarVisible()}
            onChange={(next) => layout.setRightSidebarVisible(next)}
          />
          <MenuItem
            label="Customize widgets…"
            onClick={() => setWidgetCustomizerOpen(true)}
          />
          <MenuDivider />
          <MenuRadio<PresentationMode>
            label="Mode"
            value={layout.presentationMode()}
            onChange={(v) => layout.setPresentationMode(v)}
            options={PRESENTATION_OPTIONS.map((m) => ({ value: m, label: PRESENTATION_LABELS[m] }))}
          />
        </Menu>
        <Menu
          label="System"
          disabled={activeSystemId() === null}
          disabledHint="Pick a system in the sidebar"
        >
          <Show when={activeSystemId()}>
            {(id) => (
              <>
                <MenuLabel>{id()}</MenuLabel>
                <MenuDivider />
                <MenuItem
                  label="Show library"
                  onClick={() => setCurrentView({ kind: "system", id: id() })}
                />
                <MenuDivider />
                <MenuItem label="Bindings…" onClick={() => openSystemDialog("bindings", id())} />
                <MenuItem label="Default core…" onClick={() => openSystemDialog("default-core", id())} />
                <MenuItem label="Shaders…" onClick={() => openSystemDialog("shaders", id())} />
                <MenuItem label="Core options…" onClick={() => openSystemDialog("core-options", id())} />
                <MenuItem label="Rewind overrides…" onClick={() => openSystemDialog("rewind", id())} />
                <MenuItem label="Display overrides…" onClick={() => openSystemDialog("display", id())} />
                <MenuDivider />
                <MenuItem
                  label="Hide from sidebar"
                  onClick={() => {
                    const cur = layout.hiddenSystems();
                    if (!cur.includes(id())) layout.setHiddenSystems([...cur, id()]);
                    if (currentView().kind === "system" && (currentView() as { id: string }).id === id()) {
                      setCurrentView({ kind: "all" });
                    }
                  }}
                />
              </>
            )}
          </Show>
        </Menu>
        <Menu
          label="Game"
          disabled={activeGameEntry() === null}
          disabledHint="Focus a game tile, or start playing one"
        >
          <Show when={activeGameEntry()}>
            {(entry) => (
              <>
                <MenuLabel>{entry().title}</MenuLabel>
                <MenuDivider />
                <Show
                  when={runningEntry() && runningEntry()!.id === entry().id}
                  fallback={
                    <MenuItem label="Launch" onClick={() => void handleLaunch(entry())} />
                  }
                >
                  <MenuItem
                    label="Exit to library"
                    hint="Ctrl+W"
                    destructive
                    onClick={() => void handleUnload()}
                  />
                </Show>
                <MenuDivider />
                <MenuItem label="Save states…" onClick={() => setSavesEntry(entry())} />
                <MenuItem label="Game info…" onClick={() => setGameInfoFor(entry())} />
                <MenuItem label="Properties…" onClick={() => openGameDrawer(entry())} />
                <MenuDivider />
                <MenuItem label="Cheats…" onClick={() => openGameDrawer(entry(), "cheats")} />
                <MenuItem label="Milestones…" onClick={() => openGameDrawer(entry(), "milestones")} />
                <MenuItem label="Shaders…" onClick={() => openGameDrawer(entry(), "shaders")} />
                <MenuItem label="Rewind overrides…" onClick={() => openGameDrawer(entry(), "rewind")} />
                <MenuItem label="ROM patch…" onClick={() => openGameDrawer(entry(), "core")} />
                <MenuItem label="Display overrides…" onClick={() => openGameDrawer(entry(), "display")} />
                <MenuItem label="Core options…" onClick={() => openGameDrawer(entry(), "core-options")} />
                <MenuDivider />
                <MenuItem label="Pick region…" onClick={() => setRegionPickerFor(entry())} />
              </>
            )}
          </Show>
        </Menu>
        <Menu label="Tools">
          <MenuItem
            label="Rewind…"
            disabled={!gameRunning()}
            onClick={() => openQuickSettings("rewind")}
          />
          <MenuItem
            label="TAS recorder…"
            disabled={!gameRunning()}
            onClick={() => openQuickSettings("tas")}
          />
          <MenuItem
            label="Video capture…"
            disabled={!gameRunning()}
            onClick={() => openQuickSettings("video")}
          />
          <MenuItem
            label="Memory inspector…"
            disabled={!gameRunning()}
            onClick={() => openQuickSettings("memory")}
          />
          <MenuItem
            label="Disc control…"
            disabled={!gameRunning()}
            onClick={() => openQuickSettings("disc")}
          />
          <MenuDivider />
          <MenuItem
            label="Screenshot gallery…"
            disabled={activeGameEntry() === null}
            onClick={() => {
              const entry = activeGameEntry();
              if (entry) setScreenshotGalleryFor(entry);
            }}
          />
          <MenuCheckbox
            label="Performance HUD"
            checked={perfHudVisible()}
            onChange={(next) => setPerfHudVisible(next)}
          />
          <MenuCheckbox
            label="Game focus"
            hint="Ctrl+G"
            checked={gameFocus()}
            onChange={(next) => toggleGameFocus(next)}
          />
        </Menu>
        <Menu label="Settings">
          <MenuItem label="Display…" onClick={() => setSettingsDialog("display")} />
          <MenuItem label="Audio…" onClick={() => setSettingsDialog("audio")} />
          <MenuItem label="Gameplay…" onClick={() => setSettingsDialog("gameplay")} />
          <MenuItem label="Shaders…" onClick={() => setSettingsDialog("shaders")} />
          <MenuDivider />
          <MenuRadio<ShellMode>
            label="Shell mode"
            value={settings.shellModePref()}
            onChange={(v) => settings.setShellModePref(v)}
            options={SHELL_OPTIONS.map((m) => ({ value: m, label: SHELL_MODE_LABELS[m] }))}
          />
        </Menu>
        <Menu label="Help">
          <MenuItem label="Debug log…" onClick={() => setHelpDialog("debug-log")} />
          <MenuDivider />
          <MenuItem label="Keyboard shortcuts…" onClick={() => setHelpDialog("shortcuts")} />
          <MenuItem label="About Overlooked Arcade…" onClick={() => setHelpDialog("about")} />
        </Menu>
      </MenuBar>
    </>
  );

  const toolbarCenter = (
    <div class="flex w-full max-w-md items-center gap-2">
      <input
        type="search"
        placeholder="Search games…"
        value={searchQuery()}
        onInput={(e) => setSearchQuery(e.currentTarget.value)}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            setSearchQuery("");
            (e.currentTarget as HTMLInputElement).blur();
          }
        }}
        class="w-full rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs text-(--color-oa-ink) placeholder:text-(--color-oa-ink-dim) focus-visible:border-(--color-system-accent) focus-visible:outline-none"
      />
      <Show when={status()}>
        <p class="hidden truncate text-[0.7rem] text-(--color-oa-ink-dim) lg:block">{status()}</p>
      </Show>
    </div>
  );

  const toolbarRight = (
    <>
      <Show when={gameFocus()}>
        <span
          title="Game focus is ON — OA hotkeys pass through to the core. Press Ctrl+G to disable."
          class="rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 px-2 py-1 text-[0.6rem] font-semibold uppercase tracking-wider text-(--color-system-accent)"
        >
          Game focus
        </span>
      </Show>
      <Show when={shellMode() === "single-window"}>
        <button
          type="button"
          onClick={(e) => {
            setLibraryVisible((v) => !v);
            e.currentTarget.blur();
          }}
          class={TOOLBAR_BTN}
          aria-pressed={libraryVisible()}
          title={gameRunning() ? "Toggle library (Esc)" : undefined}
        >
          {libraryVisible() ? "Hide" : "Show"}
        </button>
      </Show>
      <Show when={gameRunning()}>
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            void handleUnload();
          }}
          class={TOOLBAR_BTN}
          title="Unload ROM (Ctrl+W)"
        >
          Unload
        </button>
      </Show>
      <Show when={!layout.rightSidebarVisible() && layout.presentationMode() !== "cabinet"}>
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            layout.setRightSidebarVisible(true);
          }}
          class={TOOLBAR_BTN}
          title="Show right sidebar"
        >
          ‹
        </button>
      </Show>
      <button
        type="button"
        onClick={(e) => {
          e.currentTarget.blur();
          void invoke("quit_app");
        }}
        class={TOOLBAR_BTN}
        title="Quit (Ctrl+Q)"
      >
        ✕
      </button>
    </>
  );

  // Click anywhere outside the overflow menu closes it.
  onMount(() => {
    const onDocClick = () => {
      if (overflowOpen()) setOverflowOpen(false);
    };
    window.addEventListener("click", onDocClick);
    onCleanup(() => window.removeEventListener("click", onDocClick));
  });

  // Filesystem watcher — register the tracked-folder list with the Rust
  // watcher service whenever it changes. New ROMs dropped into a watched
  // folder fire `oa://library-watch-found` and auto-add to the library.
  // Deletions fire `oa://library-watch-removed` but we keep entries by
  // default (user might be moving / renaming).
  createEffect(() => {
    const folders = settings.libraryFolders();
    const extensions = allSupportedExtensions();
    invoke("set_watched_folders", { folders, extensions }).catch((e) =>
      console.warn("[oa-watch] set_watched_folders failed:", e),
    );
  });

  onMount(async () => {
    let unlistenFound: (() => void) | undefined;
    let unlistenRemoved: (() => void) | undefined;
    try {
      unlistenFound = await listen<{
        path: string;
        fileName: string;
        extension: string;
        archiveInnerPath?: string;
      }>("oa://library-watch-found", async (event) => {
        const r = event.payload;
        const systemId = systemForExtension(r.extension);
        if (!systemId) return;
        console.log("[oa-watch] new ROM detected:", r.fileName);
        await library.addScannedRoms([{
          id: romIdFromPath(r.path),
          title: titleFromFileName(r.fileName),
          systemId,
          filePath: r.path,
          addedAt: Date.now(),
          ...(r.archiveInnerPath ? { archiveInnerPath: r.archiveInnerPath } : {}),
        }]);
      });
      unlistenRemoved = await listen<{ path: string }>(
        "oa://library-watch-removed",
        async (event) => {
          // Soft policy by default: keep the entry (user might be moving
          // the file). Settings → Library → "Auto-remove on file delete"
          // flips this to a hard policy where the matching DB row gets
          // removed when the watcher reports the file gone.
          if (!settings.autoRemoveOnDelete()) {
            console.log("[oa-watch] file removed (kept in library):", event.payload.path);
            return;
          }
          try {
            const id = await invoke<string | null>("find_game_id_by_path", {
              path: event.payload.path,
            });
            if (id) {
              await library.remove(id);
              console.log("[oa-watch] auto-removed from library:", event.payload.path, "->", id);
            }
          } catch (e) {
            console.warn("[oa-watch] auto-remove failed:", e);
          }
        },
      );
    } catch (e) {
      console.warn("[oa-watch] listener setup failed:", e);
    }
    onCleanup(() => {
      unlistenFound?.();
      unlistenRemoved?.();
    });
  });

  // Diagnostic — log EVERY click reaching the document, with the target's
  // tag, id, classes, and chain of parents. This will show whether menu
  // item clicks are hitting the <button> or some other element.
  onMount(() => {
    const onAnyClick = (e: MouseEvent) => {
      const target = e.target as HTMLElement | null;
      if (!target) return;
      const chain: string[] = [];
      let n: HTMLElement | null = target;
      let depth = 0;
      while (n && depth < 6) {
        const tag = n.tagName;
        const role = n.getAttribute?.("role") ?? "";
        const cls = (n.className && typeof n.className === "string") ? n.className.slice(0, 60) : "";
        chain.push(`${tag}${role ? `[role=${role}]` : ""}${cls ? `.${cls.replace(/\s+/g, ".")}` : ""}`);
        n = n.parentElement;
        depth++;
      }
      console.log("[oa-click]", chain.join(" > "));
    };
    window.addEventListener("click", onAnyClick, { capture: true });
    onCleanup(() => window.removeEventListener("click", onAnyClick, { capture: true }));
  });

  // Window-level folder drag-drop. Two layers:
  //
  //   1) DOM-level dragover/dragleave listeners that call preventDefault to
  //      tell WebView2 "yes, we accept drops" — without this Chromium shows
  //      a no-entry cursor and never fires drop events upstream. These also
  //      drive the overlay UI.
  //
  //   2) Tauri's onDragDropEvent which carries the OS-native paths the
  //      browser File API never exposes (Chromium removed the `.path`
  //      property). When Tauri's OS handler intercepts the drop, it emits
  //      events here.
  //
  // The drop is committed via the Tauri path because DOM drop events in
  // WebView2 don't give us file paths. If the Tauri handler doesn't fire
  // (some Windows configurations leave it dormant), the DOM listener still
  // gives the user visual feedback and a hint.
  async function commitDroppedPath(path: string) {
    setBusy("scanning");
    setStatus(`Scanning ${path}…`);
    const result = await ingestFolderPath(library, path, scanProgressReporter);
    setStatus(ingestStatus(result));
    if (result.kind === "ingested" || result.kind === "empty") {
      const existing = settings.libraryFolders();
      if (!existing.includes(result.folder)) {
        settings.setLibraryFolders([...existing, result.folder]);
      }
    }
    setBusy("idle");
  }

  onMount(async () => {
    // Layer 1 — DOM events. preventDefault on dragover is the load-bearing
    // one: it overrides WebView2's default "no drop allowed" cursor.
    const onDragEnter = (e: DragEvent) => {
      e.preventDefault();
      setDropOverlayVisible(true);
    };
    const onDragOver = (e: DragEvent) => {
      e.preventDefault();
      if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
      setDropOverlayVisible(true);
    };
    const onDragLeave = (e: DragEvent) => {
      // Only clear when leaving the window itself, not when dragging across
      // child elements (which fire dragleave on the parent).
      if ((e as any).target === document.documentElement || (e as any).fromElement === null) {
        setDropOverlayVisible(false);
      }
    };
    const onDrop = (e: DragEvent) => {
      e.preventDefault();
      setDropOverlayVisible(false);
      console.log("[oa-drop] DOM drop fired. files=", e.dataTransfer?.files?.length, "items=", e.dataTransfer?.items?.length);
      // We don't try to extract paths here — WebView2 doesn't expose them.
      // The Tauri onDragDropEvent fires separately with the real OS paths.
      // If Tauri's handler isn't firing, surface a hint to the user.
      if ((e.dataTransfer?.files?.length ?? 0) > 0) {
        setStatus("Drop received — waiting for Tauri to deliver the path…");
        // Fallback safety: if Tauri's handler doesn't fire within 500ms,
        // tell the user drag-drop isn't working in this build and to use the
        // toolbar overflow → Import folder instead.
        setTimeout(() => {
          if (busy() !== "scanning") {
            setStatus("Drag-drop not delivering paths — use ⋯ → Import folder, or Settings → Library → Add.");
          }
        }, 500);
      }
    };
    window.addEventListener("dragenter", onDragEnter);
    window.addEventListener("dragover", onDragOver);
    window.addEventListener("dragleave", onDragLeave);
    window.addEventListener("drop", onDrop);
    onCleanup(() => {
      window.removeEventListener("dragenter", onDragEnter);
      window.removeEventListener("dragover", onDragOver);
      window.removeEventListener("dragleave", onDragLeave);
      window.removeEventListener("drop", onDrop);
    });

    // Layer 2 — Tauri OS-level drag-drop. This is what actually gives us
    // file paths. When it fires (default behavior), we commit the dropped
    // folder; when it doesn't (some Windows configurations), the DOM
    // listener above shows the fallback hint.
    let unlisten: (() => void) | undefined;
    console.log("[oa-drop] registering onDragDropEvent listener");
    try {
      unlisten = await getCurrentWebview().onDragDropEvent(async (event) => {
        console.log("[oa-drop] tauri event:", event.payload.type, event.payload);
        switch (event.payload.type) {
          case "enter":
          case "over":
            setDropOverlayVisible(true);
            break;
          case "leave":
            setDropOverlayVisible(false);
            break;
          case "drop": {
            setDropOverlayVisible(false);
            const paths = event.payload.paths;
            console.log("[oa-drop] tauri paths received:", paths);
            if (paths.length === 0) {
              console.warn("[oa-drop] drop event had empty paths array");
              break;
            }
            await commitDroppedPath(paths[0]);
            break;
          }
        }
      });
      console.log("[oa-drop] listener registered OK");
    } catch (e) {
      console.warn("[oa-drop] onDragDropEvent setup failed:", e);
    }
    onCleanup(() => { unlisten?.(); });
  });

  return (
    <MediaProvider>
      <Shell
        layout={layout}
        toolbar={
          <TopToolbar
            left={toolbarLeft}
            center={toolbarCenter}
            right={toolbarRight}
            hidden={headerHidden()}
          />
        }
        leftSidebar={
          <LeftSidebar
            layout={layout}
            library={library}
            currentView={currentView()}
            onNavigate={(v) => setCurrentView(v)}
            onSystemContext={(id, position) => setSystemContextFor({ id, position })}
          />
        }
        rightSidebar={
          <RightSidebar
            layout={layout}
            focused={focusedEntry}
            pinned={pinnedEntry}
            onLaunch={(e) => void handleLaunch(e)}
            onShowSaves={(e) => setSavesEntry(e)}
            onShowInfo={(e) => setGameInfoFor(e)}
          />
        }
      >
        <main class="relative h-full">
          <Switch
            fallback={
              <div
                class="oa-library-fade h-full"
                classList={{
                  "is-hidden": !libraryVisible(),
                  "oa-library-overlay":
                    shellMode() === "single-window" && gameRunning(),
                }}
                aria-hidden={!libraryVisible()}
              >
                <LibraryView
                  library={library}
                  layout={layout}
                  currentView={currentView()}
                  searchQuery={searchQuery()}
                  onLaunch={handleLaunch}
                  onShowSaves={(entry) => setSavesEntry(entry)}
                  onPickContext={(entry, position) => setContextMenuFor({ entry, position })}
                  onFocus={(entry) => setFocusedEntry(entry)}
                  onPickFolder={handlePickFolder}
                />
              </div>
            }
          >
            <Match when={currentView().kind === "settings"}>
              <div class="h-full overflow-y-auto">
                <SettingsPage
                  onBack={() => setCurrentView({ kind: "all" })}
                  settings={settings}
                  library={library}
                  layout={layout}
                  onAddLibraryFolder={handleAddLibraryFolder}
                  onRescanLibraryFolders={handleRescanLibraryFolders}
                  initialTab={libraryManagerInitialTab()}
                />
              </div>
            </Match>
            <Match when={currentView().kind === "cores"}>
              <div class="h-full overflow-y-auto">
                <CoresPage onBack={() => setCurrentView({ kind: "all" })} />
              </div>
            </Match>
          </Switch>
          <Show when={gameMode() && headerHidden()}>
            <div class="pointer-events-none fixed bottom-3 right-4 z-10 rounded-md bg-black/50 px-2 py-1 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim) backdrop-blur">
              Esc · library
            </div>
          </Show>
        </main>
        <Show when={dropOverlayVisible()}>
          {/* Folder-drop overlay. Pointer-events:none lets the underlying
              Tauri drag-drop logic handle the actual drop without our DOM
              capturing it. The Tauri payload, not the DOM event, fires the
              ingest. */}
          <div class="pointer-events-none fixed inset-0 z-50 grid place-items-center bg-(--color-oa-bg-deep)/80 backdrop-blur-sm">
            <div class="rounded-xl border-2 border-dashed border-(--color-system-accent) bg-(--color-oa-bg)/80 px-12 py-10 text-center">
              <p class="text-5xl text-(--color-system-accent)">⇩</p>
              <p class="mt-3 text-sm font-semibold uppercase tracking-[0.4em] text-(--color-oa-ink)">
                Drop folder to import
              </p>
              <p class="mt-1 text-[0.7rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                ROMs will be scanned and added
              </p>
            </div>
          </div>
        </Show>
      </Shell>
      <ImportWizard
        open={wizardOpen()}
        onClose={() => setWizardOpen(false)}
        library={library}
        settings={settings}
        onStatus={(s) => setStatus(s)}
      />
      <QuickSettings
        open={quickSettingsOpen()}
        onClose={() => {
          setQuickSettingsOpen(false);
          setQuickSettingsRequestedView(null);
        }}
        entry={runningEntry()}
        settings={settings}
        onShowSaves={(entry) => setSavesEntry(entry)}
        onShowInfo={(entry) => setGameInfoFor(entry)}
        onExitToLibrary={() => void handleUnload()}
        requestedView={quickSettingsRequestedView()}
      />
      <SaveSlotsModal
        entry={savesEntry()}
        onClose={() => setSavesEntry(null)}
        onLaunchedFromSlot={(entry, slot) => {
          setStatus(`Launched ${entry.title} (slot ${slot}).`);
          setGameRunning(true);
          setCurrentRomTitle(entry.title);
          setRunningEntry(entry);
          if (shellMode() === "single-window") {
            setLibraryVisible(false);
            (document.activeElement as HTMLElement | null)?.blur();
          }
        }}
      />
      <SystemContextMenu
        systemId={systemContextFor()?.id ?? null}
        position={systemContextFor()?.position ?? null}
        library={library}
        onClose={() => setSystemContextFor(null)}
        onShowLibrary={(id) => setCurrentView({ kind: "system", id })}
        onOpenBindings={(id) => openSystemDialog("bindings", id)}
        onOpenSettings={(id) => openSystemDialog("display", id)}
        onHideSystem={(id) => {
          const current = layout.hiddenSystems();
          if (!current.includes(id)) {
            layout.setHiddenSystems([...current, id]);
          }
          // If the user hid the system they were viewing, kick them back
          // to "All games" so they aren't stranded on a sidebar entry
          // that just disappeared.
          if (currentView().kind === "system" && (currentView() as { id: string }).id === id) {
            setCurrentView({ kind: "all" });
          }
        }}
      />
      <TileContextMenu
        entry={contextMenuFor()?.entry ?? null}
        position={contextMenuFor()?.position ?? null}
        library={library}
        onClose={() => setContextMenuFor(null)}
        onLaunch={(entry) => void handleLaunch(entry)}
        onShowSaves={(entry) => setSavesEntry(entry)}
        onShowGameInfo={(entry) => setGameInfoFor(entry)}
        onPickRegion={(entry) => setRegionPickerFor(entry)}
        onPickCore={(entry, position) => setCoreMenuFor({ entry, position })}
        onOpenProperties={(entry) => setPropertiesFor(entry)}
      />
      <PerGameSettingsDrawer
        open={propertiesFor() !== null}
        entry={propertiesFor()}
        onClose={() => {
          setPropertiesFor(null);
          setPropertiesInitialTab(undefined);
        }}
        settings={settings}
        library={library}
        initialTab={propertiesInitialTab()}
      />
      <GameInfoModal
        entry={gameInfoFor()}
        onClose={() => setGameInfoFor(null)}
        onLaunched={(entry, slot) => {
          setStatus(
            slot !== undefined
              ? `Launched ${entry.title} (slot ${slot}).`
              : `Launched ${entry.title}.`,
          );
          setGameRunning(true);
          setCurrentRomTitle(entry.title);
          setRunningEntry(entry);
          if (shellMode() === "single-window") {
            setLibraryVisible(false);
            (document.activeElement as HTMLElement | null)?.blur();
          }
        }}
      />
      <CorePickerMenu
        entry={coreMenuFor()?.entry ?? null}
        position={coreMenuFor()?.position ?? null}
        library={library}
        onClose={() => setCoreMenuFor(null)}
      />
      <RegionPicker
        entry={regionPickerFor()}
        onClose={() => setRegionPickerFor(null)}
      />
      <DisplayDialog
        open={settingsDialog() === "display"}
        onClose={() => setSettingsDialog(null)}
        settings={settings}
      />
      <AudioDialog
        open={settingsDialog() === "audio"}
        onClose={() => setSettingsDialog(null)}
        settings={settings}
      />
      <GameplayDialog
        open={settingsDialog() === "gameplay"}
        onClose={() => setSettingsDialog(null)}
        settings={settings}
      />
      <ShadersDialog
        open={settingsDialog() === "shaders"}
        onClose={() => setSettingsDialog(null)}
        settings={settings}
      />
      <Show when={systemDialog()} keyed>
        {(sd) => (
          <Switch>
            <Match when={sd.section === "bindings"}>
              <SystemBindingsDialog
                open
                systemId={sd.target}
                onClose={() => setSystemDialog(null)}
              />
            </Match>
            <Match when={sd.section === "core-options"}>
              <SystemCoreOptionsDialog
                open
                systemId={sd.target}
                onClose={() => setSystemDialog(null)}
              />
            </Match>
            <Match when={true}>
              <SystemSettingsDialog
                open
                section={sd.section}
                systemId={sd.target}
                onClose={() => setSystemDialog(null)}
                settings={settings}
              />
            </Match>
          </Switch>
        )}
      </Show>
      <WidgetCustomizerDialog
        open={widgetCustomizerOpen()}
        onClose={() => setWidgetCustomizerOpen(false)}
        layout={layout}
      />
      <ScreenshotGalleryDialog
        open={screenshotGalleryFor() !== null}
        onClose={() => setScreenshotGalleryFor(null)}
        entry={screenshotGalleryFor()}
      />
      <PerformanceHud visible={perfHudVisible()} />
      <KeyboardShortcutsDialog
        open={helpDialog() === "shortcuts"}
        onClose={() => setHelpDialog(null)}
      />
      <AboutDialog
        open={helpDialog() === "about"}
        onClose={() => setHelpDialog(null)}
      />
      <DebugLogDialog
        open={helpDialog() === "debug-log"}
        onClose={() => setHelpDialog(null)}
      />
      <ToastStack />
    </MediaProvider>
  );
};

export default App;
