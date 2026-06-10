// Tools → Screenshot gallery. Lists PNG files under
// `appData/screenshots/<rom-stem>/` for the currently-running (or
// focused) game. Pre-redesign, F12 wrote screenshots that the user could
// only see by opening the appData folder; now they have a UI.

import { createResource, For, Show, type Component } from "solid-js";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listScreenshots, deleteScreenshot, openScreenshotFolder } from "@oa/platform/api/captureApi";
import { Dialog } from "@oa/platform/components/Dialog";
import type { RomEntry } from "@oa/platform/library/types";

type ScreenshotEntry = {
  path: string;
  fileName: string;
  sizeBytes: number;
  modifiedUnixMs: number;
};

type Props = {
  open: boolean;
  onClose: () => void;
  entry: RomEntry | null;
};

function humanBytes(b: number): string {
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
  return `${(b / 1024 / 1024).toFixed(1)} MB`;
}

function formatTimestamp(unixMs: number): string {
  if (!Number.isFinite(unixMs) || unixMs <= 0) return "";
  try {
    const d = new Date(unixMs);
    return d.toLocaleString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return "";
  }
}

export const ScreenshotGalleryDialog: Component<Props> = (props) => {
  // Re-fetch each time the dialog opens or the target game changes. F12
  // can write new files mid-session, so a fresh fetch each open keeps the
  // list current without us subscribing to fs events.
  const [shots, { refetch }] = createResource(
    () => ({ open: props.open, romPath: props.entry?.filePath ?? "" }),
    async (src): Promise<ScreenshotEntry[]> => {
      if (!src.open || !src.romPath) return [];
      try {
        return await listScreenshots(src.romPath);
      } catch (e) {
        console.warn("[oa-screenshots] list_screenshots failed:", e);
        return [];
      }
    },
  );

  async function deleteOne(path: string) {
    try {
      await deleteScreenshot(path);
      void refetch();
    } catch (e) {
      console.warn("[oa-screenshots] delete failed:", e);
    }
  }

  async function openFolder() {
    const romPath = props.entry?.filePath;
    if (!romPath) return;
    try {
      await openScreenshotFolder(romPath);
    } catch (e) {
      console.warn("[oa-screenshots] open folder failed:", e);
    }
  }

  return (
    <Dialog
      open={props.open}
      onClose={props.onClose}
      title="Screenshot gallery"
      subtitle={props.entry?.title}
      system={props.entry?.systemId}
      size="lg"
    >
      <div class="mb-3 flex items-center justify-between">
        <span class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          {shots()?.length ?? 0} screenshot{shots()?.length === 1 ? "" : "s"}{" · "}F12 saves new ones
        </span>
        <button
          type="button"
          onClick={openFolder}
          disabled={!shots() || shots()!.length === 0}
          class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:cursor-not-allowed disabled:opacity-40"
        >
          📁 Open folder
        </button>
      </div>

      <Show
        when={(shots() ?? []).length > 0}
        fallback={
          <div class="grid place-items-center rounded-md border border-white/5 bg-white/[0.02] px-8 py-12 text-center">
            <p class="text-3xl text-(--color-oa-ink-dim)">📸</p>
            <p class="mt-3 text-[0.65rem] uppercase tracking-[0.3em] text-(--color-oa-ink-dim)">
              No screenshots yet
            </p>
            <p class="mt-2 text-xs text-(--color-oa-ink-dim)">
              Press <kbd class="rounded border border-white/15 bg-white/[0.04] px-1.5 py-0.5 font-mono text-[0.7rem] text-(--color-oa-ink)">F12</kbd> during gameplay to capture the framebuffer.
            </p>
          </div>
        }
      >
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
          <For each={shots() ?? []}>
            {(s) => (
              <figure class="overflow-hidden rounded-md border border-white/10 bg-black/40">
                <div class="aspect-[4/3] w-full overflow-hidden">
                  <img
                    src={convertFileSrc(s.path)}
                    alt={s.fileName}
                    loading="lazy"
                    decoding="async"
                    class="h-full w-full object-contain"
                  />
                </div>
                <figcaption class="flex items-center justify-between gap-2 border-t border-white/5 px-2 py-1.5">
                  <div class="min-w-0 flex-1">
                    <p class="truncate text-[0.65rem] text-(--color-oa-ink-dim)">
                      {formatTimestamp(s.modifiedUnixMs)}
                    </p>
                    <p class="truncate text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                      {humanBytes(s.sizeBytes)}
                    </p>
                  </div>
                  <button
                    type="button"
                    onClick={() => void deleteOne(s.path)}
                    class="rounded border border-red-500/30 px-1.5 py-0.5 text-xs text-red-300 transition hover:bg-red-500/10"
                    title="Delete screenshot"
                    aria-label="Delete screenshot"
                  >
                    ✕
                  </button>
                </figcaption>
              </figure>
            )}
          </For>
        </div>
      </Show>
    </Dialog>
  );
};
