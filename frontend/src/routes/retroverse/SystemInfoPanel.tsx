// HOME right pane — dense system-information layout per the
// operator-supplied HOME mockup. Three live-data cards + an
// achievements stub:
//
//   - SYSTEM INFORMATION: manufacturer / type / generation / release
//     dates / units sold / media / CPU / sound / resolution / palette
//     / display ratio.
//   - TECHNICAL DETAILS: architecture / max players / multiplayer
//     (free-form, refined from the old binary co-op flag) / region /
//     storage / RAM / video output / aspect ratio / refresh rate.
//   - SUPPORTED PERIPHERALS: grid of glyph + label tiles.
//   - ACHIEVEMENTS: placeholder stub numbers (RetroAchievements
//     integration ships in a separate stream).
//
// Phase 3 cutover (2026-05-31): data now flows from the three-layer
// merge in apps/oa-shell/src/system_info.rs via the
// `get_system_info` Tauri command instead of the hand-typed
// `systemMetadataStubs.ts`. The stub file was deleted in this commit;
// the panel renders "—" for any field the merged record leaves
// undefined (L1+L2+L3 all blank → panel still works, just empty).
//
// Drops from the old schema: Input Latency (every system had "Low"
// hardcoded; meaningless), Emulator Core (lives in Settings → per-
// system → Default core, not system metadata). Adds Refresh Rate
// (now sourced from MAME's <display refresh=…> attribute).

import { createResource, For, Show, type Component } from "solid-js";
import type { SystemId } from "../../themes/registry";
import { systemThemes } from "../../themes/registry";
import { getSystemInfo, type MergedSystemInfo } from "../../library/systemInfo";

type Props = {
  systemId: SystemId;
};

const InfoRow: Component<{ label: string; value?: string }> = (props) => (
  <div class="flex items-baseline justify-between gap-3 py-0.5 text-[0.7rem]">
    <span class="shrink-0 text-(--color-oa-ink-dim)">{props.label}</span>
    <span class="text-right text-(--color-oa-ink)">{props.value ?? "—"}</span>
  </div>
);

const TechRow: Component<{ glyph: string; label: string; value?: string }> = (props) => (
  <div class="flex items-baseline justify-between gap-3 py-0.5 text-[0.7rem]">
    <span class="flex shrink-0 items-baseline gap-1.5 text-(--color-oa-ink-dim)">
      <span class="text-(--color-system-accent)">{props.glyph}</span>
      {props.label}
    </span>
    <span class="text-right text-(--color-oa-ink)">{props.value ?? "—"}</span>
  </div>
);

const Card: Component<{
  title: string;
  children: import("solid-js").JSX.Element;
}> = (props) => (
  <section class="rounded-lg border border-white/10 bg-white/[0.02] p-4">
    <p class="mb-3 text-[0.6rem] font-semibold uppercase tracking-[0.3em] text-(--color-oa-ink)">
      {props.title}
    </p>
    {props.children}
  </section>
);

const SystemInfoPanel: Component<Props> = (props) => {
  const theme = () => systemThemes[props.systemId];

  // Live merged record from the three-layer backend. Re-fires when
  // the focused system changes. Errors degrade to undefined (panel
  // renders "—" everywhere) rather than throwing.
  const [merged] = createResource(
    () => props.systemId,
    async (systemId): Promise<MergedSystemInfo | undefined> => {
      try {
        return await getSystemInfo({ systemId });
      } catch (e) {
        console.warn("[SystemInfoPanel] get_system_info failed:", e);
        return undefined;
      }
    },
  );

  const info = (): MergedSystemInfo | undefined => merged();

  // Achievement stubs — placeholder numbers per the operator's spec
  // (mockup-faithful, not derived from real data). RetroAchievements
  // integration ships separately later.
  const achievementStub = () => ({
    earned: 68,
    total: 147,
    percent: 46,
    locked: 29,
    unlocked: 68,
    bronze: 50,
    silver: 14,
    gold: 4,
  });

  return (
    <div
      class="flex h-full flex-col gap-3 overflow-y-auto bg-(--color-oa-bg-deep) px-4 py-4"
      data-system={props.systemId}
    >
      {/* SYSTEM INFORMATION */}
      <Card title="System information">
        <div class="flex flex-col">
          <InfoRow label="Manufacturer" value={info()?.manufacturer} />
          <InfoRow label="Type" value={info()?.systemType} />
          <InfoRow label="Generation" value={info()?.generation} />
          <InfoRow label="Release Date" value={info()?.releaseDate} />
          <InfoRow label="Discontinued" value={info()?.discontinued} />
          <InfoRow label="Units Sold" value={info()?.unitsSold} />
          <InfoRow label="Media" value={info()?.media} />
          <InfoRow label="CPU" value={info()?.cpu} />
          <InfoRow label="Sound" value={info()?.sound} />
          <InfoRow label="Resolution" value={info()?.resolution} />
          <InfoRow label="Color Palette" value={info()?.colorPalette} />
          <InfoRow label="Display Ratio" value={info()?.displayRatio} />
        </div>
      </Card>

      {/* TECHNICAL DETAILS */}
      <Card title="Technical details">
        <div class="flex flex-col">
          <TechRow glyph="▤" label="Architecture" value={info()?.architecture} />
          <TechRow glyph="👥" label="Max Players" value={info()?.maxPlayers} />
          <TechRow glyph="🤝" label="Multiplayer" value={info()?.multiplayer} />
          <TechRow glyph="🌐" label="Region" value={info()?.region} />
          <TechRow glyph="💾" label="Storage" value={info()?.storage} />
          <TechRow glyph="⚡" label="RAM" value={info()?.ram} />
          <TechRow glyph="📺" label="Video Output" value={info()?.videoOutput} />
          <TechRow glyph="📐" label="Aspect Ratio" value={info()?.aspectRatio} />
          <TechRow glyph="🔁" label="Refresh Rate" value={info()?.refreshRate} />
        </div>
      </Card>

      {/* SUPPORTED PERIPHERALS */}
      <Show when={info()?.peripherals && info()!.peripherals.length > 0}>
        <Card title="Supported peripherals">
          <div class="grid grid-cols-3 gap-2">
            <For each={info()?.peripherals ?? []}>
              {(peripheral) => (
                <div class="flex flex-col items-center gap-1 rounded-md border border-white/5 bg-white/[0.02] px-2 py-2 text-center">
                  <span class="text-xl">{peripheral.glyph}</span>
                  <span class="text-[0.55rem] leading-tight text-(--color-oa-ink-dim)">
                    {peripheral.name}
                  </span>
                </div>
              )}
            </For>
          </div>
        </Card>
      </Show>

      {/* ACHIEVEMENTS — placeholder per operator spec. */}
      <Card title="Achievements">
        <div class="flex items-center gap-3">
          <span class="text-3xl text-(--color-system-accent)">🏆</span>
          <div class="min-w-0 flex-1">
            <p class="text-lg font-semibold text-(--color-oa-ink)">
              {achievementStub().earned}{" "}
              <span class="text-[0.7rem] text-(--color-oa-ink-dim)">
                / {achievementStub().total}
              </span>
            </p>
            <div class="mt-1 h-1.5 w-full overflow-hidden rounded-full bg-white/5">
              <div
                class="h-full rounded-full bg-(--color-system-accent)"
                style={{ width: `${achievementStub().percent}%` }}
              />
            </div>
          </div>
          <span class="text-sm font-semibold text-(--color-oa-ink)">
            {achievementStub().percent}%
          </span>
        </div>
        <div class="mt-3 grid grid-cols-5 gap-1.5 text-center">
          <div class="rounded border border-white/5 bg-white/[0.02] py-1.5">
            <p class="text-sm font-semibold text-(--color-oa-ink)">
              {achievementStub().locked}
            </p>
            <p class="text-[0.5rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Locked
            </p>
          </div>
          <div class="rounded border border-white/5 bg-white/[0.02] py-1.5">
            <p class="text-sm font-semibold text-(--color-oa-ink)">
              {achievementStub().unlocked}
            </p>
            <p class="text-[0.5rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Unlocked
            </p>
          </div>
          <div class="rounded border border-amber-700/30 bg-amber-900/20 py-1.5">
            <p class="text-sm font-semibold text-amber-300">
              {achievementStub().bronze}
            </p>
            <p class="text-[0.5rem] uppercase tracking-widest text-amber-300/70">
              Bronze
            </p>
          </div>
          <div class="rounded border border-slate-400/30 bg-slate-400/[0.08] py-1.5">
            <p class="text-sm font-semibold text-slate-200">
              {achievementStub().silver}
            </p>
            <p class="text-[0.5rem] uppercase tracking-widest text-slate-300/70">
              Silver
            </p>
          </div>
          <div class="rounded border border-yellow-400/30 bg-yellow-500/10 py-1.5">
            <p class="text-sm font-semibold text-yellow-300">
              {achievementStub().gold}
            </p>
            <p class="text-[0.5rem] uppercase tracking-widest text-yellow-300/70">
              Gold
            </p>
          </div>
        </div>
        <button
          type="button"
          class="mt-3 flex w-full items-center justify-between rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
          title="RetroAchievements integration ships separately"
          disabled
        >
          <span>View all achievements</span>
          <span>›</span>
        </button>
      </Card>
      {/* Tag-along system name footer so the operator can confirm which
          system the panel is showing without scrolling back to the top. */}
      <p class="px-2 pb-2 text-[0.55rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)/60">
        {theme()?.displayName ?? props.systemId}
      </p>
    </div>
  );
};

export default SystemInfoPanel;
