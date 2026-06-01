// Retroverse SETTINGS → Per-system drill-in → "System info" section
// (Phase 4 of docs/PLANS/system-info-panel-v1.md).
//
// Form rendering for the ~21 L2/L3 fields that populate the HOME tab's
// right pane. Each row shows the operator's current OVERRIDE value
// (or the inherited L1/L2 value as a placeholder when no override is
// set), plus a provenance badge:
//
//   - no badge       → showing L1 (MAME baseline) or no data at any layer
//   - "curated"      → showing L2 (OA-curated YAML)
//   - "edited"       → showing L3 (operator's local override)
//
// Form state binds directly to a SystemInfoOverride. Empty inputs
// clear the corresponding override (which DELETE the column at the
// backend — the row stays only as long as at least one column is
// non-empty). The "Reset all overrides for this system" button blasts
// the entire L3 row so the panel falls through cleanly to L2/L1.
//
// Peripherals get their own editor: an add/remove list with separate
// name + glyph inputs per row. v1 doesn't try to merge or polish the
// L2 list — the operator's override (Some(vec)) entirely replaces it.

import {
  createEffect,
  createMemo,
  createResource,
  createSignal,
  For,
  Show,
  type Accessor,
  type Component,
} from "solid-js";
import type { SystemId } from "../../themes/registry";
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
} from "../../library/systemInfo";

type Props = {
  /// Currently focused system slug. The section refetches all three
  /// resources when this changes.
  systemId: Accessor<SystemId | null>;
};

/// Display label + the keys into MergedSystemInfo / SystemInfoCurated /
/// SystemInfoOverride. All three structs use the same camelCase field
/// names on the wire, so one key serves all three.
type FieldKey = Exclude<
  keyof SystemInfoOverride,
  // Drop the array field — peripherals get a dedicated editor below.
  "peripherals"
>;

type FieldDef = {
  label: string;
  key: FieldKey;
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
      { label: "Max players", key: "maxPlayers" },
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
    title: "Hero",
    fields: [
      { label: "Release flag", key: "releaseFlag" },
      { label: "Tagline", key: "tagline" },
      { label: "Blurb", key: "blurb" },
      { label: "Sidebar subline", key: "sidebarSubline" },
    ],
  },
];

type Provenance = "edited" | "curated" | "default";

function provenanceFor(
  override: string | undefined,
  curated: string | undefined,
  merged: string | undefined,
): Provenance | null {
  // Treat empty string the same as undefined — the merge backend
  // doesn't distinguish them at the column level. Future authoring
  // tool could; v1 collapses both.
  const hasOverride = override !== undefined && override !== "";
  if (hasOverride) return "edited";
  const hasCurated = curated !== undefined && curated !== "";
  if (hasCurated) return "curated";
  if (merged !== undefined && merged !== "") return "default";
  return null;
}

const ProvenanceBadge: Component<{ kind: Provenance | null }> = (props) => (
  <Show when={props.kind === "edited" || props.kind === "curated"}>
    <span
      class="ml-2 rounded-full px-1.5 py-0.5 text-[0.5rem] font-semibold uppercase tracking-widest"
      classList={{
        "bg-(--color-system-accent)/30 text-(--color-system-accent-soft)":
          props.kind === "edited",
        "bg-white/10 text-(--color-oa-ink-dim)": props.kind === "curated",
      }}
      title={
        props.kind === "edited"
          ? "Operator override — your local edit replaces the curated/L1 value"
          : "Curated — from docs/cores/<id>/system-info.yaml"
      }
    >
      {props.kind}
    </span>
  </Show>
);

const FieldRow: Component<{
  label: string;
  effectiveValue?: string;
  overrideValue?: string;
  curatedValue?: string;
  onChange: (next: string) => void;
}> = (props) => {
  const kind = () =>
    provenanceFor(props.overrideValue, props.curatedValue, props.effectiveValue);
  // Input value: the operator's override if set, otherwise blank so
  // the placeholder (showing the inherited effective value) is what
  // the operator sees. Typing into a blank field promotes the value
  // to an override.
  const inputValue = () => props.overrideValue ?? "";
  const placeholder = () => {
    if (props.overrideValue !== undefined && props.overrideValue !== "") {
      // Currently editing — no inherited value to show as placeholder.
      return "";
    }
    return props.effectiveValue ?? "—";
  };
  return (
    <label class="flex items-center gap-3 py-1">
      <span class="flex w-44 shrink-0 items-center text-[0.7rem] text-(--color-oa-ink-dim)">
        {props.label}
        <ProvenanceBadge kind={kind()} />
      </span>
      <input
        type="text"
        value={inputValue()}
        placeholder={placeholder()}
        onInput={(e) => props.onChange(e.currentTarget.value)}
        class="min-w-0 flex-1 rounded-md border border-white/10 bg-black/40 px-2 py-1 text-[0.75rem] text-(--color-oa-ink) placeholder:text-(--color-oa-ink-dim)/60 focus:border-(--color-system-accent)/60 focus:outline-none"
      />
    </label>
  );
};

const PeripheralEditor: Component<{
  overridden: Peripheral[] | undefined;
  curatedPeripherals: Peripheral[];
  effectivePeripherals: Peripheral[];
  onChange: (next: Peripheral[] | undefined) => void;
}> = (props) => {
  // The override is one of three states:
  //   undefined → "no override; inherit from L2/L1"
  //   []        → "operator says no peripherals" (explicit clear)
  //   [...]     → operator's replacement list
  //
  // For the editor, we operate on a working list. If the override is
  // None, the working list starts from the effective list so the
  // operator can edit IN PLACE; their first change promotes the
  // working list to an override.
  const working = (): Peripheral[] =>
    props.overridden ?? props.effectivePeripherals;
  const isInherited = () => props.overridden === undefined;
  const kind = (): Provenance | null => {
    if (props.overridden !== undefined) return "edited";
    if (props.curatedPeripherals.length > 0) return "curated";
    if (props.effectivePeripherals.length > 0) return "default";
    return null;
  };
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
  const add = () => {
    props.onChange([...working(), { name: "", glyph: "" }]);
  };
  return (
    <div class="flex flex-col gap-2">
      <div class="flex items-center justify-between">
        <span class="flex items-center text-[0.7rem] text-(--color-oa-ink-dim)">
          Peripherals
          <ProvenanceBadge kind={kind()} />
        </span>
        <Show when={!isInherited()}>
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              props.onChange(undefined);
            }}
            class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim) hover:bg-white/[0.08]"
            title="Drop the override; inherit the curated / default peripheral list"
          >
            Inherit
          </button>
        </Show>
      </div>
      <For each={working()}>
        {(row, idx) => (
          <div class="flex items-center gap-2">
            <input
              type="text"
              value={row.glyph}
              maxLength={4}
              onInput={(e) => updateAt(idx(), { ...row, glyph: e.currentTarget.value })}
              placeholder="🎮"
              title="Glyph — single emoji or short symbol"
              class="w-12 shrink-0 rounded-md border border-white/10 bg-black/40 px-2 py-1 text-center text-[0.85rem] text-(--color-oa-ink)"
            />
            <input
              type="text"
              value={row.name}
              onInput={(e) => updateAt(idx(), { ...row, name: e.currentTarget.value })}
              placeholder="Peripheral name"
              class="min-w-0 flex-1 rounded-md border border-white/10 bg-black/40 px-2 py-1 text-[0.75rem] text-(--color-oa-ink) placeholder:text-(--color-oa-ink-dim)/60"
            />
            <button
              type="button"
              onClick={(e) => {
                e.currentTarget.blur();
                remove(idx());
              }}
              class="shrink-0 rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.7rem] text-(--color-oa-ink-dim) hover:border-red-400/40 hover:bg-red-400/10 hover:text-red-200"
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
          add();
        }}
        class="self-start rounded-md border border-dashed border-white/15 bg-white/[0.02] px-3 py-1 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim) hover:border-(--color-system-accent)/40 hover:bg-white/[0.05] hover:text-(--color-oa-ink)"
      >
        + Add peripheral
      </button>
    </div>
  );
};

const PerSystemInfoSection: Component<Props> = (props) => {
  // Three concurrent reads: merged (effective values for the
  // placeholder column), curated (for the "curated" badge), and the
  // override (form state). All three refetch when systemId changes.
  // Save / Reset write through then bump `reloadKey` to refresh the
  // three resources so the form reflects the persisted state.
  const [reloadKey, setReloadKey] = createSignal(0);
  const sourceKey = () => ({ id: props.systemId(), k: reloadKey() });

  const [merged] = createResource(
    sourceKey,
    async ({ id }): Promise<MergedSystemInfo | undefined> => {
      if (!id) return undefined;
      try {
        return await getSystemInfo({ systemId: id });
      } catch (e) {
        console.warn("[PerSystemInfoSection] get_system_info failed:", e);
        return undefined;
      }
    },
  );
  const [curated] = createResource(
    sourceKey,
    async ({ id }): Promise<SystemInfoCurated | null> => {
      if (!id) return null;
      try {
        return await getSystemInfoCurated({ systemId: id });
      } catch (e) {
        console.warn("[PerSystemInfoSection] get_system_info_curated failed:", e);
        return null;
      }
    },
  );
  const [persisted] = createResource(
    sourceKey,
    async ({ id }): Promise<SystemInfoOverride> => {
      if (!id) return EMPTY_SYSTEM_INFO_OVERRIDE;
      try {
        return await getSystemInfoOverride({ systemId: id });
      } catch (e) {
        console.warn("[PerSystemInfoSection] get_system_info_override failed:", e);
        return EMPTY_SYSTEM_INFO_OVERRIDE;
      }
    },
  );

  // The form state is a working copy of the persisted override.
  // createEffect syncs the working copy from the resource each time
  // a new persisted value arrives (system swap, save, reset).
  const [draft, setDraft] = createSignal<SystemInfoOverride>(EMPTY_SYSTEM_INFO_OVERRIDE);
  createEffect(() => {
    const p = persisted();
    if (p) setDraft({ ...p });
  });

  const dirty = createMemo(() => {
    const a = draft();
    const b = persisted();
    if (!b) return false;
    return JSON.stringify(a) !== JSON.stringify(b);
  });

  const updateField = (key: FieldKey, raw: string) => {
    setDraft((prev) => {
      const next = { ...prev };
      if (raw === "") {
        delete next[key];
      } else {
        next[key] = raw;
      }
      return next;
    });
  };

  const updatePeripherals = (next: Peripheral[] | undefined) => {
    setDraft((prev) => {
      const out = { ...prev };
      if (next === undefined) {
        delete out.peripherals;
      } else {
        out.peripherals = next;
      }
      return out;
    });
  };

  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [savedAt, setSavedAt] = createSignal<number | null>(null);

  const handleSave = async () => {
    const sid = props.systemId();
    if (!sid) return;
    setError(null);
    setSaving(true);
    try {
      await setSystemInfoOverride({
        systemId: sid,
        overrideRecord: draft(),
      });
      setReloadKey((k) => k + 1);
      setSavedAt(Date.now());
    } catch (e) {
      console.error("[PerSystemInfoSection] save failed:", e);
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleReset = async () => {
    const sid = props.systemId();
    if (!sid) return;
    if (!confirm("Reset every System Info override for this system?")) return;
    setError(null);
    setSaving(true);
    try {
      await resetSystemInfoToDefault({ systemId: sid });
      setReloadKey((k) => k + 1);
      setSavedAt(Date.now());
    } catch (e) {
      console.error("[PerSystemInfoSection] reset failed:", e);
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const fieldEffective = (key: FieldKey): string | undefined => {
    const m = merged();
    if (!m) return undefined;
    return m[key];
  };
  const fieldCurated = (key: FieldKey): string | undefined => {
    const c = curated();
    if (!c) return undefined;
    return c[key];
  };
  const fieldOverride = (key: FieldKey): string | undefined => draft()[key];

  return (
    <div class="flex flex-col gap-5">
      <p class="text-[0.65rem] leading-relaxed text-(--color-oa-ink-dim)">
        Every field falls back to the curated{" "}
        <code class="text-(--color-oa-ink-dim)/80">
          docs/cores/{props.systemId()}/system-info.yaml
        </code>{" "}
        — and to MAME's baseline below that — when you leave it blank.
        Type a value to override; clear it to inherit again.
      </p>

      <For each={SECTION_FIELDS}>
        {(group) => (
          <div class="flex flex-col">
            <h4 class="mb-2 text-[0.55rem] font-semibold uppercase tracking-[0.4em] text-(--color-system-accent)/70">
              {group.title}
            </h4>
            <For each={group.fields}>
              {(field) => (
                <FieldRow
                  label={field.label}
                  effectiveValue={fieldEffective(field.key)}
                  overrideValue={fieldOverride(field.key)}
                  curatedValue={fieldCurated(field.key)}
                  onChange={(v) => updateField(field.key, v)}
                />
              )}
            </For>
          </div>
        )}
      </For>

      <div class="flex flex-col gap-2">
        <h4 class="text-[0.55rem] font-semibold uppercase tracking-[0.4em] text-(--color-system-accent)/70">
          Peripherals
        </h4>
        <PeripheralEditor
          overridden={draft().peripherals}
          curatedPeripherals={curated()?.peripherals ?? []}
          effectivePeripherals={merged()?.peripherals ?? []}
          onChange={updatePeripherals}
        />
      </div>

      <div class="flex items-center justify-end gap-2 border-t border-white/5 pt-3">
        <Show when={error()}>
          <span class="text-[0.65rem] text-red-300">{error()}</span>
        </Show>
        <Show when={savedAt() && !error()}>
          <span class="text-[0.65rem] text-(--color-oa-ink-dim)">Saved</span>
        </Show>
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            handleReset();
          }}
          disabled={saving()}
          class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim) transition hover:border-red-400/40 hover:bg-red-400/10 hover:text-red-200 disabled:opacity-50"
          title="Drop every System Info override for this system; the panel falls back to L2/L1"
        >
          Reset all overrides
        </button>
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            handleSave();
          }}
          disabled={!dirty() || saving()}
          class="rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/20 px-3 py-1.5 text-[0.65rem] uppercase tracking-widest text-(--color-system-accent-soft) transition hover:bg-(--color-system-accent)/30 disabled:opacity-40"
        >
          {saving() ? "Saving…" : dirty() ? "Save changes" : "Saved"}
        </button>
      </div>
    </div>
  );
};

export default PerSystemInfoSection;
