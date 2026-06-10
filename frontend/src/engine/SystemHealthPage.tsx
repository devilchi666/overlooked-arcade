// System Health — consolidated diagnostic hub for OA's operational state.
//
// Replaces four previously-standalone SETTINGS categories (BIOS, Cores,
// Storage, Background Jobs) with one category whose body carries an
// internal tab strip. Plan: docs/PLANS/settings-declutter-system-health.md.
//
// Phase 1 shipped the scaffolding: tab strip + four absorbed bodies +
// an Overview placeholder + the lifted readiness checklist.
// Phase 2 (this file) fills Overview with the 5 status rollup cards:
// Cores / BIOS / Readiness / Background jobs / Storage. Each fetches its
// own counts via existing Tauri commands, derives a green/amber/red pill
// state, and a CTA that deep-links to the relevant tab.

import {
  createMemo,
  createResource,
  createSignal,
  For,
  Match,
  Show,
  Switch,
  type Component,
  type JSX,
} from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import * as coresApi from "@oa/platform/api/coresApi";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import {
  BackgroundJobsSettings,
  BiosSettings,
  CoresCategorySettings,
  StorageSettings,
} from "./SettingsSections";
import SystemReadinessChecklist from "./import-wizard/SystemReadinessChecklist";
import { activeJobs } from "@oa/platform/lib/backgroundJobs";
import { usePlatform } from "@oa/platform/platformContext";

export type HealthTabId = "overview" | "bios" | "cores" | "storage" | "jobs";

type TabDef = {
  id: HealthTabId;
  label: string;
  glyph: string;
};

const TABS: readonly TabDef[] = [
  { id: "overview", label: "Overview", glyph: "◌" },
  { id: "bios", label: "BIOS", glyph: "⊟" },
  { id: "cores", label: "Cores", glyph: "⊙" },
  { id: "storage", label: "Storage", glyph: "⌑" },
  { id: "jobs", label: "Jobs", glyph: "⟳" },
];

const TAB_STORAGE_KEY = "oa.systemHealth.activeTab";

const SystemHealthPage: Component = () => {
  const initialTab: HealthTabId = (() => {
    try {
      const saved = localStorage.getItem(TAB_STORAGE_KEY) as HealthTabId | null;
      if (saved && TABS.some((t) => t.id === saved)) return saved;
    } catch {
      // localStorage unavailable — fall through.
    }
    return "overview";
  })();

  const [activeTab, setActiveTabRaw] = createSignal<HealthTabId>(initialTab);
  const setActiveTab = (tab: HealthTabId) => {
    setActiveTabRaw(tab);
    try {
      localStorage.setItem(TAB_STORAGE_KEY, tab);
    } catch {
      // localStorage unavailable — ignore.
    }
  };

  return (
    <div class="flex h-full flex-col">
      {/* Tab strip. Sticky-ish at top of the page body; the SettingsPage
          center pane is the scroll container. */}
      <div
        role="tablist"
        aria-label="System Health sections"
        class="mb-6 flex shrink-0 gap-1 overflow-x-auto rounded-lg border border-white/5 bg-white/[0.02] p-1"
      >
        <For each={TABS}>
          {(tab) => {
            const isActive = () => activeTab() === tab.id;
            return (
              <button
                type="button"
                role="tab"
                aria-selected={isActive()}
                onClick={(e) => {
                  e.currentTarget.blur();
                  setActiveTab(tab.id);
                }}
                class="flex items-center gap-2 rounded-md px-3 py-1.5 text-[0.7rem] font-semibold uppercase tracking-wider transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                classList={{
                  "bg-(--color-system-accent)/15 text-(--color-oa-ink)": isActive(),
                  "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)": !isActive(),
                }}
              >
                <span class="text-sm" aria-hidden="true">
                  {tab.glyph}
                </span>
                <span>{tab.label}</span>
              </button>
            );
          }}
        </For>
      </div>

      {/* Active tab body. Each <Show> mounts only when active so resources
          tied to a tab body don't fire until the operator visits it. */}
      <div class="min-h-0 flex-1">
        <Switch>
          <Match when={activeTab() === "overview"}>
            <OverviewBody onSwitchTab={setActiveTab} />
          </Match>
          <Match when={activeTab() === "bios"}>
            <BiosSettings />
          </Match>
          <Match when={activeTab() === "cores"}>
            <CoresCategorySettings />
          </Match>
          <Match when={activeTab() === "storage"}>
            <StorageSettings />
          </Match>
          <Match when={activeTab() === "jobs"}>
            <BackgroundJobsSettings />
          </Match>
        </Switch>
      </div>
    </div>
  );
};

export default SystemHealthPage;

// --- Overview body ---------------------------------------------------------

type OverviewProps = {
  /// Callback to switch the parent tab strip. Each rollup card's CTA
  /// uses this to deep-link to BIOS / Cores / Storage / Jobs.
  onSwitchTab: (tab: HealthTabId) => void;
};

const OverviewBody: Component<OverviewProps> = (props) => {
  const ctx = usePlatform();

  const librarySystems = createMemo<SystemId[]>(() =>
    Array.from(new Set(ctx.library.state.entries.map((e) => e.systemId))),
  );

  return (
    <div class="flex flex-col gap-6">
      <section>
        <h2 class="mb-3 text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
          Status rollup
        </h2>
        <div class="flex flex-col gap-2">
          <CoresRollupCard onOpen={() => props.onSwitchTab("cores")} />
          <BiosRollupCard onOpen={() => props.onSwitchTab("bios")} />
          <ReadinessRollupCard librarySystems={librarySystems} />
          <JobsRollupCard onOpen={() => props.onSwitchTab("jobs")} />
          <StorageRollupCard onOpen={() => props.onSwitchTab("storage")} />
        </div>
      </section>

      <section id="oa-health-readiness-detail" style="scroll-margin-top: 12px">
        <h2 class="mb-3 text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
          Per-system readiness
        </h2>
        <Show
          when={librarySystems().length > 0}
          fallback={
            <p class="rounded-lg border border-white/5 bg-white/[0.02] p-4 text-sm text-(--color-oa-ink-dim)">
              Library is empty — import a folder from Settings → Library
              to populate the readiness checklist.
            </p>
          }
        >
          <SystemReadinessChecklist
            systems={librarySystems}
            emptyStateLabel="Library is empty — import a folder to get started."
          />
        </Show>
      </section>
    </div>
  );
};

// --- StatusRollupCard ------------------------------------------------------

type RollupStatus = "good" | "warn" | "bad" | "neutral" | "loading";

const STATUS_DOT_CLASSES: Record<RollupStatus, string> = {
  good: "bg-emerald-400 shadow-[0_0_0_3px_rgba(52,211,153,0.18)]",
  warn: "bg-amber-400 shadow-[0_0_0_3px_rgba(251,191,36,0.18)]",
  bad: "bg-rose-400 shadow-[0_0_0_3px_rgba(251,113,133,0.18)]",
  neutral: "bg-white/40 shadow-[0_0_0_3px_rgba(255,255,255,0.08)]",
  loading: "bg-white/20 animate-pulse",
};

type StatusRollupCardProps = {
  status: RollupStatus;
  title: string;
  primary: string;
  detail?: string;
  ctaLabel?: string;
  /// CTA glyph — defaults to → for tab deep-links. Use ↓ for the
  /// Readiness card's same-page scroll-to-details affordance.
  ctaArrow?: string;
  /// Click handler for the CTA. If omitted, no CTA renders.
  onClick?: () => void;
};

const StatusRollupCard: Component<StatusRollupCardProps> = (props): JSX.Element => {
  return (
    <div class="flex items-center gap-4 rounded-lg border border-white/10 bg-white/[0.03] px-4 py-3 transition hover:border-white/15">
      <span
        class={`mt-0.5 inline-block h-2.5 w-2.5 shrink-0 rounded-full ${STATUS_DOT_CLASSES[props.status]}`}
        aria-hidden="true"
      />
      <div class="min-w-0 flex-1">
        <div class="flex items-baseline gap-3">
          <h3 class="shrink-0 text-[0.85rem] font-semibold text-(--color-oa-ink)">
            {props.title}
          </h3>
          <p class="min-w-0 truncate text-[0.75rem] text-(--color-oa-ink-dim)">
            {props.primary}
          </p>
        </div>
        <Show when={props.detail}>
          <p class="mt-0.5 truncate text-[0.65rem] text-(--color-oa-ink-dim)/80">
            {props.detail}
          </p>
        </Show>
      </div>
      <Show when={props.onClick && props.ctaLabel}>
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            props.onClick!();
          }}
          class="shrink-0 rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:border-(--color-system-accent)/40 hover:bg-(--color-system-accent)/10 hover:text-(--color-oa-ink) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
        >
          {props.ctaLabel} {props.ctaArrow ?? "→"}
        </button>
      </Show>
    </div>
  );
};

// --- Rollup card data fetchers ---------------------------------------------

type CoreEntry = {
  fileName: string;
  libraryName: string;
  libraryVersion: string;
  validExtensions: string;
};

const CoresRollupCard: Component<{ onOpen: () => void }> = (props) => {
  const [cores] = createResource(async () => {
    try {
      return await coresApi.listCores<CoreEntry>();
    } catch (e) {
      console.warn("[oa-health] list_cores failed:", e);
      return [] as CoreEntry[];
    }
  });

  const status = (): RollupStatus => {
    if (cores.loading) return "loading";
    const list = cores();
    if (!list || list.length === 0) return "warn";
    return "good";
  };

  const primary = () => {
    if (cores.loading) return "Scanning cores folder…";
    const n = cores()?.length ?? 0;
    if (n === 0) return "No cores installed yet";
    return `${n} core${n === 1 ? "" : "s"} installed`;
  };

  const detail = () => {
    if (cores.loading) return undefined;
    const n = cores()?.length ?? 0;
    if (n === 0) return "Install at least one libretro .dll to play games";
    return undefined;
  };

  return (
    <StatusRollupCard
      status={status()}
      title="Cores"
      primary={primary()}
      detail={detail()}
      ctaLabel="Manage"
      onClick={props.onOpen}
    />
  );
};

type BiosEntryStatus = "ok" | "unknownHash" | "missing" | "error";

type BiosStatusResponse = {
  systemDir: string;
  entries: { slug: string; status: BiosEntryStatus; label: string }[];
};

const BiosRollupCard: Component<{ onOpen: () => void }> = (props) => {
  const [bios] = createResource(async () => {
    try {
      return await coresApi.getBiosStatus<BiosStatusResponse>();
    } catch (e) {
      console.warn("[oa-health] get_bios_status failed:", e);
      return null;
    }
  });

  const counts = createMemo(() => {
    const entries = bios()?.entries ?? [];
    return {
      total: entries.length,
      ok: entries.filter((e) => e.status === "ok").length,
      missing: entries.filter((e) => e.status === "missing").length,
      unknown: entries.filter(
        (e) => e.status === "unknownHash" || e.status === "error",
      ).length,
      missingSlugs: entries
        .filter((e) => e.status === "missing")
        .map((e) => e.slug),
    };
  });

  const status = (): RollupStatus => {
    if (bios.loading) return "loading";
    if (!bios()) return "neutral";
    const c = counts();
    if (c.missing > 0) return "bad";
    if (c.unknown > 0) return "warn";
    return "good";
  };

  const primary = () => {
    if (bios.loading) return "Checking BIOS files…";
    const c = counts();
    if (c.total === 0) return "No BIOS-requiring systems yet";
    return `${c.ok}/${c.total} staged${c.missing > 0 ? ` · ${c.missing} missing` : ""}${c.unknown > 0 ? ` · ${c.unknown} present-but-unrecognized` : ""}`;
  };

  const detail = () => {
    if (bios.loading) return undefined;
    const c = counts();
    if (c.missing === 0) return undefined;
    const slugs = c.missingSlugs.slice(0, 4).join(", ");
    const more = c.missingSlugs.length > 4 ? ` + ${c.missingSlugs.length - 4} more` : "";
    return `Missing: ${slugs}${more}`;
  };

  return (
    <StatusRollupCard
      status={status()}
      title="BIOS"
      primary={primary()}
      detail={detail()}
      ctaLabel={counts().missing > 0 ? "Resolve" : "Manage"}
      onClick={props.onOpen}
    />
  );
};

type AvailableCore = {
  base: string;
  fileName: string;
  systems: string[];
  installed: boolean;
};

const ReadinessRollupCard: Component<{
  librarySystems: () => SystemId[];
}> = (props) => {
  // Reuses the same signals SystemReadinessChecklist consumes — cheap,
  // since createResource dedups via the resource cache key (no key →
  // each call is its own; the rendered cost is acceptable for a 1-shot
  // fetch on tab visit).
  const [cores] = createResource(async () => {
    try {
      return await coresApi.listCores<CoreEntry>();
    } catch {
      return [] as CoreEntry[];
    }
  });
  const [available] = createResource(async () => {
    try {
      return await coresApi.availableCores<AvailableCore>();
    } catch {
      return [] as AvailableCore[];
    }
  });
  const [bios] = createResource(async () => {
    try {
      return await coresApi.getBiosStatus<BiosStatusResponse>();
    } catch {
      return null;
    }
  });

  const counts = createMemo(() => {
    const systems = props.librarySystems();
    if (systems.length === 0) {
      return { total: 0, ready: 0, incomplete: 0, incompleteIds: [] as SystemId[] };
    }
    const coresList = cores() ?? [];
    const availList = available() ?? [];
    const biosBySlug = new Map<string, BiosEntryStatus>();
    for (const e of bios()?.entries ?? []) biosBySlug.set(e.slug, e.status);

    let ready = 0;
    const incompleteIds: SystemId[] = [];
    for (const id of systems) {
      const coreOk = isCoreInstalledFor(id, availList, coresList);
      const biosStatus = biosBySlug.get(id);
      const biosOk = biosStatus === undefined || biosStatus === "ok";
      if (coreOk && biosOk) ready += 1;
      else incompleteIds.push(id);
    }
    return {
      total: systems.length,
      ready,
      incomplete: incompleteIds.length,
      incompleteIds,
    };
  });

  const loading = () => cores.loading || available.loading || bios.loading;

  const status = (): RollupStatus => {
    if (loading()) return "loading";
    const c = counts();
    if (c.total === 0) return "neutral";
    if (c.incomplete === 0) return "good";
    if (c.incomplete <= 2) return "warn";
    return "bad";
  };

  const primary = () => {
    if (loading()) return "Computing readiness…";
    const c = counts();
    if (c.total === 0) return "No systems in library yet";
    return `${c.ready}/${c.total} systems ready${c.incomplete > 0 ? ` · ${c.incomplete} incomplete` : ""}`;
  };

  /// Names of incomplete systems for the detail line. First 3 by alpha
  /// display name + "+ N more" when there are more, so the line stays
  /// readable even when half the library is incomplete.
  const detailLine = (): string | undefined => {
    const c = counts();
    if (c.incomplete === 0) return undefined;
    const names = c.incompleteIds
      .map((id) => systemThemes[id]?.displayName ?? id)
      .sort((a, b) => a.localeCompare(b));
    const shown = names.slice(0, 3);
    const more = names.length - shown.length;
    const list = shown.join(", ") + (more > 0 ? ` + ${more} more` : "");
    return `${list} need${c.incomplete === 1 ? "s" : ""} attention`;
  };

  function jumpToDetails() {
    const el = document.getElementById("oa-health-readiness-detail");
    if (el) el.scrollIntoView({ behavior: "smooth", block: "start" });
  }

  return (
    <StatusRollupCard
      status={status()}
      title="Library readiness"
      primary={primary()}
      detail={detailLine()}
      ctaLabel={counts().incomplete > 0 ? "Jump to details" : undefined}
      ctaArrow="↓"
      onClick={counts().incomplete > 0 ? jumpToDetails : undefined}
    />
  );
};

function isCoreInstalledFor(
  systemId: SystemId,
  available: AvailableCore[],
  cores: CoreEntry[],
): boolean {
  // Catalog-membership check.
  if (available.some((c) => c.systems.includes(systemId) && c.installed)) {
    return true;
  }
  // Extension-overlap fallback for systems without catalog entries.
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

type JobPrefsShape = {
  promptBeforeResumeOnLaunch: Record<string, boolean>;
  soundOnCompletion: boolean;
  alwaysShowBar: boolean;
};

const JobsRollupCard: Component<{ onOpen: () => void }> = (props) => {
  const [prefs] = createResource(async () => {
    try {
      return await invoke<JobPrefsShape>("get_job_prefs");
    } catch {
      return null;
    }
  });

  const activeCount = () => activeJobs().length;
  const optOutCount = createMemo(() => {
    const map = prefs()?.promptBeforeResumeOnLaunch ?? {};
    return Object.values(map).filter((v) => v === true).length;
  });

  const status = (): RollupStatus => {
    if (activeCount() > 0) return "warn";
    return "good";
  };

  const primary = () => {
    const n = activeCount();
    if (n === 0) return "Idle";
    return `${n} job${n === 1 ? "" : "s"} active`;
  };

  const detail = () => {
    const n = optOutCount();
    if (n === 0) return undefined;
    return `${n} resume-prompt opt-out${n === 1 ? "" : "s"} set`;
  };

  return (
    <StatusRollupCard
      status={status()}
      title="Background jobs"
      primary={primary()}
      detail={detail()}
      ctaLabel="Open"
      onClick={props.onOpen}
    />
  );
};

type StorageSystemStatus = {
  cpuPercent: number;
  ramUsedBytes: number;
  ramTotalBytes: number;
  dataDirFreeBytes: number | null;
  dataDirTotalBytes: number | null;
};

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const kb = bytes / 1024;
  if (kb < 1024) return `${kb.toFixed(0)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(1)} MB`;
  const gb = mb / 1024;
  if (gb < 1024) return `${gb.toFixed(1)} GB`;
  return `${(gb / 1024).toFixed(2)} TB`;
}

const StorageRollupCard: Component<{ onOpen: () => void }> = (props) => {
  const [info] = createResource(async () => {
    try {
      return await invoke<StorageSystemStatus>("get_system_status");
    } catch (e) {
      console.warn("[oa-health] get_system_status failed:", e);
      return null;
    }
  });

  const freePercent = createMemo(() => {
    const i = info();
    if (!i || !i.dataDirFreeBytes || !i.dataDirTotalBytes) return null;
    return Math.round((i.dataDirFreeBytes / i.dataDirTotalBytes) * 100);
  });

  const status = (): RollupStatus => {
    if (info.loading) return "loading";
    const pct = freePercent();
    if (pct === null) return "neutral";
    if (pct < 5) return "bad";
    if (pct < 10) return "warn";
    return "good";
  };

  const primary = () => {
    if (info.loading) return "Reading disk info…";
    const i = info();
    if (!i || !i.dataDirFreeBytes) return "Free space unavailable";
    const pct = freePercent();
    return `${formatBytes(i.dataDirFreeBytes)} free${pct !== null ? ` · ${pct}%` : ""}`;
  };

  const detail = () => {
    const i = info();
    if (!i || !i.dataDirTotalBytes) return undefined;
    return `of ${formatBytes(i.dataDirTotalBytes)} total on data drive`;
  };

  return (
    <StatusRollupCard
      status={status()}
      title="Storage"
      primary={primary()}
      detail={detail()}
      ctaLabel="Details"
      onClick={props.onOpen}
    />
  );
};
