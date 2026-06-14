// Bare — the minimal valid theme. The lowest-floor reference shell, the S4
// validator's canonical fixture, AND the substrate TEST BED (operator framing,
// S5.2): the eventual proper list-view theme grows from here, so new substrate
// capabilities get their first real consumer in bare.
//
// Theming Substrate ARC 1 Phase 3 S4 (the versioned manifest + validator,
// docs/PLANS/theming-substrate.md §13.3 + scope-call #7); extended in S5.2 to
// consume the per-system palette seam. This is the floor of the north star
// "low floor, high ceiling": the smallest thing that is a *valid, functional*
// whole-shell theme. If a creator can read one file to see "this is all a theme
// MUST be," it's this one.
//
// What it is: a plain vertical list of every game (title + system) with a
// per-system accent dot, Up/Down to move, Confirm to launch, plus the
// engine-summon icon (D3, the always-present path back to Settings). No covers,
// no design-token overrides, no ceremony — deliberately. It consumes ONLY the
// platform layer (usePlatform stores + the useTheme host services + the S1
// `list` primitive + the platform-homed EngineSummonIcon), exactly like any
// other theme.
//
// S5.2 PER-SYSTEM SEAM DEMO: each row carries `data-system`, so its accent dot
// shows that system's baseline palette colour — AND bare's `perSystemTokens`
// (below) recolours a couple of systems to prove the override is SCOPED to this
// theme's mount. In bare, NES dots read cyan / PSX dots read magenta; in engine
// territory (Settings → Per-system) those same systems keep their baseline red /
// teal. That's the D19 optional sub-cascade + the D2 sibling-scope, visible.
//
// S5.3 GLYPH-SET DEMO: bare's manifest declares `glyph_set: "playstation"`, so
// the HintBar paints ✕/◯/□/△ (the Launch hint reads ✕) while Retroverse keeps
// the default Xbox A — a theme picking its controller-glyph identity in one field.
//
// S5.4 PER-THEME SETTINGS DEMO: the header "Compact" toggle writes a theme-owned
// pref via `useThemeSettings()` (auto-bound to bare's id → collision-free, never
// touches OA settings or another theme). It's localStorage-persisted, so the
// density choice survives the restart-based theme swap.
//
// DUAL ROLE: themes/index.ts registers it (so the operator can switch to it and
// see the floor end-to-end — browse + launch + restart all work), and
// validate.test.ts validates this exact package as the "canonical minimal
// valid theme" fixture. One artifact, both jobs — so the reference can never
// silently drift from what the validator accepts.

import { createMemo } from "solid-js";
import { usePlatform } from "@oa/platform/platformContext";
import { useTheme } from "@oa/platform/theme/host";
import { useThemeSettings } from "@oa/platform/theme/themeSettings";
import { ListNav } from "@oa/platform/nav";
import { systemThemes } from "@oa/platform/themes/registry";
import EngineSummonIcon from "@oa/platform/components/EngineSummonIcon";
import type { RomEntry } from "@oa/platform/library/types";
import type { ThemeEntry, ThemePackage } from "@oa/platform/theme/types";
import type { ThemeManifest } from "@oa/platform/theme/manifest";

// Authored inline as a typed object (the manifest reader lands in Phase 5).
// This is the minimum a manifest must declare to pass the S4 validator: every
// required field present, schema_version supported, surfaces ⊆ honored, no
// required engine capabilities, no token overrides.
const BARE_MANIFEST: ThemeManifest = {
  id: "bare",
  name: "Bare",
  version: "1.0.0",
  schema_version: 1,
  oa_version: "^0.x",
  entry: "./index.tsx",
  entry_export: "bare",
  default_route: "library",
  routes: ["library"],
  // Bare consumes only the library store + shared selection. Declaring the
  // smallest honest slot set is part of being the reference.
  context_slots: ["library"],
  required_engine_capabilities: [],
  reserves_corner: "top-right",
  surfaces: ["main"],
  // S5.3 test-bed demo: bare paints PlayStation glyphs (✕/◯/□/△) in the
  // HintBar, so the Launch hint reads ✕ here while Retroverse (no glyph_set)
  // keeps the default Xbox A. Proves a theme picks its controller-glyph
  // identity via one manifest field. Omitting it inherits "xbox".
  glyph_set: "playstation",
  // Settings IA Slice 3 — bare's one appearance option, now DECLARED rather
  // than hand-rendered. The engine's generic Appearance panel reads this and
  // draws the toggle; bare just consumes the value (`compact()` below). Same
  // per-theme storage key as before, so existing choices carry over.
  settings_schema: [
    {
      key: "compactRows",
      type: "toggle",
      label: "Compact rows",
      hint: "Tighter list spacing",
      default: false,
    },
  ],
};

const BareEntry: ThemeEntry = (_props) => {
  const platform = usePlatform();
  const host = useTheme();
  // S5.4 demo: a theme-owned pref in the collision-free per-theme namespace.
  // `useThemeSettings()` auto-binds bare's id, so this writes `themeSettings.bare`
  // only — never OA settings or another theme's slice. localStorage-persisted, so
  // it survives the restart-based theme swap. `get` is reactive → the toggle
  // repaints the list density live.
  const settings = useThemeSettings();
  const compact = (): boolean => settings.get<boolean>("compactRows", false);

  // Every real (non-seed) game, one row per identity (multi-disc / multi-region
  // collapse), sorted by title. Same dedup contract CoverFlow uses; the RomEntry
  // refs are stable across renders so the list reconciles.
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

  const sysShort = (entry: RomEntry): string =>
    systemThemes[entry.systemId]?.shortName ?? entry.systemId;

  return (
    <div class="flex h-full w-full flex-col bg-(--color-oa-bg-deep) text-(--color-oa-ink)">
      <header class="flex items-center justify-between border-b border-white/5 px-6 py-4">
        <div class="leading-tight">
          <p class="text-sm font-semibold uppercase tracking-[0.3em]">Bare</p>
          <p class="text-[0.55rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
            {games().length} games · minimal reference theme
          </p>
        </div>
        <div class="flex items-center gap-3">
          {/* The "Compact" density toggle moved to Settings → Themes / Appearance
              (Settings IA Slice 3): bare DECLARES it in `settings_schema` and the
              engine renders it. bare just consumes `compact()` below — same
              per-theme storage key, so the choice carries over. */}
          {/* D3 — every theme reserves the top-right slot for the engine summon
              icon (F12 · Select+Start). The operator's always-available path back
              to Settings → Themes to switch shells. */}
          <EngineSummonIcon />
        </div>
      </header>

      <ListNav
        id="bare-library"
        class="min-h-0 flex-1 overflow-y-auto px-4 py-3"
        items={games}
        hints={{ dpad: "Move", stick: "Move", Confirm: "Launch" }}
        onConfirm={(_i, entry) => void host.onLaunch(entry)}
        onSecondary={(_i, entry) => host.onShowInfo(entry)}
      >
        {(entry, ctx) => (
          // `data-system` makes `--color-system-accent` resolve to this game's
          // system palette (baseline, or bare's scoped perSystemTokens override).
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
              {/* Per-system accent dot — the visible consumer of the S5.2
                  palette seam (baseline colour, overridden for the demo systems). */}
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
        )}
      </ListNav>
    </div>
  );
};

export const bare: ThemePackage = {
  manifest: BARE_MANIFEST,
  entry: BareEntry,
  // No design `tokens` — bare doesn't restyle the global palette (CoverFlow
  // already proves the token path). It DOES demo the S5.2 per-system seam:
  // these scoped overrides recolour two systems' accent so the dots prove the
  // override is theme-mount-scoped (engine territory keeps the baseline). NES
  // baseline is red → demo cyan; PSX baseline is teal → demo magenta. Picked
  // two systems likely to be in a test library; the value strings are valid CSS
  // so `validateTheme(bare)` still passes (and now exercises perSystemTokens).
  perSystemTokens: {
    nes: { accent: "oklch(0.72 0.16 195)" },
    psx: { accent: "oklch(0.65 0.22 340)" },
  },
};
