# genesis — Roadmap

Per-core phase tracking for Sega Mega Drive / Genesis. Mirrors the
project-wide ROADMAP shape (Phase 0 = onboarded, Phase 1 = first ROM
running, Phase 2 = polish, Phase 3+ = shared infra) but scoped to MD.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-19)

Core comes online via the libretro pivot — no Rust crate vendoring.
ClownMDEmu installed by operator; OA wires the system into the existing
shell, scanner, bindings, library DB, and settings pipelines.

- ✅ System registered in `frontend/src/platform/themes/registry.ts` — `SystemId`
  union extended with `genesis`, `systemThemes.genesis` entry (extensions
  `md / smd / gen / 68k`, landscape tile aspect 4/3, `crt-lite` default
  shader preset for the period-correct CRT feel).
- ✅ Per-system palette in `frontend/src/platform/themes/systemPalettes.ts` —
  cobalt blue (hue 245°, chroma 0.22), distinct from PCE-CD's cyan-blue
  (220°). Lives in the typed `SYSTEM_PALETTES` map, injected as `[data-system]`
  CSS at boot.
- ✅ Per-system button bits + bindings in `apps/oa-shell/src/bindings.rs::genesis`
  — 10-button 6-button-Mega-Drive layout (A/B/C + X/Y/Z + Start + Mode +
  d-pad), `GENESIS_BUTTONS` table, `default_genesis_bindings()`,
  `defaults_for("genesis")` arm.
- ✅ `genesis_to_libretro_bits` identity remap (bits laid out to match
  libretro RetroPad positions directly).
- ✅ `bindings::to_libretro_bits` + `bit_for` + `buttons_for` dispatch
  arms include `"genesis"`.
- ✅ `default_core_dll_for_system("genesis") → "clownmdemu_libretro.dll"`
  in `apps/oa-shell/src/main.rs`.
- ✅ `parse_system_id("genesis") → SystemId::Genesis` (new variant on
  `oa_core::SystemId` enum) in `apps/oa-shell/src/main.rs`.
- ✅ `rom_hashes::libretro_dat_refs_for_system("genesis")` returns
  `&[DatRef { subdir: "metadat/no-intro", basename: "Sega - Mega Drive - Genesis" }]`.
- ✅ `media::repo_for_system_id("genesis")` returns
  `Some("Sega_-_Mega_Drive_-_Genesis")` so cover sync works as soon as
  the operator runs it.
- ✅ Catalog entry `clownmdemu_libretro` already present in
  `core_installer::CATALOG` (recommended=false; the multi-Sega `genesis_plus_gx_libretro`
  is recommended=true). Both .dll names valid; user picks via per-system Cores.
- ✅ Per-core docs scaffold (this directory).

**Acceptance gate:** Operator can drop `clownmdemu_libretro.dll` into
the install, scan a Genesis ROMs folder, see Genesis-themed (cobalt blue)
tiles appear in the library, and click one to launch — without rebuilding
Rust.

---

## ⬜ Phase 1 — First Genesis ROM running

- ⬜ Operator validation: **Sonic the Hedgehog**, **Streets of Rage 2**, **Phantasy Star IV**, **Gunstar Heroes** — operator playtest.
- ✅ Save state F5/F8 round-trip — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ⬜ Multi-region testing — operator playtest (NTSC US/JP + PAL EU).
- ✅ Per-game cover sync via libretro-thumbnails — closed by cross-system media sync (`media::sync_media_for_system`).
- ✅ Libretro-database hash matching — closed by cross-system hash ID (`rom_hashes::resolve_rom_hashes_for_system`).
- ⬜ SMD-format dump validation — operator spot-check that ClownMDEmu deinterleaves `.smd` transparently.

**Acceptance gate:** A reference set of Genesis games run with pixels +
audio + working controller at native 59.92 Hz NTSC.

---

## ⬜ Phase 2 — Polish

- ⬜ Per-system shader tweaks — operator-driven MD-palette vs crt-lite validation (shader pipeline + per-system override shipped cross-system).
- ⬜ 3-button vs 6-button game compatibility map — operator-driven KNOWN_GAME_BUGS curation.
- ✅ MD-specific glyphs for the bindings UI — `frontend/src/platform/components/GenesisPadReference.tsx` renders the physical 6-button pad layout (X-Y-Z above A-B-C with D-pad + Mode + Start to the side) with each face button labeled by its current keyboard / gamepad binding. Mounted in both `engine/SystemBindingsEditor.tsx` (per-system Bindings dialog) and `frontend/src/platform/components/GameDialogs.tsx` per-game Input dialog via the shared `GENESIS_SYSTEMS` set so all four Genesis-family slugs (genesis / segacd / sega32x / sega32xcd) pick it up. Shipped 2026-06-01.
- ⬜ ClownMDEmu vs Genesis Plus GX vs PicoDrive comparison — operator-driven DECISIONS doc.

---

## ⬜ Phase 3+ — Stretch

Genesis-specific items:

- ✅ Sega CD path as separate `segacd` slug — shipped (segacd onboarded with own ROADMAP).
- ✅ 32X path as separate `sega32x` slug — shipped (sega32x onboarded with own ROADMAP).
- ⬜ Game Genie / Pro Action Replay code support — operator-driven validation of ClownMDEmu's `retro_cheat_set`.
- ⬜ Custom forked Mega Drive core — deferred.

---

## Scope clarifications

- **No vendoring for Genesis today.** The libretro pivot means we ship
  the upstream nightly .dll alongside our binary and tell operators to
  drop it into `<exe_dir>/cores/`. If we ever modify the core, we
  maintain a separate libretro-frontend build of our patched source —
  see project `DECISIONS.md` 2026-05-16 entry.
- **No BIOS required.** Mega Drive cart playback is BIOS-free
  (TMSS is internal to cores). Sega CD / 32X add-ons would need BIOSes
  in `<exe_dir>/system/`; those live behind future slugs, not this one.
- **`.bin` extension intentionally excluded** from the genesis registry
  entry to avoid collision with PCE-CD track files. Users with `.bin`
  MD dumps rename to `.md` — same as the Atari 7800 `.bin` → `.a78`
  rename convention.
