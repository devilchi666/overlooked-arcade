import { Show, type Component, type JSX } from "solid-js";

type Props = {
  /** Left zone — identity + breadcrumb. */
  left?: JSX.Element;
  /** Center zone — search + navigation. Currently a search input slot. */
  center?: JSX.Element;
  /** Right zone — primary action + secondary actions + window controls. */
  right?: JSX.Element;
  /** True while the toolbar should fade out (game-mode idle timer). */
  hidden?: boolean;
};

/**
 * Three-zone top toolbar: identity/breadcrumb on the left, search in the
 * center, action cluster on the right. Each zone is a slot so the caller
 * controls content. The toolbar itself owns:
 *   - Height (--layout-top-toolbar-height via parent Shell grid)
 *   - Background + border
 *   - Idle-fade transition (oa-header-fade class shared with prior shell)
 *   - z-index above the main content
 */
const TopToolbar: Component<Props> = (props) => {
  return (
    <header
      class="oa-header-fade flex h-full items-center gap-4 border-b border-white/5 bg-(--color-oa-bg-deep)/95 px-4 backdrop-blur"
      classList={{ "is-hidden": props.hidden === true }}
    >
      <div class="flex min-w-0 flex-1 items-center gap-2">{props.left}</div>
      <Show when={props.center}>
        <div class="flex min-w-0 flex-1 items-center justify-center">{props.center}</div>
      </Show>
      <div class="flex shrink-0 items-center gap-1.5">{props.right}</div>
    </header>
  );
};

export default TopToolbar;
