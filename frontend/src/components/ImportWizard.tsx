import { createEffect, createSignal, For, onCleanup, onMount, Show, type Component } from "solid-js";
import { captureFocusReturn, useDomQueryFocusGroup } from "../nav/focus";
import { useBackHandler } from "../nav/back";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open as pickDirectory } from "@tauri-apps/plugin-dialog";
import {
  romIdFromPath,
  titleFromFileName,
  type ScanProgress,
} from "../library/ingest";
import type { LibraryStore } from "../library/store";
import type { RomEntry } from "../library/types";
import type { SettingsStore } from "../settings/store";
import {
  allSupportedExtensions,
  systemForExtension,
  systemThemes,
  type SystemId,
} from "../themes/registry";
import { ScummvmDetectDialog } from "./ScummvmDetectDialog";

// Phase 2.7 slice C — Import wizard.
//
// 4-step modal driving folder import end-to-end. Sits alongside (not above)
// the existing dialog flow in App.tsx — the wizard is opened explicitly
// from Settings → Library → "Re-scan with smart detection" (Phase 1B
// Slice 1). The dialog-then-progress fallback that shipped in slice B
// stays as a quick path for users who don't need the per-folder rules
// editor: the LibraryView empty-state "Import folder" button and
// Settings → Library → Add folder both route through it.
//
// Step 1: pick a folder + scan toggles + watch toggle.
// Step 2: extension → system mapping editor. Pre-populated from the registry,
//         user can override per extension, persisted via set_folder_rules.
// Step 3: live preview — runs the background scan, shows progress + per-
//         system tally, cancellable.
// Step 4: confirm + optional post-import auto-sync of media + metadata.
//
// Commit writes to: SQLite folders + folder_rules (new in this slice) AND
// settings.libraryFolders (so the existing watcher + Rescan-all flows keep
// working unchanged).

type ScannedRom = {
  path: string;
  fileName: string;
  extension: string;
  archiveInnerPath?: string;
  /// Backend-emitted classification hint. Set when the scan walker
  /// can't disambiguate via extension alone — Neo Geo .zip ROM-sets,
  /// or any directory-mode scan (dosbox) where there's no extension
  /// to match against. When present, the bucketing logic prefers it
  /// over the extension lookup.
  systemHint?: string;
  // Phase 1B Slice 1 — smart-classification fields emitted by the
  // Rust scanner. Slice 1 plumbs them through; the per-ROM results
  // table in Slice 2 is the first UI consumer.
  systemId?: SystemId;
  suggestedTitle?: string;
  confidence?: "hash" | "header" | "extension" | "hint";
  sha1?: string;
};

type Folder = {
  id: string;
  path: string;
  scanSubfolders: boolean;
  subfoldersAreSystems: boolean;
  watchEnabled: boolean;
  lastScannedAt?: number;
  rules?: FolderRule[];
};

type FolderRule = {
  id?: number;
  folderId: string;
  matchPattern: string;
  systemId: string;
};

type RuleDraft = {
  /// Frontend-only id so SolidJS keyed rendering doesn't recycle DOM nodes
  /// when patterns/systems change. Not persisted.
  uiKey: string;
  pattern: string;
  systemId: SystemId;
};

type Step = 1 | 2 | 3 | 4;

type Props = {
  open: boolean;
  onClose: () => void;
  library: LibraryStore;
  settings: SettingsStore;
  /// Bridges to the existing scan-progress status bar in App.tsx so the
  /// toolbar still surfaces live updates while the wizard is on Step 3.
  onStatus?: (status: string) => void;
};

const BTN =
  "rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs font-medium uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:cursor-not-allowed disabled:opacity-50";
const BTN_PRIMARY =
  "rounded-md bg-(--color-system-accent) px-3 py-1.5 text-xs font-semibold uppercase tracking-wider text-black/90 transition hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-50";
const INPUT =
  "w-full rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-sm text-(--color-oa-ink) placeholder:text-(--color-oa-ink-dim) focus-visible:border-(--color-system-accent) focus-visible:outline-none disabled:opacity-50";
const SELECT =
  "rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) focus-visible:border-(--color-system-accent) focus-visible:outline-none disabled:opacity-50";

const STEP_LABELS: Record<Step, string> = {
  1: "Folder",
  2: "Mapping",
  3: "Preview",
  4: "Confirm",
};

function nextUiKey(): string {
  return `rule-${Math.random().toString(36).slice(2, 10)}`;
}

function patternToExtension(pattern: string): string | null {
  const trimmed = pattern.trim().toLowerCase();
  // Accept "*.pce", "*.PCE", ".pce", "pce" — normalize to bare ext.
  const m = trimmed.match(/^\*?\.?([a-z0-9]+)$/);
  return m ? m[1] : null;
}

/// Directory-mode systems — each top-level subdir of the operator's
/// library folder is one game. Renders in the rules editor with a
/// "(folder)" sentinel pattern that signals intent (the pattern
/// itself doesn't match any file extension; the wizard branches on
/// systemId to fire the directory-mode scanner). Currently just
/// dosbox; future engine launchers slot in here.
const DIR_MODE_SYSTEMS: readonly SystemId[] = ["dosbox"] as const;
const DIR_MODE_PATTERN = "(folder)";

function defaultRulesFromRegistry(): RuleDraft[] {
  // One row per extension in the registry, plus one row per
  // directory-mode system. Order: by system, then by ext.
  const out: RuleDraft[] = [];
  const sysIds = Object.keys(systemThemes) as SystemId[];
  for (const sysId of sysIds) {
    for (const ext of systemThemes[sysId].extensions) {
      out.push({ uiKey: nextUiKey(), pattern: `*.${ext}`, systemId: sysId });
    }
    if (DIR_MODE_SYSTEMS.includes(sysId)) {
      out.push({ uiKey: nextUiKey(), pattern: DIR_MODE_PATTERN, systemId: sysId });
    }
  }
  return out;
}

function rulesFromPersisted(persisted: FolderRule[]): RuleDraft[] {
  return persisted
    .map((r) => ({
      uiKey: nextUiKey(),
      pattern: r.matchPattern,
      systemId: r.systemId as SystemId,
    }))
    .filter((r) => systemThemes[r.systemId] !== undefined);
}

const ImportWizard: Component<Props> = (props) => {
  const [step, setStep] = createSignal<Step>(1);
  const [folder, setFolder] = createSignal<string | null>(null);
  const [scanSubfolders, setScanSubfolders] = createSignal(true);
  const [subfoldersAreSystems, setSubfoldersAreSystems] = createSignal(false);
  const [watchEnabled, setWatchEnabled] = createSignal(true);
  const [rules, setRules] = createSignal<RuleDraft[]>(defaultRulesFromRegistry());

  // Cached list of currently-tracked folders so we can pre-populate the rules
  // editor when the user picks a folder that's already been imported.
  const [trackedFolders, setTrackedFolders] = createSignal<Folder[]>([]);
  // The Folder row for the currently-selected path — null when this is a
  // fresh add. Determines commit-time UPDATE vs INSERT.
  const [existingFolder, setExistingFolder] = createSignal<Folder | null>(null);
  // Top-level empty-directory pre-flight: null = unchecked / checking,
  // true = empty (block Next + show banner), false = has entries.
  // Pre-flight check is intentionally top-level only (cheap O(1) read_dir);
  // post-scan "no supported ROMs" already covers the deeper edge cases.
  const [folderEmpty, setFolderEmpty] = createSignal<boolean | null>(null);
  // Error from the empty-check call itself (path doesn't exist, permission
  // denied, etc.) — surfaced inline next to the folder input.
  const [folderCheckError, setFolderCheckError] = createSignal<string | null>(null);

  // Step 3 scan state.
  const [scanRows, setScanRows] = createSignal<ScannedRom[]>([]);
  const [scanProgress, setScanProgress] = createSignal<ScanProgress | null>(null);
  const [scanRunning, setScanRunning] = createSignal(false);
  const [scanJobId, setScanJobId] = createSignal<number | null>(null);
  const [scanError, setScanError] = createSignal<string | null>(null);
  const [scanCancelled, setScanCancelled] = createSignal(false);

  // ScummVM auto-detect dialog — opened from step 2's banner button
  // when the active rule set includes a `scummvm` rule. The dialog
  // walks the picked folder for game subdirs, runs the curated
  // sentinel-filename detection, and writes `.scummvm` descriptors
  // for operator-confirmed matches. After the dialog closes the
  // operator advances to step 3 and the regular scan picks up the
  // newly-written descriptors.
  const [scummvmDetectOpen, setScummvmDetectOpen] = createSignal(false);

  // Step 4 sync options.
  const [syncCovers, setSyncCovers] = createSignal(true);
  const [syncSnaps, setSyncSnaps] = createSignal(true);
  const [syncTitles, setSyncTitles] = createSignal(true);
  const [syncMetadata, setSyncMetadata] = createSignal(true);
  const [committing, setCommitting] = createSignal(false);
  const [commitError, setCommitError] = createSignal<string | null>(null);

  // Listener lifecycle for Step 3 — kept here (not inside startScan) so
  // onCleanup can tear them down if the modal closes mid-scan.
  let progressUnlisten: UnlistenFn | undefined;
  let completeUnlisten: UnlistenFn | undefined;
  let progressUnlistenDone = false;
  let completeUnlistenDone = false;

  function teardownListeners() {
    if (progressUnlisten && !progressUnlistenDone) {
      progressUnlisten();
      progressUnlistenDone = true;
    }
    if (completeUnlisten && !completeUnlistenDone) {
      completeUnlisten();
      completeUnlistenDone = true;
    }
  }

  // Refresh tracked-folders cache when the modal opens. List with rules so
  // we can pre-populate the editor without a second round-trip.
  createEffect(() => {
    if (!props.open) return;
    void (async () => {
      try {
        const folders = await invoke<Folder[]>("list_folders", { includeRules: true });
        setTrackedFolders(folders);
      } catch (e) {
        console.warn("[oa-wizard] list_folders failed:", e);
      }
    })();
  });

  // Each time `folder` changes, check if it's already tracked and pre-load
  // its rules — otherwise reset to registry defaults. Also run the
  // top-level empty-directory pre-flight so Step 1 can warn before the
  // user wastes time stepping through Mapping into a no-op scan.
  createEffect(() => {
    const f = folder();
    if (!f) {
      setExistingFolder(null);
      setFolderEmpty(null);
      setFolderCheckError(null);
      return;
    }
    const tracked = trackedFolders().find((t) => t.path === f);
    if (tracked) {
      setExistingFolder(tracked);
      setScanSubfolders(tracked.scanSubfolders);
      setSubfoldersAreSystems(tracked.subfoldersAreSystems);
      setWatchEnabled(tracked.watchEnabled);
      const persisted = tracked.rules ?? [];
      setRules(persisted.length > 0 ? rulesFromPersisted(persisted) : defaultRulesFromRegistry());
    } else {
      setExistingFolder(null);
      setRules(defaultRulesFromRegistry());
    }
    setFolderEmpty(null);
    setFolderCheckError(null);
    void (async () => {
      try {
        const empty = await invoke<boolean>("directory_is_empty", { path: f });
        // Race guard: only apply if the user hasn't picked a different
        // folder while the IPC was in flight.
        if (folder() !== f) return;
        setFolderEmpty(empty);
      } catch (e) {
        if (folder() !== f) return;
        setFolderCheckError(String(e));
      }
    })();
  });

  // Reset all transient state when the modal closes.
  createEffect(() => {
    if (props.open) return;
    setStep(1);
    setFolder(null);
    setScanSubfolders(true);
    setSubfoldersAreSystems(false);
    setWatchEnabled(true);
    setRules(defaultRulesFromRegistry());
    setExistingFolder(null);
    setFolderEmpty(null);
    setFolderCheckError(null);
    setScanRows([]);
    setScanProgress(null);
    setScanRunning(false);
    setScanJobId(null);
    setScanError(null);
    setScanCancelled(false);
    setCommitError(null);
    setCommitting(false);
    teardownListeners();
  });

  // ESC closes the modal (when not actively scanning — there a confirm would
  // prevent accidentally cancelling a long walk; keeping that simple here).
  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!props.open) return;
      if (e.key === "Escape" && !scanRunning() && !committing()) {
        e.preventDefault();
        props.onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    onCleanup(() => window.removeEventListener("keydown", onKey));
  });

  onCleanup(() => teardownListeners());

  async function handleBrowse() {
    const picked = await pickDirectory({ directory: true, multiple: false }).catch(() => null);
    if (!picked || Array.isArray(picked)) return;
    setFolder(picked);
  }

  function pickRecent(path: string) {
    setFolder(path);
  }

  function addRule() {
    setRules((prev) => [
      ...prev,
      { uiKey: nextUiKey(), pattern: "*.", systemId: (Object.keys(systemThemes)[0] ?? "tg16") as SystemId },
    ]);
  }

  function updateRule(uiKey: string, patch: Partial<RuleDraft>) {
    setRules((prev) => prev.map((r) => (r.uiKey === uiKey ? { ...r, ...patch } : r)));
  }

  function removeRule(uiKey: string) {
    setRules((prev) => prev.filter((r) => r.uiKey !== uiKey));
  }

  function resetRulesToDefaults() {
    setRules(defaultRulesFromRegistry());
  }

  /// Build an extension → SystemId map from the current rules. Later rules
  /// for the same ext override earlier ones (last-write-wins).
  function ruleMap(): Map<string, SystemId> {
    const m = new Map<string, SystemId>();
    for (const r of rules()) {
      const ext = patternToExtension(r.pattern);
      if (!ext) continue;
      if (!systemThemes[r.systemId]) continue;
      m.set(ext, r.systemId);
    }
    return m;
  }

  /// Bucket scanned rows by system using rules + registry fallback. Returns
  /// matched RomEntries + a Map<extension, count> of unmatched.
  /// Resolve a scanned row to its system_id, preferring (in order):
  ///   1. the backend-emitted `systemHint` (Neo Geo zip disambiguation,
  ///      dosbox directory scan), since the walker has more context;
  ///   2. the rule map (user-set extension → system override);
  ///   3. the registry extension lookup (default mapping).
  /// Returns null for rows that don't classify (kept as "unmatched").
  function classifyScanRow(r: ScannedRom): SystemId | null {
    if (r.systemHint && systemThemes[r.systemHint as SystemId]) {
      return r.systemHint as SystemId;
    }
    return ruleMap().get(r.extension) ?? systemForExtension(r.extension);
  }

  function bucketScanned() {
    const t0 = performance.now();
    const now = Date.now();
    const entries: RomEntry[] = [];
    const unmatched = new Map<string, number>();
    const rowCount = scanRows().length;
    for (const r of scanRows()) {
      const sysId = classifyScanRow(r);
      if (!sysId) {
        unmatched.set(r.extension, (unmatched.get(r.extension) ?? 0) + 1);
        continue;
      }
      entries.push({
        id: romIdFromPath(r.path),
        title: titleFromFileName(r.fileName),
        systemId: sysId,
        filePath: r.path,
        addedAt: now,
        ...(r.archiveInnerPath ? { archiveInnerPath: r.archiveInnerPath } : {}),
      });
    }
    const ms = performance.now() - t0;
    // Quiet on fast calls so the log isn't drowned; loud when it's slow.
    if (ms > 10) {
      console.log(`[oa-wizard] bucketScanned rows=${rowCount} entries=${entries.length} took ${ms.toFixed(1)}ms`);
    }
    return { entries, unmatched };
  }

  /// Per-system tally for the Step 3 found-so-far box.
  function bucketTally(): { systemId: SystemId; count: number; archived: number }[] {
    const tally = new Map<SystemId, { count: number; archived: number }>();
    for (const r of scanRows()) {
      const sysId = classifyScanRow(r);
      if (!sysId) continue;
      const cur = tally.get(sysId) ?? { count: 0, archived: 0 };
      cur.count++;
      if (r.archiveInnerPath) cur.archived++;
      tally.set(sysId, cur);
    }
    return [...tally.entries()]
      .map(([systemId, v]) => ({ systemId, count: v.count, archived: v.archived }))
      .sort((a, b) => b.count - a.count);
  }

  function unmatchedTally(): { extension: string; count: number }[] {
    const { unmatched } = bucketScanned();
    return [...unmatched.entries()]
      .map(([extension, count]) => ({ extension, count }))
      .sort((a, b) => b.count - a.count);
  }

  /// Union of registry exts + cores' valid_extensions. Same shape as
  /// resolveScannableExtensions in ingest.ts — but the wizard scopes more
  /// tightly to the active rule set, so we expand to "everything the user
  /// might want to scan" via the registry + the user-defined patterns.
  async function buildScanExtensionList(): Promise<string[]> {
    const exts = new Set<string>(allSupportedExtensions());
    for (const r of rules()) {
      const ext = patternToExtension(r.pattern);
      if (ext) exts.add(ext);
    }
    // Pull core valid_extensions too so a newly-dropped libretro .dll
    // expands the scan without anyone editing the registry. Failures are
    // soft — we just fall back to the registry+rule union.
    try {
      type CoreEntry = { validExtensions: string };
      const cores = await invoke<CoreEntry[]>("list_cores");
      for (const c of cores) {
        for (const e of (c.validExtensions ?? "").split("|")) {
          const norm = e.trim().toLowerCase().replace(/^\./, "");
          if (norm) exts.add(norm);
        }
      }
    } catch {
      // ignore
    }
    return [...exts];
  }

  async function startScan() {
    const f = folder();
    if (!f) return;
    // Belt-and-braces: tear down any prior listeners before re-arming.
    // Pre-fix `startScan` would just overwrite `progressUnlisten` /
    // `completeUnlisten` if called twice (rescan from step 3, or
    // Back→Next re-firing the auto-start effect). The previous
    // listeners stayed attached to Tauri's event bus for the lifetime
    // of the modal — orphaned, never unregistered. Each one filtered
    // events by jobId (current call's payload would be dropped by the
    // orphan listener since jobId didn't match), so behaviour was
    // "right" but every replay leaked a listener + a small allocation
    // for its filter closure.
    teardownListeners();

    setScanRunning(true);
    setScanError(null);
    setScanRows([]);
    setScanProgress(null);
    setScanCancelled(false);

    const extensions = await buildScanExtensionList();
    // Directory-mode scan dispatch: dosbox library folders contain one
    // subdirectory per game and have no scannable file extensions. When
    // any active rule maps to a directory-mode system, we also fire
    // the directory-mode scanner and merge its results into the same
    // scanRows() list.
    const dirModeSystems = new Set<SystemId>();
    for (const r of rules()) {
      if (DIR_MODE_SYSTEMS.includes(r.systemId)) {
        dirModeSystems.add(r.systemId);
      }
    }

    // Set of jobIds we're waiting on. Each complete event removes its
    // jobId; the overall scan is "done" when the set empties.
    const pendingJobs = new Set<number>();
    // Buffer for complete events that arrive BEFORE their jobId lands in
    // pendingJobs. Race fix: a directory-mode scan over an empty subdir
    // tree finishes in ~2 ms — faster than `await invoke()` round-trips
    // jobId back to the JS thread, so the complete event hits this
    // listener with pendingJobs.has(jobId) == false. Pre-fix we dropped
    // those events; the parent scan then never finished because its
    // jobId stayed orphan-added to pendingJobs forever. Now we buffer
    // by jobId; registerJob() drains the buffer once the jobId lands.
    const earlyCompletes = new Map<number, {
      jobId: number;
      cancelled: boolean;
      errorMessage?: string;
      rows: ScannedRom[];
    }>();
    const accumulatedRows: ScannedRom[] = [];
    let firstError: string | null = null;
    let anyCancelled = false;

    function applyCompletePayload(payload: {
      jobId: number;
      cancelled: boolean;
      errorMessage?: string;
      rows: ScannedRom[];
    }) {
      pendingJobs.delete(payload.jobId);
      if (payload.errorMessage) {
        firstError = firstError ?? payload.errorMessage;
      } else if (payload.cancelled) {
        anyCancelled = true;
      } else {
        accumulatedRows.push(...payload.rows);
      }
      finishIfDone();
    }

    /// Call this with the jobId returned from `invoke(start_background_scan)`.
    /// Adds it to pendingJobs AND drains any complete event for this jobId
    /// that already arrived (the race the dosbox empty-dir scan loses).
    function registerJob(jobId: number) {
      pendingJobs.add(jobId);
      const buffered = earlyCompletes.get(jobId);
      if (buffered) {
        earlyCompletes.delete(jobId);
        // Keep this log — fires only when the race wins. Useful
        // regression signal if the buffer-then-drain path stops
        // working in a future refactor.
        console.log(`[oa-wizard] registerJob job=${jobId} draining buffered early-complete`);
        applyCompletePayload(buffered);
      }
    }

    function finishIfDone() {
      if (pendingJobs.size > 0) return;
      if (firstError) {
        setScanError(firstError);
      } else if (anyCancelled) {
        setScanCancelled(true);
      } else {
        setScanRows(accumulatedRows);
      }
      setScanRunning(false);
      setScanJobId(null);
      teardownListeners();
    }

    try {
      progressUnlistenDone = false;
      completeUnlistenDone = false;
      progressUnlisten = await listen<ScanProgress>(
        "oa://library-scan-progress",
        (event) => {
          if (!pendingJobs.has(event.payload.jobId)) return;
          setScanProgress(event.payload);
          props.onStatus?.(
            `Scanning ${event.payload.folder}: ${event.payload.matches} matched (${event.payload.archived} archived)`,
          );
        },
      );
      completeUnlisten = await listen<{
        jobId: number;
        folder: string;
        matches: number;
        archived: number;
        cancelled: boolean;
        errorMessage?: string;
        rows: ScannedRom[];
      }>("oa://library-scan-complete", (event) => {
        if (!pendingJobs.has(event.payload.jobId)) {
          // Race fix: jobId hasn't been registerJob()'d yet because the
          // backend scan finished faster than `await invoke()` could
          // round-trip the jobId back to us. Buffer the payload; when
          // registerJob(jobId) runs, it'll drain the buffer. Logged
          // because this happens only when the race wins — if a future
          // regression resurrects the "wizard freezes on Step 3" bug,
          // this line in oa-current.log is the trigger to look for.
          console.log(`[oa-wizard] complete event buffered (jobId ${event.payload.jobId} arrived before invoke returned)`);
          earlyCompletes.set(event.payload.jobId, {
            jobId: event.payload.jobId,
            cancelled: event.payload.cancelled,
            errorMessage: event.payload.errorMessage,
            rows: event.payload.rows,
          });
          return;
        }
        applyCompletePayload(event.payload);
      });
      // Kick off the extension-mode scan unconditionally — it's the
      // path 38+ existing systems use, and it costs nothing for
      // dosbox-only folders since there are no scannable extensions.
      // Phase 1B Slice 1: pass the current rule map as extensionToSystem
      // so the Rust smart-classification stage knows which system to
      // hash each row against.
      const extensionToSystem = Object.fromEntries(ruleMap());
      const extJobId = await invoke<number>("start_background_scan", {
        folder: f,
        extensions,
        extensionToSystem,
      });
      registerJob(extJobId);
      // The Cancel button targets the extension job — dosbox dir scans
      // are short enough that cancelling the visible one is sufficient.
      setScanJobId(extJobId);
      // Kick off any directory-mode scans alongside.
      for (const sysId of dirModeSystems) {
        const dirJobId = await invoke<number>("start_background_directory_scan", {
          folder: f,
          systemId: sysId,
        });
        registerJob(dirJobId);
      }
    } catch (e) {
      setScanError(String(e));
      setScanRunning(false);
      teardownListeners();
    }
  }

  async function cancelScan() {
    const jid = scanJobId();
    if (jid === null) return;
    try {
      await invoke("cancel_background_scan", { jobId: jid });
    } catch (e) {
      console.warn("[oa-wizard] cancel_background_scan failed:", e);
    }
  }

  // When entering Step 3, auto-start the scan once (unless one's already
  // landed). Re-running the scan is via the manual Restart button.
  createEffect(() => {
    if (step() === 3 && !scanRunning() && scanRows().length === 0 && !scanError() && !scanCancelled()) {
      void startScan();
    }
  });

  async function commit(withSync: boolean) {
    const f = folder();
    if (!f) return;
    setCommitting(true);
    setCommitError(null);
    try {
      // 1) Upsert the folder + replace its rules.
      let folderRow = existingFolder();
      if (folderRow) {
        await invoke("update_folder", {
          id: folderRow.id,
          fields: {
            scanSubfolders: scanSubfolders(),
            subfoldersAreSystems: subfoldersAreSystems(),
            watchEnabled: watchEnabled(),
            lastScannedAt: Date.now(),
          },
        });
      } else {
        folderRow = await invoke<Folder>("add_folder", {
          path: f,
          scanSubfolders: scanSubfolders(),
          subfoldersAreSystems: subfoldersAreSystems(),
          watchEnabled: watchEnabled(),
        });
        await invoke("update_folder", {
          id: folderRow.id,
          fields: { lastScannedAt: Date.now() },
        });
      }
      // Replace rules — empty array clears any prior rules so users can
      // remove everything in the editor and get a clean slate.
      const rulesPayload: FolderRule[] = rules()
        .filter((r) => patternToExtension(r.pattern) !== null && systemThemes[r.systemId] !== undefined)
        .map((r) => ({
          folderId: folderRow!.id,
          matchPattern: r.pattern,
          systemId: r.systemId,
        }));
      await invoke("set_folder_rules", {
        folderId: folderRow.id,
        rules: rulesPayload,
      });

      // 2) Add games. Reuse the existing library store path so the seed-row
      // drop + cache-refresh side effects fire correctly.
      const { entries } = bucketScanned();
      const added = entries.length > 0 ? await props.library.addScannedRoms(entries) : 0;

      // 3) Refresh the settings store's SQLite-backed folder signal so the
      // Settings → Library tab + watcher effect + Rescan-all menu pick up
      // the new row immediately. (Before 2026-05-21 we mirrored the path
      // into a localStorage list; SQLite is now the single source of
      // truth — see DECISIONS.md "Library folders: SQLite is the single
      // source of truth".)
      await props.settings.refreshLibraryFolders();

      // 4) Auto-identify FIRST so subsequent media + metadata syncs see
      // hash-matched entries. The `only_identified` filter inside
      // sync_media_for_system is evaluated once at function start;
      // without sha1s stamped on the library rows, every cart game is
      // filtered out and the sync no-ops with "matched 0". Awaiting
      // resolve_rom_hashes_for_system before the syncs fire fixes the
      // race that previously left covers + metadata empty after a fresh
      // Import + sync (see DECISIONS.md 2026-05-21 "Post-import sync
      // ordering" entry).
      //
      // The resolve is awaited per system rather than fired in parallel
      // because resolve_rom_hashes_for_system holds the LibraryDb write
      // lock for the duration of its DB applies; parallelizing across
      // systems would just queue them up behind each other anyway. For
      // a typical single-system import this is a single await; for a
      // mixed-system import it's a sequence in order of touched
      // systems.
      if (added > 0) {
        const touchedSystems = Array.from(new Set(entries.map((e) => e.systemId))) as SystemId[];
        for (const systemId of touchedSystems) {
          try {
            await invoke("resolve_rom_hashes_for_system", { systemId });
          } catch (err) {
            console.warn(`[oa-wizard] auto-identify ${systemId} failed:`, err);
            // Best-effort: even if one system's resolve fails we still
            // fire the media/metadata syncs for the others. The failed
            // system's syncs will just see 0 hash-matched entries —
            // same as the pre-fix behaviour for everyone, so no
            // regression on the failure path.
          }
        }
      }

      // 5) Post-import media + metadata sync. Best-effort — failures
      // here don't block the wizard from completing. Each system that
      // got at least one entry triggers its own sync. Media + metadata
      // are independent of each other; both safely run in parallel now
      // that sha1s are stamped.
      if (withSync && entries.length > 0) {
        const systemEntries = new Map<SystemId, RomEntry[]>();
        for (const e of entries) {
          const arr = systemEntries.get(e.systemId) ?? [];
          arr.push(e);
          systemEntries.set(e.systemId, arr);
        }
        // Fire-and-forget — the actual sync emits its own progress events
        // and we don't want the wizard to block on a long network walk.
        for (const [systemId, sysEntries] of systemEntries) {
          const syncEntries = sysEntries.map((e) => ({
            id: e.id,
            title: e.title,
            filePath: e.filePath,
            systemId: e.systemId,
          }));
          if (syncCovers() || syncSnaps() || syncTitles()) {
            void invoke("sync_media_for_system", {
              systemId,
              entries: syncEntries,
            }).catch((err) =>
              console.warn(`[oa-wizard] sync_media_for_system(${systemId}) failed:`, err),
            );
          }
          if (syncMetadata()) {
            void invoke("sync_metadata_for_system", {
              systemId,
              entries: syncEntries,
            }).catch((err) =>
              console.warn(`[oa-wizard] sync_metadata_for_system(${systemId}) failed:`, err),
            );
          }
        }
      }

      props.onStatus?.(`Added ${added} of ${entries.length} from ${f}.`);
      setCommitting(false);
      props.onClose();
    } catch (e) {
      setCommitError(String(e));
      setCommitting(false);
    }
  }

  // --- Per-step UI ----------------------------------------------------

  const Step1 = () => (
    <div class="flex flex-col gap-5">
      <p class="text-xs uppercase tracking-widest text-(--color-oa-ink-dim)">
        Pick a folder to scan.
      </p>
      <div class="flex gap-2">
        <input
          type="text"
          readOnly
          value={folder() ?? ""}
          placeholder="No folder selected"
          class={INPUT}
        />
        <button type="button" class={BTN} onClick={handleBrowse}>
          Browse…
        </button>
      </div>
      <Show when={props.settings.libraryFolders().length > 0}>
        <div class="flex flex-col gap-1.5">
          <p class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            Recently tracked
          </p>
          <select
            class={INPUT}
            value={folder() ?? ""}
            onChange={(e) => {
              const v = e.currentTarget.value;
              if (v) pickRecent(v);
            }}
          >
            <option value="">— pick a tracked folder —</option>
            <For each={props.settings.libraryFolders()}>
              {(p) => <option value={p}>{p}</option>}
            </For>
          </select>
        </div>
      </Show>
      <Show when={existingFolder() !== null}>
        <div class="rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/10 px-3 py-2 text-xs text-(--color-oa-ink)">
          This folder is already tracked. Its existing rules will be loaded; the
          wizard will rescan and update the library.
        </div>
      </Show>
      <Show when={folderEmpty() === true}>
        <div class="rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-xs text-(--color-oa-ink)">
          <p class="font-semibold">This folder is empty.</p>
          <p class="mt-0.5 text-(--color-oa-ink-dim)">
            Pick a different folder — there's nothing to scan here.
          </p>
        </div>
      </Show>
      <Show when={folderCheckError() !== null}>
        <div class="rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-xs text-(--color-oa-ink)">
          Couldn't read the folder: {folderCheckError()}
        </div>
      </Show>
      <div class="flex flex-col gap-2 rounded-md border border-white/5 bg-black/20 px-3 py-2.5">
        <Toggle
          label="Scan subfolders"
          checked={scanSubfolders()}
          onChange={setScanSubfolders}
        />
        <Toggle
          label="Treat each subfolder as a system (folder name → system)"
          checked={subfoldersAreSystems()}
          onChange={setSubfoldersAreSystems}
          disabled={!scanSubfolders()}
          hint="Persisted with the folder; informs mapping in the next step."
        />
        <Toggle
          label="Watch for new ROMs in this folder"
          checked={watchEnabled()}
          onChange={setWatchEnabled}
          hint="Newly-dropped files auto-add to the library."
        />
      </div>
    </div>
  );

  const Step2 = () => (
    <div class="flex flex-col gap-4">
      <div class="flex items-center justify-between">
        <p class="text-xs uppercase tracking-widest text-(--color-oa-ink-dim)">
          For this folder, ROMs of type:
        </p>
        <div class="flex gap-2">
          <button type="button" class={BTN} onClick={resetRulesToDefaults}>
            Reset
          </button>
          <button type="button" class={BTN} onClick={addRule}>
            + Add rule
          </button>
        </div>
      </div>
      {/* Banner — visible when a scummvm rule is in the active set.
          Opens the ScummvmDetectDialog scoped to the wizard's
          currently-picked folder. Operator runs detection, confirms
          matches, writes `.scummvm` descriptors, then advances to
          Step 3 where the regular extension scan picks them up. */}
      <Show when={rules().some((r) => r.systemId === "scummvm") && folder()}>
        <div class="flex items-center justify-between rounded-md border border-(--color-system-accent)/30 bg-(--color-system-accent)/10 px-3 py-2 text-xs">
          <div>
            <p class="text-(--color-oa-ink)">ScummVM games in this folder?</p>
            <p class="text-[0.65rem] text-(--color-oa-ink-dim)">
              Auto-detect known SCUMM titles + ScummVM freewares and
              generate <code class="font-mono">.scummvm</code> descriptor
              files in bulk — no need to hand-craft one per game.
            </p>
          </div>
          <button
            type="button"
            onClick={() => setScummvmDetectOpen(true)}
            class="shrink-0 rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/20 px-3 py-1.5 text-xs uppercase tracking-wider text-(--color-system-accent-soft) hover:bg-(--color-system-accent)/30"
          >
            Detect ScummVM games…
          </button>
        </div>
      </Show>
      <div class="flex max-h-[420px] flex-col gap-1.5 overflow-y-auto pr-1">
        <For
          each={rules()}
          fallback={
            <p class="rounded-md border border-dashed border-white/10 px-3 py-4 text-center text-xs text-(--color-oa-ink-dim)">
              No rules. The scanner will fall back to the system registry's defaults.
            </p>
          }
        >
          {(r) => (
            <div class="flex items-center gap-2 rounded-md border border-white/5 bg-black/20 px-2 py-1.5">
              <input
                type="text"
                value={r.pattern}
                onInput={(e) => updateRule(r.uiKey, { pattern: e.currentTarget.value })}
                class={INPUT}
                placeholder="*.pce"
              />
              <span class="shrink-0 text-(--color-oa-ink-dim)">→</span>
              <select
                value={r.systemId}
                onChange={(e) =>
                  updateRule(r.uiKey, { systemId: e.currentTarget.value as SystemId })
                }
                class={SELECT}
              >
                <For each={Object.keys(systemThemes) as SystemId[]}>
                  {(sysId) => (
                    <option value={sysId}>{systemThemes[sysId].displayName}</option>
                  )}
                </For>
              </select>
              <button
                type="button"
                class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                onClick={() => removeRule(r.uiKey)}
                title="Remove rule"
              >
                ✕
              </button>
            </div>
          )}
        </For>
      </div>
      <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
        Archives (.zip / .7z) are auto-detected and unpacked at scan time — no rule needed.
      </p>
    </div>
  );

  const Step3 = () => {
    const tally = () => bucketTally();
    const unmatched = () => unmatchedTally();
    const total = () => scanRows().length;
    return (
      <div class="flex flex-col gap-4">
        <Show
          when={!scanError()}
          fallback={
            <div class="rounded-md border border-red-500/40 bg-red-500/10 px-3 py-3 text-xs text-(--color-oa-ink)">
              Scan failed: {scanError()}
            </div>
          }
        >
          <Show when={scanRunning() || scanProgress()}>
            <div class="rounded-md border border-white/5 bg-black/30 px-3 py-3">
              <div class="flex items-baseline justify-between text-xs">
                <p class="text-(--color-oa-ink)">
                  {scanRunning() ? "Scanning…" : "Scan complete"}
                </p>
                <p class="text-(--color-oa-ink-dim)">
                  {scanProgress()?.filesSeen ?? 0} files scanned
                </p>
              </div>
              <div class="mt-2 h-1.5 overflow-hidden rounded-full bg-white/5">
                <div
                  class="h-full bg-(--color-system-accent) transition-all"
                  style={{
                    width: scanRunning()
                      ? `${Math.min(95, 10 + ((scanProgress()?.filesSeen ?? 0) % 100))}%`
                      : "100%",
                  }}
                />
              </div>
              <Show when={scanRunning() && scanProgress()?.currentFile}>
                <p class="mt-2 truncate font-mono text-[0.65rem] text-(--color-oa-ink-dim)">
                  {scanProgress()!.currentFile}
                </p>
              </Show>
            </div>
          </Show>
          <Show when={scanCancelled()}>
            <p class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-2 text-xs text-(--color-oa-ink-dim)">
              Scan cancelled.
            </p>
          </Show>
          <Show when={!scanRunning() && total() > 0}>
            <div class="flex flex-col gap-2">
              <p class="text-xs uppercase tracking-widest text-(--color-oa-ink-dim)">
                Found {total()} games
              </p>
              <div class="flex flex-col gap-1">
                <For each={tally()}>
                  {(t) => (
                    <div class="flex items-center justify-between rounded-md border border-white/5 bg-black/20 px-3 py-1.5 text-xs">
                      <span class="text-(--color-oa-ink)">
                        {systemThemes[t.systemId]?.displayName ?? t.systemId}
                      </span>
                      <span class="text-(--color-oa-ink-dim)">
                        {t.count} {t.count === 1 ? "game" : "games"}
                        {t.archived > 0 && ` · ${t.archived} in archives`}
                      </span>
                    </div>
                  )}
                </For>
                <Show when={unmatched().length > 0}>
                  <div class="rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs text-(--color-oa-ink)">
                    <p class="font-semibold">
                      {unmatched().reduce((s, u) => s + u.count, 0)} unmatched files
                    </p>
                    <p class="mt-0.5 text-(--color-oa-ink-dim)">
                      {unmatched().map((u) => `${u.extension} (${u.count})`).join(", ")} —
                      add a rule on the Mapping step to assign.
                    </p>
                  </div>
                </Show>
              </div>
            </div>
          </Show>
          <Show when={!scanRunning() && total() === 0 && !scanError() && !scanCancelled() && scanProgress()}>
            <p class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-2 text-xs text-(--color-oa-ink-dim)">
              No supported ROMs found in this folder.
            </p>
          </Show>
        </Show>
      </div>
    );
  };

  const Step4 = () => {
    const total = () => bucketScanned().entries.length;
    return (
      <div class="flex flex-col gap-4">
        <p class="text-sm text-(--color-oa-ink)">
          Add <span class="font-semibold text-(--color-system-accent)">{total()}</span>{" "}
          {total() === 1 ? "game" : "games"} to your library?
        </p>
        <div class="flex flex-col gap-2 rounded-md border border-white/5 bg-black/20 px-3 py-2.5">
          <p class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            Run media + metadata sync after import?
          </p>
          <Toggle label="Cover art" checked={syncCovers()} onChange={setSyncCovers} />
          <Toggle label="Snapshots" checked={syncSnaps()} onChange={setSyncSnaps} />
          <Toggle label="Title screens" checked={syncTitles()} onChange={setSyncTitles} />
          <Toggle
            label="Year / genre / developer / publisher / players"
            checked={syncMetadata()}
            onChange={setSyncMetadata}
          />
        </div>
        <Show when={commitError()}>
          <div class="rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-xs text-(--color-oa-ink)">
            Commit failed: {commitError()}
          </div>
        </Show>
      </div>
    );
  };

  // Controller-nav coverage. One DOM-query focus group spans every
  // button inside the wizard panel; MutationObserver re-binds across
  // step changes. B closes the wizard via the back-stack so the
  // gamepad path mirrors Esc.
  let wizardPanelRef: HTMLDivElement | undefined;
  useDomQueryFocusGroup({
    id: "import-wizard",
    containerRef: () => wizardPanelRef,
    orientation: "vertical",
    onActivate: (_i, el) => el.click(),
    onCancel: () => {
      if (!scanRunning() && !committing()) props.onClose();
    },
  });

  return (
    <Show when={props.open}>
      {(() => {
        // Capture-restore + back-stack live inside the Show body so they
        // mount/unmount with the modal — same pattern as other menus.
        useBackHandler(() => {
          if (!scanRunning() && !committing()) props.onClose();
        });
        const restoreFocus = captureFocusReturn();
        onCleanup(restoreFocus);
        return null;
      })()}
      <div
        class="fixed inset-0 z-50 grid place-items-center bg-black/60 backdrop-blur-sm"
        onClick={(e) => {
          if (e.currentTarget === e.target && !scanRunning() && !committing()) {
            props.onClose();
          }
        }}
      >
        <div
          ref={(el) => (wizardPanelRef = el)}
          class="flex w-full max-w-3xl flex-col overflow-hidden rounded-lg border border-white/10 bg-(--color-oa-bg-deep) shadow-2xl shadow-black/60"
          style={{ height: "min(640px, 85vh)" }}
          role="dialog"
          aria-modal="true"
          aria-labelledby="import-wizard-title"
        >
          <header class="flex items-center justify-between border-b border-white/5 px-6 py-4">
            <div>
              <h2
                id="import-wizard-title"
                class="text-sm font-semibold uppercase tracking-[0.3em] text-(--color-oa-ink)"
              >
                Import games
              </h2>
              <p class="mt-0.5 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                Step {step()} of 4 · {STEP_LABELS[step()]}
              </p>
            </div>
            <div class="flex items-center gap-3">
              <ol class="flex items-center gap-1.5 text-[0.6rem] uppercase tracking-widest">
                <For each={[1, 2, 3, 4] as Step[]}>
                  {(n) => (
                    <li
                      class="rounded-full px-2 py-0.5"
                      classList={{
                        "bg-(--color-system-accent) text-black/90": step() === n,
                        "bg-white/[0.06] text-(--color-oa-ink-dim)": step() !== n,
                      }}
                    >
                      {n}
                    </li>
                  )}
                </For>
              </ol>
              <button
                type="button"
                onClick={(e) => {
                  e.currentTarget.blur();
                  if (!scanRunning() && !committing()) props.onClose();
                }}
                class={BTN}
                disabled={scanRunning() || committing()}
              >
                Close
              </button>
            </div>
          </header>

          <div class="flex min-h-0 flex-1 flex-col overflow-y-auto px-6 py-5">
            <Show when={step() === 1}>{Step1()}</Show>
            <Show when={step() === 2}>{Step2()}</Show>
            <Show when={step() === 3}>{Step3()}</Show>
            <Show when={step() === 4}>{Step4()}</Show>
          </div>

          <footer class="flex items-center justify-between border-t border-white/5 px-6 py-3">
            <div class="flex gap-2">
              <Show when={step() > 1}>
                <button
                  type="button"
                  class={BTN}
                  disabled={scanRunning() || committing()}
                  onClick={() => setStep((s) => (s - 1) as Step)}
                >
                  ‹ Back
                </button>
              </Show>
            </div>
            <div class="flex gap-2">
              <Show when={step() === 1}>
                <button
                  type="button"
                  class={BTN_PRIMARY}
                  disabled={
                    folder() === null
                    || folderEmpty() === true
                    || folderCheckError() !== null
                  }
                  onClick={() => setStep(2)}
                >
                  Next ›
                </button>
              </Show>
              <Show when={step() === 2}>
                <button type="button" class={BTN_PRIMARY} onClick={() => setStep(3)}>
                  Next ›
                </button>
              </Show>
              <Show when={step() === 3}>
                <Show when={scanRunning()}>
                  <button type="button" class={BTN} onClick={cancelScan}>
                    Cancel scan
                  </button>
                </Show>
                <Show when={!scanRunning()}>
                  <button
                    type="button"
                    class={BTN}
                    onClick={() => {
                      setScanRows([]);
                      setScanProgress(null);
                      setScanError(null);
                      setScanCancelled(false);
                      void startScan();
                    }}
                  >
                    Rescan
                  </button>
                  <button
                    type="button"
                    class={BTN_PRIMARY}
                    disabled={scanError() !== null}
                    onClick={() => setStep(4)}
                  >
                    Next ›
                  </button>
                </Show>
              </Show>
              <Show when={step() === 4}>
                <button
                  type="button"
                  class={BTN}
                  disabled={committing()}
                  onClick={() => void commit(false)}
                >
                  Skip sync
                </button>
                <button
                  type="button"
                  class={BTN_PRIMARY}
                  disabled={committing()}
                  onClick={() => void commit(true)}
                >
                  {committing() ? "Importing…" : "Import + sync"}
                </button>
              </Show>
            </div>
          </footer>
        </div>
      </div>
      {/* ScummVM auto-detect — mounted at wizard root so it can layer
          on top of the wizard's own backdrop. Scoped to the currently-
          picked folder. */}
      <ScummvmDetectDialog
        open={scummvmDetectOpen()}
        onClose={() => setScummvmDetectOpen(false)}
        initialFolder={folder()}
      />
    </Show>
  );
};

const Toggle: Component<{
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
  hint?: string;
}> = (props) => (
  <label
    class="flex cursor-pointer items-start gap-2 text-xs text-(--color-oa-ink)"
    classList={{ "opacity-50 cursor-not-allowed": props.disabled === true }}
  >
    <input
      type="checkbox"
      class="mt-0.5 accent-(--color-system-accent)"
      checked={props.checked}
      disabled={props.disabled === true}
      onChange={(e) => props.onChange(e.currentTarget.checked)}
    />
    <span class="flex flex-col gap-0.5">
      <span>{props.label}</span>
      <Show when={props.hint}>
        <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          {props.hint}
        </span>
      </Show>
    </span>
  </label>
);

export default ImportWizard;
