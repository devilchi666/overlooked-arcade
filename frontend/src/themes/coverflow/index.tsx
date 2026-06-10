// CoverFlow — the second whole-shell theme (the rough pilot proving a
// DIFFERENT IA, not a reskin).
//
// Theming Substrate ARC 1 Phase 3 S2 (the walking skeleton / swap gate,
// docs/PLANS/theming-substrate.md §13.3). Where Retroverse is a tabbed
// 3-pane launcher, CoverFlow is a full-bleed horizontal coverflow: a centred,
// scaled focused cover with neighbours fanning out on a sliding track, a
// metadata strip below, Left/Right to browse, Confirm to launch.
//
// (Renamed from "Wheel" → "CoverFlow" 2026-06-10 per operator: what this
// proves is a coverflow IA; a true radial/arc Wheel is the separate `wheel`
// nav primitive, parked for S5.)
//
// HONEST CAVEAT (S2): this is layout + a distinct feel only. The CINEMATIC
// layer — attract mode, CRT ceremony, launch shaders, per-shell sound — is
// ARC 2-3 (DECISIONS D20). CoverFlow proves swappability + distinct identity
// early, not the finished article.
//
// Nav: built directly on `useFocusGroup` (the lower-level platform nav hook),
// NOT the ListNav primitive — a coverflow over a multi-thousand-game library
// needs windowed rendering, exactly like VirtualLibraryGrid uses the hook +
// its own virtualization rather than ListNav (which renders every row). Only
// a ±WINDOW slice of cards is in the DOM; the track slides via CSS transform.
//
// BOUNDARY: a theme consumes ONLY platform — usePlatform stores, the host
// services (useTheme), the nav hook, media, the per-system registry, and the
// platform-homed EngineSummonIcon. Never engine/, routes/, or another theme.
// System-AGNOSTIC by choice (DECISIONS D19).

import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import { usePlatform } from "@oa/platform/platformContext";
import { useTheme } from "@oa/platform/theme/host";
import { useFocusGroup, HintRegion } from "@oa/platform/nav";
import { useMedia } from "@oa/platform/library/media";
import { systemThemes } from "@oa/platform/themes/registry";
import EngineSummonIcon from "@oa/platform/components/EngineSummonIcon";
import type { RomEntry } from "@oa/platform/library/types";
import type { ThemeEntry, ThemePackage } from "@oa/platform/theme/types";
import type { ThemeManifest } from "@oa/platform/theme/manifest";

// Card footprint + spacing. PITCH < CARD_W so covers overlap (the coverflow
// look). WINDOW = how many cards either side of focus are kept in the DOM.
const CARD_W = 210;
const PITCH = 168;
const WINDOW = 8;

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
  // (multi-disc / multi-region variants collapse), sorted by title. The
  // returned RomEntry refs are stable across renders (same game = same object
  // from the store), so the windowed <For> below reconciles instead of
  // remounting as the window slides.
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

  const [focusedIndexRaw, setFocusedIndexRaw] = createSignal(0);
  const focusedIndex = focusedIndexRaw;
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

  const group = useFocusGroup({
    id: "coverflow",
    orientation: "horizontal",
    itemCount: () => games().length,
    focusedIndex,
    setFocusedIndex,
    onActivate: (i) => {
      const g = games()[i];
      if (g) void host.onLaunch(g);
    },
    onSecondary: (i) => {
      const g = games()[i];
      if (g) host.onShowInfo(g);
    },
  });

  // Force-claim once the list is populated. useFocusGroup auto-claims on mount
  // only when nothing is already active; this theme mounts LATE (gated on the
  // async active-theme seed), so a stray earlier group could hold the active
  // slot. Claiming when games first appear makes CoverFlow reliably own input.
  let claimed = false;
  createEffect(() => {
    if (!claimed && games().length > 0) {
      claimed = true;
      group.activate();
    }
  });

  // Windowed slice — only ±WINDOW cards around focus exist in the DOM. The
  // slice holds stable RomEntry refs so <For> reuses staying cards; the track
  // (not the cards) slides as focus moves.
  const lo = createMemo(() => Math.max(0, focusedIndex() - WINDOW));
  const visibleEntries = createMemo(() => {
    const list = games();
    const hi = Math.min(list.length - 1, focusedIndex() + WINDOW);
    return list.slice(lo(), hi + 1);
  });

  const focusedGame = () => games()[focusedIndex()] ?? null;

  const coverFor = (entry: RomEntry): string | null =>
    (entry.identityId ? media.coverUrl(entry.systemId, entry.identityId) : null) ??
    media.coverUrl(entry.systemId, entry.id);

  const sysShortName = (entry: RomEntry): string =>
    systemThemes[entry.systemId]?.shortName ?? entry.systemId;

  // Mouse wheel = browse (nice for a coverflow, and usable even if controller
  // nav is toggled off). Click handling is per-card below.
  const onWheelScroll = (e: WheelEvent): void => {
    e.preventDefault();
    setFocusedIndex(focusedIndex() + (e.deltaY > 0 ? 1 : -1));
    group.activate();
  };

  // Track transform centres the focused card: shift the track so the focused
  // card's centre sits at the container's horizontal centre (50%).
  const trackTransform = (): string =>
    `translateX(calc(50% - ${focusedIndex() * PITCH + CARD_W / 2}px))`;

  return (
    <div class="relative grid h-full w-full grid-rows-[64px_minmax(0,1fr)_auto] overflow-hidden bg-(--color-oa-bg-deep) text-(--color-oa-ink)">
      <style>{`
        .oa-cf-track {
          position: absolute; inset: 0;
          transition: transform 260ms cubic-bezier(.22,.61,.36,1);
          will-change: transform;
        }
        .oa-cf-card {
          position: absolute; top: 50%;
          width: ${CARD_W}px;
          transition: transform 260ms cubic-bezier(.22,.61,.36,1), opacity 260ms ease;
          will-change: transform, opacity;
        }
      `}</style>

      {/* Top bar — CoverFlow brand + the engine summon icon (D3: theme
          reserves the top-right slot; the always-available path back to
          Settings → Themes to switch shells). */}
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
                Open Settings (⚙ top-right · F12 · Select+Start) → Library to
                add a folder, then come back to CoverFlow.
              </p>
            </div>
          </div>
        }
      >
        <div
          class="relative min-h-0 overflow-hidden"
          onWheel={onWheelScroll}
        >
          <HintRegion
            hints={{
              dpad: "Browse",
              stick: "Browse",
              Confirm: "Launch",
              Secondary: "Game info",
            }}
          />
          <div class="oa-cf-track" style={{ transform: trackTransform() }}>
            <For each={visibleEntries()}>
              {(entry, i) => {
                const absIndex = (): number => lo() + i();
                const offset = (): number => absIndex() - focusedIndex();
                const isFocused = (): boolean => offset() === 0;
                return (
                  <div
                    class="oa-cf-card"
                    data-system={entry.systemId}
                    style={{
                      left: `${absIndex() * PITCH}px`,
                      transform: `translateY(-50%) scale(${isFocused() ? 1 : 0.78})`,
                      opacity: String(Math.max(0.25, 1 - Math.abs(offset()) * 0.16)),
                      "z-index": String(100 - Math.abs(offset())),
                    }}
                    onClick={() => {
                      // Click a side cover → centre it; click the centred
                      // cover → launch. Keeps CoverFlow mouse-usable even
                      // without controller nav.
                      if (isFocused()) void host.onLaunch(entry);
                      else {
                        setFocusedIndex(absIndex());
                        group.activate();
                      }
                    }}
                  >
                    <div
                      class="relative aspect-[3/4] w-full overflow-hidden rounded-xl border bg-(--color-oa-bg-deep) shadow-2xl shadow-black/60"
                      classList={{
                        "border-(--color-system-accent) ring-2 ring-(--color-system-accent)/60":
                          isFocused(),
                        "border-white/10": !isFocused(),
                      }}
                    >
                      <Show
                        when={coverFor(entry)}
                        fallback={
                          <div
                            class="absolute inset-0"
                            style={{
                              background:
                                "radial-gradient(circle at 30% 25%, var(--color-system-glow), transparent 60%), linear-gradient(135deg, var(--color-system-accent) 0%, var(--color-oa-bg-deep) 100%)",
                            }}
                          />
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
                  </div>
                );
              }}
            </For>
          </div>
        </div>
      </Show>

      {/* Metadata strip for the focused game. */}
      <footer class="z-10 border-t border-white/5 bg-(--color-oa-bg-deep)/95 px-8 py-5 backdrop-blur">
        <Show
          when={focusedGame()}
          fallback={<p class="text-sm text-(--color-oa-ink-dim)">—</p>}
        >
          {(g) => (
            <div
              class="flex items-end justify-between gap-6"
              data-system={g().systemId}
            >
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
};
