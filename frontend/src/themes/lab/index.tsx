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
// STATE (2026-06-17): LIVE motion. M-mod.1 = the §2 basis drives the Home↔Library
// route swap (SpecTransition/WAAPI, LAB_VIEW_SPEC). M-mod.2 = selection
// choreography — a focus-driven hero whose cover springs in (createSpringValue +
// BENCH_SELECTION_SPRING) and whose title/meta rise staggered. Each motion site is
// tagged `// [GRAPHICS-LAB] MOTION (M-mod.N)`.
//
// Like every theme it consumes ONLY the platform layer (usePlatform stores +
// useTheme host + @oa/platform/nav + media), never engine/ and never another
// theme — so stripping it leaves nothing dangling but the four touch-points in
// the strip checklist.

import { createEffect, createMemo, createSignal, For, Show, type JSX } from "solid-js";
import { usePlatform } from "@oa/platform/platformContext";
import { useTheme } from "@oa/platform/theme/host";
import { useMedia } from "@oa/platform/library/media";
import { GridNav } from "@oa/platform/nav";
import { systemThemes } from "@oa/platform/themes/registry";
import SpecTransition from "@oa/platform/theme/SpecTransition";
import AmbientMotion from "@oa/platform/theme/AmbientMotion";
import { resolveThemeMotionSpec, usePrefersReducedMotion } from "@oa/platform/theme/motion";
import { createSpringValue } from "@oa/platform/theme/springValue";
import { BENCH_SELECTION_SPRING } from "@oa/platform/theme/spring";
import type { MotionSpec } from "@oa/platform/theme/motionSpec";
import EngineSummonIcon from "@oa/platform/components/EngineSummonIcon";
import type { RomEntry } from "@oa/platform/library/types";
import type { ThemeEntry, ThemePackage } from "@oa/platform/theme/types";
import type { ThemeManifest } from "@oa/platform/theme/manifest";

// The lab's internal routes. Kept LOCAL to this file (a private signal, not the
// platform router) so the whole theme is one strippable unit. Two routes is
// enough to dogfood view-transitions; more get added as the model grows.
type LabRoute = "home" | "library";

// [GRAPHICS-LAB] The authored view-transition spec — the §2 basis as DATA:
// separate channels (opacity + y travel) over timing primitives. Tuned past the
// fixed M1 preset to address the 2026-06-17 "a little fast" note — 560ms + 140px
// of travel with an even easeOutCubic. It now lives in the manifest (below), so
// this is what a real theme would author; the SpecTransition player compiles it
// to WAAPI keyframes via resolveThemeMotionSpec.
const LAB_VIEW_SPEC: MotionSpec = {
  duration: 560,
  easing: "cubic-bezier(0.33, 1, 0.68, 1)",
  channels: { opacity: [0, 1], y: [140, 0] },
};

// [GRAPHICS-LAB] MOTION (M-mod.3) — the ambient "breathe": a gentle infinite scale
// pulse (audit §3 category C). repeat:"infinite" + direction:"alternate" over a
// [rest, peak] scale channel = the loop. Played by AmbientMotion (which no-ops
// under reduced motion). Subtle (3%) so it reads as life, not a throb.
const LAB_BREATHE_SPEC: MotionSpec = {
  duration: 2600,
  easing: "ease-in-out",
  repeat: "infinite",
  direction: "alternate",
  channels: { scale: [1, 1.03] },
};

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
  // [GRAPHICS-LAB] MOTION (M-mod.1): motion authored as manifest DATA (graduated
  // off the former lab-inline usage). `view_transition_spec` is the §2 basis;
  // `resolveThemeMotionSpec` turns it (or, for other themes, a `view_transition`
  // preset) into the spec SpecTransition plays. Disk themes get the same path once
  // the Rust theme_loader widens for `[motion.view_transition_spec]` (deferred —
  // no disk theme authors one yet).
  motion: { view_transition_spec: LAB_VIEW_SPEC },
};

const GRID_COLUMNS = 4; // narrower now that the hero panel takes the right column
// GridNav is NOT virtualized (it renders one DOM cell + <img> per item). The lab
// is a motion testbed, not the shipping library view, so cap the grid — a cheap
// first mount keeps grid-build work from janking the route-transition animation.
const LAB_GRID_CAP = 60;

// [GRAPHICS-LAB] MOTION (M-mod.2): the selection-choreography entrance keyframes,
// lab-local (self-contained, like the F10 bench). `oa-lab-rise` = the title/meta
// rise-with-overshoot; the back-ease `cubic-bezier(.34,1.7,.64,1)` (overshoot 1.7)
// is the bench's tuned entrance curve. The hero ART scale-in is spring-driven
// instead (createSpringValue), not a keyframe — that's where the §2 spring shows.
const LAB_CHOREO_KEYFRAMES = `
@keyframes oa-lab-rise {
  from { opacity: 0; transform: translateY(20px); }
  to   { opacity: 1; transform: translateY(0); }
}
`;
const LAB_RISE_EASE = "cubic-bezier(.34, 1.7, .64, 1)";

const LabEntry: ThemeEntry = (_props) => {
  const platform = usePlatform();
  const host = useTheme();
  const media = useMedia();

  const [route, setRoute] = createSignal<LabRoute>("home");
  const reducedMotion = usePrefersReducedMotion();

  // [GRAPHICS-LAB] MOTION (M-mod.2): selection choreography. The grid runs in
  // CONTROLLED focus mode so the lab knows the focused item and can drive a hero
  // panel off it (BigBox-style: the hero re-plays an entrance on every focus move,
  // not just on click).
  const [focusedIndex, setFocusedIndex] = createSignal(0);
  const focusedGame = (): RomEntry | undefined => games()[focusedIndex()];

  // The hero ART scale-in is SPRING-driven (the §2 spring finally driving pixels):
  // on each focus change we snap to 0.9 then spring to 1 → an alive grow-in with
  // the bench-validated {bounce:0.13,duration:456} feel (BENCH_SELECTION_SPRING,
  // the F10 k=190/damping=24 back-solve). restDelta tightened for a unit-scale
  // value. Reduced motion holds it at 1 (no grow).
  const artScale = createSpringValue(1, BENCH_SELECTION_SPRING, { restDelta: 0.002, restVelocity: 0.004 });
  createEffect(() => {
    focusedIndex(); // re-run on every focus move
    if (reducedMotion()) {
      artScale.snap(1);
      return;
    }
    artScale.snap(0.9);
    artScale.set(1);
  });

  // Mount-once-keep latch for the grid: the heavy GridNav mounts on the first
  // Library visit and then STAYS in the DOM (display-toggled below), so later
  // Home↔Library switches never rebuild/teardown it — that rebuild was the
  // route-switch delay AND the frame-drop that turned the slide into a hiccup.
  const [libVisited, setLibVisited] = createSignal(false);
  createEffect(() => {
    if (route() === "library") setLibVisited(true);
  });

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
    return out.slice(0, LAB_GRID_CAP);
  });

  const coverFor = (e: RomEntry): string | null =>
    (e.identityId ? media.coverUrl(e.systemId, e.identityId) : null) ??
    media.coverUrl(e.systemId, e.id);

  // Real per-game metadata chips for the hero's staggered entrance (year / genre /
  // players, from MediaDb — the games table never carries these). System short
  // name always leads so there's at least one chip.
  const metaChips = (e: RomEntry): string[] => {
    const m = media.media(e.identityId ?? e.id)?.metadata;
    const chips: string[] = [systemThemes[e.systemId]?.shortName ?? e.systemId];
    if (m?.year) chips.push(String(m.year));
    if (m?.genre) chips.push(m.genre);
    if (m?.players) chips.push(`${m.players}P`);
    return chips;
  };

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
    // `overflow-hidden` clips the view-transition's translateY overshoot so it
    // can't extend an ancestor's scroll area → no transient second scrollbar
    // during the slide (the M0 MOTION.md rule #2 scroll-container interaction,
    // confirmed live 2026-06-17). A shell root never scrolls as a whole; the
    // library's own grid keeps its inner overflow-y-auto scrollbar.
    <div class="flex h-full w-full flex-col overflow-hidden bg-[#0b0d12] text-white">
      <style>{LAB_CHOREO_KEYFRAMES}</style>
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

      {/* [GRAPHICS-LAB] MOTION (M-mod.1) — LIVE: the routed region IS the
          `SpecTransition` host. `trigger={route}` replays LAB_VIEW_SPEC (the §2
          basis, WAAPI) on every Home↔Library swap; children render synchronously
          so switching is never blocked (interruptible). The slide's translateY
          overshoot is clipped by the shell root's `overflow-hidden` (no transient
          scrollbar — MOTION.md rule #2). */}
      <SpecTransition
        class="relative min-h-0 flex-1"
        trigger={route}
        spec={() => resolveThemeMotionSpec(LAB_MANIFEST.motion, reducedMotion())}
      >
        {/* Home — cheap, kept mounted; display-toggled so the switch never rebuilds. */}
        <section
          class="h-full flex-col items-center justify-center gap-3 px-8 text-center"
          classList={{ flex: route() === "home", hidden: route() !== "home" }}
        >
          <h1 class="text-3xl font-black tracking-tight">Graphics Lab</h1>
          <p class="max-w-lg text-sm leading-relaxed text-white/55">
            In-flight rendering testbed. Switch to <b class="text-white/80">Library</b>{" "}
            to exercise the cover grid; motion, shaders, and video land here as the
            declarative model grows. Nothing here ships.
          </p>
          <p class="text-[0.6rem] uppercase tracking-[0.3em] text-white/30">
            {games().length} games shown (capped at {LAB_GRID_CAP})
          </p>
        </section>

        {/* Library — mounts on first visit (libVisited latch) and STAYS; only its
            display toggles after that, so Home↔Library never rebuilds the grid.
            Two columns: the focusable grid (left) + the choreographed hero (right). */}
        <Show when={libVisited()}>
          <div
            class="grid h-full grid-cols-[1fr_22rem] gap-4 p-6"
            classList={{ grid: route() === "library", hidden: route() !== "library" }}
          >
            <GridNav
              id="lab-library"
              class="min-h-0 overflow-y-auto pr-2"
              items={games}
              columns={GRID_COLUMNS}
              focusedIndex={focusedIndex}
              setFocusedIndex={setFocusedIndex}
              hints={{ dpad: "Move", stick: "Move", Confirm: "Launch", Secondary: "Details" }}
              onConfirm={(_i, entry) => void host.onLaunch(entry)}
              onSecondary={(_i, entry) => host.onShowInfo(entry)}
            >
              {(entry, ctx) => {
                const cover = coverFor(entry);
                // Per-tile focus lift = a cheap CSS scale (one transition per tile,
                // not a spring — the spring drives the single hero art below, the
                // bench's split: CSS for the row, spring for the hero/track).
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

            {/* [GRAPHICS-LAB] MOTION (M-mod.2) — the choreographed hero. Keyed on
                the focused game so the title/meta entrance REPLAYS on every focus
                move; the cover scale is spring-driven (artScale) for an alive
                grow-in. This is the selection-choreography "soul" (MOTION.md). */}
            <Show
              when={focusedGame()}
              fallback={
                <aside class="grid min-h-0 place-items-center rounded-xl border border-white/10 bg-black/30 p-6 text-center text-xs text-white/40">
                  No games in library
                </aside>
              }
              keyed
            >
              {(g) => (
                <aside class="flex min-h-0 flex-col items-center justify-center gap-4 rounded-xl border border-white/10 bg-black/30 p-6">
                  {/* [GRAPHICS-LAB] MOTION (M-mod.3) — AMBIENT: the outer layer
                      breathes (scale loop) so the hero has idle life; the inner
                      cover independently springs in on focus (M-mod.2). Nested
                      transforms compose — two effects, no conflict. */}
                  <AmbientMotion spec={() => LAB_BREATHE_SPEC} reducedMotion={reducedMotion}>
                    <Show
                      when={coverFor(g)}
                      fallback={
                        <div
                          class="aspect-[3/4] w-44 rounded-lg bg-white/[0.06]"
                          style={{ transform: `scale(${artScale.value()})`, "transform-origin": "center" }}
                        />
                      }
                    >
                      {(u) => (
                        <img
                          src={u()}
                          alt=""
                          class="aspect-[3/4] w-44 rounded-lg object-cover shadow-2xl"
                          style={{ transform: `scale(${artScale.value()})`, "transform-origin": "center" }}
                        />
                      )}
                    </Show>
                  </AmbientMotion>
                  <h2
                    class="text-center text-xl font-black leading-tight tracking-tight"
                    style={{ animation: `oa-lab-rise 420ms ${LAB_RISE_EASE} both` }}
                  >
                    {g.title}
                  </h2>
                  <div class="flex flex-wrap justify-center gap-2">
                    <For each={metaChips(g)}>
                      {(chip, i) => (
                        <span
                          class="rounded-full border border-white/20 bg-black/40 px-3 py-1 text-[0.6rem] uppercase tracking-wider text-white/80"
                          style={{ animation: `oa-lab-rise 420ms ${LAB_RISE_EASE} ${120 + i() * 70}ms both` }}
                        >
                          {chip}
                        </span>
                      )}
                    </For>
                  </div>
                </aside>
              )}
            </Show>
          </div>
        </Show>
      </SpecTransition>
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
