// Engine territory — Settings → Metadata → Games (Metadata Curation arc
// Wave 1 / S3). The GAME-factual editor over the S1
// `game_metadata_overrides` backend, keyed by identity_id (D3).
//
// Same full-screen shell + quiet-provenance + optimistic-autosave model
// as the system pane, but with the typed controls that earn their keep
// for games (metadataControls.tsx): year stepper, rating stars, genre
// chips with library-corpus typeahead, region / release-type segmented
// pills. Provenance "Default" baseline = the pristine `game_identities`
// row (get_identity) beneath the override; reset drops the override and
// the synced/enriched value shines back through.
//
// The picker (D4): a searchable game list from `list_game_groups`
// (identity-backed groups), scoped by a system filter, with cover
// thumbs + per-game "edited" dots.

import {
  createEffect,
  createMemo,
  createResource,
  createSignal,
  For,
  onCleanup,
  Show,
  type Accessor,
  type Component,
} from "solid-js";
import { convertFileSrc } from "@tauri-apps/api/core";
import { confirm } from "@oa/platform/lib/confirm";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import { listGameGroups } from "@oa/platform/api/libraryApi";
import type { GameGroupInfo } from "@oa/platform/library/types";
import {
  deleteGameMetadataOverride,
  EMPTY_GAME_METADATA_OVERRIDE,
  getGameMetadataOverride,
  getIdentity,
  listGameMetadataOverridden,
  setGameMetadata,
  type GameIdentityRow,
  type GameMetadataOverride,
} from "@oa/platform/library/gameMetadata";
import {
  ChipInput,
  NumberStepper,
  ProvenanceField,
  SegmentedPills,
  StarRating,
  TextArea,
  TextField,
} from "./metadataControls";

const REGIONS = ["World", "USA", "Europe", "Japan", "Asia"] as const;
const RELEASE_TYPES = [
  "Released",
  "DLC",
  "Homebrew",
  "ROM Hack",
  "Unlicensed",
  "Unreleased",
] as const;

const DEFAULT_TITLE = "From synced metadata";

function splitGenre(s: string | undefined): string[] {
  if (!s) return [];
  return s
    .split(",")
    .map((x) => x.trim())
    .filter(Boolean);
}

// --- Game picker (left rail) ---------------------------------------------

const GamePicker: Component<{
  groups: GameGroupInfo[];
  systems: { id: SystemId; displayName: string }[];
  activeId: Accessor<string | null>;
  overridden: Accessor<Set<string>>;
  onPick: (group: GameGroupInfo) => void;
}> = (props) => {
  const [query, setQuery] = createSignal("");
  const [systemFilter, setSystemFilter] = createSignal<string>("");
  const [editedOnly, setEditedOnly] = createSignal(false);

  const filtered = createMemo(() => {
    const q = query().trim().toLowerCase();
    const sys = systemFilter();
    const ov = props.overridden();
    return props.groups
      .filter((g) => {
        if (!g.identityId) return false;
        if (sys && g.systemId !== sys) return false;
        if (editedOnly() && !ov.has(g.identityId)) return false;
        if (!q) return true;
        return g.displayBaseTitle.toLowerCase().includes(q);
      })
      .slice(0, 500);
  });

  return (
    <div class="flex h-full min-h-0 w-72 shrink-0 flex-col gap-2 border-r border-white/5 px-4 py-4">
      <input
        type="search"
        value={query()}
        onInput={(e) => setQuery(e.currentTarget.value)}
        placeholder="Search games…"
        class="w-full rounded-md border border-white/10 bg-black/40 px-3 py-1.5 text-sm text-(--color-oa-ink) placeholder:text-(--color-oa-ink-dim)/50 focus:border-(--color-system-accent)/60 focus:outline-none"
      />
      <div class="flex items-center gap-2">
        <select
          value={systemFilter()}
          onChange={(e) => setSystemFilter(e.currentTarget.value)}
          class="min-w-0 flex-1 rounded-md border border-white/10 bg-black/40 px-2 py-1 text-xs text-(--color-oa-ink) focus:border-(--color-system-accent)/60 focus:outline-none"
        >
          <option value="">All systems</option>
          <For each={props.systems}>
            {(s) => <option value={s.id}>{s.displayName}</option>}
          </For>
        </select>
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            setEditedOnly((v) => !v);
          }}
          class="shrink-0 rounded-full border px-2 py-1 text-[0.55rem] uppercase tracking-widest transition"
          classList={{
            "border-(--color-system-accent)/50 bg-(--color-system-accent)/20 text-(--color-system-accent-soft)":
              editedOnly(),
            "border-white/10 bg-white/[0.03] text-(--color-oa-ink-dim) hover:bg-white/[0.06]":
              !editedOnly(),
          }}
          aria-pressed={editedOnly()}
          title="Show only games you've edited"
        >
          Edited
        </button>
      </div>
      <ul class="-mr-2 flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto pr-2">
        <For
          each={filtered()}
          fallback={
            <li class="px-2 py-4 text-center text-[0.7rem] text-(--color-oa-ink-dim)/70">
              No games match.
            </li>
          }
        >
          {(g) => {
            const isActive = () => props.activeId() === g.identityId;
            const isEdited = () => !!g.identityId && props.overridden().has(g.identityId);
            const cover = () =>
              g.canonicalCoverPath ? convertFileSrc(g.canonicalCoverPath) : undefined;
            return (
              <li>
                <button
                  type="button"
                  onClick={(e) => {
                    e.currentTarget.blur();
                    props.onPick(g);
                  }}
                  class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                  classList={{
                    "bg-(--color-system-accent)/15": isActive(),
                    "hover:bg-white/[0.04]": !isActive(),
                  }}
                  aria-current={isActive() ? "page" : undefined}
                  title={g.displayBaseTitle}
                >
                  <span class="flex h-8 w-6 shrink-0 items-center justify-center overflow-hidden rounded bg-black/40">
                    <Show
                      when={cover()}
                      fallback={<span class="text-[0.6rem] text-(--color-oa-ink-dim)/50">▦</span>}
                    >
                      <img src={cover()} alt="" class="h-full w-full object-cover" />
                    </Show>
                  </span>
                  <span class="flex min-w-0 flex-1 flex-col">
                    <span
                      class="truncate text-xs"
                      classList={{
                        "text-(--color-oa-ink)": isActive(),
                        "text-(--color-oa-ink-dim)": !isActive(),
                      }}
                    >
                      {g.displayBaseTitle}
                    </span>
                    <span class="truncate text-[0.6rem] text-(--color-oa-ink-dim)/60">
                      {systemThemes[g.systemId]?.displayName ?? g.systemId}
                    </span>
                  </span>
                  <span
                    class="inline-block h-2 w-2 shrink-0 rounded-full"
                    classList={{
                      "bg-(--color-system-accent)": isEdited(),
                      "bg-transparent": !isEdited(),
                    }}
                    aria-hidden="true"
                    title={isEdited() ? "Has operator edits" : undefined}
                  />
                </button>
              </li>
            );
          }}
        </For>
      </ul>
    </div>
  );
};

// --- Live preview (game tile / hero) -------------------------------------

const GamePreview: Component<{
  title: string;
  cover?: string;
  year?: number;
  developer?: string;
  publisher?: string;
  genres: string[];
  players?: number;
  region?: string;
  rating?: number;
  releaseType?: string;
  description?: string;
}> = (props) => {
  const metaLine = () =>
    [props.year?.toString(), props.developer, props.region].filter(Boolean).join(" · ");
  return (
    <div class="overflow-hidden rounded-2xl border border-(--color-system-accent)/20 bg-gradient-to-br from-(--color-system-accent)/[0.12] via-black/40 to-black/60">
      <div class="flex flex-col gap-3 p-5">
        <span class="text-[0.5rem] font-semibold uppercase tracking-[0.4em] text-(--color-system-accent)/80">
          Live preview
        </span>
        <div class="flex gap-3">
          <div class="h-28 w-20 shrink-0 overflow-hidden rounded-lg border border-white/10 bg-black/40">
            <Show
              when={props.cover}
              fallback={
                <div class="flex h-full w-full items-center justify-center text-2xl text-(--color-oa-ink-dim)/40">
                  ▦
                </div>
              }
            >
              <img src={props.cover} alt="" class="h-full w-full object-cover" />
            </Show>
          </div>
          <div class="flex min-w-0 flex-col gap-1">
            <h2 class="text-lg font-semibold leading-tight text-(--color-oa-ink)">{props.title}</h2>
            <Show when={metaLine()}>
              <p class="text-[0.72rem] text-(--color-oa-ink-dim)">{metaLine()}</p>
            </Show>
            <Show when={props.rating !== undefined}>
              <p class="text-sm text-(--color-system-accent)" aria-label={`${props.rating} of 5`}>
                {"★".repeat(Math.round(props.rating!))}
                <span class="text-(--color-oa-ink-dim)/30">
                  {"★".repeat(Math.max(0, 5 - Math.round(props.rating!)))}
                </span>
              </p>
            </Show>
            <Show when={props.releaseType}>
              <span class="mt-0.5 self-start rounded-full border border-(--color-system-accent)/40 bg-(--color-system-accent)/20 px-2 py-0.5 text-[0.55rem] uppercase tracking-widest text-(--color-system-accent-soft)">
                {props.releaseType}
              </span>
            </Show>
          </div>
        </div>

        <Show when={props.genres.length > 0}>
          <div class="flex flex-wrap gap-1.5">
            <For each={props.genres}>
              {(g) => (
                <span class="rounded-full border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.65rem] text-(--color-oa-ink-dim)">
                  {g}
                </span>
              )}
            </For>
          </div>
        </Show>

        <Show when={props.publisher || props.players !== undefined}>
          <p class="text-[0.7rem] text-(--color-oa-ink-dim)">
            {[
              props.publisher,
              props.players !== undefined
                ? `${props.players} player${props.players > 1 ? "s" : ""}`
                : undefined,
            ]
              .filter(Boolean)
              .join(" · ")}
          </p>
        </Show>

        <Show when={props.description}>
          <p class="line-clamp-4 text-[0.75rem] leading-relaxed text-(--color-oa-ink-dim)">
            {props.description}
          </p>
        </Show>
      </div>
    </div>
  );
};

// --- Main pane -----------------------------------------------------------

const MetadataGamePane: Component<{ previewOpen: Accessor<boolean> }> = (props) => {
  const systems = createMemo<{ id: SystemId; displayName: string }[]>(() => {
    const ids = Object.keys(systemThemes) as SystemId[];
    return ids
      .map((id) => ({ id, displayName: systemThemes[id]?.displayName ?? id }))
      .sort((a, b) => a.displayName.localeCompare(b.displayName));
  });

  const [groups] = createResource(async (): Promise<GameGroupInfo[]> => {
    try {
      return await listGameGroups();
    } catch (e) {
      console.warn("[MetadataGamePane] list_game_groups failed:", e);
      return [];
    }
  });
  const groupList = () => groups() ?? [];

  // Typeahead corpora from the merged library metadata.
  const genreCorpus = createMemo(() => {
    const set = new Set<string>();
    for (const g of groupList()) for (const t of splitGenre(g.genre)) set.add(t);
    return [...set].sort();
  });
  const developerCorpus = createMemo(() => {
    const set = new Set<string>();
    for (const g of groupList()) if (g.developer) set.add(g.developer);
    return [...set].sort();
  });
  const publisherCorpus = createMemo(() => {
    const set = new Set<string>();
    for (const g of groupList()) if (g.publisher) set.add(g.publisher);
    return [...set].sort();
  });

  const [selected, setSelected] = createSignal<GameGroupInfo | null>(null);
  const activeId = () => selected()?.identityId ?? null;

  // Which identities carry overrides — picker dots + filter.
  const [overridden, { refetch: refetchOverridden }] = createResource(
    async (): Promise<Set<string>> => {
      try {
        return new Set(await listGameMetadataOverridden());
      } catch (e) {
        console.warn("[MetadataGamePane] list_game_metadata_overridden failed:", e);
        return new Set<string>();
      }
    },
  );
  const overriddenSet = () => overridden() ?? new Set<string>();

  // Pristine identity baseline (the "Default" provenance) — refetched on
  // selection change.
  const [identity] = createResource(activeId, async (id): Promise<GameIdentityRow | null> => {
    try {
      return await getIdentity({ identityId: id });
    } catch (e) {
      console.warn("[MetadataGamePane] get_identity failed:", e);
      return null;
    }
  });

  // Saved override + working draft (same baseline/draft pattern as the
  // system pane).
  const [baseline, setBaseline] = createSignal<GameMetadataOverride>(EMPTY_GAME_METADATA_OVERRIDE);
  const [draft, setDraft] = createSignal<GameMetadataOverride>(EMPTY_GAME_METADATA_OVERRIDE);
  const [loaded] = createResource(activeId, async (id): Promise<GameMetadataOverride> => {
    try {
      return await getGameMetadataOverride({ identityId: id });
    } catch (e) {
      console.warn("[MetadataGamePane] get_game_metadata_override failed:", e);
      return EMPTY_GAME_METADATA_OVERRIDE;
    }
  });
  createEffect(() => {
    const o = loaded();
    if (o) {
      setBaseline({ ...o });
      setDraft({ ...o });
    }
  });

  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [savedAt, setSavedAt] = createSignal<number | null>(null);
  const isDirty = () => JSON.stringify(draft()) !== JSON.stringify(baseline());

  const persistFor = async (id: string, snapshot: GameMetadataOverride) => {
    setError(null);
    setSaving(true);
    try {
      await setGameMetadata({ identityId: id, overrideRecord: snapshot });
      if (activeId() === id) {
        setBaseline({ ...snapshot });
        setSavedAt(Date.now());
      }
      void refetchOverridden();
    } catch (e) {
      console.error("[MetadataGamePane] save failed:", e);
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  createEffect(() => {
    const snapshot = draft();
    void baseline();
    const id = activeId();
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = undefined;
    }
    if (!id || !isDirty()) return;
    saveTimer = setTimeout(() => {
      void persistFor(id, snapshot);
    }, 600);
  });
  onCleanup(() => {
    if (saveTimer) clearTimeout(saveTimer);
  });

  // Typed setters — `undefined` / empty clears the override field so the
  // row stays sparse (empty = inherit, matching the system pane).
  type StrKey = "title" | "sortTitle" | "developer" | "publisher" | "region" | "releaseType" | "series" | "description";
  const setStr = (key: StrKey, raw: string | undefined) =>
    setDraft((prev) => {
      const next = { ...prev };
      if (!raw) delete next[key];
      else next[key] = raw;
      return next;
    });
  type NumKey = "year" | "players" | "maxPlayers" | "rating";
  const setNum = (key: NumKey, n: number | undefined) =>
    setDraft((prev) => {
      const next = { ...prev };
      if (n === undefined) delete next[key];
      else next[key] = n;
      return next;
    });
  const setGenre = (next: string[]) =>
    setDraft((prev) => {
      const out = { ...prev };
      if (next.length === 0) delete out.genre;
      else out.genre = next;
      return out;
    });

  // Provenance baseline accessors (pristine identity values).
  const idn = () => identity();
  const genreWorking = () => draft().genre ?? splitGenre(idn()?.genre);

  // Live-preview effective values (draft over identity baseline).
  const effTitle = () => draft().title ?? idn()?.canonicalTitle ?? selected()?.displayBaseTitle ?? "";
  const effYear = () => draft().year ?? idn()?.year;
  const effDeveloper = () => draft().developer ?? idn()?.developer;
  const effPublisher = () => draft().publisher ?? idn()?.publisher;
  const effPlayers = () => draft().players ?? idn()?.players;
  const effRating = () => draft().rating ?? idn()?.rating;
  const coverUrl = () => {
    const p = selected()?.canonicalCoverPath;
    return p ? convertFileSrc(p) : undefined;
  };

  const handleResetAll = async () => {
    const id = activeId();
    if (!id) return;
    if (
      !(await confirm("Reset every metadata override for this game?", {
        title: "Reset game metadata",
        confirmLabel: "Reset",
        danger: true,
      }))
    )
      return;
    setError(null);
    setSaving(true);
    try {
      await deleteGameMetadataOverride({ identityId: id });
      setBaseline({ ...EMPTY_GAME_METADATA_OVERRIDE });
      setDraft({ ...EMPTY_GAME_METADATA_OVERRIDE });
      setSavedAt(Date.now());
      void refetchOverridden();
    } catch (e) {
      console.error("[MetadataGamePane] reset failed:", e);
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div class="flex min-h-0 flex-1">
      <GamePicker
        groups={groupList()}
        systems={systems()}
        activeId={activeId}
        overridden={overriddenSet}
        onPick={setSelected}
      />

      <Show
        when={selected()}
        fallback={
          <div class="flex flex-1 items-center justify-center p-8 text-center">
            <div class="max-w-sm">
              <p class="text-3xl">✎</p>
              <p class="mt-3 text-sm text-(--color-oa-ink-dim)">
                Pick a game to curate its facts. Every field falls back to the
                synced/enriched value when you leave it blank — reset any field
                to restore it.
              </p>
            </div>
          </div>
        }
      >
        <div class="flex min-h-0 flex-1">
          {/* Editor */}
          <div class="flex min-w-0 flex-1 flex-col overflow-y-auto">
            <div class="flex items-center justify-between border-b border-white/5 px-6 py-2.5">
              <span class="text-[0.7rem]">
                <Show
                  when={saving()}
                  fallback={
                    <Show
                      when={error()}
                      fallback={
                        <Show
                          when={savedAt()}
                          fallback={
                            <span class="text-(--color-oa-ink-dim)">Edits save automatically.</span>
                          }
                        >
                          <span class="text-(--color-system-accent-soft)">All changes saved.</span>
                        </Show>
                      }
                    >
                      <span class="text-red-300">{error()}</span>
                    </Show>
                  }
                >
                  <span class="text-(--color-oa-ink-dim)">Saving…</span>
                </Show>
              </span>
              <button
                type="button"
                onClick={(e) => {
                  e.currentTarget.blur();
                  handleResetAll();
                }}
                disabled={saving() || (!overriddenSet().has(activeId() ?? "") && !isDirty())}
                class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim) transition hover:border-red-400/40 hover:bg-red-400/10 hover:text-red-200 disabled:opacity-40"
                title="Drop every metadata override for this game"
              >
                Reset all
              </button>
            </div>

            <div class="flex flex-col gap-1 px-6 py-4">
              {/* Typeahead datalists for the free-text fields. */}
              <datalist id="meta-developers">
                <For each={developerCorpus()}>{(d) => <option value={d} />}</For>
              </datalist>
              <datalist id="meta-publishers">
                <For each={publisherCorpus()}>{(p) => <option value={p} />}</For>
              </datalist>

              <h3 class="px-2 pt-1 text-[0.65rem] font-semibold uppercase tracking-[0.3em] text-(--color-system-accent)/80">
                Identity
              </h3>
              <ProvenanceField
                label="Title"
                overridden={draft().title !== undefined}
                defaultText={idn()?.canonicalTitle}
                defaultTitle={DEFAULT_TITLE}
                onReset={() => setStr("title", undefined)}
              >
                <TextField
                  value={draft().title}
                  placeholder={idn()?.canonicalTitle ?? "—"}
                  onInput={(v) => setStr("title", v)}
                />
              </ProvenanceField>
              <ProvenanceField
                label="Sort title"
                overridden={draft().sortTitle !== undefined}
                onReset={() => setStr("sortTitle", undefined)}
              >
                <TextField
                  value={draft().sortTitle}
                  placeholder="e.g. Legend of Zelda, The"
                  onInput={(v) => setStr("sortTitle", v)}
                />
              </ProvenanceField>
              <ProvenanceField
                label="Year"
                overridden={draft().year !== undefined}
                defaultText={idn()?.year?.toString()}
                defaultTitle={DEFAULT_TITLE}
                onReset={() => setNum("year", undefined)}
              >
                <NumberStepper
                  value={draft().year}
                  min={1970}
                  max={2035}
                  placeholder={idn()?.year?.toString() ?? "—"}
                  onChange={(n) => setNum("year", n)}
                />
              </ProvenanceField>
              <ProvenanceField
                label="Developer"
                overridden={draft().developer !== undefined}
                defaultText={idn()?.developer}
                defaultTitle={DEFAULT_TITLE}
                onReset={() => setStr("developer", undefined)}
              >
                <TextField
                  value={draft().developer}
                  list="meta-developers"
                  placeholder={idn()?.developer ?? "—"}
                  onInput={(v) => setStr("developer", v)}
                />
              </ProvenanceField>
              <ProvenanceField
                label="Publisher"
                overridden={draft().publisher !== undefined}
                defaultText={idn()?.publisher}
                defaultTitle={DEFAULT_TITLE}
                onReset={() => setStr("publisher", undefined)}
              >
                <TextField
                  value={draft().publisher}
                  list="meta-publishers"
                  placeholder={idn()?.publisher ?? "—"}
                  onInput={(v) => setStr("publisher", v)}
                />
              </ProvenanceField>
              <ProvenanceField
                label="Genre"
                overridden={draft().genre !== undefined}
                defaultText={idn()?.genre}
                defaultTitle={DEFAULT_TITLE}
                onReset={() => setGenre([])}
              >
                <ChipInput
                  values={genreWorking()}
                  suggestions={genreCorpus()}
                  placeholder="Add a genre…"
                  listId="meta-genres"
                  onChange={setGenre}
                />
              </ProvenanceField>
              <ProvenanceField
                label="Series"
                overridden={draft().series !== undefined}
                onReset={() => setStr("series", undefined)}
              >
                <TextField
                  value={draft().series}
                  placeholder="e.g. Castlevania"
                  onInput={(v) => setStr("series", v)}
                />
              </ProvenanceField>

              <h3 class="mt-3 px-2 pt-1 text-[0.65rem] font-semibold uppercase tracking-[0.3em] text-(--color-system-accent)/80">
                Details
              </h3>
              <ProvenanceField
                label="Players"
                overridden={draft().players !== undefined}
                defaultText={idn()?.players?.toString()}
                defaultTitle={DEFAULT_TITLE}
                onReset={() => setNum("players", undefined)}
              >
                <NumberStepper
                  value={draft().players}
                  min={1}
                  max={16}
                  placeholder={idn()?.players?.toString() ?? "—"}
                  onChange={(n) => setNum("players", n)}
                />
              </ProvenanceField>
              <ProvenanceField
                label="Max players"
                overridden={draft().maxPlayers !== undefined}
                onReset={() => setNum("maxPlayers", undefined)}
              >
                <NumberStepper
                  value={draft().maxPlayers}
                  min={1}
                  max={16}
                  placeholder="—"
                  onChange={(n) => setNum("maxPlayers", n)}
                />
              </ProvenanceField>
              <ProvenanceField
                label="Rating"
                overridden={draft().rating !== undefined}
                defaultText={idn()?.rating?.toString()}
                defaultTitle={DEFAULT_TITLE}
                onReset={() => setNum("rating", undefined)}
              >
                <StarRating value={draft().rating} onChange={(n) => setNum("rating", n)} />
              </ProvenanceField>
              <ProvenanceField
                label="Region"
                overridden={draft().region !== undefined}
                onReset={() => setStr("region", undefined)}
              >
                <SegmentedPills
                  value={draft().region}
                  options={REGIONS}
                  onChange={(v) => setStr("region", v)}
                />
              </ProvenanceField>
              <ProvenanceField
                label="Release type"
                overridden={draft().releaseType !== undefined}
                onReset={() => setStr("releaseType", undefined)}
              >
                <SegmentedPills
                  value={draft().releaseType}
                  options={RELEASE_TYPES}
                  onChange={(v) => setStr("releaseType", v)}
                />
              </ProvenanceField>
              <ProvenanceField
                label="Description"
                overridden={draft().description !== undefined}
                onReset={() => setStr("description", undefined)}
              >
                <TextArea
                  value={draft().description}
                  placeholder="Overview / description…"
                  onInput={(v) => setStr("description", v)}
                />
              </ProvenanceField>
            </div>
          </div>

          {/* Live preview (collapsible). */}
          <Show when={props.previewOpen()}>
            <aside class="hidden w-80 shrink-0 overflow-y-auto border-l border-white/5 p-5 lg:block">
              <GamePreview
                title={effTitle()}
                cover={coverUrl()}
                year={effYear()}
                developer={effDeveloper()}
                publisher={effPublisher()}
                genres={genreWorking()}
                players={effPlayers()}
                region={draft().region}
                rating={effRating()}
                releaseType={draft().releaseType}
                description={draft().description}
              />
            </aside>
          </Show>
        </div>
      </Show>
    </div>
  );
};

export default MetadataGamePane;
