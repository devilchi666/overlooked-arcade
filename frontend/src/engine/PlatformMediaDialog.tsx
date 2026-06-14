// Library Manager → Game media → "Platform media…"
//
// Per-system hardware-photo / controller / wheel / banner / etc. management.
// The actual slot grid lives in PlatformMediaSlots (shared with the Systems hub
// Media domain); this component owns the system <select> + the Dialog / panel
// chrome and delegates the slots to PlatformMediaSlots.

import { createSignal, For, Show, type Component, type JSX } from "solid-js";
import { Dialog } from "@oa/platform/components/Dialog";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import { PlatformMediaSlots } from "./PlatformMediaSlots";

type Props = {
  open: boolean;
  onClose: () => void;
  /// Optional initial system to focus when opening. Defaults to the first
  /// system in the registry.
  initialSystemId?: SystemId;
  /// "dialog" (default) wraps the body in the Dialog primitive — the legacy
  /// Library Manager → Game media → Platform media… entry. "panel" drops the
  /// Dialog wrapper + Close so the body embeds inside a parent shell
  /// (Retroverse-UI SETTINGS → Media category). In panel mode `open`/`onClose`
  /// are ignored.
  variant?: "dialog" | "panel";
};

const ALL_SYSTEM_IDS = Object.keys(systemThemes) as SystemId[];

export const PlatformMediaDialog: Component<Props> = (props) => {
  const [systemId, setSystemId] = createSignal<SystemId>(
    props.initialSystemId ?? ALL_SYSTEM_IDS[0],
  );

  const body: JSX.Element = (
    <div class="space-y-4 p-4">
      <div class="space-y-1">
        <label class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
          System
        </label>
        <select
          value={systemId()}
          onChange={(e) => setSystemId(e.currentTarget.value as SystemId)}
          class="w-full rounded border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs text-(--color-oa-ink)"
        >
          <For each={ALL_SYSTEM_IDS}>
            {(sid) => (
              <option value={sid}>
                {systemThemes[sid].displayName} ({sid})
              </option>
            )}
          </For>
        </select>
      </div>

      <PlatformMediaSlots systemId={systemId} />

      <Show when={props.variant !== "panel"}>
        <div class="flex justify-end gap-2 pt-2">
          <button
            type="button"
            onClick={props.onClose}
            class="rounded border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
          >
            Close
          </button>
        </div>
      </Show>
    </div>
  );

  return (
    <Show when={props.variant !== "panel"} fallback={body}>
      <Dialog
        open={props.open}
        onClose={props.onClose}
        title="Platform media"
        subtitle="Per-system hardware photos, controllers, wheel art, banners"
        size="xl"
      >
        {body}
      </Dialog>
    </Show>
  );
};
