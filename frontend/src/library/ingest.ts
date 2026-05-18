import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { allSupportedExtensions, systemForExtension, systemThemes, type SystemId } from "../themes/registry";
import type { LibraryStore } from "./store";
import type { RomEntry } from "./types";

type ScannedRom = {
  path: string;
  fileName: string;
  extension: string;
  /// Set when the entry came from inside an archive. Stored on RomEntry as
  /// archiveInnerPath; routes the launch path through archive extraction.
  archiveInnerPath?: string;
};

export type ScanProgress = {
  jobId: number;
  folder: string;
  filesSeen: number;
  matches: number;
  archived: number;
  currentFile: string;
};

type ScanCompletePayload = {
  jobId: number;
  folder: string;
  matches: number;
  archived: number;
  cancelled: boolean;
  errorMessage?: string;
  rows: ScannedRom[];
};

/// Kick off a background scan in Rust and resolve with the rows when the
/// matching complete event fires. Caller can pass `onProgress` to receive
/// throttled updates (per-file emission throttled to ~12 Hz on the Rust
/// side). Rejects if Rust returns an error or the complete event carries
/// an `errorMessage`.
export async function runBackgroundScan(
  folder: string,
  extensions: string[],
  onProgress?: (p: ScanProgress) => void,
): Promise<ScannedRom[]> {
  let progressUnlisten: UnlistenFn | undefined;
  let completeUnlisten: UnlistenFn | undefined;
  try {
    const result = await new Promise<ScannedRom[]>(async (resolve, reject) => {
      let jobId = -1;
      try {
        progressUnlisten = await listen<ScanProgress>(
          "oa://library-scan-progress",
          (event) => {
            if (event.payload.jobId !== jobId) return;
            onProgress?.(event.payload);
          },
        );
        completeUnlisten = await listen<ScanCompletePayload>(
          "oa://library-scan-complete",
          (event) => {
            if (event.payload.jobId !== jobId) return;
            if (event.payload.errorMessage) {
              reject(new Error(event.payload.errorMessage));
            } else if (event.payload.cancelled) {
              reject(new Error("scan cancelled"));
            } else {
              resolve(event.payload.rows);
            }
          },
        );
        jobId = await invoke<number>("start_background_scan", { folder, extensions });
      } catch (e) {
        reject(e);
      }
    });
    return result;
  } finally {
    progressUnlisten?.();
    completeUnlisten?.();
  }
}

type CoreEntry = {
  fileName: string;
  libraryName: string;
  libraryVersion: string;
  validExtensions: string;
};

/// Build the set of extensions worth scanning for. The registry is the
/// canonical mapping ("a tg16 ROM looks like .pce / .cue / .chd / ..."), but
/// we also union in every extension that any libretro core in <exe_dir>/cores/
/// claims to handle. That way dropping a new core (e.g. mednafen_supergrafx)
/// expands the catalog without anyone editing the registry.
async function resolveScannableExtensions(): Promise<{ extensions: string[]; coreSystemMap: Map<string, SystemId> }> {
  const registryExts = allSupportedExtensions();
  const coreSystemMap = new Map<string, SystemId>();

  // First pass: registry-known extensions get their system mapping from the
  // registry directly (so chd, m3u, etc. correctly map to tg16 today).
  for (const ext of registryExts) {
    const sys = systemForExtension(ext);
    if (sys) coreSystemMap.set(ext, sys);
  }

  // Second pass: cores' valid_extensions. If an extension isn't already
  // mapped to a system in the registry, try to find a system that lists this
  // core's valid_extensions overlap — pick the first match. Falls back to
  // tg16 (today's only system) if nothing matches but a core handles it.
  try {
    const cores = await invoke<CoreEntry[]>("list_cores");
    const knownSystemIds = Object.keys(systemThemes) as SystemId[];
    for (const c of cores) {
      const coreExts = (c.validExtensions ?? "")
        .split("|")
        .map((s) => s.trim().toLowerCase().replace(/^\./, ""))
        .filter(Boolean);
      // Best-effort system guess: the system whose registered extensions
      // overlap most with this core's. If no system is registered for any of
      // the core's extensions, default to the first system in the registry.
      let bestSystem: SystemId | null = null;
      let bestScore = 0;
      for (const sysId of knownSystemIds) {
        const sysExts = systemThemes[sysId].extensions;
        const score = coreExts.filter((e) => sysExts.includes(e)).length;
        if (score > bestScore) {
          bestScore = score;
          bestSystem = sysId;
        }
      }
      const fallback = bestSystem ?? knownSystemIds[0] ?? null;
      for (const ext of coreExts) {
        if (!coreSystemMap.has(ext) && fallback) coreSystemMap.set(ext, fallback);
      }
    }
  } catch {
    // list_cores failing isn't fatal — fall back to registry-only.
  }

  return { extensions: [...coreSystemMap.keys()], coreSystemMap };
}

export type IngestResult =
  | { kind: "cancelled" }
  | { kind: "empty"; folder: string }
  | { kind: "error"; message: string }
  | { kind: "ingested"; folder: string; added: number; skipped: number; total: number };

export function romIdFromPath(path: string): string {
  let h = 5381;
  for (let i = 0; i < path.length; i++) {
    h = (h * 33) ^ path.charCodeAt(i);
  }
  return `rom-${(h >>> 0).toString(36)}`;
}

export function titleFromFileName(name: string): string {
  const lastDot = name.lastIndexOf(".");
  const base = lastDot > 0 ? name.slice(0, lastDot) : name;
  return base.replace(/_+/g, " ").trim();
}

export async function pickFolderAndIngest(
  store: LibraryStore,
  onProgress?: (p: ScanProgress) => void,
): Promise<IngestResult> {
  let picked: string | string[] | null;
  try {
    picked = await open({ directory: true, multiple: false });
  } catch (e) {
    return { kind: "error", message: `folder picker failed: ${String(e)}` };
  }
  if (picked === null || Array.isArray(picked)) return { kind: "cancelled" };
  return ingestFolderPath(store, picked, onProgress);
}

/// Ingest a folder by absolute path — bypasses the dialog. Used by the
/// window-level drag-drop handler (Tauri 2 onDragDropEvent payload contains
/// the dropped paths) and by the toolbar's Import folder option after the
/// dialog resolves. Runs the scan in Rust on a tokio blocking task; the
/// optional `onProgress` callback fires throttled progress events while the
/// walk is in flight (~12 Hz max).
export async function ingestFolderPath(
  store: LibraryStore,
  folder: string,
  onProgress?: (p: ScanProgress) => void,
): Promise<IngestResult> {
  const { extensions, coreSystemMap } = await resolveScannableExtensions();

  let scanned: ScannedRom[];
  try {
    scanned = await runBackgroundScan(folder, extensions, onProgress);
  } catch (e) {
    return { kind: "error", message: `scan failed: ${String(e)}` };
  }

  if (scanned.length === 0) return { kind: "empty", folder };

  const now = Date.now();
  const entries: RomEntry[] = [];
  let skipped = 0;
  for (const r of scanned) {
    const systemId = coreSystemMap.get(r.extension) ?? systemForExtension(r.extension);
    if (!systemId) {
      skipped++;
      continue;
    }
    entries.push({
      id: romIdFromPath(r.path),
      title: titleFromFileName(r.fileName),
      systemId,
      filePath: r.path,
      addedAt: now,
      ...(r.archiveInnerPath ? { archiveInnerPath: r.archiveInnerPath } : {}),
    });
  }

  const added = await store.addScannedRoms(entries);
  return { kind: "ingested", folder, added, skipped: skipped + (entries.length - added), total: scanned.length };
}

export type RescanSummary = {
  folders: number;
  totalAdded: number;
  errors: string[];
};

export async function rescanFolders(
  store: LibraryStore,
  folders: string[],
  onProgress?: (p: ScanProgress) => void,
): Promise<RescanSummary> {
  const { extensions, coreSystemMap } = await resolveScannableExtensions();
  const errors: string[] = [];
  const now = Date.now();
  let totalAdded = 0;
  for (const folder of folders) {
    let scanned: ScannedRom[];
    try {
      scanned = await runBackgroundScan(folder, extensions, onProgress);
    } catch (e) {
      errors.push(`${folder}: ${String(e)}`);
      continue;
    }
    const entries: RomEntry[] = [];
    for (const r of scanned) {
      const systemId = coreSystemMap.get(r.extension) ?? systemForExtension(r.extension);
      if (!systemId) continue;
      entries.push({
        id: romIdFromPath(r.path),
        title: titleFromFileName(r.fileName),
        systemId,
        filePath: r.path,
        addedAt: now,
        ...(r.archiveInnerPath ? { archiveInnerPath: r.archiveInnerPath } : {}),
      });
    }
    totalAdded += await store.addScannedRoms(entries);
  }
  return { folders: folders.length, totalAdded, errors };
}
