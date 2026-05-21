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
- ✅ System registered in `frontend/src/themes/registry.ts` — `SystemId`
  union extended with `gb`, `systemThemes.gb` entry (extensions
  `["gb", "gbc"]`, portrait 3/4 tile aspect, `crt-lite` default shader
  preset per the handheld convention).
- ✅ Theme block in `frontend/src/themes/systems.css` — muted DMG
  pea-green (hue 145°, chroma 0.13). Decisively distinct from GG
  (130°, 0.18) by hue + chroma.
- ✅ Per-core docs scaffold (this directory).

**Acceptance gate:** Operator can drop `gambatte_libretro.dll` into
the install, scan a GB/GBC ROMs folder, see Game Boy-themed (DMG
pea-green) tiles appear in the library, and click one to launch —
without rebuilding Rust.

---

## ⬜ Phase 1 — First Game Boy ROM running

- ⬜ Operator validation: launch a real `.gb` ROM end-to-end (pixels +
  audio + controller). Suggested DMG reference set: **Tetris**,
  **Super Mario Land**, **The Legend of Zelda: Link's Awakening**,
  **Pokémon Red/Blue**, **Kirby's Dream Land**.
- ⬜ CGB validation: launch a `.gbc` ROM. Suggested set: **Pokémon
  Crystal**, **The Legend of Zelda: Link's Awakening DX**, **Wario
  Land 3**, **Shantae**.
- ⬜ Save state F5/F8 round-trip confirmation via the existing path.
  Gambatte supports `retro_serialize`.
- ⬜ Battery-save persistence — Pokémon games are the canonical test
  (they write SRAM frequently). Gambatte exposes this via libretro's
  standard SaveRam region; OA's per-game save infra should pick it
  up automatically.
- ⬜ Per-game cover sync via libretro-thumbnails `Nintendo_-_Game_Boy` —
  **infra ready 2026-05-19, needs operator validation.** Operator: run
  `Settings → Library → Sync media for Game Boy` and confirm DMG covers
  download. GBC-specific covers stay missing until the multi-repo
  follow-up lands.
- ⬜ Libretro-database hash matching against the merged GB + GBC
  corpus — operator runs `Settings → Library → Identify ROMs`.
- ⬜ DMG vs CGB visual distinction: a GBC-only game (Pokémon Crystal)
  should launch in 32k-color mode; a backward-compat game (Pokémon
  Gold/Silver) should respect the cart's CGB flag and pick the right
  palette. Gambatte handles this automatically — needs operator
  spot-check.

**Acceptance gate:** A reference set of GB + GBC games run with pixels +
audio + working controller at native 59.73 Hz.

---

## ⬜ Phase 2 — Polish

- ⬜ Dedicated `lcd-handheld` shader preset. Game Boy's 160×144 LCD
  source needs a different visual treatment than CRT-era systems —
  subpixel grid + matrix dot pattern, no scanlines. Currently using
  `crt-lite` as a temporary compromise (same as Lynx + Game Gear). The
  shared preset infra lands once 3+ handhelds need it; this would be
  triggered by GB onboarding.
- ⬜ Game Boy bezel — handheld systems benefit from era-correct bezel
  art (the DMG plastic frame around the LCD, optionally the original
  green-on-pea-soup screen tint). Same shared bezel infra as Lynx +
  Game Gear.
- ⬜ DMG palette presets: 4-shade grayscale is the default, but real
  DMG screens shipped pea-soup green, and Gambatte has a per-game
  palette option for DMG-on-CGB hardware (Tetris in red, Mario in
  yellow, etc.). Surface via the per-system Core Options page.
- ⬜ Super Game Boy palette support — SNES adapter palette data for
  DMG games. Niche; deferred until the `snes`-side SGB path lands.
- ⬜ Multi-repo cover sync: extend `repo_for_system_id` to optionally
  return multiple repos so `gb` can sync from BOTH `Nintendo_-_Game_Boy`
  AND `Nintendo_-_Game_Boy_Color`. Same architectural change that
  benefits any future system with multi-hardware-variant single-slug
  coverage (Wonderswan mono+color is the next candidate).

---

## ⬜ Phase 3+ — Stretch

Per the project ROADMAP, all post-Phase-3 work (rewind, TAS, WebM
export, memory inspector, cheats, milestones, run-ahead) is
system-agnostic and lights up automatically once the engine work
ships. GB-specific items:

- ⬜ Game Genie / GameShark code support — runs through the libretro
  cheat path (project RetroArch parity slice 8); Gambatte's
  `retro_cheat_set` accepts both GG and GS codes.
- ⬜ Link Cable multiplayer (Pokémon trading, Tetris versus, etc.) —
  out of scope for Gambatte's single-instance path. Operators wanting
  link-cable scenarios swap to `tgbdual_libretro` via per-system Cores.
- ⬜ Custom forked Gambatte — only if upstream regresses or we want
  OA-specific extensions. Recipe mirrors the Beetle PCE Fast plan.

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

---

## 2026-05-21 — Stale-cleanup audit

The Phase 1+ items above were written when this system onboarded, before cross-system infrastructure (Phases 1.5 / 2.5–2.8 / 3 / 4 + direct-launch CLI) landed. Many `⬜` items are actually shipped — see `docs/cores/AUDIT_2026-05-21.md` for the per-item breakdown (stale vs open-code vs open-operator) for this system.
