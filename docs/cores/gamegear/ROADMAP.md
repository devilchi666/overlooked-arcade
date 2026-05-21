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

- ⬜ Operator validation: launch a real `.gg` ROM end-to-end (pixels +
  audio + controller). Suggested reference set: **Sonic the Hedgehog
  (Game Gear)**, **Shinobi**, **Tails Adventure**, **Streets of Rage**
  (GG port), **Columns**.
- ⬜ Save state F5/F8 round-trip confirmation via the existing path.
- ⬜ Per-game cover sync via libretro-thumbnails — **infra ready
  2026-05-19, needs operator validation.** Operator: run
  `Settings → Library → Sync media for Game Gear` and confirm covers
  download.
- ⬜ Libretro-database hash matching — same — operator runs
  `Settings → Library → Identify ROMs` to confirm No-Intro SHA-1 lookup
  populates canonical titles.
- ⬜ SMS-mode-via-GG-ROM detection — a handful of GG titles ship with
  SMS-mode signatures and render at 256×192. GPGX handles this
  transparently; needs operator confirmation that the per-game
  letterbox aspect reads correctly.

**Acceptance gate:** A reference set of GG games run with pixels +
audio + working controller at native 59.92 Hz.

---

## ⬜ Phase 2 — Polish

- ⬜ Per-system shader tweaks: GG's 160×144 LCD source is chunky at
  modern scales. `crt-lite` softens the upscale acceptably but a
  dedicated `lcd-handheld` preset (subpixel grid, no scanlines) would
  be more period-correct. Out of scope until shared LCD preset infra
  lands.
- ⬜ Game Gear bezel — handheld systems benefit from era-correct bezel
  art (the unit's plastic frame around the LCD). Same shared bezel
  infra that lights up for Lynx + Virtual Boy.
- ⬜ Master Gear adapter (Game Gear → SMS-cart converter) didn't change
  the dump format; no separate slug needed. Documented here so a
  future contributor doesn't add one.
- ⬜ Game Gear's link cable (multiplayer Columns / Pop Breaker / a few
  others) — deferred. Libretro's link-cable support exists for some
  systems but Game Gear specifically isn't surfaced yet.

---

## ⬜ Phase 3+ — Stretch

Per the project ROADMAP, all post-Phase-3 work (rewind, TAS, WebM
export, memory inspector, cheats, milestones, run-ahead) is
system-agnostic and lights up automatically once the engine work
ships. GG-specific items:

- ⬜ Game Genie / Pro Action Replay code support — runs through the
  libretro cheat path; needs validation that GPGX's `retro_cheat_set`
  accepts GG Game Genie format.
- ⬜ Custom forked Genesis Plus GX — only if upstream regresses or we
  want OA-specific GG extensions.

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
