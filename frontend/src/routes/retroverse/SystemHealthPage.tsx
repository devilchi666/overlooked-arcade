// System Health — consolidated diagnostic hub for OA's operational state.
//
// Replaces four previously-standalone SETTINGS categories (BIOS, Cores,
// Storage, Background Jobs) with one category whose body carries an
// internal tab strip. Plan: docs/PLANS/settings-declutter-system-health.md.
//
// Phase 1 ships the scaffolding: tab strip + four absorbed bodies + an
// Overview placeholder. Phase 2 fills Overview with the rollup cards +
// the readiness checklist (lifted from Settings → Library).

import { createSignal, For, Match, Show, Switch, type Component } from "solid-js";
import { type SystemId } from "../../themes/registry";
import {
  BackgroundJobsSettings,
  BiosSettings,
  CoresCategorySettings,
  StorageSettings,
} from "../../components/SettingsSections";
import SystemReadinessChecklist from "../../components/import-wizard/SystemReadinessChecklist";
import { useRetroverse } from "./context";
import { createMemo } from "solid-js";

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
      {/* Tab strip — sticky-ish at the top of the page body. The
          SettingsPage center pane provides the surrounding scroll
          container, so we don't need our own. */}
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
          (createResource, listeners) tied to a tab body don't fire until
          you visit. */}
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
//
// Phase 1: placeholder + lifted SystemReadinessChecklist so the
// readiness rollup has a home from day one. Phase 2 will add the
// 5 status rollup cards above the checklist.

type OverviewProps = {
  /// Callback to switch the parent tab strip. Phase 2's rollup card
  /// CTAs will use this to deep-link into BIOS / Cores / Storage / Jobs.
  onSwitchTab: (tab: HealthTabId) => void;
};

const OverviewBody: Component<OverviewProps> = (_props) => {
  const ctx = useRetroverse();

  // Mirror LibrarySettings' derivation — the per-system readiness
  // checklist consumes the systems the operator actually has games for.
  const librarySystems = createMemo<SystemId[]>(() =>
    Array.from(new Set(ctx.library.state.entries.map((e) => e.systemId))),
  );

  return (
    <div class="flex flex-col gap-4">
      <div class="rounded-xl border border-dashed border-white/10 bg-white/[0.02] p-6 text-center">
        <p class="text-[0.6rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
          Coming in Phase 2
        </p>
        <p class="mt-2 text-sm text-(--color-oa-ink-dim)">
          Status rollup cards (Cores · BIOS · Readiness · Background ·
          Storage) land here in Phase 2 with live counts + deep-link CTAs.
          For now the per-system readiness checklist below is the live
          surface.
        </p>
      </div>

      <section>
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
