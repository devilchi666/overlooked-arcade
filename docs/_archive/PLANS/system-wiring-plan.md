# System Wiring Plan

Operational checklist of systems to onboard end-to-end (Phase 0 of the
project ROADMAP) along with the **recommended default libretro core**
for each. Wiring a system once unlocks every libretro core for that
system — operators swap defaults via per-system Settings → Cores or
per-game overrides. The recommended core is just the **fallback** that
loads when the operator does nothing.

Scope filter (set 2026-05-19): **home consoles + handhelds + ScummVM +
DOSBox**. Home computers (MSX/MSX2, Amiga, C64, ZX Spectrum, Atari 8-bit,
Sharp X68000, NEC PC-88/98, FM Towns, Apple II, etc.) are **deferred**
from this list — they need keyboard-passthrough hardening and per-system
BIOS/disk handling beyond the current console-shape recipe. MAME stays
on the wired list because it already shipped, but new arcade-only or
computer-only systems are out of this plan's scope.

Status legend: ✅ wired · 🟨 partial · ⬜ to wire.

---

## Already wired (38 systems)

| Slug | System | Default core (.dll) | Alternates already in catalog |
|---|---|---|---|
| ✅ `tg16` | TurboGrafx-16 / PC Engine (HuCard) | `mednafen_pce_fast_libretro.dll` (Beetle PCE Fast) | — |
| ✅ `pce-cd` | TurboGrafx-CD / PC Engine CD-ROM² | `mednafen_pce_fast_libretro.dll` (same — handles CD) | full Mednafen build for per-game fallback |
| ✅ `lynx` | Atari Lynx | `mednafen_lynx_libretro.dll` (Beetle Lynx / Handy) | — |
| ✅ `nes` | Nintendo Entertainment System / Famicom | `fceumm_libretro.dll` (FCEUmm) | Mesen, Nestopia |
| ✅ `snes` | Super Nintendo / Super Famicom | `snes9x_libretro.dll` (Snes9x) | bsnes, bsnes-hd, Snes9x 2010 |
| ✅ `atari7800` | Atari 7800 ProSystem | `prosystem_libretro.dll` (ProSystem) | a7800 |
| ✅ `mame` | Arcade (MAME) | `mame_libretro.dll` (latest MAME) | mame2003_plus, mame2010, fbneo |
| ✅ `genesis` | Sega Mega Drive / Genesis | `clownmdemu_libretro.dll` (ClownMDEmu) | Genesis Plus GX, PicoDrive, BlastEm |
| ✅ `segacd` | Sega CD / Mega-CD | `genesis_plus_gx_libretro.dll` (Genesis Plus GX — same .dll as `sms` / `gamegear`) | PicoDrive |
| ✅ `sega32x` | Sega 32X | `picodrive_libretro.dll` (PicoDrive — only mainstream libretro 32X core) | — |
| ✅ `saturn` | Sega Saturn | `mednafen_saturn_libretro.dll` (Beetle Saturn) | Kronos, YabaSanshiro |
| ✅ `psx` | Sony PlayStation | `mednafen_psx_hw_libretro.dll` (Beetle PSX HW) | Beetle PSX SW (catalog peer), SwanStation |
| ✅ `neogeo` | SNK Neo Geo (AES + MVS) | `fbneo_libretro.dll` (FBNeo) | — (MAME drives Neo Geo too at higher CPU cost) |
| ✅ `neocd` | SNK Neo Geo CD | `neocd_libretro.dll` (NeoCD) | — |
| ✅ `ngp` | SNK Neo Geo Pocket + Color | `mednafen_ngp_libretro.dll` (Beetle NeoPop) | — |
| ✅ `jaguar` | Atari Jaguar | `virtualjaguar_libretro.dll` (Virtual Jaguar) | — |
| ✅ `3do` | 3DO Interactive Multiplayer | `opera_libretro.dll` (Opera, formerly 4DO) | — |
| ✅ `pcfx` | NEC PC-FX | `mednafen_pcfx_libretro.dll` (Beetle PC-FX) | — |
| ✅ `n64` | Nintendo 64 | `mupen64plus_next_libretro.dll` (Mupen64Plus-Next) | parallel_n64 |
| ✅ `gamecube` | Nintendo GameCube + Wii | `dolphin_libretro.dll` (Dolphin) | — |
| ✅ `dreamcast` | Sega Dreamcast | `flycast_libretro.dll` (Flycast) | redream |
| ✅ `psp` | Sony PlayStation Portable | `ppsspp_libretro.dll` (PPSSPP) | — |
| ✅ `ps2` | Sony PlayStation 2 | `pcsx2_libretro.dll` (LRPS2) | — |
| ✅ `nds` | Nintendo DS | `melonds_libretro.dll` (melonDS) | desmume |
| ✅ `sms` | Sega Master System | `genesis_plus_gx_libretro.dll` (Genesis Plus GX) | PicoDrive |
| ✅ `gamegear` | Sega Game Gear | `genesis_plus_gx_libretro.dll` (Genesis Plus GX — same .dll as `sms`) | PicoDrive |
| ✅ `gb` | Game Boy / Game Boy Color | `gambatte_libretro.dll` (Gambatte — covers both DMG + CGB via one .dll) | SameBoy, TGB Dual |
| ✅ `gba` | Game Boy Advance | `mgba_libretro.dll` (mGBA) | VBA-Next, VBA-M |
| ✅ `2600` | Atari 2600 / VCS | `stella_libretro.dll` (Stella) | — |
| ✅ `5200` | Atari 5200 SuperSystem | `atari800_libretro.dll` (Atari800) | — |
| ✅ `pokemini` | Nintendo Pokémon Mini | `pokemini_libretro.dll` (PokeMini) | — |
| ✅ `coleco` | ColecoVision | `bluemsx_libretro.dll` (blueMSX) | gearcoleco |
| ✅ `intv` | Mattel Intellivision | `freeintv_libretro.dll` (FreeIntv) | — |
| ✅ `o2` | Magnavox Odyssey² / Videopac | `o2em_libretro.dll` (O2EM) | — |
| ✅ `channelf` | Fairchild Channel F | `freechaf_libretro.dll` (FreeChaF) | — |
| ✅ `vectrex` | GCE Vectrex | `vecx_libretro.dll` (vecx) | — |
| ✅ `virtualboy` | Nintendo Virtual Boy | `mednafen_vb_libretro.dll` (Beetle VB) | — |
| ✅ `wonderswan` | Bandai WonderSwan + WS Color | `mednafen_wswan_libretro.dll` (Beetle WonderSwan) | — |

---

## Wave 1 — VISION first-wave remainder (0 systems remaining — COMPLETE)

VISION's original "overlooked consoles" lineup is now FULLY WIRED.
SMS + Game Gear + ColecoVision shipped 2026-05-19. Vectrex + Virtual
Boy + WonderSwan shipped 2026-05-20. MSX/MSX2 lives in the deferred
home-computers bucket per the 2026-05-19 scope filter.

(See "Already wired" table above for all six entries.)

---

## Wave 2 — Sega family completion (0 systems remaining — COMPLETE)

`segacd` + `sega32x` shipped 2026-05-20; `saturn` shipped 2026-05-20
(paired with `psx`); `dreamcast` shipped 2026-05-20. **Sega family
fully wired:** SMS + Game Gear + Genesis + Sega CD + 32X + Saturn +
Dreamcast all live in OA. See "Already wired" table above.

---

## Wave 3 — Nintendo home post-SNES (0 systems remaining — COMPLETE)

`n64` + `gamecube` shipped 2026-05-20; both wired with the new
cross-cutting analog input infra (gamepad LeftStick + RightStick X/Y
flow through libretro RETRO_DEVICE_ANALOG dispatch). See "Already
wired" table above.

(Switch / Wii U / 3DS are deferred — Yuzu/Ryujinx aren't shipped through
libretro buildbot, Cemu isn't libretro, Citra is libretro but standalone
Citra-libretro is rarely updated. Pick those up when stable libretro
.dlls exist.)

---

## Wave 4 — Nintendo handhelds (0 systems remaining — COMPLETE)

`gb` and `gba` shipped 2026-05-19; `nds` shipped 2026-05-20 with new
RETRO_DEVICE_POINTER input infra (mouse-as-touch). See "Already
wired" table above.

---

## Wave 5 — Sony (0 systems remaining — COMPLETE)

`psx` shipped 2026-05-20 (paired with `saturn`); `psp` + `ps2`
shipped 2026-05-20 (paired with `nds`). See "Already wired" table.

---

## Wave 6 — Other consoles (0 systems remaining — COMPLETE)

`2600`, `intv`, `o2`, `channelf` shipped 2026-05-19; `5200` + `pokemini`
shipped 2026-05-20 evening (with BIOS infra pre-staged from the
2026-05-20 BIOS audit session). Wave 6 closes the "completionist"
console list.

(See "Already wired" table above for all six Wave 6 entries.)

(`jaguar` + `3do` + `pcfx` shipped 2026-05-20; see "Already wired"
table above. SNK family — `neogeo` + `neocd` + `ngp` — same day.)

(SNK family — `neogeo` + `neocd` + `ngp` — shipped 2026-05-20; see
"Already wired" table above.)

---

## Wave 7 — Engine cores (the operator-requested exceptions)

These aren't hardware systems — they emulate the software environments
that ran on PCs in the era. Treat them as their own slugs with their own
themes; the library tile shows the game name, not the hardware.

| Slug | System | Default core (.dll) | Notes |
|---|---|---|---|
| ⬜ `scummvm` | ScummVM (point-and-click adventures) | `scummvm_libretro.dll` (ScummVM) | Each game is a folder containing the game's data files plus a `.scummvm` shortcut file the core reads to identify the title. The library scanner needs to be taught about folder-as-game for this slug. No BIOS, but each game's data files are mandatory. Extensions: `.scummvm`. |
| ⬜ `dosbox` | DOS (DOSBox) | `dosbox_pure_libretro.dll` (DOSBox Pure) | Alternate: `dosbox_core_libretro.dll` (Core), `dosbox_svn_libretro.dll` (SVN). DOSBox Pure ships zip-based game packaging (drop a `.zip` containing the game's DOS folder + `dosbox.conf`) which fits OA's archive flow well. Per-game `dosbox.conf` for CPU cycles / GPU / mount tweaks. Extensions: `.exe`, `.com`, `.bat`, `.conf`, `.zip` (zip needs disambiguation against MAME). |

---

## Deferred — home computers + emerging consoles

Out of this plan's scope per the 2026-05-19 "consoles only" filter,
but called out so they don't disappear:

- **Home computers:** MSX / MSX2 (was in VISION first wave), Commodore 64,
  ZX Spectrum, Amiga (OCS/ECS/AGA), Atari 8-bit family, Sharp X68000,
  NEC PC-88 / PC-98, FM Towns, Apple II, Amstrad CPC. These need
  keyboard-passthrough (Phase 2 done for MAME, generalizes) plus per-system
  disk/cassette handling. Pick up after the console list is mostly complete.
- **Modern consoles:** Nintendo Switch (Yuzu/Ryujinx — not libretro yet),
  Wii U (Cemu — not libretro), 3DS (Citra-libretro exists but is rarely
  current). Add as upstream stability matures.
- **Light-pen / motion / VR-input platforms:** Saturn light gun, Wii Remote,
  Dreamcast VMU game-side, Virtual Boy real-VR mode. These all hit the
  same deferred analog-input work that Robotron 2084 (Atari 7800 twin-stick)
  is waiting on.

---

## Recipe per onboarding

Each ⬜ entry follows the **6-step post-libretro-pivot recipe** validated
across Lynx → NES → SNES → MAME → A7800 → Genesis. See the
`feedback_multi_core_architecture_ready` memory + the ROADMAP Phase 6+
section for the canonical list. Roughly:

1. Extend `SystemId` union + `systemThemes` entry in `frontend/src/themes/registry.ts`.
2. Add `[data-system="<id>"]` block to `frontend/src/themes/systems.css`.
3. Per-system button bits + `default_<sys>_bindings()` + `<sys>_to_libretro_bits` + dispatch arms in `apps/oa-shell/src/bindings.rs`.
4. `default_core_dll_for_system(id)` arm + `parse_system_id(s)` arm in `apps/oa-shell/src/main.rs`.
5. `media::repo_for_system_id` libretro-thumbnails mapping + `rom_hashes::libretro_dat_refs_for_system` arm.
6. Per-core docs at `docs/cores/<sys>/` (README + ROADMAP + SESSION_LOG + KNOWN_GAME_BUGS + DECISIONS) + switch `docs/ACTIVE_CORE.md`.

Time-budget validated by Lynx and Genesis: one session for Phase 0 of a
console-shape system. CD-shaped systems (segacd, saturn, dreamcast, psx,
3do, pcfx) add a BIOS-validation pass on top — plan ~1.5 sessions.
Touch / analog-input systems (DS, GBA-with-tilt-titles, 7800 Trak-Ball)
hit deferred work and shouldn't be picked up until that lands.

---

## Order-of-attack suggestion

Pick from this list in roughly the order below. SMS + GG + `gb` +
`gba` + `2600` + `coleco` + `intv` + `o2` + `channelf` shipped
2026-05-19; `vectrex` + `virtualboy` + `wonderswan` + `segacd` +
`sega32x` + `saturn` + `psx` + `neogeo` + `neocd` + `ngp` + `jaguar` +
`3do` + `pcfx` + `n64` + `gamecube` + `dreamcast` shipped 2026-05-20.
`psp` + `ps2` + `nds` also shipped 2026-05-20 (paired triple,
including new RETRO_DEVICE_POINTER infra for mouse-as-touch).
**VISION first-wave + Sega family + Sony family (PSX/PS2/PSP) +
SNK family + overlooked-console thesis + Nintendo home (SNES/N64/
GC+Wii) + Nintendo handhelds (GB/GBA/NDS) + completionist consoles
(5200 + pokemini) all complete.** Remaining:

1. **`scummvm` + `dosbox`** — engine cores; need folder-as-game
    scanner extension before they slot in cleanly.

2 systems remaining; 38 wired. **The original 34-system plan is
exceeded by 4 systems**, thanks to scope-expanded heavyweights
(N64 + GameCube + Wii combined slug + PS2 + Dreamcast) plus the
2026-05-20 evening pickup of 5200 + pokemini.
At Lynx/Genesis pace (one session per console-shape Phase 0, ~1.5 for CD-shape) plus
operator validation gaps, roughly a year of episodic onboarding. The
recipe is repeatable — most of this is operator-pace, not engineering-pace.
