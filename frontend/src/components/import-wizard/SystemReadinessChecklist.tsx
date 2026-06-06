import {
  createEffect,
  createMemo,
  createResource,
  createSignal,
  For,
  onCleanup,
  onMount,
  Show,
  type Accessor,
  type Component,
  type JSX,
} from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listenScoped } from "../../lib/eventListener";
import { systemThemes, type SystemId } from "../../themes/registry";
import MissingCoreBulkPrompt from "./MissingCoreBulkPrompt";
import BiosResolutionDetail, { type BiosFile } from "./BiosResolutionDetail";

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

/// Guided Setup Phase 2 — recommendation returned by the
/// `recommended_core_for_system` Tauri command. Mirror of the Rust
/// `CoreRecommendation` struct (camelCase via serde).
type CoreRecommendation = {
  /// Host-specific filename (e.g. `mednafen_psx_hw_libretro.dll` on
  /// Windows). Directly comparable against `cores` resource entries.
  core: string;
  /// The effective CPU tier the recommendation was made against.
  tier: "high" | "mid" | "low";
  /// Where the tier came from — heuristic-detected / cached / operator
  /// override. Detail copy uses this to render "(override-set tier)"
  /// when the operator manually picked one.
  tierSource: "detected" | "cached" | "override";
  /// Whether the core came from the per-system tier table or the
  /// registry default. Drives the "(high-tier pick)" vs
  /// "(system default)" suffix.
  cameFrom: "tierTable" | "systemDefault";
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
  /// Phase 1B Slice 5 — per-file inventory the readiness checklist
  /// renders inline below the BIOS pill via BiosResolutionDetail.
  /// Always present in the Slice 5+ response shape; pre-Slice-5
  /// installs serialize an empty array via Vec::default.
  files: BiosFile[];
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

  const [bios, { refetch: refetchBios }] = createResource(async () => {
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

  // Guided Setup Phase 2 — per-system core recommendation from the
  // CPU-tier heuristic. Map of SystemId → CoreRecommendation. Drives
  // the Core pill detail copy: when the recommended core is what the
  // operator has installed, the detail confirms "Using {core} (high-
  // tier pick)"; when it's missing, "Install {core} ({tier}-tier
  // pick)" replaces the generic "drop a .dll in /cores/" hint.
  //
  // Fetched on systems change. Doesn't refetch on download completion
  // since the recommendation only depends on (system, tier) — not on
  // what's installed.
  const [coreRecsBySystem, setCoreRecsBySystem] = createSignal<
    Map<SystemId, CoreRecommendation>
  >(new Map());
  createEffect(() => {
    const systems = props.systems();
    if (systems.length === 0) {
      setCoreRecsBySystem(new Map());
      return;
    }
    void (async () => {
      const next = new Map<SystemId, CoreRecommendation>();
      const results = await Promise.allSettled(
        systems.map((sid) =>
          invoke<CoreRecommendation>("recommended_core_for_system", { systemId: sid }),
        ),
      );
      results.forEach((r, i) => {
        if (r.status === "fulfilled") next.set(systems[i], r.value);
      });
      setCoreRecsBySystem(next);
    })();
  });

  // Phase 1B Slice 4: refetch installed cores when a download
  // completes so the matching row's Core pill flips ⚠ → ✓ without
  // the operator closing + reopening the wizard / Settings card.
  // Both resources refetch — the catalog-aware `available` drives
  // the primary pill state, the `cores` extension-overlap drives
  // the no-catalog-entry fallback.
  listenScoped<{ fileName: string; phase: string }>(
    "oa://core-download-progress",
    (event) => {
      if (event.payload.phase === "done") {
        void refetchCores();
        void refetchAvailable();
      }
    },
  );

  /// Manual refetch of every readiness signal. Called by the
  /// "Refresh" button + the window-focus handler below so operator
  /// drops into <exe_dir>/system/ or /cores/ via the OS file
  /// manager get picked up without a wizard close-and-reopen.
  function refetchReadiness() {
    void refetchCores();
    void refetchAvailable();
    void refetchBios();
    // Re-derive the per-system Core options map too — operator may
    // have launched a game between focus loss and regain, which would
    // populate a new system's schema.
    const systems = props.systems();
    if (systems.length > 0) {
      void (async () => {
        const next = new Map<SystemId, boolean>();
        const results = await Promise.allSettled(
          systems.map((sid) =>
            invoke<boolean>("has_core_options_schema", { systemId: sid }).catch(() => false),
          ),
        );
        results.forEach((r, i) => {
          const sid = systems[i];
          next.set(sid, r.status === "fulfilled" ? r.value : false);
        });
        setOptionsBySystem(next);
      })();
    }
  }

  /// Refetch on OS-window focus. Operator clicks "Open BIOS folder"
  /// (or alt-tabs out to drop a core .dll into /cores/), drops files,
  /// switches back to OA — the readiness pills should reflect what's
  /// now on disk without a manual refresh click. Tauri DOM focus
  /// events are reliable per the saved memory; per-event refetch is
  /// cheap (a few hundred ms of filesystem scans worst-case).
  onMount(() => {
    const onWindowFocus = () => refetchReadiness();
    window.addEventListener("focus", onWindowFocus);
    onCleanup(() => window.removeEventListener("focus", onWindowFocus));
  });

  // Phase 1B Slice 4: bulk-prompt modal state. null = closed. Array =
  // systems to offer cores for. Both the per-row "Install core…"
  // button and the top-of-list "Install N missing cores…" button
  // funnel through this signal.
  const [bulkPromptSystems, setBulkPromptSystems] = createSignal<SystemId[] | null>(null);

  /// Alphabetical-by-displayName ordering of the systems to render.
  /// Matches the operator's expectation (2026-06-03): "I want the
  /// list in alphabetical order." Position is then predictable
  /// regardless of which order the wizard / library emits them in.
  const sortedSystems = createMemo<SystemId[]>(() => {
    const ids = [...props.systems()];
    ids.sort((a, b) => {
      const da = systemThemes[a]?.displayName ?? a;
      const db = systemThemes[b]?.displayName ?? b;
      return da.localeCompare(db);
    });
    return ids;
  });

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
    const recommendation = (): CoreRecommendation | undefined =>
      coreRecsBySystem().get(systemId);
    /// Format the tier-pick suffix for the Core pill detail string.
    /// `(high-tier pick)` when the core came from the tier table,
    /// `(system default)` when no tier-aware option exists. When the
    /// operator's override is in play, prepend "override-set ".
    const tierSuffix = (rec: CoreRecommendation): string => {
      if (rec.cameFrom === "systemDefault") return "(system default)";
      const prefix = rec.tierSource === "override" ? "override-set " : "";
      return `(${prefix}${rec.tier}-tier pick)`;
    };
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
      const rec = recommendation();
      if (hasCore()) {
        // Confirm the curated pick. Render even on ✓ rows so the
        // operator sees the heuristic's reasoning. Skip when no
        // recommendation has resolved (transient resource-loading
        // state).
        if (rec) return `Using ${rec.core} ${tierSuffix(rec)}`;
        return undefined;
      }
      if (hasCatalogRow()) {
        if (rec) return `Install ${rec.core} ${tierSuffix(rec)}`;
        return "Drop a .dll into /cores/ when you've got one";
      }
      return "Drop a .dll in manually via Settings → Cores";
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
        detail={hasOptions() ? undefined : "Fills in the first time you launch a game of this system"}
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
        {/* Phase 1B Slice 5: inline per-file BIOS detail. Auto-expands
            when the BIOS pill is ⚠ (missing / unknown-hash / error) —
            mirrors Slice 3's "action buttons only show on ⚠" pattern.
            Ready / not-required rows stay compact. */}
        <Show
          when={
            biosEntry() &&
            biosEntry()!.status !== "ok" &&
            (biosEntry()!.files?.length ?? 0) > 0
          }
        >
          <BiosResolutionDetail
            systemId={systemId}
            files={biosEntry()!.files}
            systemDir={bios()?.systemDir ?? ""}
            onInstalled={() => void refetchBios()}
          />
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
                {missingCoreSystems().length === 1
                  ? "One system needs a core to launch"
                  : `${missingCoreSystems().length} systems need a core to launch`}
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
          <For each={sortedSystems()}>{(sid) => renderRow(sid)}</For>
        </div>
        <div class="flex items-center justify-between gap-2">
          <Show when={(bios()?.systemDir ?? "").length > 0}>
            <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              BIOS folder: <code class="font-mono text-(--color-oa-ink-dim)">{bios()?.systemDir}</code>
            </p>
          </Show>
          <button
            type="button"
            class="shrink-0 rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
            onClick={refetchReadiness}
            title="Re-scan /cores/ and /system/ for newly-dropped files"
          >
            Refresh
          </button>
        </div>
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
