// Layout state shared across the shell + region primitives.
//
// Source of truth lives in Rust (`appDataDir/layout.json` and
// `presentation.json`). This module hydrates on mount and writes through on
// every change. There's a brief default-state flash before hydration (~ first
// frame) — acceptable for desktop launchers; future work could inject
// initial values into the WebView at startup via window.__OA_INITIAL_LAYOUT.

import { createEffect, createSignal, onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/core";

export type PresentationMode = "desktop" | "theater" | "cabinet";

export const PRESENTATION_OPTIONS: readonly PresentationMode[] = [
  "desktop",
  "theater",
  "cabinet",
];

export const PRESENTATION_LABELS: Record<PresentationMode, string> = {
  desktop: "Desktop",
  theater: "Theater",
  cabinet: "Cabinet",
};

// Mirrors Rust's `layout::LayoutPrefs` (camelCase via serde).
export type LayoutPrefs = {
  leftSidebarWidth: number;
  leftSidebarCollapsed: boolean;
  rightSidebarWidth: number;
  rightSidebarVisible: boolean;
  rightSidebarPinnedGameId: string | null;
  widgetOrder: string[];
  widgetHidden: string[];
  viewMode: ViewMode;
  sortKey: SortKey;
  groupBy: GroupBy;
  systemOrder: string[];
};

export type ViewMode = "capsule" | "list";
export type SortKey = "title" | "addedAt" | "year";
export type GroupBy = "none" | "letter" | "system";

export const VIEW_MODE_OPTIONS: readonly ViewMode[] = ["capsule", "list"];
export const VIEW_MODE_LABELS: Record<ViewMode, string> = {
  capsule: "Capsule grid",
  list: "Detail list",
};

export const SORT_KEY_OPTIONS: readonly SortKey[] = ["title", "addedAt", "year"];
export const SORT_KEY_LABELS: Record<SortKey, string> = {
  title: "Name",
  addedAt: "Date added",
  year: "Year",
};

export const GROUP_BY_OPTIONS: readonly GroupBy[] = ["none", "letter", "system"];
export const GROUP_BY_LABELS: Record<GroupBy, string> = {
  none: "None",
  letter: "Letter",
  system: "System",
};

function isViewMode(v: unknown): v is ViewMode {
  return v === "capsule" || v === "list";
}
function isSortKey(v: unknown): v is SortKey {
  return v === "title" || v === "addedAt" || v === "year";
}
function isGroupBy(v: unknown): v is GroupBy {
  return v === "none" || v === "letter" || v === "system";
}

const DEFAULT_LAYOUT: LayoutPrefs = {
  leftSidebarWidth: 280,
  leftSidebarCollapsed: false,
  rightSidebarWidth: 320,
  rightSidebarVisible: true,
  rightSidebarPinnedGameId: null,
  widgetOrder: ["hero", "title", "metadata"],
  widgetHidden: [],
  viewMode: "capsule",
  sortKey: "title",
  groupBy: "none",
  systemOrder: [],
};

function clamp(n: number, lo: number, hi: number): number {
  return Math.max(lo, Math.min(hi, n));
}

function isPresentationMode(v: unknown): v is PresentationMode {
  return v === "desktop" || v === "theater" || v === "cabinet";
}

export function createLayoutStore() {
  // Signals start at defaults; hydrate replaces them inside onMount.
  const [presentationMode, setPresentationMode] = createSignal<PresentationMode>("desktop");
  const [leftSidebarWidth, setLeftSidebarWidth] = createSignal<number>(DEFAULT_LAYOUT.leftSidebarWidth);
  const [leftSidebarCollapsed, setLeftSidebarCollapsed] = createSignal<boolean>(DEFAULT_LAYOUT.leftSidebarCollapsed);
  const [rightSidebarWidth, setRightSidebarWidth] = createSignal<number>(DEFAULT_LAYOUT.rightSidebarWidth);
  const [rightSidebarVisible, setRightSidebarVisible] = createSignal<boolean>(DEFAULT_LAYOUT.rightSidebarVisible);
  const [rightSidebarPinnedGameId, setRightSidebarPinnedGameId] = createSignal<string | null>(
    DEFAULT_LAYOUT.rightSidebarPinnedGameId,
  );
  const [widgetOrder, setWidgetOrder] = createSignal<string[]>(DEFAULT_LAYOUT.widgetOrder);
  const [widgetHidden, setWidgetHidden] = createSignal<string[]>(DEFAULT_LAYOUT.widgetHidden);
  const [viewMode, setViewMode] = createSignal<ViewMode>(DEFAULT_LAYOUT.viewMode);
  const [sortKey, setSortKey] = createSignal<SortKey>(DEFAULT_LAYOUT.sortKey);
  const [groupBy, setGroupBy] = createSignal<GroupBy>(DEFAULT_LAYOUT.groupBy);
  const [systemOrder, setSystemOrder] = createSignal<string[]>(DEFAULT_LAYOUT.systemOrder);
  // Suppress write-through during the initial hydrate so we don't echo
  // defaults back to disk before the real values land.
  const [hydrated, setHydrated] = createSignal(false);

  // Mirror presentation onto body for CSS-cascade geometry tokens.
  createEffect(() => {
    document.body.dataset.presentation = presentationMode();
  });

  onMount(async () => {
    try {
      const mode = await invoke<string>("get_presentation_mode");
      if (isPresentationMode(mode)) setPresentationMode(mode);
    } catch (e) {
      console.warn("LayoutStore: get_presentation_mode failed:", e);
    }
    try {
      const prefs = await invoke<LayoutPrefs>("get_layout");
      setLeftSidebarWidth(clamp(prefs.leftSidebarWidth, 200, 360));
      setLeftSidebarCollapsed(prefs.leftSidebarCollapsed === true);
      setRightSidebarWidth(clamp(prefs.rightSidebarWidth, 240, 440));
      setRightSidebarVisible(prefs.rightSidebarVisible !== false);
      setRightSidebarPinnedGameId(typeof prefs.rightSidebarPinnedGameId === "string"
        ? prefs.rightSidebarPinnedGameId
        : null);
      if (Array.isArray(prefs.widgetOrder)) {
        setWidgetOrder(prefs.widgetOrder.filter((w): w is string => typeof w === "string"));
      }
      if (Array.isArray(prefs.widgetHidden)) {
        setWidgetHidden(prefs.widgetHidden.filter((w): w is string => typeof w === "string"));
      }
      if (isViewMode(prefs.viewMode)) setViewMode(prefs.viewMode);
      if (isSortKey(prefs.sortKey)) setSortKey(prefs.sortKey);
      if (isGroupBy(prefs.groupBy)) setGroupBy(prefs.groupBy);
      if (Array.isArray(prefs.systemOrder)) {
        setSystemOrder(prefs.systemOrder.filter((s): s is string => typeof s === "string"));
      }
    } catch (e) {
      console.warn("LayoutStore: get_layout failed:", e);
    }
    setHydrated(true);
  });

  // Write-through to Rust on every change after hydration.
  createEffect(() => {
    if (!hydrated()) return;
    const prefs: LayoutPrefs = {
      leftSidebarWidth: leftSidebarWidth(),
      leftSidebarCollapsed: leftSidebarCollapsed(),
      rightSidebarWidth: rightSidebarWidth(),
      rightSidebarVisible: rightSidebarVisible(),
      rightSidebarPinnedGameId: rightSidebarPinnedGameId(),
      widgetOrder: widgetOrder(),
      widgetHidden: widgetHidden(),
      viewMode: viewMode(),
      sortKey: sortKey(),
      groupBy: groupBy(),
      systemOrder: systemOrder(),
    };
    invoke("set_layout", { prefs }).catch((e) =>
      console.warn("LayoutStore: set_layout failed:", e),
    );
  });

  createEffect(() => {
    if (!hydrated()) return;
    const mode = presentationMode();
    invoke("set_presentation_mode", { mode }).catch((e) =>
      console.warn("LayoutStore: set_presentation_mode failed:", e),
    );
  });

  return {
    presentationMode,
    setPresentationMode,
    leftSidebarWidth,
    setLeftSidebarWidth,
    leftSidebarCollapsed,
    setLeftSidebarCollapsed,
    rightSidebarWidth,
    setRightSidebarWidth,
    rightSidebarVisible,
    setRightSidebarVisible,
    rightSidebarPinnedGameId,
    setRightSidebarPinnedGameId,
    widgetOrder,
    setWidgetOrder,
    widgetHidden,
    setWidgetHidden,
    viewMode,
    setViewMode,
    sortKey,
    setSortKey,
    groupBy,
    setGroupBy,
    systemOrder,
    setSystemOrder,
  };
}

export type LayoutStore = ReturnType<typeof createLayoutStore>;
