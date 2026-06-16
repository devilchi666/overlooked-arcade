// Engine territory — SETTINGS → System → Privacy.
//
// oa-packs arc Slice 4. The transparency surface for OA's network posture
// (content-packs.md §3, §9): OA never contacts a server unless the operator
// asks. This panel discloses exactly which URLs OA will hit and when, hosts
// the master "Allow network calls" toggle (mirrored from the Packs panel —
// both write the same pref), and shows a per-call audit log with a Clear.
//
// All Tauri calls go through `@oa/platform/api/packsApi` (no raw invoke).

import {
  createResource,
  createSignal,
  For,
  Show,
  type Component,
} from "solid-js";
import * as packsApi from "@oa/platform/api/packsApi";
import { pushToast } from "@oa/platform/lib/toast";
import { confirm } from "@oa/platform/lib/confirm";
import SettingRow from "@oa/platform/components/SettingRow";

const btnClass =
  "rounded-md border border-white/10 bg-white/[0.04] px-3 py-1 text-[0.7rem] font-medium uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:cursor-not-allowed disabled:opacity-40";

const PrivacySettings: Component = () => {
  const [prefs, { refetch: refetchPrefs }] = createResource(async () => {
    try {
      return await packsApi.getPrefs();
    } catch (e) {
      console.warn("[oa-packs] get_prefs failed:", e);
      return null;
    }
  });
  const [log, { refetch: refetchLog }] = createResource(async () => {
    try {
      return await packsApi.getNetworkLog();
    } catch (e) {
      console.warn("[oa-packs] get_network_log failed:", e);
      return [] as packsApi.NetLogEntry[];
    }
  });
  const [clearing, setClearing] = createSignal(false);

  async function toggleNetwork(v: boolean) {
    try {
      await packsApi.setAllowNetwork(v);
      void refetchPrefs();
    } catch (e) {
      pushToast("error", `Couldn't change the network setting: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function clearLog() {
    if (
      !(await confirm("Clear the network log?", {
        title: "Clear network log",
        confirmLabel: "Clear",
        danger: true,
      }))
    ) {
      return;
    }
    setClearing(true);
    try {
      await packsApi.clearNetworkLog();
      void refetchLog();
    } catch (e) {
      pushToast("error", `Couldn't clear the log: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setClearing(false);
    }
  }

  return (
    <div class="flex flex-col gap-4">
      <p class="text-sm leading-relaxed text-(--color-oa-ink)">
        OA never contacts any server unless you ask it to. There is no
        telemetry, no analytics, and no operator ID, machine fingerprint, or
        location ever sent — pack downloads are anonymous HTTP GETs.
      </p>

      {/* Disclosure */}
      <section class="rounded-xl border border-white/10 bg-white/[0.02] p-4">
        <h3 class="mb-2 text-sm font-semibold text-(--color-oa-ink)">
          When OA makes a network call
        </h3>
        <p class="mb-3 text-xs text-(--color-oa-ink-dim)">
          Only these two actions, both from Settings → Content → Packs, reach
          out — and only when you click them:
        </p>
        <ul class="flex flex-col gap-2 text-xs">
          <li class="rounded-md border border-white/5 bg-white/[0.02] px-3 py-2">
            <span class="font-medium text-(--color-oa-ink)">Browse / Check for updates</span>
            <span class="text-(--color-oa-ink-dim)"> → fetches the registry:</span>
            <div class="mt-1 break-all font-mono text-[0.7rem] text-(--color-oa-ink-dim)">
              {prefs()?.registryUrl ?? "…"}
            </div>
          </li>
          <li class="rounded-md border border-white/5 bg-white/[0.02] px-3 py-2">
            <span class="font-medium text-(--color-oa-ink)">Install / Update</span>
            <span class="text-(--color-oa-ink-dim)">
              {" "}
              → downloads the specific pack's zip from the URL the registry
              lists for it (always pinned to a published release).
            </span>
          </li>
        </ul>
      </section>

      {/* Master toggle */}
      <section class="rounded-xl border border-white/10 bg-white/[0.02] p-4">
        <SettingRow
          label="Allow network calls"
          inherited={null}
          overridden={false}
          toggle={{
            checked: prefs()?.allowNetwork ?? true,
            onChange: (v) => void toggleNetwork(v),
          }}
          description="Default on. When off, Browse / Install / Update are refused before any request leaves your machine. This is the same switch shown in Content → Packs."
        />
      </section>

      {/* Network log */}
      <section class="rounded-xl border border-white/10 bg-white/[0.02] p-4">
        <header class="mb-3 flex items-center justify-between gap-3">
          <div>
            <h3 class="text-sm font-semibold text-(--color-oa-ink)">Network log</h3>
            <p class="mt-0.5 text-xs text-(--color-oa-ink-dim)">
              Every URL OA has hit, most recent first. Last 100 calls.
            </p>
          </div>
          <div class="flex items-center gap-2">
            <button type="button" class={btnClass} onClick={() => void refetchLog()}>
              ↻ Refresh
            </button>
            <button
              type="button"
              class={btnClass}
              disabled={clearing() || (log() ?? []).length === 0}
              onClick={() => void clearLog()}
            >
              Clear
            </button>
          </div>
        </header>
        <Show
          when={(log() ?? []).length > 0}
          fallback={
            <p class="rounded-md border border-dashed border-white/10 bg-white/[0.01] px-3 py-4 text-center text-xs text-(--color-oa-ink-dim)">
              No network calls yet. The log fills in when you Browse or install
              a pack.
            </p>
          }
        >
          <div class="flex flex-col gap-1.5">
            <For each={log() ?? []}>
              {(entry) => (
                <div class="flex flex-wrap items-center gap-2 rounded-md border border-white/5 bg-white/[0.02] px-3 py-2 text-xs">
                  <span
                    class="rounded px-1.5 py-0.5 text-[0.6rem] font-semibold uppercase tracking-wider"
                    classList={{
                      "bg-emerald-500/15 text-emerald-300/90": entry.outcome === "ok",
                      "bg-red-500/15 text-red-300/90": entry.outcome !== "ok",
                    }}
                    title={entry.detail ?? entry.outcome}
                  >
                    {entry.outcome}
                  </span>
                  <span class="text-(--color-oa-ink-dim)">{entry.action}</span>
                  <span class="min-w-0 flex-1 break-all font-mono text-[0.7rem] text-(--color-oa-ink)">
                    {entry.url}
                  </span>
                  <Show when={entry.at}>
                    <span class="text-[0.6rem] text-(--color-oa-ink-dim)">
                      {new Date(entry.at!).toLocaleString()}
                    </span>
                  </Show>
                </div>
              )}
            </For>
          </div>
        </Show>
      </section>
    </div>
  );
};

export default PrivacySettings;
