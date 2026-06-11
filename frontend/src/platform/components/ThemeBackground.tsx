// Theme-owned background surface (S5.5 — the revival of the dead SystemBackground).
//
// A theme mounts `<ThemeBackground systemId={…} />` to paint a full-bleed backdrop
// behind its content. It is the **live consumer of the S5.1 background resolver
// tier**: the asset resolves through the theme→platform cascade
// (`assets/themes/<activeTheme>/system-ui/<system|_baseline>/backgrounds/…` →
// `assets/system-ui/<system|_baseline>/backgrounds/…`), with the active theme id
// supplied AMBIENTLY by `resolveBackgroundAsset` (S5.1). So:
//   - a theme drops `assets/themes/<id>/system-ui/_baseline/backgrounds/default.png`
//     for one theme-wide backdrop, or per-system files for per-system backdrops;
//   - with nothing theme-specific, it falls back to the platform per-system /
//     `_baseline` asset, or renders nothing (a quiet shell).
//
// History: the original `SystemBackground` was a Retroverse-era overlay dropped
// 2026-05-31 (its accent-gradient + per-system focus competed with Retroverse's
// own chrome) and sat unmounted ever since. S5.5 revives it as a generic,
// theme-opt-in surface (no `perSystemUiEnabled` gate — a theme that mounts it
// WANTS it; no accent gradient — the backdrop is the theme's own image) so the
// S5.1 background tier finally has a consumer. CoverFlow is the first to mount it.

import { Show, createMemo, createResource, type Component } from "solid-js";
import { convertFileSrc } from "@tauri-apps/api/core";
import { resolveBackgroundAsset } from "@oa/platform/api/mediaApi";
import { activeThemeId } from "@oa/platform/theme/registry";
import type { SystemId } from "@oa/platform/themes/registry";

type BackgroundKind = "default" | "animated";

type Props = {
  /// System whose backdrop to resolve (drives the per-system tier of the
  /// cascade). `null` → render nothing. A system-agnostic theme can pass a fixed
  /// system or the focused game's system; either way a theme-wide `_baseline`
  /// backdrop wins when no per-system file exists.
  systemId: () => SystemId | null;
  /// Static image (`"default"`) or looping video (`"animated"`). Default static.
  /// An animated request with no animated asset falls back to the static cascade.
  kind?: BackgroundKind;
  /// Backdrop opacity (0–1). Default 0.5 — present but subordinate to foreground.
  opacity?: number;
};

type Resolved = { kind: BackgroundKind; url: string };

async function resolveUrl(systemId: SystemId, kind: BackgroundKind): Promise<Resolved | null> {
  try {
    // S5.1: pass the ambient active theme id so the resolver checks the theme
    // tier (assets/themes/<id>/…) before the platform per-system / _baseline.
    const themeId = activeThemeId();
    const primary = await resolveBackgroundAsset(themeId, systemId, kind);
    if (primary) return { kind, url: convertFileSrc(primary) };
    if (kind === "animated") {
      const fallback = await resolveBackgroundAsset(themeId, systemId, "default");
      if (fallback) return { kind: "default", url: convertFileSrc(fallback) };
    }
    return null;
  } catch (e) {
    console.warn("[oa-theme-bg] resolve failed:", e);
    return null;
  }
}

const ThemeBackground: Component<Props> = (props) => {
  const sid = createMemo(() => props.systemId());
  const kind = (): BackgroundKind => props.kind ?? "default";

  const [resolved] = createResource<Resolved | null, { systemId: SystemId; kind: BackgroundKind }>(
    () => {
      const s = sid();
      if (!s) return null as never;
      return { systemId: s, kind: kind() };
    },
    (params) => resolveUrl(params.systemId, params.kind),
  );

  return (
    <Show when={sid() && resolved()?.url}>
      <div class="pointer-events-none absolute inset-0 z-0 overflow-hidden" aria-hidden="true">
        <Show when={resolved()?.kind === "default"}>
          <div
            class="absolute inset-0"
            style={{
              "background-image": `url(${resolved()!.url})`,
              "background-size": "cover",
              "background-position": "center",
              opacity: String(props.opacity ?? 0.5),
            }}
          />
        </Show>
        <Show when={resolved()?.kind === "animated"}>
          <video
            src={resolved()!.url}
            autoplay
            loop
            muted
            playsinline
            class="absolute inset-0 h-full w-full object-cover"
            style={{ opacity: String(props.opacity ?? 0.5) }}
          />
        </Show>
        {/* Dark scrim so foreground cards/text stay legible over any backdrop. */}
        <div
          class="absolute inset-0"
          style={{
            background:
              "linear-gradient(to top, var(--color-oa-bg-deep) 4%, transparent 45%, var(--color-oa-bg-deep)/0 60%, var(--color-oa-bg-deep) 98%)",
          }}
        />
      </div>
    </Show>
  );
};

export default ThemeBackground;
