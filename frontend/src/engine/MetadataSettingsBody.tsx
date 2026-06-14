// Engine territory — Settings → Metadata, a full-screen takeover (Metadata
// Curation arc Wave 1 / S2; layout per the 2026-06-12 review, DECISIONS D6–D9).
//
// This shell owns the takeover chrome (back button · Systems/Games switch ·
// preview toggle), the searchable system rail, and the empty-system filter. The
// per-system editor itself lives in SystemMetaForm (extracted so the Systems hub
// reuses it). The GAME half stays in MetadataGamePane.
//
// NOTE (Per-System Settings Hub arc): the SYSTEM half is being migrated into the
// Systems hub's Metadata domain card; this full-screen takeover is removed in
// that arc's S5 once parity is confirmed.

import {
  createMemo,
  createResource,
  createSignal,
  For,
  Show,
  type Accessor,
  type Component,
} from "solid-js";
import { HintRegion, useDomQueryFocusGroup } from "@oa/platform/nav";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import { listGameGroups } from "@oa/platform/api/libraryApi";
import type { GameGroupInfo } from "@oa/platform/library/types";
import MetadataGamePane from "./MetadataGamePane";
import { listSystemInfoOverridden } from "@oa/platform/library/systemInfo";
import { SystemMetaForm } from "./SystemMetaForm";

// --- System entity list (left rail) --------------------------------------

const SystemList: Component<{
  systems: { id: SystemId; displayName: string }[];
  activeId: Accessor<SystemId | null>;
  overridden: Accessor<Set<string>>;
  onPick: (id: SystemId) => void;
}> = (props) => {
  const [query, setQuery] = createSignal("");
  const [editedOnly, setEditedOnly] = createSignal(false);
  const filtered = createMemo(() => {
    const q = query().trim().toLowerCase();
    const ov = props.overridden();
    return props.systems.filter((s) => {
      if (editedOnly() && !ov.has(s.id)) return false;
      if (!q) return true;
      return s.displayName.toLowerCase().includes(q) || s.id.toLowerCase().includes(q);
    });
  });
  return (
    <div class="flex h-full min-h-0 w-64 shrink-0 flex-col gap-2 border-r border-white/5 px-4 py-4">
      <input
        type="search"
        value={query()}
        onInput={(e) => setQuery(e.currentTarget.value)}
        placeholder="Search systems…"
        class="w-full rounded-md border border-white/10 bg-black/40 px-3 py-1.5 text-sm text-(--color-oa-ink) placeholder:text-(--color-oa-ink-dim)/50 focus:border-(--color-system-accent)/60 focus:outline-none"
      />
      <button
        type="button"
        onClick={(e) => {
          e.currentTarget.blur();
          setEditedOnly((v) => !v);
        }}
        class="flex items-center gap-2 self-start rounded-full border px-2.5 py-1 text-[0.6rem] uppercase tracking-widest transition"
        classList={{
          "border-(--color-system-accent)/50 bg-(--color-system-accent)/20 text-(--color-system-accent-soft)":
            editedOnly(),
          "border-white/10 bg-white/[0.03] text-(--color-oa-ink-dim) hover:bg-white/[0.06]":
            !editedOnly(),
        }}
        aria-pressed={editedOnly()}
      >
        <span class="inline-block h-1.5 w-1.5 rounded-full bg-(--color-system-accent)" aria-hidden="true" />
        Edited only
      </button>
      <ul class="-mr-2 flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto pr-2">
        <For
          each={filtered()}
          fallback={
            <li class="px-2 py-4 text-center text-[0.7rem] text-(--color-oa-ink-dim)/70">
              No systems match.
            </li>
          }
        >
          {(sys) => {
            const isActive = () => props.activeId() === sys.id;
            const isEdited = () => props.overridden().has(sys.id);
            return (
              <li>
                <button
                  type="button"
                  onClick={(e) => {
                    e.currentTarget.blur();
                    props.onPick(sys.id);
                  }}
                  class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                  classList={{
                    "bg-(--color-system-accent)/15 text-(--color-oa-ink)": isActive(),
                    "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)":
                      !isActive(),
                  }}
                  aria-current={isActive() ? "page" : undefined}
                  data-system={sys.id}
                  title={sys.displayName}
                >
                  <span
                    class="inline-block h-2 w-2 shrink-0 rounded-full"
                    classList={{
                      "bg-(--color-system-accent)": isEdited(),
                      "bg-white/15": !isEdited(),
                    }}
                    aria-hidden="true"
                    title={isEdited() ? "Has operator edits" : undefined}
                  />
                  <span class="truncate">{sys.displayName}</span>
                </button>
              </li>
            );
          }}
        </For>
      </ul>
    </div>
  );
};

// --- Main body -----------------------------------------------------------

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

  const [activeId, setActiveId] = createSignal<SystemId | null>(null);
  const [previewOpen, setPreviewOpen] = createSignal(true);
  const [mode, setMode] = createSignal<"systems" | "games">("systems");

  // Which systems carry overrides — list dots + filter. Refetched after every
  // save / reset via SystemMetaForm's onSaved.
  const [overridden, { refetch: refetchOverridden }] = createResource(
    async (): Promise<Set<string>> => {
      try {
        return new Set(await listSystemInfoOverridden());
      } catch (e) {
        console.warn("[MetadataSettingsBody] list_system_info_overridden failed:", e);
        return new Set<string>();
      }
    },
  );
  const overriddenSet = () => overridden() ?? new Set<string>();

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
          Metadata
        </h1>

        <div class="flex items-center gap-0.5 rounded-lg border border-white/10 bg-black/30 p-0.5">
          <For each={["systems", "games"] as const}>
            {(m) => (
              <button
                type="button"
                onClick={(e) => {
                  e.currentTarget.blur();
                  setMode(m);
                }}
                class="rounded-md px-3 py-1 text-[0.7rem] font-medium capitalize transition"
                classList={{
                  "bg-(--color-system-accent)/25 text-(--color-oa-ink)": mode() === m,
                  "text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)": mode() !== m,
                }}
                aria-pressed={mode() === m}
              >
                {m}
              </button>
            )}
          </For>
        </div>

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

      <Show
        when={mode() === "systems"}
        fallback={
          <MetadataGamePane previewOpen={previewOpen} groups={gameGroups() ?? []} systems={systems()} />
        }
      >
        <div class="flex min-h-0 flex-1">
          <SystemList
            systems={systems()}
            activeId={activeId}
            overridden={overriddenSet}
            onPick={setActiveId}
          />

          <Show
            when={activeId()}
            fallback={
              <div class="flex flex-1 items-center justify-center p-8 text-center">
                <div class="max-w-sm">
                  <p class="text-3xl">✎</p>
                  <p class="mt-3 text-sm text-(--color-oa-ink-dim)">
                    Pick a system to curate its facts. Every field falls back to a default —
                    OA's curated copy, or the bundled hardware database below that — when you
                    leave it blank.
                  </p>
                </div>
              </div>
            }
          >
            {(id) => (
              <SystemMetaForm
                systemId={id}
                showPreview={previewOpen}
                onSaved={() => void refetchOverridden()}
              />
            )}
          </Show>
        </div>
      </Show>
    </div>
  );
};

export default MetadataSettingsBody;
