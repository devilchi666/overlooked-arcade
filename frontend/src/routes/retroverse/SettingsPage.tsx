// Retroverse-UI Phase C1 Slice 8 — SETTINGS tab.
//
// Three-pane internal layout matching docs/PLANS/settings-tab-retroverse.md:
//   - Left:   category sidebar (OA-WIDE / CONTENT / SYSTEM groups +
//             collapsed PER-SYSTEM group)
//   - Center: active category's content as a stack of glass-morphism cards
//   - Right:  live-preview pane (Phase C1 ships static help text per
//             category; rich previews are a later polish slice)
//
// Slice 8 ships the layout + category nav with every category as a
// stub. Slice 9 fills in real bodies for Display / Audio / Gameplay /
// Shaders by lifting them out of the existing SettingsDialogs.tsx
// modals. The other categories stay as "Coming in a follow-up" stubs
// until each gets its own polish slice.

import { createSignal, For, Show, type Component } from "solid-js";
import { HintRegion } from "../../nav/HintBar";

type CategoryGroup = "oa-wide" | "content" | "system";

type CategoryId =
  | "display"
  | "audio"
  | "shaders"
  | "gameplay"
  | "controller-nav"
  | "per-system-ui"
  | "themes"
  | "library"
  | "media"
  | "cores"
  | "bios"
  | "storage"
  | "profile"
  | "about";

type CategoryDef = {
  id: CategoryId;
  group: CategoryGroup;
  label: string;
  glyph: string;
  /// One-line description shown as the center pane header subtitle.
  description: string;
  /// Right-pane help text. Phase C1 keeps this static; rich live previews
  /// (sample tile, audio meter, shader preview) come later.
  helpText: string;
};

const CATEGORIES: readonly CategoryDef[] = [
  // OA-WIDE — the four current modal dialogs + the three newer surfaces.
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
    id: "themes",
    group: "oa-wide",
    label: "Themes",
    glyph: "▦",
    description: "Default OA theme picker.",
    helpText:
      "Reserved for when shells become swappable (e.g. Retroverse vs Heroic-style vs kiosk). One theme today — operator can switch presentation modes via the menu bar's Tools menu.",
  },
  // CONTENT & LIBRARY.
  {
    id: "library",
    group: "content",
    label: "Library",
    glyph: "▤",
    description: "Library folders + scanner cadence.",
    helpText:
      "Where OA looks for ROMs. Each folder can scan-subfolders, treat-subfolders-as-systems, or watch for changes. Reachable today via the menu bar's Library Manager…",
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
    id: "cores",
    group: "content",
    label: "Cores",
    glyph: "⊙",
    description: "Installed libretro cores.",
    helpText:
      "Status view of installed .dll cores in <exe_dir>/cores/. Versions + last-modified + which systems each is wired to. Updates flow through RetroArch's own buildbot — OA doesn't bundle its own core updater.",
  },
  {
    id: "bios",
    group: "content",
    label: "BIOS",
    glyph: "⊟",
    description: "Per-system BIOS file status.",
    helpText:
      "Status grid showing which systems have their required BIOS files staged in <exe_dir>/system/. Green = ready, amber = present-but-untested, red = missing.",
  },
  // SYSTEM.
  {
    id: "storage",
    group: "system",
    label: "Storage",
    glyph: "⌑",
    description: "Data directory + portable install + saves location.",
    helpText:
      "Shows whether you're running in AppData mode or portable mode (portable.txt next to oa-shell.exe). Lists save / state / log directories and free space on each.",
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

const GROUP_LABELS: Record<CategoryGroup, string> = {
  "oa-wide": "OA-WIDE",
  content: "CONTENT",
  system: "SYSTEM",
};

const GROUP_ORDER: readonly CategoryGroup[] = ["oa-wide", "content", "system"];

const SettingsPage: Component = () => {
  const [activeCategoryId, setActiveCategoryId] = createSignal<CategoryId>("display");
  const activeCategory = () =>
    CATEGORIES.find((c) => c.id === activeCategoryId()) ?? CATEGORIES[0]!;

  const categoriesInGroup = (group: CategoryGroup) =>
    CATEGORIES.filter((c) => c.group === group);

  return (
    <div
      class="grid h-full w-full"
      style={{
        "grid-template-columns": "260px minmax(0,1fr) 320px",
      }}
    >
      {/* Phase C1 hints — keep stub-compatible nav + add Y reset. */}
      <HintRegion
        hints={{
          a: "Select",
          b: "Back",
          x: "Search",
          y: "Reset",
          l1: "Prev tab",
          r1: "Next tab",
        }}
      />

      {/* Left pane — category sidebar with grouped sections. */}
      <aside class="min-w-0 overflow-y-auto border-r border-white/5 px-3 py-4">
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
          <p class="px-2 text-[0.55rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)/60">
            Per-system ▾
          </p>
          <p class="mt-1.5 px-2 text-[0.65rem] text-(--color-oa-ink-dim)/70">
            Expand to drop into a single system's override tier.
            Coming in a follow-up slice.
          </p>
        </section>
      </aside>

      {/* Center pane — active category content. Phase C1 ships stubs;
          Slice 9 lifts the four existing dialog bodies in. */}
      <section class="min-h-0 min-w-0 overflow-y-auto px-8 py-6">
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

        <div class="rounded-xl border border-dashed border-white/10 bg-white/[0.02] p-8 text-center">
          <p class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
            Coming in Slice 9
          </p>
          <p class="mt-3 text-sm text-(--color-oa-ink-dim)">
            The Display / Audio / Gameplay / Shaders bodies port directly from
            the existing modal dialogs (frontend/src/components/SettingsDialogs.tsx).
            Other categories ship as their own polish slices once their data
            paths exist.
          </p>
        </div>
      </section>

      {/* Right pane — live preview placeholder (static help text in
          Phase C1). Rich previews per category land as polish later. */}
      <aside class="min-w-0 overflow-y-auto border-l border-white/5 px-6 py-6">
        <p class="text-[0.55rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
          About this setting
        </p>
        <h2 class="mt-2 text-base font-semibold text-(--color-oa-ink)">
          {activeCategory().label}
        </h2>
        <p class="mt-3 text-sm leading-relaxed text-(--color-oa-ink-dim)">
          {activeCategory().helpText}
        </p>
        <Show
          when={
            activeCategory().id === "library" ||
            activeCategory().id === "media" ||
            activeCategory().id === "cores"
          }
        >
          <p class="mt-4 rounded-md border border-(--color-system-accent)/20 bg-(--color-system-accent)/[0.04] p-3 text-[0.7rem] text-(--color-oa-ink-dim)">
            Today's surface for this category lives in the menu bar
            (Library Manager… / Cores Manager… / Platform Media…). The
            Retroverse-UI port wraps each into this tab as its own slice.
          </p>
        </Show>
      </aside>
    </div>
  );
};

export default SettingsPage;
