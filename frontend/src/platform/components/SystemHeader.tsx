import { Show, createResource, type Component } from "solid-js";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import { usePlatformMedia } from "@oa/platform/library/platformMedia";
import { systemSupportsBootless } from "@oa/platform/library/launch";
import SystemCoresStrip from "./SystemCoresStrip";

type Props = {
  systemId: SystemId;
  gameCount: number;
  /// Boot the system's libretro core with no content (DOSBox / ScummVM
  /// built-in browser). Supplied by the theme (LibraryPage → LibraryView)
  /// — platform components are prop-driven and must NOT reach into the
  /// theme context, so the "Boot without game" button only renders when
  /// this handler is wired AND the system supports it. See the layer
  /// boundary contract in docs/features/theming-substrate/SURFACES.md.
  onBootWithoutGame?: (systemId: SystemId) => void;
};

/// System landing band — shown above the GridControls bar when the user
/// has navigated into a system-filtered view (left-clicked a system in
/// the left sidebar). Surfaces the system's identity + a collapsible
/// per-system cores strip (browse + install). All other per-system
/// surfaces (bindings, shaders, default core, etc.) are reachable from
/// the top-bar `System ▾` menu or the sidebar right-click context menu.
///
/// When the operator has set a wheel-art PNG for this system via
/// Library Manager → Platform media (Phase 6 of the media-taxonomy
/// plan), the wheel image renders left of the title in place of the
/// short-name chip. Falls back to the chip when no wheel art is set.
const SystemHeader: Component<Props> = (props) => {
  const theme = () => systemThemes[props.systemId];
  const platformMedia = usePlatformMedia();
  const wheel = () => platformMedia.wheelUrl(props.systemId);
  // Whether to offer "Boot without game" for this system — true only for
  // no-game-capable cores (DOSBox-Pure, ScummVM) not routed to an
  // external emulator. Re-queried whenever the visible system changes.
  const [bootless] = createResource(() => props.systemId, systemSupportsBootless);
  return (
    <header
      class="border-b border-white/5 bg-(--color-oa-bg-deep)/95 px-(--layout-content-padding-x) py-4 backdrop-blur"
      data-system={props.systemId}
    >
      <div class="flex items-center gap-3">
        <Show
          when={wheel()}
          fallback={
            <span class="inline-flex items-center rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 px-2 py-0.5 font-mono text-[0.65rem] uppercase tracking-wider text-(--color-system-accent-soft)">
              {theme()?.shortName ?? props.systemId.toUpperCase()}
            </span>
          }
        >
          <img
            src={wheel()!}
            alt={theme()?.displayName ?? props.systemId}
            class="h-10 max-w-[200px] object-contain object-left"
            // System name still on the page in the h1 below; the wheel
            // is decorative for screen readers (the alt above keeps the
            // attribute valid + meaningful if styles fail).
          />
        </Show>
        <h1 class="truncate text-lg font-semibold text-(--color-oa-ink)">
          {theme()?.displayName ?? props.systemId}
        </h1>
        <Show when={bootless() && props.onBootWithoutGame}>
          <button
            type="button"
            class="ml-auto inline-flex shrink-0 items-center gap-1.5 rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 px-3 py-1.5 text-xs font-medium text-(--color-system-accent-soft) transition hover:bg-(--color-system-accent)/25 focus-visible:outline-none data-[oa-focus=true]:outline data-[oa-focus=true]:outline-2 data-[oa-focus=true]:outline-(--color-system-accent)"
            title="Boot this core with no game — opens its own built-in browser (DOSBox / ScummVM)"
            onClick={() => props.onBootWithoutGame?.(props.systemId)}
          >
            ▶ Boot without game
          </button>
        </Show>
      </div>
      <p class="mt-1 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
        {props.gameCount} {props.gameCount === 1 ? "game" : "games"} in library
      </p>
      <SystemCoresStrip systemId={props.systemId} />
    </header>
  );
};

export default SystemHeader;
