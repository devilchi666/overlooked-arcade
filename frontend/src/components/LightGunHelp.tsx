import type { Component } from "solid-js";
import { For } from "solid-js";

/// Systems that use the libretro RETRO_DEVICE_LIGHTGUN device type
/// (NES Zapper, SNES Super Scope, SMS Phaser, Saturn Virtua Gun, PSX
/// GunCon/Justifier, Dreamcast Light Gun, Atari 7800 XEGS Light Gun).
/// NDS uses RETRO_DEVICE_POINTER for its stylus — different dispatch
/// path, so it's NOT in this list even though
/// `light_gun_systems.rs::LIGHT_GUN_SYSTEMS` includes it for the
/// shared-pointer-path reason.
///
/// Mirror of the Rust `apps/oa-shell/src/light_gun_systems.rs::LIGHT_GUN_SYSTEMS`
/// table minus the NDS entry. When adding a new light-gun system,
/// update both.
export const LIGHT_GUN_SYSTEM_IDS = [
  "nes",
  "snes",
  "sms",
  "saturn",
  "psx",
  "dreamcast",
  "atari7800",
] as const;

/// The fixed JOYPAD-bit → LIGHTGUN-id mapping the Rust
/// `oa_input::lightgun_buttons_from_joypad_bits` performs at every
/// input-poll tick when a per-game device is set to LIGHTGUN. Operators
/// don't bind LIGHTGUN ids directly — they bind the listed JOYPAD bit
/// via the standard per-system bindings UI, and the Rust layer
/// reinterprets the bits automatically.
///
/// Mirror of the docstring table in
/// `crates/oa-input/src/lib.rs::lightgun_buttons_from_joypad_bits`.
/// When that table changes, mirror the change here.
const LIGHT_GUN_MAPPING: readonly { joypad: string; lightgun: string; why: string }[] = [
  { joypad: "Y", lightgun: "AUX_A", why: "First gun-side face button" },
  { joypad: "A", lightgun: "AUX_B", why: "Second gun-side face button" },
  { joypad: "X", lightgun: "AUX_C", why: "Third gun-side face button (Justifier-style)" },
  { joypad: "START", lightgun: "START", why: "Pause / continue prompts" },
  { joypad: "SELECT", lightgun: "SELECT", why: "Menu screens" },
  { joypad: "DPad", lightgun: "DPAD", why: "Menu nav (Hogan's Alley, Wild Gunman)" },
  { joypad: "R-shoulder", lightgun: "RELOAD", why: "Button-reload (Time Crisis pedal-equivalent)" },
];

/// Detailed mapping table + intro copy. Used by `GameDialogs.tsx`
/// (when a LIGHTGUN device is selected per-game) and by
/// `SystemBindingsEditor.tsx` (as an informational banner for systems
/// where a light-gun is on the per-game menu).
///
/// `compact` mode renders just the intro + a single-line mapping
/// summary suitable for the bindings editor's tight space; the default
/// mode renders the full mapping table for the per-game Input dialog.
export const LightGunMappingHelp: Component<{ compact?: boolean }> = (props) => {
  return (
    <div class="rounded border border-(--color-system-accent)/30 bg-(--color-system-accent)/5 px-2.5 py-2 text-[0.65rem] leading-relaxed text-(--color-oa-ink-dim)">
      <div class="text-(--color-oa-ink)">
        Light-gun mapping (fixed)
      </div>
      <p class="mt-1">
        Mouse position drives aim; left-click fires{" "}
        <span class="font-mono text-(--color-system-accent-soft)">TRIGGER</span>{" "}
        directly — no binding needed. The standard RetroPad bits get
        reinterpreted as gun-side buttons automatically:
      </p>
      {props.compact ? (
        <p class="mt-1 font-mono text-[0.6rem]">
          Y→AUX_A · A→AUX_B · X→AUX_C · START→START · SELECT→SELECT ·
          DPad→DPad · R-shoulder→RELOAD
        </p>
      ) : (
        <table class="mt-1.5 w-full text-[0.6rem]">
          <thead>
            <tr class="text-(--color-oa-ink-dim)">
              <th class="py-0.5 pr-3 text-left font-medium">RetroPad bit</th>
              <th class="py-0.5 pr-3 text-left font-medium">Gun-side button</th>
              <th class="py-0.5 text-left font-medium">Why</th>
            </tr>
          </thead>
          <tbody>
            <For each={LIGHT_GUN_MAPPING}>
              {(m) => (
                <tr>
                  <td class="py-0.5 pr-3 font-mono text-(--color-system-accent-soft)">
                    {m.joypad}
                  </td>
                  <td class="py-0.5 pr-3 font-mono text-(--color-system-accent-soft)">
                    {m.lightgun}
                  </td>
                  <td class="py-0.5 text-(--color-oa-ink-dim)">{m.why}</td>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      )}
      <p class="mt-1.5">
        Time Crisis–style off-screen aim-to-reload also fires{" "}
        <span class="font-mono text-(--color-system-accent-soft)">RELOAD</span>{" "}
        directly via the IS_OFFSCREEN flag, so both paths work.
      </p>
    </div>
  );
};

/// Type-narrowing helper for use sites that key off a SystemId-like
/// string. Returns true when the given slug supports light-gun games
/// via RETRO_DEVICE_LIGHTGUN.
export function isLightGunSystem(systemId: string | null | undefined): boolean {
  if (!systemId) return false;
  return (LIGHT_GUN_SYSTEM_IDS as readonly string[]).includes(systemId);
}
