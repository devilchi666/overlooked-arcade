import { createMemo, createResource, createSignal, For, onCleanup, onMount, Show, type Component } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open as pickFile } from "@tauri-apps/plugin-dialog";
import type { CoreEntry } from "../settings/store";
import { systemThemes, type SystemId } from "../themes/registry";
import {
  CatalogCoreCard,
  humanBytes,
  type AvailableCore,
  type DownloadProgress,
} from "./CatalogCoreCard";

/// Display labels for system slugs that don't have OA registry entries
/// yet. The frontend uses these to render section headers under
/// "Not yet wired in OA" so the user sees a real system name instead of
/// a raw slug. Add to this map any new slug you mention in the Rust
/// catalog's `systems` field that isn't in `systemThemes` yet.
const QUEUED_SYSTEM_LABELS: Record<string, string> = {
  atari2600: "Atari 2600",
  atari5200: "Atari 5200",
  atari7800: "Atari 7800",
  jaguar: "Atari Jaguar",
  sms: "Sega Master System",
  gamegear: "Sega Game Gear",
  genesis: "Sega Genesis / Mega Drive",
  segacd: "Sega CD",
  sega32x: "Sega 32X",
  saturn: "Sega Saturn",
  dreamcast: "Sega Dreamcast",
  gameboy: "Game Boy / Color",
  gba: "Game Boy Advance",
  n64: "Nintendo 64",
  nds: "Nintendo DS",
  "3ds": "Nintendo 3DS",
  gamecube: "Nintendo GameCube",
  wii: "Nintendo Wii",
  virtualboy: "Virtual Boy",
  pokemini: "Pokémon Mini",
  wonderswan: "WonderSwan / Color",
  ngp: "Neo Geo Pocket / Color",
  neogeocd: "Neo Geo CD",
  pcfx: "PC-FX",
  psx: "Sony PlayStation",
  ps2: "Sony PlayStation 2",
  psp: "Sony PSP",
  msx: "MSX",
  msx2: "MSX2",
  coleco: "ColecoVision",
  vectrex: "Vectrex",
  odyssey2: "Magnavox Odyssey²",
  intellivision: "Mattel Intellivision",
  channelf: "Fairchild Channel F",
  fbneo: "Arcade — FinalBurn Neo",
  dos: "MS-DOS",
  scummvm: "ScummVM",
};

type CatalogGroup = {
  /// Stable key for accordion expanded-state.
  id: string;
  /// Section header label.
  label: string;
  /// True when the system has an OA registry entry (themed in-app).
  wired: boolean;
  /// `data-system` for the section header — drives accent color.
  accentSystem?: string;
  entries: AvailableCore[];
};

type Props = {
  /// Back button target — same shape as LibraryManagerPage.
  onBack: () => void;
};

function humanModified(unixMs: number): string {
  if (unixMs <= 0) return "—";
  const d = new Date(unixMs);
  if (Number.isNaN(d.getTime())) return "—";
  return d.toISOString().slice(0, 10);
}

/// Dedicated OA-wide Cores view. Lists every libretro core in
/// `<exe_dir>/cores/` with the metadata `retro_get_system_info` reports
/// (library name + version + supported extensions), plus disk metadata
/// and per-row actions: pick a system to assign as default, remove the
/// .dll after confirm, and an "Add core…" button that picks a .dll via
/// dialog and copies it into the cores folder.
///
/// Feature 2 (buildbot catalog + install + update) layers on top of this
/// surface in a follow-up commit.
const CoresPage: Component<Props> = (props) => {
  const [cores, { refetch }] = createResource(async (): Promise<CoreEntry[]> => {
    try {
      return await invoke<CoreEntry[]>("list_cores");
    } catch (e) {
      console.warn("list_cores failed:", e);
      return [];
    }
  });
  // Per-system core prefs — we need to know which system, if any, currently
  // points to a given .dll so the row's chip can say "default for tg16".
  const systemIds = Object.keys(systemThemes) as SystemId[];
  const [prefsTick, setPrefsTick] = createSignal(0);
  const [prefs] = createResource(prefsTick, async (): Promise<Record<string, string | null>> => {
    const result: Record<string, string | null> = {};
    for (const id of systemIds) {
      try {
        const v = await invoke<string | null>("get_core_pref", { systemId: id });
        result[id] = v ?? null;
      } catch (e) {
        console.warn(`get_core_pref(${id}) failed:`, e);
        result[id] = null;
      }
    }
    return result;
  });

  const [busy, setBusy] = createSignal<string | null>(null);
  const [status, setStatus] = createSignal<string>("");

  // Feature 2 — catalog of buildbot-installable cores. Re-fetched after every
  // install/remove so the "Installed (v…)" chip stays in sync with the
  // installed list.
  const [catalogTick, setCatalogTick] = createSignal(0);
  const [catalog] = createResource(catalogTick, async (): Promise<AvailableCore[]> => {
    try {
      return await invoke<AvailableCore[]>("available_cores");
    } catch (e) {
      console.warn("available_cores failed:", e);
      return [];
    }
  });
  const refreshCatalog = () => setCatalogTick((n) => n + 1);

  // Per-base progress map. Phase + downloaded/total bytes feed the row's
  // little progress strip while a download is in flight.
  const [progress, setProgress] = createSignal<Record<string, DownloadProgress>>({});
  onMount(() => {
    let unlisten: (() => void) | undefined;
    void listen<DownloadProgress>("oa://core-download-progress", (e) => {
      setProgress((m) => ({ ...m, [e.payload.fileName]: e.payload }));
      if (e.payload.phase === "done" || e.payload.phase === "error") {
        // Refresh the installed-cores list once the .dll lands so the
        // row flips to "Installed (v…)".
        refetch();
        refreshCatalog();
      }
    }).then((un) => (unlisten = un));
    onCleanup(() => unlisten?.());
  });

  // Accordion expanded-state. Wired-in-OA sections + Multi-system start
  // expanded; "Not yet wired" subsections also start expanded per design
  // intent (give the user a punch list). User can collapse anything.
  const [expandedGroups, setExpandedGroups] = createSignal<Set<string>>(new Set());
  function toggleGroup(id: string) {
    setExpandedGroups((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  /// Group the flat catalog into the three section classes used in the
  /// UI: per-wired-system, multi-system, per-queued-system. Single-system
  /// cores land under their system; cores listing 2+ systems land under
  /// Multi-system; cores whose system slugs don't match the registry land
  /// under their queued-system section (e.g. "Sega Saturn" once we add a
  /// Saturn slug to `systemThemes` the section migrates up automatically).
  const catalogGroups = createMemo((): CatalogGroup[] => {
    const all = catalog() ?? [];
    if (all.length === 0) return [];

    const wiredSlugs = new Set(Object.keys(systemThemes));

    // Bucket: wired-system groups (keyed by slug, ordered by registry).
    // Multi-system bucket (one section).
    // Queued-system groups (keyed by slug, ordered by first appearance in catalog).
    const wiredBuckets = new Map<string, AvailableCore[]>();
    const multi: AvailableCore[] = [];
    const queuedBuckets = new Map<string, AvailableCore[]>();

    for (const entry of all) {
      const wiredHits = entry.systems.filter((s) => wiredSlugs.has(s));
      const queuedHits = entry.systems.filter((s) => !wiredSlugs.has(s));

      // Multi-system if it covers more than one slug, period. This is
      // independent of "wired" — Genesis Plus GX is multi-system even
      // though some of its slugs (sms / gamegear / genesis) are queued.
      if (entry.systems.length > 1) {
        multi.push(entry);
        continue;
      }

      if (wiredHits.length === 1) {
        const slug = wiredHits[0];
        if (!wiredBuckets.has(slug)) wiredBuckets.set(slug, []);
        wiredBuckets.get(slug)!.push(entry);
        continue;
      }

      if (queuedHits.length === 1) {
        const slug = queuedHits[0];
        if (!queuedBuckets.has(slug)) queuedBuckets.set(slug, []);
        queuedBuckets.get(slug)!.push(entry);
        continue;
      }

      // Shouldn't happen — catalog policy says `systems` is never empty.
      // Drop into a "misc" queued bucket so the entry still renders.
      if (!queuedBuckets.has("misc")) queuedBuckets.set("misc", []);
      queuedBuckets.get("misc")!.push(entry);
    }

    const groups: CatalogGroup[] = [];

    // Wired sections in sidebar order — same iteration as Object.keys
    // over the registry preserves declaration order (tg16, pce-cd, lynx,
    // nes, snes today).
    for (const slug of Object.keys(systemThemes)) {
      const entries = wiredBuckets.get(slug);
      if (!entries || entries.length === 0) continue;
      const theme = systemThemes[slug as SystemId];
      groups.push({
        id: `sys-${slug}`,
        label: theme?.displayName ?? slug,
        wired: true,
        accentSystem: slug,
        entries,
      });
    }

    // Multi-system section.
    if (multi.length > 0) {
      groups.push({
        id: "multi-system",
        label: "Multi-system",
        wired: true,
        entries: multi,
      });
    }

    // Queued sections — sort alphabetically by display label so the
    // punch list is scannable.
    const queuedKeys = Array.from(queuedBuckets.keys()).sort((a, b) => {
      const la = QUEUED_SYSTEM_LABELS[a] ?? a;
      const lb = QUEUED_SYSTEM_LABELS[b] ?? b;
      return la.localeCompare(lb);
    });
    for (const slug of queuedKeys) {
      groups.push({
        id: `queued-${slug}`,
        label: QUEUED_SYSTEM_LABELS[slug] ?? slug,
        wired: false,
        entries: queuedBuckets.get(slug)!,
      });
    }

    return groups;
  });

  // Expand every group by default the first time the catalog hydrates.
  // Subsequent group additions (e.g. after wiring a new system) also
  // start expanded if the user hasn't touched the accordion since launch.
  createMemo(() => {
    const groups = catalogGroups();
    if (groups.length === 0) return;
    setExpandedGroups((prev) => {
      if (prev.size > 0) return prev;
      return new Set(groups.map((g) => g.id));
    });
  });

  async function handleInstall(c: AvailableCore) {
    if (!c.supportedOnHost) {
      setStatus(`No buildbot build for this OS/ARCH (${navigator.platform}).`);
      return;
    }
    setBusy(`install-${c.base}`);
    setStatus(`Downloading ${c.displayName}…`);
    try {
      await invoke<string>("download_core", { base: c.base });
      setStatus(`Installed ${c.displayName}.`);
      refetch();
      refreshCatalog();
    } catch (e) {
      setStatus(`Install failed: ${String(e)}`);
    } finally {
      setBusy(null);
    }
  }

  const systemsUsingCore = (fileName: string): SystemId[] => {
    const p = prefs() ?? {};
    return systemIds.filter((id) => p[id] === fileName);
  };

  async function handleAddCore() {
    const picked = await pickFile({
      multiple: false,
      filters: [
        {
          name: "libretro core",
          extensions: ["dll", "so", "dylib"],
        },
      ],
    }).catch((e) => {
      console.warn("pickFile failed:", e);
      return null;
    });
    if (!picked || Array.isArray(picked)) return;
    setBusy("install");
    setStatus(`Validating ${picked}…`);
    try {
      await invoke<string>("install_core_from_path", { path: picked });
      setStatus(`Added ${picked.split(/[/\\]/).pop()} to cores folder.`);
      refetch();
    } catch (e) {
      setStatus(`Add failed: ${String(e)}`);
    } finally {
      setBusy(null);
    }
  }

  async function handleRemove(c: CoreEntry) {
    const label = c.libraryName || c.fileName;
    if (!window.confirm(`Remove ${label} from the cores folder?\n\nThe .dll file will be deleted from <exe_dir>/cores/. Any system or game pointing at this core will fall back to auto-detect on next launch.`)) {
      return;
    }
    setBusy(c.fileName);
    setStatus(`Removing ${label}…`);
    try {
      await invoke("remove_installed_core", { fileName: c.fileName });
      // Clear any per-system prefs that still point at this file so the
      // UI doesn't show a stale "default for X" chip after refetch.
      for (const id of systemsUsingCore(c.fileName)) {
        try {
          await invoke("set_core_pref", { systemId: id, fileName: null });
        } catch (e) {
          console.warn(`clearing core_pref(${id}) after remove failed:`, e);
        }
      }
      setStatus(`Removed ${label}.`);
      setPrefsTick((n) => n + 1);
      refetch();
    } catch (e) {
      setStatus(`Remove failed: ${String(e)}`);
    } finally {
      setBusy(null);
    }
  }

  async function handleSetDefault(systemId: string, fileName: string | null) {
    setBusy(`pref-${systemId}`);
    try {
      await invoke("set_core_pref", { systemId, fileName });
      setPrefsTick((n) => n + 1);
    } catch (e) {
      setStatus(`Set default failed: ${String(e)}`);
    } finally {
      setBusy(null);
    }
  }

  const empty = createMemo(() => (cores() ?? []).length === 0);

  return (
    <div
      class="flex h-full w-full flex-col bg-(--color-oa-bg)"
      role="region"
      aria-labelledby="cores-title"
    >
      <header class="flex items-center justify-between border-b border-white/5 bg-(--color-oa-bg-deep)/60 px-6 py-4">
        <div class="flex items-center gap-3">
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              props.onBack();
            }}
            class="rounded-md border border-white/10 bg-white/[0.04] px-2.5 py-1 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
          >
            ← Back
          </button>
          <div>
            <h2
              id="cores-title"
              class="text-sm font-semibold uppercase tracking-[0.3em] text-(--color-oa-ink)"
            >
              Cores
            </h2>
            <p class="mt-0.5 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Libretro cores installed in &lt;exe_dir&gt;/cores/
            </p>
          </div>
        </div>
        <div class="flex items-center gap-2">
          <button
            type="button"
            onClick={() => refetch()}
            class="rounded-md border border-white/10 bg-white/[0.04] px-2.5 py-1 text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
            title="Re-scan the cores folder"
          >
            ↻ Refresh
          </button>
          <button
            type="button"
            onClick={handleAddCore}
            disabled={busy() === "install"}
            class="rounded-md border border-(--color-system-accent)/30 bg-(--color-system-accent)/15 px-3 py-1 text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink) transition hover:bg-(--color-system-accent)/25 disabled:opacity-50"
          >
            {busy() === "install" ? "Adding…" : "Add core…"}
          </button>
        </div>
      </header>

      <Show when={status()}>
        <div class="border-b border-white/5 bg-black/20 px-6 py-2 text-[0.7rem] text-(--color-oa-ink-dim)">
          {status()}
        </div>
      </Show>

      <section class="min-h-0 flex-1 overflow-y-auto px-6 py-6 space-y-8">
        <Show when={!empty()} fallback={
          <div class="rounded-lg border border-dashed border-white/10 bg-black/10 px-6 py-12 text-center">
            <p class="text-sm text-(--color-oa-ink-dim)">
              No libretro cores found in <code class="text-(--color-oa-ink)">&lt;exe_dir&gt;/cores/</code>.
            </p>
            <p class="mt-2 text-[0.7rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Use "Add core…" to install one, or browse the buildbot catalog below.
            </p>
          </div>
        }>
          <div class="flex flex-col gap-3" data-installed-list>
            <h3 class="text-[0.7rem] uppercase tracking-[0.3em] text-(--color-oa-ink-dim)">
              Installed
            </h3>
            <For each={cores() ?? []}>
              {(c) => (
                <article
                  class="rounded-lg border border-white/10 bg-black/20 p-4"
                  classList={{
                    "border-red-500/40 bg-red-950/10": Boolean(c.error),
                  }}
                >
                  <header class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                      <h3 class="truncate text-sm font-semibold text-(--color-oa-ink)">
                        {c.libraryName || c.fileName}
                      </h3>
                      <p class="mt-0.5 truncate text-[0.7rem] text-(--color-oa-ink-dim)">
                        <code>{c.fileName}</code>
                        <Show when={c.libraryVersion}>
                          <span> · v{c.libraryVersion}</span>
                        </Show>
                        <span> · {humanBytes(c.sizeBytes)}</span>
                        <span> · {humanModified(c.modifiedUnixMs)}</span>
                      </p>
                    </div>
                    <div class="flex shrink-0 items-center gap-2">
                      {/* Per-row Update — only renders when the file maps
                          back to a catalog entry that supports this host. */}
                      {(() => {
                        const cat = (catalog() ?? []).find((x) => x.fileName === c.fileName);
                        if (!cat || !cat.supportedOnHost) return null;
                        const busyKey = `install-${cat.base}`;
                        return (
                          <button
                            type="button"
                            onClick={() => void handleInstall(cat)}
                            disabled={busy() === busyKey}
                            class="rounded-md border border-white/10 bg-white/[0.03] px-2.5 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:opacity-50"
                            title={`Re-fetch from ${cat.buildbotUrl ?? "buildbot"}`}
                          >
                            {busy() === busyKey ? "Updating…" : "Update"}
                          </button>
                        );
                      })()}
                      <button
                        type="button"
                        onClick={() => handleRemove(c)}
                        disabled={busy() === c.fileName}
                        class="rounded-md border border-white/10 bg-white/[0.03] px-2.5 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:border-red-500/40 hover:bg-red-950/30 hover:text-red-300 disabled:opacity-50"
                      >
                        {busy() === c.fileName ? "Removing…" : "Remove"}
                      </button>
                    </div>
                  </header>

                  <Show when={c.error}>
                    <p class="mt-3 rounded bg-red-950/30 px-3 py-2 text-[0.7rem] text-red-300">
                      {c.error}
                    </p>
                  </Show>

                  <Show when={!c.error && c.validExtensions}>
                    <div class="mt-3 flex flex-wrap gap-1.5">
                      <For each={c.validExtensions.split("|").filter(Boolean)}>
                        {(ext) => (
                          <span class="rounded bg-white/[0.06] px-1.5 py-0.5 text-[0.6rem] font-mono uppercase tracking-wider text-(--color-oa-ink-dim)">
                            .{ext}
                          </span>
                        )}
                      </For>
                      <Show when={c.needFullpath}>
                        <span class="rounded bg-amber-950/40 px-1.5 py-0.5 text-[0.6rem] uppercase tracking-wider text-amber-300" title="Core requires a filesystem path — archived ROMs are extracted before launch.">
                          path-only
                        </span>
                      </Show>
                    </div>
                  </Show>

                  <Show when={!c.error}>
                    <div class="mt-3 flex items-center gap-3">
                      <label class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                        Set as default for
                      </label>
                      <select
                        class="rounded border border-white/10 bg-black/40 px-2 py-1 text-xs text-(--color-oa-ink)"
                        value=""
                        onChange={(e) => {
                          const v = e.currentTarget.value;
                          e.currentTarget.value = "";
                          if (!v) return;
                          void handleSetDefault(v, c.fileName);
                        }}
                      >
                        <option value="">— pick a system —</option>
                        <For each={systemIds}>
                          {(id) => (
                            <option value={id}>
                              {systemThemes[id].shortName} ({systemThemes[id].displayName})
                            </option>
                          )}
                        </For>
                      </select>
                      <Show when={systemsUsingCore(c.fileName).length > 0}>
                        <div class="flex flex-wrap gap-1.5">
                          <For each={systemsUsingCore(c.fileName)}>
                            {(id) => (
                              <span
                                class="rounded bg-(--color-system-accent)/20 px-2 py-0.5 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink)"
                                data-system={id}
                              >
                                default · {systemThemes[id].shortName}
                                <button
                                  type="button"
                                  class="ml-1.5 text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)"
                                  title={`Clear ${systemThemes[id].shortName} default`}
                                  onClick={() => void handleSetDefault(id, null)}
                                >
                                  ✕
                                </button>
                              </span>
                            )}
                          </For>
                        </div>
                      </Show>
                    </div>
                  </Show>
                </article>
              )}
            </For>
          </div>
        </Show>

        {/* --- Browse cores (buildbot catalog) -------------------------- */}
        <div class="flex flex-col gap-3">
          <div class="flex items-baseline justify-between">
            <h3 class="text-[0.7rem] uppercase tracking-[0.3em] text-(--color-oa-ink-dim)">
              Browse cores
            </h3>
            <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              buildbot.libretro.com · nightly
            </span>
          </div>
          <Show when={(catalog() ?? []).some((c) => !c.supportedOnHost)}>
            <p class="rounded-md border border-amber-500/30 bg-amber-950/20 px-3 py-2 text-[0.7rem] text-amber-200">
              No buildbot build for this OS/architecture — Install is disabled. You can still
              drop a .dll/.so/.dylib in <code>&lt;exe_dir&gt;/cores/</code> manually.
            </p>
          </Show>
          <For each={catalogGroups()}>
            {(group) => {
              const isExpanded = () => expandedGroups().has(group.id);
              return (
                <section
                  class="rounded-md border border-white/10 bg-black/15"
                  data-system={group.accentSystem}
                >
                  <button
                    type="button"
                    onClick={() => toggleGroup(group.id)}
                    class="flex w-full items-center justify-between gap-3 px-3 py-2 text-left transition hover:bg-white/[0.03]"
                  >
                    <div class="flex items-baseline gap-3">
                      <span
                        class="text-[0.6rem] uppercase tracking-widest"
                        classList={{
                          "text-(--color-system-accent)": isExpanded(),
                          "text-(--color-oa-ink-dim)": !isExpanded(),
                        }}
                      >
                        {isExpanded() ? "▼" : "▶"}
                      </span>
                      <h4 class="text-sm font-semibold text-(--color-oa-ink)">{group.label}</h4>
                      <Show when={!group.wired}>
                        <span class="rounded border border-amber-500/30 bg-amber-500/10 px-1.5 py-0.5 text-[0.55rem] uppercase tracking-widest text-amber-300">
                          not yet wired
                        </span>
                      </Show>
                    </div>
                    <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                      {group.entries.filter((c) => c.installed).length} installed ·
                      {" "}{group.entries.length} {group.entries.length === 1 ? "core" : "cores"}
                    </span>
                  </button>
                  <Show when={isExpanded()}>
                    <div class="grid grid-cols-1 gap-3 border-t border-white/5 p-3 lg:grid-cols-2">
                      <For each={group.entries}>
                        {(c) => (
                          <CatalogCoreCard
                            core={c}
                            progress={progress()[c.fileName]}
                            busyKey={busy()}
                            onInstall={() => void handleInstall(c)}
                          />
                        )}
                      </For>
                    </div>
                  </Show>
                </section>
              );
            }}
          </For>
        </div>
      </section>
    </div>
  );
};


export default CoresPage;
