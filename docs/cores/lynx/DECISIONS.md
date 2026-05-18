# lynx — Decisions

Per-core integration choices and the *why* behind them. Project-wide architectural decisions live in `docs/DECISIONS.md`.

---

## 2026-05-18 — Bit layout matches libretro directly

The `lynx::*` button-bit constants in `apps/oa-shell/src/bindings.rs` are laid out to match `RETRO_DEVICE_ID_JOYPAD_*` positions directly (B=1<<0, SELECT=1<<2, START=1<<3, d-pad 4-7, A=1<<8, L=1<<10). The PCE constants instead use a separate shell-internal layout that `pce_to_libretro_bits` remaps at every `set_input` call.

**Why:** PCE's internal bit layout originated from the pre-libretro static-crate era when the wrapped C core expected `oa_pce_*` constants directly; the remap was a translation layer for the libretro port. Lynx came online after the libretro pivot, so there's no historical layout to honor — picking libretro-aligned bits gives an identity remap (`lynx_to_libretro_bits` is a masked pass-through), which is cheaper and easier to reason about. The downside is the `lynx::SELECT`/`START` names are libretro-oriented rather than Lynx-native ("OPT1" / "OPT2"); the `LYNX_BUTTONS` table preserves the Lynx-native names for the bindings UI, so this is a naming-internal-only quirk.

**Lesson for the next core:** if a new system's native button positions don't clash with libretro's standard joypad layout, lay them out to match — saves a function call per port per frame and removes one class of "buttons go to the wrong place" debugging.

---

## 2026-05-18 — Pause → libretro L

The Lynx had a dedicated Pause button distinct from Option 1/2. RetroArch's convention maps it to libretro L (the left shoulder); we mirror that. The default keyboard binding is `Space` (the canonical pause key on PC gaming), default gamepad is `LeftTrigger`.

**Alternative considered:** mapping Pause to libretro START would collide with Option 1's natural START binding. Mapping to a button the user never uses (libretro R2 / R3) would be unreachable on small Bluetooth gamepads. L is the RetroArch standard and most controllers expose it on a clearly-marked surface (LB / L1 / Z).

---

## 2026-05-18 — No vendored Lynx core in-tree

Unlike tg16 (which vendored Beetle PCE Fast under `crates/oa-pce-sys/vendor/` from before the libretro pivot), Lynx ships with no in-tree vendoring. The operator drops the upstream `mednafen_lynx_libretro.dll` from buildbot.libretro.com into `<exe_dir>/cores/`. This is the modern-OA shape (project `DECISIONS.md` 2026-05-16 entry) — adding a system is registry+CSS+docs, not a Cargo workspace change.

**If we ever need to fork the Lynx core,** the recipe is the same as the planned modified Beetle PCE Fast: maintain a separate libretro-frontend build of the patched source (CMake or Makefile, not part of `cargo build`), emit a `mednafen_lynx_oa_libretro.dll`, ship it in the OA installer's `cores/` folder. The operator sees a different filename in the cores picker; everything else is identical.

---

## 2026-05-18 — Tile aspect 4/3, not framebuffer's 160:102

The Lynx framebuffer is 160×102 native (close to 16:10). The library tile aspect is `4/3` to fit Lynx box art, which mirrors the home-console family's landscape boxes. There's no rule that tile aspect has to match framebuffer aspect — the framebuffer drives the renderer's viewport math (scaling-mode dependent); the tile aspect drives the library grid's slot shape so cover art doesn't letterbox awkwardly. Different concerns, picked separately per system.
