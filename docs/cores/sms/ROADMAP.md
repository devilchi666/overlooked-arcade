# sms — Roadmap

Per-core phase tracking for Sega Master System. Mirrors the project-wide
ROADMAP shape (Phase 0 = onboarded, Phase 1 = first ROM running, Phase 2
= polish, Phase 3+ = shared infra) but scoped to SMS.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-19)

Core comes online via the libretro pivot — no Rust crate vendoring.
Genesis Plus GX installed by operator; OA wires the system into the
existing shell, scanner, bindings, library DB, and settings pipelines.

- ✅ System registered in `frontend/src/themes/registry.ts` — `SystemId`
  union extended with `sms`, `systemThemes.sms` entry (extension `sms`,
  landscape tile aspect 4/3, `crt-lite` default shader preset).
- ✅ Theme block in `frontend/src/themes/systems.css` — neon magenta
  (hue 340°, chroma 0.22), evoking the 1986-1990 Western Big Box
  grid-floor art.
- ✅ Per-system button bits + bindings in `apps/oa-shell/src/bindings.rs::sms`
  — 7-button layout (4-way d-pad + B1 + B2 + PAUSE), `SMS_BUTTONS`
  table, `default_sms_bindings()`, `defaults_for("sms")` arm.
- ✅ `sms_to_libretro_bits` identity remap (bits laid out to match
  libretro RetroPad positions directly).
- ✅ `bindings::to_libretro_bits` + `bit_for` + `buttons_for` dispatch
  arms include `"sms"`.
- ✅ `default_core_dll_for_system("sms") → "genesis_plus_gx_libretro.dll"`
  in `apps/oa-shell/src/main.rs`. `parse_system_id("sms") → SystemId::Sms`
  (already wired from a prior session).
- ✅ `rom_hashes::libretro_dat_refs_for_system("sms")` returns
  `&[DatRef { subdir: "metadat/no-intro", basename: "Sega - Master System - Mark III" }]`.
- ✅ `media::repo_for_system_id("sms")` returns
  `Some("Sega_-_Master_System_-_Mark_III")` (was wired ahead of onboarding;
  test fixture bumped to include `sms` in the onboarded set).
- ✅ Per-core docs scaffold (this directory).

**Acceptance gate:** Operator can drop `genesis_plus_gx_libretro.dll`
into the install, scan an SMS ROMs folder, see SMS-themed (neon magenta)
tiles appear in the library, and click one to launch — without rebuilding
Rust.

---

## ⬜ Phase 1 — First SMS ROM running

- ⬜ Operator validation: **Alex Kidd in Miracle World**, **Phantasy Star**, **Wonder Boy III: The Dragon's Trap**, **Sonic the Hedgehog (SMS)**, **Shinobi** — operator playtest.
- ✅ Save state F5/F8 round-trip — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ⬜ Multi-region testing — operator playtest (USA + Europe + Japan/Mark III dumps).
- ✅ Per-game cover sync via libretro-thumbnails — closed by cross-system media sync (`media::sync_media_for_system`).
- ✅ Libretro-database hash matching — closed by cross-system hash ID (`rom_hashes::resolve_rom_hashes_for_system`).
- ⬜ Japan-region FM sound (YM2413) compatibility — operator-driven Core-Option curation (per-system Core Options surface shipped cross-system).

**Acceptance gate:** A reference set of SMS games run with pixels +
audio + working controller at native 59.92 Hz NTSC.

---

## ⬜ Phase 2 — Polish

- ⬜ Per-system shader tweaks — operator-driven shader-preset choice (per-system shader override shipped cross-system).
- ⬜ Optional BIOS handling — operator-driven UI polish (per-system settings page shipped cross-system).
- ⬜ Light Phaser — operator validation. LIGHTGUN dispatch shipped 2026-05-25 on `feat/light-gun-harness` (`crates/oa-libretro/src/state.rs::lightgun_field_value`). Genesis Plus GX exposes the Phaser through RETRO_DEVICE_LIGHTGUN; SCREEN_X/Y/TRIGGER now reach the core. Flagship validation: Operation Wolf / Rambo III / Shooting Gallery. Catalogued in `apps/oa-shell/src/light_gun_systems.rs`.
- ⬜ 3D glasses (SMS SegaScope add-on) — operator-driven Genesis Plus GX anaglyph fallback toggle (deferred).

---

## ⬜ Phase 3+ — Stretch

SMS-specific items:

- ⬜ Game Genie / Pro Action Replay code support — operator-driven validation of Genesis Plus GX's `retro_cheat_set`.
- ⬜ Custom forked Genesis Plus GX — deferred.

---

## Scope clarifications

- **No vendoring for SMS today.** The libretro pivot means we ship the
  upstream nightly Genesis Plus GX .dll alongside our binary and tell
  operators to drop it into `<exe_dir>/cores/`.
- **No BIOS required.** SMS cart playback is BIOS-optional — boot splash
  is the only thing affected. `bios.sms` in `<exe_dir>/system/` is the
  per-system convention if the operator wants the era-correct boot.
- **`.bin` extension intentionally excluded** to avoid collision with
  PCE-CD track files, Sega CD audio tracks, ColecoVision, and Atari
  2600 dumps. Users with `.bin` SMS dumps rename to `.sms`.
- **Shared .dll with Game Gear.** One Genesis Plus GX install services
  both slugs — operators installing for one get the other for free.
