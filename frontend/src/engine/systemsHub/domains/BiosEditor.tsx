// BIOS domain editor — per-system slice of the global BIOS status. Filters
// get_bios_status to the rows whose slug matches this system; systems that need
// no BIOS say so. Read-only status + a Refresh + the system/ folder hint.
// Persistence unchanged (BIOS files live in <exe_dir>/system/).

import {
  createMemo,
  createResource,
  For,
  Show,
  type Accessor,
  type Component,
} from "solid-js";
import { getBiosStatus } from "@oa/platform/api/coresApi";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import { HubSection, PanelScaffold } from "../PanelScaffold";

type BiosEntryStatus = "ok" | "unknownHash" | "missing" | "error";
type BiosEntry = {
  slug: string;
  label: string;
  required: string;
  status: BiosEntryStatus;
  detail: string;
};
type BiosResp = { systemDir: string; entries: BiosEntry[] };

const PILL: Record<BiosEntryStatus, { label: string; cls: string }> = {
  ok: { label: "Ready", cls: "border-emerald-400/40 bg-emerald-500/10 text-emerald-300" },
  unknownHash: { label: "Unrecognized", cls: "border-amber-400/40 bg-amber-500/10 text-amber-300" },
  missing: { label: "Missing", cls: "border-rose-400/40 bg-rose-500/10 text-rose-300" },
  error: { label: "Error", cls: "border-rose-400/40 bg-rose-500/10 text-rose-300" },
};

export const BiosEditor: Component<{ systemId: Accessor<SystemId> }> = (props) => {
  const [status, { refetch }] = createResource(async (): Promise<BiosResp> => {
    try {
      return await getBiosStatus<BiosResp>();
    } catch (e) {
      console.warn("[per-system-hub] get_bios_status failed:", e);
      return { systemDir: "", entries: [] };
    }
  });
  const rows = createMemo<BiosEntry[]>(() =>
    (status()?.entries ?? []).filter((e) => e.slug === props.systemId()),
  );

  return (
    <PanelScaffold
      system={props.systemId()}
      title={systemThemes[props.systemId()]?.displayName ?? props.systemId()}
      subtitle="BIOS · required firmware status"
    >
      <Show
        when={rows().length > 0}
        fallback={
          <HubSection title="BIOS">
            <p class="text-[0.8rem] text-(--color-oa-ink-dim)">
              This system needs no BIOS files — it runs from the cartridge / disc
              image alone.
            </p>
          </HubSection>
        }
      >
        <HubSection title="Required BIOS">
          <div class="flex flex-col gap-3">
            <div class="flex items-center justify-between gap-3">
              <p class="text-[0.7rem] text-(--color-oa-ink-dim)">
                OA verifies each file's content hash at launch and refuses to
                start with a missing / wrong BIOS.
              </p>
              <button
                type="button"
                disabled={status.loading}
                onClick={(e) => {
                  e.currentTarget.blur();
                  void refetch();
                }}
                class="shrink-0 rounded-md border border-white/10 bg-white/[0.04] px-3 py-1 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink) transition hover:bg-white/[0.08] disabled:opacity-50"
              >
                {status.loading ? "Scanning…" : "Refresh"}
              </button>
            </div>
            <ul class="flex flex-col gap-2">
              <For each={rows()}>
                {(e) => {
                  const pill = PILL[e.status];
                  return (
                    <li class="grid grid-cols-[1fr_auto] gap-x-3 gap-y-1 rounded-lg border border-white/5 bg-white/[0.02] px-3 py-2">
                      <div class="min-w-0">
                        <p class="text-[0.8rem] font-semibold text-(--color-oa-ink)">{e.label}</p>
                        <p class="mt-0.5 truncate text-[0.65rem] text-(--color-oa-ink-dim)" title={e.required}>
                          {e.required}
                        </p>
                      </div>
                      <span class={`self-start rounded border ${pill.cls} px-2 py-0.5 text-[0.55rem] uppercase tracking-widest`}>
                        {pill.label}
                      </span>
                      <Show when={e.detail}>
                        <p class="col-span-2 break-all border-t border-white/5 pt-1.5 font-mono text-[0.6rem] text-(--color-oa-ink-dim)">
                          {e.detail}
                        </p>
                      </Show>
                    </li>
                  );
                }}
              </For>
            </ul>
            <Show when={status()?.systemDir}>
              <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)/70">
                Drop files into:{" "}
                <code class="font-mono normal-case text-(--color-oa-ink-dim)">{status()!.systemDir}</code>
              </p>
            </Show>
          </div>
        </HubSection>
      </Show>
    </PanelScaffold>
  );
};
