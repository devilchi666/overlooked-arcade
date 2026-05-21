# gamegear — Roadmap

Per-core phase tracking for Sega Game Gear. Mirrors the project-wide
ROADMAP shape (Phase 0 = onboarded, Phase 1 = first ROM running, Phase 2
= polish, Phase 3+ = shared infra) but scoped to GG.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-19)

Core comes online via the libretro pivot — no Rust crate vendoring.
Genesis Plus GX installed by operator (one .dll services both
`gamegear` and `sms`); OA wires the system into the existing shell,
scanner, bindings, library DB, and settings pipelines.

- ✅ System registered in `frontend/src/themes/registry.ts` — `SystemId`
  union extended with `gamegear`, `systemThemes.gamegear` entry
  (extension `gg`, landscape tile aspect 4/3, `crt-lite` default
  shader preset).
- ✅ Theme block in `frontend/src/themes/systems.css` — yellow-green
  (hue 130°, chroma 0.18), pulled from the GG launch packaging palette.
- ✅ Per-system button bits + bindings in `apps/oa-shell/src/bindings.rs::gamegear`
  — 7-button layout (4-way d-pad + B1 + B2 + START), `GAMEGEAR_BUTTONS`
  table, `default_gamegear_bindings()`, `defaults_for("gamegear")` arm.
- ✅ `gamegear_to_libretro_bits` identity remap.
- ✅ `bindings::to_libretro_bits` + `bit_for` + `buttons_for` dispatch
  arms include `"gamegear"`.
- ✅ `default_core_dll_for_system("gamegear") → "genesis_plus_gx_libretro.dll"`
  in `apps/oa-shell/src/main.rs`. `parse_system_id("gamegear" |
  "game-gear") → SystemId::GameGear` (already wired from a prior
  session).
- ✅ `rom_hashes::libretro_dat_refs_for_system("gamegear")` returns
  `&[DatRef { subdir: "metadat/no-intro", basename: "Sega - Game Gear" }]`.
- ✅ `media::repo_for_system_id("gamegear")` returns
  `Some("Sega_-_Game_Gear")` (was wired ahead of onboarding; test
  fixture bumped to include `gamegear` in the onboarded set).
- ✅ Per-core docs scaffold (this directory).

**Acceptance gate:** Operator can drop `genesis_plus_gx_libretro.dll`
into the install (also services `sms`), scan a Game Gear ROMs folder,
see GG-themed (yellow-green) tiles appear in the library, and click
one to launch — without rebuilding Rust.

---

## ⬜ Phase 1 — First Game Gear ROM running

- ⬜ Operator validation: launch a real `.gg` ROM end-to-end (pixels + audio + controller). Suggested: **Sonic the Hedgehog (Game Gear)**, **Shinobi**, **Tails Adventure**, **Streets of Rage** (GG port), **Columns** — operator playtest.
- ✅ Save state F5/F8 round-trip — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ✅ Per-game cover sync via libretro-thumbnails — closed by cross-system media sync (`media::sync_media_for_system`).
- ✅ Libretro-database hash matching — closed by cross-system hash ID (`rom_hashes::resolve_rom_hashes_for_system`).
- ⬜ SMS-mode-via-GG-ROM detection — operator confirmation that the per-game letterbox aspect reads correctly.

**Acceptance gate:** A reference set of GG games run with pixels +
audio + working controller at native 59.92 Hz.

---

## ⬜ Phase 2 — Polish

- ⬜ Per-system shader tweaks — operator-driven preset choice; `lcd-handheld` shader preset shipped (`ShaderPreset::LcdHandheld` id 4) but per-system default binding for gg still ⬜.
- ⬜ Game Gear bezel — bezel-rendering infra shipped via shader pipeline (`crates/oa-render/src/lib.rs::ShaderPreset` + `shaders/presets/*.preset.toml`); GG-specific bezel asset still operator-driven.
- ⬜ Master Gear adapter — documented here so a future contributor doesn't add a separate slug.
- ⬜ Game Gear's link cable (multiplayer Columns / Pop Breaker / a few others) — deferred.

---

## ⬜ Phase 3+ — Stretch

GG-specific items:

- ⬜ Game Genie / Pro Action Replay code support — operator-driven validation that GPGX's `retro_cheat_set` accepts GG Game Genie format.
- ⬜ Custom forked Genesis Plus GX — deferred.

---

## Scope clarifications

- **Shared .dll with SMS.** One Genesis Plus GX install services both
  slugs — operators installing for one get the other for free. The
  default-core arm in `default_core_dll_for_system` is the only
  per-system difference; everything else (bindings, theme, extensions)
  is independent.
- **No BIOS required.** GG cart playback is BIOS-optional — boot splash
  is the only thing affected. `bios.gg` in `<exe_dir>/system/` is the
  per-system convention.
- **`.bin` extension intentionally excluded** to avoid collision with
  every other `.bin`-claiming system. Users with `.bin` GG dumps
  rename to `.gg`.
