// Solid context for the game-media catalog. Hydrates once from Rust via
// `get_media_index`, refreshes single entries when Rust emits `oa://media-updated`,
// and resolves `oa-media://` URLs for the LibraryTile to use as <img src>.
//
// Cover bytes live on disk; the URL is just a routing token. The protocol
// handler (apps/oa-shell/src/media.rs::handle_uri_request) returns the
// active variant's thumb (or full image) with Cache-Control: immutable.
//
// Implementation note: we use createSignal<Map<...>> rather than
// createStore<Record<...>>. createStore's proxy reactivity has documented
// edge cases with dynamically-added record keys (consumer reads a key that
// doesn't exist yet → setStore(key, value) doesn't always propagate). At our
// scale (≤ a few thousand entries) the trade-off of "all consumers re-run
// on every update" is negligible and the reliability is worth it. If we
// scale past ~5000 entries, virtualize the grid first; the signal pattern
// still works because off-screen tiles aren't subscribers.

import {
  createContext,
  createSignal,
  onCleanup,
  onMount,
  useContext,
  type Accessor,
  type Component,
  type JSX,
} from "solid-js";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getDataDir } from "../lib/dataDir";
import type { SystemId } from "../themes/registry";

export type MediaSourceKind = "manual" | "libretroThumbnails";
export type MediaSource = { kind: MediaSourceKind };
export type Region = string; // "USA" | "Japan" | "Europe" | "World" | Other(string)

export type MediaVariant = {
  source: MediaSource;
  region?: Region;
  path: string;
  thumbPath?: string;
  width?: number;
  height?: number;
  sha1?: string;
  bytes?: number;
};

// Per-slot pinned variant index. Mirrors Rust `SelectedMedia`. v1
// shipped just three indexes (boxartIndex / snapIndex / titleIndex);
// the renamed fields below preserve those keys via the Rust-side serde
// aliases. We keep the legacy field names as optional so frontend code
// can still read old in-memory snapshots without crashing if a stale
// MediaIndex slips through during the 2026-05-23 transition.
export type SelectedMedia = {
  // v1 fields (renamed). Rust serializes via the new names — these
  // optional aliases let dormant frontend code keep compiling.
  boxFrontIndex?: number;
  screenshotGameplayIndex?: number;
  screenshotTitleIndex?: number;
  cartFrontIndex?: number;
  discIndex?: number;
  // New slots.
  boxBackIndex?: number;
  box3dIndex?: number;
  boxSpineIndex?: number;
  boxFullIndex?: number;
  cartBackIndex?: number;
  cart3dIndex?: number;
  screenshotSelectIndex?: number;
  bannerIndex?: number;
  clearLogoIndex?: number;
  fanartBackgroundIndex?: number;
  fanartDiscIndex?: number;
  advertFrontIndex?: number;
  advertBackIndex?: number;
  arcadeCabinetIndex?: number;
  arcadeMarqueeIndex?: number;
  arcadeControlpanelIndex?: number;
  arcadeControlsinfoIndex?: number;
  arcadePlayerselectIndex?: number;
  arcadeFlyerIndex?: number;
  videoIndex?: number;
  musicIndex?: number;
  manualIndex?: number;
  // v1 legacy keys — kept for one release as defensive read-side
  // fallbacks. Rust's serde aliases handle the inbound direction; this
  // covers any cached/in-memory v1 snapshot the frontend might still
  // be holding when the upgraded shell first runs.
  boxartIndex?: number;
  snapIndex?: number;
  titleIndex?: number;
};

export type GameMetadata = {
  year?: number;
  genre?: string;
  developer?: string;
  publisher?: string;
  players?: number;
  description?: string;
};

// Mirrors Rust `GameMedia`. The 2026-05-23 media-taxonomy pivot
// expanded this from 5 slots to the full ~26-slot LaunchBox shape.
// Field names are camelCase per Rust's serde rename_all = "camelCase".
// v1 legacy keys (`boxart`/`snap`/`title`/`cart`) remain readable via
// Rust's serde aliases, but Rust serializes only the new names —
// frontend reads should prefer `boxFront` etc. and treat the v1
// aliases as defensive read-only fallbacks.
export type GameMedia = {
  // v1 fields, renamed.
  boxFront?: MediaVariant[];
  screenshotGameplay?: MediaVariant[];
  screenshotTitle?: MediaVariant[];
  cartFront?: MediaVariant[];
  disc?: MediaVariant[];
  // New slots (Phase 1 plumbing; UI rendering catches up incrementally).
  boxBack?: MediaVariant[];
  box3d?: MediaVariant[];
  boxSpine?: MediaVariant[];
  boxFull?: MediaVariant[];
  cartBack?: MediaVariant[];
  cart3d?: MediaVariant[];
  screenshotSelect?: MediaVariant[];
  banner?: MediaVariant[];
  clearLogo?: MediaVariant[];
  fanartBackground?: MediaVariant[];
  fanartDisc?: MediaVariant[];
  advertFront?: MediaVariant[];
  advertBack?: MediaVariant[];
  arcadeCabinet?: MediaVariant[];
  arcadeMarquee?: MediaVariant[];
  arcadeControlpanel?: MediaVariant[];
  arcadeControlsinfo?: MediaVariant[];
  arcadePlayerselect?: MediaVariant[];
  arcadeFlyer?: MediaVariant[];
  video?: MediaVariant[];
  music?: MediaVariant[];
  manual?: MediaVariant[];
  // v1 legacy keys — defensive read-side fallbacks for one release.
  boxart?: MediaVariant[];
  snap?: MediaVariant[];
  title?: MediaVariant[];
  cart?: MediaVariant[];
  selected?: SelectedMedia;
  metadata?: GameMetadata;
};

// Kebab-case names — matches Rust `MediaKind::as_str()` and folder
// names exactly. The Rust-side `MediaKind::parse` still accepts the
// v1 strings ("boxart"/"snap"/"title"/"cart") for one release, so
// any callers that haven't migrated keep working.
export type MediaKind =
  | "box-front" | "box-back" | "box-3d" | "box-spine" | "box-full"
  | "cart-front" | "cart-back" | "cart-3d"
  | "disc"
  | "screenshot-gameplay" | "screenshot-title" | "screenshot-select"
  | "banner" | "clear-logo"
  | "fanart-background" | "fanart-disc"
  | "advert-front" | "advert-back"
  | "arcade-cabinet" | "arcade-marquee" | "arcade-controlpanel"
  | "arcade-controlsinfo" | "arcade-playerselect" | "arcade-flyer"
  | "video" | "music" | "manual";
export type MediaSize = "thumb" | "full";

// Rust's MediaDb serializes as BTreeMap<String, GameMedia> → JS object.
export type MediaIndex = Record<string, GameMedia>;

type MediaUpdatedEvent = {
  romId: string;
  // Rust may include the new entry inline to avoid an extra round-trip;
  // when omitted, we re-fetch the entry via get_media_index.
  media?: GameMedia;
};

export type MediaStore = {
  /// Lookup. Returns undefined when no media is known for the rom_id.
  media(romId: string): GameMedia | undefined;
  /// Asset-protocol URL for an image of the requested kind ("boxart" by
  /// default, also "snap" / "title" for Game Info screenshots / title
  /// screens). Returns null when no variant exists — caller falls back to
  /// the gradient placeholder.
  coverUrl(systemId: SystemId, romId: string, kind?: MediaKind, size?: MediaSize): string | null;
  /// True once the initial hydrate from Rust has completed. The grid can
  /// avoid showing shimmer for tiles before this flips.
  hydrated: Accessor<boolean>;
  /// Region preference (ordered, first match wins). Hydrated lazily.
  regionPriority: Accessor<string[]>;
  setRegionPriority(regions: string[]): Promise<void>;
  /// Which media kinds the libretro-thumbnails sync fetches per ROM.
  /// Default is all three slottable kinds (boxart / snap / title).
  kindsToFetch: Accessor<MediaKind[]>;
  setKindsToFetch(kinds: MediaKind[]): Promise<void>;
  /// Re-hydrate the entire MediaIndex from Rust. Belt-and-braces after a
  /// sync — the per-ROM `oa://media-updated` events should already have
  /// covered everything, but a full pull guards against any missed events.
  refreshAll(): Promise<void>;
  /// Mutations — wrap Tauri commands that update Rust's MediaDb. All of them
  /// rely on Rust emitting `oa://media-updated` for the local store to
  /// catch up; the Promise resolves once the command itself returns.
  /// `kind` defaults to "box-front" (the historical "set cover" gesture).
  /// Pass an explicit kind to target a different slot — e.g. "clear-logo",
  /// "screenshot-gameplay", "manual" — once the UI surfaces them.
  setManualCover(romId: string, systemId: SystemId, sourcePath: string, kind?: MediaKind): Promise<void>;
  setSelectedVariant(romId: string, kind: MediaKind, index: number): Promise<void>;
  clearMedia(romId: string): Promise<void>;
  syncSystem(systemId: SystemId, entries: Array<{ id: string; title: string; filePath: string; systemId: string }>): Promise<void>;
  syncMetadata(systemId: SystemId, entries: Array<{ id: string; title: string; filePath: string; systemId: string }>): Promise<void>;
};

/// Dispatch a MediaKind name to the matching GameMedia variants array.
/// Accepts both new kebab-case names and v1 legacy strings — the latter
/// for any caller still passing "boxart"/"snap"/"title"/"cart". Reads
/// the v1 GameMedia field as a defensive fallback in case a stale
/// in-memory snapshot from the pre-rename world is still around.
function variantsForKind(gm: GameMedia, kind: MediaKind | string): MediaVariant[] | undefined {
  switch (kind) {
    case "box-front":
    case "boxart": return gm.boxFront ?? gm.boxart;
    case "screenshot-gameplay":
    case "snap": return gm.screenshotGameplay ?? gm.snap;
    case "screenshot-title":
    case "title": return gm.screenshotTitle ?? gm.title;
    case "cart-front":
    case "cart": return gm.cartFront ?? gm.cart;
    case "disc": return gm.disc;
    case "box-back": return gm.boxBack;
    case "box-3d": return gm.box3d;
    case "box-spine": return gm.boxSpine;
    case "box-full": return gm.boxFull;
    case "cart-back": return gm.cartBack;
    case "cart-3d": return gm.cart3d;
    case "screenshot-select": return gm.screenshotSelect;
    case "banner": return gm.banner;
    case "clear-logo": return gm.clearLogo;
    case "fanart-background": return gm.fanartBackground;
    case "fanart-disc": return gm.fanartDisc;
    case "advert-front": return gm.advertFront;
    case "advert-back": return gm.advertBack;
    case "arcade-cabinet": return gm.arcadeCabinet;
    case "arcade-marquee": return gm.arcadeMarquee;
    case "arcade-controlpanel": return gm.arcadeControlpanel;
    case "arcade-controlsinfo": return gm.arcadeControlsinfo;
    case "arcade-playerselect": return gm.arcadePlayerselect;
    case "arcade-flyer": return gm.arcadeFlyer;
    case "video": return gm.video;
    case "music": return gm.music;
    case "manual": return gm.manual;
    default: return undefined;
  }
}

/// Look up the pinned-variant index for a given kind. Falls back to v1
/// legacy fields when the new ones are missing.
function pinnedIndexForKind(sel: SelectedMedia | undefined, kind: MediaKind | string): number | undefined {
  if (!sel) return undefined;
  switch (kind) {
    case "box-front":
    case "boxart": return sel.boxFrontIndex ?? sel.boxartIndex;
    case "screenshot-gameplay":
    case "snap": return sel.screenshotGameplayIndex ?? sel.snapIndex;
    case "screenshot-title":
    case "title": return sel.screenshotTitleIndex ?? sel.titleIndex;
    case "cart-front":
    case "cart": return sel.cartFrontIndex;
    case "disc": return sel.discIndex;
    case "box-back": return sel.boxBackIndex;
    case "box-3d": return sel.box3dIndex;
    case "box-spine": return sel.boxSpineIndex;
    case "box-full": return sel.boxFullIndex;
    case "cart-back": return sel.cartBackIndex;
    case "cart-3d": return sel.cart3dIndex;
    case "screenshot-select": return sel.screenshotSelectIndex;
    case "banner": return sel.bannerIndex;
    case "clear-logo": return sel.clearLogoIndex;
    case "fanart-background": return sel.fanartBackgroundIndex;
    case "fanart-disc": return sel.fanartDiscIndex;
    case "advert-front": return sel.advertFrontIndex;
    case "advert-back": return sel.advertBackIndex;
    case "arcade-cabinet": return sel.arcadeCabinetIndex;
    case "arcade-marquee": return sel.arcadeMarqueeIndex;
    case "arcade-controlpanel": return sel.arcadeControlpanelIndex;
    case "arcade-controlsinfo": return sel.arcadeControlsinfoIndex;
    case "arcade-playerselect": return sel.arcadePlayerselectIndex;
    case "arcade-flyer": return sel.arcadeFlyerIndex;
    case "video": return sel.videoIndex;
    case "music": return sel.musicIndex;
    case "manual": return sel.manualIndex;
    default: return undefined;
  }
}

const MediaContext = createContext<MediaStore>();

export const MediaProvider: Component<{ children: JSX.Element }> = (props) => {
  // Signal-of-Map: any setter call (replace OR per-key) triggers ALL
  // consumers reading via `index()`. That's the right trade-off here —
  // simple, predictable, fast enough at our scale.
  const [index, setIndex] = createSignal<Map<string, GameMedia>>(new Map());
  const [hydrated, setHydrated] = createSignal(false);
  const [regionPriority, setRegionPriorityInternal] = createSignal<string[]>([
    "USA", "World", "Europe", "Japan",
  ]);
  const [kindsToFetch, setKindsToFetchInternal] = createSignal<MediaKind[]>([
    "box-front", "screenshot-gameplay", "screenshot-title",
  ]);
  // appDataDir absolute path — resolved at mount, used to construct
  // asset-protocol URLs for cover images. Custom URI schemes
  // (oa-media://) get ERR_UNKNOWN_URL_SCHEME from the WebView when the
  // page is loaded from a Vite HTTP origin in dev mode; Tauri's asset
  // protocol is specifically configured to work cross-origin.
  const [appDataPath, setAppDataPath] = createSignal<string>("");
  let unlistenUpdate: UnlistenFn | undefined;

  function upsertOne(romId: string, gm: GameMedia | undefined) {
    setIndex((prev) => {
      const next = new Map(prev);
      if (gm === undefined) {
        next.delete(romId);
      } else {
        next.set(romId, gm);
      }
      return next;
    });
  }

  async function refresh(romId: string, inline?: GameMedia) {
    if (inline !== undefined) {
      upsertOne(romId, inline);
      return;
    }
    try {
      const next = await invoke<MediaIndex>("get_media_index");
      upsertOne(romId, next[romId]);
    } catch (e) {
      console.warn("MediaProvider: refresh failed:", e);
    }
  }

  onMount(async () => {
    // Resolve the absolute data dir first — `convertFileSrc` requires
    // the full path. Without this, coverUrl() can't construct asset URLs.
    // In portable mode this returns `<exe_dir>/settings/`; otherwise
    // it's Tauri's app_data_dir.
    try {
      const p = await getDataDir();
      setAppDataPath(p);
      console.log("[oa-media] dataDir =", p);
    } catch (e) {
      console.warn("MediaProvider: getDataDir() failed:", e);
    }
    // Install the listener FIRST so events fired during hydration (or during
    // a fast sync started moments after app launch) aren't dropped between
    // hydrate-await and listen-await.
    try {
      unlistenUpdate = await listen<MediaUpdatedEvent>("oa://media-updated", (ev) => {
        console.log("[oa-media] event for", ev.payload.romId, "media:", ev.payload.media);
        void refresh(ev.payload.romId, ev.payload.media);
      });
    } catch (e) {
      console.warn("MediaProvider: listen('oa://media-updated') failed:", e);
    }
    try {
      const initial = await invoke<MediaIndex>("get_media_index");
      const map = new Map(Object.entries(initial));
      console.log("[oa-media] hydrated MediaIndex with", map.size, "entries:", [...map.keys()]);
      // Merge instead of replace — the listener was installed first
      // (above) so per-row oa://media-updated events that arrived
      // between listener-install and this hydrate are already in
      // `prev`. A plain `setIndex(map)` would clobber them with the
      // older hydrate snapshot. Newer wins: copy hydrate values
      // first, then overlay anything `prev` already has so in-flight
      // updates take precedence.
      setIndex((prev) => {
        const merged = new Map(map);
        for (const [k, v] of prev) {
          merged.set(k, v);
        }
        return merged;
      });
    } catch (e) {
      console.warn("MediaProvider: initial hydrate failed:", e);
    }
    try {
      const priority = await invoke<string[]>("get_region_priority");
      if (Array.isArray(priority) && priority.length > 0) {
        setRegionPriorityInternal(priority);
      }
    } catch (e) {
      console.warn("MediaProvider: get_region_priority failed:", e);
    }
    try {
      const kinds = await invoke<string[]>("get_media_kinds_to_fetch");
      if (Array.isArray(kinds)) {
        // Filter to libretro-thumbnails-served kinds only (the only ones
        // sync actually fetches). Accept both new kebab-case names and
        // v1 aliases so a media-prefs.json from the pre-rename world
        // doesn't reset to defaults silently on first load.
        const remap: Record<string, MediaKind> = {
          "boxart": "box-front",
          "snap": "screenshot-gameplay",
          "title": "screenshot-title",
          "box-front": "box-front",
          "screenshot-gameplay": "screenshot-gameplay",
          "screenshot-title": "screenshot-title",
        };
        const validated: MediaKind[] = kinds
          .map((k) => remap[k])
          .filter((k): k is MediaKind => k !== undefined);
        if (validated.length > 0) setKindsToFetchInternal(validated);
      }
    } catch (e) {
      console.warn("MediaProvider: get_media_kinds_to_fetch failed:", e);
    }
    setHydrated(true);
  });

  function joinAppData(rel: string): string {
    const base = appDataPath();
    if (!base) return "";
    // Forward slashes work on Windows for file paths; Tauri normalizes.
    return base.endsWith("/") || base.endsWith("\\") ? `${base}${rel}` : `${base}/${rel}`;
  }

  onCleanup(() => unlistenUpdate?.());

  const store: MediaStore = {
    media(romId) {
      return index().get(romId);
    },
    coverUrl(_systemId, romId, kind = "box-front", size = "thumb") {
      const gm = index().get(romId);
      if (!gm) return null;
      const variants = variantsForKind(gm, kind);
      if (!variants || variants.length === 0) return null;
      const base = appDataPath();
      if (!base) return null;
      const pinned = pinnedIndexForKind(gm.selected, kind);
      const variant = pinned !== undefined && pinned < variants.length
        ? variants[pinned]
        : variants[0];
      // Resolve to the absolute on-disk path. Thumb path preferred for the
      // grid; falls back to the full image when no thumb was generated.
      const rel = size === "thumb" ? (variant.thumbPath ?? variant.path) : variant.path;
      return convertFileSrc(joinAppData(rel));
    },
    hydrated,
    regionPriority,
    kindsToFetch,
    async setKindsToFetch(kinds) {
      try {
        await invoke("set_media_kinds_to_fetch", { kinds });
        setKindsToFetchInternal(kinds);
      } catch (e) {
        console.warn("setKindsToFetch failed:", e);
        throw e;
      }
    },
    async refreshAll() {
      try {
        const next = await invoke<MediaIndex>("get_media_index");
        const map = new Map(Object.entries(next));
        console.log("[oa-media] refreshAll → MediaIndex now has", map.size, "entries:", [...map.keys()]);
        setIndex(map);
      } catch (e) {
        console.warn("MediaProvider: refreshAll failed:", e);
      }
    },
    async setRegionPriority(regions) {
      try {
        await invoke("set_region_priority", { regions });
        setRegionPriorityInternal(regions);
      } catch (e) {
        console.warn("setRegionPriority failed:", e);
        throw e;
      }
    },
    async setManualCover(romId, systemId, sourcePath, kind) {
      await invoke("set_manual_cover", { romId, systemId, sourcePath, kind });
      await refresh(romId);
    },
    async setSelectedVariant(romId, kind, idx) {
      await invoke("set_selected_variant", { romId, kind, index: idx });
      await refresh(romId);
    },
    async clearMedia(romId) {
      await invoke("clear_media", { romId });
      await refresh(romId);
    },
    async syncSystem(systemId, entries) {
      await invoke("sync_media_for_system", { systemId, entries });
      try {
        const next = await invoke<MediaIndex>("get_media_index");
        setIndex(new Map(Object.entries(next)));
      } catch (e) {
        console.warn("syncSystem post-hydrate failed:", e);
      }
    },
    async syncMetadata(systemId, entries) {
      await invoke("sync_metadata_for_system", { systemId, entries });
      try {
        const next = await invoke<MediaIndex>("get_media_index");
        setIndex(new Map(Object.entries(next)));
      } catch (e) {
        console.warn("syncMetadata post-hydrate failed:", e);
      }
    },
  };

  return (
    <MediaContext.Provider value={store}>{props.children}</MediaContext.Provider>
  );
};

export function useMedia(): MediaStore {
  const v = useContext(MediaContext);
  if (!v) {
    throw new Error("useMedia called outside <MediaProvider>");
  }
  return v;
}
