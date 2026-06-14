// Media domain editor — per-system artwork (PlatformMediaSlots) + the game-media
// data ops (Identify / Sync covers / Sync metadata / Clear metadata / Refresh
// hash DB) inline, driven by the shared useGameMediaOps hook. In-pane; progress
// surfaces through the global BackgroundJobsBar. Persistence unchanged.

import { For, type Accessor, type Component } from "solid-js";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import { PlatformMediaSlots } from "../../PlatformMediaSlots";
import { HubSection, PanelScaffold } from "../PanelScaffold";
import { useGameMediaOps } from "../gameMediaOps";
import { useSystemsStats } from "../systemsHubStats";

type OpDef = {
  label: string;
  hint: string;
  cta: string;
  run: (id: SystemId) => Promise<void>;
  destructive?: boolean;
};

export const MediaEditor: Component<{ systemId: Accessor<SystemId> }> = (props) => {
  const ops = useGameMediaOps();
  const stats = useSystemsStats();
  const s = () => stats.statsFor(props.systemId());
  const busy = () => ops.isSystemBusy(props.systemId());

  const opDefs = (): OpDef[] => [
    { label: "Identify ROMs", hint: "Hash + match against the canonical DB to stamp titles.", cta: "Run", run: ops.startHashResolve },
    { label: "Sync covers", hint: "Download box art for matched games.", cta: "Run", run: ops.startSync },
    { label: "Sync metadata", hint: "Pull year / genre / developer / publisher.", cta: "Run", run: ops.startMetadataSync },
    { label: "Refresh hash DB", hint: "Re-pull the libretro hash catalog for this system.", cta: "Refresh", run: ops.startHashSync },
    { label: "Clear metadata", hint: "Wipe stored metadata for this system (art untouched).", cta: "Clear", run: ops.startClearMetadata, destructive: true },
  ];

  return (
    <PanelScaffold
      system={props.systemId()}
      title={systemThemes[props.systemId()]?.displayName ?? props.systemId()}
      subtitle="Media · artwork + game data"
    >
      <HubSection title="Game data">
        <div class="flex flex-col gap-2">
          <p class="text-[0.7rem] text-(--color-oa-ink-dim)">
            {s().total} game{s().total === 1 ? "" : "s"} ·{" "}
            <span class="tabular-nums">{s().identified}</span> identified ·{" "}
            <span class="tabular-nums">{s().covered}</span> covered ·{" "}
            <span class="tabular-nums">{s().metadataed}</span> metadata
          </p>
          <div class="flex flex-col gap-2">
            <For each={opDefs()}>
              {(op) => (
                <div
                  class="flex items-center justify-between gap-3 rounded-md border bg-white/[0.02] px-3 py-2"
                  classList={{
                    "border-white/10": !op.destructive,
                    "border-rose-500/20": !!op.destructive,
                  }}
                >
                  <div class="min-w-0">
                    <p
                      class="text-[0.8rem] font-semibold"
                      classList={{
                        "text-(--color-oa-ink)": !op.destructive,
                        "text-rose-200": !!op.destructive,
                      }}
                    >
                      {op.label}
                    </p>
                    <p class="text-[0.65rem] text-(--color-oa-ink-dim)">{op.hint}</p>
                  </div>
                  <button
                    type="button"
                    disabled={busy() || s().total === 0}
                    onClick={(e) => {
                      e.currentTarget.blur();
                      void op.run(props.systemId());
                    }}
                    class="shrink-0 rounded-md px-3 py-1.5 text-[0.65rem] font-semibold uppercase tracking-wider transition disabled:cursor-not-allowed disabled:opacity-50"
                    classList={{
                      "border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 text-(--color-oa-ink) hover:border-(--color-system-accent) hover:bg-(--color-system-accent)/25":
                        !op.destructive,
                      "border border-rose-500/30 bg-rose-500/10 text-rose-200 hover:border-rose-400/60 hover:bg-rose-500/20":
                        !!op.destructive,
                    }}
                  >
                    {busy() ? "Working…" : op.cta}
                  </button>
                </div>
              )}
            </For>
          </div>
          <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)/80">
            Progress shows in the background jobs bar — you can leave this page.
          </p>
        </div>
      </HubSection>

      <HubSection title="Artwork">
        <PlatformMediaSlots systemId={props.systemId} />
      </HubSection>
    </PanelScaffold>
  );
};
