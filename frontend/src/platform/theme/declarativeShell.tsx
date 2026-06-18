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

import { createMemo, createSignal, For, Match, Show, Switch, type JSX } from "solid-js";
import { usePlatform } from "@oa/platform/platformContext";
import { useTheme } from "@oa/platform/theme/host";
import { useThemeSettings } from "@oa/platform/theme/themeSettings";
import { useMedia, type GameMetadata } from "@oa/platform/library/media";
import { activeTheme } from "@oa/platform/theme/registry";
import { useResolvedLayout } from "@oa/platform/theme/layoutResolver";
import { resolveMotionRef, resolveThemeMotionSpec, usePrefersReducedMotion } from "@oa/platform/theme/motion";
import SpecTransition from "@oa/platform/theme/SpecTransition";
import SelectionMotion from "@oa/platform/theme/SelectionMotion";
import { windowShown } from "@oa/platform/theme/windowShown";
import { CarouselNav, GridNav, ListNav, WheelNav } from "@oa/platform/nav";
import { systemThemes } from "@oa/platform/themes/registry";
import EngineSummonIcon from "@oa/platform/components/EngineSummonIcon";
import ThemeBackground from "@oa/platform/components/ThemeBackground";
import type { RomEntry } from "@oa/platform/library/types";
import type { SystemId } from "@oa/platform/themes/registry";
import type { NavItemContext } from "@oa/platform/nav";
import type { ThemeEntry } from "@oa/platform/theme/types";
import type { ThemeElement } from "@oa/platform/theme/manifest";

/// The settings_schema keys the DeclarativeShell itself interprets (the seed
/// vocabulary, P.1 S2). A theme's other declared controls still surface in the
/// Appearance panel + persist per-theme — they're just inert in THIS generic
/// shell until the vocabulary accretes (the plan's "settle the minimum, accrete
/// additively" call). Today: one key.
const RECOGNIZED_SETTINGS = {
  /** Tighter list rows. Mirrors the hand-coded `bare` theme's `compactRows`. */
  compactRows: "compactRows",
} as const;

/// BIOS / boot-ROM title rule — a standalone `[BIOS]` / `(BIOS)` annotation,
/// case-insensitive. Mirrors the Rust `title_parse::is_bios_flag` rule so the
/// no-code browse hides BIOS files the same way the backend tags them.
const BIOS_TITLE = /[[(]\s*bios\s*[\])]/i;

const DeclarativeShell: ThemeEntry = () => {
  const platform = usePlatform();
  const host = useTheme();
  const settings = useThemeSettings();
  const media = useMedia();

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
      // Skip seed rows + BIOS/boot-ROM files. `RomEntry` (the raw per-file browse
      // list) doesn't carry the backend `is_bios` flag — that lives on the grouped
      // `VariantInfo` (`list_game_groups`) — so we derive it from the title using
      // the SAME pure rule the Rust `title_parse` uses (a standalone `[BIOS]` /
      // `(BIOS)` annotation). Proper long-term home: surface `is_bios` onto
      // `RomEntry`, or wire `casual_view_defaults` into the live browse (VL Phase F).
      if (e.seed || BIOS_TITLE.test(e.title)) continue;
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

  // Declarative motion (ARC 3 M-mod / D52): resolve the active theme's declared
  // motion (the `transition` slot — preset name or inline spec — falling back to
  // the legacy `view_transition*` fields) + the live reduced-motion preference
  // into the §2 `MotionSpec` the browse surface plays via `SpecTransition` (the
  // basis-driven WAAPI player; M-mod.1). Reduced-motion downgrades to a short
  // fade inside the resolver. The motion is purely visual + interruptible — it
  // never gates browsing (see SpecTransition).
  const reducedMotion = usePrefersReducedMotion();
  const viewSpec = createMemo(() =>
    resolveThemeMotionSpec(activeTheme()?.manifest.motion, reducedMotion()),
  );

  // Per-item selection + ambient choreography (ARC 3 — the declarative
  // selection/ambient hook). The theme's `[motion.selection]` / `[motion.ambient]`
  // slots resolve (preset name OR inline spec) to §2 specs the FOCUSED card plays
  // via <SelectionMotion>: an entrance pop on focus-gain + an idle loop on the
  // focused card only. Both are `null` when the theme declares neither — then
  // SelectionMotion is inert and the browse looks exactly as before (so a bare
  // theme stays bare). Reduced-motion is floored by the players (D58.6), so these
  // resolve the RAW spec (no `reducedMotion` arg, unlike `viewSpec`).
  const selectionSpec = createMemo(() => resolveMotionRef(activeTheme()?.manifest.motion?.selection));
  const ambientSpec = createMemo(() => resolveMotionRef(activeTheme()?.manifest.motion?.ambient));

  // The transition's trigger — what counts as a "view change". The resolved
  // layout primitive (the per-view/per-system axis varies at runtime) gated by
  // `windowShown` — the backend's real "window is on screen now" signal (Rust
  // shows the hidden window on frontend-ready; see windowShown.ts). D54 LANDMINE:
  // the OS presents the window ~hundreds of ms after first paint, so a play at
  // mount would finish (`fill: both`) while the window is still hidden — the M1
  // "entrance before window shown" bug. `SpecTransition skipInitial` suppresses
  // that mount play; keying the trigger on `windowShown` then fires the entrance
  // exactly when the window is presented (the false→true flip is the only play).
  const viewKey = createMemo(() => `${layout()}|${windowShown()}`);

  const sysShort = (entry: RomEntry): string =>
    systemThemes[entry.systemId as SystemId]?.shortName ?? entry.systemId;

  // Box-art cover for a card (identity-keyed first, then the raw rom id — the
  // same lookup the lab/coverflow use). Returns null when the library has no art
  // for this game, so the card falls back to its system-tinted panel. An accessor
  // (not a captured value) so it re-renders when the media store finishes loading.
  const coverFor = (entry: RomEntry): string | null =>
    (entry.identityId ? media.coverUrl(entry.systemId as SystemId, entry.identityId) : null) ??
    media.coverUrl(entry.systemId as SystemId, entry.id);

  // --- card renderers ---------------------------------------------------------

  // List row — mirrors `bare`: per-system accent dot + title + system short,
  // focus highlight, compact-density honoring. `data-system` makes
  // `--color-system-accent` resolve to this game's (possibly theme-overridden)
  // palette.
  const renderRow = (entry: RomEntry, ctx: NavItemContext): JSX.Element => (
    <SelectionMotion
      focused={ctx.focused}
      selection={selectionSpec}
      ambient={ambientSpec}
      reducedMotion={reducedMotion}
    >
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
    </SelectionMotion>
  );

  // Tile card for grid/carousel/wheel — box art when the library has a cover,
  // else a system-tinted panel (so a library with no art still reads cleanly). A
  // bottom gradient scrim keeps the title legible over any cover. Focus emphasis
  // is the accent border + an accent glow shadow (the static, no-motion baseline);
  // when the theme declares a `selection` preset, SelectionMotion's animated pop
  // adds the motion on top (centred cards scale symmetrically — MOTION.md #3).
  // h-full/w-full passes through both SelectionMotion transform carriers so the
  // card still fills its aspect box.
  const renderCard = (entry: RomEntry, ctx: NavItemContext): JSX.Element => {
    const cover = (): string | null => coverFor(entry);
    return (
      <SelectionMotion
        class="h-full w-full"
        focused={ctx.focused}
        selection={selectionSpec}
        ambient={ambientSpec}
        reducedMotion={reducedMotion}
      >
        <div
          data-system={entry.systemId}
          class="relative flex h-full w-full flex-col justify-end overflow-hidden rounded-xl border transition-[border-color,box-shadow] duration-200"
          classList={{
            "border-(--color-system-accent)": ctx.focused(),
            "border-white/10": !ctx.focused(),
          }}
          style={{
            "box-shadow": ctx.focused()
              ? "0 18px 40px rgba(0,0,0,.55), 0 0 38px -8px var(--color-system-accent)"
              : "0 6px 16px rgba(0,0,0,.4)",
          }}
        >
          <Show
            when={cover()}
            fallback={
              <div
                class="absolute inset-0"
                style={{
                  "background-image":
                    "linear-gradient(160deg, var(--color-system-glow), color-mix(in oklch, var(--color-oa-bg-deep) 80%, transparent))",
                }}
              />
            }
          >
            {(u) => <img src={u()} alt="" class="absolute inset-0 h-full w-full object-cover" loading="lazy" />}
          </Show>
          {/* title plate — gradient scrim so the text stays legible over art */}
          <div class="relative z-10 flex flex-col gap-0.5 bg-gradient-to-t from-black/85 via-black/40 to-transparent px-3 pb-2.5 pt-7">
            <span class="line-clamp-2 text-sm font-medium text-white drop-shadow">{entry.title}</span>
            <span class="text-[0.55rem] uppercase tracking-[0.3em] text-white/65">{sysShort(entry)}</span>
          </div>
        </div>
      </SelectionMotion>
    );
  };

  // --- focused-game detail composition (ARC 3 — the declarative element model) ---
  // A theme's `[[detail.elements]]` render in an engine-arranged focused-detail
  // region (carousel/wheel layouts). Each element binds to the focused game's data
  // and plays its entrance `motion` (a keyed SpecTransition) on focus change. The
  // reserved `position`/`size`/`ambient` fields are IGNORED here — the slot
  // renderer arranges by declared order; the future free-form canvas honors them.
  const detailElements = createMemo<ThemeElement[]>(
    () => activeTheme()?.manifest.detail?.elements ?? [],
  );
  const focusedMeta = (): GameMetadata | undefined => {
    const g = focusedEntry();
    return g ? media.media(g.identityId ?? g.id)?.metadata : undefined;
  };
  const logoFor = (entry: RomEntry): string | null =>
    (entry.identityId ? media.coverUrl(entry.systemId as SystemId, entry.identityId, "clear-logo") : null) ??
    media.coverUrl(entry.systemId as SystemId, entry.id, "clear-logo");

  // Bind one element kind to the focused game (reactive). Absent data → nothing
  // (graceful); a reserved/unbound kind → nothing (the vocabulary accretes).
  const elementContent = (kind: string): JSX.Element => {
    const meta = (): GameMetadata | undefined => focusedMeta();
    const metaLine = (text: () => string | number | undefined): JSX.Element => (
      <Show when={text() != null && text() !== ""}>
        <span class="text-sm text-(--color-oa-ink-dim)">{String(text())}</span>
      </Show>
    );
    switch (kind) {
      case "title":
        return (
          <h2 class="max-w-3xl text-4xl font-black leading-tight tracking-tight drop-shadow">
            {focusedEntry()?.title}
          </h2>
        );
      case "system":
        return (
          <span class="text-xs font-bold uppercase tracking-[0.4em] text-(--color-system-accent)">
            {focusedEntry() ? sysShort(focusedEntry()!) : ""}
          </span>
        );
      case "year":
        return metaLine(() => meta()?.year);
      case "genre":
        return metaLine(() => meta()?.genre);
      case "developer":
        return metaLine(() => meta()?.developer);
      case "publisher":
        return metaLine(() => meta()?.publisher);
      case "players":
        return metaLine(() => (meta()?.players != null ? `${meta()!.players}P` : undefined));
      case "description":
        return (
          <Show when={meta()?.description}>
            {(d) => (
              <p class="line-clamp-3 max-w-2xl text-sm leading-relaxed text-(--color-oa-ink-dim)">{d()}</p>
            )}
          </Show>
        );
      case "logo":
        return (
          <Show when={focusedEntry() && logoFor(focusedEntry()!)}>
            {(u) => <img src={u()} alt="" class="max-h-24 w-auto max-w-md object-contain drop-shadow-lg" />}
          </Show>
        );
      case "cover":
        return (
          <Show when={focusedEntry() && coverFor(focusedEntry()!)}>
            {(u) => <img src={u()} alt="" class="max-h-48 w-auto rounded-lg object-contain shadow-xl" />}
          </Show>
        );
      default:
        // Reserved / not-yet-bound kind (rating / stats / …) — renders nothing.
        return <></>;
    }
  };

  // The detail region: each declared element wrapped in a keyed SpecTransition so
  // its entrance `motion` replays when the focused game changes. `skipInitial`
  // keeps it quiet at boot (the browse-level transition covers that entrance, and
  // the first focus move is post-window-shown — D54 landmine covered).
  const renderDetail = (): JSX.Element => (
    <For each={detailElements()}>
      {(el) => {
        const spec = resolveMotionRef(el.motion);
        return (
          <SpecTransition
            trigger={() => focusedEntry()?.id ?? ""}
            spec={() => spec}
            skipInitial
            reducedMotion={reducedMotion}
          >
            {elementContent(el.kind)}
          </SpecTransition>
        );
      }}
    </For>
  );

  // Overlay the detail region across the top of a hero-style (carousel/wheel)
  // browse — the coverflow/wheel cards sit in the vertical middle, leaving the top
  // band for the focused game's title/logo/metadata. `pointer-events-none` so it
  // never blocks card interaction. Renders nothing when no `detail` is declared.
  const detailOverlay = (): JSX.Element => (
    <Show when={detailElements().length > 0}>
      <div class="pointer-events-none absolute inset-x-0 top-0 z-10 flex flex-col items-start gap-2 px-12 pt-10">
        {renderDetail()}
      </div>
    </Show>
  );

  return (
    // `overflow-hidden` is the MOTION.md rule #2 clipping ancestor: the shell root
    // never scrolls as a whole (the inner nav primitive owns its own
    // `overflow-y-auto`), so clipping here keeps a per-item selection transform (a
    // `title-rise` y-translate, or a card scale) from briefly extending an
    // ancestor's scroll area and spawning a transient scrollbar.
    <div class="relative flex h-full w-full flex-col overflow-hidden bg-(--color-oa-bg-deep) text-(--color-oa-ink)">
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

      <SpecTransition
        class="relative z-10 min-h-0 flex-1"
        trigger={viewKey}
        spec={viewSpec}
        skipInitial
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
              <div class="relative h-full w-full">
                {detailOverlay()}
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
              </div>
            </Match>

            <Match when={layout() === "wheel"}>
              <div class="relative h-full w-full">
                {detailOverlay()}
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
              </div>
            </Match>
          </Switch>
        </Show>
      </SpecTransition>
    </div>
  );
};

export default DeclarativeShell;
