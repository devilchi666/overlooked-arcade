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
import { getDataDir } from "@oa/platform/lib/dataDir";
import { launchRom } from "@oa/platform/library/launch";
import { useMedia, type MediaVariant } from "@oa/platform/library/media";
import type { RomEntry } from "@oa/platform/library/types";
import { systemThemes } from "@oa/platform/themes/registry";
import { captureFocusReturn, useFocusGroup } from "../nav/focus";
import { useBackHandler } from "../nav/back";
import { HintRegion } from "../nav/HintBar";
import {
  getGameInfoOverride,
  setGameInfoOverride,
  deleteGameInfoOverride,
  type BugSeverity,
  type GameBug,
  type GameInfoOverride,
} from "@oa/platform/library/gameInfo";
import { useGameInfoBadges } from "@oa/platform/library/gameInfoBadges";

type Props = {
  entry: RomEntry | null;
  onClose: () => void;
  /// Called after a successful launch (with or without a slot). Used by
  /// App.tsx to set gameRunning + collapse the library overlay etc.
  onLaunched?: (entry: RomEntry, slot?: number) => void;
  /// Retroverse-UI Phase A Slice 3 — render mode. Default "modal"
  /// preserves the existing GameInfoModal call site behavior (backdrop +
  /// max-w-5xl box + Close button + HintRegion). "panel" omits the
  /// backdrop, fills its container, drops the Close button and the
  /// modal-mode HintRegion, and switches role from dialog → region —
  /// suitable for Phase B's persistent right-side detail pane in the
  /// Retroverse LIBRARY tab. `onClose` is still called in panel mode
  /// (e.g. caller deselects the focused tile) so the prop stays
  /// required.
  variant?: "modal" | "panel";
};

type SaveSlot = {
  slot: number;
  exists: boolean;
  sizeBytes: number;
  modifiedAtMs?: number;
  thumbnailDataUrl?: string;
};

type TabId = "screenshots" | "titles" | "saves" | "gameInfo";

const BUG_SEVERITIES: BugSeverity[] = ["blocker", "major", "minor", "cosmetic"];

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

  // ---- Game Info Panel v1 — inline editor state (Phase 7) ------------
  //
  // Bound to the per-game SQLite `game_info_overrides` row. Loads on
  // every entry change so opening the modal for a different game
  // resets the form. Save / Reset / Submit Correction handlers below.
  // Field semantics: empty string / undefined / empty array all mean
  // "no override on this field; fall back to file value." A future
  // refinement could surface the file-layer values inline as gray
  // placeholders so the operator can see what they're choosing to
  // override.
  const [editShortSummary, setEditShortSummary] = createSignal("");
  /// One control category per line in the textarea — simplest UX for
  /// v1's free-form strings. Save splits on newline + trims.
  const [editControlsRaw, setEditControlsRaw] = createSignal("");
  const [editBestEmu, setEditBestEmu] = createSignal("");
  const [editBestEmuReason, setEditBestEmuReason] = createSignal("");
  const [editBugs, setEditBugs] = createSignal<GameBug[]>([]);
  const [editAppliedBestEmu, setEditAppliedBestEmu] = createSignal(false);
  const [editAppliedControls, setEditAppliedControls] = createSignal(false);
  /// Editor saving / submitting state — disables the buttons while a
  /// round-trip is in flight so the operator doesn't double-click.
  const [editSaving, setEditSaving] = createSignal(false);
  const [editSubmitToast, setEditSubmitToast] = createSignal<string | null>(null);

  /// Hydrate the editor when the entry changes. Always reads the raw
  /// override (not the merged record) so the form clearly distinguishes
  /// "operator set this" from "use file value."
  createEffect(() => {
    const e = props.entry;
    if (!e) return;
    void (async () => {
      try {
        const ov = await getGameInfoOverride({
          systemId: e.systemId,
          romId: e.id,
        });
        setEditShortSummary(ov.shortSummary ?? "");
        setEditControlsRaw((ov.controlsSupported ?? []).join("\n"));
        setEditBestEmu(ov.bestEmulator ?? "");
        setEditBestEmuReason(ov.bestEmulatorReason ?? "");
        setEditBugs(ov.bugs ?? []);
        setEditAppliedBestEmu(ov.appliedBestEmulator);
        setEditAppliedControls(ov.appliedControls);
      } catch (err) {
        console.warn("[GameInfoModal] getGameInfoOverride failed:", err);
      }
    })();
    setEditSubmitToast(null);
  });

  const gameInfoBadges = useGameInfoBadges();

  /// Build the GameInfoOverride from the current form state. Empty
  /// strings → undefined per the schema (the row gets DELETEd when all
  /// fields collapse to default).
  function formToOverride(): GameInfoOverride {
    const trimmedSummary = editShortSummary().trim();
    const controlsList = editControlsRaw()
      .split("\n")
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
    const trimmedEmu = editBestEmu().trim();
    const trimmedReason = editBestEmuReason().trim();
    const bugs = editBugs().filter((b) => b.description.trim().length > 0);
    return {
      shortSummary: trimmedSummary.length > 0 ? trimmedSummary : undefined,
      controlsSupported: controlsList.length > 0 ? controlsList : undefined,
      bestEmulator: trimmedEmu.length > 0 ? trimmedEmu : undefined,
      bestEmulatorReason: trimmedReason.length > 0 ? trimmedReason : undefined,
      bugs: bugs.length > 0 ? bugs : undefined,
      appliedBestEmulator: editAppliedBestEmu(),
      appliedControls: editAppliedControls(),
    };
  }

  async function handleEditorSave(): Promise<void> {
    const e = props.entry;
    if (!e || editSaving()) return;
    setEditSaving(true);
    try {
      await setGameInfoOverride({
        systemId: e.systemId,
        romId: e.id,
        overrideRecord: formToOverride(),
      });
      // Refresh the badge cache so the tile-badge layer reflects any
      // newly-added bugs / local-edit indicator without a full library
      // reload.
      await gameInfoBadges.refresh();
    } catch (err) {
      console.warn("[GameInfoModal] setGameInfoOverride failed:", err);
    } finally {
      setEditSaving(false);
    }
  }

  async function handleEditorReset(): Promise<void> {
    const e = props.entry;
    if (!e || editSaving()) return;
    setEditSaving(true);
    try {
      await deleteGameInfoOverride({
        systemId: e.systemId,
        romId: e.id,
      });
      // Re-hydrate the form to the default state.
      setEditShortSummary("");
      setEditControlsRaw("");
      setEditBestEmu("");
      setEditBestEmuReason("");
      setEditBugs([]);
      setEditAppliedBestEmu(false);
      setEditAppliedControls(false);
      await gameInfoBadges.refresh();
    } catch (err) {
      console.warn("[GameInfoModal] deleteGameInfoOverride failed:", err);
    } finally {
      setEditSaving(false);
    }
  }

  /// Phase 9 stub — copy the operator's edits as JSON to the clipboard
  /// + show an informational toast. v2 will replace this with a
  /// pre-populated GitHub Issue URL flow against the
  /// overlooked-arcade-game-info data repo. For v1 the surface is
  /// visible so operators know contribution is coming.
  async function handleSubmitCorrection(): Promise<void> {
    const e = props.entry;
    if (!e) return;
    const payload = {
      systemId: e.systemId,
      romId: e.id,
      title: e.title,
      sha1: e.sha1,
      override: formToOverride(),
    };
    try {
      await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
      setEditSubmitToast(
        "Your changes are copied to the clipboard. We're not yet set up to receive submissions automatically — coming soon.",
      );
    } catch (err) {
      console.warn("[GameInfoModal] clipboard write failed:", err);
      setEditSubmitToast(
        "Could not copy to clipboard. See the debug log for details.",
      );
    }
  }

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

  // Controller-nav: modal acts as a single "primary action" surface.
  // Body is read-only metadata + galleries; the buttons that matter are
  // Launch (A), Resume-from-slot (Y, when one exists), Close (B), and
  // L1/R1 cycle the tabs. No focus ring on individual elements — the
  // modal owns the visible focus.
  const TABS: TabId[] = ["screenshots", "titles", "saves", "gameInfo"];
  function cycleTab(delta: -1 | 1): void {
    const cur = TABS.indexOf(activeTab());
    const next = (cur + delta + TABS.length) % TABS.length;
    setActiveTab(TABS[next]);
  }
  const [infoFocusIndex, setInfoFocusIndex] = createSignal(0);
  const infoFocusGroup = useFocusGroup({
    id: "game-info-modal",
    orientation: "vertical",
    itemCount: () => (props.entry ? 1 : 0),
    focusedIndex: infoFocusIndex,
    setFocusedIndex: setInfoFocusIndex,
    onActivate: () => void handleLaunch(),
    onCancel: () => props.onClose(),
    onTertiary: () => {
      const s = resumeSlot();
      if (s !== undefined) void handleLaunchSlot(s);
    },
    onShoulderL: () => cycleTab(-1),
    onShoulderR: () => cycleTab(1),
  });

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

  // Retroverse-UI Phase A Slice 3 — render mode. Reactive (props.variant
  // may swap if a future caller wants to toggle), but in practice each
  // call site picks one and sticks with it.
  const isModal = () => (props.variant ?? "modal") === "modal";

  return (
    <Show when={props.entry}>
      {(entry) => {
        // Modal mode owns the back-stack + steals focus into the
        // "game-info" group. Panel mode (Phase B LIBRARY tab) leaves
        // both to the parent tab so focus stays in the library grid /
        // tab strip while the panel just displays the focused entry.
        if (isModal()) {
          useBackHandler(() => props.onClose());
          const restoreFocus = captureFocusReturn();
          onMount(() => {
            infoFocusGroup.activate();
            setInfoFocusIndex(0);
          });
          onCleanup(restoreFocus);
        }
        const box = (
          <div
            class={
              isModal()
                ? "flex w-full max-w-5xl flex-col overflow-hidden rounded-lg border border-white/10 bg-(--color-oa-bg-deep) shadow-2xl shadow-black/60"
                : "flex h-full w-full flex-col overflow-hidden bg-(--color-oa-bg-deep)"
            }
            style={isModal() ? { height: "min(720px, 85vh)" } : undefined}
            data-system={entry().systemId}
            role={isModal() ? "dialog" : "region"}
            aria-modal={isModal() ? "true" : undefined}
            aria-labelledby="game-info-title"
          >
            {/* Header — title + close (modal only — panel mode lets the
                tab's hint bar handle backout). Region badge / system
                name in subhead. */}
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
              <Show when={isModal()}>
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
              </Show>
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
                    <button
                      type="button"
                      onClick={(e) => { e.currentTarget.blur(); setActiveTab("gameInfo"); }}
                      class={TAB_BUTTON_CLASS}
                      classList={{
                        "border-(--color-system-accent) bg-white/[0.06] text-(--color-oa-ink)": activeTab() === "gameInfo",
                        "border-white/10 bg-white/[0.04] text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)": activeTab() !== "gameInfo",
                      }}
                      title="Operator-editable per-game notes — short summary, controls, recommended core, known bugs"
                    >
                      Game info
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

                    {/* Game Info tab — operator-editable per-game notes.
                        Phase 7 of the Game Info Panel arc. Form binds
                        to the raw SQLite override so empty fields read
                        as "no override; fall back to file value." */}
                    <Show when={activeTab() === "gameInfo"}>
                      <div class="flex flex-col gap-4 text-xs">
                        <p class="text-[0.65rem] uppercase tracking-[0.35em] text-(--color-oa-ink-dim)">
                          Operator notes
                        </p>
                        <p class="text-[0.7rem] leading-relaxed text-(--color-oa-ink-dim)/80">
                          Edits stay local to this install. Use{" "}
                          <span class="text-(--color-oa-ink-dim)">Submit correction</span> to share.
                          Empty fields fall back to the project-curated values
                          (visible in the detail panel).
                        </p>

                        {/* Short summary */}
                        <label class="flex flex-col gap-1">
                          <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                            Short summary
                          </span>
                          <textarea
                            value={editShortSummary()}
                            onInput={(e) => setEditShortSummary(e.currentTarget.value)}
                            rows={3}
                            class="w-full rounded border border-white/10 bg-white/[0.03] px-2 py-1.5 text-xs text-(--color-oa-ink) focus:border-(--color-system-accent)/60 focus:outline-none"
                            placeholder="A short note — your impression, why this version, anything worth surfacing."
                          />
                        </label>

                        {/* Controls supported (multi-line) */}
                        <label class="flex flex-col gap-1">
                          <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                            Controls supported — one per line
                          </span>
                          <textarea
                            value={editControlsRaw()}
                            onInput={(e) => setEditControlsRaw(e.currentTarget.value)}
                            rows={3}
                            class="w-full rounded border border-white/10 bg-white/[0.03] px-2 py-1.5 text-xs text-(--color-oa-ink) focus:border-(--color-system-accent)/60 focus:outline-none"
                            placeholder={"Standard gamepad\nLight gun\nMouse"}
                          />
                        </label>

                        {/* Best emulator — two fields side by side */}
                        <div class="flex flex-col gap-1">
                          <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                            Recommended core
                          </span>
                          <input
                            type="text"
                            value={editBestEmu()}
                            onInput={(e) => setEditBestEmu(e.currentTarget.value)}
                            class="w-full rounded border border-white/10 bg-white/[0.03] px-2 py-1.5 font-mono text-[0.7rem] text-(--color-oa-ink) focus:border-(--color-system-accent)/60 focus:outline-none"
                            placeholder="e.g. beetle_psx_hw_libretro.dll"
                          />
                          <textarea
                            value={editBestEmuReason()}
                            onInput={(e) => setEditBestEmuReason(e.currentTarget.value)}
                            rows={2}
                            class="mt-1 w-full rounded border border-white/10 bg-white/[0.03] px-2 py-1.5 text-xs text-(--color-oa-ink) focus:border-(--color-system-accent)/60 focus:outline-none"
                            placeholder="Short justification — why this core, what it does better."
                          />
                        </div>

                        {/* Bugs */}
                        <div class="flex flex-col gap-2">
                          <div class="flex items-center justify-between">
                            <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                              Known issues
                            </span>
                            <button
                              type="button"
                              onClick={(e) => {
                                e.currentTarget.blur();
                                setEditBugs((bs) => [
                                  ...bs,
                                  { description: "", severity: "minor", workaround: undefined },
                                ]);
                              }}
                              class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                            >
                              + Add
                            </button>
                          </div>
                          <Show
                            when={editBugs().length > 0}
                            fallback={
                              <p class="text-[0.65rem] text-(--color-oa-ink-dim)/60">
                                No operator bug entries — the panel will use the project's curated list.
                              </p>
                            }
                          >
                            <For each={editBugs()}>
                              {(bug, i) => (
                                <div class="flex flex-col gap-1 rounded border border-white/5 bg-white/[0.02] p-2">
                                  <div class="flex items-center gap-2">
                                    <select
                                      value={bug.severity}
                                      onChange={(e) =>
                                        setEditBugs((bs) =>
                                          bs.map((b, idx) =>
                                            idx === i()
                                              ? { ...b, severity: e.currentTarget.value as BugSeverity }
                                              : b,
                                          ),
                                        )
                                      }
                                      class="rounded border border-white/10 bg-white/[0.05] px-1.5 py-0.5 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink)"
                                    >
                                      <For each={BUG_SEVERITIES}>
                                        {(s) => <option value={s}>{s}</option>}
                                      </For>
                                    </select>
                                    <input
                                      type="text"
                                      value={bug.description}
                                      onInput={(e) =>
                                        setEditBugs((bs) =>
                                          bs.map((b, idx) =>
                                            idx === i()
                                              ? { ...b, description: e.currentTarget.value }
                                              : b,
                                          ),
                                        )
                                      }
                                      class="flex-1 rounded border border-white/10 bg-white/[0.03] px-2 py-0.5 text-xs text-(--color-oa-ink) focus:border-(--color-system-accent)/60 focus:outline-none"
                                      placeholder="Bug description"
                                    />
                                    <button
                                      type="button"
                                      onClick={(e) => {
                                        e.currentTarget.blur();
                                        setEditBugs((bs) => bs.filter((_, idx) => idx !== i()));
                                      }}
                                      class="rounded border border-white/10 bg-white/[0.04] px-1.5 py-0.5 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim) hover:bg-red-500/10 hover:text-(--color-oa-ink)"
                                      aria-label="Remove bug entry"
                                    >
                                      ✕
                                    </button>
                                  </div>
                                  <input
                                    type="text"
                                    value={bug.workaround ?? ""}
                                    onInput={(e) => {
                                      const v = e.currentTarget.value;
                                      setEditBugs((bs) =>
                                        bs.map((b, idx) =>
                                          idx === i()
                                            ? { ...b, workaround: v.length > 0 ? v : undefined }
                                            : b,
                                        ),
                                      );
                                    }}
                                    class="rounded border border-white/10 bg-white/[0.03] px-2 py-0.5 text-[0.7rem] text-(--color-oa-ink-dim) focus:border-(--color-system-accent)/60 focus:outline-none"
                                    placeholder="Workaround (optional)"
                                  />
                                </div>
                              )}
                            </For>
                          </Show>
                        </div>

                        {/* Action row — Save / Reset / Submit correction */}
                        <div class="flex flex-wrap items-center justify-end gap-2 border-t border-white/5 pt-3">
                          <button
                            type="button"
                            disabled={editSaving()}
                            onClick={(e) => {
                              e.currentTarget.blur();
                              void handleEditorReset();
                            }}
                            class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:opacity-60"
                            title="Delete the local override row — falls back to project-curated values"
                          >
                            Reset to default
                          </button>
                          <button
                            type="button"
                            disabled={editSaving()}
                            onClick={(e) => {
                              e.currentTarget.blur();
                              void handleSubmitCorrection();
                            }}
                            class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:opacity-60"
                            title="Copy your edits to the clipboard (v1 stub; v2 submits via GitHub Issue)"
                          >
                            Submit correction
                          </button>
                          <button
                            type="button"
                            disabled={editSaving()}
                            onClick={(e) => {
                              e.currentTarget.blur();
                              void handleEditorSave();
                            }}
                            class="rounded-md border border-(--color-system-accent) bg-(--color-system-accent)/15 px-4 py-1.5 text-[0.65rem] font-semibold uppercase tracking-wider text-(--color-oa-ink) transition hover:bg-(--color-system-accent)/25 disabled:opacity-60"
                          >
                            {editSaving() ? "Saving…" : "Save"}
                          </button>
                        </div>

                        <Show when={editSubmitToast()}>
                          <p class="rounded border border-white/10 bg-white/[0.04] px-3 py-2 text-[0.7rem] leading-relaxed text-(--color-oa-ink-dim)">
                            {editSubmitToast()}
                          </p>
                        </Show>
                      </div>
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
        );
        // Modal variant — wrap box in backdrop + HintRegion. Panel variant —
        // return the box bare so the caller's container controls layout.
        return isModal() ? (
          <div
            class="fixed inset-0 z-50 grid place-items-center bg-black/60 backdrop-blur-sm"
            onClick={(e) => {
              if (e.currentTarget === e.target) props.onClose();
            }}
          >
            <HintRegion hints={() => {
              const base = { a: "Launch", b: "Close", l1: "Prev tab", r1: "Next tab" } as Record<string, string>;
              if (resumeSlot() !== undefined) base.y = "Resume";
              return base as never;
            }} />
            {box}
          </div>
        ) : (
          box
        );
      }}
    </Show>
  );
};

export default GameInfoModal;
