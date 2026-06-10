// Frontend Solid context for the per-system platform-media catalog
// (Phase 6 of the media-taxonomy plan + 2026-05-24 wheel-art followup).
//
// Shape mirrors Rust's `PlatformMediaState`: BTreeMap<systemId, PlatformMedia>
// where each PlatformMedia has 9 Option<MediaVariant> slots (banner /
// clear-logo / console / controller / fanart / marquee / photo / wheel
// / background). Today's only consumer is the SystemHeader's wheel-art
// surface; the kiosk shell will lean on this for its tile UI when that
// work stream picks up.
//
// Pattern matches the existing MediaProvider — hydrate once at mount,
// listen for oa://platform-media-updated for live patches, expose a
// `wheelUrl(systemId)` helper that returns the convertFileSrc URL for
// the wheel variant if set, else null.

import {
  createContext,
  createSignal,
  onCleanup,
  onMount,
  useContext,
  type Component,
  type JSX,
} from "solid-js";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { listenTo, OA_EVENTS } from "@oa/platform/api/eventsApi";
import { getDataDir } from "@oa/platform/lib/dataDir";
import { getPlatformMediaIndex } from "@oa/platform/api/mediaApi";

type MediaVariant = {
  source: { kind: string };
  region?: string;
  path: string;
  thumbPath?: string;
  width?: number;
  height?: number;
  sha1?: string;
  bytes?: number;
};

type PlatformMedia = {
  banner?: MediaVariant;
  clearLogo?: MediaVariant;
  console?: MediaVariant;
  controller?: MediaVariant;
  fanart?: MediaVariant;
  marquee?: MediaVariant;
  photo?: MediaVariant;
  wheel?: MediaVariant;
  background?: MediaVariant;
};

type PlatformMediaIndex = Record<string, PlatformMedia>;

type PlatformMediaStore = {
  /// Convenience: the wheel-art URL for `systemId` if present,
  /// else null (caller falls back to whatever the current
  /// gradient/text identity surface looks like).
  wheelUrl(systemId: string): string | null;
  /// Generic slot accessor — useful if future surfaces want banner /
  /// background / etc. Same null-on-miss semantics as wheelUrl.
  slotUrl(systemId: string, slot: keyof PlatformMedia): string | null;
  /// Raw access to the per-system media bundle.
  forSystem(systemId: string): PlatformMedia | undefined;
};

const PlatformMediaContext = createContext<PlatformMediaStore>();

export const PlatformMediaProvider: Component<{ children: JSX.Element }> = (props) => {
  const [index, setIndex] = createSignal<PlatformMediaIndex>({});
  const [dataDir, setDataDir] = createSignal<string>("");
  let unlisten: UnlistenFn | undefined;

  onMount(async () => {
    try {
      setDataDir(await getDataDir());
    } catch (e) {
      console.warn("[oa-platform-media] getDataDir failed:", e);
    }
    // Install listener BEFORE hydrate so any update fired during
    // hydration doesn't get dropped between await points.
    try {
      unlisten = await listenTo<{ systemId: string; slot: string; media: PlatformMedia }>(
        OA_EVENTS.platformMediaUpdated,
        (ev) => {
          setIndex((prev) => ({ ...prev, [ev.payload.systemId]: ev.payload.media }));
        },
      );
    } catch (e) {
      console.warn("[oa-platform-media] listen failed:", e);
    }
    try {
      const initial = await getPlatformMediaIndex<PlatformMediaIndex>();
      // Merge instead of replace — events that arrived between
      // listener-install and hydrate already populated `prev`; newer
      // wins.
      setIndex((prev) => {
        const merged: PlatformMediaIndex = { ...initial };
        for (const [k, v] of Object.entries(prev)) merged[k] = v;
        return merged;
      });
    } catch (e) {
      console.warn("[oa-platform-media] hydrate failed:", e);
    }
  });

  onCleanup(() => unlisten?.());

  function joinDataDir(rel: string): string {
    const base = dataDir();
    if (!base) return "";
    return base.endsWith("/") || base.endsWith("\\")
      ? `${base}${rel}`
      : `${base}/${rel}`;
  }

  const store: PlatformMediaStore = {
    forSystem(systemId) {
      return index()[systemId];
    },
    slotUrl(systemId, slot) {
      const pm = index()[systemId];
      if (!pm) return null;
      const v = pm[slot];
      if (!v) return null;
      const base = dataDir();
      if (!base) return null;
      return convertFileSrc(joinDataDir(v.path));
    },
    wheelUrl(systemId) {
      return this.slotUrl(systemId, "wheel");
    },
  };

  return (
    <PlatformMediaContext.Provider value={store}>
      {props.children}
    </PlatformMediaContext.Provider>
  );
};

export function usePlatformMedia(): PlatformMediaStore {
  const v = useContext(PlatformMediaContext);
  if (!v) {
    throw new Error("usePlatformMedia called outside <PlatformMediaProvider>");
  }
  return v;
}
