// External-emulator config — standalone-emulator profiles from
// config/emulators/*.yaml: per-profile binary path + per-system "Default
// launcher". Settings IA Slice 4 relocated this here (from CoresPage) so the
// External Emulators tab is the single home for standalone-emulator setup;
// CoresPage no longer carries a duplicate. Self-contained: owns its own
// profiles / launcher-pref resources + busy/status state.
//
// The install pipeline (one-click download/setup) rides Virtual-Library Phase D
// (unbuilt); this surface is bring-your-own-binary today.

import {
  createResource,
  createSignal,
  For,
  Show,
  type Component,
} from "solid-js";
import * as emulatorApi from "@oa/platform/api/emulatorApi";
import { open as pickFile } from "@tauri-apps/plugin-dialog";
import { systemThemes } from "@oa/platform/themes/registry";

/// Mirror of Rust's `EmulatorProfileInfo` (VL Phase C2) — one external
/// standalone-emulator profile, with the effective binary path resolved
/// (appData pref → profile field).
type EmulatorProfileInfo = {
  id: string;
  displayName: string;
  vendor: string;
  officialDownloadUrl: string;
  binaryName: string;
  supportedSystems: string[];
  binaryPath: string | null;
};

/// Display label for a system slug — wired registry name first, raw slug last.
/// (The CoresPage version also consulted a queued-label map; external systems
/// that aren't yet wired show their slug here, which improves as they're added.)
const externalSystemLabel = (id: string): string =>
  (systemThemes as Record<string, { displayName: string } | undefined>)[id]?.displayName ??
  id;

export const ExternalEmulatorsSection: Component = () => {
  const [profilesTick, setProfilesTick] = createSignal(0);
  const [profiles] = createResource(profilesTick, async (): Promise<EmulatorProfileInfo[]> => {
    try {
      return await emulatorApi.listEmulatorProfiles<EmulatorProfileInfo>();
    } catch (e) {
      console.warn("list_emulator_profiles failed:", e);
      return [];
    }
  });

  const [launcherTick, setLauncherTick] = createSignal(0);
  const [launcherPrefs] = createResource(
    () => [launcherTick(), profiles()] as const,
    async ([, profs]): Promise<Record<string, string | null>> => {
      const result: Record<string, string | null> = {};
      const systems = new Set<string>();
      for (const p of profs ?? []) for (const s of p.supportedSystems) systems.add(s);
      for (const id of systems) {
        try {
          result[id] = (await emulatorApi.getLauncherPref(id)) ?? null;
        } catch (e) {
          console.warn(`get_launcher_pref(${id}) failed:`, e);
          result[id] = null;
        }
      }
      return result;
    },
  );

  const [busy, setBusy] = createSignal<string | null>(null);
  const [status, setStatus] = createSignal<string>("");

  async function handlePickEmulatorBinary(p: EmulatorProfileInfo) {
    const picked = await pickFile({
      multiple: false,
      filters: [{ name: p.binaryName, extensions: ["exe"] }],
    }).catch((e) => {
      console.warn("pickFile failed:", e);
      return null;
    });
    if (!picked || Array.isArray(picked)) return;
    setBusy(`emu-${p.id}`);
    try {
      await emulatorApi.setEmulatorBinaryPath(p.id, picked);
      setStatus(`${p.displayName} binary path set.`);
      setProfilesTick((n) => n + 1);
    } catch (e) {
      setStatus(`Set binary path failed: ${String(e)}`);
    } finally {
      setBusy(null);
    }
  }

  async function handleClearEmulatorBinary(p: EmulatorProfileInfo) {
    setBusy(`emu-${p.id}`);
    try {
      await emulatorApi.setEmulatorBinaryPath(p.id, null);
      setStatus(`${p.displayName} binary path cleared.`);
      setProfilesTick((n) => n + 1);
    } catch (e) {
      setStatus(`Clear binary path failed: ${String(e)}`);
    } finally {
      setBusy(null);
    }
  }

  async function handleSetLauncherPref(systemId: string, profileId: string | null) {
    setBusy(`launcher-${systemId}`);
    try {
      await emulatorApi.setLauncherPref(systemId, profileId);
      setLauncherTick((n) => n + 1);
    } catch (e) {
      setStatus(`Set default launcher failed: ${String(e)}`);
    } finally {
      setBusy(null);
    }
  }

  return (
    <div class="flex flex-col gap-3" data-external-emulators>
      <div class="flex items-baseline justify-between">
        <h3 class="text-[0.7rem] uppercase tracking-[0.3em] text-(--color-oa-ink-dim)">
          Standalone emulators
        </h3>
        <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          config/emulators/
        </span>
      </div>

      <Show
        when={(profiles() ?? []).length > 0}
        fallback={
          <p class="text-sm text-(--color-oa-ink-dim)">
            No external-emulator profiles found. Profiles ship in{" "}
            <span class="font-mono">config/emulators/*.yaml</span> as support grows.
          </p>
        }
      >
        <For each={profiles() ?? []}>
          {(p) => (
            <article class="rounded-lg border border-white/10 bg-black/20 p-4">
              <header class="flex items-start justify-between gap-3">
                <div class="min-w-0">
                  <h3 class="truncate text-sm font-semibold text-(--color-oa-ink)">
                    {p.displayName}
                  </h3>
                  <p class="mt-0.5 truncate text-[0.7rem] text-(--color-oa-ink-dim)">
                    <Show when={p.vendor}>
                      <span>{p.vendor} · </span>
                    </Show>
                    <span>
                      runs {p.supportedSystems.map(externalSystemLabel).join(", ")} as its
                      own process
                    </span>
                  </p>
                </div>
                <Show when={p.officialDownloadUrl}>
                  <a
                    href={p.officialDownloadUrl}
                    target="_blank"
                    rel="noreferrer"
                    class="shrink-0 rounded-md border border-white/10 bg-white/[0.03] px-2.5 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                    title={p.officialDownloadUrl}
                  >
                    Download ↗
                  </a>
                </Show>
              </header>

              {/* Binary path — where the operator's install lives. */}
              <div class="mt-3 flex items-center gap-3">
                <label class="shrink-0 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                  Binary path
                </label>
                <Show
                  when={p.binaryPath}
                  fallback={
                    <span class="flex-1 truncate text-xs text-amber-300/90">
                      Not set — pick your {p.binaryName} to enable launching
                    </span>
                  }
                >
                  <span
                    class="flex-1 truncate font-mono text-xs text-(--color-oa-ink)"
                    title={p.binaryPath!}
                  >
                    {p.binaryPath}
                  </span>
                </Show>
                <button
                  type="button"
                  onClick={() => void handlePickEmulatorBinary(p)}
                  disabled={busy() === `emu-${p.id}`}
                  class="shrink-0 rounded-md border border-white/10 bg-white/[0.03] px-2.5 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:opacity-50"
                >
                  Pick…
                </button>
                <Show when={p.binaryPath}>
                  <button
                    type="button"
                    onClick={() => void handleClearEmulatorBinary(p)}
                    disabled={busy() === `emu-${p.id}`}
                    class="shrink-0 rounded-md border border-white/10 bg-white/[0.03] px-2.5 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:border-red-500/40 hover:bg-red-950/30 hover:text-red-300 disabled:opacity-50"
                  >
                    Clear
                  </button>
                </Show>
              </div>

              {/* Per-system default launcher — unset = libretro core, today's
                  behavior. Takes effect on the next launch. */}
              <For each={p.supportedSystems}>
                {(sysId) => (
                  <div class="mt-3 flex items-center gap-3">
                    <label class="shrink-0 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                      Default launcher · {externalSystemLabel(sysId)}
                    </label>
                    <select
                      class="rounded border border-white/10 bg-black/40 px-2 py-1 text-xs text-(--color-oa-ink)"
                      disabled={busy() === `launcher-${sysId}`}
                      value={launcherPrefs()?.[sysId] === p.id ? p.id : ""}
                      onChange={(e) => {
                        const v = e.currentTarget.value;
                        void handleSetLauncherPref(sysId, v === "" ? null : v);
                      }}
                    >
                      <option value="">Libretro core (default)</option>
                      <option value={p.id}>{p.displayName} (standalone)</option>
                    </select>
                    <Show when={launcherPrefs()?.[sysId] === p.id && !p.binaryPath}>
                      <span class="rounded border border-amber-500/30 bg-amber-500/10 px-1.5 py-0.5 text-[0.55rem] uppercase tracking-widest text-amber-300">
                        set the binary path first
                      </span>
                    </Show>
                  </div>
                )}
              </For>
            </article>
          )}
        </For>
      </Show>

      <Show when={status()}>
        <p class="text-[0.65rem] text-(--color-oa-ink-dim)">{status()}</p>
      </Show>
    </div>
  );
};

export default ExternalEmulatorsSection;
