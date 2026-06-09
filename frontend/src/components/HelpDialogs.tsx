// Help menu dialogs — Keyboard shortcuts cheatsheet + About modal.
//
// These were orphans before the menu-bar redesign: every Fn key was
// invisible to a new user and there was nowhere to learn them. Now they
// have an obvious home under Help ▾.

import { For, type Component } from "solid-js";
import { Dialog } from "@oa/platform/components/Dialog";

type Shortcut = {
  keys: string;
  /// Where the shortcut works — "anywhere", "in-game", or "library".
  scope: "anywhere" | "in-game" | "library";
  action: string;
};

const SHORTCUTS: readonly Shortcut[] = [
  { keys: "Click",        scope: "library", action: "Select game (no launch)" },
  { keys: "Double-click", scope: "library", action: "Launch selected game" },
  { keys: "Right-click",  scope: "library", action: "Open tile context menu" },
  { keys: "Esc", scope: "in-game", action: "Pause / open Quick Settings" },
  { keys: "Esc", scope: "anywhere", action: "Close menus / dialogs" },
  { keys: "Ctrl+W", scope: "in-game", action: "Exit to library" },
  { keys: "Ctrl+B", scope: "anywhere", action: "Toggle left sidebar" },
  { keys: "Ctrl+G*", scope: "anywhere", action: "Toggle Game focus (keyboard: shell ↔ core) — *rebindable in Settings › Controls" },
  { keys: "F1", scope: "in-game", action: "Reset" },
  { keys: "F2", scope: "in-game", action: "Pause / resume" },
  { keys: "F3", scope: "in-game", action: "Frame advance (while paused)" },
  { keys: "F5", scope: "in-game", action: "Save state to current slot" },
  { keys: "F8", scope: "in-game", action: "Load state from current slot" },
  { keys: "F6", scope: "in-game", action: "Hold for fast-forward" },
  { keys: "F7", scope: "in-game", action: "Hold for slow-motion" },
  { keys: "F12", scope: "in-game", action: "Screenshot" },
  { keys: "Backspace", scope: "in-game", action: "Hold to rewind" },
  { keys: "0 – 9", scope: "in-game", action: "Select save state slot" },
];

export const KeyboardShortcutsDialog: Component<{
  open: boolean;
  onClose: () => void;
}> = (props) => (
  <Dialog open={props.open} onClose={props.onClose} title="Keyboard shortcuts" size="md">
    <div class="overflow-hidden rounded-md border border-white/5">
      <table class="w-full text-xs">
        <thead class="bg-white/[0.03] text-(--color-oa-ink-dim)">
          <tr>
            <th class="px-3 py-2 text-left font-medium uppercase tracking-widest text-[0.6rem]">Key</th>
            <th class="px-3 py-2 text-left font-medium uppercase tracking-widest text-[0.6rem]">Scope</th>
            <th class="px-3 py-2 text-left font-medium uppercase tracking-widest text-[0.6rem]">Action</th>
          </tr>
        </thead>
        <tbody>
          <For each={SHORTCUTS}>
            {(s) => (
              <tr class="border-t border-white/5">
                <td class="px-3 py-1.5">
                  <kbd class="rounded border border-white/15 bg-white/[0.04] px-1.5 py-0.5 font-mono text-[0.7rem] text-(--color-oa-ink)">
                    {s.keys}
                  </kbd>
                </td>
                <td class="px-3 py-1.5 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                  {s.scope}
                </td>
                <td class="px-3 py-1.5 text-(--color-oa-ink)">{s.action}</td>
              </tr>
            )}
          </For>
        </tbody>
      </table>
    </div>
    <p class="mt-3 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
      Bindings inside game (D-pad, A/B, Start/Select) are per-system — edit them under{" "}
      <span class="text-(--color-oa-ink)">System ▾ → Bindings…</span>
    </p>
  </Dialog>
);

export const AboutDialog: Component<{
  open: boolean;
  onClose: () => void;
}> = (props) => (
  <Dialog open={props.open} onClose={props.onClose} title="Overlooked Arcade" size="sm">
    <div class="space-y-3 text-sm text-(--color-oa-ink)">
      <p class="text-[0.65rem] uppercase tracking-[0.3em] text-(--color-system-accent)">
        Premium emulator frontend
      </p>
      <p class="text-(--color-oa-ink-dim)">
        A dedicated home for the consoles modern emulators forgot —
        TurboGrafx-16, Lynx, Atari 7800, SMS / Game Gear, MSX, ColecoVision,
        Vectrex, Virtual Boy, WonderSwan, and friends.
      </p>
      <p class="text-(--color-oa-ink-dim)">
        Non-commercial. A gift to the retro community. Built on forked C
        cores (Beetle PCE Fast, Mednafen, MAME modules) loaded as libretro
        .dlls. Shell is GPL-2.0; cores keep their upstream licenses.
      </p>
    </div>
  </Dialog>
);
