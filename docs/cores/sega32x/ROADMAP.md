# sega32x — Roadmap

Per-core phase tracking for Sega 32X. Mirrors the project-wide ROADMAP
shape but scoped to 32X.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

Paired with Sega CD. Core comes online via the libretro pivot — no
Rust crate vendoring. PicoDrive is the recommended default (the only
mainstream libretro core with 32X support).

- ✅ System registered in `frontend/src/themes/registry.ts` —
  `SystemId` union extended with `sega32x`, `systemThemes.sega32x`
  entry (extensions `["32x"]`, landscape tile aspect 4/3, `crt-lite`
  default shader preset).
- ✅ Theme block in `frontend/src/themes/systems.css` — neon orange at
  hue 42° + L=0.68 + C=0.22. Period-accurate to the 32X marketing
  palette; lands in the open 35-50° hue band so no collisions with
  TG-16 55° / ChannelF 25° / NES 28° / MAME 12°.
- ✅ Per-system input wiring — 32X shares the 6-button Mega Drive
  controller via the `"genesis" | "segacd" | "sega32x" => genesis_*`
  dispatch arms in `apps/oa-shell/src/bindings.rs`. Same pattern PCE-CD
  uses to share TG-16's controller.
- ✅ `default_core_dll_for_system("sega32x") → "picodrive_libretro.dll"`
  in `apps/oa-shell/src/main.rs`.
- ✅ `parse_system_id("sega32x" | "32x" | "sega-32x") →
  SystemId::Sega32X` (new variant on `oa_core::SystemId` enum).
- ✅ `rom_hashes::libretro_dat_refs_for_system("sega32x")` returns
  `&[DatRef { subdir: "metadat/no-intro", basename: "Sega - 32X" }]`.
- ✅ `media::repo_for_system_id("sega32x")` returns
  `Some("Sega_-_32X")` so cover sync works as soon as the operator
  runs it.
- ✅ Per-core docs scaffold (this directory).

**Acceptance gate:** Operator can drop `picodrive_libretro.dll` into
the install, scan a Sega 32X ROMs folder, see 32X-themed (neon orange)
tiles appear in the library, and click one to launch — without
rebuilding Rust.

---

## ⬜ Phase 1 — First 32X cart running

- ⬜ Operator validation: **Knuckles' Chaotix**, **Virtua Racing Deluxe**, **Doom 32X**, **Star Wars Arcade**, **Kolibri** — operator playtest.
- ✅ Save state F5/F8 round-trip — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ⬜ Multi-region testing — operator playtest (NTSC US + NTSC JP + PAL EU 32X carts).
- ✅ Per-game cover sync via libretro-thumbnails — closed by cross-system media sync (`media::sync_media_for_system`).
- ✅ Libretro-database hash matching — closed by cross-system hash ID (`rom_hashes::resolve_rom_hashes_for_system`).

**Acceptance gate:** A reference set of 32X carts run with pixels +
audio + working controller at native 59.92 Hz NTSC.

---

## ⬜ Phase 2 — Polish

- ⬜ **Per-system shader tweaks** — operator-driven shader-preset choice (per-system shader override shipped cross-system).
- ⬜ **Region quirks compatibility map** — operator-driven KNOWN_GAME_BUGS curation (per-game region override drawer shipped cross-system).
- ⬜ **32X-specific theming polish** — operator-driven UI polish (per-system theming infra shipped cross-system).

---

## ⬜ Phase 3+ — Stretch

32X-specific items:

- ⬜ **32X-CD games (Night Trap 32X, Corpse Killer 32X, Slam City)** — deferred (Phase 3+; needs stacked Sega CD + 32X end-to-end validation).
- ⬜ **Game Genie / Pro Action Replay code support** — operator-driven validation of PicoDrive's `retro_cheat_set`.
- ⬜ **Custom forked 32X core** — deferred.

---

## Scope clarifications

- **No vendoring for 32X today.** The libretro pivot means we ship
  the upstream nightly PicoDrive .dll alongside our binary.
- **No BIOS required** for cart-only 32X playback. PicoDrive
  synthesizes the SH-2 boot vector internally.
- **`.bin` extension intentionally excluded.** Same collision rationale
  Genesis uses; operators with `.bin` 32X dumps rename to `.32x`.
- **`.md` / `.smd` intentionally NOT cross-registered** for sega32x.
  Slug separation forces the right core selection (PicoDrive with
  32X mode, not ClownMDEmu with cart-only mode).
- **32X-CD games route through `segacd`** with a stacked core override
  pointing at PicoDrive. Phase 3+ work.
