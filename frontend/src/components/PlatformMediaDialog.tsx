// Library Manager → Game media → "Platform media…"
//
// Per-system hardware-photo / controller / wheel / banner / etc.
// management. Phase 6 of the media-taxonomy plan — the data model
// + storage land here; the kiosk shell (separate work stream) will
// consume `wheel/<systemId>.png` for its tile UI.
//
// Each system gets its own PlatformMedia (9 Option-shaped slots).
// Slots are file-per-system (not per-rom + region variants like
// GameMedia), so the UI is simpler: one preview tile per slot, with
// Choose… / Clear affordances.

import {
  createSignal,
  For,
  Show,
  onMount,
  onCleanup,
  type Component,
  type JSX,
} from "solid-js";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { Dialog } from "../layout/Dialog";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import { getDataDir } from "@oa/platform/lib/dataDir";

type Props = {
  open: boolean;
  onClose: () => void;
  /// Optional initial system to focus when opening. Used by future
  /// per-system page entry points; for now defaults to the first
  /// system in the registry.
  initialSystemId?: SystemId;
  /// "dialog" (default) wraps the body in the Dialog primitive — the
  /// legacy Library Manager → Game media → Platform media… entry.
  /// "panel" drops the Dialog wrapper + the Close button so the body
  /// embeds inside a parent shell that owns its own chrome
  /// (Retroverse-UI SETTINGS → Media category). In panel mode `open`
  /// and `onClose` are ignored.
  variant?: "dialog" | "panel";
};

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

type PlatformSlot =
  | "banner" | "clear-logo" | "console" | "controller"
  | "fanart" | "marquee" | "photo" | "wheel" | "background";

const SLOT_FIELD_NAME: Record<PlatformSlot, keyof PlatformMedia> = {
  "banner":     "banner",
  "clear-logo": "clearLogo",
  "console":    "console",
  "controller": "controller",
  "fanart":     "fanart",
  "marquee":    "marquee",
  "photo":      "photo",
  "wheel":      "wheel",
  "background": "background",
};

const SLOT_LABELS: Record<PlatformSlot, string> = {
  "banner":     "Banner",
  "clear-logo": "Clear logo",
  "console":    "Console photo",
  "controller": "Controller",
  "fanart":     "Fanart",
  "marquee":    "Marquee",
  "photo":      "Real-world photo",
  "wheel":      "Wheel art",
  "background": "Background",
};

const SLOT_HINTS: Record<PlatformSlot, string> = {
  "banner":     "Wide banner image (typically 1280×400-ish).",
  "clear-logo": "Transparent system logo. PNG recommended.",
  "console":    "Hardware photo — system unit on a neutral background.",
  "controller": "Photo of the controller, ideally hero shot.",
  "fanart":     "Atmospheric / promotional artwork. Wide landscape.",
  "marquee":    "Arcade-marquee-style branded banner.",
  "photo":      "Real-world / in-context photo of the hardware.",
  "wheel":      "Transparent system logo for kiosk-wheel UI (PNG).",
  "background": "Full-bleed background image for system pages.",
};

const ALL_SLOTS: PlatformSlot[] = [
  "banner", "clear-logo", "console", "controller",
  "fanart", "marquee", "photo", "wheel", "background",
];

const ALL_SYSTEM_IDS = Object.keys(systemThemes) as SystemId[];

export const PlatformMediaDialog: Component<Props> = (props) => {
  const [systemId, setSystemId] = createSignal<SystemId>(
    props.initialSystemId ?? ALL_SYSTEM_IDS[0],
  );
  const [index, setIndex] = createSignal<PlatformMediaIndex>({});
  const [busy, setBusy] = createSignal<{ slot: PlatformSlot; op: string } | null>(null);
  const [errMsg, setErrMsg] = createSignal<string>("");
  const [dataDir, setDataDir] = createSignal<string>("");
  let unlisten: UnlistenFn | undefined;

  // Initial hydrate + listener install when the dialog mounts.
  onMount(async () => {
    try {
      setDataDir(await getDataDir());
    } catch (e) {
      console.warn("[oa-platform-media] getDataDir failed:", e);
    }
    try {
      unlisten = await listen<{ systemId: string; slot: string; media: PlatformMedia }>(
        "oa://platform-media-updated",
        (ev) => {
          setIndex((prev) => ({ ...prev, [ev.payload.systemId]: ev.payload.media }));
        },
      );
    } catch (e) {
      console.warn("[oa-platform-media] listen failed:", e);
    }
    await refreshIndex();
  });
  onCleanup(() => unlisten?.());

  async function refreshIndex() {
    try {
      const next = await invoke<PlatformMediaIndex>("get_platform_media_index");
      setIndex(next);
    } catch (e) {
      console.warn("[oa-platform-media] hydrate failed:", e);
    }
  }

  function currentPm(): PlatformMedia {
    return index()[systemId()] ?? {};
  }

  function variantForSlot(slot: PlatformSlot): MediaVariant | undefined {
    return currentPm()[SLOT_FIELD_NAME[slot]];
  }

  function joinDataDir(rel: string): string {
    const base = dataDir();
    if (!base) return "";
    return base.endsWith("/") || base.endsWith("\\")
      ? `${base}${rel}`
      : `${base}/${rel}`;
  }

  function previewUrl(slot: PlatformSlot): string | null {
    const v = variantForSlot(slot);
    if (!v) return null;
    if (!dataDir()) return null;
    return convertFileSrc(joinDataDir(v.path));
  }

  async function chooseFile(slot: PlatformSlot) {
    setErrMsg("");
    try {
      const sel = await openFileDialog({
        directory: false,
        multiple: false,
        filters: [
          { name: "Image", extensions: ["png", "jpg", "jpeg", "webp"] },
        ],
        title: `Pick ${SLOT_LABELS[slot].toLowerCase()} image for ${systemThemes[systemId()].displayName}`,
      });
      if (typeof sel !== "string" || !sel) return;
      setBusy({ slot, op: "set" });
      await invoke("set_platform_media", {
        systemId: systemId(),
        slot,
        sourcePath: sel,
      });
      // Listener will update the index; we don't need to refetch.
    } catch (e) {
      console.warn("[oa-platform-media] set failed:", e);
      setErrMsg(String(e));
    } finally {
      setBusy(null);
    }
  }

  async function clearSlot(slot: PlatformSlot) {
    setErrMsg("");
    setBusy({ slot, op: "clear" });
    try {
      await invoke("clear_platform_media", { systemId: systemId(), slot });
    } catch (e) {
      console.warn("[oa-platform-media] clear failed:", e);
      setErrMsg(String(e));
    } finally {
      setBusy(null);
    }
  }

  const slotRow = (slot: PlatformSlot): JSX.Element => {
    const isBusy = () => busy()?.slot === slot;
    const v = () => variantForSlot(slot);
    const url = () => previewUrl(slot);
    return (
      <div class="flex items-start gap-3 rounded border border-white/10 bg-white/[0.02] p-2">
        {/* Preview cell — fixed 96×72-ish to keep rows aligned. */}
        <div class="flex h-[72px] w-[96px] shrink-0 items-center justify-center overflow-hidden rounded border border-white/10 bg-black/30">
          <Show
            when={url()}
            fallback={
              <span class="text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim)">
                empty
              </span>
            }
          >
            <img
              src={url()!}
              alt={SLOT_LABELS[slot]}
              class="h-full w-full object-contain"
            />
          </Show>
        </div>
        {/* Label + hint + actions. */}
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
            <p class="truncate text-[0.65rem] text-(--color-oa-ink-dim)/70">
              {v()!.path}
            </p>
          </Show>
        </div>
      </div>
    );
  };

  const body: JSX.Element = (
    <div class="space-y-4 p-4">
      {/* System picker */}
      <div class="space-y-1">
        <label class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
          System
        </label>
        <select
          value={systemId()}
          onChange={(e) => setSystemId(e.currentTarget.value as SystemId)}
          class="w-full rounded border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs text-(--color-oa-ink)"
        >
          <For each={ALL_SYSTEM_IDS}>
            {(sid) => (
              <option value={sid}>
                {systemThemes[sid].displayName} ({sid})
              </option>
            )}
          </For>
        </select>
      </div>

      {/* Error */}
      <Show when={errMsg()}>
        <p class="rounded border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-200">
          {errMsg()}
        </p>
      </Show>

      {/* Slot grid */}
      <div class="grid gap-2 lg:grid-cols-2">
        <For each={ALL_SLOTS}>{(slot) => slotRow(slot)}</For>
      </div>

      {/* Close — dialog only. In panel mode the embedding shell
          (SETTINGS sidebar) owns navigation away from this category. */}
      <Show when={props.variant !== "panel"}>
        <div class="flex justify-end gap-2 pt-2">
          <button
            type="button"
            onClick={props.onClose}
            class="rounded border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
          >
            Close
          </button>
        </div>
      </Show>
    </div>
  );

  return (
    <Show
      when={props.variant !== "panel"}
      fallback={body}
    >
      <Dialog
        open={props.open}
        onClose={props.onClose}
        title="Platform media"
        subtitle="Per-system hardware photos, controllers, wheel art, banners"
        size="xl"
      >
        {body}
      </Dialog>
    </Show>
  );
};
