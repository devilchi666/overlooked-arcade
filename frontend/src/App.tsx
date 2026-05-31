import { createEffect, createMemo, createResource, createSignal, Match, onCleanup, onMount, Show, Switch, type Component } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open as pickDirectory } from "@tauri-apps/plugin-dialog";
import CorePickerMenu from "./components/CorePickerMenu";
import GameInfoModal from "./components/GameInfoModal";
import ImportWizard from "./components/ImportWizard";
import LibraryView from "./components/LibraryView";
import SystemBackground from "./components/SystemBackground";
import SystemBootAnimation from "./components/SystemBootAnimation";
import StylusOverlay from "./components/StylusOverlay";
import GamePropertiesDialog from "./components/GamePropertiesDialog";
import {
  CheatsDialog,
  GameCoreOptionsDialog,
  GameDisplayDialog,
  GameInputDialog,
  GameRewindDialog,
  GameShadersDialog,
  MilestonesDialog,
  type GameDialogState,
} from "./components/GameDialogs";
import CoresPage from "./components/CoresPage";
import QuickSettings, { type QuickSettingsView } from "./components/QuickSettings";
import SaveSlotsModal from "./components/SaveSlotsModal";
import LibraryManagerPage from "./components/LibraryManagerPage";
import SystemContextMenu, { type MoveTarget } from "./components/SystemContextMenu";
import RegionPicker from "./components/RegionPicker";
import TileContextMenu from "./components/TileContextMenu";
import NewCollectionDialog, { type CollectionDialogMode } from "./components/NewCollectionDialog";
import ToastStack from "./components/ToastStack";
import Shell from "./layout/Shell";
import TopToolbar from "./layout/TopToolbar";
import LeftSidebar, { type SidebarView } from "./layout/LeftSidebar";
import { createViewsStore } from "./views/store";
import { platformNodeIdFor, parsePlatformNodeId } from "./views/defaults";
import { findNode, nodeContainsId } from "./views/resolver";
import type { ContainerNode } from "./views/types";
import ContainerContextMenu from "./components/ContainerContextMenu";
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
import { PlatformMediaProvider } from "./library/platformMedia";
import { GameInfoBadgesProvider } from "./library/gameInfoBadges";
import { createLibraryStore } from "./library/store";
import { createCustomCollectionsStore } from "./library/customCollections";
import type { RomEntry } from "./library/types";
import { createSettingsStore } from "./settings/store";
import { loadShaderPresets, applyShaderPresetsUpdate, type ShaderPresetEntry } from "./settings/shader_presets";
import type { SystemId } from "./themes/registry";
import { onNavEvent, setNavEnabled, startGamepadInput, stopGamepadInput } from "./nav/gamepad";
import { HintBar, HintRegion, type Hints } from "./nav/HintBar";
import { activeFocusGroupId, setSwapAB } from "./nav/focus";
import { requestOpenFirstMenu } from "./layout/MenuBar";
import { setPerSystemUiEnabled } from "./themes/systemUiSound";
import { setBootAnimationsEnabled } from "./themes/systemBootAnimation";
import { setRetroverseUiEnabled, isRetroverseUiEnabled } from "./lib/retroverseFlag";
import RetroverseShell from "./layout/retroverse/RetroverseShell";
import { RetroverseProvider } from "./routes/retroverse/context";
import {
  currentRoute as currentRetroverseRoute,
  setCurrentRoute as setRetroverseRoute,
  cycleRouteForward as cycleRetroverseRouteForward,
  cycleRouteBackward as cycleRetroverseRouteBackward,
  RETROVERSE_ROUTES,
  type RetroverseRoute,
} from "./routing/currentRoute";

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

/** Tauri-side `DirectLaunchConfig` mirror. None = library mode. */
type DirectLaunchConfig = {
  romPath: string;
  systemId: string;
  coreOverride: string | null;
  slot: number | null;
  stateFile: string | null;
  tasReplay: string | null;
  fullscreen: boolean;
  matchedEntryId: string | null;
  /// Phase H: when set, romPath points at a .zip/.7z and archiveInnerPath
  /// is the posix-style path of the single cart ROM inside. The launch
  /// path forwards both to launch_rom so archive::extract_for_launch
  /// runs the same way it does for library-launched archived entries.
  archiveInnerPath: string | null;
};

const App: Component = () => {
  // Fire `get_direct_launch_config` once at mount; both the library store
  // (to decide whether to bootstrap) and the resource below (to drive UI
  // chrome hiding + auto-launch) subscribe to the same promise.
  const directLaunchPromise: Promise<DirectLaunchConfig | null> = invoke<DirectLaunchConfig | null>(
    "get_direct_launch_config",
  ).catch((e) => {
    console.warn("[oa-direct-launch] get_direct_launch_config failed:", e);
    return null;
  });

  const library = createLibraryStore({
    shouldBootstrap: directLaunchPromise.then((cfg) => cfg === null),
  });
  const customCollections = createCustomCollectionsStore();
  const settings = createSettingsStore();
  const layout = createLayoutStore();
  const viewsStore = createViewsStore();

  /// Build a SidebarView pointing at the active view's platform leaf for
  /// `id`. The leaf encoding (`platform:<systemId>`) is what defaults +
  /// migration emit and what `synthesizeLeafForSystem` falls back to —
  /// so a deep-link to a system the active view's tree excludes still
  /// resolves correctly via the synthesized-leaf path.
  function viewForSystem(id: SystemId): SidebarView {
    return {
      kind: "view-node",
      viewId: viewsStore.activeView()?.id ?? "",
      nodeId: platformNodeIdFor(id),
    };
  }

  /// Resolve a SidebarView back to its platform SystemId, if it points at
  /// a leaf in (or synthesizable from) the active view. Used by menu-bar
  /// logic that previously read `cv.id` directly off the system variant.
  function viewToSystemId(view: SidebarView): SystemId | null {
    if (view.kind !== "view-node") return null;
    const active = viewsStore.activeView();
    if (active && active.id === view.viewId) {
      const node = findNode(active, view.nodeId);
      if (node && "kind" in node && node.kind === "platform") return node.systemId;
    }
    return parsePlatformNodeId(view.nodeId);
  }

  // Resource form drives reactive UI gating (chrome hide, auto-launch).
  // While the resource is pending, `directLaunchConfig()` returns
  // `undefined` — we treat that the same as "not direct-launch" for the
  // chrome layer, so first-paint shows nothing rather than a library flash.
  const [directLaunchConfig] = createResource<DirectLaunchConfig | null>(
    () => directLaunchPromise,
  );
  const isDirectLaunch = createMemo(() => {
    const cfg = directLaunchConfig();
    return cfg !== undefined && cfg !== null;
  });
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
  /// Phase C3 Slice 12 — NewCollectionDialog mode. `null` = closed.
  /// Two open modes: `create` (optional seedRomId — non-null when the
  /// dialog was launched from a tile context menu so the rom is
  /// dropped in on create) and `rename` (relabel an existing list).
  const [collectionDialogMode, setCollectionDialogMode] =
    createSignal<CollectionDialogMode | null>(null);
  function openProperties(entry: RomEntry) {
    setPropertiesFor(entry);
  }
  // Game ▾ menu items that previously deep-linked into the drawer's tabs
  // now launch focused dialogs from GameDialogs.tsx. Single discriminated
  // signal covers all seven; clears on close.
  const [gameDialog, setGameDialog] = createSignal<GameDialogState>(null);
  function closeGameDialog() {
    setGameDialog(null);
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

  // Controller-nav: start the Web Gamepad API poller. Emits NavEvents on
  // the bus exposed by `frontend/src/nav/gamepad.ts`. The poller is rAF-
  // driven so it suspends when the window is hidden, and only fires when
  // the user is in the UI (the emulator's gilrs poller is gated to game-
  // window focus, so the two never overlap).
  onMount(() => {
    startGamepadInput();
    onCleanup(() => stopGamepadInput());
  });

  // Track whether the library WebView (this window) currently has OS
  // focus. The DOM `focus` / `blur` events on window fire on user
  // click-through between the library and game windows in two-window
  // shell mode — the only reliable cross-platform signal for
  // "which window does the gamepad feed?" Initial value comes from
  // document.hasFocus() so the first frame is correct even before any
  // event fires.
  const [webviewFocused, setWebviewFocused] = createSignal(
    typeof document !== "undefined" ? document.hasFocus() : true,
  );
  onMount(() => {
    const onFocus = () => setWebviewFocused(true);
    const onBlur = () => setWebviewFocused(false);
    window.addEventListener("focus", onFocus);
    window.addEventListener("blur", onBlur);
    onCleanup(() => {
      window.removeEventListener("focus", onFocus);
      window.removeEventListener("blur", onBlur);
    });
  });

  // Push controller-nav preferences into the gamepad poller + focus
  // manager whenever the settings store mutates them. Three knobs:
  // master enable (suppress all events), source (dpad / stick / both),
  // A/B swap (Nintendo convention).
  //
  // Game-running gate: while the emulator runs, the Rust gilrs poller
  // owns the gamepad and feeds the core. The Web Gamepad API only
  // emits NavEvents when an obvious UI surface is up:
  //  - No game running → always on (operator is browsing the library).
  //  - Single-window mode + library overlay visible OR Quick Settings
  //    open → on (operator paused or alt-tabbed to browse).
  //  - Two-window mode + this WebView has OS focus → on (the operator
  //    is clearly interacting with the library window, not the game).
  //  - Otherwise off — same gamepad press would otherwise drive both
  //    the UI and the running game.
  createEffect(() => {
    const userEnabled = settings.controllerNavEnabled();
    if (!userEnabled) {
      setNavEnabled(false);
      return;
    }
    if (!gameRunning()) {
      setNavEnabled(true);
      return;
    }
    if (shellMode() === "single-window") {
      setNavEnabled(libraryVisible() || quickSettingsOpen());
      return;
    }
    // Two-window: library lives in this WebView, game lives in the
    // sibling no-WebView window. OS focus on this WebView = operator
    // interacting with us. tauri::is_focused is unreliable for the
    // no-WebView window (see memory), but DOM focus on this WebView
    // is reliable.
    setNavEnabled(webviewFocused());
  });
  createEffect(() => setSwapAB(settings.controllerNavSwapAB()));
  // Per-System UI Stage 1 Slice 2: bridge the Settings master toggle
  // to the per-system SFX dispatcher. When the operator flips
  // "Per-system experiences" off, every per-system UI sound suppresses
  // at the dispatch layer — uniform plain library mode.
  createEffect(() => setPerSystemUiEnabled(settings.perSystemUiEnabled()));
  // Per-System UI Stage 1 Slice 4: bridge the Boot-animations
  // sub-toggle. Flipping it off keeps system-entry visual identity
  // (a 200ms cross-fade) but skips the full ~1s boot. Reduced-motion
  // collapses to the same short path orthogonally — accessibility
  // floor regardless of this flag.
  createEffect(() => setBootAnimationsEnabled(settings.bootAnimationsEnabled()));
  // Retroverse UI rollout Phase A Slice 1: bridge the experimental
  // master toggle to the lib/retroverseFlag accessor. Phase A wires
  // the flag without consumers; Phase B's RetroverseShell is the
  // first surface that reads it. See docs/PLANS/retroverse-ui-rollout.md.
  createEffect(() => setRetroverseUiEnabled(settings.experimentalRetroverseUi()));

  // Retroverse UI rollout Phase A Slice 4: install DevTools globals so
  // the operator can exercise the route signal independent of UI
  // consumers — Phase B's top-tab strip is the first surface that
  // reads currentRoute(). Open DevTools (F12), then in the console:
  //   __retroverse_debug.currentRoute()      → current route
  //   __retroverse_debug.setRoute("home")    → jump to a specific tab
  //   __retroverse_debug.cycleForward()      → next tab (wraps)
  //   __retroverse_debug.cycleBackward()     → prev tab (wraps)
  //   __retroverse_debug.routes              → all 6 route values
  // Dev-only — guarded by import.meta.env.DEV so production builds
  // don't expose the helper.
  onMount(() => {
    if (import.meta.env.DEV) {
      (window as unknown as { __retroverse_debug?: unknown }).__retroverse_debug = {
        currentRoute: () => currentRetroverseRoute(),
        setRoute: (r: RetroverseRoute) => {
          setRetroverseRoute(r);
          console.log(`[retroverse] currentRoute = ${r}`);
        },
        cycleForward: () => {
          const next = cycleRetroverseRouteForward();
          console.log(`[retroverse] currentRoute = ${next}`);
          return next;
        },
        cycleBackward: () => {
          const next = cycleRetroverseRouteBackward();
          console.log(`[retroverse] currentRoute = ${next}`);
          return next;
        },
        routes: RETROVERSE_ROUTES,
      };
    }
  });

  // Global Start button → open the menu bar. Bypasses the per-group
  // onStart routing in focus.ts so Start works from any active group
  // (sidebar, library-grid, etc.). When the menu is already open the
  // request is a no-op (the createEffect inside MenuBar only re-fires
  // on tick change and re-opens the first menu; switching menus is
  // handled by L1/R1).
  onMount(() => {
    const dispose = onNavEvent((event) => {
      if (event.kind !== "button") return;
      if (event.button !== "start") return;
      if (event.phase !== "down") return;
      if (gameMode() || isDirectLaunch()) return;
      requestOpenFirstMenu();
    });
    onCleanup(dispose);
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
  // Per-System UI Stage 1 Slice 3: mouse-hover preview signal for the
  // SystemBackground. LibraryTile deliberately doesn't change focus on
  // hover (selection is click-only — see LibraryTile.tsx:90-94), but
  // the background is purely decorative and benefits from following
  // the cursor. Walks `closest('[data-system]')` so it picks up tiles
  // and sidebar leaves both — anything that already declared a
  // per-system CSS scope.
  const [hoveredSystemId, setHoveredSystemId] = createSignal<SystemId | null>(null);
  onMount(() => {
    const onOver = (e: MouseEvent) => {
      const target = e.target as Element | null;
      const el = target?.closest("[data-system]");
      if (!el) return; // empty-space hover — sticky on last value
      const sys = el.getAttribute("data-system");
      if (sys && sys !== hoveredSystemId()) {
        setHoveredSystemId(sys as SystemId);
      }
    };
    document.addEventListener("mouseover", onOver, { passive: true });
    onCleanup(() => document.removeEventListener("mouseover", onOver));
  });
  // Overflow menu state (toolbar … button).
  const [overflowOpen, setOverflowOpen] = createSignal(false);
  // Library menu deep-links into the Library Manager page. The page hosts
  // two tabs (library / media); these menu items pick which one to land on.
  const [libraryManagerInitialTab, setLibraryManagerInitialTab] =
    createSignal<"library" | "media" | undefined>(undefined);
  function openLibraryManager(tab?: "library" | "media") {
    setLibraryManagerInitialTab(tab);
    setCurrentView({ kind: "library-manager" });
  }
  // Right-click context menu over a system entry in the left sidebar.
  // Open when the user right-clicks a SystemItem; null when closed.
  const [systemContextFor, setSystemContextFor] = createSignal<{
    id: SystemId;
    position: { x: number; y: number };
  } | null>(null);
  // Container right-click context menu (sister to systemContextFor —
  // operator can right-click a container header in the sidebar tree to
  // hide the whole bucket).
  const [containerContextFor, setContainerContextFor] = createSignal<{
    container: ContainerNode;
    position: { x: number; y: number };
  } | null>(null);

  /// Hide a system from the sidebar. Writes both the per-node `hidden`
  /// flag in the active view (PR-γ source of truth — survives view
  /// switches, container-level cascade) AND the legacy
  /// `layout.hiddenSystems` set (covers systems not present in the
  /// active view's tree + keeps Settings checkbox state coherent
  /// across views). Reads use a union — either-or marks a system
  /// hidden.
  function hideSystemInActiveView(id: SystemId): void {
    const list = layout.hiddenSystems();
    if (!list.includes(id)) layout.setHiddenSystems([...list, id]);
    viewsStore.setNodeHidden(platformNodeIdFor(id), true);
  }

  /// Containers in the active view that the right-clicked system isn't
  /// already in — feeds the SystemContextMenu's "Move to category…"
  /// submenu. Returns empty when no system is right-clicked, when the
  /// active view has only one container (nowhere to move to), or when
  /// the active view doesn't host the system at all (synth-leaf case —
  /// no current parent to exclude, but also no per-view context for
  /// the move to act on).
  const sidebarMoveTargets = createMemo<MoveTarget[]>(() => {
    const ctx = systemContextFor();
    if (!ctx) return [];
    const view = viewsStore.activeView();
    if (!view) return [];
    const leafId = platformNodeIdFor(ctx.id);
    let currentParentId: string | null = null;
    for (const child of view.root.children) {
      if (child.kind === "container") {
        for (const inner of child.children) {
          if (inner.id === leafId) {
            currentParentId = child.id;
            break;
          }
        }
        if (currentParentId) break;
      }
    }
    if (!currentParentId) return [];
    return view.root.children
      .filter((c): c is ContainerNode & { kind: "container" } => c.kind === "container")
      .filter((c) => c.id !== currentParentId)
      .map((c) => ({ id: c.id, label: c.label }));
  });
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

  // True when the desktop chrome (menu bar / toolbar / sidebars) should be
  // visible. UI_POLISH_PLAN.md §E.2 — Phase 0 of the kiosk shell. Zero
  // behavior change today; Phase 1 kiosk gates the chrome off this memo
  // when a future PresentationMode variant lands (e.g. "kiosk-locked").
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const chromeVisible = createMemo(() => !isDirectLaunch() && !gameMode());
  // Keep the closure live (Solid eliminates unread memos) so the linter
  // and future call sites can rely on it being defined. Cheap.
  void chromeVisible;

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

  // Reflect direct-launch state on body so CSS rules (index.css) can hide
  // any chrome that slips through the JSX-level Show guards. Stays in sync
  // with the resource — sets the attribute when the resource resolves to a
  // non-null payload, removes it otherwise.
  createEffect(() => {
    if (isDirectLaunch()) {
      document.body.dataset.directLaunch = "true";
    } else {
      delete document.body.dataset.directLaunch;
    }
  });

  // Emu thread emits `oa://rom-unloaded` after UnloadRom finishes draining.
  // In direct-launch mode there's no library to return to — quit the process.
  onMount(() => {
    let unlisten: (() => void) | undefined;
    void listen("oa://rom-unloaded", () => {
      if (!isDirectLaunch()) return;
      console.log("[oa-direct-launch] ROM unloaded → quitting process");
      void invoke("quit_app");
    }).then((un) => { unlisten = un; });
    onCleanup(() => unlisten?.());
  });

  // Auto-launch the supplied ROM once the direct-launch payload arrives.
  // Guard prevents re-launch on subsequent resource invalidations.
  let autoLaunched = false;
  createEffect(() => {
    if (autoLaunched) return;
    const cfg = directLaunchConfig();
    if (!cfg) return; // undefined (pending) or null (library mode)
    autoLaunched = true;
    void autoLaunchDirect(cfg);
  });

  async function autoLaunchDirect(cfg: DirectLaunchConfig): Promise<void> {
    let entry: RomEntry | null = null;
    // If Phase D's hash-lookup matched a library row, fetch it so per-game
    // overrides apply through the existing cascade in handleLaunch.
    if (cfg.matchedEntryId) {
      try {
        entry = await invoke<RomEntry | null>("get_game", { id: cfg.matchedEntryId });
      } catch (e) {
        console.warn("[oa-direct-launch] get_game failed, falling back to synthesized entry:", e);
      }
    }
    if (!entry) {
      // Title heuristic: when the ROM is wrapped in an archive, the inner
      // filename is the meaningful one. Otherwise use the outer path.
      const titleSource = cfg.archiveInnerPath ?? cfg.romPath;
      // For archive launches, fold the inner path into the id so two
      // different inners inside the same archive get separate ids
      // (matters for CD-in-archive temp-dir cleanup keyed off entryId).
      // Library entries already encode `<archive>#<inner>` as filePath;
      // mirror that for the synthesized RomEntry too.
      const idSource = cfg.archiveInnerPath
        ? `${cfg.romPath}#${cfg.archiveInnerPath}`
        : cfg.romPath;
      const filePath = cfg.archiveInnerPath
        ? `${cfg.romPath}#${cfg.archiveInnerPath}`
        : cfg.romPath;
      entry = {
        id: cfg.matchedEntryId ?? romIdFromPath(idSource),
        title: titleFromFileName(titleSource),
        systemId: cfg.systemId as SystemId,
        filePath,
        addedAt: 0,
        seed: false,
        coreOverride: cfg.coreOverride ?? undefined,
        archiveInnerPath: cfg.archiveInnerPath ?? undefined,
      };
    }
    console.log("[oa-direct-launch] auto-launching:", entry);
    await handleLaunch(entry, cfg.slot ?? undefined, cfg.stateFile ?? undefined);

    // After the launch cascade settles, apply CLI-only overrides that
    // sit on top of per-game / per-system / OA-wide settings.
    if (cfg.fullscreen) {
      void invoke("set_window_mode", { mode: "fullscreen", monitorIndex: null }).catch((e) =>
        console.warn("[oa-direct-launch] --fullscreen set_window_mode failed:", e),
      );
    }
    if (cfg.tasReplay) {
      void invoke("start_tas_replay", { filePath: cfg.tasReplay }).catch((e) =>
        console.warn("[oa-direct-launch] --tas-replay start_tas_replay failed:", e),
      );
    }
  }

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
      currentView().kind !== "library-manager"
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
      if (currentView().kind === "library-manager") return;
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
      await settings.addLibraryFolderPath(result.folder);
    }
    if (result.kind === "ingested" && result.added > 0) {
      void autoSyncAfterIngest(result.systemIds, result.entries);
    }
    setBusy("idle");
    setOverflowOpen(false);
  }

  async function handleAddLibraryFolder() {
    const picked = await pickDirectory({ directory: true, multiple: false }).catch(() => null);
    if (!picked || Array.isArray(picked)) return;
    // Pre-flight: refuse to add an empty directory. Same check the
    // ImportWizard uses on its Step 1; keeps the quick-add path from
    // polluting the library with a directory that has nothing in it.
    try {
      const empty = await invoke<boolean>("directory_is_empty", { path: picked });
      if (empty) {
        setStatus(`${picked} is empty — nothing to import.`);
        return;
      }
    } catch (e) {
      setStatus(`Couldn't read ${picked}: ${String(e)}`);
      return;
    }
    // Persist the folder row BEFORE the scan so a watcher / restart
    // can still find it if the operator interrupts mid-scan. On a
    // total scan failure (0 added AND every folder errored — i.e.,
    // the directory was unreadable or every ROM hit an error) we
    // roll back so the operator doesn't see a phantom folder in
    // Settings → Library. Partial errors (some imports succeeded)
    // keep the folder and surface the error count in the status.
    await settings.addLibraryFolderPath(picked);
    setBusy("scanning");
    setStatus(`Scanning ${picked}…`);
    const summary = await rescanFolders(library, [picked], scanProgressReporter);
    if (summary.totalAdded === 0 && summary.errors.length > 0) {
      // Total scan failure — roll back the persisted folder. The
      // store doesn't expose a by-path remove; look the row up from
      // libraryFolderRows() (post-refresh) and remove by id.
      console.warn("rescan errors:", summary.errors);
      try {
        const row = settings.libraryFolderRows().find((r) => r.path === picked);
        if (row) {
          await settings.removeLibraryFolderById(row.id);
        }
      } catch (e) {
        console.warn(`[oa-app] folder rollback failed for ${picked}:`, e);
      }
      const firstErr = summary.errors[0] ?? "unknown error";
      setStatus(`Failed to scan ${picked}: ${firstErr}`);
      setBusy("idle");
      return;
    }
    const errSuffix = summary.errors.length > 0 ? ` (${summary.errors.length} errored)` : "";
    setStatus(`Added ${summary.totalAdded} from ${picked}${errSuffix}.`);
    if (summary.errors.length > 0) console.warn("rescan errors:", summary.errors);
    if (summary.totalAdded > 0) {
      void autoSyncAfterIngest(summary.systemIds, summary.entries);
    }
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
    if (summary.totalAdded > 0) {
      void autoSyncAfterIngest(summary.systemIds, summary.entries);
    }
    setBusy("idle");
  }

  /// Post-launch UI bridge. Status toast, gameRunning flip, runningEntry
  /// capture, single-window library auto-hide. Shared between
  /// GameInfoModal's onLaunched callback (modal launch path) and
  /// RetroverseContext's onPostLaunch (Retroverse GameDetailPanel
  /// launch path). Tile-click launches go through handleLaunch
  /// directly — handleLaunch does these updates inline so the
  /// modal/panel callback isn't on the hot path.
  function postLaunchUiUpdate(entry: RomEntry, slot?: number): void {
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
  }

  async function handleLaunch(entry: RomEntry, slot?: number, stateFile?: string) {
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
      slot,
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
      type OverscanCropPrefs = { top: number; bottom: number; left: number; right: number };
      type SysSettings = {
        shaderPreset?: string | null;
        bloomAmount?: number | null;
        scalingOverride?: string | null;
        windowModeOverride?: string | null;
        monitorIndexOverride?: number | null;
        rewindEnabled?: boolean | null;
        rewindCaptureIntervalFrames?: number | null;
        rewindBufferMegabytes?: number | null;
        displayAspectOverride?: number | null;
        overscanCropOverride?: OverscanCropPrefs | null;
        bezelImagePath?: string | null;
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
        // Display-aspect override chain. No OA-wide value — the bottom
        // of the chain is `null` = "trust whatever the libretro core
        // reports via retro_get_system_av_info" (the typical case;
        // cores like Beetle PCE Fast set the PCE aspect correctly).
        // Operators tune per-system or per-game when the core's
        // reported value doesn't suit their display preference.
        displayAspectOverride:
          game?.displayAspectOverride
          ?? sys?.displayAspectOverride
          ?? null,
        // Overscan crop chain. No OA-wide value — bottom of the chain
        // is "no crop" (zero on every edge). Operators tune per-system
        // (typical case: NES top=8 bottom=8) or per-game when one
        // specific title has nasty edge garbage.
        overscanCropOverride:
          game?.overscanCropOverride
          ?? sys?.overscanCropOverride
          ?? null,
        // Bezel image chain. Per-game → per-system → null (use shader
        // preset's TOML default, which may itself be no bezel). The
        // override is an absolute path to a PNG/JPEG/WebP.
        bezelImagePath:
          game?.bezelImagePath
          ?? sys?.bezelImagePath
          ?? null,
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
        invoke("set_display_aspect_override", { aspect: effective.displayAspectOverride }).catch((e) =>
          console.warn("[oa-launch] set_display_aspect_override failed:", e),
        ),
        invoke("set_overscan_crop", {
          top:    effective.overscanCropOverride?.top    ?? 0,
          bottom: effective.overscanCropOverride?.bottom ?? 0,
          left:   effective.overscanCropOverride?.left   ?? 0,
          right:  effective.overscanCropOverride?.right  ?? 0,
        }).catch((e) => console.warn("[oa-launch] set_overscan_crop failed:", e)),
        // Bezel: if the resolved chain ends in a path, push as
        // override (overrides the preset's bezel from set_shader_preset
        // above). If null, leave whatever set_shader_preset uploaded
        // alone — the preset's TOML default stays active.
        effective.bezelImagePath
          ? invoke("set_bezel_image_override", { path: effective.bezelImagePath }).catch((e) =>
              console.warn("[oa-launch] set_bezel_image_override failed:", e),
            )
          : Promise.resolve(),
      ]);
    } catch (e) {
      console.warn("[oa-launch] override resolution failed:", e);
    }

    const result = await launchRom(entry, slot, stateFile);
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
      // Phase 2.5 — resolve per-game → per-system analog routing for
      // all 5 ports and push to the emu thread. Soft failure: at worst
      // the game uses per-system-only routing until restart.
      void invoke<void>("arm_analog_routing", { gameId: entry.id })
        .catch((e) => console.warn("[oa-launch] arm_analog_routing failed:", e));
      // Shared analog input infra — resolve per-game libretro device
      // type (Mouse / Light Gun / Paddle / etc.) and dispatch to the
      // emu thread AFTER retro_load_game. Mednafen cores clobber
      // data_ptr[] during load, so this must run post-launch_rom;
      // we're already inside the launch-completed branch so the
      // ordering is correct. Soft failure: at worst the game runs
      // with the default JOYPAD device.
      void invoke<void>("arm_libretro_device", { gameId: entry.id })
        .catch((e) => console.warn("[oa-launch] arm_libretro_device failed:", e));
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
        // Drop any per-game / per-system display-aspect override so the
        // next launch starts from the core-reported value.
        invoke("set_display_aspect_override", { aspect: null }).catch(() => {}),
        // Drop any overscan crop so the next launch starts un-cropped.
        invoke("set_overscan_crop", { top: 0, bottom: 0, left: 0, right: 0 }).catch(() => {}),
        // Drop any per-game / per-system bezel override.
        invoke("clear_bezel_image_override").catch(() => {}),
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
    return viewToSystemId(currentView());
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
                  onClick={() => setCurrentView(viewForSystem(id()))}
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
                    hideSystemInActiveView(id());
                    if (activeSystemId() === id()) {
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
                <MenuItem label="Properties…" onClick={() => openProperties(entry())} />
                <MenuDivider />
                <MenuItem label="Cheats…" onClick={() => setGameDialog({ kind: "cheats", target: entry() })} />
                <MenuItem label="Milestones…" onClick={() => setGameDialog({ kind: "milestones", target: entry() })} />
                <MenuItem label="Shaders…" onClick={() => setGameDialog({ kind: "shaders", target: entry() })} />
                <MenuItem label="Rewind overrides…" onClick={() => setGameDialog({ kind: "rewind", target: entry() })} />
                <MenuItem label="Input…" onClick={() => setGameDialog({ kind: "input", target: entry() })} />
                <MenuItem label="Display overrides…" onClick={() => setGameDialog({ kind: "display", target: entry() })} />
                <MenuItem label="Core options…" onClick={() => setGameDialog({ kind: "core-options", target: entry() })} />
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
  // External drag-drop is **known unreliable** on this build (see
  // `docs/PARKING_LOT.md` 2026-05-19 entry) — neither shell mode delivers
  // paths consistently. The DOM listeners give visual feedback + a
  // fallback hint pointing at the Import Wizard. Internal HTML5
  // drag-drop (sidebar reorder, region priority drag) is unaffected.
  /// Kick off the canonical-title sync for each system that received
  /// fresh games. Fire-and-forget — the resolve flow emits its own
  /// progress events + the library auto-refreshes on completion via
  /// the existing `oa://rom-hash-resolve-complete` listener in the
  /// Settings page. Calling more than once for the same system in
  /// quick succession is idempotent (no-op when no entries are
  /// missing a sha1).
  /// Post-ingest auto-sync hook. Used by every non-wizard ingest path
  /// (drag-drop, "Add library folder", "Rescan tracked folders",
  /// "Import folder…" from the toolbar). Mirrors the wizard's commit()
  /// flow at frontend/src/components/ImportWizard.tsx:550-630:
  ///
  ///   1. AWAIT resolve_rom_hashes_for_system per system, sequentially.
  ///      Parallel resolves would contend on the LibraryDb write lock
  ///      (each resolve holds the lock during DB writes); the wizard
  ///      explicitly serialized this for the same reason. The
  ///      server-side H11 per-system gate would also serialize them
  ///      regardless, but doing it on the client surfaces useful
  ///      sequential progress events to the user.
  ///
  ///   2. THEN fire sync_media_for_system + sync_metadata_for_system
  ///      per touched system. These can fire in parallel — different
  ///      systems don't contend (each holds its own per-system gate),
  ///      and within a system the two operations are independent
  ///      (media and metadata DBs are separate slots on the same
  ///      MediaDb row). Fire-and-forget — the per-ROM progress events
  ///      drive the visible UI.
  ///
  /// Pre-2026-05-21 this function only invoked resolve and skipped
  /// media/metadata entirely — drag-drop ingest never got cover art
  /// without the user also running the wizard. Fixed in the H8 audit
  /// follow-up.
  async function autoSyncAfterIngest(
    systemIds: readonly SystemId[],
    entries: readonly RomEntry[],
  ) {
    // Step 1: per-system resolve, awaited sequentially.
    for (const id of systemIds) {
      try {
        await invoke("resolve_rom_hashes_for_system", { systemId: id });
      } catch (e) {
        console.warn(`[oa-ingest] auto-identify ${id} failed:`, e);
        // Continue with the next system + still fire the syncs below.
        // Failed resolve means stale-sha1 entries, but sync_media will
        // simply filter them out via only_identified rather than
        // crashing.
      }
    }
    // Step 2: media + metadata sync per system, fire-and-forget.
    const entriesBySystem = new Map<SystemId, RomEntry[]>();
    for (const e of entries) {
      const arr = entriesBySystem.get(e.systemId) ?? [];
      arr.push(e);
      entriesBySystem.set(e.systemId, arr);
    }
    for (const id of systemIds) {
      const sysEntries = entriesBySystem.get(id) ?? [];
      if (sysEntries.length === 0) continue;
      const payload = sysEntries.map((e) => ({
        id: e.id,
        title: e.title,
        filePath: e.filePath,
        systemId: e.systemId,
      }));
      void invoke("sync_media_for_system", { systemId: id, entries: payload }).catch(
        (e) => console.warn(`[oa-ingest] sync_media ${id} failed:`, e),
      );
      void invoke("sync_metadata_for_system", { systemId: id, entries: payload }).catch(
        (e) => console.warn(`[oa-ingest] sync_metadata ${id} failed:`, e),
      );
    }
  }

  async function commitDroppedPath(path: string) {
    setBusy("scanning");
    setStatus(`Scanning ${path}…`);
    const result = await ingestFolderPath(library, path, scanProgressReporter);
    setStatus(ingestStatus(result));
    if (result.kind === "ingested" || result.kind === "empty") {
      await settings.addLibraryFolderPath(result.folder);
    }
    if (result.kind === "ingested" && result.added > 0) {
      void autoSyncAfterIngest(result.systemIds, result.entries);
    }
    setBusy("idle");
  }

  onMount(async () => {
    // DOM events. preventDefault on dragover is the load-bearing one:
    // it overrides WebView2's default "no drop allowed" cursor. The
    // main shell is now opaque in both shell modes — Tauri's
    // drag-drop hook fires reliably on this window so we no longer
    // need any overlay-window workaround.

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
      // In two-window mode Tauri's onDragDropEvent fires with the real
      // OS paths; in single-window mode the drop-overlay window catches
      // it and emits `oa://overlay-drop` (see the listener below).
      // If neither path lands within 500ms, surface a fallback hint.
      if ((e.dataTransfer?.files?.length ?? 0) > 0) {
        setStatus("Drop received — waiting for Tauri to deliver the path…");
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
    // file paths when it works. Currently unreliable across shell modes
    // (parking-lot item from 2026-05-19); the listener is wired up
    // anyway so any successful drops still ingest, and the DOM listener
    // above shows the fallback hint when paths don't arrive.
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
      <PlatformMediaProvider>
      <GameInfoBadgesProvider entries={() => library.state.entries}>
      {/* Retroverse-UI Phase B Slice 5 — entire Shell swaps to
          RetroverseShell when the experimental flag is ON. Two
          distinct UIs, no hybrid state. Modals (ImportWizard /
          GameInfoModal / etc.) below this Show stay accessible in
          both modes. */}
      <Show
        when={isRetroverseUiEnabled()}
        fallback={
      <Shell
        layout={layout}
        fullBleed={isDirectLaunch() || gameMode()}
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
            views={viewsStore}
            currentView={currentView()}
            onNavigate={(v) => setCurrentView(v)}
            onSystemContext={(id, position) => setSystemContextFor({ id, position })}
            onContainerContext={(container, position) =>
              setContainerContextFor({ container, position })
            }
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
          {/* Per-System UI Stage 1 Slice 3: per-system background
              layer mounted behind the library content. Source chain
              follows "where is the operator looking right now?":
                1. hoveredSystemId — mouse over any [data-system]
                   element (tiles, sidebar leaves, …); updates live
                   without changing tile selection
                2. focusedEntry().systemId — click/gamepad-selected
                   tile (sticky)
                3. activeSystemId() — sidebar-filtered system view
                   (e.g. operator clicked into NES from the sidebar
                   but hasn't clicked a tile yet)
                4. pinnedEntry().systemId — right-sidebar pin as the
                   "nothing else applies" fallback
              Honors the perSystemUiEnabled master toggle. */}
          {/* Suppress library-chrome overlays (background + boot
              animation) when the game is "full bleed" — single-window
              shell + game running + library hidden. In that state the
              WebView is supposed to melt away so wgpu emulator pixels
              show through; an opaque CSS background-image overlay
              would visually cover the running game. StylusOverlay
              below DOES render through gameMode because that's
              literally its purpose (visual stylus feedback while
              playing an NDS game). */}
          <Show when={!gameMode()}>
            <SystemBackground
              systemId={() =>
                (hoveredSystemId()
                  ?? (focusedEntry()?.systemId as SystemId | undefined)
                  ?? activeSystemId()
                  ?? (pinnedEntry()?.systemId as SystemId | undefined)
                  ?? null) as SystemId | null
              }
            />
            {/* Per-System UI Stage 1 Slice 4: boot animation overlay
                triggered by explicit system entry (sidebar nav).
                activeSystemId() reflects "viewToSystemId(currentView())"
                — i.e. the system the operator filtered to via the
                sidebar. Hover/focus changes don't fire the boot; only
                switching the library's active view does. */}
            <SystemBootAnimation activeSystemId={activeSystemId} />
          </Show>
          {/* NDS Phase 2: visual stylus reticle. Tracks the OS cursor
              while a stylus-using game is running and adds explicit
              press feedback (the OS cursor doesn't change appearance
              on click). Closes the docs/cores/nds/ROADMAP.md "Per-
              game touch overlay UI — visual stylus cursor" bullet for
              the cursor portion; per-game touch hotspots remain ⬜
              as a separate larger feature. */}
          <StylusOverlay
            runningSystemId={() => (runningEntry()?.systemId ?? null) as SystemId | null}
          />
          <div class="relative z-10 h-full">
          <Switch
            fallback={
              <Show when={!isDirectLaunch()}>
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
                    views={viewsStore}
                    currentView={currentView()}
                    searchQuery={searchQuery()}
                    onLaunch={handleLaunch}
                    onShowSaves={(entry) => setSavesEntry(entry)}
                    onPickContext={(entry, position) => setContextMenuFor({ entry, position })}
                    onFocus={(entry) => setFocusedEntry(entry)}
                    onShowInfo={(entry) => setGameInfoFor(entry)}
                    selectedId={() => focusedEntry()?.id ?? null}
                    onPickFolder={handlePickFolder}
                    onToggleFavorite={(entry, value) => void library.setFavorite(entry.id, value)}
                  />
                </div>
              </Show>
            }
          >
            <Match when={currentView().kind === "library-manager"}>
              <div class="h-full overflow-y-auto">
                <LibraryManagerPage
                  onBack={() => setCurrentView({ kind: "all" })}
                  settings={settings}
                  library={library}
                  layout={layout}
                  views={viewsStore}
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
          </div>
        </main>
      </Shell>
        }
      >
        <RetroverseProvider
          value={{
            library,
            customCollections,
            layout,
            views: viewsStore,
            settings,
            searchQuery,
            setSearchQuery,
            focusedEntry,
            setFocusedEntry,
            currentView,
            setCurrentView,
            onLaunch: handleLaunch,
            onShowSaves: (e) => setSavesEntry(e),
            onShowInfo: (e) => setGameInfoFor(e),
            onPickContext: (entry, position) => setContextMenuFor({ entry, position }),
            onPickFolder: handlePickFolder,
            onPostLaunch: postLaunchUiUpdate,
            onToggleFavorite: (entry, value) => void library.setFavorite(entry.id, value),
            onToggleCompleted: (entry, value) => void library.setCompleted(entry.id, value),
            onAddLibraryFolder: handleAddLibraryFolder,
            onRescanLibraryFolders: handleRescanLibraryFolders,
            onOpenNewCollection: (seedRomId) =>
              setCollectionDialogMode({ kind: "create", seedRomId }),
            onOpenRenameCollection: (collectionId, currentName) =>
              setCollectionDialogMode({ kind: "rename", collectionId, currentName }),
            gameFocus,
            onQuit: () => void invoke("quit_app"),
            onOpenDebugLog: () => setHelpDialog("debug-log"),
            onOpenKeyboardShortcuts: () => setHelpDialog("shortcuts"),
          }}
        >
          {/* Phase B Slice 7 fix — mirror existing Shell's fullBleed
              gate: hide the entire Retroverse shell when the game is
              "full bleed" (single-window with library hidden, OR
              direct-launch boot). The wgpu emulator surface paints to
              the WebView's transparent background; when no Retroverse
              chrome is on top, emulator pixels show through. Esc /
              Ctrl+W toggle libraryVisible back, gameMode goes false,
              shell re-renders. */}
          <Show when={!(isDirectLaunch() || gameMode())}>
            <RetroverseShell />
          </Show>
        </RetroverseProvider>
      </Show>
      {/* Folder-drop overlay — rendered as a sibling of the flag-gate
          Show so it overlays both the legacy Shell and RetroverseShell.
          Pointer-events:none lets the underlying Tauri drag-drop logic
          handle the actual drop without our DOM capturing it. The Tauri
          payload, not the DOM event, fires the ingest. The drop
          listener (window.addEventListener("drop", …)) at the bottom of
          this component is already window-global, so the overlay's only
          job is the visual hint. */}
      <Show when={dropOverlayVisible()}>
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
        exitMode={isDirectLaunch() ? "quit" : "library"}
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
        onShowLibrary={(id) => setCurrentView(viewForSystem(id))}
        onOpenBindings={(id) => openSystemDialog("bindings", id)}
        onOpenSettings={(id) => openSystemDialog("display", id)}
        onHideSystem={(id) => {
          hideSystemInActiveView(id);
          // If the user hid the system they were viewing, kick them back
          // to "All games" so they aren't stranded on a sidebar entry
          // that just disappeared.
          if (activeSystemId() === id) {
            setCurrentView({ kind: "all" });
          }
        }}
        moveTargets={sidebarMoveTargets()}
        onMoveToContainer={(containerId) => {
          const ctx = systemContextFor();
          if (!ctx) return;
          viewsStore.moveNode(platformNodeIdFor(ctx.id), containerId, null);
        }}
      />
      <ContainerContextMenu
        container={containerContextFor()?.container ?? null}
        position={containerContextFor()?.position ?? null}
        entries={library.state.entries}
        onClose={() => setContainerContextFor(null)}
        onHide={(containerId) => {
          viewsStore.setNodeHidden(containerId, true);
          // If the user's currently-viewing node was inside the hidden
          // container, kick them back to "All games" so the sidebar
          // doesn't strand them on a row that just disappeared.
          const cv = currentView();
          if (cv.kind === "view-node") {
            const active = viewsStore.activeView();
            if (active && active.id === cv.viewId) {
              const containerNode = findNode(active, containerId);
              if (containerNode && nodeContainsId(containerNode, cv.nodeId)) {
                setCurrentView({ kind: "all" });
              }
            }
          }
        }}
      />
      <TileContextMenu
        entry={contextMenuFor()?.entry ?? null}
        position={contextMenuFor()?.position ?? null}
        library={library}
        customCollections={customCollections}
        onClose={() => setContextMenuFor(null)}
        onLaunch={(entry) => void handleLaunch(entry)}
        onShowSaves={(entry) => setSavesEntry(entry)}
        onShowGameInfo={(entry) => setGameInfoFor(entry)}
        onPickRegion={(entry) => setRegionPickerFor(entry)}
        onPickCore={(entry, position) => setCoreMenuFor({ entry, position })}
        onOpenProperties={(entry) => setPropertiesFor(entry)}
        onOpenNewCollection={(romId) =>
          setCollectionDialogMode({ kind: "create", seedRomId: romId })
        }
      />
      <NewCollectionDialog
        mode={collectionDialogMode()}
        customCollections={customCollections}
        onClose={() => setCollectionDialogMode(null)}
      />
      <GamePropertiesDialog
        open={propertiesFor() !== null}
        entry={propertiesFor()}
        onClose={() => setPropertiesFor(null)}
        settings={settings}
        library={library}
      />
      <GameCoreOptionsDialog
        open={gameDialog()?.kind === "core-options"}
        entry={gameDialog()?.kind === "core-options" ? gameDialog()!.target : null}
        onClose={closeGameDialog}
      />
      <GameDisplayDialog
        open={gameDialog()?.kind === "display"}
        entry={gameDialog()?.kind === "display" ? gameDialog()!.target : null}
        onClose={closeGameDialog}
        settings={settings}
      />
      <GameInputDialog
        open={gameDialog()?.kind === "input"}
        entry={gameDialog()?.kind === "input" ? gameDialog()!.target : null}
        onClose={closeGameDialog}
      />
      <GameRewindDialog
        open={gameDialog()?.kind === "rewind"}
        entry={gameDialog()?.kind === "rewind" ? gameDialog()!.target : null}
        onClose={closeGameDialog}
        settings={settings}
      />
      <GameShadersDialog
        open={gameDialog()?.kind === "shaders"}
        entry={gameDialog()?.kind === "shaders" ? gameDialog()!.target : null}
        onClose={closeGameDialog}
        settings={settings}
      />
      <MilestonesDialog
        open={gameDialog()?.kind === "milestones"}
        entry={gameDialog()?.kind === "milestones" ? gameDialog()!.target : null}
        onClose={closeGameDialog}
      />
      <CheatsDialog
        open={gameDialog()?.kind === "cheats"}
        entry={gameDialog()?.kind === "cheats" ? gameDialog()!.target : null}
        onClose={closeGameDialog}
      />
      <GameInfoModal
        entry={gameInfoFor()}
        onClose={() => setGameInfoFor(null)}
        onLaunched={postLaunchUiUpdate}
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
      <HintBar />
      <Show when={!gameMode() && !isDirectLaunch()}>
        <HintRegion
          hints={(): Hints => {
            switch (activeFocusGroupId()) {
              case "left-sidebar":
                return { a: "Open", x: "System menu", r1: "Library", start: "Menu bar" };
              case "library-grid":
                return { a: "Launch", x: "Menu", y: "Info", l1: "Sidebar", r1: "Widgets", start: "Menu bar" };
              case "right-sidebar":
                return { a: "Activate", b: "Library", l1: "Library", start: "Menu bar" };
              default:
                return {};
            }
          }}
        />
      </Show>
      </GameInfoBadgesProvider>
      </PlatformMediaProvider>
    </MediaProvider>
  );
};

export default App;
