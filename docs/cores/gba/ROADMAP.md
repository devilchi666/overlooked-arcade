# gba — Roadmap

Per-core phase tracking for Nintendo Game Boy Advance. Mirrors the
project-wide ROADMAP shape (Phase 0 = onboarded, Phase 1 = first ROM
running, Phase 2 = polish, Phase 3+ = shared infra) but scoped to GBA.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-19)

Core comes online via the libretro pivot — no Rust crate vendoring.
mGBA installed by operator; OA wires the system into the existing
shell, scanner, bindings, library DB, and settings pipelines.

- ✅ `oa_core::SystemId::Gba` variant added.
- ✅ `parse_system_id("gba" | "game-boy-advance" | "gameboyadvance")
  → SystemId::Gba` in `apps/oa-shell/src/main.rs`.
- ✅ `default_core_dll_for_system("gba") → "mgba_libretro.dll"`.
- ✅ Per-system button bits + bindings in `apps/oa-shell/src/bindings.rs::gba`
  — 10-button layout (4-way d-pad + A + B + L + R + START + SELECT),
  `GBA_BUTTONS` table, `default_gba_bindings()`, `defaults_for("gba")` arm.
- ✅ `gba_to_libretro_bits` identity remap.
- ✅ `bindings::to_libretro_bits` + `bit_for` + `buttons_for` dispatch
  arms include `"gba"`.
- ✅ `rom_hashes::libretro_dat_refs_for_system("gba")` returns
  `metadat/no-intro/Nintendo - Game Boy Advance`.
- ✅ `media::repo_for_system_id("gba")` returns
  `Some("Nintendo_-_Game_Boy_Advance")`.
- ✅ System registered in `frontend/src/themes/registry.ts` — `SystemId`
  union extended with `gba`, `systemThemes.gba` entry (extension
  `["gba"]`, portrait 3/4 tile aspect, `crt-lite` default shader preset
  per the handheld convention).
- ✅ Theme block in `frontend/src/themes/systems.css` — deep indigo
  (hue 285°, lightness 0.55, chroma 0.20). Sits between SNES (270°,
  L=0.62) and Lynx (290°, L=0.65) in hue but the lightness axis
  separates the three: GBA = darkest, SNES = mid, Lynx = brightest.
- ✅ Per-core docs scaffold (this directory).

**Acceptance gate:** Operator can drop `mgba_libretro.dll` into the
install, scan a GBA ROMs folder, see GBA-themed (deep indigo) tiles
appear in the library, and click one to launch — without rebuilding
Rust.

---

## ⬜ Phase 1 — First GBA ROM running

- ⬜ Operator validation: launch a real `.gba` ROM end-to-end (pixels +
  audio + controller). Suggested reference set: **The Legend of Zelda:
  The Minish Cap**, **Pokémon FireRed / LeafGreen / Emerald**,
  **Metroid: Zero Mission**, **Advance Wars**, **Castlevania: Aria of
  Sorrow**, **Final Fantasy Tactics Advance**, **Mario Kart: Super
  Circuit**, **Mother 3**.
- ⬜ Save state F5/F8 round-trip confirmation. mGBA supports
  `retro_serialize`.
- ⬜ Battery-save persistence — Pokémon games are the canonical test
  (frequent SRAM writes); GBA also has Flash-based saves on some
  cartridges that mGBA handles transparently via libretro's SaveRam
  region.
- ⬜ Per-game cover sync via libretro-thumbnails — operator runs
  `Settings → Library → Sync media for Game Boy Advance` and confirms
  covers download.
- ⬜ Libretro-database hash matching — operator runs
  `Settings → Library → Identify ROMs` to confirm No-Intro SHA-1 lookup
  populates canonical titles + publishers + years.
- ⬜ BIOS-optional vs BIOS-required behavior: launch a BIOS-required
  title (Splinter Cell etc.) WITHOUT `gba_bios.bin` present and confirm
  the failure mode is informative (mGBA's BIOS-less path emulates most
  but not all functions, and some titles hang silently rather than
  surfacing an error).

**Acceptance gate:** A reference set of GBA games run with pixels +
audio + working controller at native 59.73 Hz.

---

## ⬜ Phase 2 — Polish

- ⬜ Dedicated `lcd-handheld` shader preset — same temporary `crt-lite`
  compromise as Lynx / GG / GB.
- ⬜ Per-system aspect override — GBA's 240×160 is a 3:2 ratio (not the
  4:3 default). Either ship a per-system override (`display_aspect_override = 1.5`)
  or document the manual setting.
- ⬜ BIOS auto-detection / pre-launch check — when the operator launches
  a known-BIOS-required title and `gba_bios.bin` is absent, surface a
  banner in the per-game launch flow. Same shape as the PCE-CD BIOS
  pre-check.
- ⬜ Game-tilt sensor support (Kirby Tilt 'n' Tumble GBA port, Yoshi
  Topsy-Turvy, WarioWare Twisted!) — mGBA supports this via libretro's
  pointer-sensor extension, but OA's input layer doesn't yet route
  motion. Deferred to the same analog-input pass as Atari 7800 Trak-Ball.
- ⬜ Solar-sensor support (Boktai 1/2/3) — same gating as tilt sensor.
- ⬜ Rumble support — some GBA Pokémon titles + Drill Dozer used
  cartridge-side rumble packs. mGBA surfaces this via libretro's
  rumble extension; needs operator-side test.

---

## ⬜ Phase 3+ — Stretch

Per the project ROADMAP, all post-Phase-3 work (rewind, TAS, WebM
export, memory inspector, cheats, milestones, run-ahead) is
system-agnostic and lights up automatically once the engine work
ships. GBA-specific items:

- ⬜ Game Genie / Action Replay / CodeBreaker code support — runs
  through the libretro cheat path (project RetroArch parity slice 8);
  mGBA's `retro_cheat_set` accepts the GBA cheat formats.
- ⬜ Game Link Cable multiplayer (Pokémon trading / battles, Four
  Swords, Mario Kart Super Circuit lap-sharing) — out of scope for
  single-instance playback. mGBA has experimental link-cable support
  via libretro extensions but it's deferred.
- ⬜ GBA Wireless Adapter (Pokémon FRLG / Emerald wireless trading) —
  same deferral as Link Cable.
- ⬜ Custom forked mGBA — only if upstream regresses or we want
  OA-specific extensions.

---

## Scope clarifications

- **Separate slug from `gb`.** Despite the family name, GBA hardware
  is a different generation (32-bit ARM7TDMI vs Sharp LR35902) and
  the libretro cores don't share. Keeping the slugs separate matches
  the libretro / RetroArch convention + lets per-system settings
  (input, shader, BIOS path) diverge cleanly.
- **GB/GBC backward compat is `gb`-slug routing.** The GBA console
  hardware could play .gb/.gbc carts via the slot's hardware
  compatibility mode, but in OA terms those games still go through
  the `gb` slug + Gambatte. Users wanting to play a .gb game "the
  GBA way" can use the per-game core override to swap in mGBA, but
  that's the unusual case.
- **`.bin` extension intentionally excluded** to avoid collision.
  Users with `.bin` GBA dumps rename to `.gba`.
- **No vendoring.** Buildbot mGBA .dll, treated as a black box.
