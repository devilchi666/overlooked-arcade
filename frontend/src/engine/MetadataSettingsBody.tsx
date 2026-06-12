// Engine territory — Settings → Metadata category body (Metadata
// Curation arc Wave 1 / S2).
//
// The premium curation surface the plan (docs/PLANS/metadata-editing.md
// §"UX pillar") calls for — NOT a flat label:input property grid. S2
// ships the SYSTEM half over the already-shipped three-layer system-info
// override backend (`*_system_info_override` commands); the GAME half
// lands in S3 over the S1 `game_metadata_overrides` backend.
//
// What makes it premium (the gated D5 bar):
//   • Live preview hero (right) updates in real time as you edit — you
//     see the exact HOME-tab result of every change before it commits.
//   • Per-field provenance + one-click reset via SettingRow: each field
//     shows the inherited (curated L2 / MAME L1) value it falls back to,
//     struck through when you've overridden it, with a Reset chip.
//   • Optimistic autosave (debounced) — no modal save ceremony; a quiet
//     "Saving…/Saved" status. Reset-all is the only confirm gate.
//   • Search-as-you-type system list with an "Edited only" filter +
//     per-system "edited" dots (one `list_system_info_overridden` query).
//
// Architecture: engine territory (Settings), theme-free. The live
// preview is a self-contained engine hero — it deliberately does NOT
// import the retroverse SystemInfoPanel (engine ↛ themes boundary).
//
// Provenance limitation (v1): the inherited chip shows the curated (L2)
// value reliably; for fields that exist ONLY at the MAME (L1) layer it
// shows the baseline only while the field is un-overridden (the merge
// backend exposes no L1-without-L3 read). Good enough for v1; revisit if
// operators want the pre-override L1 value visible mid-edit.

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
import SettingRow, { type InheritScope } from "@oa/platform/components/SettingRow";
import { confirm } from "@oa/platform/lib/confirm";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import {
  EMPTY_SYSTEM_INFO_OVERRIDE,
  getSystemInfo,
  getSystemInfoCurated,
  getSystemInfoOverride,
  listSystemInfoOverridden,
  resetSystemInfoToDefault,
  setSystemInfoOverride,
  type MergedSystemInfo,
  type Peripheral,
  type SystemInfoCurated,
  type SystemInfoOverride,
} from "@oa/platform/library/systemInfo";

/// Scalar override keys — every field except the peripherals list
/// (which gets its own editor). All three wire structs share these
/// camelCase names so one key reads across merged / curated / override.
type FieldKey = Exclude<keyof SystemInfoOverride, "peripherals">;

type FieldDef = {
  label: string;
  key: FieldKey;
  /// Render a numeric input. System info is mostly free-text; only a
  /// couple of fields are genuinely numeric.
  numeric?: boolean;
};

const SECTION_FIELDS: { title: string; fields: FieldDef[] }[] = [
  {
    title: "System information",
    fields: [
      { label: "Manufacturer", key: "manufacturer" },
      { label: "Type", key: "systemType" },
      { label: "Generation", key: "generation" },
      { label: "Release date", key: "releaseDate" },
      { label: "Discontinued", key: "discontinued" },
      { label: "Units sold", key: "unitsSold" },
      { label: "Media", key: "media" },
      { label: "CPU", key: "cpu" },
      { label: "Sound", key: "sound" },
      { label: "Resolution", key: "resolution" },
      { label: "Color palette", key: "colorPalette" },
      { label: "Display ratio", key: "displayRatio" },
    ],
  },
  {
    title: "Technical details",
    fields: [
      { label: "Architecture", key: "architecture" },
      { label: "Max players", key: "maxPlayers", numeric: true },
      { label: "Multiplayer", key: "multiplayer" },
      { label: "Region", key: "region" },
      { label: "Storage", key: "storage" },
      { label: "RAM", key: "ram" },
      { label: "Video output", key: "videoOutput" },
      { label: "Aspect ratio", key: "aspectRatio" },
      { label: "Refresh rate", key: "refreshRate" },
    ],
  },
  {
    title: "Hero copy",
    fields: [
      { label: "Release flag", key: "releaseFlag" },
      { label: "Tagline", key: "tagline" },
      { label: "Blurb", key: "blurb" },
      { label: "Sidebar subline", key: "sidebarSubline" },
    ],
  },
];

const TEXT_INPUT_CLASS =
  "min-w-0 flex-1 rounded-md border border-white/10 bg-black/40 px-3 py-2 text-sm text-(--color-oa-ink) placeholder:text-(--color-oa-ink-dim)/50 transition focus:border-(--color-system-accent)/60 focus:outline-none";

// --- Live preview hero ---------------------------------------------------
//
// Self-contained engine render of the HOME hero from the LIVE (draft-
// merged) values. No theme imports — this is engine territory.

const PreviewChip: Component<{ label: string; value?: string }> = (props) => (
  <Show when={props.value}>
    <div class="flex flex-col gap-0.5 rounded-lg border border-white/5 bg-black/30 px-3 py-2">
      <span class="text-[0.5rem] font-semibold uppercase tracking-[0.3em] text-(--color-oa-ink-dim)/70">
        {props.label}
      </span>
      <span class="truncate text-[0.8rem] text-(--color-oa-ink)">{props.value}</span>
    </div>
  </Show>
);

const LivePreviewHero: Component<{
  displayName: string;
  effective: (key: FieldKey) => string | undefined;
  peripherals: Accessor<Peripheral[]>;
}> = (props) => {
  const eff = props.effective;
  return (
    <div class="overflow-hidden rounded-2xl border border-(--color-system-accent)/20 bg-gradient-to-br from-(--color-system-accent)/[0.12] via-black/40 to-black/60">
      <div class="flex flex-col gap-3 p-5">
        <div class="flex items-center justify-between">
          <span class="text-[0.5rem] font-semibold uppercase tracking-[0.4em] text-(--color-system-accent)/80">
            Live preview · HOME hero
          </span>
          <Show when={eff("releaseFlag")}>
            <span class="rounded-full border border-(--color-system-accent)/40 bg-(--color-system-accent)/20 px-2 py-0.5 text-[0.55rem] uppercase tracking-widest text-(--color-system-accent-soft)">
              {eff("releaseFlag")}
            </span>
          </Show>
        </div>

        <div>
          <h2 class="text-2xl font-semibold tracking-tight text-(--color-oa-ink)">
            {props.displayName}
          </h2>
          <Show when={eff("tagline")}>
            <p class="mt-1 text-sm italic text-(--color-system-accent-soft)">
              {eff("tagline")}
            </p>
          </Show>
          <Show when={eff("manufacturer") || eff("generation") || eff("releaseDate")}>
            <p class="mt-1 text-[0.75rem] text-(--color-oa-ink-dim)">
              {[eff("manufacturer"), eff("generation"), eff("releaseDate")]
                .filter(Boolean)
                .join(" · ")}
            </p>
          </Show>
        </div>

        <Show when={eff("blurb")}>
          <p class="line-clamp-3 text-[0.78rem] leading-relaxed text-(--color-oa-ink-dim)">
            {eff("blurb")}
          </p>
        </Show>

        <div class="grid grid-cols-2 gap-2">
          <PreviewChip label="CPU" value={eff("cpu")} />
          <PreviewChip label="Sound" value={eff("sound")} />
          <PreviewChip label="Resolution" value={eff("resolution")} />
          <PreviewChip label="Media" value={eff("media")} />
        </div>

        <Show when={props.peripherals().length > 0}>
          <div class="flex flex-wrap gap-1.5">
            <For each={props.peripherals()}>
              {(p) => (
                <span
                  class="flex items-center gap-1 rounded-full border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.65rem] text-(--color-oa-ink-dim)"
                  title={p.name}
                >
                  <span aria-hidden="true">{p.glyph}</span>
                  <span class="truncate">{p.name}</span>
                </span>
              )}
            </For>
          </div>
        </Show>

        <Show when={eff("sidebarSubline")}>
          <p class="border-t border-white/5 pt-2 text-[0.65rem] text-(--color-oa-ink-dim)/80">
            Sidebar: {eff("sidebarSubline")}
          </p>
        </Show>
      </div>
    </div>
  );
};

// --- Peripheral editor (ported from PerSystemInfoSection) ----------------

const PeripheralEditor: Component<{
  overridden: Peripheral[] | undefined;
  effective: Peripheral[];
  onChange: (next: Peripheral[] | undefined) => void;
}> = (props) => {
  const working = (): Peripheral[] => props.overridden ?? props.effective;
  const isInherited = () => props.overridden === undefined;
  const updateAt = (idx: number, next: Peripheral) => {
    const list = working().slice();
    list[idx] = next;
    props.onChange(list);
  };
  const remove = (idx: number) => {
    const list = working().slice();
    list.splice(idx, 1);
    props.onChange(list);
  };
  return (
    <div class="flex flex-col gap-2">
      <div class="flex items-center justify-between">
        <span class="flex items-center gap-2 text-sm font-medium text-(--color-oa-ink)">
          Peripherals
          <Show when={!isInherited()}>
            <span class="rounded-full bg-(--color-system-accent)/25 px-1.5 py-0.5 text-[0.5rem] font-semibold uppercase tracking-widest text-(--color-system-accent-soft)">
              edited
            </span>
          </Show>
        </span>
        <Show when={!isInherited()}>
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              props.onChange(undefined);
            }}
            class="rounded-md border border-white/10 bg-white/[0.03] px-2 py-0.5 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim) transition hover:bg-white/[0.07] hover:text-(--color-oa-ink)"
            title="Drop the override; inherit the curated / MAME peripheral list"
          >
            Reset
          </button>
        </Show>
      </div>
      <For each={working()}>
        {(rowItem, idx) => (
          <div class="flex items-center gap-2">
            <input
              type="text"
              value={rowItem.glyph}
              maxLength={4}
              onInput={(e) => updateAt(idx(), { ...rowItem, glyph: e.currentTarget.value })}
              placeholder="🎮"
              title="Glyph — single emoji or short symbol"
              class="w-12 shrink-0 rounded-md border border-white/10 bg-black/40 px-2 py-1 text-center text-[0.85rem] text-(--color-oa-ink)"
            />
            <input
              type="text"
              value={rowItem.name}
              onInput={(e) => updateAt(idx(), { ...rowItem, name: e.currentTarget.value })}
              placeholder="Peripheral name"
              class="min-w-0 flex-1 rounded-md border border-white/10 bg-black/40 px-2 py-1 text-sm text-(--color-oa-ink) placeholder:text-(--color-oa-ink-dim)/50"
            />
            <button
              type="button"
              onClick={(e) => {
                e.currentTarget.blur();
                remove(idx());
              }}
              class="shrink-0 rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.7rem] text-(--color-oa-ink-dim) transition hover:border-red-400/40 hover:bg-red-400/10 hover:text-red-200"
              title="Remove this peripheral row"
            >
              ×
            </button>
          </div>
        )}
      </For>
      <button
        type="button"
        onClick={(e) => {
          e.currentTarget.blur();
          props.onChange([...working(), { name: "", glyph: "" }]);
        }}
        class="self-start rounded-md border border-dashed border-white/15 bg-white/[0.02] px-3 py-1 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim) transition hover:border-(--color-system-accent)/40 hover:bg-white/[0.05] hover:text-(--color-oa-ink)"
      >
        + Add peripheral
      </button>
    </div>
  );
};

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
    <div class="flex h-full min-h-0 w-64 shrink-0 flex-col gap-2 border-r border-white/5 pr-3">
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
      <ul class="-mr-1 flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto pr-1">
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

const MetadataSettingsBody: Component = () => {
  const systems = createMemo<{ id: SystemId; displayName: string }[]>(() => {
    const ids = Object.keys(systemThemes) as SystemId[];
    return ids
      .map((id) => ({ id, displayName: systemThemes[id]?.displayName ?? id }))
      .sort((a, b) => a.displayName.localeCompare(b.displayName));
  });

  const [activeId, setActiveId] = createSignal<SystemId | null>(null);

  // Which systems carry overrides — drives the list dots + filter. One
  // query; refetched after every save / reset.
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

  // Effective (merged, incl. saved L3) + curated (L2) for one system —
  // refetched only on system switch. The live preview + provenance read
  // from these plus the live draft.
  const [merged] = createResource(
    activeId,
    async (id): Promise<MergedSystemInfo | undefined> => {
      try {
        return await getSystemInfo({ systemId: id });
      } catch (e) {
        console.warn("[MetadataSettingsBody] get_system_info failed:", e);
        return undefined;
      }
    },
  );
  const [curated] = createResource(
    activeId,
    async (id): Promise<SystemInfoCurated | null> => {
      try {
        return await getSystemInfoCurated({ systemId: id });
      } catch (e) {
        console.warn("[MetadataSettingsBody] get_system_info_curated failed:", e);
        return null;
      }
    },
  );

  // The operator's saved override + the working draft. `baseline` is the
  // last-known-saved value (updated on save) so `dirty` + autosave gate
  // correctly without refetching after every keystroke.
  const [baseline, setBaseline] = createSignal<SystemInfoOverride>(EMPTY_SYSTEM_INFO_OVERRIDE);
  const [draft, setDraft] = createSignal<SystemInfoOverride>(EMPTY_SYSTEM_INFO_OVERRIDE);
  const [loadedOverride] = createResource(
    activeId,
    async (id): Promise<SystemInfoOverride> => {
      try {
        return await getSystemInfoOverride({ systemId: id });
      } catch (e) {
        console.warn("[MetadataSettingsBody] get_system_info_override failed:", e);
        return EMPTY_SYSTEM_INFO_OVERRIDE;
      }
    },
  );
  createEffect(() => {
    const o = loadedOverride();
    if (o) {
      setBaseline({ ...o });
      setDraft({ ...o });
    }
  });

  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [savedAt, setSavedAt] = createSignal<number | null>(null);

  const isDirty = () => JSON.stringify(draft()) !== JSON.stringify(baseline());

  // Persist a specific (system, snapshot) pair — captured at schedule
  // time so a fast system switch mid-debounce can't write one system's
  // draft to another. The baseline write is guarded to the still-active
  // system for the same reason (an in-flight save resolving after a
  // switch must not clobber the new system's baseline).
  const persistFor = async (sid: SystemId, snapshot: SystemInfoOverride) => {
    setError(null);
    setSaving(true);
    try {
      await setSystemInfoOverride({ systemId: sid, overrideRecord: snapshot });
      if (activeId() === sid) {
        setBaseline({ ...snapshot });
        setSavedAt(Date.now());
      }
      void refetchOverridden();
    } catch (e) {
      console.error("[MetadataSettingsBody] save failed:", e);
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  // Debounced optimistic autosave: any draft change schedules a save
  // 600ms later. The timer is cleared unconditionally on every run so a
  // system switch (activeId is a dependency) cancels a pending save for
  // the prior system before the new one loads.
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  createEffect(() => {
    const snapshot = draft();
    void baseline();
    const sid = activeId();
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = undefined;
    }
    if (!sid || !isDirty()) return;
    saveTimer = setTimeout(() => {
      void persistFor(sid, snapshot);
    }, 600);
  });
  onCleanup(() => {
    if (saveTimer) clearTimeout(saveTimer);
  });

  const updateField = (key: FieldKey, raw: string) => {
    setDraft((prev) => {
      const next = { ...prev };
      if (raw === "") delete next[key];
      else next[key] = raw;
      return next;
    });
  };
  const updatePeripherals = (next: Peripheral[] | undefined) => {
    setDraft((prev) => {
      const out = { ...prev };
      if (next === undefined) delete out.peripherals;
      else out.peripherals = next;
      return out;
    });
  };

  // Provenance: the value a field falls back to with NO operator
  // override (curated L2 first; MAME L1 via `merged` only when the saved
  // baseline doesn't override it — see the header note on the v1 limit).
  const inheritedFor = (key: FieldKey): { value: string; from: InheritScope } | null => {
    const c = curated()?.[key];
    if (c !== undefined && c !== "") return { value: c, from: "Curated" };
    const savedHasIt = (() => {
      const b = baseline()[key];
      return b !== undefined && b !== "";
    })();
    const m = merged()?.[key];
    if (!savedHasIt && m !== undefined && m !== "") return { value: m, from: "MAME baseline" };
    return null;
  };
  const isOverridden = (key: FieldKey): boolean => {
    const v = draft()[key];
    return v !== undefined && v !== "";
  };
  // Live preview value: the draft wins, else the inherited baseline.
  const effective = (key: FieldKey): string | undefined => {
    const v = draft()[key];
    if (v !== undefined && v !== "") return v;
    return inheritedFor(key)?.value;
  };
  const effectivePeripherals = (): Peripheral[] =>
    draft().peripherals ?? merged()?.peripherals ?? [];

  const handleResetAll = async () => {
    const sid = activeId();
    if (!sid) return;
    if (
      !(await confirm("Reset every metadata override for this system?", {
        title: "Reset system metadata",
        confirmLabel: "Reset",
        danger: true,
      }))
    )
      return;
    setError(null);
    setSaving(true);
    try {
      await resetSystemInfoToDefault({ systemId: sid });
      setBaseline({ ...EMPTY_SYSTEM_INFO_OVERRIDE });
      setDraft({ ...EMPTY_SYSTEM_INFO_OVERRIDE });
      setSavedAt(Date.now());
      void refetchOverridden();
    } catch (e) {
      console.error("[MetadataSettingsBody] reset failed:", e);
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const activeName = () =>
    systems().find((s) => s.id === activeId())?.displayName ?? activeId() ?? "";

  return (
    <div class="flex h-[calc(100vh-12rem)] min-h-[32rem] gap-5">
      <SystemList
        systems={systems()}
        activeId={activeId}
        overridden={overriddenSet}
        onPick={setActiveId}
      />

      <Show
        when={activeId()}
        fallback={
          <div class="flex flex-1 items-center justify-center rounded-xl border border-dashed border-white/10 bg-white/[0.02] p-8 text-center">
            <div class="max-w-sm">
              <p class="text-3xl">✎</p>
              <p class="mt-3 text-sm text-(--color-oa-ink-dim)">
                Pick a system to curate its facts. Every field falls back to
                the curated <code class="text-(--color-oa-ink-dim)/80">system-info.yaml</code>{" "}
                — and MAME's baseline below that — when you leave it blank.
              </p>
              <p class="mt-2 text-[0.7rem] text-(--color-oa-ink-dim)/70">
                Game-by-game editing arrives in a follow-up slice.
              </p>
            </div>
          </div>
        }
      >
        <div class="flex min-h-0 flex-1 gap-5">
          {/* Editor column */}
          <div class="flex min-w-0 flex-1 flex-col gap-4 overflow-y-auto pr-1">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2 text-[0.7rem] text-(--color-oa-ink-dim)">
                <Show
                  when={saving()}
                  fallback={
                    <Show
                      when={error()}
                      fallback={
                        <Show when={savedAt()} fallback={<span>Edits save automatically.</span>}>
                          <span class="text-(--color-system-accent-soft)">All changes saved.</span>
                        </Show>
                      }
                    >
                      <span class="text-red-300">{error()}</span>
                    </Show>
                  }
                >
                  <span>Saving…</span>
                </Show>
              </div>
              <button
                type="button"
                onClick={(e) => {
                  e.currentTarget.blur();
                  handleResetAll();
                }}
                disabled={saving() || (overriddenSet().size === 0 && !isDirty())}
                class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim) transition hover:border-red-400/40 hover:bg-red-400/10 hover:text-red-200 disabled:opacity-40"
                title="Drop every metadata override for this system; fall back to curated / MAME"
              >
                Reset all
              </button>
            </div>

            <For each={SECTION_FIELDS}>
              {(group) => (
                <section class="rounded-xl border border-white/10 bg-white/[0.02] p-4">
                  <h3 class="mb-3 text-[0.6rem] font-semibold uppercase tracking-[0.3em] text-(--color-system-accent)/80">
                    {group.title}
                  </h3>
                  <div class="flex flex-col gap-2">
                    <For each={group.fields}>
                      {(field) => (
                        <SettingRow
                          label={field.label}
                          overridden={isOverridden(field.key)}
                          inherited={inheritedFor(field.key)}
                          onReset={() => updateField(field.key, "")}
                        >
                          <input
                            type={field.numeric ? "number" : "text"}
                            value={draft()[field.key] ?? ""}
                            placeholder={inheritedFor(field.key)?.value ?? "—"}
                            onInput={(e) => updateField(field.key, e.currentTarget.value)}
                            class={TEXT_INPUT_CLASS}
                          />
                        </SettingRow>
                      )}
                    </For>
                  </div>
                </section>
              )}
            </For>

            <section class="rounded-xl border border-white/10 bg-white/[0.02] p-4">
              <h3 class="mb-3 text-[0.6rem] font-semibold uppercase tracking-[0.3em] text-(--color-system-accent)/80">
                Peripherals
              </h3>
              <PeripheralEditor
                overridden={draft().peripherals}
                effective={merged()?.peripherals ?? []}
                onChange={updatePeripherals}
              />
            </section>
          </div>

          {/* Live preview column */}
          <div class="hidden w-80 shrink-0 overflow-y-auto lg:block">
            <div class="sticky top-0">
              <LivePreviewHero
                displayName={activeName()}
                effective={effective}
                peripherals={effectivePeripherals}
              />
            </div>
          </div>
        </div>
      </Show>
    </div>
  );
};

export default MetadataSettingsBody;
