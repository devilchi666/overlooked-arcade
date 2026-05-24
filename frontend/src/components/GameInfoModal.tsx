// Game Info modal — the synthesis surface that makes the launcher feel
// premium. Combines hero boxart, region/source metadata, libretro-database
// fields, screenshots/title-screens galleries, and save-slot launching in
// one detail view. Opened from TileContextMenu → "Game info…".
//
// Layout: ~max-w-5xl × min(720px, 85vh). Left ~40% is hero artwork + cover
// hint; right column carries title, metadata grid, description, and the
// tabbed sub-section. Footer pinned to bottom with Launch / Resume / Close.
//
// Asset URLs: variants are addressed via convertFileSrc(appData/<rel>)
// rather than MediaContext.coverUrl, because we want explicit per-variant
// access in the galleries (not just the resolved active variant).

import {
  createEffect,
  createMemo,
  createResource,
  createSignal,
  For,
  onCleanup,
  onMount,
  Show,
  type Component,
  type JSX,
} from "solid-js";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { getDataDir } from "../lib/dataDir";
import { launchRom } from "../library/launch";
import { useMedia, type MediaVariant } from "../library/media";
import type { RomEntry } from "../library/types";
import { systemThemes } from "../themes/registry";

type Props = {
  entry: RomEntry | null;
  onClose: () => void;
  /// Called after a successful launch (with or without a slot). Used by
  /// App.tsx to set gameRunning + collapse the library overlay etc.
  onLaunched?: (entry: RomEntry, slot?: number) => void;
};

type SaveSlot = {
  slot: number;
  exists: boolean;
  sizeBytes: number;
  modifiedAtMs?: number;
  thumbnailDataUrl?: string;
};

type TabId = "screenshots" | "titles" | "saves";

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatModified(ms?: number): string {
  if (!ms) return "";
  return new Date(ms).toLocaleString();
}

const TAB_BUTTON_CLASS =
  "rounded-md border px-3 py-1 text-[0.65rem] uppercase tracking-widest transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)";

const GameInfoModal: Component<Props> = (props) => {
  const media = useMedia();

  // Resolved data dir for asset-protocol URL construction. Same approach
  // as RegionPicker — resolved on mount, reused for every variant.
  // getDataDir() returns the portable settings dir or AppData per the
  // Rust resolver; absolute path either way.
  const [appDataPath, setAppDataPath] = createSignal("");
  onMount(async () => {
    try {
      setAppDataPath(await getDataDir());
    } catch (e) {
      console.warn("GameInfoModal: getDataDir failed:", e);
    }
  });
  function joinAppData(rel: string): string {
    const base = appDataPath();
    if (!base) return "";
    return base.endsWith("/") || base.endsWith("\\") ? `${base}${rel}` : `${base}/${rel}`;
  }
  function variantSrc(v: MediaVariant, prefer: "thumb" | "full" = "full"): string {
    const rel = prefer === "thumb" ? (v.thumbPath ?? v.path) : v.path;
    return convertFileSrc(joinAppData(rel));
  }

  // GameMedia is reactive via MediaContext — region pick / sync updates
  // re-render the modal in place.
  const gm = createMemo(() => {
    const e = props.entry;
    return e ? media.media(e.id) : undefined;
  });
  // Post-2026-05-23 media-taxonomy rename. Prefer the new field names;
  // fall back to v1 for one release in case a stale in-memory snapshot
  // is still around.
  const boxarts = createMemo<MediaVariant[]>(() => gm()?.boxFront ?? gm()?.boxart ?? []);
  const snaps   = createMemo<MediaVariant[]>(() => gm()?.screenshotGameplay ?? gm()?.snap ?? []);
  const titles  = createMemo<MediaVariant[]>(() => gm()?.screenshotTitle ?? gm()?.title ?? []);
  const metadata = createMemo(() => gm()?.metadata);

  // Hero boxart: pin → fallback to first variant (manual or earliest
  // synced; manual sorts to index 0 in ingest_manual_cover).
  const heroBoxart = createMemo<MediaVariant | undefined>(() => {
    const bs = boxarts();
    if (bs.length === 0) return undefined;
    const pinned = gm()?.selected?.boxFrontIndex ?? gm()?.selected?.boxartIndex;
    return pinned !== undefined && pinned < bs.length ? bs[pinned] : bs[0];
  });

  // Save slots — refetched when the entry changes or when the user deletes
  // a slot inside this modal.
  const [slotsRefreshKey, setSlotsRefreshKey] = createSignal(0);
  const [slots] = createResource(
    () => (props.entry ? { path: props.entry.filePath, _: slotsRefreshKey() } : null),
    async (input): Promise<SaveSlot[]> => {
      if (!input) return [];
      try {
        return await invoke<SaveSlot[]>("list_save_slots", { romPath: input.path });
      } catch (e) {
        console.warn("GameInfoModal: list_save_slots failed:", e);
        return [];
      }
    },
  );
  // Highest-modified existing slot — drives the "Resume from slot N" footer button.
  const resumeSlot = createMemo<number | undefined>(() => {
    const list = slots() ?? [];
    let best: SaveSlot | undefined;
    for (const s of list) {
      if (!s.exists) continue;
      if (!best || (s.modifiedAtMs ?? 0) > (best.modifiedAtMs ?? 0)) best = s;
    }
    return best?.slot;
  });

  // Active tab. Default = Screenshots if any are available, else Saves.
  const [activeTab, setActiveTab] = createSignal<TabId>("screenshots");
  // Reset / pick default when entry changes.
  createEffect(() => {
    if (!props.entry) return;
    const initial: TabId = snaps().length > 0 ? "screenshots" : "saves";
    setActiveTab(initial);
  });

  // Description collapse (some libretro-database descriptions are paragraph-
  // length; v1 caps to ~3 lines with a Read more toggle).
  const [descExpanded, setDescExpanded] = createSignal(false);

  function onWindowKey(e: KeyboardEvent) {
    if (e.key === "Escape" && props.entry) {
      e.stopPropagation();
      props.onClose();
    }
  }
  onMount(() => window.addEventListener("keydown", onWindowKey, { capture: true }));
  onCleanup(() => window.removeEventListener("keydown", onWindowKey, { capture: true }));

  async function handleLaunch() {
    const entry = props.entry;
    if (!entry) return;
    const result = await launchRom(entry);
    if (result.kind === "launched") {
      props.onLaunched?.(entry);
      props.onClose();
    } else if (result.kind === "error") {
      console.warn("GameInfoModal: launch failed:", result.message);
    }
  }
  async function handleLaunchSlot(slot: number) {
    const entry = props.entry;
    if (!entry) return;
    const result = await launchRom(entry, slot);
    if (result.kind === "launched") {
      props.onLaunched?.(entry, slot);
      props.onClose();
    } else if (result.kind === "error") {
      console.warn("GameInfoModal: launch (slot) failed:", result.message);
    }
  }
  async function handleDeleteSlot(slot: number) {
    const entry = props.entry;
    if (!entry) return;
    try {
      await invoke("delete_save_slot", { romPath: entry.filePath, slot });
      setSlotsRefreshKey((k) => k + 1);
    } catch (e) {
      console.warn("GameInfoModal: delete_save_slot failed:", e);
    }
  }

  function MetadataRow(label: string, value: JSX.Element): JSX.Element {
    return (
      <div class="contents">
        <dt class="py-1 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          {label}
        </dt>
        <dd class="py-1 text-sm text-(--color-oa-ink)">{value}</dd>
      </div>
    );
  }

  return (
    <Show when={props.entry}>
      {(entry) => (
        <div
          class="fixed inset-0 z-50 grid place-items-center bg-black/60 backdrop-blur-sm"
          onClick={(e) => {
            if (e.currentTarget === e.target) props.onClose();
          }}
        >
          <div
            class="flex w-full max-w-5xl flex-col overflow-hidden rounded-lg border border-white/10 bg-(--color-oa-bg-deep) shadow-2xl shadow-black/60"
            style={{ height: "min(720px, 85vh)" }}
            data-system={entry().systemId}
            role="dialog"
            aria-modal="true"
            aria-labelledby="game-info-title"
          >
            {/* Header — title + close. Region badge / system name in subhead. */}
            <header class="flex items-center justify-between border-b border-white/5 px-6 py-3">
              <div class="min-w-0">
                <p class="text-[0.6rem] uppercase tracking-[0.4em] text-(--color-system-accent)">
                  Game info
                </p>
                <h2
                  id="game-info-title"
                  class="mt-0.5 truncate text-base font-semibold text-(--color-oa-ink)"
                  title={entry().title}
                >
                  {entry().title}
                </h2>
              </div>
              <button
                type="button"
                onClick={(e) => {
                  e.currentTarget.blur();
                  props.onClose();
                }}
                class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
              >
                Close
              </button>
            </header>

            {/* Body — split layout. Left ~40% hero, right metadata + tabs. */}
            <section class="grid min-h-0 flex-1 grid-cols-[minmax(0,2fr)_minmax(0,3fr)] gap-6 overflow-hidden px-6 py-5">
              {/* Left: hero boxart */}
              <div class="flex min-h-0 flex-col gap-3">
                <div class="relative flex flex-1 items-center justify-center overflow-hidden rounded-md border border-white/10 bg-black/60">
                  <Show
                    when={heroBoxart()}
                    fallback={
                      <div
                        class="absolute inset-0"
                        style={{
                          background:
                            "radial-gradient(circle at 30% 25%, var(--color-system-glow), transparent 60%), linear-gradient(135deg, var(--color-system-accent) 0%, var(--color-oa-bg-deep) 100%)",
                        }}
                      />
                    }
                  >
                    {(v) => (
                      <>
                        <img
                          src={variantSrc(v(), "full")}
                          alt={`${entry().title} boxart`}
                          class="max-h-full max-w-full object-contain"
                          loading="eager"
                        />
                        <Show when={v().region}>
                          {(region) => (
                            <span class="absolute right-2 top-2 rounded bg-black/60 px-1.5 py-0.5 text-[0.6rem] font-medium uppercase tracking-widest text-(--color-system-accent-soft) backdrop-blur">
                              {region()}
                            </span>
                          )}
                        </Show>
                      </>
                    )}
                  </Show>
                </div>
                <div class="flex items-center justify-between text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                  <Show
                    when={boxarts().length > 1}
                    fallback={<span>&nbsp;</span>}
                  >
                    <span>{boxarts().length} regional covers</span>
                  </Show>
                  <Show when={heroBoxart()}>
                    {(v) => (
                      <span>{v().source.kind === "manual" ? "manual" : "synced"}</span>
                    )}
                  </Show>
                </div>
              </div>

              {/* Right: title-area, metadata grid, description, tabs */}
              <div class="flex min-h-0 flex-col gap-4 overflow-hidden">
                <div>
                  <p class="text-xs text-(--color-oa-ink-dim)">
                    {systemThemes[entry().systemId].displayName}
                    <Show when={entry().coreOverride}>
                      {" · "}
                      <span class="text-(--color-system-accent-soft)">
                        core override: {entry().coreOverride}
                      </span>
                    </Show>
                  </p>
                </div>

                {/* Metadata grid — only renders rows where the field is set. */}
                <dl class="grid grid-cols-[max-content_minmax(0,1fr)] gap-x-4 gap-y-0">
                  <Show when={metadata()?.year}>
                    {(year) => MetadataRow("Year", <>{year()}</>)}
                  </Show>
                  <Show when={metadata()?.genre}>
                    {(genre) => MetadataRow("Genre", <>{genre()}</>)}
                  </Show>
                  <Show when={metadata()?.developer}>
                    {(dev) => MetadataRow("Developer", <>{dev()}</>)}
                  </Show>
                  <Show when={metadata()?.publisher}>
                    {(pub) => MetadataRow("Publisher", <>{pub()}</>)}
                  </Show>
                  <Show when={metadata()?.players}>
                    {(p) => MetadataRow("Players", <>{p()}</>)}
                  </Show>
                  <Show when={!metadata()}>
                    <div class="col-span-2 py-1 text-xs text-(--color-oa-ink-dim)">
                      No metadata yet — run Settings → Game media → Sync metadata
                      to populate.
                    </div>
                  </Show>
                </dl>

                {/* Description (collapsible) — only when present. */}
                <Show when={metadata()?.description}>
                  {(desc) => (
                    <div>
                      <p
                        class="text-sm leading-relaxed text-(--color-oa-ink)"
                        classList={{ "line-clamp-3": !descExpanded() }}
                      >
                        {desc()}
                      </p>
                      <Show when={desc().length > 200}>
                        <button
                          type="button"
                          onClick={(e) => {
                            e.currentTarget.blur();
                            setDescExpanded((v) => !v);
                          }}
                          class="mt-1 text-[0.6rem] uppercase tracking-widest text-(--color-system-accent-soft) hover:underline"
                        >
                          {descExpanded() ? "Read less" : "Read more"}
                        </button>
                      </Show>
                    </div>
                  )}
                </Show>

                {/* Tabs */}
                <div class="flex min-h-0 flex-1 flex-col gap-2 overflow-hidden">
                  <div class="flex flex-wrap gap-1.5">
                    <button
                      type="button"
                      onClick={(e) => { e.currentTarget.blur(); setActiveTab("screenshots"); }}
                      class={TAB_BUTTON_CLASS}
                      classList={{
                        "border-(--color-system-accent) bg-white/[0.06] text-(--color-oa-ink)": activeTab() === "screenshots",
                        "border-white/10 bg-white/[0.04] text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)": activeTab() !== "screenshots",
                      }}
                    >
                      Screenshots <Show when={snaps().length > 0}>({snaps().length})</Show>
                    </button>
                    <button
                      type="button"
                      onClick={(e) => { e.currentTarget.blur(); setActiveTab("titles"); }}
                      class={TAB_BUTTON_CLASS}
                      classList={{
                        "border-(--color-system-accent) bg-white/[0.06] text-(--color-oa-ink)": activeTab() === "titles",
                        "border-white/10 bg-white/[0.04] text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)": activeTab() !== "titles",
                      }}
                    >
                      Title screens <Show when={titles().length > 0}>({titles().length})</Show>
                    </button>
                    <button
                      type="button"
                      onClick={(e) => { e.currentTarget.blur(); setActiveTab("saves"); }}
                      class={TAB_BUTTON_CLASS}
                      classList={{
                        "border-(--color-system-accent) bg-white/[0.06] text-(--color-oa-ink)": activeTab() === "saves",
                        "border-white/10 bg-white/[0.04] text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)": activeTab() !== "saves",
                      }}
                    >
                      Save states
                    </button>
                  </div>

                  <div class="min-h-0 flex-1 overflow-auto rounded border border-white/5 bg-white/[0.02] p-3">
                    <Show when={activeTab() === "screenshots"}>
                      <Show
                        when={snaps().length > 0}
                        fallback={
                          <p class="text-xs text-(--color-oa-ink-dim)">
                            No screenshots yet — Sync media in Settings to fetch.
                          </p>
                        }
                      >
                        <ul class="grid grid-cols-2 gap-2 sm:grid-cols-3">
                          <For each={snaps()}>
                            {(v) => (
                              <li class="relative overflow-hidden rounded border border-white/5 bg-black/40">
                                <img
                                  src={variantSrc(v, "full")}
                                  alt={`${entry().title} screenshot`}
                                  class="aspect-[4/3] w-full object-contain"
                                  loading="lazy"
                                />
                                <Show when={v.region}>
                                  {(r) => (
                                    <span class="absolute right-1 top-1 rounded bg-black/60 px-1 py-0.5 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim) backdrop-blur">
                                      {r()}
                                    </span>
                                  )}
                                </Show>
                              </li>
                            )}
                          </For>
                        </ul>
                      </Show>
                    </Show>

                    <Show when={activeTab() === "titles"}>
                      <Show
                        when={titles().length > 0}
                        fallback={
                          <p class="text-xs text-(--color-oa-ink-dim)">
                            No title screens yet — Sync media in Settings to fetch.
                          </p>
                        }
                      >
                        <ul class="grid grid-cols-2 gap-2 sm:grid-cols-3">
                          <For each={titles()}>
                            {(v) => (
                              <li class="relative overflow-hidden rounded border border-white/5 bg-black/40">
                                <img
                                  src={variantSrc(v, "full")}
                                  alt={`${entry().title} title screen`}
                                  class="aspect-[4/3] w-full object-contain"
                                  loading="lazy"
                                />
                                <Show when={v.region}>
                                  {(r) => (
                                    <span class="absolute right-1 top-1 rounded bg-black/60 px-1 py-0.5 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim) backdrop-blur">
                                      {r()}
                                    </span>
                                  )}
                                </Show>
                              </li>
                            )}
                          </For>
                        </ul>
                      </Show>
                    </Show>

                    <Show when={activeTab() === "saves"}>
                      <Show
                        when={(slots() ?? []).length > 0}
                        fallback={
                          <p class="text-xs text-(--color-oa-ink-dim)">Loading slots…</p>
                        }
                      >
                        <ul class="grid grid-cols-2 gap-2 sm:grid-cols-3 md:grid-cols-5">
                          <For each={slots() ?? []}>
                            {(s) => (
                              <li
                                class="group flex flex-col overflow-hidden rounded border border-white/5 bg-white/[0.03]"
                                classList={{
                                  "opacity-60": !s.exists,
                                  "hover:border-(--color-system-accent)/60 hover:bg-white/[0.06]": s.exists,
                                }}
                              >
                                <button
                                  type="button"
                                  disabled={!s.exists}
                                  onClick={(e) => {
                                    e.currentTarget.blur();
                                    void handleLaunchSlot(s.slot);
                                  }}
                                  class="relative aspect-[4/3] w-full overflow-hidden bg-black/40 text-left disabled:cursor-not-allowed"
                                  title={s.exists ? `Launch with slot ${s.slot} restored` : "Empty slot"}
                                >
                                  <Show
                                    when={s.thumbnailDataUrl}
                                    fallback={
                                      <div class="grid h-full place-items-center text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                                        {s.exists ? "no thumb" : "empty"}
                                      </div>
                                    }
                                  >
                                    {(url) => (
                                      <img
                                        src={url()}
                                        alt={`Slot ${s.slot}`}
                                        class="absolute inset-0 h-full w-full object-contain"
                                      />
                                    )}
                                  </Show>
                                  <span class="absolute left-1 top-1 rounded bg-black/60 px-1 py-0.5 text-[0.55rem] font-semibold text-(--color-system-accent-soft) backdrop-blur">
                                    {s.slot}
                                  </span>
                                </button>
                                <div class="flex flex-col gap-0.5 px-1.5 py-1 text-[0.55rem] text-(--color-oa-ink-dim)">
                                  <Show when={s.exists} fallback={<span>empty</span>}>
                                    <span>{formatSize(s.sizeBytes)}</span>
                                    <span class="truncate" title={formatModified(s.modifiedAtMs)}>
                                      {formatModified(s.modifiedAtMs)}
                                    </span>
                                    <button
                                      type="button"
                                      onClick={(e) => {
                                        e.stopPropagation();
                                        e.currentTarget.blur();
                                        void handleDeleteSlot(s.slot);
                                      }}
                                      class="mt-0.5 rounded border border-white/10 bg-white/[0.04] px-1.5 py-0.5 text-[0.55rem] uppercase tracking-wider hover:bg-red-500/10 hover:text-(--color-oa-ink)"
                                    >
                                      Delete
                                    </button>
                                  </Show>
                                </div>
                              </li>
                            )}
                          </For>
                        </ul>
                      </Show>
                    </Show>
                  </div>
                </div>
              </div>
            </section>

            {/* Footer — Resume (when a slot exists) + Launch + Close. */}
            <footer class="flex items-center justify-end gap-2 border-t border-white/5 px-6 py-3">
              <Show when={resumeSlot() !== undefined}>
                <button
                  type="button"
                  onClick={(e) => {
                    e.currentTarget.blur();
                    const s = resumeSlot();
                    if (s !== undefined) void handleLaunchSlot(s);
                  }}
                  class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                >
                  Resume from slot {resumeSlot()}
                </button>
              </Show>
              <button
                type="button"
                onClick={(e) => {
                  e.currentTarget.blur();
                  void handleLaunch();
                }}
                class="rounded-md border border-(--color-system-accent) bg-(--color-system-accent)/15 px-4 py-1.5 text-xs font-semibold uppercase tracking-wider text-(--color-oa-ink) transition hover:bg-(--color-system-accent)/25 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
              >
                ▶ Launch
              </button>
            </footer>
          </div>
        </div>
      )}
    </Show>
  );
};

export default GameInfoModal;
