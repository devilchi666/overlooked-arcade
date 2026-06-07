// Game ▾ → Properties… consolidated dialog. Slimmed from the old 10-tab
// PerGameSettingsDrawer (UI_POLISH_PLAN.md §D.5) — surviving content is
// Overview (read-only metadata) + Core (per-game core override + IPS/UPS/
// BPS patch picker). Every other tab moved to its own focused dialog in
// GameDialogs.tsx; the Region tab is gone (no runtime effect).
//
// Persistence shape unchanged: core override goes through
// LibraryStore.setCoreOverride (writes the games.core_override column);
// patchPath rides on the existing get_game_overrides / set_game_overrides
// Tauri pair.

import {
  createEffect,
  createSignal,
  For,
  Show,
  type Component,
} from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { Dialog, DialogSection } from "@oa/platform/components/Dialog";
import SettingRow, { selectClass } from "@oa/platform/components/SettingRow";
import { type CoreEntry, type SettingsStore } from "@oa/platform/settings/store";
import { systemThemes } from "@oa/platform/themes/registry";
import type { LibraryStore } from "@oa/platform/library/store";
import type { RomEntry } from "@oa/platform/library/types";
import type { GameOverrides } from "./GameDialogs";

type Props = {
  open: boolean;
  entry: RomEntry | null;
  onClose: () => void;
  settings: SettingsStore;
  library: LibraryStore;
};

const GamePropertiesDialog: Component<Props> = (props) => {
  const [overrides, setOverrides] = createSignal<GameOverrides>({});
  const [cores, setCores] = createSignal<CoreEntry[]>([]);
  const [systemCorePref, setSystemCorePref] = createSignal<string | null>(null);

  createEffect(() => {
    const e = props.entry;
    if (!props.open || !e) return;
    void (async () => {
      try {
        const got = await invoke<GameOverrides>("get_game_overrides", { id: e.id });
        setOverrides(got ?? {});
      } catch (err) {
        console.warn("[oa-properties] get_game_overrides failed:", err);
        setOverrides({});
      }
      try {
        const list = await invoke<CoreEntry[]>("list_cores");
        setCores(list ?? []);
      } catch {
        setCores([]);
      }
      try {
        const v = await invoke<string | null>("get_core_pref", { systemId: e.systemId });
        setSystemCorePref(v ?? null);
      } catch {
        setSystemCorePref(null);
      }
    })();
  });

  async function patch(p: Partial<GameOverrides>) {
    const e = props.entry;
    if (!e) return;
    const next: GameOverrides = { ...overrides(), ...p };
    // Properties only touches patchPath here — other override fields are
    // owned by the GameDialogs surfaces. Merge instead of replace so we
    // don't accidentally clobber the rest of the overrides JSON.
    const cleaned: GameOverrides = { ...overrides() };
    if (next.patchPath != null && next.patchPath !== "") {
      cleaned.patchPath = next.patchPath;
    } else {
      delete cleaned.patchPath;
    }
    setOverrides(cleaned);
    try {
      await invoke("set_game_overrides", { id: e.id, overrides: cleaned });
    } catch (err) {
      console.warn("[oa-properties] set_game_overrides failed:", err);
    }
  }

  async function patchCoreOverride(fileName: string | null) {
    const e = props.entry;
    if (!e) return;
    try {
      await props.library.setCoreOverride(e.id, fileName);
    } catch (err) {
      console.warn("[oa-properties] setCoreOverride failed:", err);
    }
  }

  function inheritedCore(): { label: string; from: string } {
    const sysCore = systemCorePref();
    if (sysCore) {
      const found = cores().find((c) => c.fileName === sysCore);
      return {
        label: found ? `${found.libraryName} (${found.libraryVersion})` : sysCore,
        from: "Per-system",
      };
    }
    const first = cores()[0];
    return {
      label: first ? first.libraryName || first.fileName : "Auto-detect",
      from: "Auto-detect",
    };
  }

  const theme = () => {
    const e = props.entry;
    if (!e) return null;
    return systemThemes[e.systemId];
  };

  const dialogTitle = () => {
    const e = props.entry;
    if (!e) return "Properties";
    return `Properties — ${e.title}`;
  };

  return (
    <Dialog
      open={props.open}
      onClose={props.onClose}
      title={dialogTitle()}
      subtitle={theme()?.displayName ?? props.entry?.systemId}
      system={props.entry?.systemId}
      size="xl"
    >
      <Show when={props.entry}>
        {(e) => (
          <div class="flex flex-col gap-5">
            <DialogSection title="Overview">
              <dl class="grid grid-cols-[7rem_1fr] gap-x-3 gap-y-1.5 text-sm">
                <dt class="text-(--color-oa-ink-dim)">Title</dt>
                <dd class="text-(--color-oa-ink)">{e().title}</dd>
                <dt class="text-(--color-oa-ink-dim)">System</dt>
                <dd class="text-(--color-oa-ink)">
                  {theme()?.displayName ?? e().systemId}
                </dd>
                <dt class="text-(--color-oa-ink-dim)">ROM path</dt>
                <dd class="truncate font-mono text-xs text-(--color-oa-ink)" title={e().filePath}>
                  {e().filePath}
                </dd>
                <Show when={e().archiveInnerPath}>
                  <dt class="text-(--color-oa-ink-dim)">In archive</dt>
                  <dd class="truncate font-mono text-xs text-(--color-oa-ink)">
                    {e().archiveInnerPath}
                  </dd>
                </Show>
                <dt class="text-(--color-oa-ink-dim)">Added</dt>
                <dd class="text-(--color-oa-ink)">
                  {new Date(e().addedAt).toLocaleDateString()}
                </dd>
              </dl>
              <p class="rounded-md border border-white/5 bg-black/20 px-3 py-2 text-xs text-(--color-oa-ink-dim)">
                Custom user fields (tags, personal notes) and rich metadata editing land
                alongside the per-game activity log in Phase 4.
              </p>
            </DialogSection>

            <DialogSection title="Core">
              <SettingRow
                label="Core override"
                hint="Used instead of the per-system default for this game only"
                inheritedValue={inheritedCore().label}
                inheritedFrom={inheritedCore().from}
                overridden={!!e().coreOverride}
              >
                <select
                  class={selectClass("system")}
                  value={e().coreOverride ?? ""}
                  onChange={(ev) => {
                    const v = ev.currentTarget.value;
                    void patchCoreOverride(v === "" ? null : v);
                  }}
                >
                  <option value="">— Use per-system / auto —</option>
                  <For each={cores()}>
                    {(c) => (
                      <option value={c.fileName}>
                        {c.libraryName} ({c.libraryVersion})
                      </option>
                    )}
                  </For>
                </select>
              </SettingRow>
              <p class="text-xs leading-relaxed text-(--color-oa-ink-dim)">
                Takes effect on next launch. Right-click the tile → "Change core…"
                reaches the same setting.
              </p>

              <SettingRow
                label="ROM patch"
                hint="IPS / UPS / BPS patch applied to ROM bytes before the core sees them"
                inheritedValue="No patch"
                overridden={overrides().patchPath != null}
              >
                <div class="flex items-center gap-2">
                  <span
                    class="flex-1 truncate text-sm"
                    classList={{
                      "text-(--color-oa-ink)": overrides().patchPath != null,
                      "text-(--color-oa-ink-dim)": overrides().patchPath == null,
                    }}
                    title={overrides().patchPath ?? "No patch"}
                  >
                    {overrides().patchPath
                      ? overrides().patchPath!.split(/[\/\\]/).pop()
                      : "No patch selected"}
                  </span>
                  <button
                    type="button"
                    class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) hover:bg-white/[0.08]"
                    onClick={async (ev) => {
                      ev.currentTarget.blur();
                      try {
                        const picked = await invoke<string | null>("pick_patch_file");
                        if (picked) void patch({ patchPath: picked });
                      } catch (err) {
                        console.warn("[oa-properties] pick_patch_file failed:", err);
                      }
                    }}
                  >
                    Pick…
                  </button>
                  <Show when={overrides().patchPath != null}>
                    <button
                      type="button"
                      class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink-dim) hover:bg-white/[0.08]"
                      onClick={() => void patch({ patchPath: null })}
                    >
                      Clear
                    </button>
                  </Show>
                </div>
              </SettingRow>
              <p class="text-xs leading-relaxed text-(--color-oa-ink-dim)">
                Takes effect on next launch. Byte-source ROMs only (HuCards, NES, SNES
                carts); CD images can't be patched in-place from this side.
              </p>
            </DialogSection>
          </div>
        )}
      </Show>
    </Dialog>
  );
};

export default GamePropertiesDialog;
