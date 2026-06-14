// Per-system platform-media art slots (banner / clear-logo / console /
// controller / fanart / marquee / photo / wheel / background). Controlled by a
// `systemId` accessor — no internal system picker — so it embeds in both the
// PlatformMediaDialog (which drives systemId via its own <select>) and the
// Systems hub's Media domain (scoped to the active system). The full index is
// fetched once; switching systemId just re-indexes into it.

import {
  createSignal,
  For,
  onCleanup,
  onMount,
  Show,
  type Accessor,
  type Component,
  type JSX,
} from "solid-js";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { listenTo, OA_EVENTS } from "@oa/platform/api/eventsApi";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import {
  clearPlatformMedia,
  getPlatformMediaIndex,
  setPlatformMedia,
} from "@oa/platform/api/mediaApi";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import { getDataDir } from "@oa/platform/lib/dataDir";

type MediaVariant = { path: string; thumbPath?: string };

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

type PlatformSlot =
  | "banner" | "clear-logo" | "console" | "controller"
  | "fanart" | "marquee" | "photo" | "wheel" | "background";

const SLOT_FIELD_NAME: Record<PlatformSlot, keyof PlatformMedia> = {
  "banner": "banner",
  "clear-logo": "clearLogo",
  "console": "console",
  "controller": "controller",
  "fanart": "fanart",
  "marquee": "marquee",
  "photo": "photo",
  "wheel": "wheel",
  "background": "background",
};

const SLOT_LABELS: Record<PlatformSlot, string> = {
  "banner": "Banner",
  "clear-logo": "Clear logo",
  "console": "Console photo",
  "controller": "Controller",
  "fanart": "Fanart",
  "marquee": "Marquee",
  "photo": "Real-world photo",
  "wheel": "Wheel art",
  "background": "Background",
};

const SLOT_HINTS: Record<PlatformSlot, string> = {
  "banner": "Wide banner image (typically 1280×400-ish).",
  "clear-logo": "Transparent system logo. PNG recommended.",
  "console": "Hardware photo — system unit on a neutral background.",
  "controller": "Photo of the controller, ideally hero shot.",
  "fanart": "Atmospheric / promotional artwork. Wide landscape.",
  "marquee": "Arcade-marquee-style branded banner.",
  "photo": "Real-world / in-context photo of the hardware.",
  "wheel": "Transparent system logo for kiosk-wheel UI (PNG).",
  "background": "Full-bleed background image for system pages.",
};

const ALL_SLOTS: PlatformSlot[] = [
  "banner", "clear-logo", "console", "controller",
  "fanart", "marquee", "photo", "wheel", "background",
];

export const PlatformMediaSlots: Component<{ systemId: Accessor<SystemId> }> = (props) => {
  const [index, setIndex] = createSignal<PlatformMediaIndex>({});
  const [busy, setBusy] = createSignal<{ slot: PlatformSlot; op: string } | null>(null);
  const [errMsg, setErrMsg] = createSignal<string>("");
  const [dataDir, setDataDir] = createSignal<string>("");
  let unlisten: UnlistenFn | undefined;

  onMount(async () => {
    try {
      setDataDir(await getDataDir());
    } catch (e) {
      console.warn("[oa-platform-media] getDataDir failed:", e);
    }
    try {
      unlisten = await listenTo<{ systemId: string; slot: string; media: PlatformMedia }>(
        OA_EVENTS.platformMediaUpdated,
        (ev) => setIndex((prev) => ({ ...prev, [ev.payload.systemId]: ev.payload.media })),
      );
    } catch (e) {
      console.warn("[oa-platform-media] listen failed:", e);
    }
    await refreshIndex();
  });
  onCleanup(() => unlisten?.());

  async function refreshIndex(): Promise<void> {
    try {
      setIndex(await getPlatformMediaIndex<PlatformMediaIndex>());
    } catch (e) {
      console.warn("[oa-platform-media] hydrate failed:", e);
    }
  }

  const currentPm = (): PlatformMedia => index()[props.systemId()] ?? {};
  const variantForSlot = (slot: PlatformSlot): MediaVariant | undefined =>
    currentPm()[SLOT_FIELD_NAME[slot]];

  function joinDataDir(rel: string): string {
    const base = dataDir();
    if (!base) return "";
    return base.endsWith("/") || base.endsWith("\\") ? `${base}${rel}` : `${base}/${rel}`;
  }
  function previewUrl(slot: PlatformSlot): string | null {
    const v = variantForSlot(slot);
    if (!v || !dataDir()) return null;
    return convertFileSrc(joinDataDir(v.path));
  }

  async function chooseFile(slot: PlatformSlot): Promise<void> {
    setErrMsg("");
    try {
      const sel = await openFileDialog({
        directory: false,
        multiple: false,
        filters: [{ name: "Image", extensions: ["png", "jpg", "jpeg", "webp"] }],
        title: `Pick ${SLOT_LABELS[slot].toLowerCase()} image for ${systemThemes[props.systemId()]?.displayName ?? props.systemId()}`,
      });
      if (typeof sel !== "string" || !sel) return;
      setBusy({ slot, op: "set" });
      await setPlatformMedia(props.systemId(), slot, sel);
    } catch (e) {
      console.warn("[oa-platform-media] set failed:", e);
      setErrMsg(String(e));
    } finally {
      setBusy(null);
    }
  }

  async function clearSlot(slot: PlatformSlot): Promise<void> {
    setErrMsg("");
    setBusy({ slot, op: "clear" });
    try {
      await clearPlatformMedia(props.systemId(), slot);
    } catch (e) {
      console.warn("[oa-platform-media] clear failed:", e);
      setErrMsg(String(e));
    } finally {
      setBusy(null);
    }
  }

  const slotRow = (slot: PlatformSlot): JSX.Element => {
    const isBusy = (): boolean => busy()?.slot === slot;
    const v = (): MediaVariant | undefined => variantForSlot(slot);
    const url = (): string | null => previewUrl(slot);
    return (
      <div class="flex items-start gap-3 rounded border border-white/10 bg-white/[0.02] p-2">
        <div class="flex h-[72px] w-[96px] shrink-0 items-center justify-center overflow-hidden rounded border border-white/10 bg-black/30">
          <Show
            when={url()}
            fallback={
              <span class="text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim)">
                empty
              </span>
            }
          >
            <img src={url()!} alt={SLOT_LABELS[slot]} class="h-full w-full object-contain" />
          </Show>
        </div>
        <div class="flex min-w-0 flex-1 flex-col gap-1">
          <div class="flex items-center justify-between gap-2">
            <span class="text-xs font-semibold uppercase tracking-wider text-(--color-oa-ink)">
              {SLOT_LABELS[slot]}
            </span>
            <div class="flex items-center gap-2">
              <button
                type="button"
                disabled={isBusy()}
                onClick={() => void chooseFile(slot)}
                class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink) transition hover:bg-white/[0.08] disabled:cursor-not-allowed disabled:opacity-40"
              >
                {isBusy() && busy()?.op === "set" ? "Setting…" : "Choose…"}
              </button>
              <Show when={v()}>
                <button
                  type="button"
                  disabled={isBusy()}
                  onClick={() => void clearSlot(slot)}
                  class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:cursor-not-allowed disabled:opacity-40"
                >
                  {isBusy() && busy()?.op === "clear" ? "Clearing…" : "Clear"}
                </button>
              </Show>
            </div>
          </div>
          <p class="text-[0.7rem] text-(--color-oa-ink-dim)">{SLOT_HINTS[slot]}</p>
          <Show when={v()}>
            <p class="truncate text-[0.65rem] text-(--color-oa-ink-dim)/70">{v()!.path}</p>
          </Show>
        </div>
      </div>
    );
  };

  return (
    <div class="space-y-4">
      <Show when={errMsg()}>
        <p class="rounded border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-200">
          {errMsg()}
        </p>
      </Show>
      <div class="grid gap-2 lg:grid-cols-2">
        <For each={ALL_SLOTS}>{(slot) => slotRow(slot)}</For>
      </div>
    </div>
  );
};
