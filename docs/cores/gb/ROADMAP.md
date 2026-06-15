# gb — Roadmap

Per-core phase tracking for Nintendo Game Boy / Game Boy Color. Mirrors
the project-wide ROADMAP shape (Phase 0 = onboarded, Phase 1 = first
ROM running, Phase 2 = polish, Phase 3+ = shared infra) but scoped to
GB.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-19)

Core comes online via the libretro pivot — no Rust crate vendoring.
Gambatte installed by operator; OA wires the system into the existing
shell, scanner, bindings, library DB, and settings pipelines.

- ✅ `oa_core::SystemId::Gb` variant added.
- ✅ `parse_system_id("gb" | "gbc" | "gameboy" | "game-boy" | "game-boy-color")
  → SystemId::Gb` in `apps/oa-shell/src/main.rs`.
- ✅ `default_core_dll_for_system("gb") → "gambatte_libretro.dll"`.
- ✅ Per-system button bits + bindings in `apps/oa-shell/src/bindings.rs::gb`
  — 8-button layout (4-way d-pad + A + B + START + SELECT, NES-shape),
  `GB_BUTTONS` table, `default_gb_bindings()`, `defaults_for("gb")` arm.
- ✅ `gb_to_libretro_bits` identity remap.
- ✅ `bindings::to_libretro_bits` + `bit_for` + `buttons_for` dispatch
  arms include `"gb"`.
- ✅ `rom_hashes::libretro_dat_refs_for_system("gb")` returns TWO
  DatRefs — `metadat/no-intro/Nintendo - Game Boy` AND
  `metadat/no-intro/Nintendo - Game Boy Color` — merged into one local
  corpus by `fetch_and_parse_all`, so both `.gb` and `.gbc` dumps match.
- ✅ `media::repo_for_system_id("gb")` returns
  `Some("Nintendo_-_Game_Boy")` as the primary cover repo. GBC-specific
  cover coverage from the Game-Boy-Color thumbnails repo is a documented
  follow-up gap.
- ✅ System registered in `frontend/src/platform/themes/registry.ts` — `SystemId`
  union extended with `gb`, `systemThemes.gb` entry (extensions
  `["gb", "gbc"]`, portrait 3/4 tile aspect, `crt-lite` default shader
  preset per the handheld convention).
- ✅ Theme palette in the per-system palette map
  (`frontend/src/platform/themes/systemPalettes.ts`) — muted DMG
  pea-green (hue 145°, chroma 0.13). Decisively distinct from GG
  (130°, 0.18) by hue + chroma.
- ✅ Per-core docs scaffold (this directory).

**Acceptance gate:** Operator can drop `gambatte_libretro.dll` into
the install, scan a GB/GBC ROMs folder, see Game Boy-themed (DMG
pea-green) tiles appear in the library, and click one to launch —
without rebuilding Rust.

---

## ⬜ Phase 1 — First Game Boy ROM running

- ⬜ Operator validation (DMG): **Tetris**, **Super Mario Land**, **Link's Awakening**, **Pokémon Red/Blue**, **Kirby's Dream Land** — operator playtest.
- ⬜ CGB validation: **Pokémon Crystal**, **Link's Awakening DX**, **Wario Land 3**, **Shantae** — operator playtest.
- ✅ Save state F5/F8 round-trip — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ⬜ Battery-save persistence — operator playtest of Pokémon SRAM persistence.
- ✅ Per-game cover sync via libretro-thumbnails (DMG + CGB) — closed by cross-system multi-repo cover sync (`media::repos_for_system_id` returning a slice).
- ✅ Libretro-database hash matching against the merged GB + GBC corpus — closed by cross-system hash ID (`rom_hashes::resolve_rom_hashes_for_system`).
- ⬜ DMG vs CGB visual distinction — operator spot-check that Gambatte handles cart CGB flag correctly.

**Acceptance gate:** A reference set of GB + GBC games run with pixels +
audio + working controller at native 59.73 Hz.

---

## ⬜ Phase 2 — Polish

- ✅ Dedicated `lcd-handheld` shader preset — defaulted 2026-05-24 for `gb` (in `frontend/src/platform/themes/registry.ts::systemThemes.gb.defaultShaderPreset`). The same wave defaulted gbc / gba / gamegear / ngp / wonderswan / pokemini / psp.
- ⬜ Game Boy bezel — bezel-rendering infra shipped via shader pipeline; DMG-specific bezel asset still operator-driven.
- ⬜ DMG palette presets — operator-driven Gambatte core-option curation via the per-system Core Options page (per-system settings shipped).
- ⬜ Super Game Boy palette support — deferred until the `snes`-side SGB path lands.
- ✅ Multi-repo cover sync — shipped via `apps/oa-shell/src/media.rs::repos_for_system_id` returning a slice (DMG + CGB).

---

## ⬜ Phase 3+ — Stretch

GB-specific items:

- ⬜ Game Genie / GameShark code support — operator-driven validation of Gambatte's `retro_cheat_set`.
- ⬜ Link Cable multiplayer — deferred (out of scope for Gambatte's single-instance path).
- ⬜ Custom forked Gambatte — deferred.

---

## Scope clarifications

- **Single slug for DMG + CGB.** Both Game Boy and Game Boy Color
  hardware variants share `gb` as their SystemId. Gambatte
  auto-detects from the ROM header (CGB flag byte at offset 0x143).
  Splitting into separate slugs was rejected (see DECISIONS).
- **No BIOS required.** Both DMG and CGB run without their boot ROMs;
  the era-correct logo + jingle splashes get skipped. Optional
  `dmg_boot.bin` + `cgb_boot.bin` in `<exe_dir>/system/`.
- **`.bin` extension intentionally excluded** to avoid collision with
  every other `.bin`-claiming system. Users with `.bin` GB dumps
  rename to `.gb`.
- **No vendoring.** Buildbot Gambatte .dll, treated as a black box.
