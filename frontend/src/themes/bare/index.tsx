// Bare — the minimal valid theme. The lowest-floor reference shell AND the
// S4 validator's canonical fixture.
//
// Theming Substrate ARC 1 Phase 3 S4 (the versioned manifest + validator,
// docs/PLANS/theming-substrate.md §13.3 + scope-call #7). This is the floor of
// the north star "low floor, high ceiling": the smallest thing that is a
// *valid, functional* whole-shell theme. If a creator can read one file to see
// "this is all a theme MUST be," it's this one.
//
// What it is: a plain vertical list of every game (title + system), Up/Down to
// move, Confirm to launch, plus the engine-summon icon (D3, the always-present
// path back to Settings). No covers, no per-system colour, no tokens, no
// ceremony — deliberately. It consumes ONLY the platform layer (usePlatform
// stores + the useTheme host services + the S1 `list` primitive + the
// platform-homed EngineSummonIcon), exactly like any other theme.
//
// DUAL ROLE: themes/index.ts registers it (so the operator can switch to it and
// see the floor end-to-end — browse + launch + restart all work), and
// validate.test.ts validates this exact package as the "canonical minimal
// valid theme" fixture. One artifact, both jobs — so the reference can never
// silently drift from what the validator accepts.

import { createMemo } from "solid-js";
import { usePlatform } from "@oa/platform/platformContext";
import { useTheme } from "@oa/platform/theme/host";
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
};

const BareEntry: ThemeEntry = (_props) => {
  const platform = usePlatform();
  const host = useTheme();

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
        {/* D3 — every theme reserves the top-right slot for the engine summon
            icon (F12 · Select+Start). The operator's always-available path back
            to Settings → Themes to switch shells. */}
        <EngineSummonIcon />
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
          <div
            class="flex items-baseline justify-between gap-4 rounded-md px-4 py-2.5"
            classList={{
              "bg-white/[0.06] text-(--color-oa-ink)": ctx.focused(),
              "text-(--color-oa-ink-dim)": !ctx.focused(),
            }}
          >
            <span class="truncate text-sm">{entry.title}</span>
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
  // No tokens — the purest floor. CoverFlow already proves the token-override
  // path; Bare proves a theme needs none.
};
