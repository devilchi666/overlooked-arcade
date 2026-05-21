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

- ⬜ Operator validation: launch a real `.32x` ROM end-to-end (pixels +
  audio + controller). Suggested reference set: **Knuckles' Chaotix**,
  **Virtua Racing Deluxe**, **Doom 32X**, **Star Wars Arcade**,
  **Kolibri**.
- ⬜ Save state F5/F8 round-trip. PicoDrive supports `retro_serialize`;
  twin-SH-2 state machinery is worth explicit smoke-testing.
- ⬜ Multi-region testing: load NTSC US + NTSC JP + PAL EU 32X carts to
  confirm region auto-detect (NTSC 59.92 Hz vs PAL 49.70 Hz timing).
- ⬜ Per-game cover sync via libretro-thumbnails — **infra ready 2026-05-20,
  needs operator validation.** Mapping `sega32x → Sega_-_32X` shipped in
  `media::repo_for_system_id`. Operator: run `Settings → Library → Sync
  media for Sega 32X` and confirm covers download.
- ⬜ Libretro-database hash matching — operator runs `Settings → Library
  → Identify ROMs` to confirm No-Intro SHA-1 lookup populates canonical
  titles + publishers + years.

**Acceptance gate:** A reference set of 32X carts run with pixels +
audio + working controller at native 59.92 Hz NTSC.

---

## ⬜ Phase 2 — Polish

- ⬜ **Per-system shader tweaks.** 32X games shipped on CRTs but the
  enhanced-mode 32X VDP framebuffer (320×224 with twin-SH-2 rendering)
  was visibly crisper than stock MD output — operator may want a
  per-system shader override that goes lighter on scanlines for 32X
  than for cart Genesis.
- ⬜ **Region quirks compatibility map.** A handful of 32X titles
  (Doom 32X with the patched NTSC-J build, Mortal Kombat II 32X PAL
  timing) misbehave with auto-region detect — when found, document
  in `KNOWN_GAME_BUGS.md` with per-game region override.
- ⬜ **32X-specific theming polish.** The neon orange palette ships
  v1; per-system page header art / sidebar icon / cart-art frame may
  need 32X-specific tweaks once we have titles in the library.

---

## ⬜ Phase 3+ — Stretch

Per the project ROADMAP, all post-Phase-3 work (rewind, TAS, WebM
export, memory inspector, cheats, milestones, run-ahead) is system-
agnostic and lights up automatically once the engine work ships.
32X-specific items:

- ⬜ **32X-CD games (Night Trap 32X, Corpse Killer 32X, Slam City).**
  These layer the 32X cart-slot addon ON TOP of Sega CD — they need
  both the Sega CD BIOS and the 32X .dll. Currently routed through
  `segacd` with a stacked per-game core override pointing at PicoDrive.
  Needs end-to-end testing because PicoDrive's CD-via-32X path is a
  unique combined-emulation surface. Phase 3+ work paired with Sega CD
  Phase 3+.
- ⬜ **Game Genie / Pro Action Replay code support** — runs through
  the libretro cheat path (project RetroArch parity slice 8); needs
  validation that PicoDrive's `retro_cheat_set` accepts 32X Game Genie
  format (the 32X memory map differs from cart MD).
- ⬜ **Custom forked 32X core** — extremely unlikely. PicoDrive's
  upstream is responsive; OA-specific extensions for a 36-game library
  are hard to justify.

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
