// ┌──────────────────────────────────────────────────────────────────────────┐
// │  [GRAPHICS-LAB]  STRIP-ON-SHIP SHOWCASE THEME — NOT a shipped theme.        │
// │  Permanent home for in-flight graphical work (motion → shaders → video).  │
// │  Reachable ONLY via Settings → Experimental → Graphics Lab (the normal    │
// │  Appearance picker hides it via the `experimental` flag). Remove at ship   │
// │  per docs/features/theming-substrate/GRAPHICS_LAB_TESTBED.md.             │
// └──────────────────────────────────────────────────────────────────────────┘
//
// Theming ARC 3 Thrust M. A deliberately MAXIMAL cinematic showcase — the look a
// premium BigBox/HyperSpin theme (Unify / CityHunter) reaches for, built from
// what we have (real cover art + the per-system palette + the motion engine), to
// prove OA's potential to the community. Every effect here is the declarative
// motion model (DECISIONS D55–D57) dogfooded on a navigable surface:
//
//   • Fanart backdrop — the focused cover, blurred + darkened, CROSSFADING on
//     selection (audit `fanart-crossfade`) with a slow KEN-BURNS (ambient).
//   • Hero box-art — the cover with the full §3 box-art stack: spring grow-in
//     (M-mod.2) + breathe (M-mod.3) + pointer-tilt (M-mod.4) + gloss sweep +
//     grounded reflection + a per-system accent glow, composed via nesting.
//   • Hero text — big title + metadata, staggered entrance, system-accent themed.
//   • Coverflow carousel — the library browser (CarouselNav), focused card lifted.
//   • Route transition — Home↔Library via the §2 spec (SpecTransition).
//
// Consumes ONLY the platform layer; stripping it leaves nothing dangling but the
// four touch-points in the strip checklist. All engine modules it imports
// (motionSpec/spring/springValue/SpecTransition/AmbientMotion/tilt) are PRODUCT
// code that STAYS — only this folder strips.

import { createEffect, createMemo, createSignal, For, onCleanup, Show, type JSX } from "solid-js";
import { usePlatform } from "@oa/platform/platformContext";
import { useTheme } from "@oa/platform/theme/host";
import { useMedia } from "@oa/platform/library/media";
import { CarouselNav } from "@oa/platform/nav";
import { systemThemes } from "@oa/platform/themes/registry";
import SpecTransition from "@oa/platform/theme/SpecTransition";
import AmbientMotion from "@oa/platform/theme/AmbientMotion";
import { useTilt } from "@oa/platform/theme/tilt";
import { resolveThemeMotionSpec, usePrefersReducedMotion } from "@oa/platform/theme/motion";
import { createSpringValue } from "@oa/platform/theme/springValue";
import { BENCH_SELECTION_SPRING } from "@oa/platform/theme/spring";
import type { MotionSpec } from "@oa/platform/theme/motionSpec";
import EngineSummonIcon from "@oa/platform/components/EngineSummonIcon";
import type { RomEntry } from "@oa/platform/library/types";
import type { ThemeEntry, ThemePackage } from "@oa/platform/theme/types";
import type { ThemeManifest } from "@oa/platform/theme/manifest";

type LabRoute = "home" | "library";

// The §2 view-transition spec (authored as manifest data, below).
const LAB_VIEW_SPEC: MotionSpec = {
  duration: 520,
  easing: "cubic-bezier(0.33, 1, 0.68, 1)",
  channels: { opacity: [0, 1], y: [120, 0] },
};

// Ambient breathe for the hero cover (subtle infinite scale pulse).
const LAB_BREATHE_SPEC: MotionSpec = {
  duration: 3000,
  easing: "ease-in-out",
  repeat: "infinite",
  direction: "alternate",
  channels: { scale: [1, 1.025] },
};

const LAB_MANIFEST: ThemeManifest = {
  id: "lab",
  name: "Graphics Lab",
  version: "0.2.0",
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
  // [GRAPHICS-LAB] MOTION: route transition authored as manifest DATA; played by
  // SpecTransition via resolveThemeMotionSpec (the same path a real theme uses).
  motion: { view_transition_spec: LAB_VIEW_SPEC },
};

// Coverflow card geometry.
const CARD_W = 132;
const PITCH = 104; // < CARD_W → cards overlap (depth)
const LAB_CAP = 80; // CarouselNav windows the DOM, but keep the testbed modest

// Lab-local keyframes (self-contained, like the F10 bench). Ken-burns + gloss
// sweep are continuous background treatments (CSS is the right fit); the title/
// meta rise uses the bench's back-ease overshoot.
const LAB_KEYFRAMES = `
@keyframes oa-lab-fade-in {
  from { opacity: 0; }
  to   { opacity: 1; }
}
@keyframes oa-lab-rise {
  from { opacity: 0; transform: translateY(22px); }
  to   { opacity: 1; transform: translateY(0); }
}
@keyframes oa-lab-kenburns {
  0%   { transform: scale(1.18) translate(0, 0); }
  100% { transform: scale(1.32) translate(-2.5%, -2%); }
}
@keyframes oa-lab-gloss {
  0%   { background-position: -160% 0; }
  100% { background-position: 260% 0; }
}
`;
const LAB_RISE_EASE = "cubic-bezier(.34, 1.7, .64, 1)";

const LabEntry: ThemeEntry = (_props) => {
  const platform = usePlatform();
  const host = useTheme();
  const media = useMedia();

  const [route, setRoute] = createSignal<LabRoute>("home");
  const reducedMotion = usePrefersReducedMotion();

  const [focusedIndex, setFocusedIndex] = createSignal(0);
  const [libVisited, setLibVisited] = createSignal(false);
  createEffect(() => {
    if (route() === "library") setLibVisited(true);
  });

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
    return out.slice(0, LAB_CAP);
  });

  const focusedGame = (): RomEntry | undefined => games()[focusedIndex()];

  const coverFor = (e: RomEntry): string | null =>
    (e.identityId ? media.coverUrl(e.systemId, e.identityId) : null) ?? media.coverUrl(e.systemId, e.id);

  const sysName = (e: RomEntry): string => systemThemes[e.systemId]?.shortName ?? e.systemId;
  const metaChips = (e: RomEntry): string[] => {
    const m = media.media(e.identityId ?? e.id)?.metadata;
    const chips: string[] = [];
    if (m?.year) chips.push(String(m.year));
    if (m?.genre) chips.push(m.genre);
    if (m?.players) chips.push(`${m.players}P`);
    if (m?.developer) chips.push(m.developer);
    return chips;
  };

  // Hero cover spring grow-in (M-mod.2) — re-played on every focus change.
  const artScale = createSpringValue(1, BENCH_SELECTION_SPRING, { restDelta: 0.002, restVelocity: 0.004 });
  const heroTilt = useTilt(13);
  createEffect(() => {
    focusedIndex();
    if (reducedMotion()) {
      artScale.snap(1);
      return;
    }
    artScale.snap(0.9);
    artScale.set(1);
  });

  // Fanart backdrop crossfade (audit `fanart-crossfade`): keep a tiny stack of
  // layers; the new cover fades in OVER the old, which is removed after the fade
  // → a true crossfade, no flash. Keyed so each new layer replays its fade-in.
  const [bgLayers, setBgLayers] = createSignal<{ key: number; url: string }[]>([]);
  let bgKey = 0;
  createEffect<string | null>((prevUrl) => {
    const g = focusedGame();
    const url = g ? coverFor(g) : null;
    if (!url || url === prevUrl) return url ?? prevUrl ?? null;
    bgKey += 1;
    const k = bgKey;
    setBgLayers((ls) => [...ls.slice(-1), { key: k, url }]);
    const id = window.setTimeout(() => setBgLayers((ls) => ls.filter((l) => l.key === k)), 900);
    onCleanup(() => clearTimeout(id));
    return url;
  });

  const tab = (id: LabRoute, label: string): JSX.Element => (
    <button
      class="border-b-2 px-4 py-2 text-xs font-semibold uppercase tracking-[0.25em] transition-colors"
      classList={{
        "border-(--color-system-accent) text-white": route() === id,
        "border-transparent text-white/40 hover:text-white/70": route() !== id,
      }}
      onClick={() => setRoute(id)}
    >
      {label}
    </button>
  );

  return (
    // data-system makes --color-system-accent resolve to the FOCUSED game's system
    // colour (S5.2 baseline), so the whole showcase re-themes as you browse.
    <div
      data-system={focusedGame()?.systemId}
      class="relative flex h-full w-full flex-col overflow-hidden bg-[#06070a] text-white"
    >
      <style>{LAB_KEYFRAMES}</style>

      <header class="relative z-20 flex items-center justify-between border-b border-white/10 px-6 py-3">
        <div class="flex items-center gap-1">
          <span class="mr-4 text-sm font-black tracking-tight">
            Graphics Lab{" "}
            <span class="text-[0.55rem] font-normal uppercase tracking-[0.3em] text-amber-400/80">experimental</span>
          </span>
          {tab("home", "Home")}
          {tab("library", "Library")}
        </div>
        <EngineSummonIcon />
      </header>

      {/* [GRAPHICS-LAB] MOTION — the routed region is the SpecTransition host;
          trigger={route} replays LAB_VIEW_SPEC on every Home↔Library swap. */}
      <SpecTransition
        class="relative z-10 min-h-0 flex-1"
        trigger={route}
        spec={() => resolveThemeMotionSpec(LAB_MANIFEST.motion, reducedMotion())}
      >
        {/* HOME */}
        <section
          class="h-full flex-col items-center justify-center gap-4 px-8 text-center"
          classList={{ flex: route() === "home", hidden: route() !== "home" }}
        >
          <h1 class="text-5xl font-black tracking-tight">
            Overlooked <span class="text-(--color-system-accent)">Arcade</span>
          </h1>
          <p class="max-w-xl text-sm leading-relaxed text-white/55">
            Graphics Lab — the cinematic showcase. Switch to <b class="text-white/80">Library</b> for
            a fanart backdrop, spring-driven box-art, and a coverflow carousel, all from the
            declarative motion engine. Nothing here ships; it's where the look gets built.
          </p>
        </section>

        {/* LIBRARY — the cinematic showcase */}
        <Show when={libVisited()}>
          <div
            class="relative h-full overflow-hidden"
            classList={{ block: route() === "library", hidden: route() !== "library" }}
          >
            {/* Fanart backdrop: blurred focused cover, ken-burns, crossfading. */}
            <div class="pointer-events-none absolute inset-0 bg-[#06070a]">
              <For each={bgLayers()}>
                {(layer) => (
                  <div
                    class="absolute inset-0 bg-cover bg-center"
                    style={{
                      "background-image": `url("${layer.url}")`,
                      filter: "blur(48px) saturate(1.3) brightness(0.42)",
                      animation: `oa-lab-fade-in 900ms ease forwards${
                        reducedMotion() ? "" : ", oa-lab-kenburns 26s ease-in-out infinite alternate"
                      }`,
                    }}
                  />
                )}
              </For>
              {/* legibility scrim + accent wash */}
              <div class="absolute inset-0 bg-gradient-to-t from-[#06070a] via-[#06070a]/70 to-[#06070a]/30" />
              <div
                class="absolute inset-0 opacity-30"
                style={{
                  "background-image":
                    "radial-gradient(120% 80% at 15% 30%, var(--color-system-accent), transparent 55%)",
                }}
              />
            </div>

            <Show
              when={focusedGame()}
              fallback={
                <div class="relative grid h-full place-items-center text-sm text-white/40">No games in library</div>
              }
            >
              {/* foreground: hero (fills) + carousel (bottom) */}
              <div class="relative grid h-full grid-rows-[1fr_auto]">
                {/* HERO */}
                <div class="flex min-h-0 items-center gap-12 px-14">
                  {/* hero box-art: tilt > breathe > spring(cover + gloss) + reflection */}
                  <Show when={focusedGame()} keyed>
                    {(g) => (
                      <div class="relative shrink-0">
                        <div
                          style={{ transform: heroTilt.transform(), transition: "transform 160ms ease-out" }}
                          onPointerMove={heroTilt.onPointerMove}
                          onPointerLeave={heroTilt.onPointerLeave}
                        >
                          <AmbientMotion spec={() => LAB_BREATHE_SPEC} reducedMotion={reducedMotion}>
                            <div style={{ transform: `scale(${artScale.value()})`, "transform-origin": "bottom center" }}>
                              <div
                                class="relative aspect-[3/4] w-56 overflow-hidden rounded-xl"
                                style={{
                                  "box-shadow":
                                    "0 30px 60px rgba(0,0,0,.65), 0 0 70px -10px var(--color-system-accent)",
                                }}
                              >
                                <Show
                                  when={coverFor(g)}
                                  fallback={
                                    <div class="grid h-full w-full place-items-center bg-white/[0.06] p-4 text-center text-xs text-white/60">
                                      {g.title}
                                    </div>
                                  }
                                >
                                  {(u) => <img src={u()} alt="" class="h-full w-full object-cover" />}
                                </Show>
                                {/* gloss: frost edge + moving specular sweep */}
                                <div
                                  class="pointer-events-none absolute inset-0 rounded-xl"
                                  style={{
                                    "background-image":
                                      "linear-gradient(135deg, rgba(255,255,255,.28), rgba(255,255,255,0) 38%, rgba(255,255,255,.04) 60%, rgba(255,255,255,.18))",
                                    "box-shadow": "inset 0 1px 0 rgba(255,255,255,.4), inset 0 0 0 1px rgba(255,255,255,.12)",
                                  }}
                                />
                                <div
                                  class="pointer-events-none absolute inset-0 rounded-xl"
                                  style={{
                                    "background-image":
                                      "linear-gradient(110deg, transparent 38%, rgba(255,255,255,.45) 50%, transparent 62%)",
                                    "background-size": "250% 100%",
                                    animation: reducedMotion() ? "none" : "oa-lab-gloss 4.5s linear infinite",
                                  }}
                                />
                              </div>
                              {/* grounded reflection */}
                              <div class="pointer-events-none absolute left-0 top-full h-24 w-56 overflow-hidden" aria-hidden="true">
                                <Show when={coverFor(g)}>
                                  {(u) => (
                                    <img
                                      src={u()}
                                      alt=""
                                      class="w-56 -scale-y-100 object-cover"
                                      style={{
                                        opacity: "0.28",
                                        "mask-image": "linear-gradient(to bottom, rgba(0,0,0,.7), transparent 70%)",
                                        "-webkit-mask-image": "linear-gradient(to bottom, rgba(0,0,0,.7), transparent 70%)",
                                      }}
                                    />
                                  )}
                                </Show>
                              </div>
                            </div>
                          </AmbientMotion>
                        </div>
                      </div>
                    )}
                  </Show>

                  {/* hero text — staggered entrance, system-accent themed */}
                  <Show when={focusedGame()} keyed>
                    {(g) => (
                      <div class="flex min-w-0 flex-col gap-4">
                        <span
                          class="text-xs font-bold uppercase tracking-[0.4em] text-(--color-system-accent)"
                          style={{ animation: `oa-lab-rise 420ms ${LAB_RISE_EASE} both` }}
                        >
                          {sysName(g)}
                        </span>
                        <h2
                          class="max-w-2xl text-5xl font-black leading-[1.05] tracking-tight drop-shadow-[0_2px_20px_rgba(0,0,0,.6)]"
                          style={{ animation: `oa-lab-rise 460ms ${LAB_RISE_EASE} 60ms both` }}
                        >
                          {g.title}
                        </h2>
                        <div class="flex flex-wrap gap-2">
                          <For each={metaChips(g)}>
                            {(chip, i) => (
                              <span
                                class="rounded-full border border-white/20 bg-white/10 px-3.5 py-1 text-[0.65rem] font-medium uppercase tracking-wider text-white/85 backdrop-blur-sm"
                                style={{ animation: `oa-lab-rise 420ms ${LAB_RISE_EASE} ${160 + i() * 70}ms both` }}
                              >
                                {chip}
                              </span>
                            )}
                          </For>
                        </div>
                        <div
                          class="mt-1 h-1 w-24 rounded-full bg-(--color-system-accent)"
                          style={{ animation: `oa-lab-rise 500ms ${LAB_RISE_EASE} 320ms both` }}
                        />
                      </div>
                    )}
                  </Show>
                </div>

                {/* COVERFLOW CARROUSEL — the browser */}
                <div class="relative pb-8 pt-2">
                  <CarouselNav
                    id="lab-carousel"
                    items={games}
                    focusedIndex={focusedIndex}
                    setFocusedIndex={setFocusedIndex}
                    cardWidth={CARD_W}
                    pitch={PITCH}
                    focusedScale={1}
                    sideScale={0.74}
                    window={9}
                    hints={{ dpad: "Browse", stick: "Browse", Confirm: "Launch", Secondary: "Details" }}
                    onConfirm={(_i, entry) => void host.onLaunch(entry)}
                    onSecondary={(_i, entry) => host.onShowInfo(entry)}
                  >
                    {(entry, ctx) => {
                      const cover = coverFor(entry);
                      return (
                        <div
                          class="aspect-[3/4] w-full overflow-hidden rounded-lg transition-shadow duration-200"
                          classList={{ "ring-2 ring-(--color-system-accent)": ctx.focused() }}
                          style={{
                            "box-shadow": ctx.focused()
                              ? "0 16px 36px rgba(0,0,0,.6), 0 0 40px -8px var(--color-system-accent)"
                              : "0 8px 20px rgba(0,0,0,.5)",
                          }}
                        >
                          <Show
                            when={cover}
                            fallback={
                              <div class="grid h-full w-full place-items-center bg-white/[0.06] p-2 text-center text-[0.55rem] text-white/55">
                                {entry.title}
                              </div>
                            }
                          >
                            {(u) => <img src={u()} alt="" class="h-full w-full object-cover" />}
                          </Show>
                        </div>
                      );
                    }}
                  </CarouselNav>
                </div>
              </div>
            </Show>
          </div>
        </Show>
      </SpecTransition>
    </div>
  );
};

// [GRAPHICS-LAB] `experimental: true` keeps it out of the Appearance picker while
// still registered/valid/activatable (Settings → Experimental → Graphics Lab).
export const lab: ThemePackage = {
  manifest: LAB_MANIFEST,
  entry: LabEntry,
  experimental: true,
};
