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

- ✅ System registered in `frontend/src/themes/registry.ts` — `SystemId`
  union extended with `genesis`, `systemThemes.genesis` entry (extensions
  `md / smd / gen / 68k`, landscape tile aspect 4/3, `crt-lite` default
  shader preset for the period-correct CRT feel).
- ✅ Theme block in `frontend/src/themes/systems.css` — cobalt blue
  (hue 245°, chroma 0.22), distinct from PCE-CD's cyan-blue (220°).
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

- ⬜ Operator validation: launch a real `.md` ROM end-to-end (pixels +
  audio + controller). Suggested reference set: **Sonic the Hedgehog**,
  **Streets of Rage 2**, **Phantasy Star IV**, **Gunstar Heroes**.
- ⬜ Save state F5/F8 round-trip confirmation. ClownMDEmu supports
  `retro_serialize`; should work via the existing path but needs live
  validation.
- ⬜ Multi-region testing: load USA + Europe + Japan ROM dumps to confirm
  region auto-detect works (NTSC 59.92 Hz vs PAL 49.70 Hz timing).
- ⬜ Per-game cover sync via libretro-thumbnails — **infra ready 2026-05-19,
  needs operator validation.** Mapping `genesis → Sega_-_Mega_Drive_-_Genesis`
  shipped in `apps/oa-shell/src/media.rs::repo_for_system_id`. Operator:
  run `Settings → Library → Sync media for Genesis` and confirm covers
  download.
- ⬜ Libretro-database hash matching — same — operator runs
  `Settings → Library → Identify ROMs` to confirm No-Intro SHA-1 lookup
  populates canonical titles + publishers + years.
- ⬜ SMD-format dump validation — drop a `.smd` file (interleaved Super
  Magic Drive format), confirm ClownMDEmu deinterleaves it transparently.

**Acceptance gate:** A reference set of Genesis games run with pixels +
audio + working controller at native 59.92 Hz NTSC.

---

## ⬜ Phase 2 — Polish

- ⬜ Per-system shader tweaks: MD games shipped on CRTs with visible
  scanlines; `crt-lite` is the registered default but operator may want
  to confirm it reads correctly against the saturated MD palette
  (Sonic's hyper-blue, Streets of Rage's neon).
- ⬜ 3-button vs 6-button game compatibility map. A handful of titles
  (Sonic 3D Blast, World of Illusion, Mortal Kombat 3 Ultimate's MD
  port) misbehave with 6-button pad announce — when found, document in
  `KNOWN_GAME_BUGS.md` with per-game pad-mode override via the per-game
  settings drawer's Input tab.
- ⬜ MD-specific glyphs for the bindings UI (A/B/C diamond + 6-button
  shoulder triplet visualization).
- ⬜ ClownMDEmu vs Genesis Plus GX vs PicoDrive comparison: validate
  swapping cores via the per-system Cores override works without binding
  drift. Document the practical differences in `DECISIONS.md`.

---

## ⬜ Phase 3+ — Stretch

Per the project ROADMAP, all post-Phase-3 work (rewind, TAS, WebM export,
memory inspector, cheats, milestones, run-ahead) is system-agnostic and
lights up automatically once the engine work ships. Genesis-specific
items:

- ⬜ Sega CD path as separate `segacd` slug. Different libretro core
  (`genesis_plus_gx_libretro` or `picodrive_libretro`), BIOS-required,
  shares the MD controller convention. Out of scope until MD itself is
  validated.
- ⬜ 32X path as separate `sega32x` slug. Same considerations as Sega CD.
- ⬜ Game Genie / Pro Action Replay code support — runs through the
  libretro cheat path (project RetroArch parity slice 8); needs validation
  that ClownMDEmu's `retro_cheat_set` accepts Genesis Game Genie format.
- ⬜ Custom forked Mega Drive core — only if upstream regresses or we
  want OA-specific extensions. Recipe mirrors the Beetle PCE Fast plan:
  separate libretro-frontend build of our patched source that emits a
  .dll we ship in the installer.

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

---

## 2026-05-21 — Stale-cleanup audit

The Phase 1+ items above were written when this system onboarded, before cross-system infrastructure (Phases 1.5 / 2.5–2.8 / 3 / 4 + direct-launch CLI) landed. Many `⬜` items are actually shipped — see `docs/cores/AUDIT_2026-05-21.md` for the per-item breakdown (stale vs open-code vs open-operator) for this system.
