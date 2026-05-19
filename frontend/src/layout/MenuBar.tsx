// Menu bar primitive — top-of-shell, named menus opening popovers.
//
// Modeled on a desktop menu bar (LaunchBox / classic Windows) but styled to
// match the rest of the shell (no pipes, no chrome, stylized text). The bar
// is the top-level navigation surface; each <Menu> opens a dropdown of
// <MenuItem> rows on click. Hovering an adjacent menu while one is already
// open switches to it (the standard desktop affordance). Click outside or
// Esc closes the open menu.
//
// Step 2 (current) ships the shell only: primitives + placeholder labels.
// Real menu contents land in subsequent steps as we delete the SettingsPage
// route etc. and rehome each field per docs/UI_MENU_BAR_PLAN.md.

import {
  Show,
  createContext,
  createMemo,
  createSignal,
  onCleanup,
  onMount,
  useContext,
  type Component,
  type JSX,
} from "solid-js";

// --- Context ------------------------------------------------------------
//
// The MenuBar owns "which menu is currently open" (or null). Children call
// `openMenu(id)` / `closeMenu()` to drive it. A simple stable counter gives
// each <Menu> a unique id without callers needing to think about it.

type MenuBarContextValue = {
  openId: () => string | null;
  setOpenId: (id: string | null) => void;
  hovering: () => boolean;
  setHovering: (v: boolean) => void;
};

const MenuBarContext = createContext<MenuBarContextValue>();

let menuIdCounter = 0;
function nextMenuId(): string {
  menuIdCounter += 1;
  return `oa-menu-${menuIdCounter}`;
}

// --- MenuBar ------------------------------------------------------------

type MenuBarProps = {
  children: JSX.Element;
  /// Accessible label for the bar's role. Defaults to "Main menu".
  ariaLabel?: string;
};

/**
 * Top-level menu bar. Renders children in a horizontal flex row, manages
 * the "which menu is open" signal, and handles outside-click + Esc to
 * close. Children are expected to be <Menu> elements.
 */
export const MenuBar: Component<MenuBarProps> = (props) => {
  const [openId, setOpenId] = createSignal<string | null>(null);
  /// Whether the user's mouse is currently hovering ANY menu's button or
  /// popover. Used so that opening one menu by click lets adjacent menus
  /// take over on hover, but a mouseleave from the bar doesn't immediately
  /// close (gives a small intent window before auto-close).
  const [hovering, setHovering] = createSignal(false);

  // Outside-click and Esc handlers. Capture-phase Esc so we win against
  // page-level handlers that may also listen.
  onMount(() => {
    const onClick = (e: MouseEvent) => {
      if (!openId()) return;
      const target = e.target as HTMLElement | null;
      // The menu's own buttons stop propagation so this only fires for
      // clicks genuinely outside the bar.
      if (!target?.closest("[data-oa-menu-root]")) {
        setOpenId(null);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && openId() !== null) {
        e.stopPropagation();
        setOpenId(null);
      }
    };
    window.addEventListener("click", onClick);
    window.addEventListener("keydown", onKey, { capture: true });
    onCleanup(() => {
      window.removeEventListener("click", onClick);
      window.removeEventListener("keydown", onKey, { capture: true });
    });
  });

  return (
    <MenuBarContext.Provider value={{ openId, setOpenId, hovering, setHovering }}>
      <nav
        role="menubar"
        aria-label={props.ariaLabel ?? "Main menu"}
        class="flex items-center gap-1"
        onMouseEnter={() => setHovering(true)}
        onMouseLeave={() => setHovering(false)}
      >
        {props.children}
      </nav>
    </MenuBarContext.Provider>
  );
};

// --- Menu ---------------------------------------------------------------

type MenuProps = {
  /// Visible label in the bar (e.g. "Library", "View").
  label: string;
  /// Disabled state (e.g. "System" / "Game" when nothing is selected).
  /// The button stays visible but dimmed and unclickable; tooltip explains.
  disabled?: boolean;
  /// Tooltip shown on the disabled button. Defaults to "Unavailable".
  disabledHint?: string;
  /// Render the caret (▾) suffix. Default true — every menu has children.
  /// Set false for menus that are actually just buttons (none today).
  showCaret?: boolean;
  /// Menu contents — <MenuItem>, <MenuLabel>, <MenuDivider>, etc.
  children: JSX.Element;
};

/**
 * One named menu in the bar. Renders the labeled trigger button + a
 * dropdown popover below it when open. Hover-while-another-is-open swaps
 * to this menu (desktop menu-bar affordance).
 */
export const Menu: Component<MenuProps> = (props) => {
  const ctx = useContext(MenuBarContext);
  if (!ctx) throw new Error("<Menu> must be nested inside <MenuBar>");
  const id = nextMenuId();
  const isOpen = createMemo(() => ctx.openId() === id);
  const showCaret = () => props.showCaret !== false;

  const toggle = () => {
    if (props.disabled) return;
    ctx.setOpenId(isOpen() ? null : id);
  };

  const onHover = () => {
    // If another menu is already open, swap to this one on hover.
    if (props.disabled) return;
    const cur = ctx.openId();
    if (cur !== null && cur !== id) ctx.setOpenId(id);
  };

  return (
    <div class="relative" data-oa-menu-root>
      <button
        type="button"
        role="menuitem"
        aria-haspopup="menu"
        aria-expanded={isOpen()}
        aria-disabled={props.disabled === true}
        disabled={props.disabled === true}
        title={props.disabled ? (props.disabledHint ?? "Unavailable") : undefined}
        onClick={(e) => {
          e.stopPropagation();
          e.currentTarget.blur();
          toggle();
        }}
        onMouseEnter={onHover}
        class="flex items-center gap-1 rounded-md px-2.5 py-1 text-[0.75rem] font-medium transition"
        classList={{
          "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)":
            !isOpen() && !props.disabled,
          "bg-white/[0.06] text-(--color-system-accent)": isOpen(),
          "opacity-40 cursor-default": props.disabled === true,
        }}
      >
        <span>{props.label}</span>
        <Show when={showCaret()}>
          <span
            aria-hidden="true"
            class="text-[0.6rem] opacity-70"
            classList={{ "opacity-100": isOpen() }}
          >
            ▾
          </span>
        </Show>
      </button>

      <Show when={isOpen()}>
        <div
          role="menu"
          aria-label={props.label}
          // Popover. Positioned just below the button, left-aligned. Width
          // sized to content with a sensible minimum so single-item menus
          // don't render as a stamp. Max width keeps long item labels
          // from making the popover comically wide.
          class="absolute left-0 top-full z-50 mt-1 min-w-[12rem] max-w-[20rem] overflow-hidden rounded-md border border-white/10 bg-(--color-oa-bg-deep)/95 py-1 shadow-2xl shadow-black/60 backdrop-blur"
          onClick={(e) => e.stopPropagation()}
        >
          {props.children}
        </div>
      </Show>
    </div>
  );
};

// --- MenuItem -----------------------------------------------------------

type MenuItemProps = {
  /// Display text.
  label: string;
  /// Optional right-aligned hint (e.g. shortcut like "Ctrl+W").
  hint?: string;
  /// Click handler. Closes the menu automatically before firing.
  onClick?: () => void;
  /// Greyed-out + unclickable.
  disabled?: boolean;
  /// Destructive style (red-tinged hover).
  destructive?: boolean;
};

/**
 * Single clickable row inside a menu. Closes the menu and then fires
 * `onClick` so caller logic can mutate state without worrying about the
 * popover still being open during the transition.
 */
export const MenuItem: Component<MenuItemProps> = (props) => {
  const ctx = useContext(MenuBarContext);
  return (
    <button
      type="button"
      role="menuitem"
      disabled={props.disabled === true}
      onClick={(e) => {
        e.stopPropagation();
        e.currentTarget.blur();
        if (props.disabled) return;
        ctx?.setOpenId(null);
        props.onClick?.();
      }}
      class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs text-(--color-oa-ink) transition disabled:cursor-not-allowed disabled:opacity-50"
      classList={{
        "hover:bg-white/[0.06]": !props.destructive && !props.disabled,
        "text-red-300 hover:bg-red-500/10 hover:text-red-200":
          props.destructive === true && !props.disabled,
      }}
    >
      <span class="flex-1 truncate">{props.label}</span>
      <Show when={props.hint}>
        <span class="rounded border border-white/10 bg-white/[0.04] px-1.5 py-0.5 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          {props.hint}
        </span>
      </Show>
    </button>
  );
};

// --- MenuRadio ----------------------------------------------------------

type RadioOption<T extends string> = {
  value: T;
  label: string;
  /// Optional right-aligned hint (e.g. shortcut).
  hint?: string;
};

type MenuRadioProps<T extends string> = {
  /// Optional section header rendered above the options.
  label?: string;
  /// Currently-selected value.
  value: T;
  /// Called with the new value when the user picks one. Menu auto-closes.
  onChange: (next: T) => void;
  /// Options to display. Order = display order.
  options: readonly RadioOption<T>[];
};

/**
 * One-of-N radio group inside a menu. Renders an optional section label
 * followed by one row per option. The active row has a leading `●` glyph;
 * others get a `·` placeholder so labels align.
 */
export function MenuRadio<T extends string>(props: MenuRadioProps<T>): JSX.Element {
  const ctx = useContext(MenuBarContext);
  return (
    <>
      <Show when={props.label}>
        <MenuLabel>{props.label}</MenuLabel>
      </Show>
      {props.options.map((opt) => {
        const active = () => props.value === opt.value;
        return (
          <button
            type="button"
            role="menuitemradio"
            aria-checked={active()}
            onClick={(e) => {
              e.stopPropagation();
              e.currentTarget.blur();
              ctx?.setOpenId(null);
              if (!active()) props.onChange(opt.value);
            }}
            class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs text-(--color-oa-ink) transition hover:bg-white/[0.06]"
          >
            <span
              class="w-3 text-center text-[0.65rem]"
              classList={{
                "text-(--color-system-accent)": active(),
                "text-(--color-oa-ink-dim)/40": !active(),
              }}
              aria-hidden="true"
            >
              {active() ? "●" : "·"}
            </span>
            <span class="flex-1 truncate">{opt.label}</span>
            <Show when={opt.hint}>
              <span class="rounded border border-white/10 bg-white/[0.04] px-1.5 py-0.5 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                {opt.hint}
              </span>
            </Show>
          </button>
        );
      })}
    </>
  );
}

// --- MenuCheckbox -------------------------------------------------------

type MenuCheckboxProps = {
  label: string;
  hint?: string;
  checked: boolean;
  onChange: (next: boolean) => void;
  disabled?: boolean;
};

/**
 * Toggleable row. Leading `✓` glyph when checked; blank otherwise.
 * Stays open while clicking? No — closes the menu like every other row,
 * because users typically toggle one thing and move on.
 */
export const MenuCheckbox: Component<MenuCheckboxProps> = (props) => {
  const ctx = useContext(MenuBarContext);
  return (
    <button
      type="button"
      role="menuitemcheckbox"
      aria-checked={props.checked}
      disabled={props.disabled === true}
      onClick={(e) => {
        e.stopPropagation();
        e.currentTarget.blur();
        if (props.disabled) return;
        ctx?.setOpenId(null);
        props.onChange(!props.checked);
      }}
      class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs text-(--color-oa-ink) transition hover:bg-white/[0.06] disabled:cursor-not-allowed disabled:opacity-50"
    >
      <span
        class="w-3 text-center text-[0.65rem]"
        classList={{
          "text-(--color-system-accent)": props.checked,
          "text-(--color-oa-ink-dim)/40": !props.checked,
        }}
        aria-hidden="true"
      >
        {props.checked ? "✓" : ""}
      </span>
      <span class="flex-1 truncate">{props.label}</span>
      <Show when={props.hint}>
        <span class="rounded border border-white/10 bg-white/[0.04] px-1.5 py-0.5 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          {props.hint}
        </span>
      </Show>
    </button>
  );
};

// --- MenuLabel ----------------------------------------------------------

type MenuLabelProps = {
  children: JSX.Element;
};

/**
 * Non-interactive label row. Used at the top of contextual menus (System,
 * Game) to display the active system / game title.
 */
export const MenuLabel: Component<MenuLabelProps> = (props) => (
  <div class="px-3 pt-1.5 pb-1 text-[0.55rem] font-semibold uppercase tracking-[0.3em] text-(--color-oa-ink-dim)">
    {props.children}
  </div>
);

// --- MenuDivider --------------------------------------------------------

/// Thin separator between menu sections.
export const MenuDivider: Component = () => (
  <div class="my-1 border-t border-white/5" role="separator" />
);
