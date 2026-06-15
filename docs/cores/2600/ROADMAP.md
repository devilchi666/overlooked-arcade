# 2600 — Roadmap

Per-core phase tracking for Atari 2600 / VCS. Mirrors the project-wide
ROADMAP shape but scoped to 2600.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-19)

Core comes online via the libretro pivot — no Rust crate vendoring.
Stella installed by operator; OA wires the system into the existing
shell, scanner, bindings, library DB, and settings pipelines.

- ✅ `oa_core::SystemId::Atari2600` variant added (Rust variant name
  `Atari2600` since identifiers can't start with a digit; string
  slug stays `"2600"`).
- ✅ `parse_system_id("2600" | "atari2600" | "vcs") → SystemId::Atari2600`.
- ✅ `default_core_dll_for_system("2600") → "stella_libretro.dll"`.
- ✅ Per-system button bits + bindings in `apps/oa-shell/src/bindings.rs::atari2600`
  — 7-button layout (4-way d-pad + FIRE + SELECT + RESET),
  `ATARI2600_BUTTONS` table, `default_atari2600_bindings()`,
  `defaults_for("2600")` arm.
- ✅ `atari2600_to_libretro_bits` identity remap.
- ✅ `bindings::to_libretro_bits` + `bit_for` + `buttons_for` dispatch
  arms include `"2600"`.
- ✅ `rom_hashes::libretro_dat_refs_for_system("2600")` returns
  `metadat/no-intro/Atari - 2600`.
- ✅ `media::repo_for_system_id("2600")` returns `Some("Atari_-_2600")`.
- ✅ System registered in `frontend/src/platform/themes/registry.ts` —
  `systemThemes["2600"]` entry (extension `["a26"]`, portrait 3/4 tile
  aspect, `crt-lite` default shader preset).
- ✅ Theme palette in `frontend/src/platform/themes/systemPalettes.ts`
  (typed `SYSTEM_PALETTES`, injected as `[data-system]` CSS at boot) —
  muted wood-grain brown (hue 60°, chroma 0.07).
- ✅ Single-button exception documented in the
  `z_is_the_primary_action_button_on_every_system` test header;
  Z=FIRE assertion lives in `defaults_cover_every_2600_button`
  instead.
- ✅ Per-core docs scaffold (this directory).

**Acceptance gate:** Operator can drop `stella_libretro.dll` into
the install, scan a 2600 ROMs folder, see wood-grain-themed tiles
appear in the library, and click one to launch — without rebuilding
Rust.

---

## 🟨 Phase 1 — First 2600 ROM running

- ⬜ Operator validation: launch a real `.a26` ROM end-to-end. Suggested
  joystick-only reference set: **Adventure**, **Pitfall!**, **Yars'
  Revenge**, **River Raid**, **Asteroids**, **Combat** (1977 pack-in),
  **Space Invaders**, **Centipede**, **Missile Command**.
- ✅ Save state F5/F8 round-trip — shipped via the cross-system save-state
  infra (Phase 1.5 + Phase 4 multi-slot UI + thumbnails).
- ⬜ Console-switch behavior: launch a multi-game cart and confirm
  Game Select (RShift) cycles through game variations. Confirm Game
  Reset (Enter) restarts cleanly — operator validation.
- ✅ Per-game cover sync via libretro-thumbnails — `2600 → Atari_-_2600`
  shipped in `apps/oa-shell/src/media.rs::repos_for_system_id`.
- ✅ Libretro-database hash matching — shipped via cross-system
  `rom_hashes::resolve_rom_hashes_for_system`; 2600 dat ref lives in
  `apps/oa-shell/src/rom_hashes.rs`.
- ⬜ Region auto-detect: launch one US-NTSC, one EU-PAL ROM and
  confirm Stella switches timing (59.92 Hz vs 49.86 Hz) — operator
  validation.
- ⬜ Per-folder `*.bin → 2600` rule demo — operator workflow
  documentation, not code.

**Acceptance gate:** A reference set of joystick-controlled 2600 games
run with pixels + audio + working FIRE / Game Select / Game Reset
controls at native 59.92 Hz NTSC.

---

## Phase 2 — Polish

- ✅ Paddle controller support — Breakout, Kaboom!, Warlords, Night
  Driver, Super Breakout, Indy 500. Closed by shared analog input
  infra Phase A (PADDLE device-type in `apps/oa-shell/src/main.rs::arm_libretro_device`)
  + Phase C (mouse-as-stick analog source via
  `crates/oa-input/src/lib.rs::MouseSource::X`). Operator workflow:
  per-game Input → device = "Analog / Paddle", left-stick mouse
  source = X. Operator playtest pending against Stella.
- ✅ Driving controller (Indy 500 hybrid paddle/spinner) — same
  PADDLE + mouse-X path as paddle games.
- ⬜ Keypad / "Star Raiders" controller (overlay-based games) — niche;
  deferred indefinitely.
- ⬜ Light gun (Sentinel, Shooting Arcade) — shared light-gun infra
  (POINTER device shipped); operator validation pending.
- ✅ Per-game difficulty / TV-type / phosphor preset surface — Stella's
  RETRO_VARIABLEs flow through `core_options::refresh_schema` into the
  per-system / per-game core-options surface automatically.
- ⬜ Supercharger / multicart bankswitching header-strip pass — only
  add a header-aware sha1 candidate to `rom_header.rs` if operator
  validation shows misses on Supercharger dumps.

---

## Phase 3+ — Stretch

- ⬜ Cheat support — Stella's `retro_cheat_set` accepts cheat formats;
  needs validation via the project RetroArch parity slice 8.
- ⬜ Homebrew / hack tile distinction — DATA work; per-game
  source-of-origin tag (No-Intro / homebrew / reproduction / hack).
  Project-level enhancement.

---

## Scope clarifications

- **No vendoring.** Buildbot Stella .dll in `<exe_dir>/cores/`.
- **No BIOS.** 2600 carts contained the entire system firmware; no
  external BIOS file exists or is needed.
- **`.a26` only at the global registry; `.bin` via per-folder rules.**
  The community-standard `.bin` extension collides with too many
  other systems to safely auto-classify globally.
- **Single-button system.** Cross-system "Z is primary" rule applies
  to FIRE; the matching `z_is_the_primary_action_button_on_every_system`
  test fixture omits 2600 because there's no secondary to assert.
- **Paddle / analog-input games unplayable in Phase 0/1.** Documented
  reality — joystick games are the supported corpus; paddle games
  wait for shared analog-input infra.

