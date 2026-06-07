// Platform — dialog open/close state.
//
// Per the 2026-06-06 Theming Substrate decision (recorded in
// docs/features/theming-substrate/SURFACES.md): Platform owns the
// open/close signal; the active theme chooses where to anchor the
// dialog visually. Dialog COMPONENTS still mount from App.tsx for
// now — moving their mount point lands in Phase 6 when Retroverse
// becomes a theme on the SDK and per-game dialogs gain theme-chosen
// anchors. This module ships the state migration; theme-chosen
// anchors arrive later.
//
// Phase 1 (2026-06-06) migrated the 5 SURFACES.md-listed dialogs:
// savesEntry, contextMenuFor, gameInfoFor, helpDialog, wizardOpen.
// Phase 2 Slice A (2026-06-06) extends with the 10 residual signals
// SURFACES.md "Open boundary questions" flagged — coreMenuFor,
// regionPickerFor, propertiesFor, collectionDialogMode, gameDialog,
// quickSettingsOpen, screenshotGalleryFor, systemContextFor,
// containerContextFor, systemDialog.
//
// settingsDialog was dead-letter (zero open call sites) and deleted
// outright in Phase 2 Slice A alongside SettingsDialogs.tsx.
//
// Architectural note: type imports from components/ are pragmatic for
// Phase 2 Slice A — the ESLint boundary rule (Phase 4) will catch any
// runtime imports, and Slice B / later phases that move dialog
// components into platform/ will eliminate the type-only crossing.
// Slice A keeps the diff focused on the signal migration.

import { createSignal, type Accessor } from "solid-js";
import type { ContainerNode } from "@oa/platform/views/types";
import type { GameDialogState } from "../components/GameDialogs";
import type { CollectionDialogMode } from "../components/NewCollectionDialog";
import type { SystemDialogSection } from "../components/SystemDialogs";
import type { RomEntry } from "@oa/platform/library/types";
import type { SystemId } from "@oa/platform/themes/registry";

/// Tile/grid context-menu payload. `entry` is the game the menu was
/// summoned for; `position` is the click coordinate where the menu
/// anchors. Centrally tracked here so any theme's tile component can
/// trigger the menu without prop-drilling through RetroverseShell.
export type ContextMenuPayload = {
  entry: RomEntry;
  position: { x: number; y: number };
};

/// Which help dialog is open, if any. `shortcuts` / `about` / `debug-log`
/// are the 3 dialogs mounted from App.tsx. `null` = none open.
export type HelpDialogKind = "shortcuts" | "about" | "debug-log" | null;

// --- Saves slot picker -------------------------------------------------

const [savesEntrySig, setSavesEntrySig] = createSignal<RomEntry | null>(null);

/// Reactive accessor. The SaveSlotsModal in App.tsx reads this to know
/// when to open + for which game.
export const savesEntry: Accessor<RomEntry | null> = savesEntrySig;

/// Open the saves picker for `entry`, or close it by passing `null`.
export function setSavesEntry(entry: RomEntry | null): void {
  setSavesEntrySig(entry);
}

// --- Tile context menu -------------------------------------------------

const [contextMenuForSig, setContextMenuForSig] = createSignal<ContextMenuPayload | null>(null);

/// Reactive accessor. The TileContextMenu component in App.tsx reads
/// this to render at the anchor position.
export const contextMenuFor: Accessor<ContextMenuPayload | null> = contextMenuForSig;

/// Open the tile context menu, or close by passing `null`.
export function setContextMenuFor(payload: ContextMenuPayload | null): void {
  setContextMenuForSig(payload);
}

// --- Game Info modal ---------------------------------------------------

const [gameInfoForSig, setGameInfoForSig] = createSignal<RomEntry | null>(null);

/// Reactive accessor. GameInfoModal in App.tsx reads this.
export const gameInfoFor: Accessor<RomEntry | null> = gameInfoForSig;

/// Open Game Info for `entry`, or close by passing `null`.
export function setGameInfoFor(entry: RomEntry | null): void {
  setGameInfoForSig(entry);
}

// --- Help dialogs (Shortcuts / About / Debug log) ----------------------

const [helpDialogSig, setHelpDialogSig] = createSignal<HelpDialogKind>(null);

/// Reactive accessor. KeyboardShortcutsDialog / AboutDialog /
/// DebugLogDialog in App.tsx each read this to decide visibility.
export const helpDialog: Accessor<HelpDialogKind> = helpDialogSig;

/// Open a specific help dialog, or close all by passing `null`.
export function setHelpDialog(kind: HelpDialogKind): void {
  setHelpDialogSig(kind);
}

// --- Import Wizard -----------------------------------------------------

const [wizardOpenSig, setWizardOpenSig] = createSignal(false);

/// Reactive accessor. ImportWizard in App.tsx reads this.
export const wizardOpen: Accessor<boolean> = wizardOpenSig;

/// Toggle the Import Wizard.
export function setWizardOpen(open: boolean): void {
  setWizardOpenSig(open);
}

// --- Core picker menu (per-game) ---------------------------------------

/// Anchor payload for the CorePickerMenu — operator right-clicks a tile
/// → "Pick core…" → menu opens at the cursor position with the game's
/// core options listed.
export type CoreMenuPayload = {
  entry: RomEntry;
  position: { x: number; y: number };
};

const [coreMenuForSig, setCoreMenuForSig] = createSignal<CoreMenuPayload | null>(null);
export const coreMenuFor: Accessor<CoreMenuPayload | null> = coreMenuForSig;
export function setCoreMenuFor(payload: CoreMenuPayload | null): void {
  setCoreMenuForSig(payload);
}

// --- Region picker (per-game) ------------------------------------------

const [regionPickerForSig, setRegionPickerForSig] = createSignal<RomEntry | null>(null);
export const regionPickerFor: Accessor<RomEntry | null> = regionPickerForSig;
export function setRegionPickerFor(entry: RomEntry | null): void {
  setRegionPickerForSig(entry);
}

// --- Game properties drawer (per-game) ---------------------------------

const [propertiesForSig, setPropertiesForSig] = createSignal<RomEntry | null>(null);
export const propertiesFor: Accessor<RomEntry | null> = propertiesForSig;
export function setPropertiesFor(entry: RomEntry | null): void {
  setPropertiesForSig(entry);
}

// --- New Collection / Rename Collection dialog -------------------------

const [collectionDialogModeSig, setCollectionDialogModeSig] =
  createSignal<CollectionDialogMode | null>(null);
export const collectionDialogMode: Accessor<CollectionDialogMode | null> =
  collectionDialogModeSig;
export function setCollectionDialogMode(mode: CollectionDialogMode | null): void {
  setCollectionDialogModeSig(mode);
}

// --- Per-game settings focus dialogs (display / shaders / input / etc.) -

const [gameDialogSig, setGameDialogSig] = createSignal<GameDialogState>(null);
export const gameDialog: Accessor<GameDialogState> = gameDialogSig;
export function setGameDialog(state: GameDialogState): void {
  setGameDialogSig(state);
}

// --- Quick Settings overlay (in-game gamepad chord summon) ------------

const [quickSettingsOpenSig, setQuickSettingsOpenSig] = createSignal(false);
export const quickSettingsOpen: Accessor<boolean> = quickSettingsOpenSig;
export function setQuickSettingsOpen(open: boolean): void {
  setQuickSettingsOpenSig(open);
}

// --- Screenshot gallery (per-game viewer) ------------------------------

const [screenshotGalleryForSig, setScreenshotGalleryForSig] =
  createSignal<RomEntry | null>(null);
export const screenshotGalleryFor: Accessor<RomEntry | null> = screenshotGalleryForSig;
export function setScreenshotGalleryFor(entry: RomEntry | null): void {
  setScreenshotGalleryForSig(entry);
}

// --- Sidebar system context menu --------------------------------------

/// System-row right-click payload — `id` is the SystemId picked from
/// the sidebar; `position` is the cursor anchor.
export type SystemContextPayload = {
  id: SystemId;
  position: { x: number; y: number };
};

const [systemContextForSig, setSystemContextForSig] = createSignal<SystemContextPayload | null>(null);
export const systemContextFor: Accessor<SystemContextPayload | null> = systemContextForSig;
export function setSystemContextFor(payload: SystemContextPayload | null): void {
  setSystemContextForSig(payload);
}

// --- Sidebar container context menu -----------------------------------

/// Container-row right-click payload — `container` is the picked node;
/// `position` is the cursor anchor.
export type ContainerContextPayload = {
  container: ContainerNode;
  position: { x: number; y: number };
};

const [containerContextForSig, setContainerContextForSig] =
  createSignal<ContainerContextPayload | null>(null);
export const containerContextFor: Accessor<ContainerContextPayload | null> = containerContextForSig;
export function setContainerContextFor(payload: ContainerContextPayload | null): void {
  setContainerContextForSig(payload);
}

// --- Per-system bindings / core-options / settings dialog -------------

/// Per-system dialog payload — `section` picks which dialog to open
/// (bindings vs core-options vs the catch-all settings sections);
/// `target` is the SystemId being edited.
export type SystemDialogPayload = {
  section: SystemDialogSection;
  target: SystemId;
};

const [systemDialogSig, setSystemDialogSig] = createSignal<SystemDialogPayload | null>(null);
export const systemDialog: Accessor<SystemDialogPayload | null> = systemDialogSig;
export function setSystemDialog(payload: SystemDialogPayload | null): void {
  setSystemDialogSig(payload);
}

/// Convenience for the two callers (QuickSettings, SystemContextMenu)
/// that open the per-system dialog. Preserved from App.tsx's existing
/// `openSystemDialog(section, target)` shape so call sites stay
/// identical.
export function openSystemDialog(section: SystemDialogSection, target: SystemId): void {
  setSystemDialogSig({ section, target });
}
