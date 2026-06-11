// CoverFlow — the second whole-shell theme (the rough pilot proving a
// DIFFERENT IA, not a reskin).
//
// Theming Substrate ARC 1 Phase 3 S2 (the walking skeleton / swap gate,
// docs/PLANS/theming-substrate.md §13.3). Where Retroverse is a tabbed
// 3-pane launcher, CoverFlow is a full-bleed horizontal coverflow: a centred,
// scaled focused cover with neighbours fanning out on a sliding track, a
// metadata strip below, Left/Right to browse, Confirm to launch.
//
// S5.5: CoverFlow now consumes the `CarouselNav` PRIMITIVE instead of its own
// hand-rolled windowed track — the primitive owns the windowing / track shift /
// per-card layout / focus / wheel-browse / click-to-centre, and CoverFlow keeps
// only the theme-specific parts (the cover content, the preload buffer, the
// metadata footer, the shared-selection mirror). It also mounts `ThemeBackground`
// (the revived S5.1 background tier) and wires `onNavSound` (the #6 nav-sound
// hook) — so a theme built on the primitive gets browse SFX for free.
//
// (Renamed from "Wheel" → "CoverFlow" 2026-06-10 per operator: what this proves
// is a coverflow IA; a true radial/arc Wheel is the reserved `WheelNav` contract,
// deferred until a wheel-using theme lands.)
//
// HONEST CAVEAT (S2): this is layout + a distinct feel only. The CINEMATIC layer
// — attract mode, CRT ceremony, launch shaders — is ARC 2-3 (DECISIONS D20).
//
// BOUNDARY: a theme consumes ONLY platform — usePlatform stores, the host
// services (useTheme), the nav primitives, media, the per-system registry, and
// the platform-homed leaves (EngineSummonIcon / ThemeBackground). Never engine/,
// routes/, or another theme. System-AGNOSTIC by choice (DECISIONS D19).

import { createEffect, createMemo, createSignal, Show } from "solid-js";
import { usePlatform } from "@oa/platform/platformContext";
import { useTheme } from "@oa/platform/theme/host";
import { CarouselNav } from "@oa/platform/nav";
import { navSoundDispatcher } from "@oa/platform/themes/systemUiSound";
import { useMedia } from "@oa/platform/library/media";
import { systemThemes } from "@oa/platform/themes/registry";
import EngineSummonIcon from "@oa/platform/components/EngineSummonIcon";
import ThemeBackground from "@oa/platform/components/ThemeBackground";
import type { RomEntry } from "@oa/platform/library/types";
import type { ThemeEntry, ThemePackage } from "@oa/platform/theme/types";
import type { ThemeManifest } from "@oa/platform/theme/manifest";

// Card footprint + spacing. PITCH < CARD_W so covers overlap (the coverflow
// look). WINDOW = how many cards either side of focus the primitive keeps in the
// DOM. PRELOAD = a wider cover-warming range (theme-level optimisation).
const CARD_W = 210;
const PITCH = 168;
const WINDOW = 8;
const PRELOAD = 24;

const COVERFLOW_MANIFEST: ThemeManifest = {
  id: "coverflow",
  name: "CoverFlow",
  version: "0.1.0",
  schema_version: 1,
  oa_version: "^0.x",
  entry: "./index.tsx",
  entry_export: "coverflow",
  default_route: "library",
  routes: ["library"],
  context_slots: ["library", "settings"],
  required_engine_capabilities: [],
  reserves_corner: "top-right",
  surfaces: ["main"],
};

const CoverFlowEntry: ThemeEntry = (_props) => {
  const platform = usePlatform();
  const host = useTheme();
  const media = useMedia();

  // Browse list: every real (non-seed) game, deduped to one cover per identity
  // (multi-disc / multi-region variants collapse), sorted by title. Stable
  // RomEntry refs so the primitive's windowed <For> reconciles.
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

  const [focusedIndex, setFocusedIndexRaw] = createSignal(0);
  const setFocusedIndex = (n: number): void => {
    const len = games().length;
    if (len === 0) return;
    setFocusedIndexRaw(Math.max(0, Math.min(len - 1, n)));
  };

  // Clamp focus when the list shrinks.
  createEffect(() => {
    const n = games().length;
    if (n > 0 && focusedIndex() > n - 1) setFocusedIndexRaw(n - 1);
  });

  // Mirror the focused cover into the platform's shared selection state (same
  // contract Retroverse's grid feeds via setFocusedEntry).
  createEffect(() => {
    platform.setFocusedEntry(games()[focusedIndex()] ?? null);
  });

  const focusedGame = (): RomEntry | null => games()[focusedIndex()] ?? null;

  const coverFor = (entry: RomEntry): string | null =>
    (entry.identityId ? media.coverUrl(entry.systemId, entry.identityId) : null) ??
    media.coverUrl(entry.systemId, entry.id);

  // Image-preload buffer for smooth FAST scroll: warm covers a wider ±PRELOAD
  // range than the primitive's render window into the browser cache via off-DOM
  // Image() loads, so a card mounting as we scroll is a cache hit (no flash).
  // Deduped by URL; no retained Image refs (browser resource cache governs).
  const prefetched = new Set<string>();
  createEffect(() => {
    const list = games();
    if (list.length === 0) return;
    const f = focusedIndex();
    const lo = Math.max(0, f - PRELOAD);
    const hi = Math.min(list.length - 1, f + PRELOAD);
    for (let i = lo; i <= hi; i++) {
      const src = coverFor(list[i]!);
      if (!src || prefetched.has(src)) continue;
      prefetched.add(src);
      const img = new Image();
      img.decoding = "async";
      img.src = src;
    }
  });

  const sysShortName = (entry: RomEntry): string =>
    systemThemes[entry.systemId]?.shortName ?? entry.systemId;

  return (
    <div class="relative grid h-full w-full grid-rows-[64px_minmax(0,1fr)_auto] overflow-hidden bg-(--color-oa-bg-deep) text-(--color-oa-ink)">
      {/* Top bar — CoverFlow brand + the engine summon icon (D3). */}
      <header class="z-10 flex items-center justify-between border-b border-white/5 bg-(--color-oa-bg-deep)/95 px-6 backdrop-blur">
        <div class="flex items-center gap-2">
          <span class="text-lg text-(--color-system-accent)">◎</span>
          <div class="leading-tight">
            <p class="text-sm font-semibold uppercase tracking-[0.3em]">CoverFlow</p>
            <p class="text-[0.55rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
              {games().length} games
            </p>
          </div>
        </div>
        <EngineSummonIcon />
      </header>

      {/* Coverflow body. */}
      <Show
        when={games().length > 0}
        fallback={
          <div class="grid place-items-center px-8 text-center">
            <div>
              <p class="text-sm uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                No games yet
              </p>
              <p class="mt-3 max-w-md text-sm text-(--color-oa-ink-dim)">
                Open Settings (⚙ top-right · F12 · Select+Start) → Library to add a
                folder, then come back to CoverFlow.
              </p>
            </div>
          </div>
        }
      >
        <div class="relative min-h-0">
          {/* S5.5: the revived theme-owned backdrop, consuming the S5.1 cascade.
              CoverFlow ships no backdrop by default → resolves the platform
              per-system / _baseline asset (or nothing). Drop
              assets/themes/coverflow/system-ui/_baseline/backgrounds/default.png
              for one cohesive theme-wide backdrop. */}
          <ThemeBackground systemId={() => focusedGame()?.systemId ?? null} opacity={0.4} />

          {/* The carousel primitive owns windowing / track / focus / wheel /
              click; CoverFlow supplies only the card content + the visual config.
              System-AGNOSTIC (D19): no data-system → every card uses the THEME's
              own accent token, one cohesive identity (not a per-system rainbow). */}
          <CarouselNav
            id="coverflow"
            class="absolute inset-0"
            items={games}
            focusedIndex={focusedIndex}
            setFocusedIndex={setFocusedIndex}
            cardWidth={CARD_W}
            pitch={PITCH}
            window={WINDOW}
            sideScale={0.78}
            minOpacity={0.25}
            opacityFalloff={0.16}
            hints={{ dpad: "Browse", stick: "Browse", Confirm: "Launch", Secondary: "Game info" }}
            onConfirm={(_i, g) => void host.onLaunch(g)}
            onSecondary={(_i, g) => host.onShowInfo(g)}
            onNavSound={navSoundDispatcher<RomEntry>((g) => g?.systemId)}
          >
            {(entry, ctx) => (
              <div
                class="relative aspect-[3/4] w-full overflow-hidden rounded-xl border bg-(--color-oa-bg-deep) shadow-2xl shadow-black/60"
                classList={{
                  "border-(--color-system-accent) ring-2 ring-(--color-system-accent)/60":
                    ctx.focused(),
                  "border-white/10": !ctx.focused(),
                }}
              >
                <Show
                  when={coverFor(entry)}
                  fallback={
                    // Subtle missing-art placeholder — a faint theme-accent haze
                    // over the deep surface + the title, so art-less cards recede.
                    <div
                      class="absolute inset-0 flex items-end p-3"
                      style={{
                        background:
                          "radial-gradient(ellipse 130% 70% at 50% -10%, var(--color-system-glow), transparent 70%), var(--color-oa-bg-deep)",
                      }}
                    >
                      <span class="line-clamp-3 text-[0.7rem] font-medium leading-tight text-(--color-oa-ink-dim)">
                        {entry.title}
                      </span>
                    </div>
                  }
                >
                  {(src) => (
                    <img
                      src={src()}
                      alt={entry.title}
                      class="absolute inset-0 h-full w-full object-contain"
                      loading="lazy"
                      decoding="async"
                    />
                  )}
                </Show>
              </div>
            )}
          </CarouselNav>
        </div>
      </Show>

      {/* Metadata strip for the focused game. */}
      <footer class="z-10 border-t border-white/5 bg-(--color-oa-bg-deep)/95 px-8 py-5 backdrop-blur">
        <Show when={focusedGame()} fallback={<p class="text-sm text-(--color-oa-ink-dim)">—</p>}>
          {(g) => (
            // No data-system: the system NAME shows as text, but its colour stays
            // the theme accent — CoverFlow's uniform identity (D19).
            <div class="flex items-end justify-between gap-6">
              <div class="min-w-0">
                <p class="text-[0.6rem] uppercase tracking-[0.5em] text-(--color-system-accent)">
                  {sysShortName(g())}
                </p>
                <h1 class="mt-1 truncate text-2xl font-semibold text-(--color-oa-ink)">
                  {g().title}
                </h1>
              </div>
              <button
                type="button"
                onClick={() => void host.onLaunch(g())}
                class="shrink-0 rounded-lg border border-(--color-system-accent)/60 bg-(--color-system-accent)/15 px-5 py-2.5 text-sm font-semibold uppercase tracking-widest text-(--color-oa-ink) transition hover:bg-(--color-system-accent)/25 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
              >
                ▶ Launch
              </button>
            </div>
          )}
        </Show>
      </footer>
    </div>
  );
};

export const coverflow: ThemePackage = {
  manifest: COVERFLOW_MANIFEST,
  entry: CoverFlowEntry,
  // S3 token override (minimal-but-distinct, to prove the mechanism): a cool
  // steel-blue / cyan "cinematic" identity vs Retroverse's warm default. Injected
  // scoped to the theme-mount wrapper (App.tsx), so swapping to CoverFlow visibly
  // recolours the shell chrome with ZERO code change — same component, different
  // tokens.
  tokens: {
    bgDeep: "oklch(0.12 0.035 250)",
    bg: "oklch(0.16 0.04 250)",
    accent: "oklch(0.80 0.13 225)",
    accentSoft: "oklch(0.91 0.05 225)",
    accentGlow: "oklch(0.80 0.13 225 / 0.22)",
    focusRing: "oklch(0.80 0.13 225)",
  },
};
