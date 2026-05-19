# atari7800 Decisions Log

Atari-7800-specific integration choices. Project-wide decisions live in
`docs/DECISIONS.md`. Append-only; newest at the bottom.

---

## 2026-05-19 — Register only `.a78` (no headerless `.bin`)

**Decision:** The Atari 7800 system extension list in
`frontend/src/themes/registry.ts` is `["a78"]` only — headerless `.bin`
dumps are intentionally excluded.

**Why:** `.bin` is a generic extension claimed by many retro systems
(Atari 2600, Mega Drive, Sega CD audio tracks, MAME ROM-set internals,
etc.). Registering it for the 7800 would collide with future system
onboarding and force the library scanner to disambiguate via file
content. Modern Atari 7800 dump conventions (No-Intro, TOSEC, current
ROMset standards) all ship `.a78` with the 128-byte header. The rare
headerless dump can be renamed by the user — ProSystem autodetects
mapper from binary content when no header is present, so a renamed
file works without re-dumping.

**Considered and rejected:**
- **Register `.bin` here and resolve collisions when future systems
  land.** Pushes the problem forward. Each new system claiming `.bin`
  would need content-sniffing to route — not worth the future engineering
  to support a deprecated dump format.

---

## 2026-05-19 — Primary fire on libretro B (bit 0), gamepad East

**Decision:** Atari 7800 Button 1 (primary fire) maps to libretro bit 0
(libretro B) and is bound to keyboard Z + gamepad East by default.
Button 2 maps to libretro bit 8 (libretro A) and is bound to keyboard
X + gamepad South.

**Why:** The libretro ProSystem core's input descriptors explicitly
register Button 1 against `RETRO_DEVICE_ID_JOYPAD_B` (bit 0) and
Button 2 against `RETRO_DEVICE_ID_JOYPAD_A` (bit 8). Forcing OA's
bindings to match those descriptors means the per-system Bindings UI's
labels stay honest — "B1" in OA = "Button 1" in MAME's TAB menu = the
button the core fires when bit 0 goes high.

The keyboard side follows the cross-system "Z is primary" rule
established in PCE Phase 1 (locked by the
`z_is_the_primary_action_button_on_every_system` test). The gamepad
side follows the PCE / NES / SNES / Lynx convention that "primary
action = East" — diverges from MAME's "primary on South" but matches
every other console-shape system in OA.

**Considered and rejected:**
- **Swap so Z + East fire Button 2.** Would feel natural to a user
  whose muscle memory comes from emulators that pin the joystick's
  right button to "1" — but every other OA system has Z firing the
  bit-8 button, and consistency across the lineup wins over honoring
  any single system's emulator-side tradition.
