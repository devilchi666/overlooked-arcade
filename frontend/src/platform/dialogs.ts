// Platform — dialog open/close state.
//
// Hosts the 5 dialog signals previously created at App.tsx top-level:
// savesEntry, contextMenuFor, gameInfoFor, helpDialog, wizardOpen.
// Per the 2026-06-06 Theming Substrate decision (recorded in
// docs/features/theming-substrate/SURFACES.md): Platform owns the
// open/close signal; the active theme chooses where to anchor the
// dialog visually.
//
// Phase 1 of ARC 1 migrates only these 5. The other ~12 dialog-shaped
// signals in App.tsx (coreMenuFor, regionPickerFor, propertiesFor,
// collectionDialog, gameDialog, quickSettingsOpen, screenshotGalleryFor,
// systemContextFor, containerContextFor, settingsDialog, systemDialog)
// migrate in Phase 2 alongside the broader platform/ extraction. They
// stay in App.tsx until then to keep this PR's diff focused.
//
// The dialog COMPONENTS still mount from App.tsx — moving their mount
// point lands in Phase 6 when Retroverse becomes a theme on the SDK
// and per-game dialogs gain theme-chosen anchors. Phase 1's deliverable
// is the platform-owned state; the theme-chosen anchor part lands later.

import { createSignal, type Accessor } from "solid-js";
import type { RomEntry } from "../library/types";

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
