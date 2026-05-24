import { createMemo, createSignal, For, onCleanup, onMount, Show, type Component } from "solid-js";
import { convertFileSrc } from "@tauri-apps/api/core";
import { getDataDir } from "../lib/dataDir";
import type { RomEntry } from "../library/types";
import { useMedia } from "../library/media";
import { DEFAULT_TILE_ASPECT, systemThemes } from "../themes/registry";

type Props = {
  entry: RomEntry | null;
  onClose: () => void;
};

/// Modal for picking which regional variant of a game's boxart to display.
/// Renders all available variants side-by-side at ~200px wide, each with a
/// region badge. Click a variant to pin it via set_selected_variant. ESC and
/// backdrop-click close. Reuses the standard modal backdrop conventions
/// (SaveSlotsModal / GameInfoModal / ImportWizard).
const RegionPicker: Component<Props> = (props) => {
  const media = useMedia();
  const [appDataPath, setAppDataPath] = createSignal("");
  onMount(async () => {
    try {
      setAppDataPath(await getDataDir());
    } catch (e) {
      console.warn("RegionPicker: getDataDir failed:", e);
    }
  });
  function joinAppData(rel: string): string {
    const base = appDataPath();
    if (!base) return "";
    return base.endsWith("/") || base.endsWith("\\") ? `${base}${rel}` : `${base}/${rel}`;
  }

  const gm = createMemo(() => {
    const e = props.entry;
    return e ? media.media(e.id) : undefined;
  });
  // Post-2026-05-23 media-taxonomy rename: boxFront is the new field
  // name, `boxart` stays as a defensive fallback for one release in
  // case a stale in-memory snapshot is still around.
  const variants = createMemo(() => gm()?.boxFront ?? gm()?.boxart ?? []);
  const activePin = createMemo(() => gm()?.selected?.boxFrontIndex ?? gm()?.selected?.boxartIndex);
  const tileAspect = createMemo(() => {
    const e = props.entry;
    return (e ? systemThemes[e.systemId].tileAspect : null) ?? DEFAULT_TILE_ASPECT;
  });

  function onWindowKey(e: KeyboardEvent) {
    if (e.key === "Escape" && props.entry) {
      e.stopPropagation();
      props.onClose();
    }
  }
  onMount(() => window.addEventListener("keydown", onWindowKey, { capture: true }));
  onCleanup(() => window.removeEventListener("keydown", onWindowKey, { capture: true }));

  async function pick(idx: number) {
    if (!props.entry) return;
    try {
      await media.setSelectedVariant(props.entry.id, "box-front", idx);
    } catch (e) {
      console.warn("setSelectedVariant failed:", e);
    }
    props.onClose();
  }

  return (
    <Show when={props.entry}>
      <div
        class="fixed inset-0 z-50 grid place-items-center bg-black/60 backdrop-blur-sm"
        onClick={(e) => {
          if (e.currentTarget === e.target) props.onClose();
        }}
      >
        <div
          class="w-full max-w-3xl rounded-lg border border-white/10 bg-(--color-oa-bg-deep) shadow-2xl shadow-black/60"
          role="dialog"
          aria-modal="true"
          aria-labelledby="region-picker-title"
          data-system={props.entry!.systemId}
        >
          <header class="flex items-center justify-between border-b border-white/5 px-6 py-4">
            <div>
              <h2
                id="region-picker-title"
                class="text-sm font-semibold uppercase tracking-[0.3em] text-(--color-oa-ink)"
              >
                Pick region
              </h2>
              <p class="mt-0.5 text-xs text-(--color-oa-ink-dim)">{props.entry!.title}</p>
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

          <section class="px-6 py-6">
            <Show
              when={variants().length > 0}
              fallback={
                <p class="text-xs text-(--color-oa-ink-dim)">
                  No boxart variants available for this game yet. Pick a cover file
                  or run a libretro-thumbnails sync first.
                </p>
              }
            >
              <ul class="flex flex-wrap items-start gap-4">
                <For each={variants()}>
                  {(v, i) => {
                    const isActive = () => (activePin() ?? -1) === i();
                    const region = v.region ?? "—";
                    // Resolve directly to this variant's on-disk path via
                    // the asset protocol — no per-variant index gymnastics
                    // needed because we already have the variant in hand.
                    const rel = v.thumbPath ?? v.path;
                    const src = convertFileSrc(joinAppData(rel));
                    return (
                      <li>
                        <button
                          type="button"
                          onClick={() => void pick(i())}
                          class="group relative block w-[180px] overflow-hidden rounded-md border border-white/10 bg-white/[0.03] shadow-lg shadow-black/40 transition hover:-translate-y-0.5 hover:shadow-xl hover:shadow-black/60 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                          aria-pressed={isActive()}
                        >
                          <div
                            class="relative w-full overflow-hidden bg-black"
                            style={{ "aspect-ratio": tileAspect() }}
                          >
                            {/*
                              Note: this <img> bypasses the MediaContext's
                              coverUrl helper because we want each variant
                              indexed individually, not the resolved active
                              one. Use index-based URL with cache-buster.
                            */}
                            <img
                              src={src}
                              alt={`${props.entry!.title} (${region})`}
                              class="absolute inset-0 h-full w-full object-contain"
                              loading="eager"
                            />
                            <Show when={isActive()}>
                              <div
                                class="pointer-events-none absolute inset-0"
                                style={{ "box-shadow": "inset 0 0 0 3px var(--color-system-accent)" }}
                              />
                            </Show>
                          </div>
                          <div class="flex items-center justify-between gap-2 px-2 py-1.5">
                            <span class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                              {region}
                            </span>
                            <span class="text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                              {v.source.kind === "manual" ? "manual" : "synced"}
                            </span>
                          </div>
                        </button>
                      </li>
                    );
                  }}
                </For>
              </ul>
            </Show>
          </section>
        </div>
      </div>
    </Show>
  );
};

export default RegionPicker;
