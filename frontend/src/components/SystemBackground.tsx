// Per-system background layer.
//
// Renders behind the library content based on `SystemUIConfig.background`
// for the currently-focused (or pinned) system. Three rendering paths:
//
//   - `static` (default): a soft radial gradient using
//     `--color-system-accent-soft`; if an operator has dropped
//     `<exe_dir>/assets/system-ui/<systemId>/backgrounds/default.<ext>`,
//     the image overlays the gradient.
//   - `animated`: looping `<video autoplay muted>` with the asset
//     under `<systemId>/backgrounds/animated.<ext>`; falls back to the
//     gradient when no file exists.
//   - `shader`: Slice 3 falls back to `static` rendering. Slice 8
//     (Vectrex pilot) ships the shader-driven render path via the
//     custom-component escape hatch in `systemUIConfigs.ts`.
//
// Honors the `perSystemUiEnabled` master toggle — when off, the
// component renders nothing and the base body background shows
// through (uniform plain library mode).

import {
  Show,
  createMemo,
  createResource,
  type Component,
} from "solid-js";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import type { SystemId } from "../themes/registry";
import { uiConfigFor } from "../themes/systemUIConfigs";
import { isPerSystemUiEnabled } from "../themes/systemUiSound";

type Props = {
  /// Reactive accessor for the system whose background should render.
  /// `null` means no system is in focus — the background unmounts and
  /// the base body background shows through. Typically driven by the
  /// focused-or-pinned library tile from App.tsx.
  systemId: () => SystemId | null;
};

type BackgroundKind = "default" | "animated";

type ResolvedBackground = {
  /// Resolved kind. Matches the requested kind unless the requested
  /// kind was `animated` and no `animated.{webm,mp4}` existed — then
  /// `default` so the renderer uses the static image path.
  kind: BackgroundKind;
  url: string;
};

async function resolveBackgroundUrl(
  systemId: SystemId,
  kind: BackgroundKind,
): Promise<ResolvedBackground | null> {
  try {
    const primary = await invoke<string | null>("resolve_background_asset", {
      systemId,
      kind,
    });
    if (primary) return { kind, url: convertFileSrc(primary) };
    // Forgiveness fallback: if the system is configured for `animated`
    // but no animated asset exists, walk the static cascade so a
    // dropped `default.{png,jpg,jpeg,webp}` still renders. Per-system
    // experiences feel less broken when any single asset drop "just
    // works" for casual testing; pilot slices (6-8) will populate the
    // configured kinds properly.
    if (kind === "animated") {
      const fallback = await invoke<string | null>("resolve_background_asset", {
        systemId,
        kind: "default",
      });
      if (fallback) return { kind: "default", url: convertFileSrc(fallback) };
    }
    return null;
  } catch (e) {
    console.warn("[oa-bg] resolve failed:", e);
    return null;
  }
}

const SystemBackground: Component<Props> = (props) => {
  const sid = createMemo(() => props.systemId());

  // For "shader" config Slice 3 falls back to "default" rendering until
  // Slice 8 ships the shader path. "static" → "default" too. Only
  // "animated" routes to the video branch.
  const renderKind = createMemo<BackgroundKind>(() => {
    const s = sid();
    if (!s) return "default";
    return uiConfigFor(s).background === "animated" ? "animated" : "default";
  });

  // Source signal for the resource: a tuple of (systemId, kind) that
  // changes whenever the active system or its config.background flips.
  // Returning falsy from the source skips the fetcher (no asset lookup
  // while the master toggle is off or no system is focused).
  const [resolved] = createResource<
    ResolvedBackground | null,
    { systemId: SystemId; kind: BackgroundKind }
  >(
    () => {
      const s = sid();
      if (!s) return null as never;
      if (!isPerSystemUiEnabled()) return null as never;
      return { systemId: s, kind: renderKind() };
    },
    (params) => resolveBackgroundUrl(params.systemId, params.kind),
  );

  return (
    <Show when={isPerSystemUiEnabled() && sid()}>
      <div
        class="pointer-events-none absolute inset-0 z-0 overflow-hidden"
        data-system={sid() ?? undefined}
        aria-hidden="true"
      >
        {/* Accent-gradient base layer — always renders so a system
            without a bundled asset still feels themed via its
            systemThemes accent. The image / video overlays on top
            when present. */}
        <div
          class="absolute inset-0"
          style={{
            background:
              "radial-gradient(ellipse at center, var(--color-system-accent-soft, transparent) 0%, transparent 70%)",
            opacity: 0.5,
          }}
        />
        <Show when={resolved()?.kind === "default" && resolved()?.url}>
          {(url) => (
            <div
              class="absolute inset-0"
              style={{
                "background-image": `url(${url()})`,
                "background-size": "cover",
                "background-position": "center",
                opacity: 0.85,
              }}
            />
          )}
        </Show>
        <Show when={resolved()?.kind === "animated" && resolved()?.url}>
          {(url) => (
            <video
              src={url()}
              autoplay
              loop
              muted
              // `playsinline` keeps the WebView from going fullscreen
              // on mobile-shaped browsers — harmless on desktop.
              playsinline
              class="absolute inset-0 h-full w-full object-cover"
              style={{ opacity: 0.85 }}
            />
          )}
        </Show>
      </div>
    </Show>
  );
};

export default SystemBackground;
