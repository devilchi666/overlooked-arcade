// ┌──────────────────────────────────────────────────────────────────────────┐
// │  [GRAPHICS-LAB]  STRIP-ON-SHIP TESTBED THEME — NOT a shipped theme.        │
// │  Permanent home for in-flight graphical work (motion → shaders → video).  │
// │  Reachable ONLY via Settings → Experimental → Graphics Lab (the normal    │
// │  Appearance picker hides it via the `experimental` flag). Remove at ship   │
// │  per docs/features/theming-substrate/GRAPHICS_LAB_TESTBED.md.             │
// └──────────────────────────────────────────────────────────────────────────┘
//
// Theming ARC 3 Thrust M. This is the navigable surface the declarative motion
// MODEL dogfoods on (DECISIONS D55 — a real routes/tabs + grid shell, NOT the
// single-surface DeclarativeShell). It exists in RELEASE builds (unlike the
// F10 `frontend/src/dev/` bench, which is import.meta.env.DEV-gated and so
// absent from the `cargo tauri build` the operator playtests).
//
// FOUNDATION STATE (2026-06-17): the shell is navigable but static — the motion
// model (M-mod.1) has not landed yet. Every spot motion will wire into is marked
// `// [GRAPHICS-LAB] MOTION:` so the M-mod.1 pass is a fill-in, not a search.
//
// Like every theme it consumes ONLY the platform layer (usePlatform stores +
// useTheme host + @oa/platform/nav + media), never engine/ and never another
// theme — so stripping it leaves nothing dangling but the four touch-points in
// the strip checklist.

import { createMemo, createSignal, Show, type JSX } from "solid-js";
import { usePlatform } from "@oa/platform/platformContext";
import { useTheme } from "@oa/platform/theme/host";
import { useMedia } from "@oa/platform/library/media";
import { GridNav } from "@oa/platform/nav";
import EngineSummonIcon from "@oa/platform/components/EngineSummonIcon";
import type { RomEntry } from "@oa/platform/library/types";
import type { ThemeEntry, ThemePackage } from "@oa/platform/theme/types";
import type { ThemeManifest } from "@oa/platform/theme/manifest";

// The lab's internal routes. Kept LOCAL to this file (a private signal, not the
// platform router) so the whole theme is one strippable unit. Two routes is
// enough to dogfood view-transitions; more get added as the model grows.
type LabRoute = "home" | "library";

const LAB_MANIFEST: ThemeManifest = {
  id: "lab",
  name: "Graphics Lab",
  version: "0.1.0",
  schema_version: 1,
  oa_version: "^0.x",
  entry: "./index.tsx",
  entry_export: "lab",
  default_route: "home",
  routes: ["home", "library"],
  context_slots: ["library"],
  required_engine_capabilities: [],
  reserves_corner: "top-right",
  surfaces: ["main"],
  // [GRAPHICS-LAB] MOTION (M-mod.1): `motion.view_transition` will be declared
  // here once the resolver widens past M1's preset set — the lab is where we
  // prove a declared preset drives the route swap below on a real surface.
};

const GRID_COLUMNS = 5;

const LabEntry: ThemeEntry = (_props) => {
  const platform = usePlatform();
  const host = useTheme();
  const media = useMedia();

  const [route, setRoute] = createSignal<LabRoute>("home");
  const [selected, setSelected] = createSignal<RomEntry | null>(null);

  // Real, deduped library (one row per identity), sorted by title — same
  // contract bare/CoverFlow use, so the lab reconciles against stable refs.
  const games = createMemo<RomEntry[]>(() => {
    const seen = new Set<string>();
    const out: RomEntry[] = [];
    for (const e of platform.library.state.entries) {
      if (e.seed) continue;
      const key = e.identityId ?? e.id;
      if (seen.has(key)) continue;
      seen.add(key);
      out.push(e);
    }
    out.sort((a, b) => a.title.localeCompare(b.title));
    return out;
  });

  const coverFor = (e: RomEntry): string | null =>
    (e.identityId ? media.coverUrl(e.systemId, e.identityId) : null) ??
    media.coverUrl(e.systemId, e.id);

  const tab = (id: LabRoute, label: string): JSX.Element => (
    <button
      class="border-b-2 px-4 py-2 text-xs font-semibold uppercase tracking-[0.25em] transition-colors"
      classList={{
        "border-cyan-400 text-white": route() === id,
        "border-transparent text-white/40 hover:text-white/70": route() !== id,
      }}
      onClick={() => setRoute(id)}
    >
      {label}
    </button>
  );

  return (
    <div class="flex h-full w-full flex-col bg-[#0b0d12] text-white">
      <header class="flex items-center justify-between border-b border-white/10 px-6 py-3">
        <div class="flex items-center gap-1">
          <span class="mr-4 text-sm font-black tracking-tight">
            Graphics Lab <span class="text-[0.55rem] font-normal uppercase tracking-[0.3em] text-amber-400/80">experimental</span>
          </span>
          {tab("home", "Home")}
          {tab("library", "Library")}
        </div>
        {/* D3 — every theme reserves top-right for the engine summon (the always-
            available path back to Settings, incl. switching back off the lab). */}
        <EngineSummonIcon />
      </header>

      {/* [GRAPHICS-LAB] MOTION (M-mod.1): this region wraps the routed content and
          becomes the `ViewTransition` host — the route swap below is what the
          declarative view-transition preset will animate. */}
      <main class="relative min-h-0 flex-1">
        <Show when={route() === "home"}>
          <section class="flex h-full flex-col items-center justify-center gap-3 px-8 text-center">
            <h1 class="text-3xl font-black tracking-tight">Graphics Lab</h1>
            <p class="max-w-lg text-sm leading-relaxed text-white/55">
              In-flight rendering testbed. Switch to <b class="text-white/80">Library</b>{" "}
              to exercise the cover grid; motion, shaders, and video land here as the
              declarative model grows. Nothing here ships.
            </p>
            <p class="text-[0.6rem] uppercase tracking-[0.3em] text-white/30">
              {games().length} games in library
            </p>
          </section>
        </Show>

        <Show when={route() === "library"}>
          <GridNav
            id="lab-library"
            class="h-full overflow-y-auto p-6"
            items={games}
            columns={GRID_COLUMNS}
            hints={{ dpad: "Move", stick: "Move", Confirm: "Launch", Secondary: "Details" }}
            onConfirm={(_i, entry) => void host.onLaunch(entry)}
            onSecondary={(_i, entry) => {
              setSelected(entry);
              host.onShowInfo(entry);
            }}
          >
            {(entry, ctx) => {
              const cover = coverFor(entry);
              // [GRAPHICS-LAB] MOTION (M-mod.1): `ctx.focused()` is the trigger
              // for selection choreography (lift / art-grow / title-rise). Today
              // it's a static scale+ring; M-mod.2 swaps in the spring presets.
              return (
                <div
                  class="m-1.5 flex flex-col gap-1 rounded-lg p-1.5 transition-transform duration-150"
                  style={{ transform: ctx.focused() ? "scale(1.06)" : "scale(1)" }}
                  classList={{ "ring-2 ring-cyan-400": ctx.focused() }}
                >
                  <Show
                    when={cover}
                    fallback={
                      <div class="grid aspect-[3/4] w-full place-items-center rounded bg-white/[0.06] p-2 text-center text-[0.6rem] text-white/50">
                        {entry.title}
                      </div>
                    }
                  >
                    {(u) => <img src={u()} alt="" class="aspect-[3/4] w-full rounded object-cover" />}
                  </Show>
                  <span class="truncate text-[0.6rem] text-white/60">{entry.title}</span>
                </div>
              );
            }}
          </GridNav>
        </Show>
      </main>

      {/* Selection echo — a stand-in for the hero/detail panel the selection
          choreography (M-mod.2) will animate. Static for now. */}
      <Show when={selected()}>
        {(g) => (
          <footer class="border-t border-white/10 px-6 py-2 text-[0.65rem] text-white/50">
            Last selected: <span class="text-white/80">{g().title}</span>
          </footer>
        )}
      </Show>
    </div>
  );
};

// [GRAPHICS-LAB] `experimental: true` keeps it out of the Appearance picker
// (registry.availableThemes) while still registered/valid/activatable.
export const lab: ThemePackage = {
  manifest: LAB_MANIFEST,
  entry: LabEntry,
  experimental: true,
};
