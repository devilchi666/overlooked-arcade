import {
  createEffect,
  createMemo,
  createResource,
  createSignal,
  For,
  onCleanup,
  Show,
  type Accessor,
  type Component,
  type JSX,
} from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { systemThemes, type SystemId } from "../../themes/registry";
import MissingCoreBulkPrompt from "./MissingCoreBulkPrompt";

// Phase 1B Slice 3 — Per-system readiness checklist.
//
// Mounts in two surfaces with the same shape:
//   1) New wizard Step 3 — sourced from commitRowsToEntries()
//      systemIds, surfaced between the per-ROM table (Step 2) and
//      Confirm (Step 4).
//   2) Settings → Library → "System readiness" card — sourced from
//      the operator's shipped library (library.state.entries
//      grouped by systemId).
//
// Five pills per system row:
//   - Core installed     (real status via list_cores)
//   - BIOS present       (real status via get_bios_status)
//   - Default bindings   (always ✓ — every onboarded system has them)
//   - Core options       (placeholder — coming Slice 4)
//   - Per-game overrides (placeholder — coming Slice 4)
//
// Resolution actions:
//   - ⚠ BIOS → "Open BIOS folder" (working, calls open_bios_folder).
//   - ⚠ Core → "Install core…" stub (toast pointing at Slice 4).

type CoreEntry = {
  fileName: string;
  libraryName: string;
  libraryVersion: string;
  validExtensions: string;
};

/// Subset of the shape `core_installer::available_cores` returns. Same
/// shape `MissingCoreBulkPrompt.tsx` consumes — using identical types
/// here keeps the readiness checklist and the modal aligned on the
/// "which catalog entries exist for which system + are they installed"
/// signal, which is the source-of-truth fix for the 2026-06-01 bug
/// where the banner counted "1 missing core" but the modal showed
/// "no missing cores" (the banner's extension-overlap heuristic
/// diverged from the modal's catalog-membership filter for systems
/// without CATALOG entries — jagcd / sega32xcd / stv).
type AvailableCore = {
  base: string;
  fileName: string;
  systems: string[];
  installed: boolean;
  recommended: boolean;
  supportedOnHost: boolean;
};

type BiosEntryStatus = "ok" | "unknownHash" | "missing" | "error";

type BiosStatusEntry = {
  slug: string;
  label: string;
  required: string;
  status: BiosEntryStatus;
  detail: string;
};

type BiosStatusResponse = {
  systemDir: string;
  entries: BiosStatusEntry[];
};

export type SystemReadinessChecklistProps = {
  /// Systems to render. Wizard surface = unique systemIds from
  /// commitRowsToEntries(); Settings surface = unique systemIds
  /// from library.state.entries.
  systems: Accessor<SystemId[]>;
  /// Override the empty-state copy. Wizard says "No systems in
  /// this scan"; Settings says "Library is empty — import a folder
  /// to get started."
  emptyStateLabel?: string;
};

type PillState = "ready" | "warning" | "na" | "coming";

const PILL_STYLES: Record<PillState, { ring: string; bg: string; text: string }> = {
  ready: {
    ring: "border-emerald-400/40",
    bg: "bg-emerald-500/10",
    text: "text-emerald-300",
  },
  warning: {
    ring: "border-amber-400/40",
    bg: "bg-amber-500/10",
    text: "text-amber-300",
  },
  na: {
    ring: "border-white/15",
    bg: "bg-white/[0.04]",
    text: "text-(--color-oa-ink-dim)",
  },
  coming: {
    ring: "border-white/10",
    bg: "bg-white/[0.02]",
    text: "text-(--color-oa-ink-dim)",
  },
};

function Pill(props: {
  state: PillState;
  label: string;
  detail?: string;
}): JSX.Element {
  const s = PILL_STYLES[props.state];
  return (
    <div class={`flex flex-col gap-0.5 rounded border ${s.ring} ${s.bg} px-2 py-1`}>
      <span class={`text-[0.6rem] font-semibold uppercase tracking-widest ${s.text}`}>
        {props.label}
      </span>
      <Show when={props.detail}>
        <span class="text-[0.6rem] text-(--color-oa-ink-dim)" title={props.detail}>
          {props.detail!.length > 40 ? `${props.detail!.slice(0, 38)}…` : props.detail}
        </span>
      </Show>
    </div>
  );
}

/// Extension-overlap helper. Returns true if any installed core's
/// `validExtensions` covers at least one of the system's registered
/// extensions. Used as the FALLBACK signal when the system isn't in
/// the curated CATALOG (`available_cores`) — covers cases like jagcd
/// where the operator has the Jaguar core installed and it handles
/// `.cue` too, even though jagcd has no dedicated catalog entry.
function extensionOverlapInstalled(systemId: SystemId, cores: CoreEntry[] | undefined): boolean {
  if (!cores) return false;
  const sysExts = new Set(
    systemThemes[systemId]?.extensions.map((e) => e.toLowerCase()) ?? [],
  );
  if (sysExts.size === 0) return false;
  for (const c of cores) {
    for (const raw of (c.validExtensions ?? "").split("|")) {
      const ext = raw.trim().toLowerCase().replace(/^\./, "");
      if (ext && sysExts.has(ext)) return true;
    }
  }
  return false;
}

/// True when at least one CATALOG entry targeting this system is
/// currently installed. Primary signal for the Core pill — same
/// source of truth the bulk-install modal filters against, so the
/// banner count and the modal contents agree.
function catalogCoreInstalled(systemId: SystemId, available: AvailableCore[] | undefined): boolean {
  if (!available) return false;
  return available.some((c) => c.systems.includes(systemId) && c.installed);
}

/// True when CATALOG has at least one entry targeting this system —
/// regardless of installed-state. Used to decide whether the
/// "Install core…" affordances should appear; for systems with no
/// catalog entries (jagcd / sega32xcd / stv right now, or anything
/// added to the registry before its CATALOG row lands), the modal
/// has nothing to offer and the operator must install manually.
function catalogHasEntry(systemId: SystemId, available: AvailableCore[] | undefined): boolean {
  if (!available) return false;
  return available.some((c) => c.systems.includes(systemId));
}

/// Hybrid "is the operator able to launch this system?" check.
/// Catalog-membership first (the authoritative signal for systems we
/// curate), extension-overlap fallback for systems missing from
/// the catalog. Together these align with the modal's filter — if
/// this function returns true, the modal would have no row to offer;
/// if false, the modal has a row to offer when `catalogHasEntry`
/// also returns true.
function coreInstalledFor(
  systemId: SystemId,
  available: AvailableCore[] | undefined,
  cores: CoreEntry[] | undefined,
): boolean {
  return catalogCoreInstalled(systemId, available) || extensionOverlapInstalled(systemId, cores);
}

const SystemReadinessChecklist: Component<SystemReadinessChecklistProps> = (props) => {
  // Two installed-cores signals, both refetched when a download finishes:
  //   - `cores` (list_cores)         — installed cores' validExtensions, for
  //                                    extension-overlap fallback on systems
  //                                    not in CATALOG.
  //   - `available` (available_cores) — CATALOG entries with per-entry
  //                                     installed-state. Source of truth that
  //                                     the bulk-install modal also uses.
  const [cores, { refetch: refetchCores }] = createResource(async () => {
    try {
      return await invoke<CoreEntry[]>("list_cores");
    } catch (e) {
      console.warn("[oa-readiness] list_cores failed:", e);
      return [] as CoreEntry[];
    }
  });

  const [available, { refetch: refetchAvailable }] = createResource(async () => {
    try {
      return await invoke<AvailableCore[]>("available_cores");
    } catch (e) {
      console.warn("[oa-readiness] available_cores failed:", e);
      return [] as AvailableCore[];
    }
  });

  const [bios] = createResource(async () => {
    try {
      return await invoke<BiosStatusResponse>("get_bios_status");
    } catch (e) {
      console.warn("[oa-readiness] get_bios_status failed:", e);
      return { systemDir: "", entries: [] as BiosStatusEntry[] };
    }
  });

  // Pre-compute the BIOS lookup-by-slug for fast per-row queries.
  const biosBySlug = createMemo(() => {
    const m = new Map<string, BiosStatusEntry>();
    for (const e of bios()?.entries ?? []) {
      m.set(e.slug, e);
    }
    return m;
  });

  // Phase 1B Slice 4: per-system Core options schema status. Map of
  // SystemId → boolean (true when the schema file exists + has at least
  // one option definition). Fetched in one batch when props.systems()
  // changes; later re-fetched if a download completes (cores resource
  // refetch). The schema only populates after the operator launches a
  // game of the system once — so newly-imported systems usually report
  // false until first play.
  const [optionsBySystem, setOptionsBySystem] = createSignal<Map<SystemId, boolean>>(new Map());
  createEffect(() => {
    const systems = props.systems();
    if (systems.length === 0) {
      setOptionsBySystem(new Map());
      return;
    }
    void (async () => {
      const next = new Map<SystemId, boolean>();
      // Parallel — the Tauri command is a cheap fs read.
      const results = await Promise.allSettled(
        systems.map((sid) =>
          invoke<boolean>("has_core_options_schema", { systemId: sid }).catch(() => false),
        ),
      );
      results.forEach((r, i) => {
        const sid = systems[i];
        if (r.status === "fulfilled") next.set(sid, r.value);
        else next.set(sid, false);
      });
      setOptionsBySystem(next);
    })();
  });

  // Phase 1B Slice 4: refetch installed cores when a download
  // completes so the matching row's Core pill flips ⚠ → ✓ without
  // the operator closing + reopening the wizard / Settings card.
  // Both resources refetch — the catalog-aware `available` drives
  // the primary pill state, the `cores` extension-overlap drives
  // the no-catalog-entry fallback.
  let unlistenDownloadProgress: UnlistenFn | undefined;
  void (async () => {
    try {
      unlistenDownloadProgress = await listen<{ fileName: string; phase: string }>(
        "oa://core-download-progress",
        (event) => {
          if (event.payload.phase === "done") {
            void refetchCores();
            void refetchAvailable();
          }
        },
      );
    } catch (e) {
      console.warn("[oa-readiness] listen oa://core-download-progress failed:", e);
    }
  })();
  onCleanup(() => {
    if (unlistenDownloadProgress) unlistenDownloadProgress();
  });

  // Phase 1B Slice 4: bulk-prompt modal state. null = closed. Array =
  // systems to offer cores for. Both the per-row "Install core…"
  // button and the top-of-list "Install N missing cores…" button
  // funnel through this signal.
  const [bulkPromptSystems, setBulkPromptSystems] = createSignal<SystemId[] | null>(null);

  const missingCoreSystems = createMemo<SystemId[]>(() => {
    if (cores.loading || available.loading) return [];
    // Two filters: (1) we don't already have a core for it, (2) the
    // catalog has SOMETHING to offer. If catalog has no entry, the
    // modal can't surface anything to install — banner skips it too
    // so the count + the modal contents stay in lockstep.
    return props.systems().filter(
      (s) => !coreInstalledFor(s, available(), cores()) && catalogHasEntry(s, available()),
    );
  });

  async function openBiosFolder() {
    try {
      await invoke("open_bios_folder");
    } catch (e) {
      console.warn("[oa-readiness] open_bios_folder failed:", e);
    }
  }

  function renderRow(systemId: SystemId): JSX.Element {
    const theme = systemThemes[systemId];
    const displayName = theme?.displayName ?? systemId;

    // Core — three states, not two:
    //   ✓ Installed: any catalog entry or extension-overlap core for the system
    //   ⚠ No core: catalog has installable entries but none installed
    //   ↪ Manual install: no catalog entry exists for this system (e.g.
    //                     jagcd / sega32xcd / stv as of 2026-06-01 —
    //                     systems wired in the registry that don't yet
    //                     have curated catalog rows)
    const hasCore = () => coreInstalledFor(systemId, available(), cores());
    const hasCatalogRow = () => catalogHasEntry(systemId, available());
    const corePillState = (): PillState => {
      if (hasCore()) return "ready";
      if (hasCatalogRow()) return "warning";
      return "na";
    };
    const corePillLabel = (): string => {
      if (hasCore()) return "✓ Core installed";
      if (hasCatalogRow()) return "⚠ No core";
      return "↪ No catalog core";
    };
    const corePillDetail = (): string | undefined => {
      if (hasCore()) return undefined;
      if (hasCatalogRow()) return "Drop a .dll into /cores/";
      return "Install manually via Settings → Cores";
    };
    const corePill = <Pill state={corePillState()} label={corePillLabel()} detail={corePillDetail()} />;

    // BIOS
    const biosEntry = () => biosBySlug().get(systemId);
    const biosState = (): PillState => {
      const e = biosEntry();
      if (!e) return "na"; // system not in BIOS-requiring list → ↪ not required
      if (e.status === "ok") return "ready";
      return "warning";
    };
    const biosLabel = (): string => {
      const e = biosEntry();
      if (!e) return "↪ BIOS not required";
      if (e.status === "ok") return "✓ BIOS ready";
      if (e.status === "missing") return "⚠ BIOS missing";
      if (e.status === "unknownHash") return "⚠ Unknown hash";
      return "⚠ Read error";
    };
    const biosDetail = (): string | undefined => {
      const e = biosEntry();
      if (!e || e.status === "ok") return undefined;
      return e.detail || e.required;
    };
    const biosPill = <Pill state={biosState()} label={biosLabel()} detail={biosDetail()} />;

    // Bindings — always ✓ for any registered system.
    const bindingsPill = <Pill state="ready" label="✓ Bindings ready" />;

    // Core options — real status as of Slice 4. The schema lands lazily
    // when the operator first launches a game of this system. ↪ means
    // "no per-system tuning has happened yet" rather than an error.
    const hasOptions = () => optionsBySystem().get(systemId) === true;
    const optionsPill = (
      <Pill
        state={hasOptions() ? "ready" : "na"}
        label={hasOptions() ? "✓ Core options" : "↪ Core options"}
        detail={hasOptions() ? undefined : "Schema populates on first game launch"}
      />
    );

    // KNOWN_GAME_BUGS overrides — placeholder. Coverage is sparse today
    // (psx + nds only); the real status pill lands inside the broader
    // KNOWN_GAME_BUGS.md → games-info.md migration arc.
    const overridesPill = <Pill state="coming" label="— Per-game overrides" detail="Coming with games-info migration" />;

    return (
      <div
        data-system={systemId}
        class="flex flex-col gap-2 rounded-md border border-white/5 bg-white/[0.02] px-3 py-2.5 hover:border-white/15 hover:bg-white/[0.04]"
      >
        <div class="flex items-baseline justify-between">
          <h4 class="text-xs font-semibold text-(--color-oa-ink)">
            <span class="text-(--color-system-accent)">{theme?.shortName ?? systemId}</span>
            <span class="ml-2 text-(--color-oa-ink-dim)">{displayName}</span>
          </h4>
        </div>
        <div class="grid grid-cols-[repeat(auto-fit,minmax(160px,1fr))] gap-1.5">
          {corePill}
          {biosPill}
          {bindingsPill}
          {optionsPill}
          {overridesPill}
        </div>
        <Show when={(!hasCore() && hasCatalogRow()) || (biosEntry() && biosEntry()!.status !== "ok")}>
          <div class="mt-1 flex items-center gap-2 text-[0.7rem]">
            <Show when={!hasCore() && hasCatalogRow()}>
              <button
                type="button"
                class="rounded border border-amber-400/40 bg-amber-500/10 px-2 py-1 text-[0.65rem] uppercase tracking-widest text-amber-300 hover:brightness-110"
                onClick={() => setBulkPromptSystems([systemId])}
              >
                Install core…
              </button>
            </Show>
            <Show when={biosEntry() && biosEntry()!.status !== "ok"}>
              <button
                type="button"
                class="rounded border border-amber-400/40 bg-amber-500/10 px-2 py-1 text-[0.65rem] uppercase tracking-widest text-amber-300 hover:brightness-110"
                onClick={openBiosFolder}
              >
                Open BIOS folder
              </button>
            </Show>
          </div>
        </Show>
      </div>
    );
  }

  return (
    <div class="flex flex-col gap-3">
      <Show when={cores.loading || bios.loading || available.loading}>
        <p class="rounded-md border border-white/5 bg-black/20 px-3 py-2 text-xs text-(--color-oa-ink-dim)">
          Loading readiness…
        </p>
      </Show>
      <Show
        when={!cores.loading && !bios.loading && !available.loading && props.systems().length === 0}
      >
        <p class="rounded-md border border-white/5 bg-black/20 px-3 py-2 text-xs text-(--color-oa-ink-dim)">
          {props.emptyStateLabel ?? "No systems in this scan."}
        </p>
      </Show>
      <Show when={!cores.loading && !bios.loading && !available.loading && props.systems().length > 0}>
        {/* Phase 1B Slice 4: top-level bulk-install banner. Hidden
            when every system already has a core. Click opens the
            modal scoped to ALL missing systems. */}
        <Show when={missingCoreSystems().length > 0}>
          <div class="flex items-center justify-between rounded-md border border-amber-400/40 bg-amber-500/10 px-3 py-2">
            <div>
              <p class="text-xs font-semibold text-amber-300">
                {missingCoreSystems().length}{" "}
                {missingCoreSystems().length === 1 ? "system needs" : "systems need"} a core
              </p>
              <p class="mt-0.5 text-[0.65rem] text-(--color-oa-ink-dim)">
                Download recommended cores from libretro buildbot — one click per system, or pick a different core per row.
              </p>
            </div>
            <button
              type="button"
              class="shrink-0 rounded-md border border-amber-400/40 bg-amber-500/20 px-3 py-1.5 text-xs uppercase tracking-wider text-amber-200 hover:bg-amber-500/30"
              onClick={() => setBulkPromptSystems(missingCoreSystems())}
            >
              Install {missingCoreSystems().length} missing{" "}
              {missingCoreSystems().length === 1 ? "core" : "cores"}…
            </button>
          </div>
        </Show>
        <div class="flex flex-col gap-2">
          <For each={props.systems()}>{(sid) => renderRow(sid)}</For>
        </div>
        <Show when={(bios()?.systemDir ?? "").length > 0}>
          <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            BIOS folder: <code class="font-mono text-(--color-oa-ink-dim)">{bios()?.systemDir}</code>
          </p>
        </Show>
      </Show>
      {/* Phase 1B Slice 4: bulk-install modal. Open state lives on
          this component because both the per-row "Install core…" button
          and the top-of-list banner-button funnel through the same
          signal. Modal calls onCoreInstalled once per successful
          download; the event listener above also fires refetchCores
          on phase=done so we double-cover the refresh path. */}
      <MissingCoreBulkPrompt
        open={bulkPromptSystems() !== null}
        onClose={() => setBulkPromptSystems(null)}
        systems={() => bulkPromptSystems() ?? []}
        onCoreInstalled={() => void refetchCores()}
      />
    </div>
  );
};

export default SystemReadinessChecklist;
