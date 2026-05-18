import { Show, type Component } from "solid-js";
import { useMedia } from "../../library/media";
import type { RomEntry } from "../../library/types";
import { DEFAULT_TILE_ASPECT, systemThemes } from "../../themes/registry";

type WidgetProps = {
  entry: RomEntry;
};

export type WidgetDef = {
  id: string;
  label: string;
  component: Component<WidgetProps>;
};

export const HeroCoverWidget: Component<WidgetProps> = (props) => {
  const media = useMedia();
  const theme = () => systemThemes[props.entry.systemId];
  const aspect = () => theme().tileAspect ?? DEFAULT_TILE_ASPECT;
  const url = () => media.coverUrl(props.entry.systemId, props.entry.id, "boxart", "full");

  return (
    <section data-system={props.entry.systemId} class="px-3">
      <div
        class="relative w-full overflow-hidden rounded-lg border border-white/5 bg-(--color-oa-bg)"
        style={{ "aspect-ratio": aspect() }}
      >
        <Show
          when={url()}
          fallback={
            <div class="grid h-full w-full place-items-center bg-linear-to-br from-(--color-system-accent-soft)/15 to-(--color-system-glow)/20 text-3xl text-(--color-system-accent)">
              ◐
            </div>
          }
        >
          {(src) => (
            <img
              src={src()}
              alt={`${props.entry.title} cover`}
              loading="lazy"
              decoding="async"
              class="h-full w-full object-contain"
            />
          )}
        </Show>
      </div>
    </section>
  );
};

export const TitleBlockWidget: Component<WidgetProps> = (props) => {
  const media = useMedia();
  const meta = () => media.media(props.entry.id)?.metadata;
  const theme = () => systemThemes[props.entry.systemId];

  return (
    <section data-system={props.entry.systemId} class="space-y-1 px-3">
      <h2 class="line-clamp-2 text-base font-semibold text-(--color-oa-ink)">
        {props.entry.title}
      </h2>
      <p class="text-[0.65rem] uppercase tracking-[0.3em] text-(--color-oa-ink-dim)">
        <span class="text-(--color-system-accent)">{theme().shortName}</span>
        <Show when={meta()?.year}>{(y) => <> · {y()}</>}</Show>
      </p>
    </section>
  );
};

export const MetadataWidget: Component<WidgetProps> = (props) => {
  const media = useMedia();
  const meta = () => media.media(props.entry.id)?.metadata;
  const hasAny = () => {
    const m = meta();
    return m !== undefined && (m.developer || m.publisher || m.genre || m.players !== undefined);
  };

  return (
    <section data-system={props.entry.systemId} class="space-y-2 px-3">
      <h3 class="text-[0.55rem] font-semibold uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
        Metadata
      </h3>
      <Show
        when={hasAny()}
        fallback={
          <p class="text-[0.7rem] text-(--color-oa-ink-dim)">
            No metadata yet — run Sync metadata in Settings.
          </p>
        }
      >
        <dl class="space-y-1 text-xs">
          <Show when={meta()?.developer}>
            {(v) => <Row label="Developer" value={v()} />}
          </Show>
          <Show when={meta()?.publisher}>
            {(v) => <Row label="Publisher" value={v()} />}
          </Show>
          <Show when={meta()?.genre}>
            {(v) => <Row label="Genre" value={v()} />}
          </Show>
          <Show when={meta()?.players !== undefined}>
            <Row label="Players" value={String(meta()!.players)} />
          </Show>
        </dl>
      </Show>
    </section>
  );
};

const Row: Component<{ label: string; value: string }> = (props) => (
  <div class="flex items-baseline justify-between gap-3 border-b border-white/5 pb-1 last:border-0 last:pb-0">
    <dt class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">{props.label}</dt>
    <dd class="truncate text-right text-(--color-oa-ink)">{props.value}</dd>
  </div>
);

export const WIDGET_REGISTRY: Record<string, WidgetDef> = {
  hero: { id: "hero", label: "Cover", component: HeroCoverWidget },
  title: { id: "title", label: "Title", component: TitleBlockWidget },
  metadata: { id: "metadata", label: "Metadata", component: MetadataWidget },
};
