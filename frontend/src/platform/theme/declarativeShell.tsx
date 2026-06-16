// DeclarativeShell — the one built-in shell that renders EVERY declarative
// (`.oatheme`) theme (Theming ARC 2 "P" P.1 S2; decision D47/PD3).
//
// A declarative theme ships ZERO code — just data (`theme.toml` + optional
// `tokens.toml` / `per-system.toml`, mapped to a `ThemePackage` by
// `diskTheme.ts::diskThemeToPackage`). This component is the engine-provided
// `ThemeEntry` that those packages all point at: it reads the ACTIVE theme's
// manifest and renders a single browse surface by interpreting it —
//
//   • layout: the per-view layout primitive resolved via the ARC 2 L3 machinery
//     (`useResolvedLayout`), mounting the matching nav primitive
//     (List/Grid/Carousel/Wheel — all already built; we compose, never rebuild).
//   • palette/typography/geometry: NOTHING to do here — App's `.oa-theme-mount`
//     wrapper already injects the theme's `tokens` + `perSystemTokens` as scoped
//     CSS vars (D23/D2). Cards carry `data-system` so the per-system accent
//     resolves automatically.
//   • glyph set: NOTHING to do here — App bridges `manifest.glyph_set` to the
//     HintBar; we just publish verb hints on the primitive.
//   • background: `ThemeBackground`, keyed to the focused game's system (S5.1
//     cascade; renders nothing when a theme ships no assets — so a bare theme
//     stays bare).
//   • settings: declared in `manifest.settings_schema`, rendered generically by
//     the engine Appearance panel + persisted per-theme. This shell interprets a
//     small RECOGNIZED vocabulary of those keys that affect its own layout (see
//     RECOGNIZED_SETTINGS below); unknown keys still render + persist but are
//     inert in the generic shell (accrete additively — D47 / plan open-question).
//
// HONEST CEILING (PD1/D45): this is a SINGLE-SURFACE browse shell. It cannot
// express Retroverse's multi-tab / detail-panel structure — that high ceiling
// stays compiled-in (Retroverse) / ARC 3 (scripted). Documenting the floor is
// the point ("low floor, high ceiling").

import { createEffect, createMemo, createSignal, Match, onCleanup, onMount, Show, Switch, type JSX } from "solid-js";
import { usePlatform } from "@oa/platform/platformContext";
import { useTheme } from "@oa/platform/theme/host";
import { useThemeSettings } from "@oa/platform/theme/themeSettings";
import { activeTheme } from "@oa/platform/theme/registry";
import { useResolvedLayout } from "@oa/platform/theme/layoutResolver";
import { resolveViewTransition, usePrefersReducedMotion } from "@oa/platform/theme/motion";
import ViewTransition from "@oa/platform/theme/ViewTransition";
import { CarouselNav, GridNav, ListNav, WheelNav } from "@oa/platform/nav";
import { systemThemes } from "@oa/platform/themes/registry";
import EngineSummonIcon from "@oa/platform/components/EngineSummonIcon";
import ThemeBackground from "@oa/platform/components/ThemeBackground";
import type { RomEntry } from "@oa/platform/library/types";
import type { SystemId } from "@oa/platform/themes/registry";
import type { NavItemContext } from "@oa/platform/nav";
import type { ThemeEntry } from "@oa/platform/theme/types";

/// The settings_schema keys the DeclarativeShell itself interprets (the seed
/// vocabulary, P.1 S2). A theme's other declared controls still surface in the
/// Appearance panel + persist per-theme — they're just inert in THIS generic
/// shell until the vocabulary accretes (the plan's "settle the minimum, accrete
/// additively" call). Today: one key.
const RECOGNIZED_SETTINGS = {
  /** Tighter list rows. Mirrors the hand-coded `bare` theme's `compactRows`. */
  compactRows: "compactRows",
} as const;

const DeclarativeShell: ThemeEntry = () => {
  // [oa-theme-motion] unconditional mount marker — if this never appears in the
  // log, the DeclarativeShell isn't being mounted (a non-declarative theme is
  // active), which is the prerequisite for any M1 motion.
  console.log("[oa-theme-motion] DeclarativeShell MOUNTED");
  const platform = usePlatform();
  const host = useTheme();
  const settings = useThemeSettings();

  // The recognized density setting (inert if the theme doesn't declare it).
  const compact = (): boolean =>
    settings.get<boolean>(RECOGNIZED_SETTINGS.compactRows, false);

  const themeName = (): string => activeTheme()?.manifest.name ?? "";

  // Every real (non-seed) game, one row per identity (multi-disc / multi-region
  // collapse), sorted by title — the same dedup contract `bare` + CoverFlow use,
  // so the RomEntry refs stay stable across renders and the list reconciles.
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

  // Controlled focus so the background can follow the focused game's system.
  const [focusedIndex, setFocusedIndex] = createSignal(0);
  const focusedEntry = createMemo<RomEntry | null>(() => {
    const list = games();
    if (list.length === 0) return null;
    const i = Math.min(Math.max(focusedIndex(), 0), list.length - 1);
    return list[i] ?? null;
  });

  // Flat all-systems browse → no single system in context, so layout resolves
  // at the VIEW level (theme's `views["game-browse"].layout`, else engine
  // default `grid`). Per-system LAYOUT variation (D32) applies in a
  // system-scoped browse, not this flat list — see module header.
  const layout = useResolvedLayout("game-browse", () => null);

  // Declarative motion (ARC 3 M1 / D52): resolve the active theme's declared
  // view transition + the live reduced-motion preference into the transition
  // the browse surface plays. Reduced-motion downgrades to a short fade inside
  // the resolver. The transition is purely visual + interruptible — it never
  // gates browsing (see ViewTransition).
  const reducedMotion = usePrefersReducedMotion();
  const viewTransition = createMemo(() =>
    resolveViewTransition(activeTheme()?.manifest.motion, reducedMotion()),
  );

  // Entrance play (ARC 3 M1). The surface mounts + first-paints while the OS
  // window is still settling, so a play at mount is unseen. We flip `entered`
  // ONCE, a beat after mount (past the window-present settle), and key the
  // transition on it — so the browse view plays exactly one clean entrance when
  // it's actually on-screen. (Earlier multi-trigger replays — mount + timer +
  // focus + visibility — stacked into a strobe; one play is the fix.) M2 adds
  // the runtime re-trigger axis (per-system/per-view layout changes); M1 is the
  // single entrance.
  const [entered, setEntered] = createSignal(false);
  onMount(() => {
    if (typeof window === "undefined") return;
    const timer = window.setTimeout(() => setEntered(true), 350);
    onCleanup(() => window.clearTimeout(timer));
  });

  // The transition's trigger — what counts as a "view change". M1: the resolved
  // layout primitive (the per-view/per-system axis M2 varies at runtime) gated
  // by `entered` so the FIRST play is the deferred, on-screen entrance.
  const viewKey = createMemo(() => `${layout()}|${entered()}`);

  // [oa-theme-motion] diagnostic — confirms the DeclarativeShell mounted (i.e. a
  // DECLARATIVE theme is active; Retroverse's compiled shell would NOT log this)
  // and shows whether the active theme's motion data arrived + how it resolved.
  createEffect(() => {
    const t = activeTheme();
    console.log(
      `[oa-theme-motion] shell theme=${t?.manifest.id ?? "?"} motion=${JSON.stringify(
        t?.manifest.motion ?? null,
      )} resolved=${JSON.stringify(viewTransition())} reducedMotion=${reducedMotion()} games=${games().length}`,
    );
  });

  const sysShort = (entry: RomEntry): string =>
    systemThemes[entry.systemId as SystemId]?.shortName ?? entry.systemId;

  // --- card renderers ---------------------------------------------------------

  // List row — mirrors `bare`: per-system accent dot + title + system short,
  // focus highlight, compact-density honoring. `data-system` makes
  // `--color-system-accent` resolve to this game's (possibly theme-overridden)
  // palette.
  const renderRow = (entry: RomEntry, ctx: NavItemContext): JSX.Element => (
    <div
      data-system={entry.systemId}
      class="flex items-center justify-between gap-4 rounded-md px-4"
      classList={{
        "bg-white/[0.06] text-(--color-oa-ink)": ctx.focused(),
        "text-(--color-oa-ink-dim)": !ctx.focused(),
        "py-1": compact(),
        "py-2.5": !compact(),
      }}
    >
      <div class="flex min-w-0 items-center gap-3">
        <span
          class="size-2 shrink-0 rounded-full"
          style={{ "background-color": "var(--color-system-accent)" }}
          aria-hidden="true"
        />
        <span class="truncate text-sm">{entry.title}</span>
      </div>
      <span class="shrink-0 text-[0.6rem] uppercase tracking-[0.3em] text-(--color-oa-ink-dim)">
        {sysShort(entry)}
      </span>
    </div>
  );

  // Tile card for grid/carousel/wheel — a system-tinted panel with the title +
  // system label. Cover-art rendering is a deliberate later accretion (the
  // dogfood surface is the list); a text tile keeps S2 correct + risk-free.
  const renderCard = (entry: RomEntry, ctx: NavItemContext): JSX.Element => (
    <div
      data-system={entry.systemId}
      class="flex h-full w-full flex-col justify-end gap-1 overflow-hidden rounded-xl border p-3 transition-[transform,border-color] duration-200"
      classList={{
        "border-(--color-system-accent) scale-[1.02]": ctx.focused(),
        "border-white/5": !ctx.focused(),
      }}
      style={{
        "background-image":
          "linear-gradient(160deg, var(--color-system-glow), color-mix(in oklch, var(--color-oa-bg-deep) 80%, transparent))",
      }}
    >
      <span class="line-clamp-2 text-sm font-medium text-(--color-oa-ink)">{entry.title}</span>
      <span class="text-[0.55rem] uppercase tracking-[0.3em] text-(--color-oa-ink-dim)">
        {sysShort(entry)}
      </span>
    </div>
  );

  return (
    <div class="relative flex h-full w-full flex-col bg-(--color-oa-bg-deep) text-(--color-oa-ink)">
      {/* Per-system backdrop, following focus. Renders nothing when the theme
          ships no background assets (S5.1 cascade → null), so a bare theme
          stays bare. */}
      <ThemeBackground systemId={() => (focusedEntry()?.systemId as SystemId | null) ?? null} />

      <header class="relative z-10 flex items-center justify-between border-b border-white/5 px-6 py-4">
        <div class="leading-tight">
          <p class="text-sm font-semibold uppercase tracking-[0.3em]">{themeName()}</p>
          <p class="text-[0.55rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
            {games().length} games
          </p>
        </div>
        <div class="flex items-center gap-3">
          {/* D3 — every theme reserves the top-right slot for the engine summon
              icon (F12 · Select+Start): the always-available path back to
              Settings → Themes to switch shells. */}
          <EngineSummonIcon />
        </div>
      </header>

      <ViewTransition
        class="relative z-10 min-h-0 flex-1"
        trigger={viewKey}
        transition={viewTransition}
      >
        <Show
          when={games().length > 0}
          fallback={
            <div class="flex h-full w-full items-center justify-center text-sm text-(--color-oa-ink-dim)">
              No games yet — add a library folder from Settings.
            </div>
          }
        >
          <Switch
            fallback={
              <GridNav
                id="declarative-library"
                class="h-full w-full overflow-y-auto px-6 py-4"
                items={games}
                columns={6}
                focusedIndex={focusedIndex}
                setFocusedIndex={setFocusedIndex}
                hints={{ dpad: "Browse", stick: "Browse", Confirm: "Launch", Secondary: "Game info" }}
                onConfirm={(_i, entry) => void host.onLaunch(entry)}
                onSecondary={(_i, entry) => host.onShowInfo(entry)}
              >
                {(entry, ctx) => <div class="aspect-[3/4]">{renderCard(entry, ctx)}</div>}
              </GridNav>
            }
          >
            <Match when={layout() === "list"}>
              <ListNav
                id="declarative-library"
                class="h-full w-full overflow-y-auto px-4 py-3"
                items={games}
                focusedIndex={focusedIndex}
                setFocusedIndex={setFocusedIndex}
                hints={{ dpad: "Move", stick: "Move", Confirm: "Launch", Secondary: "Game info" }}
                onConfirm={(_i, entry) => void host.onLaunch(entry)}
                onSecondary={(_i, entry) => host.onShowInfo(entry)}
              >
                {renderRow}
              </ListNav>
            </Match>

            <Match when={layout() === "carousel"}>
              <CarouselNav
                id="declarative-library"
                class="h-full w-full"
                items={games}
                cardWidth={210}
                pitch={168}
                focusedIndex={focusedIndex}
                setFocusedIndex={setFocusedIndex}
                hints={{ dpad: "Browse", stick: "Browse", Confirm: "Launch", Secondary: "Game info" }}
                onConfirm={(_i, entry) => void host.onLaunch(entry)}
                onSecondary={(_i, entry) => host.onShowInfo(entry)}
              >
                {(entry, ctx) => <div class="aspect-[3/4] w-[210px]">{renderCard(entry, ctx)}</div>}
              </CarouselNav>
            </Match>

            <Match when={layout() === "wheel"}>
              <WheelNav
                id="declarative-library"
                class="h-full w-full"
                items={games}
                radius={520}
                focusedIndex={focusedIndex}
                setFocusedIndex={setFocusedIndex}
                hints={{ dpad: "Browse", stick: "Browse", Confirm: "Launch", Secondary: "Game info" }}
                onConfirm={(_i, entry) => void host.onLaunch(entry)}
                onSecondary={(_i, entry) => host.onShowInfo(entry)}
              >
                {(entry, ctx) => <div class="aspect-[3/4] w-[200px]">{renderCard(entry, ctx)}</div>}
              </WheelNav>
            </Match>
          </Switch>
        </Show>
      </ViewTransition>
    </div>
  );
};

export default DeclarativeShell;
