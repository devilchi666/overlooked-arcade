// Engine territory — Settings → Game metadata, a full-screen takeover.
//
// Per-SYSTEM facts moved into the Systems hub (Settings → Systems → a system →
// Metadata) during the Per-System Settings Hub arc (S3/S5). This surface now
// hosts the per-GAME editor only (MetadataGamePane over the game_metadata_
// overrides backend). The shell keeps the takeover chrome (back · preview
// toggle) + the system/game lists the game pane needs.

import { createMemo, createResource, createSignal, type Component } from "solid-js";
import { HintRegion, useDomQueryFocusGroup } from "@oa/platform/nav";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import { listGameGroups } from "@oa/platform/api/libraryApi";
import type { GameGroupInfo } from "@oa/platform/library/types";
import MetadataGamePane from "./MetadataGamePane";

const MetadataSettingsBody: Component<{ onBack: () => void }> = (props) => {
  const [gameGroups] = createResource(async (): Promise<GameGroupInfo[]> => {
    try {
      return await listGameGroups();
    } catch (e) {
      console.warn("[MetadataSettingsBody] list_game_groups failed:", e);
      return [];
    }
  });
  const systemsWithGames = (): Set<string> | null => {
    const g = gameGroups();
    if (!g) return null;
    return new Set(g.map((x) => x.systemId));
  };

  const systems = createMemo<{ id: SystemId; displayName: string }[]>(() => {
    const withGames = systemsWithGames();
    const ids = Object.keys(systemThemes) as SystemId[];
    return ids
      .filter((id) => withGames === null || withGames.has(id))
      .map((id) => ({ id, displayName: systemThemes[id]?.displayName ?? id }))
      .sort((a, b) => a.displayName.localeCompare(b.displayName));
  });

  const [previewOpen, setPreviewOpen] = createSignal(true);

  let takeoverRef: HTMLElement | undefined;
  useDomQueryFocusGroup({
    id: "metadata-takeover",
    containerRef: () => takeoverRef,
    orientation: "vertical",
    onActivate: (_i, el) => el.click(),
  });

  return (
    <div ref={(el) => (takeoverRef = el)} class="flex h-full w-full flex-col">
      <HintRegion
        hints={{
          stick: "Navigate",
          dpad: "Navigate",
          Confirm: "Select / edit",
          Back: "Back to Settings",
        }}
      />
      <header class="flex items-center gap-4 border-b border-white/5 px-6 py-3">
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            props.onBack();
          }}
          class="flex items-center gap-1.5 rounded-md px-2 py-1 text-sm text-(--color-oa-ink-dim) transition hover:bg-white/[0.05] hover:text-(--color-oa-ink) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
        >
          <span aria-hidden="true" class="text-base">‹</span>
          Settings
        </button>
        <h1 class="text-lg font-semibold uppercase tracking-widest text-(--color-oa-ink)">
          Game metadata
        </h1>

        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            setPreviewOpen((v) => !v);
          }}
          class="ml-auto flex items-center gap-1.5 rounded-md border px-2.5 py-1 text-[0.65rem] uppercase tracking-widest transition"
          classList={{
            "border-(--color-system-accent)/50 bg-(--color-system-accent)/20 text-(--color-system-accent-soft)":
              previewOpen(),
            "border-white/10 bg-white/[0.03] text-(--color-oa-ink-dim) hover:bg-white/[0.06]":
              !previewOpen(),
          }}
          aria-pressed={previewOpen()}
          title="Toggle the live preview panel"
        >
          <span aria-hidden="true">◧</span>
          Preview
        </button>
      </header>

      <MetadataGamePane previewOpen={previewOpen} groups={gameGroups() ?? []} systems={systems()} />
    </div>
  );
};

export default MetadataSettingsBody;
