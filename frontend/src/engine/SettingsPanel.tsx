// Engine territory — Settings panel.
//
// Lifted from frontend/src/routes/retroverse/SettingsPage.tsx as part
// of ARC 1 Phase 1 of the Theming Substrate arc (docs/PLANS/theming-substrate.md
// §6 Phase 1). Same content, same UX, same internal three-pane layout
// (sidebar / center / per-system picker). The move from
// routes/retroverse/ to engine/ encodes the architectural shift:
// Settings is engine-rendered, not theme-rendered. Every theme summons
// this same panel through the EngineManagerSurface takeover.
//
// Store access (boundary, 2026-06-09): pulls `settings` from
// `usePlatform()` — the theme-agnostic platform store layer — NOT
// `useTheme()`. An engine component must not import the theme context
// (engine ↛ routes). The store-context split (DECISIONS D11) added
// `usePlatform()` for exactly this.
//
// STILL OPEN (next boundary batch): this panel imports its CONTENT —
// PerSystemSettingsBody + SystemHealthPage (routes/) + SettingsSections
// (components/) — which keeps the `engine ↛ routes` lint zone red. Those
// content files relocate into engine/ next (migrating their own store
// reads to usePlatform), after which the zone is enforced.

import { createMemo, createSignal, For, Match, Show, Switch, type Component } from "solid-js";
import { HintRegion } from "@oa/platform/nav";
import {
  AboutSettings,
  AudioSettings,
  ControllerNavSettings,
  DisplayBaseSettings,
  ExperimentalSettings,
  GameplaySettings,
  LibrarySettings,
  MediaSettings,
  PerformanceSettings,
  PerSystemUiSettings,
  ProfileSettings,
  ShadersSettings,
  ThemesSettings,
} from "./SettingsSections";
import PerSystemSettingsBody from "./PerSystemSettingsBody";
import SystemHealthPage from "./SystemHealthPage";
import { useDomQueryFocusGroup } from "@oa/platform/nav";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import { usePlatform } from "@oa/platform/platformContext";

type CategoryGroup = "oa-wide" | "content" | "system";

type CategoryId =
  | "display"
  | "audio"
  | "shaders"
  | "gameplay"
  | "performance"
  | "controller-nav"
  | "per-system-ui"
  | "experimental"
  | "themes"
  | "library"
  | "media"
  | "system-health"
  | "profile"
  | "about"
  | "per-system";

type CategoryDef = {
  id: CategoryId;
  group: CategoryGroup;
  label: string;
  glyph: string;
  description: string;
  helpText: string;
};

const CATEGORIES: readonly CategoryDef[] = [
  {
    id: "display",
    group: "oa-wide",
    label: "Display",
    glyph: "▣",
    description: "How the shell + emulator render on your monitor.",
    helpText:
      "Scaling mode keeps emulator pixels crisp; window mode picks borderless / fullscreen / windowed; monitor selects the target display. Run-ahead trims input latency at a CPU cost.",
  },
  {
    id: "audio",
    group: "oa-wide",
    label: "Audio",
    glyph: "♪",
    description: "Output device + 4-bus mixer levels.",
    helpText:
      "Output device hot-swaps the running stream. The 4-bus mixer (platform-music / ui-sounds / ceremony / snap-audio) lets per-system experiences sit at the right balance.",
  },
  {
    id: "shaders",
    group: "oa-wide",
    label: "Shaders",
    glyph: "★",
    description: "Phosphor / CRT preset + bloom amount.",
    helpText:
      "OA-wide default preset. Per-system + per-game overrides re-push on launch — pick a preset here as the baseline that systems / games inherit unless they specify otherwise.",
  },
  {
    id: "gameplay",
    group: "oa-wide",
    label: "Gameplay",
    glyph: "⏵",
    description: "Rewind capture cadence + buffer cap.",
    helpText:
      "Rewind captures a save-state every N frames; the buffer cap limits how many seconds of history are kept in memory. Larger caps = more rewind range, more RAM.",
  },
  {
    id: "performance",
    group: "oa-wide",
    label: "Performance",
    glyph: "⚡",
    description: "CPU tier detection that drives core recommendations.",
    helpText:
      "Auto-detects your CPU brand + cores + base clock and buckets into High / Mid / Low. The Import Wizard uses this to pick the right core per system (Beetle PSX HW on High, PCSX-ReARMed on Low, etc.). Override below if the heuristic misclassifies — Steam Deck under-rates, old workstations over-rate, throttled laptops over-rate.",
  },
  {
    id: "controller-nav",
    group: "oa-wide",
    label: "Controller nav",
    glyph: "◉",
    description: "Drive the shell with a gamepad.",
    helpText:
      "Master toggle + navigation source (DPad / stick / both) + A/B swap (Nintendo layout) + animation budget for focus-ring transitions.",
  },
  {
    id: "per-system-ui",
    group: "oa-wide",
    label: "Per-system UI",
    glyph: "✦",
    description: "Each system feels like its own mini-experience.",
    helpText:
      "Master toggle + boot animations / tile flourishes / per-system SFX / background art sub-toggles. Turn the master off for a uniform plain library across every system.",
  },
  {
    id: "experimental",
    group: "oa-wide",
    label: "Experimental",
    glyph: "⚗",
    description: "Preview-quality features under active development.",
    helpText:
      "Hosts the Retroverse UI master toggle. Flip it OFF to return to the legacy Shell layout immediately — no restart required. Useful as the escape hatch if you ever get stuck.",
  },
  {
    id: "themes",
    group: "oa-wide",
    label: "Themes",
    glyph: "▦",
    description: "Default OA theme picker.",
    helpText:
      "Reserved for when shells become swappable (e.g. Retroverse vs Heroic-style vs kiosk). One theme today — visual variants land here as they're built.",
  },
  {
    id: "library",
    group: "content",
    label: "Library",
    glyph: "▤",
    description: "Library folders + scanner cadence.",
    helpText:
      "Where OA looks for ROMs. Each folder can scan-subfolders, treat-subfolders-as-systems, or watch for changes. The Library Manager surface (folders, views, game media) embeds directly in this category — Library Manager's full Library / Views / Game media tab strip is what you see when you click in.",
  },
  {
    id: "media",
    group: "content",
    label: "Media",
    glyph: "⊞",
    description: "Per-platform art slots + audio assets.",
    helpText:
      "Banner / clear-logo / console / controller / fanart / marquee / photo / wheel / background per system. Operator-supplied art always wins over synced art.",
  },
  {
    id: "system-health",
    group: "system",
    label: "System Health",
    glyph: "⚕",
    description: "Operational status of your OA install.",
    helpText:
      "Status rollup of Cores / BIOS / Readiness / Background jobs / Storage in one place. Internal tabs hold the deep-dive editors for each — the categories that used to live in the sidebar (BIOS / Cores / Storage / Background Jobs) all live here now.",
  },
  {
    id: "profile",
    group: "system",
    label: "Profile",
    glyph: "👤",
    description: "Display name + avatar.",
    helpText:
      "Drives the profile chip in the top-right corner of every Retroverse tab. Avatar can be picked from a built-in set or a custom image.",
  },
  {
    id: "about",
    group: "system",
    label: "About",
    glyph: "ⓘ",
    description: "Version + license + credits.",
    helpText:
      "OA version, GPL-2.0 license notice, credits to upstream cores (Beetle PCE Fast / Mednafen / MAME / Dolphin / etc.), and a link to the OA issue tracker for bug reports.",
  },
];

const PER_SYSTEM_CATEGORY: CategoryDef = {
  id: "per-system",
  group: "system",
  label: "Per-system",
  glyph: "▥",
  description: "Per-system overrides — display / rewind / shaders / default core / bindings.",
  helpText:
    "Per-system overrides override OA-wide defaults; per-game settings override per-system in turn. Empty rows fall through to the inherited value. Bindings + Core options open as focused editors since they're large enough to warrant their own dialog UI.",
};

const GROUP_LABELS: Record<CategoryGroup, string> = {
  "oa-wide": "OA-WIDE",
  content: "CONTENT",
  system: "SYSTEM",
};

const GROUP_ORDER: readonly CategoryGroup[] = ["oa-wide", "content", "system"];

type Props = {
  /// Initial category to land on when the engine surface opens. Useful
  /// for deep-link summons (e.g. profile chip → "profile"). Defaults
  /// to "display" matching pre-Phase-1 behavior.
  initialCategory?: CategoryId;
};

const SettingsPanel: Component<Props> = (props) => {
  const ctx = usePlatform();
  const [activeCategoryId, setActiveCategoryId] = createSignal<CategoryId>(
    props.initialCategory ?? "display",
  );
  const [perSystemExpanded, setPerSystemExpanded] = createSignal(false);
  const [perSystemActiveId, setPerSystemActiveId] = createSignal<SystemId | null>(null);
  const activeCategory = () => {
    if (activeCategoryId() === "per-system") return PER_SYSTEM_CATEGORY;
    return CATEGORIES.find((c) => c.id === activeCategoryId()) ?? CATEGORIES[0]!;
  };

  const categoriesInGroup = (group: CategoryGroup) =>
    CATEGORIES.filter((c) => c.group === group);

  const allSystems = createMemo<{ id: SystemId; displayName: string }[]>(() => {
    const ids = Object.keys(systemThemes) as SystemId[];
    const rows = ids.map((id) => ({
      id,
      displayName: systemThemes[id]?.displayName ?? id,
    }));
    rows.sort((a, b) => a.displayName.localeCompare(b.displayName));
    return rows;
  });

  function pickSystem(id: SystemId) {
    setPerSystemActiveId(id);
    setActiveCategoryId("per-system");
  }

  let leftRef: HTMLElement | undefined;
  let centerRef: HTMLElement | undefined;
  const LEFT_ID = "engine-settings-left";
  const CENTER_ID = "engine-settings-center";
  useDomQueryFocusGroup({
    id: LEFT_ID,
    containerRef: () => leftRef,
    orientation: "vertical",
    onActivate: (_i, el) => el.click(),
    neighbours: { right: CENTER_ID },
  });
  useDomQueryFocusGroup({
    id: CENTER_ID,
    containerRef: () => centerRef,
    orientation: "vertical",
    autoActivate: false,
    onActivate: (_i, el) => el.click(),
    neighbours: { left: LEFT_ID },
  });

  return (
    <div
      class="grid h-full w-full"
      style={{
        "grid-template-columns": "260px minmax(0,1fr)",
      }}
    >
      <HintRegion
        hints={{
          dpad: "Switch region",
          stick: "Navigate",
          Confirm: "Select",
          Back: "Back",
          Secondary: "Search",
          Tertiary: "Reset",
          PrevSection: "Prev tab",
          NextSection: "Next tab",
        }}
      />

      <aside
        ref={(el) => (leftRef = el)}
        class="min-w-0 overflow-y-auto border-r border-white/5 px-3 py-4"
      >
        <p class="px-2 text-[0.55rem] font-semibold uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
          Categories
        </p>
        <For each={GROUP_ORDER}>
          {(group) => (
            <section class="mt-4">
              <p class="px-2 text-[0.55rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)/70">
                {GROUP_LABELS[group]}
              </p>
              <ul class="mt-1.5 flex flex-col gap-0.5">
                <For each={categoriesInGroup(group)}>
                  {(category) => {
                    const isActive = () => activeCategoryId() === category.id;
                    return (
                      <li>
                        <button
                          type="button"
                          onClick={(e) => {
                            e.currentTarget.blur();
                            setActiveCategoryId(category.id);
                          }}
                          class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                          classList={{
                            "bg-(--color-system-accent)/15 text-(--color-oa-ink)": isActive(),
                            "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)": !isActive(),
                          }}
                          aria-current={isActive() ? "page" : undefined}
                        >
                          <span class="w-4 text-center text-sm">{category.glyph}</span>
                          <span class="truncate">{category.label}</span>
                        </button>
                      </li>
                    );
                  }}
                </For>
              </ul>
            </section>
          )}
        </For>
        <section class="mt-6 border-t border-white/5 pt-4">
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              setPerSystemExpanded((v) => !v);
            }}
            class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left transition hover:bg-white/[0.04] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
            aria-expanded={perSystemExpanded()}
          >
            <span
              class="text-[0.55rem] text-(--color-oa-ink-dim)"
              aria-hidden="true"
            >
              {perSystemExpanded() ? "▾" : "▸"}
            </span>
            <span class="text-[0.55rem] font-semibold uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
              Per-system
            </span>
            <span class="ml-auto text-[0.55rem] text-(--color-oa-ink-dim)/60">
              {allSystems().length}
            </span>
          </button>
          <Show
            when={perSystemExpanded()}
            fallback={
              <p class="mt-1.5 px-2 text-[0.65rem] text-(--color-oa-ink-dim)/70">
                Expand to pick a system and edit its override tier.
              </p>
            }
          >
            <ul class="mt-1 flex max-h-[420px] flex-col gap-0.5 overflow-y-auto">
              <For each={allSystems()}>
                {(sys) => {
                  const isActive = () =>
                    activeCategoryId() === "per-system" &&
                    perSystemActiveId() === sys.id;
                  return (
                    <li>
                      <button
                        type="button"
                        onClick={(e) => {
                          e.currentTarget.blur();
                          pickSystem(sys.id);
                        }}
                        class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                        classList={{
                          "bg-(--color-system-accent)/15 text-(--color-oa-ink)": isActive(),
                          "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)":
                            !isActive(),
                        }}
                        aria-current={isActive() ? "page" : undefined}
                        data-system={sys.id}
                        title={sys.displayName}
                      >
                        <span
                          class="inline-block h-2 w-2 shrink-0 rounded-full bg-(--color-system-accent)"
                          aria-hidden="true"
                        />
                        <span class="truncate">{sys.displayName}</span>
                      </button>
                    </li>
                  );
                }}
              </For>
            </ul>
          </Show>
        </section>
      </aside>

      <section
        ref={(el) => (centerRef = el)}
        class="min-h-0 min-w-0 overflow-y-auto px-8 py-6"
      >
        <header class="mb-6">
          <div class="flex items-center gap-3">
            <span class="text-2xl text-(--color-system-accent)">
              {activeCategory().glyph}
            </span>
            <h1 class="text-xl font-semibold uppercase tracking-widest text-(--color-oa-ink)">
              {activeCategory().label}
            </h1>
          </div>
          <p class="mt-2 text-sm text-(--color-oa-ink-dim)">
            {activeCategory().description}
          </p>
        </header>

        <Switch
          fallback={
            <div class="rounded-xl border border-dashed border-white/10 bg-white/[0.02] p-8 text-center">
              <p class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                Coming in a follow-up slice
              </p>
              <p class="mt-3 text-sm text-(--color-oa-ink-dim)">
                This category's data path needs its own polish slice. The
                fallback should be unreachable in normal use — every
                category in GROUP_ORDER has a Match arm. If you're seeing
                this, please file a bug.
              </p>
            </div>
          }
        >
          <Match when={activeCategoryId() === "display"}>
            <DisplayBaseSettings settings={ctx.settings} />
          </Match>
          <Match when={activeCategoryId() === "audio"}>
            <AudioSettings settings={ctx.settings} />
          </Match>
          <Match when={activeCategoryId() === "shaders"}>
            <ShadersSettings settings={ctx.settings} />
          </Match>
          <Match when={activeCategoryId() === "gameplay"}>
            <GameplaySettings settings={ctx.settings} />
          </Match>
          <Match when={activeCategoryId() === "performance"}>
            <PerformanceSettings settings={ctx.settings} />
          </Match>
          <Match when={activeCategoryId() === "controller-nav"}>
            <ControllerNavSettings settings={ctx.settings} />
          </Match>
          <Match when={activeCategoryId() === "per-system-ui"}>
            <PerSystemUiSettings settings={ctx.settings} />
          </Match>
          <Match when={activeCategoryId() === "experimental"}>
            <ExperimentalSettings settings={ctx.settings} />
          </Match>
          <Match when={activeCategoryId() === "themes"}>
            <ThemesSettings />
          </Match>
          <Match when={activeCategoryId() === "library"}>
            <LibrarySettings />
          </Match>
          <Match when={activeCategoryId() === "media"}>
            <MediaSettings />
          </Match>
          <Match when={activeCategoryId() === "system-health"}>
            <SystemHealthPage />
          </Match>
          <Match when={activeCategoryId() === "profile"}>
            <ProfileSettings settings={ctx.settings} />
          </Match>
          <Match when={activeCategoryId() === "about"}>
            <AboutSettings />
          </Match>
          <Match when={activeCategoryId() === "per-system"}>
            <PerSystemSettingsBody
              systemId={perSystemActiveId}
              settings={ctx.settings}
            />
          </Match>
        </Switch>
      </section>
    </div>
  );
};

export default SettingsPanel;
export type { CategoryId };
