// Engine territory — SETTINGS → Content → Packs.
//
// oa-packs arc Slice 3. The operator-facing surface for the content-pack
// distribution channel: Installed / Available / Updates / Rollback sections
// plus the config registry URL (CP1) and the master network toggle (§9).
//
// Network discipline (content-packs.md §3): the registry is fetched ONLY
// when the operator clicks "Browse" — never on mount. The local sections
// (Installed / Rollback) load on mount because they touch no network.
//
// All Tauri calls go through `@oa/platform/api/packsApi` (no raw invoke here).

import {
  createMemo,
  createResource,
  createSignal,
  For,
  Show,
  type Component,
  type JSX,
} from "solid-js";
import * as packsApi from "@oa/platform/api/packsApi";
import { pushToast } from "@oa/platform/lib/toast";
import { confirm } from "@oa/platform/lib/confirm";
import SettingRow from "@oa/platform/components/SettingRow";

// Dotted-numeric version compare (mirrors oa_packs::compare_versions) — used
// only to decide which installed packs have a newer registry version.
function cmpVer(a: string, b: string): number {
  const seg = (s: string) => s.split(/[-+]/)[0].split(".").map((n) => parseInt(n, 10) || 0);
  const pa = seg(a);
  const pb = seg(b);
  const len = Math.max(pa.length, pb.length);
  for (let i = 0; i < len; i++) {
    const x = pa[i] ?? 0;
    const y = pb[i] ?? 0;
    if (x !== y) return x < y ? -1 : 1;
  }
  return 0;
}

const btnClass =
  "rounded-md border border-white/10 bg-white/[0.04] px-3 py-1 text-[0.7rem] font-medium uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:cursor-not-allowed disabled:opacity-40";
const dangerBtnClass =
  "rounded-md border border-red-500/20 bg-red-500/[0.06] px-3 py-1 text-[0.7rem] font-medium uppercase tracking-wider text-red-300/80 transition hover:bg-red-500/[0.14] hover:text-red-200 disabled:cursor-not-allowed disabled:opacity-40";

const Card: Component<{ title: string; subtitle?: string; children: JSX.Element; right?: JSX.Element }> = (
  props,
) => (
  <section class="rounded-xl border border-white/10 bg-white/[0.02] p-4">
    <header class="mb-3 flex items-start justify-between gap-3">
      <div>
        <h3 class="text-sm font-semibold text-(--color-oa-ink)">{props.title}</h3>
        <Show when={props.subtitle}>
          <p class="mt-0.5 text-xs text-(--color-oa-ink-dim)">{props.subtitle}</p>
        </Show>
      </div>
      <Show when={props.right}>{props.right}</Show>
    </header>
    {props.children}
  </section>
);

const TypeChip: Component<{ type: string }> = (props) => (
  <span class="rounded border border-white/10 bg-white/[0.05] px-1.5 py-0.5 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim)">
    {props.type}
  </span>
);

const PacksSettings: Component = () => {
  // --- Local state (loads on mount; no network) ---
  const [installed, { refetch: refetchInstalled }] = createResource(async () => {
    try {
      return await packsApi.listInstalled();
    } catch (e) {
      console.warn("[oa-packs] list failed:", e);
      return [] as packsApi.InstalledPack[];
    }
  });
  const [rollbacks, { refetch: refetchRollbacks }] = createResource(async () => {
    try {
      return await packsApi.listRollbacks();
    } catch (e) {
      console.warn("[oa-packs] list_rollbacks failed:", e);
      return [] as packsApi.RollbackEntry[];
    }
  });
  const [prefs, { refetch: refetchPrefs }] = createResource(async () => {
    try {
      return await packsApi.getPrefs();
    } catch (e) {
      console.warn("[oa-packs] get_prefs failed:", e);
      return null;
    }
  });
  const [recipes, { refetch: refetchRecipes }] = createResource(async () => {
    try {
      return await packsApi.recipeOverrides();
    } catch (e) {
      console.warn("[oa-packs] recipe_overrides failed:", e);
      return null;
    }
  });
  const hasRecipeInfo = () =>
    (recipes()?.overrides.length ?? 0) > 0 || (recipes()?.conflicts.length ?? 0) > 0;

  // After any pack change, hot-reload the recipe override tier so the
  // overrides section (and External Emulators) reflect it without a restart.
  async function syncRecipes() {
    try {
      await packsApi.reloadRecipes();
    } catch (e) {
      console.warn("[oa-packs] reload_recipes failed:", e);
    }
    void refetchRecipes();
  }

  // --- Registry (operator-initiated only) ---
  const [registry, setRegistry] = createSignal<packsApi.Registry | null>(null);
  const [registryError, setRegistryError] = createSignal<string | null>(null);
  const [browsing, setBrowsing] = createSignal(false);

  // --- Per-action busy key (e.g. "install:oa-editorial-baseline") ---
  const [busy, setBusy] = createSignal<string | null>(null);

  // --- Registry-URL editor (seeded from prefs, editable) ---
  const [urlDraft, setUrlDraft] = createSignal<string | null>(null);
  const effectiveUrl = () => urlDraft() ?? prefs()?.registryUrl ?? "";
  const urlDirty = () => urlDraft() !== null && urlDraft() !== (prefs()?.registryUrl ?? "");

  const installedById = createMemo(
    () => new Map((installed() ?? []).map((p) => [p.id, p])),
  );
  const available = createMemo(() =>
    (registry()?.packs ?? []).filter((p) => !installedById().has(p.id)),
  );
  const updatable = createMemo(() =>
    (registry()?.packs ?? [])
      .map((entry) => {
        const current = installedById().get(entry.id);
        return current && cmpVer(entry.version, current.version) > 0
          ? { entry, current }
          : null;
      })
      .filter((x): x is { entry: packsApi.PackEntry; current: packsApi.InstalledPack } => x !== null),
  );

  async function browse() {
    setBrowsing(true);
    setRegistryError(null);
    try {
      const r = await packsApi.fetchRegistry();
      setRegistry(r);
      void refetchPrefs(); // picks up the new last_checked
    } catch (e) {
      if (packsApi.isNetworkDisabled(e)) {
        setRegistryError(
          "Network calls are off. Turn on “Allow network calls” below, then try again.",
        );
      } else {
        setRegistryError(e instanceof Error ? e.message : String(e));
      }
    } finally {
      setBrowsing(false);
    }
  }

  async function saveUrl() {
    const next = effectiveUrl().trim();
    try {
      await packsApi.setRegistryUrl(next);
      setUrlDraft(null);
      setRegistry(null); // stale against the old URL
      setRegistryError(null);
      void refetchPrefs();
      pushToast("success", "Registry URL saved.");
    } catch (e) {
      pushToast("error", `Couldn't save registry URL: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function toggleNetwork(v: boolean) {
    try {
      await packsApi.setAllowNetwork(v);
      void refetchPrefs();
    } catch (e) {
      pushToast("error", `Couldn't change the network setting: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function handleInstall(entry: packsApi.PackEntry) {
    const key = `install:${entry.id}`;
    setBusy(key);
    try {
      const r = await packsApi.install(entry.id);
      pushToast("success", `Installed ${r.name} v${r.version}.`);
      void refetchInstalled();
      void refetchRollbacks();
      void syncRecipes();
    } catch (e) {
      pushToast("error", `Install failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setBusy(null);
    }
  }

  async function handleUpdate(entry: packsApi.PackEntry) {
    const key = `update:${entry.id}`;
    setBusy(key);
    try {
      const r = await packsApi.update(entry.id);
      pushToast(
        r.updated ? "success" : "info",
        r.updated ? `Updated ${entry.name} ${r.fromVersion} → ${r.toVersion}.` : `${entry.name} is already up to date.`,
      );
      void refetchInstalled();
      void refetchRollbacks();
      void syncRecipes();
    } catch (e) {
      pushToast("error", `Update failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setBusy(null);
    }
  }

  async function handleUninstall(pack: packsApi.InstalledPack) {
    if (
      !(await confirm(
        `Uninstall ${pack.name}?\n\nThe pack is kept for ${14} days so you can roll it back from here.`,
        { title: "Uninstall pack", confirmLabel: "Uninstall", danger: true },
      ))
    ) {
      return;
    }
    const key = `uninstall:${pack.id}`;
    setBusy(key);
    try {
      await packsApi.uninstall(pack.id);
      pushToast("success", `Uninstalled ${pack.name} (kept for rollback).`);
      void refetchInstalled();
      void refetchRollbacks();
      void syncRecipes();
    } catch (e) {
      pushToast("error", `Uninstall failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setBusy(null);
    }
  }

  async function handleRollback(rb: packsApi.RollbackEntry) {
    const key = `rollback:${rb.id}:${rb.version}`;
    setBusy(key);
    try {
      const r = await packsApi.rollback(rb.id, rb.version);
      pushToast("success", `Restored ${r.name} v${r.version}.`);
      void refetchInstalled();
      void refetchRollbacks();
      void syncRecipes();
    } catch (e) {
      pushToast("error", `Rollback failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setBusy(null);
    }
  }

  async function handleDiscard(rb: packsApi.RollbackEntry) {
    if (
      !(await confirm(`Permanently discard ${rb.name} v${rb.version}?`, {
        title: "Discard retained version",
        confirmLabel: "Discard",
        danger: true,
      }))
    ) {
      return;
    }
    const key = `discard:${rb.id}:${rb.version}`;
    setBusy(key);
    try {
      await packsApi.discardRollback(rb.id, rb.version);
      pushToast("success", "Retained version discarded.");
      void refetchRollbacks();
    } catch (e) {
      pushToast("error", `Discard failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setBusy(null);
    }
  }

  return (
    <div class="flex flex-col gap-4">
      <p class="text-xs leading-relaxed text-(--color-oa-ink-dim)">
        Content packs add optional content — editorial articles, emulator
        recipes, themes, and more — on top of your install. OA never contacts a
        server unless you click <span class="text-(--color-oa-ink)">Browse</span> or{" "}
        <span class="text-(--color-oa-ink)">Update</span>. Downloads are verified
        against the registry's sha256 before they install.
      </p>

      {/* Registry + network */}
      <Card
        title="Registry & network"
        subtitle="Where OA looks for packs, and whether it's allowed to."
        right={
          <button
            type="button"
            class={btnClass}
            disabled={browsing()}
            onClick={() => void browse()}
            title="Fetch the pack registry"
          >
            {browsing() ? "Browsing…" : "↻ Browse / Check for updates"}
          </button>
        }
      >
        <div class="flex flex-col gap-3">
          <div class="flex flex-wrap items-center gap-2">
            <input
              type="text"
              value={effectiveUrl()}
              spellcheck={false}
              onInput={(e) => setUrlDraft(e.currentTarget.value)}
              class="min-w-[20rem] flex-1 rounded-md border border-white/10 bg-white/[0.04] px-3 py-2 font-mono text-xs text-(--color-oa-ink) focus-visible:outline focus-visible:outline-2 focus-visible:outline-(--color-oa-ink-dim)"
              placeholder="https://…/registry.json"
            />
            <button type="button" class={btnClass} disabled={!urlDirty()} onClick={() => void saveUrl()}>
              Save
            </button>
            <button
              type="button"
              class={btnClass}
              onClick={() => {
                setUrlDraft("");
                void saveUrl();
              }}
              title="Reset to the built-in default registry"
            >
              Reset
            </button>
          </div>
          <SettingRow
            label="Allow network calls"
            inherited={null}
            overridden={false}
            toggle={{
              checked: prefs()?.allowNetwork ?? true,
              onChange: (v) => void toggleNetwork(v),
            }}
            description="When off, Browse / Install / Update are disabled. The registry and pack downloads are the only servers OA ever contacts, and only when you ask."
          />
          <Show when={prefs()?.lastChecked}>
            <p class="text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink-dim)">
              Last checked: {new Date(prefs()!.lastChecked!).toLocaleString()}
            </p>
          </Show>
          <Show when={registryError()}>
            <p class="rounded-md border border-amber-500/20 bg-amber-500/[0.06] px-3 py-2 text-xs text-amber-200/90">
              {registryError()}
            </p>
          </Show>
        </div>
      </Card>

      {/* Installed */}
      <Card title="Installed" subtitle={`${(installed() ?? []).length} pack(s) installed.`}>
        <Show
          when={(installed() ?? []).length > 0}
          fallback={
            <p class="rounded-md border border-dashed border-white/10 bg-white/[0.01] px-3 py-4 text-center text-xs text-(--color-oa-ink-dim)">
              No packs installed yet. Click <span class="text-(--color-oa-ink)">Browse</span> above to
              see what's available.
            </p>
          }
        >
          <div class="flex flex-col gap-2">
            <For each={installed() ?? []}>
              {(pack) => (
                <div class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-white/5 bg-white/[0.02] px-3 py-2">
                  <div class="flex min-w-0 flex-col gap-1">
                    <div class="flex flex-wrap items-center gap-2">
                      <span class="text-sm font-medium text-(--color-oa-ink)">{pack.name}</span>
                      <TypeChip type={pack.packType} />
                      <span class="font-mono text-xs text-(--color-oa-ink-dim)">v{pack.version}</span>
                    </div>
                    <Show when={pack.license}>
                      <span class="text-[0.65rem] text-(--color-oa-ink-dim)">{pack.license}</span>
                    </Show>
                  </div>
                  <button
                    type="button"
                    class={dangerBtnClass}
                    disabled={busy() === `uninstall:${pack.id}`}
                    onClick={() => void handleUninstall(pack)}
                  >
                    {busy() === `uninstall:${pack.id}` ? "Uninstalling…" : "Uninstall"}
                  </button>
                </div>
              )}
            </For>
          </div>
        </Show>
      </Card>

      {/* Updates */}
      <Show when={updatable().length > 0}>
        <Card title="Updates available" subtitle={`${updatable().length} pack(s) have a newer version.`}>
          <div class="flex flex-col gap-2">
            <For each={updatable()}>
              {({ entry, current }) => (
                <div class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-sky-500/15 bg-sky-500/[0.04] px-3 py-2">
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="text-sm font-medium text-(--color-oa-ink)">{entry.name}</span>
                    <TypeChip type={entry.type} />
                    <span class="font-mono text-xs text-(--color-oa-ink-dim)">
                      v{current.version} → v{entry.version}
                    </span>
                  </div>
                  <button
                    type="button"
                    class={btnClass}
                    disabled={busy() === `update:${entry.id}`}
                    onClick={() => void handleUpdate(entry)}
                  >
                    {busy() === `update:${entry.id}` ? "Updating…" : "Update"}
                  </button>
                </div>
              )}
            </For>
          </div>
        </Card>
      </Show>

      {/* Available (only after a registry fetch) */}
      <Show when={registry()}>
        <Card title="Available" subtitle="Packs in the registry you don't have installed.">
          <Show
            when={available().length > 0}
            fallback={
              <p class="rounded-md border border-dashed border-white/10 bg-white/[0.01] px-3 py-4 text-center text-xs text-(--color-oa-ink-dim)">
                Everything in the registry is already installed.
              </p>
            }
          >
            <div class="flex flex-col gap-2">
              <For each={available()}>
                {(entry) => (
                  <div class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-white/5 bg-white/[0.02] px-3 py-2">
                    <div class="flex min-w-0 flex-col gap-1">
                      <div class="flex flex-wrap items-center gap-2">
                        <span class="text-sm font-medium text-(--color-oa-ink)">{entry.name}</span>
                        <TypeChip type={entry.type} />
                        <span class="font-mono text-xs text-(--color-oa-ink-dim)">v{entry.version}</span>
                      </div>
                      <Show when={entry.summary}>
                        <span class="text-[0.7rem] text-(--color-oa-ink-dim)">{entry.summary}</span>
                      </Show>
                      <Show when={entry.license}>
                        <span class="text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim)">
                          {entry.license}
                        </span>
                      </Show>
                    </div>
                    <button
                      type="button"
                      class={btnClass}
                      disabled={busy() === `install:${entry.id}`}
                      onClick={() => void handleInstall(entry)}
                    >
                      {busy() === `install:${entry.id}` ? "Installing…" : "Install"}
                    </button>
                  </div>
                )}
              </For>
            </div>
          </Show>
        </Card>
      </Show>

      {/* Rollback retention */}
      <Show when={(rollbacks() ?? []).length > 0}>
        <Card
          title="Recoverable versions"
          subtitle="Uninstalled or replaced versions, kept for 14 days."
        >
          <div class="flex flex-col gap-2">
            <For each={rollbacks() ?? []}>
              {(rb) => (
                <div class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-white/5 bg-white/[0.02] px-3 py-2">
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="text-sm font-medium text-(--color-oa-ink)">{rb.name}</span>
                    <TypeChip type={rb.packType} />
                    <span class="font-mono text-xs text-(--color-oa-ink-dim)">v{rb.version}</span>
                    <Show when={rb.archivedAt}>
                      <span class="text-[0.6rem] text-(--color-oa-ink-dim)">
                        archived {new Date(rb.archivedAt!).toLocaleDateString()}
                      </span>
                    </Show>
                  </div>
                  <div class="flex items-center gap-2">
                    <button
                      type="button"
                      class={btnClass}
                      disabled={busy() === `rollback:${rb.id}:${rb.version}`}
                      onClick={() => void handleRollback(rb)}
                    >
                      {busy() === `rollback:${rb.id}:${rb.version}` ? "Restoring…" : "Restore"}
                    </button>
                    <button
                      type="button"
                      class={dangerBtnClass}
                      disabled={busy() === `discard:${rb.id}:${rb.version}`}
                      onClick={() => void handleDiscard(rb)}
                    >
                      Discard
                    </button>
                  </div>
                </div>
              )}
            </For>
          </div>
        </Card>
      </Show>

      {/* Emulator-recipe overrides (Slice 5) */}
      <Show when={hasRecipeInfo()}>
        <Card
          title="Emulator recipe overrides"
          subtitle="Launch recipes supplied by installed emulator-recipes packs. Changes apply as soon as you install or remove a pack — no restart needed."
        >
          <Show when={(recipes()?.conflicts.length ?? 0) > 0}>
            <div class="mb-2 flex flex-col gap-1 rounded-md border border-amber-500/20 bg-amber-500/[0.06] px-3 py-2">
              <For each={recipes()?.conflicts ?? []}>
                {(c) => (
                  <p class="text-xs text-amber-200/90">
                    ⚠ <span class="font-mono">{c.id}</span>: <span class="font-medium">{c.winner}</span>{" "}
                    wins over {c.losers.join(", ")} — uninstall one to resolve.
                  </p>
                )}
              </For>
            </div>
          </Show>
          <div class="flex flex-col gap-2">
            <For each={recipes()?.overrides ?? []}>
              {(o) => (
                <div class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-white/5 bg-white/[0.02] px-3 py-2">
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="font-mono text-sm text-(--color-oa-ink)">{o.id}</span>
                    <span class="text-(--color-oa-ink-dim)">←</span>
                    <span class="text-sm text-(--color-oa-ink)">{o.packId}</span>
                  </div>
                  <span class="rounded border border-white/10 bg-white/[0.05] px-1.5 py-0.5 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim)">
                    {o.replacedBaseline ? "replaces baseline" : "new emulator"}
                  </span>
                </div>
              )}
            </For>
          </div>
        </Card>
      </Show>
    </div>
  );
};

export default PacksSettings;
