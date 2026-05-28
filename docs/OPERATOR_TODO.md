# Operator TODO

Generated **2026-05-27** by sweeping every per-core ROADMAP /
KNOWN_GAME_BUGS + the cross-cutting `NEXT.md` / `ACTIVE_WORK.md` /
`PARKING_LOT.md` / `DECISIONS.md` / `PLANS/` / `features/<name>/`
docs for items that need **the operator** to act before they can
close.

**Re-generate when**: per-core ROADMAPs drift far enough that this
list feels stale (typically every 4-6 sessions of new code work).
Re-run the same two `Agent`-led sweeps; this file is overwriteable.

**Categories** (high → low blocker impact):

- 🔑 **LEGAL** — Acquire BIOS / ROM legally (highest unblock per minute)
- 🟢 **GREENLIGHT** — Say "go" so multi-week code can start
- 📦 **CONTENT** — Curate / produce assets (audio, art, shaders)
- 🎮 **PLAYTEST** — Run a real session, flip a ⬜ ✅
- 💭 **DECISION** — Make a design / scope call
- 📝 **INPUT** — Provide a tuning parameter (shader values, etc.)

The "Quick wins" section at the top lists low-effort items so an
hour of operator time can knock several off. Estimated effort is
operator-time, not Claude-time.

---

## ⭐ Quick wins (≤30 min each, knock these out first)

- [ ] **Install ColecoVision BIOS** — drop `coleco.rom` (8 KB) in `<exe_dir>/system/`. Unblocks operator playtest for the entire Coleco library (~30 retail games). _Source: `docs/cores/coleco/ROADMAP.md:26` · 5 min after acquiring the file._
- [ ] **Install Intellivision BIOS pair** — drop `exec.bin` + `grom.bin` in `<exe_dir>/system/`. Both mandatory. Unblocks Astrosmash / Utopia / Star Strike playtest. _Source: `docs/cores/intv/README.md` · 5 min._
- [ ] **Install 5200 BIOS** — drop `5200.rom` (SHA-1 `6AD7A1E8C9FAD486FBEC9498CB48BF5BC3ADC530`) in `<exe_dir>/system/`. _Source: `docs/cores/5200/ROADMAP.md` · 5 min._
- [ ] **Drop a CC0 click sound at `<exe_dir>/assets/system-ui/_baseline/sounds/navigate.ogg`** — every system in the library plays it on DPad nav (per-system SFX bank from Per-System UI Stage 1 Slice 2). Sources: Freesound.org CC0 filter, Kenney.nl game packs. _Source: `docs/features/per-system-ui/ASSETS.md` · ~15 min curation + drop._
- [ ] **30-min smoke test: pick any one of the small-library systems** (5200, channelf, o2, vectrex, ngp, pokemini, wonderswan) and confirm a real ROM launches end-to-end. Flips a Phase 1 ⬜ → ✅ in one session. _Source: per-core ROADMAP for whichever system you pick · 30 min._

---

## 🔑 LEGAL — BIOS / ROM acquisition unblocks playtest

You've explicitly mentioned wanting to acquire these legally; the
code is shipped and waiting on the files.

- [ ] **Jaguar CD BIOS pair** — `jagboot.rom` + `jagcd.rom` in
  `<exe_dir>/system/`. Unblocks: `jagcd` Phase 1 playtest (Hover
  Strike: Unconquered Lands / Battlemorph). _Source:
  `docs/cores/jagcd/ROADMAP.md`._
- [ ] **Sega CD BIOS** — `bios_CD_U.bin` (US) / `bios_CD_J.bin` (JP)
  / `bios_CD_E.bin` (EU) in `<exe_dir>/system/`. Unblocks BOTH the
  `segacd` Phase 1 playtest AND the `sega32xcd` Phase 1 playtest
  (both reuse this BIOS). _Source: `docs/cores/segacd/ROADMAP.md` +
  `docs/cores/sega32xcd/ROADMAP.md`._
- [ ] **ST-V arcade ROM set + `stvbios.zip`** — Unblocks `stv` Phase
  1 playtest (Radiant Silvergun arcade original, Cotton 2, Steep
  Slope Sliders, Decathlete). MAME stv driver handles BIOS
  internally if `stvbios.zip` sits alongside the ROM zips. _Source:
  `docs/cores/stv/ROADMAP.md`._
- [ ] **NES FDS BIOS** — `disksys.rom` in `<exe_dir>/system/`. Only
  needed if you want to validate the Famicom Disk System (Doki
  Doki Panic, Zelda 2 JP, Metroid JP, etc.). _Source:
  `docs/cores/nes/ROADMAP.md:21`._
- [ ] **Saturn regional BIOSes** — `sega_100.bin` (JP v1.00) /
  `mpr-17933.bin` (US) / `mpr-19367b.bin` (EU PAL). Multi-region
  Saturn playtest needs at least the US dump. _Source:
  `docs/cores/saturn/ROADMAP.md`._
- [ ] **PSX regional BIOSes** — `scph5500.bin` (JP) / `scph5501.bin`
  (US) / `scph5502.bin` (EU). At minimum US. _Source:
  `docs/cores/psx/ROADMAP.md`._
- [ ] **3DO regional BIOS** — `panafz1.bin` (Panasonic FZ-1) /
  `panafz10.bin` (FZ-10) / `goldstar.bin` / `sanyotry.bin`. Any one
  unlocks Star Control II / Road Rash. _Source:
  `docs/cores/3do/ROADMAP.md`._
- [ ] **Dreamcast BIOS** — `dc_boot.bin` + `dc_flash.bin` in
  `<exe_dir>/system/`. _Source: `docs/cores/dreamcast/ROADMAP.md`._
- [ ] **PC-FX BIOS** — `pcfx.rom` (v1.00). Japan-only library so
  optional unless you want PC-FX playtest. _Source:
  `docs/cores/pcfx/ROADMAP.md`._
- [ ] **Neo Geo CD BIOS** — `neocd_z.rom` (top-loader, more common)
  or `neocd_t.rom` (front-loader). _Source: `docs/cores/neocd/ROADMAP.md`._
- [ ] **Neo Geo AES + MVS BIOS** — `neogeo.zip` in
  `<exe_dir>/system/` (FBNeo reads BIOS ROMs out of the zip).
  _Source: `docs/cores/neogeo/ROADMAP.md`._

---

## 🟢 GREENLIGHT — Say "go" to unblock multi-week code arcs

These are short conversations / decisions that unblock weeks of
queued work.

- [ ] **Per-System UI Stage 1 content production timeline.** Code
  framework Slices 1-5 are ✅ merged; Slices 6-9 (GB / NES /
  Vectrex pilot full builds + per-core README updates) are paused
  pending CC0 audio curation + AI-generated Vectrex blips + a GB
  DMG-greenish gradient PNG. Once content is in hand, code work
  resumes immediately. _Source: `docs/ACTIVE_WORK.md` "Per-System
  Custom UI Stage 1 — code arc complete; content-side pause" + the
  default-theme mockup at `docs/features/per-system-ui/assets/default-theme-mockup.png`._
- [ ] **Per-System UI vs Guided Setup ordering.** Both are
  multi-month arcs. Options: (a) finish Per-System UI Stage 1
  first (5-7w), then Guided Setup (8-10w); (b) Guided Setup
  first; (c) interleave per `NEXT.md` pipelined sequence.
  Currently Per-System UI Stage 1 is mid-flight; Guided Setup
  hasn't started. _Source: `docs/NEXT.md:174-199` "NEXT MAJOR ARC
  — Guided Setup" + `docs/NEXT.md:201-230` "NEXT MAJOR ARC —
  Per-System Custom UI"._
- [ ] **Confirm default-theme mockup as Stage 1+ design anchor.**
  The 2026-05-27 mockup at
  `docs/features/per-system-ui/assets/default-theme-mockup.png`
  is more ambitious than the current Stage 1 plan
  (`docs/PLANS/per-system-ui.md`). Needs a planning pass to
  re-slice the work against the mockup before Slice 6 starts.
  _Source: `docs/features/per-system-ui/assets/README.md`._

---

## 📦 CONTENT — Curate / produce assets

Operator-side content gathering. Most of this enables the
Per-System UI Stage 1 pilots.

### Per-System UI Stage 1 pilots

- [ ] **Game Boy SFX bank** — 5 short OGG files at
  `<exe_dir>/assets/system-ui/gb/sounds/` named `navigate.ogg`,
  `click.ogg`, `back.ogg`, `launch.ogg`, `boot-intro.ogg`. Soft
  handheld-feel sounds (taps, quiet chimes). CC0 from
  Freesound.org or original recordings. _Source:
  `docs/features/per-system-ui/ASSETS.md` pilot 1._
- [ ] **Game Boy background** — `default.png` (or `.webp`) at
  `<exe_dir>/assets/system-ui/gb/backgrounds/` — soft DMG-greenish
  gradient. ~1920×1080. _Source: same as above._
- [ ] **Game Boy boot-animation** — LCD fade-in CSS keyframes file
  at `<exe_dir>/assets/system-ui/gb/boot-animation/keyframes.css`.
  Short (~1s). _Source: same as above._
- [ ] **NES SFX bank** — toy-piano "boop" character, brighter than
  GB. Same 5 filenames at `system-ui/nes/sounds/`. _Source:
  `ASSETS.md` pilot 2._
- [ ] **NES animated background** — `animated.webm` (VP9 low
  bitrate) at `system-ui/nes/backgrounds/` — scrolling NES-palette
  pattern. _Source: same._
- [ ] **NES boot-animation** — `keyframes.css` for the quick
  zoom-in + palette flash (~800ms). _Source: same._
- [ ] **Vectrex SFX bank** — synthesized vector blips at
  `system-ui/vectrex/sounds/`. AI-generated (sfxr, ChipTone) or
  curated CC0. _Source: `ASSETS.md` pilot 3._
- [ ] **Vectrex shader background** — `shader.wgsl` at
  `system-ui/vectrex/backgrounds/` — phosphor-screen WGSL shader
  (low-intensity glow + scanline-blur). Technical bar is highest
  here; could pair with the MEDIUM-band `vector-phosphor` shader
  preset work. _Source: same._
- [ ] **Vectrex boot-animation** — `effects.wgsl` for vector
  lines drawing in (~1.5s). _Source: same._

### Other content tasks

- [ ] **Universal baseline click** — single CC0 click at
  `<exe_dir>/assets/system-ui/_baseline/sounds/navigate.ogg`.
  Listed under Quick Wins above; repeated here as the starting
  point for the Per-System UI content arc. _Source: `ASSETS.md`._
- [ ] **MAME ROM-set name resolution curation** — per-game
  metadata pass against MAME listxml so library tiles show
  human-readable titles instead of ROM-set zip names. _Source:
  `docs/NEXT.md:421-430` DOC / DATA / TRIAGE._
- [ ] **NEC PC-FX cover-art curation** — Japan-only library;
  titles ship Japanese by default and need operator-set English
  aliases for searchability. _Source: `docs/NEXT.md:421-430`._
- [ ] **2600 homebrew / hack tile distinction** — per-game
  source-of-origin tag so retail / homebrew / hack tiles read
  differently. _Source: `docs/NEXT.md:421-430`._

---

## 🎮 PLAYTEST — Validate shipped code on real hardware

Pick by interest. Each item flips a per-core ROADMAP Phase 1
bullet from ⬜ → ✅ when complete. Group by "BIOS install required"
vs "no BIOS" so you can plan a session.

### No BIOS install needed (most accessible)

- [x] **lynx** ✅ 2026-05-27 — real `.lnx` ROM end-to-end confirmed
  by operator. Multi-region (USA / Europe / Japan) testing still
  open in the lynx ROADMAP Phase 1.
- [ ] **2600** (~30 min) — Pitfall, Adventure, Combat. _Source:
  `docs/cores/2600/ROADMAP.md`._
- [ ] **ngp** (~30 min) — SNK vs Capcom, Sonic Pocket. NGP mono
  vs NGPC color auto-detection. _Source: `docs/cores/ngp/ROADMAP.md`._
- [ ] **wonderswan** (~30 min) — Final Fantasy I (mono + WSC).
  Vertical-rotation auto-handling. _Source: `docs/cores/wonderswan/ROADMAP.md`._
- [ ] **pokemini** (~30 min) — representative Pokémon Mini ROM
  set. _Source: `docs/cores/pokemini/ROADMAP.md`._
- [ ] **virtualboy** (~1 hour) — Mario's Tennis, Galactic Pinball.
  Try anaglyph + side-by-side 3D modes. _Source: `docs/cores/virtualboy/ROADMAP.md`._
- [ ] **n64** (~1-2 hours) — SM64, Ocarina of Time, GoldenEye,
  MK64. Multi-region (NTSC US/JP + PAL EU). Analog stick + C-stick
  with WASD/IJKL fallback. _Source: `docs/cores/n64/ROADMAP.md`._
- [ ] **gba** (~2 hours) — Minish Cap, Pokémon FR/LG/Emerald,
  Aria of Sorrow, Mother 3. Battery-save persistence. _Source:
  `docs/cores/gba/ROADMAP.md`._
- [ ] **gb / gbc** (~1-2 hours) — Tetris, Link's Awakening,
  Pokémon Red/Blue/Crystal, Wario Land 3. DMG + CGB auto-detect.
  _Source: `docs/cores/gb/ROADMAP.md`._
- [ ] **gamegear** (~30 min) — Sonic, Tails Adventure. _Source:
  `docs/cores/gamegear/ROADMAP.md`._
- [ ] **vectrex** (~30 min) — Mine Storm pack-in, Berzerk,
  Star Trek. _Source: `docs/cores/vectrex/ROADMAP.md`._
- [ ] **o2 (Odyssey²)** (~30 min) — KC Munchkin, Pick Axe Pete.
  Region auto-detect (NTSC vs PAL). _Source: `docs/cores/o2/ROADMAP.md`._
- [ ] **channelf** (~30 min) — Video Whizball, Spitfire. Plunger
  controller (D-pad + push/pull/twist axes). _Source: `docs/cores/channelf/ROADMAP.md`._
- [ ] **jaguar** (~1 hour) — Iron Soldier, Tempest 2000, Rayman.
  Numpad-using games (Iron Soldier weapon select). _Source:
  `docs/cores/jaguar/ROADMAP.md`._
- [ ] **mame** (~1 hour) — known-good ROM set against MAME 0.287.
  6-button SF mapping. Service/Tab/P2 buttons via keyboard
  passthrough. _Source: `docs/cores/mame/ROADMAP.md`._
- [ ] **psp** (~1 hour) — God of War: Chains of Olympus, Crisis
  Core, Patapon. _Source: `docs/cores/psp/ROADMAP.md`._
- [ ] **nds** (~1 hour) — NSMB DS, Mario Kart DS (button-only) +
  Phantom Hourglass, Brain Age, Picross DS (stylus). _Source:
  `docs/cores/nds/ROADMAP.md`._
- [ ] **dosbox** (~1 hour) — Doom, Wing Commander, X-COM, SimCity
  2000. Gamepad + mouse + keyboard passthrough + entry-point
  override. _Source: `docs/cores/dosbox/ROADMAP.md`._
- [ ] **scummvm** (~45 min) — Monkey Island, Day of the Tentacle,
  Lure of the Temptress (`.scummvm` descriptors). _Source:
  `docs/cores/scummvm/ROADMAP.md`._

### BIOS install needed (do after LEGAL checklist)

- [ ] **gamecube + Wii** (~evening) — Smash Melee (C-stick),
  Wind Waker, RE4, Metroid Prime, Pikmin. Wii Sports + Mario Kart
  Wii with the new Wii peripheral subclasses (513/769/1025/1281/1537
  selectable per-game in the Input dialog). _Source:
  `docs/cores/gamecube/ROADMAP.md`._
- [ ] **dreamcast** (~1 hour) — Sonic Adventure, Crazy Taxi, JSR,
  Power Stone, Soulcalibur. Plus House of the Dead 2 reload-by-
  aiming-off-screen now functional. _Source:
  `docs/cores/dreamcast/ROADMAP.md`._
- [ ] **ps2** (~1-2 hours) — Shadow of the Colossus, MGS2, GTA III,
  FFX. _Source: `docs/cores/ps2/ROADMAP.md`._
- [ ] **saturn** (~2 hours) — NiGHTS, Guardian Heroes, Radiant
  Silvergun, Saturn Bomberman. Multi-disc Panzer Dragoon Saga.
  Virtua Cop / Death Crimson 2 for Light Gun + IS_OFFSCREEN. _Source:
  `docs/cores/saturn/ROADMAP.md`._
- [ ] **psx** (~2 hours) — SotN, FF7 (3-disc `.m3u`), MGS (2-disc),
  Crash, RE. Time Crisis 1/2 / Lethal Enforcers for Light Gun +
  IS_OFFSCREEN reload-by-aim. _Source: `docs/cores/psx/ROADMAP.md`._
- [ ] **segacd** (~1-2 hours) — Sonic CD, Lunar, Snatcher, Popful
  Mail. CDDA streaming validation. _Source: `docs/cores/segacd/ROADMAP.md`._
- [ ] **sega32x** (~1 hour) — Knuckles' Chaotix, Virtua Racing
  Deluxe, Doom 32X, Star Wars Arcade. _Source: `docs/cores/sega32x/ROADMAP.md`._
- [ ] **genesis** (~1 hour) — Sonic, Streets of Rage 2, Phantasy
  Star IV, Gunstar Heroes. _Source: `docs/cores/genesis/ROADMAP.md`._
- [ ] **sms** (~1 hour) — Phantasy Star, Wonder Boy III, Sonic
  SMS. Light Phaser (Operation Wolf / Rambo III) with
  IS_OFFSCREEN now functional. _Source: `docs/cores/sms/ROADMAP.md`._
- [ ] **atari7800** (~1 hour) — Asteroids, Centipede, Ms Pac-Man,
  Choplifter, Robotron 2084 (twin-stick with port 0 + port 1 both
  Standard Pad). PAL set + POKEY audio. _Source:
  `docs/cores/atari7800/ROADMAP.md`._
- [ ] **coleco** (~30 min, after BIOS install) — Donkey Kong,
  Zaxxon, Lady Bug, Mouse Trap. Keypad bindings now visualizable
  via the per-game Input dialog's new visual keypad reference.
  _Source: `docs/cores/coleco/ROADMAP.md`._
- [ ] **intv** (~1 hour, after BIOS install) — Astrosmash,
  Utopia, Star Strike, B-17 Bomber. Side-button mapping per
  game. _Source: `docs/cores/intv/ROADMAP.md`._
- [ ] **5200** (~30 min, after BIOS install) — Star Raiders,
  Missile Command, Galaxian, Pac-Man. _Source: `docs/cores/5200/ROADMAP.md`._
- [ ] **neogeo** (~1 hour, after BIOS install) — Metal Slug,
  KOF, Samurai Shodown. .neo single-file + .zip ROM-set. _Source:
  `docs/cores/neogeo/ROADMAP.md`._
- [ ] **neocd** (~1 hour, after BIOS install) — Samurai Shodown
  RPG, Metal Slug 1 CD. CDDA validation. _Source: `docs/cores/neocd/ROADMAP.md`._
- [ ] **3do** (~1 hour, after BIOS install) — Star Control II,
  Road Rash, Need for Speed. _Source: `docs/cores/3do/ROADMAP.md`._
- [ ] **pcfx** (~30 min, after BIOS install) — Battle Heat. _Source:
  `docs/cores/pcfx/ROADMAP.md`._
- [ ] **pce-cd** (~1 hour) — Multi-disc Cosmic Fantasy 4 / Tengai
  Makyō II via `.m3u`. _Source: `docs/cores/pce-cd/ROADMAP.md`._
- [ ] **jagcd** (~30 min, after BIOS install) — Hover Strike:
  Unconquered Lands, Battlemorph. _Source: `docs/cores/jagcd/ROADMAP.md`._
- [ ] **sega32xcd** (~30 min, after BIOS install) — Night Trap 32X,
  Corpse Killer. _Source: `docs/cores/sega32xcd/ROADMAP.md`._
- [ ] **stv** (~1 hour, after BIOS install) — Radiant Silvergun
  arcade original, Cotton 2, Steep Slope Sliders. _Source:
  `docs/cores/stv/ROADMAP.md`._

### Cross-cutting playtest

- [ ] **Light-gun IS_OFFSCREEN end-to-end across all 6 systems** —
  the IS_OFFSCREEN reload-by-aim flag shipped 2026-05-27. Walk
  through NES (Zapper / Duck Hunt) + SMS (Light Phaser /
  Operation Wolf) + Saturn (Virtua Gun / Virtua Cop) + PSX (GunCon
  / Time Crisis) + Dreamcast (HotD / Confidential Mission) + Atari
  7800 (XEGS Light Gun / Sentinel). Walk through "aim off-screen,
  fire to reload" mid-game. ~2 hours total. _Source:
  `docs/NEXT.md:472-486` (POINTER + LIGHTGUN inventory)._

---

## 💭 DECISION — Make a design / scope call

- [ ] **Wonderswan pause button binding preference** — pick a
  hotkey + sound-volume button binding. _Source:
  `docs/cores/wonderswan/ROADMAP.md`._
- [ ] **MAME multi-game-per-zip handling workflow** — decide how
  the operator picks a specific game from a multi-game MAME ROM
  zip. _Source: `docs/cores/mame/ROADMAP.md:66`._
- [ ] **Channel F optional BIOS install UX** — optional
  `sl31253.bin` / `sl31254.bin` / `sl90025.bin` packaging
  decision. _Source: `docs/cores/channelf/ROADMAP.md`._
- [ ] **Vectrex optional BIOS packaging** — `vectrex.bin` for the
  era-correct boot screen + Mine Storm pack-in. Optional but
  recommended. _Source: `docs/cores/vectrex/ROADMAP.md`._
- [ ] **Intellivoice voice-module support path** — libretro mic
  device dispatch evaluation. _Source: `docs/cores/intv/ROADMAP.md`._
- [ ] **Videopac+ G7400 expansion curation** — operator decides
  whether to surface as a separate sidebar entry or stay under
  `o2`. _Source: `docs/cores/o2/ROADMAP.md`._
- [ ] **Theme ecosystem WAIT lock status check** — confirm the
  2026-05-25 DECISIONS G "WAIT until user demand" lock still
  holds. Gates the GPL-2.0 → permissive license pivot. _Source:
  `docs/DECISIONS.md` + `docs/PARKING_LOT.md`._
- [ ] **Kiosk shell scheduling** — repositioned post-Per-System UI
  pipeline. Confirm this order. _Source: `docs/NEXT.md:168-171`._

---

## 📝 INPUT — Tuning parameters

- [ ] **`vector-phosphor` shader** for Vectrex — glow radius +
  persistence half-life values. ~250 lines WGSL + UI. Pair with
  the Vectrex pilot shader background work in the Per-System UI
  arc. _Source: `docs/NEXT.md:277-292` MEDIUM #1._
- [ ] **`vb-monochrome` shader** for Virtual Boy — noise
  intensity + grain pattern values. ~120 lines WGSL. _Source:
  `docs/NEXT.md:277-292` MEDIUM #2._
- [ ] **Per-system bloom_amount tuning** — defaults to 0.6;
  operators may prefer system-specific values for cores like
  Genesis (lower bloom for sharp pixel art) vs Saturn (higher
  bloom for the era's TV-glow aesthetic). _Source:
  `docs/NEXT.md` infra inventory._
- [ ] **Tile flourish per-system tuning** — once Per-System UI
  Stage 1 ships content, the `interactionStyle` transition
  timing + hover transform scales benefit from operator
  interactive tuning per pilot system. _Source: `docs/NEXT.md`
  MEDIUM band._
- [ ] **Genesis MD-palette vs crt-lite preset decision** —
  per-system shader-default polish call. _Source:
  `docs/cores/genesis/ROADMAP.md`._
- [ ] **Saturn per-game Cart RAM** — 4MB / 1MB validation per
  Saturn game (Vampire Savior, X-Men vs Street Fighter, etc.).
  _Source: `docs/cores/saturn/ROADMAP.md`._

---

## Tracking

When you complete an item, edit this file (check the box, drop a
date in the bullet) and the matching per-core ROADMAP if relevant.
The per-core ROADMAPs are the authoritative status surface — this
file is a working checklist that compiles their pending items into
one scannable page.

Re-run the agent sweeps when this list starts feeling stale.
Command shape (paste into a fresh Claude session):

> "Send agents out to sweep per-core docs + cross-cutting docs for
> everything that needs me to test or input. Save to
> `docs/OPERATOR_TODO.md`."
