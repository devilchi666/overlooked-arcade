// Wheel — the second whole-shell theme (the rough pilot proving a DIFFERENT
// IA, not a reskin).
//
// Theming Substrate ARC 1 Phase 3 S2 (the walking skeleton / swap gate,
// docs/PLANS/theming-substrate.md §13.3). Where Retroverse is a tabbed
// 3-pane launcher, the Wheel is a full-bleed horizontal COVERFLOW: one
// centred, scaled focused cover with neighbours fanning out, a metadata
// strip below, Left/Right to browse, Confirm to launch.
//
// HONEST CAVEAT (S2): this is layout + a distinct feel only. The CINEMATIC
// layer — attract mode, CRT ceremony, launch shaders, per-shell sound — is
// ARC 2-3 (DECISIONS D20). The Wheel's job here is to prove swappability +
// distinct identity early, not to be the finished article.
//
// BOUNDARY: a theme consumes ONLY the platform layer. This file imports from
// @oa/platform/* (stores via usePlatform, host services via useTheme, the
// ListNav primitive + verbs, media, the per-system registry) and the engine
// summon icon (now a platform component) — never engine/, routes/, or
// another theme. System-AGNOSTIC by choice (DECISIONS D19): the Wheel treats
// every system identically; per-system identity is Retroverse's take, not a
// substrate requirement.

import { createEffect, createMemo, createSignal, Show } from "solid-js";
import { usePlatform } from "@oa/platform/platformContext";
import { useTheme } from "@oa/platform/theme/host";
import { ListNav } from "@oa/platform/nav";
import { useMedia } from "@oa/platform/library/media";
import { systemThemes } from "@oa/platform/themes/registry";
import EngineSummonIcon from "@oa/platform/components/EngineSummonIcon";
import type { RomEntry } from "@oa/platform/library/types";
import type { ThemeEntry, ThemePackage } from "@oa/platform/theme/types";
import type { ThemeManifest } from "@oa/platform/theme/manifest";

// Layout constants — fixed card footprint + how far apart neighbours fan.
// PITCH < CARD_W so covers overlap slightly (the coverflow look). WINDOW
// bounds how many cards either side of the focused one actually render their
// cover (keeps a large library from mounting thousands of <img>s — far cards
// collapse to an empty node; the focus indices stay intact because ListNav
// still iterates the full list).
const CARD_W = 210;
const PITCH = 168;
const WINDOW = 7;

const WHEEL_MANIFEST: ThemeManifest = {
  id: "wheel",
  name: "Wheel",
  version: "0.1.0",
  schema_version: 1,
  oa_version: "^0.x",
  entry: "./index.tsx",
  entry_export: "wheel",
  default_route: "library",
  routes: ["library"],
  context_slots: ["library", "settings"],
  required_engine_capabilities: [],
  reserves_corner: "top-right",
  surfaces: ["main"],
};

const WheelEntry: ThemeEntry = (_props) => {
  const platform = usePlatform();
  const host = useTheme();
  const media = useMedia();

  // The browse list: every real (non-seed) game, deduped to one cover per
  // identity (so multi-disc / multi-region variants don't each get a card),
  // sorted by title. Rough + system-agnostic (D19) — no per-system grouping.
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

  const [focusedIndex, setFocusedIndex] = createSignal(0);

  // Clamp focus when the list shrinks/grows so we never point past the end.
  createEffect(() => {
    const n = games().length;
    if (n === 0) return;
    if (focusedIndex() > n - 1) setFocusedIndex(n - 1);
  });

  // Mirror the focused cover into the platform's shared selection state so
  // the rest of OA (engine surface, future right-pane widgets) knows the
  // pick — same contract Retroverse's grid feeds via setFocusedEntry.
  createEffect(() => {
    const g = games()[focusedIndex()] ?? null;
    platform.setFocusedEntry(g);
  });

  const focusedGame = () => games()[focusedIndex()] ?? null;

  // Cover resolution mirrors LibraryTile: prefer the identity-key art, fall
  // back to the per-file key. Reactive via the MediaContext store.
  const coverFor = (entry: RomEntry): string | null =>
    (entry.identityId ? media.coverUrl(entry.systemId, entry.identityId) : null) ??
    media.coverUrl(entry.systemId, entry.id);

  const sysShortName = (entry: RomEntry): string =>
    systemThemes[entry.systemId]?.shortName ?? entry.systemId;

  return (
    <div class="relative grid h-full w-full grid-rows-[64px_minmax(0,1fr)_auto] overflow-hidden bg-(--color-oa-bg-deep) text-(--color-oa-ink)">
      {/* Scoped coverflow CSS. Each .oa-list-nav-item is stacked at the track
          centre; the per-card transform (offset fan + scale) lives inline on
          the card so it animates on focus change without touching any
          ListNav-owned element's style. */}
      <style>{`
        .oa-wheel-track { position: relative; height: 100%; width: 100%; }
        .oa-wheel-track > .oa-list-nav-item {
          position: absolute; top: 50%; left: 50%;
          outline: none;
        }
        .oa-wheel-card {
          position: absolute; top: 0; left: 0;
          width: ${CARD_W}px;
          transition: transform 280ms cubic-bezier(.22,.61,.36,1), opacity 280ms ease;
          will-change: transform, opacity;
          transform-origin: center center;
        }
      `}</style>

      {/* Top bar — Wheel brand + the engine summon icon (D3: the theme
          reserves the top-right slot for it; it's the always-available path
          back to Settings → Appearance to switch themes). */}
      <header class="z-10 flex items-center justify-between border-b border-white/5 bg-(--color-oa-bg-deep)/95 px-6 backdrop-blur">
        <div class="flex items-center gap-2">
          <span class="text-lg text-(--color-system-accent)">◎</span>
          <div class="leading-tight">
            <p class="text-sm font-semibold uppercase tracking-[0.3em]">Wheel</p>
            <p class="text-[0.55rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
              Coverflow · {games().length} games
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
                add a folder, then come back to the Wheel.
              </p>
            </div>
          </div>
        }
      >
        <div class="relative min-h-0">
          <ListNav
            id="wheel-coverflow"
            class="oa-wheel-track"
            orientation="horizontal"
            items={games}
            focusedIndex={focusedIndex}
            setFocusedIndex={setFocusedIndex}
            focusProminence="scale"
            easing="emphasized"
            hints={{
              dpad: "Browse",
              stick: "Browse",
              Confirm: "Launch",
              Secondary: "Game info",
            }}
            onConfirm={(_i, item) => void host.onLaunch(item)}
            onSecondary={(_i, item) => host.onShowInfo(item)}
          >
            {(item, ctx) => {
              const offset = (): number => ctx.index - focusedIndex();
              const near = (): boolean => Math.abs(offset()) <= WINDOW;
              // Fan the cards out from centre; focused card sits dead-centre
              // at full scale + opacity, neighbours recede + dim.
              const transform = (): string => {
                const o = offset();
                const scale = ctx.focused() ? 1 : 0.74;
                return `translate(-50%, -50%) translateX(${o * PITCH}px) scale(${scale})`;
              };
              const opacity = (): number => {
                const a = Math.abs(offset());
                if (a === 0) return 1;
                return Math.max(0.25, 1 - a * 0.16);
              };
              return (
                <Show when={near()}>
                  <div
                    class="oa-wheel-card"
                    data-system={item.systemId}
                    style={{
                      transform: transform(),
                      opacity: opacity(),
                      "z-index": String(100 - Math.abs(offset())),
                    }}
                  >
                    <div
                      class="relative aspect-[3/4] w-full overflow-hidden rounded-xl border bg-(--color-oa-bg-deep) shadow-2xl shadow-black/60"
                      classList={{
                        "border-(--color-system-accent) ring-2 ring-(--color-system-accent)/60":
                          ctx.focused(),
                        "border-white/10": !ctx.focused(),
                      }}
                    >
                      <Show
                        when={coverFor(item)}
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
                            alt={item.title}
                            class="absolute inset-0 h-full w-full object-contain"
                            loading="lazy"
                            decoding="async"
                          />
                        )}
                      </Show>
                    </div>
                  </div>
                </Show>
              );
            }}
          </ListNav>
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

export const wheel: ThemePackage = {
  manifest: WHEEL_MANIFEST,
  entry: WheelEntry,
};
