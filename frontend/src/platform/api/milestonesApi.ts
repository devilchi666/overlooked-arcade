// Typed Tauri bridge — milestones domain (local achievement / milestone CRUD +
// arming + progress reset).
//
// Theming Phase 4 Slice 5 (PR B, with cheatsApi + captureApi). The local-
// milestone surface: the per-game milestone list + CRUD, arming milestone
// watchers into the running core, and resetting a milestone's triggered state.
// Same convention as the other platform/api modules (see
// docs/PLANS/theming-platform-api-bridge.md): one typed named export per
// command, thin pass-through, no error handling here, command-name string lives
// ONLY in this file.
//
// Per D16 the `Milestone` contract type lives here (the platform↛components
// boundary forbids importing it from GameDialogs); the sole consumer keeps its
// structurally-identical local copy, which stays assignable.

import { invoke } from "@tauri-apps/api/core";

// --- Backend-contract types this domain owns ----------------------------

/// One milestone / local-achievement watcher (`list_milestones` / add / update).
/// `id` absent = new. `triggeredAtUnixMs` set once the condition has fired.
export type Milestone = {
  id?: number;
  gameId: string;
  name: string;
  description: string;
  region: "save_ram" | "rtc" | "system_ram" | "video_ram";
  offset: number;
  width: 1 | 2 | 4;
  op: "eq" | "neq" | "gt" | "lt" | "geq" | "leq";
  target: number;
  edgeOnly: boolean;
  triggeredAtUnixMs?: number;
};

// --- Milestone CRUD + arming --------------------------------------------

/// The milestones saved for a game.
export function listMilestones(gameId: string): Promise<Milestone[]> {
  return invoke<Milestone[]>("list_milestones", { gameId });
}

/// Insert a new milestone; returns the new row id.
export function addMilestone(milestone: Milestone): Promise<number> {
  return invoke<number>("add_milestone", { milestone });
}

/// Update an existing milestone.
export function updateMilestone(milestone: Milestone): Promise<void> {
  return invoke("update_milestone", { milestone });
}

/// Delete a milestone by id.
export function deleteMilestone(id: number): Promise<void> {
  return invoke("delete_milestone", { id });
}

/// Push the game's milestone watchers into the running core; returns the count.
export function armMilestones(gameId: string): Promise<number> {
  return invoke<number>("arm_milestones", { gameId });
}

/// Clear a milestone's triggered state so it can fire again.
export function resetMilestoneProgress(id: number): Promise<void> {
  return invoke("reset_milestone_progress", { id });
}
