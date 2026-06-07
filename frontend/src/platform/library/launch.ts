import { invoke } from "@tauri-apps/api/core";
import type { RomEntry } from "./types";

export type LaunchResult =
  | { kind: "launched"; entry: RomEntry; slot?: number }
  | { kind: "skipped-seed"; entry: RomEntry }
  | { kind: "error"; entry: RomEntry; message: string };

/// Launch a ROM. If `slot` is provided, the emu loads the ROM and then
/// immediately restores from that per-game save slot — used by the save-slots
/// modal's slot click. If `stateFile` is provided, the emu loads the ROM and
/// then restores from that absolute state-file path — used by direct-launch
/// `--state-file PATH`. Mutually exclusive with `slot` (Rust ignores the
/// second when both are set; CLI parser rejects both upfront).
export async function launchRom(
  entry: RomEntry,
  slot?: number,
  stateFile?: string,
): Promise<LaunchResult> {
  if (entry.seed) return { kind: "skipped-seed", entry };
  const args = {
    path: entry.filePath,
    slot: slot ?? null,
    stateFile: stateFile ?? null,
    coreOverride: entry.coreOverride ?? null,
    // Drives the launch path through archive::extract_for_launch when set.
    // Rust derives the temp subdir from `entryId` so cleanup_temp matches
    // on unload_rom.
    archiveInnerPath: entry.archiveInnerPath ?? null,
    entryId: entry.id,
    // Drives per-system cores.json lookup + per-system input-bit remap
    // on the emu thread. Defaults to "tg16" on the Rust side if absent.
    systemId: entry.systemId,
  };
  console.log("[oa-launch] invoke launch_rom args:", args);
  try {
    await invoke("launch_rom", args);
    console.log("[oa-launch] launch_rom returned OK");
    return { kind: "launched", entry, slot };
  } catch (e) {
    console.warn("[oa-launch] launch_rom threw:", e);
    return { kind: "error", entry, message: String(e) };
  }
}
