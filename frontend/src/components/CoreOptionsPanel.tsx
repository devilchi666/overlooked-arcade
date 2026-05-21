import { createResource, createSignal, For, Show, type Component } from "solid-js";
import { invoke } from "@tauri-apps/api/core";

type CoreOptionValue = {
  value: string;
  label: string | null;
};

type CoreOption = {
  key: string;
  desc: string;
  info: string | null;
  categoryKey: string | null;
  defaultValue: string;
  values: CoreOptionValue[];
};

type CoreOptionCategory = {
  key: string;
  desc: string;
  info: string | null;
};

type CoreOptionsSnapshot = {
  schema: CoreOption[];
  categories: CoreOptionCategory[];
  systemValues: Record<string, string>;
  gameValues: Record<string, string>;
  /// Option keys the core wants hidden — libretro
  /// SET_CORE_OPTIONS_DISPLAY parity. Filtered out at render time so
  /// the user only sees options that actually apply given the current
  /// configuration (e.g. "Lightgun crosshair color" disappears when
  /// "Lightgun" is off). Cores without dynamic visibility return [].
  hiddenKeys: string[];
};

type Props = {
  systemId: string;
  /// When null, this is a per-system view. When set, the panel is rendered
  /// inside the per-game drawer: changes go to `set_game_core_option` and
  /// the per-system value renders as the inherited chip alongside each
  /// dropdown.
  gameId: string | null;
};

const SELECT_CLASS =
  "rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) transition hover:bg-white/[0.08] focus-visible:outline focus-visible:outline-2 focus-visible:outline-(--color-system-accent) disabled:opacity-50";

/// RetroArch-parity slice — render the libretro core-options table for a
/// system, with per-system or per-game overrides depending on the `gameId`
/// prop. Inherited chips show "Per-system: …" / "Default: …" so the user
/// always sees what they'd fall back to on Reset.
///
/// The schema comes from disk (`appData/core-options/<systemId>.json`),
/// captured every time a core loads. If the user opens this tab before
/// ever launching a game for this system, they get an "options will appear
/// after first launch" hint rather than an empty page.
const CoreOptionsPanel: Component<Props> = (props) => {
  const [snapshot, { refetch }] = createResource(
    () => ({ sys: props.systemId, game: props.gameId }),
    async (k) => {
      return await invoke<CoreOptionsSnapshot>("list_core_options", {
        systemId: k.sys,
        gameId: k.game,
      });
    },
  );
  const [filter, setFilter] = createSignal("");

  function resolveInherited(opt: CoreOption): string {
    const snap = snapshot();
    if (!snap) return opt.defaultValue;
    // Per-game override row inherits per-system → schema default.
    // Per-system override row inherits schema default.
    if (props.gameId) {
      return snap.systemValues[opt.key] ?? opt.defaultValue;
    }
    return opt.defaultValue;
  }

  function currentValue(opt: CoreOption): string {
    const snap = snapshot();
    if (!snap) return opt.defaultValue;
    if (props.gameId && snap.gameValues[opt.key] !== undefined) {
      return snap.gameValues[opt.key];
    }
    if (snap.systemValues[opt.key] !== undefined) {
      return snap.systemValues[opt.key];
    }
    return opt.defaultValue;
  }

  function isOverridden(opt: CoreOption): boolean {
    const snap = snapshot();
    if (!snap) return false;
    if (props.gameId) return snap.gameValues[opt.key] !== undefined;
    return snap.systemValues[opt.key] !== undefined;
  }

  async function setValue(opt: CoreOption, value: string | null) {
    try {
      if (props.gameId) {
        await invoke("set_game_core_option", {
          gameId: props.gameId,
          key: opt.key,
          value,
        });
      } else {
        await invoke("set_system_core_option", {
          systemId: props.systemId,
          key: opt.key,
          value,
        });
      }
      await refetch();
    } catch (e) {
      console.warn("[core-options] set failed:", e);
    }
  }

  const filteredOptions = (): CoreOption[] => {
    const snap = snapshot();
    if (!snap) return [];
    const hidden = new Set(snap.hiddenKeys);
    const visible = snap.schema.filter((o) => !hidden.has(o.key));
    const q = filter().trim().toLowerCase();
    if (!q) return visible;
    return visible.filter((o) =>
      o.desc.toLowerCase().includes(q) || o.key.toLowerCase().includes(q),
    );
  };

  return (
    <div class="flex flex-col gap-3">
      <Show when={snapshot()}>
        {(snap) => (
          <>
            <Show when={snap().schema.length === 0}>
              <div class="rounded-md border border-white/5 bg-white/[0.02] p-3 text-xs text-(--color-oa-ink-dim)">
                No core options captured yet. Launch a game for this system once;
                its core will register the option schema on load, and it'll appear
                here on next visit.
              </div>
            </Show>
            <Show when={snap().schema.length > 0}>
              <div class="flex items-center gap-2">
                <input
                  type="text"
                  placeholder="Filter options…"
                  value={filter()}
                  onInput={(e) => setFilter(e.currentTarget.value)}
                  class="flex-1 rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-sm text-(--color-oa-ink) focus-visible:border-(--color-system-accent) focus-visible:outline-none"
                />
                <span class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim) tabular-nums">
                  {filteredOptions().length} / {snap().schema.length - snap().hiddenKeys.length}
                </span>
              </div>
              <div class="flex max-h-[60vh] flex-col gap-2 overflow-y-auto pr-1">
                <For each={filteredOptions()}>
                  {(opt) => (
                    <div class="rounded border border-white/5 bg-white/[0.02] p-2">
                      <div class="flex items-baseline justify-between gap-3">
                        <p
                          class="truncate text-sm text-(--color-oa-ink)"
                          title={opt.info ?? undefined}
                        >
                          {opt.desc}
                        </p>
                        <Show when={isOverridden(opt)}>
                          <button
                            type="button"
                            onClick={() => void setValue(opt, null)}
                            class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)"
                          >
                            Reset
                          </button>
                        </Show>
                      </div>
                      <div class="mt-1 flex items-center gap-3">
                        <select
                          class={SELECT_CLASS + " flex-1"}
                          value={currentValue(opt)}
                          onChange={(e) => void setValue(opt, e.currentTarget.value)}
                        >
                          <For each={opt.values}>
                            {(v) => (
                              <option value={v.value}>{v.label ?? v.value}</option>
                            )}
                          </For>
                        </select>
                        <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim) min-w-[8rem] text-right">
                          {isOverridden(opt)
                            ? `inherited: ${resolveInherited(opt)}`
                            : props.gameId
                            ? snap().systemValues[opt.key] !== undefined
                              ? `per-system: ${snap().systemValues[opt.key]}`
                              : `default: ${opt.defaultValue}`
                            : `default: ${opt.defaultValue}`}
                        </span>
                      </div>
                      <Show when={opt.info}>
                        <p class="mt-1 text-[0.6rem] text-(--color-oa-ink-dim)">
                          {opt.info}
                        </p>
                      </Show>
                    </div>
                  )}
                </For>
              </div>
            </Show>
          </>
        )}
      </Show>
    </div>
  );
};

export default CoreOptionsPanel;
