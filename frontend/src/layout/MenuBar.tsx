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
  createEffect,
  createMemo,
  createSignal,
  onCleanup,
  onMount,
  useContext,
  type Accessor,
  type Component,
  type JSX,
} from "solid-js";
import { activateFocusGroup, useFocusGroup } from "../nav/focus";
import { useBackHandler } from "../nav/back";
import { HintRegion } from "../nav/HintBar";

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
  /// Mount-order registration so L1/R1 can cycle to neighbouring menus
  /// when the operator is navigating with a gamepad. Each <Menu> appends
  /// its id on mount and removes it on cleanup.
  registerMenu: (id: string) => () => void;
  menuIds: Accessor<string[]>;
};

const MenuBarContext = createContext<MenuBarContextValue>();

/// Global signal used by the top-level Start handler to request the
/// menu bar open its first menu. Exposed as an exported function so the
/// App can drive it from its gamepad-event subscription without
/// reaching into MenuBar internals.
const [menuBarOpenRequest, setMenuBarOpenRequest] = createSignal(0);
export function requestOpenFirstMenu(): void {
  setMenuBarOpenRequest((n) => n + 1);
}

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
  /// Menu registration in mount order. The first <Menu> to mount appears
  /// at index 0, and so on. Used by L1/R1 cycling + by the Start-button
  /// handler to find a default open target.
  const [menuIds, setMenuIds] = createSignal<string[]>([]);
  function registerMenu(id: string): () => void {
    setMenuIds((cur) => [...cur, id]);
    return () => setMenuIds((cur) => cur.filter((x) => x !== id));
  }

  // Open the first registered menu when the App's Start handler signals
  // requestOpenFirstMenu(). Re-triggers each time the signal increments
  // (uses a count rather than a flag so the same "press Start again"
  // re-opens after a close).
  createEffect(() => {
    const tick = menuBarOpenRequest();
    if (tick === 0) return;
    const ids = menuIds();
    if (ids.length === 0) return;
    setOpenId(ids[0]);
  });

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
    <MenuBarContext.Provider value={{ openId, setOpenId, hovering, setHovering, registerMenu, menuIds }}>
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

  // Register this menu in the bar's mount-order list. Cleanup on dispose.
  onMount(() => {
    const unreg = ctx.registerMenu(id);
    onCleanup(unreg);
  });

  // Controller-nav: when open, the menu's popover items become DPad-
  // navigable. Buttons are discovered via DOM query (the popover's role
  // selectors below). The focus group is registered unconditionally so
  // its activate() in the open-effect always works; itemCount returns 0
  // when closed so nav events are ignored.
  let popoverRef: HTMLDivElement | undefined;
  const [focusedIndex, setFocusedIndex] = createSignal(0);
  const menuItemSelector =
    'button[role="menuitem"], button[role="menuitemradio"], button[role="menuitemcheckbox"]';
  const queryButtons = (): HTMLButtonElement[] => {
    if (!popoverRef) return [];
    const all = Array.from(popoverRef.querySelectorAll<HTMLButtonElement>(menuItemSelector));
    // Skip disabled rows. Browsers refuse to focus disabled buttons, and
    // clicks on them no-op, so navigating to one would be a dead end.
    // Filtering here keeps itemCount + binds + the focus-ring mirror in
    // lockstep on the enabled set.
    return all.filter((btn) => !btn.disabled);
  };
  const [itemCount, setItemCount] = createSignal(0);
  // Bumped whenever the popover's DOM mutates (disabled-attr flip, Show
  // toggle, etc) so the focus-ring mirror knows to re-paint.
  const [domRev, setDomRev] = createSignal(0);
  let observer: MutationObserver | null = null;
  const menuFocus = useFocusGroup({
    id: `menubar-menu-${id}`,
    orientation: "vertical",
    itemCount,
    focusedIndex,
    setFocusedIndex,
    onActivate: (i) => {
      const btn = queryButtons()[i];
      btn?.click();
    },
    onCancel: () => ctx.setOpenId(null),
    onShoulderL: () => cycleMenu(-1),
    onShoulderR: () => cycleMenu(1),
  });
  function cycleMenu(delta: -1 | 1): void {
    const ids = ctx!.menuIds();
    const cur = ids.indexOf(id);
    if (cur < 0 || ids.length < 2) return;
    const next = (cur + delta + ids.length) % ids.length;
    ctx!.setOpenId(ids[next]);
  }

  // When the menu opens, activate the focus group, bind every visible
  // enabled button by DOM order, and reset the focused index. While
  // open, observe the popover for content changes (disabled-attr flips
  // from background work, Show toggles) and re-bind so the focus group
  // stays in lockstep with what's actually navigable. When it closes,
  // disconnect the observer; the popover unmounts so per-button cleanup
  // is implicit.
  createEffect(() => {
    if (!isOpen()) {
      setItemCount(0);
      observer?.disconnect();
      observer = null;
      return;
    }
    queueMicrotask(() => {
      // Guard against open-then-close-before-microtask races — if the
      // menu closed while we were queued, skip setup entirely so we
      // don't attach an observer to a detached popover node.
      if (!isOpen() || !popoverRef || !popoverRef.isConnected) return;
      const rebind = () => {
        if (!popoverRef || !popoverRef.isConnected) return;
        const btns = queryButtons();
        setItemCount(btns.length);
        btns.forEach((btn, i) => menuFocus.bind(i, btn));
        setDomRev((r) => r + 1);
      };
      rebind();
      setFocusedIndex(0);
      menuFocus.activate();
      observer = new MutationObserver(rebind);
      observer.observe(popoverRef, {
        childList: true,
        subtree: true,
        attributes: true,
        attributeFilter: ["disabled"],
      });
    });
  });
  onCleanup(() => {
    observer?.disconnect();
    observer = null;
  });

  // Visual focus ring: mirror focusedIndex into data-oa-focus on the
  // bound buttons so the operator sees up/down moving. focus.ts also
  // calls el.focus() but Tailwind's preflight removes the browser's
  // default outline — OA's pattern is data-attribute-driven so the
  // per-system accent + animation budget apply (in index.css
  // [data-oa-focus="true"]). Re-runs on focusedIndex changes (cursor
  // moves) and on domRev bumps (content changed mid-open) so the ring
  // always lands on whichever button now holds the index.
  createEffect(() => {
    if (!isOpen()) return;
    const idx = focusedIndex();
    const active = menuFocus.isActive();
    void domRev();
    queueMicrotask(() => {
      const btns = queryButtons();
      const targetIdx = btns.length === 0 ? -1 : Math.min(idx, btns.length - 1);
      btns.forEach((btn, i) => {
        if (i === targetIdx) {
          btn.setAttribute("data-oa-focus", "true");
          btn.setAttribute("data-oa-focus-active", active ? "true" : "false");
        } else {
          btn.removeAttribute("data-oa-focus");
          btn.removeAttribute("data-oa-focus-active");
        }
      });
    });
  });
  // Back-stack registration is conditional on isOpen; mount the inner
  // <MenuBackHandler> via Show so the handler auto-pops on close.

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
        <MenuOpenScope onClose={() => ctx.setOpenId(null)} />
        <HintRegion hints={{ a: "Activate", b: "Close", l1: "Prev menu", r1: "Next menu" }} />
        <div
          ref={popoverRef}
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

/// Tiny helper component — registers the menu's onClose with the back
/// stack for the lifetime of the open popover. Also transfers active
/// focus group back to library-grid on close so a B press doesn't strand
/// the operator in an emptied menu group.
const MenuOpenScope: Component<{ onClose: () => void }> = (props) => {
  useBackHandler(() => props.onClose());
  onCleanup(() => activateFocusGroup("library-grid"));
  return null;
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
