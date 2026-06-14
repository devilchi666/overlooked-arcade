// Per-system metadata editor — the curation form for ONE system, extracted from
// MetadataSettingsBody so it can be reused by both the full-screen Metadata
// takeover (which supplies the left system rail + header) and the Systems hub's
// Metadata domain card. Keyed by a single `systemId` accessor; owns the
// resource/draft/optimistic-autosave machinery + the grouped field form +
// peripherals editor + (optional) live preview. Persistence is unchanged
// (the *_system_info_override commands). Per-GAME metadata stays separate
// (MetadataGamePane).

import {
  createEffect,
  createResource,
  createSignal,
  For,
  onCleanup,
  Show,
  type Accessor,
  type Component,
} from "solid-js";
import { confirm } from "@oa/platform/lib/confirm";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import {
  EMPTY_SYSTEM_INFO_OVERRIDE,
  getSystemInfo,
  getSystemInfoCurated,
  getSystemInfoOverride,
  resetSystemInfoToDefault,
  setSystemInfoOverride,
  type MergedSystemInfo,
  type Peripheral,
  type SystemInfoCurated,
  type SystemInfoOverride,
} from "@oa/platform/library/systemInfo";

type FieldKey = Exclude<keyof SystemInfoOverride, "peripherals">;
type FieldDef = { label: string; key: FieldKey; numeric?: boolean };
type FieldGroup = { id: string; title: string; defaultOpen: boolean; fields: FieldDef[] };

const FIELD_GROUPS: readonly FieldGroup[] = [
  {
    id: "identity",
    title: "Identity & hero",
    defaultOpen: true,
    fields: [
      { label: "Tagline", key: "tagline" },
      { label: "Manufacturer", key: "manufacturer" },
      { label: "Type", key: "systemType" },
      { label: "Generation", key: "generation" },
      { label: "Release date", key: "releaseDate" },
      { label: "Blurb", key: "blurb" },
      { label: "Release flag", key: "releaseFlag" },
      { label: "Sidebar subline", key: "sidebarSubline" },
    ],
  },
  {
    id: "technical",
    title: "Technical details",
    defaultOpen: false,
    fields: [
      { label: "CPU", key: "cpu" },
      { label: "Sound", key: "sound" },
      { label: "Resolution", key: "resolution" },
      { label: "Color palette", key: "colorPalette" },
      { label: "Display ratio", key: "displayRatio" },
      { label: "Architecture", key: "architecture" },
      { label: "Max players", key: "maxPlayers", numeric: true },
      { label: "Multiplayer", key: "multiplayer" },
      { label: "Region", key: "region" },
      { label: "Storage", key: "storage" },
      { label: "RAM", key: "ram" },
      { label: "Video output", key: "videoOutput" },
      { label: "Aspect ratio", key: "aspectRatio" },
      { label: "Refresh rate", key: "refreshRate" },
      { label: "Discontinued", key: "discontinued" },
      { label: "Units sold", key: "unitsSold" },
      { label: "Media", key: "media" },
    ],
  },
];

const INPUT_CLASS =
  "min-w-0 flex-1 rounded-md border border-transparent bg-black/30 px-3 py-1.5 text-sm text-(--color-oa-ink) placeholder:text-(--color-oa-ink-dim)/45 transition hover:border-white/10 focus:border-(--color-system-accent)/60 focus:bg-black/40 focus:outline-none";

function sourceTooltip(from: "curated" | "mame"): string {
  return from === "curated"
    ? "Default from OA's curated copy for this system"
    : "Default from the bundled hardware database";
}

const MetaField: Component<{
  field: FieldDef;
  value: string | undefined;
  inherited: { value: string; from: "curated" | "mame" } | null;
  overridden: boolean;
  onInput: (raw: string) => void;
  onReset: () => void;
}> = (props) => (
  <div
    data-setting-row
    class="group relative flex items-center gap-3 rounded-md py-1 pl-3 pr-2 transition hover:bg-white/[0.03]"
  >
    <span
      class="absolute inset-y-1.5 left-0 w-0.5 rounded-full bg-(--color-system-accent) transition-opacity"
      classList={{ "opacity-0": !props.overridden, "opacity-100": props.overridden }}
      aria-hidden="true"
    />
    <label
      class="w-36 shrink-0 text-sm transition-colors"
      classList={{
        "text-(--color-oa-ink)": props.overridden,
        "text-(--color-oa-ink-dim)": !props.overridden,
      }}
    >
      {props.field.label}
    </label>
    <input
      type={props.field.numeric ? "number" : "text"}
      value={props.value ?? ""}
      placeholder={props.inherited?.value ?? "—"}
      onInput={(e) => props.onInput(e.currentTarget.value)}
      class={INPUT_CLASS}
    />
    <div class="flex w-48 shrink-0 items-center justify-end gap-2 text-[0.65rem]">
      {/* The "Default:" hint stays quiet (hover/focus only); the Reset button
          is ALWAYS visible when overridden so it's reachable by direction nav
          (and by the Y/Tertiary accelerator via data-setting-reset). */}
      <Show when={props.inherited}>
        <span
          class="truncate text-(--color-oa-ink-dim)/70 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100"
          title={sourceTooltip(props.inherited!.from)}
        >
          Default: {props.inherited!.value}
        </span>
      </Show>
      <Show when={props.overridden}>
        <button
          type="button"
          data-setting-reset
          onClick={(e) => {
            e.currentTarget.blur();
            props.onReset();
          }}
          class="shrink-0 rounded border border-white/10 bg-white/[0.04] px-1.5 py-0.5 uppercase tracking-widest text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
          title="Reset this field to its default"
        >
          Reset
        </button>
      </Show>
    </div>
  </div>
);

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
          <h2 class="text-2xl font-semibold tracking-tight text-(--color-oa-ink)">{props.displayName}</h2>
          <Show when={eff("tagline")}>
            <p class="mt-1 text-sm italic text-(--color-system-accent-soft)">{eff("tagline")}</p>
          </Show>
          <Show when={eff("manufacturer") || eff("generation") || eff("releaseDate")}>
            <p class="mt-1 text-[0.75rem] text-(--color-oa-ink-dim)">
              {[eff("manufacturer"), eff("generation"), eff("releaseDate")].filter(Boolean).join(" · ")}
            </p>
          </Show>
        </div>
        <Show when={eff("blurb")}>
          <p class="line-clamp-3 text-[0.78rem] leading-relaxed text-(--color-oa-ink-dim)">{eff("blurb")}</p>
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
    <div class="flex flex-col gap-2 pl-3">
      <Show when={!isInherited()}>
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            props.onChange(undefined);
          }}
          class="self-start rounded-md border border-white/10 bg-white/[0.03] px-2 py-0.5 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim) transition hover:bg-white/[0.07] hover:text-(--color-oa-ink)"
          title="Drop the override; inherit the default peripheral list"
        >
          Reset to default
        </button>
      </Show>
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

export const SystemMetaForm: Component<{
  /// The system being edited (non-null).
  systemId: Accessor<SystemId>;
  /// Render the live-preview aside (lg+). Default: hidden.
  showPreview?: Accessor<boolean>;
  /// Fired after a successful save / reset, so a parent list can refresh its
  /// "has edits" markers.
  onSaved?: () => void;
}> = (props) => {
  const [openGroups, setOpenGroups] = createSignal<Record<string, boolean>>(
    Object.fromEntries(FIELD_GROUPS.map((g) => [g.id, g.defaultOpen])),
  );
  const [peripheralsOpen, setPeripheralsOpen] = createSignal(false);
  const toggleGroup = (id: string) => setOpenGroups((prev) => ({ ...prev, [id]: !prev[id] }));

  const [merged] = createResource(props.systemId, async (id): Promise<MergedSystemInfo | undefined> => {
    try {
      return await getSystemInfo({ systemId: id });
    } catch (e) {
      console.warn("[SystemMetaForm] get_system_info failed:", e);
      return undefined;
    }
  });
  const [curated] = createResource(props.systemId, async (id): Promise<SystemInfoCurated | null> => {
    try {
      return await getSystemInfoCurated({ systemId: id });
    } catch (e) {
      console.warn("[SystemMetaForm] get_system_info_curated failed:", e);
      return null;
    }
  });

  const [baseline, setBaseline] = createSignal<SystemInfoOverride>(EMPTY_SYSTEM_INFO_OVERRIDE);
  const [draft, setDraft] = createSignal<SystemInfoOverride>(EMPTY_SYSTEM_INFO_OVERRIDE);
  const [loadedOverride] = createResource(props.systemId, async (id): Promise<SystemInfoOverride> => {
    try {
      return await getSystemInfoOverride({ systemId: id });
    } catch (e) {
      console.warn("[SystemMetaForm] get_system_info_override failed:", e);
      return EMPTY_SYSTEM_INFO_OVERRIDE;
    }
  });
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
  // True when the system has SAVED overrides (so "Reset all" stays usable after
  // autosave clears the dirty flag — the bug where it greyed out permanently).
  const hasSavedOverrides = () =>
    JSON.stringify(baseline()) !== JSON.stringify(EMPTY_SYSTEM_INFO_OVERRIDE);

  // Capture (system, snapshot) so a fast system switch mid-debounce can't write
  // one system's draft to another; the baseline write is guarded to the
  // still-active system for the same reason.
  const persistFor = async (sid: SystemId, snapshot: SystemInfoOverride) => {
    setError(null);
    setSaving(true);
    try {
      await setSystemInfoOverride({ systemId: sid, overrideRecord: snapshot });
      if (props.systemId() === sid) {
        setBaseline({ ...snapshot });
        setSavedAt(Date.now());
      }
      props.onSaved?.();
    } catch (e) {
      console.error("[SystemMetaForm] save failed:", e);
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  createEffect(() => {
    const snapshot = draft();
    void baseline();
    const sid = props.systemId();
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = undefined;
    }
    if (!isDirty()) return;
    saveTimer = setTimeout(() => void persistFor(sid, snapshot), 600);
  });
  onCleanup(() => {
    if (saveTimer) clearTimeout(saveTimer);
  });

  const updateField = (key: FieldKey, raw: string) =>
    setDraft((prev) => {
      const next = { ...prev };
      if (raw === "") delete next[key];
      else next[key] = raw;
      return next;
    });
  const updatePeripherals = (next: Peripheral[] | undefined) =>
    setDraft((prev) => {
      const out = { ...prev };
      if (next === undefined) delete out.peripherals;
      else out.peripherals = next;
      return out;
    });

  const inheritedFor = (key: FieldKey): { value: string; from: "curated" | "mame" } | null => {
    const c = curated()?.[key];
    if (c !== undefined && c !== "") return { value: c, from: "curated" };
    const b = baseline()[key];
    const savedHasIt = b !== undefined && b !== "";
    const m = merged()?.[key];
    if (!savedHasIt && m !== undefined && m !== "") return { value: m, from: "mame" };
    return null;
  };
  const isOverridden = (key: FieldKey): boolean => {
    const v = draft()[key];
    return v !== undefined && v !== "";
  };
  const effective = (key: FieldKey): string | undefined => {
    const v = draft()[key];
    if (v !== undefined && v !== "") return v;
    return inheritedFor(key)?.value;
  };
  const effectivePeripherals = (): Peripheral[] => draft().peripherals ?? merged()?.peripherals ?? [];
  const groupEditedCount = (g: FieldGroup): number =>
    g.fields.reduce((n, f) => n + (isOverridden(f.key) ? 1 : 0), 0);

  const handleResetAll = async () => {
    const sid = props.systemId();
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
      props.onSaved?.();
    } catch (e) {
      console.error("[SystemMetaForm] reset failed:", e);
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const displayName = () => systemThemes[props.systemId()]?.displayName ?? props.systemId();

  const SaveStatus: Component = () => (
    <span class="text-[0.7rem]">
      <Show
        when={saving()}
        fallback={
          <Show
            when={error()}
            fallback={
              <Show
                when={savedAt()}
                fallback={<span class="text-(--color-oa-ink-dim)">Edits save automatically.</span>}
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
  );

  return (
    <div class="flex min-h-0 flex-1">
      <div class="flex min-w-0 flex-1 flex-col overflow-y-auto">
        <div class="flex items-center justify-between border-b border-white/5 px-6 py-2.5">
          <SaveStatus />
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              void handleResetAll();
            }}
            disabled={saving() || (!hasSavedOverrides() && !isDirty())}
            class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim) transition hover:border-red-400/40 hover:bg-red-400/10 hover:text-red-200 disabled:opacity-40"
            title="Drop every metadata override for this system"
          >
            Reset all
          </button>
        </div>

        <div class="flex flex-col gap-1 px-6 py-4">
          <For each={FIELD_GROUPS}>
            {(group) => (
              <section>
                <button
                  type="button"
                  onClick={(e) => {
                    e.currentTarget.blur();
                    toggleGroup(group.id);
                  }}
                  class="flex w-full items-center gap-2 rounded-md px-2 py-2 text-left transition hover:bg-white/[0.03]"
                  aria-expanded={openGroups()[group.id]}
                >
                  <span class="text-[0.6rem] text-(--color-oa-ink-dim)" aria-hidden="true">
                    {openGroups()[group.id] ? "▾" : "▸"}
                  </span>
                  <span class="text-[0.65rem] font-semibold uppercase tracking-[0.3em] text-(--color-system-accent)/80">
                    {group.title}
                  </span>
                  <Show when={groupEditedCount(group) > 0}>
                    <span class="ml-2 rounded-full bg-(--color-system-accent)/25 px-1.5 py-0.5 text-[0.5rem] font-semibold tracking-widest text-(--color-system-accent-soft)">
                      {groupEditedCount(group)} edited
                    </span>
                  </Show>
                </button>
                <Show when={openGroups()[group.id]}>
                  <div class="mb-2 mt-1 flex flex-col">
                    <For each={group.fields}>
                      {(field) => (
                        <MetaField
                          field={field}
                          value={draft()[field.key]}
                          inherited={inheritedFor(field.key)}
                          overridden={isOverridden(field.key)}
                          onInput={(raw) => updateField(field.key, raw)}
                          onReset={() => updateField(field.key, "")}
                        />
                      )}
                    </For>
                  </div>
                </Show>
              </section>
            )}
          </For>

          <section>
            <button
              type="button"
              onClick={(e) => {
                e.currentTarget.blur();
                setPeripheralsOpen((v) => !v);
              }}
              class="flex w-full items-center gap-2 rounded-md px-2 py-2 text-left transition hover:bg-white/[0.03]"
              aria-expanded={peripheralsOpen()}
            >
              <span class="text-[0.6rem] text-(--color-oa-ink-dim)" aria-hidden="true">
                {peripheralsOpen() ? "▾" : "▸"}
              </span>
              <span class="text-[0.65rem] font-semibold uppercase tracking-[0.3em] text-(--color-system-accent)/80">
                Peripherals
              </span>
              <Show when={draft().peripherals !== undefined}>
                <span class="ml-2 rounded-full bg-(--color-system-accent)/25 px-1.5 py-0.5 text-[0.5rem] font-semibold tracking-widest text-(--color-system-accent-soft)">
                  edited
                </span>
              </Show>
            </button>
            <Show when={peripheralsOpen()}>
              <div class="mb-2 mt-1">
                <PeripheralEditor
                  overridden={draft().peripherals}
                  effective={merged()?.peripherals ?? []}
                  onChange={updatePeripherals}
                />
              </div>
            </Show>
          </section>
        </div>
      </div>

      <Show when={props.showPreview?.() ?? false}>
        <aside class="hidden w-80 shrink-0 overflow-y-auto border-l border-white/5 p-5 lg:block">
          <LivePreviewHero
            displayName={displayName()}
            effective={effective}
            peripherals={effectivePeripherals}
          />
        </aside>
      </Show>
    </div>
  );
};

export default SystemMetaForm;
