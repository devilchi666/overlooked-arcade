# Vision

## The pitch in one line

The modern home for as much of retro gaming as we can host, starting with the consoles everyone else forgot.

## The pitch in one paragraph

Overlooked Arcade is a Rust + Tauri + wgpu emulator frontend for the cult and underserved consoles of gaming history — TurboGrafx-16, Atari Lynx, Atari 7800, SMS/Game Gear, MSX/MSX2, ColecoVision, Vectrex, Virtual Boy, WonderSwan. Where existing emulators bolt these systems onto a generic ROM-loader UI (or skip them entirely), Overlooked Arcade pairs **forked, modifiable C cores** with a **premium desktop shell** — per-system theming, real curation, save-state and library UX that's actually pleasant. The cores are battle-tested upstream; we own the forks so we can add features RetroArch leaves out. The shell is part of the product.

## Why this exists

The dominant retro systems already have great emulators: NES → Mesen, SNES → bsnes, Game Boy → SameBoy, Genesis → Genesis Plus GX. The accuracy fight on those systems has been won.

But the **long tail** — TurboGrafx-16, Lynx, MSX, Vectrex, Virtual Boy, WonderSwan — has been left to a small number of technically excellent but cosmetically punishing tools (Mednafen and friends). The libraries are stacked with games people *should* be playing in 2026, and most of them never will because the activation energy is too high.

Overlooked Arcade lowers that activation energy to zero, then makes the experience itself worth showing up for.

This project is **non-commercial**. It's a gift to the retro community.

## What makes it different

1. **Forked cores, not vendored.** We own and modify the C source. RetroArch ships vanilla; we ship better. Examples: rewind-scrubbing UI built on save-state ring buffer, memory inspector hooked into core RAM/VRAM peeks, TAS recording with deterministic replay, frame-by-frame WebM export.
2. **Premium desktop feel.** ~15-25 MB binary, sub-1s cold start (sub-500ms goal where measurement permits), polished UI with per-system theming — TG-16 orange/cream, Vectrex phosphor green, Virtual Boy red-on-black, etc.
3. **Advanced shaders from one WGSL pipeline.** CRT curvature, scanlines, phosphor decay, HDR-aware tone mapping, bezel overlays. wgpu translates to DX12/Vulkan/Metal/GL/WebGPU so one shader works everywhere.
4. **Curation as a feature.** Every system has a welcome screen with era context, recommended starting points, "if you've never played this, start here" paths.
5. **System-specific theming inside a shared modern UI.** No console dioramas; identity comes from typography, accent, era art.

## What it is not

- Not a cycle-accurate emulator. Existing tools win at accuracy. We win at experience.
- Not a frontend over libretro. We fork the cores, modify them in-tree, and ship them embedded in our binary.
- Not a competitor to Mesen/bsnes/SameBoy on the systems they already own. We don't need NES or SNES early — those audiences are served.
- Not commercial. Free, open-source, GPLv2.

## The systems lineup

### First wave — the documented "overlooked consoles" focus

In planned bring-up order, biased toward forked-core availability (Mednafen + Beetle modules cover the early ones; later systems lean on MAME):

1. **TurboGrafx-16 / PC Engine** (HuCard) — *bring-up complete 2026-05-15, Bonk's Adventure playable*
2. **Atari Lynx**
3. **Atari 7800** (with 2600 fallback later)
4. **Sega Master System / Game Gear**
5. **MSX / MSX2**
6. **ColecoVision**
7. **TurboGrafx-CD / PC Engine CD-ROM²** (CD expansion of #1)
8. **Vectrex** (vector graphics + colored overlays)
9. **Virtual Boy** (with optional real-VR mode)
10. **WonderSwan / WonderSwan Color**

### The bigger picture — long-term ambition

The first wave is the **starting point**, not the ceiling. Long-term goal is to host **as much of retro gaming as we can** — almost all major consoles, handhelds, and computer systems we can find good upstream cores for, plus cores we modify heavily or write ourselves for systems that need new work. Likely future additions (not exhaustive, not in order):

- **Cartridge consoles:** NES, SNES, Genesis/Mega Drive, Neo Geo (AES), Intellivision, Atari Jaguar, 3DO, Channel F, Odyssey²
- **CD/optical consoles:** Sega CD, 32X, Saturn, PlayStation, Neo Geo CD, PC-FX
- **Handhelds:** Game Boy / Color / Advance, Neo Geo Pocket / Color, Nintendo DS, PSP
- **Computers:** Amiga, ZX Spectrum, Commodore 64, MSX2+, Atari 8-bit, Amstrad CPC, Apple II, Sharp X68000, NEC PC-88/98, FM Towns
- **Arcade:** MAME-driven

System priority is driven by community interest, upstream-core quality, and user direction. Adding system N+1 follows a repeatable 8-step recipe (vendor → shim → wrapper → register → theme → docs); the trait, renderer, audio sink, and input layer need zero changes per system.

**Why the framing is "overlooked first":** the underserved end of the catalogue is the differentiator. The popular consoles already have great emulators; we don't need to ship NES or SNES early to justify the project. They join when we have bandwidth, not when we need the headline.

## Marketing beats

Each system gives us a launch beat:

- "Rondo of Blood, the way it was meant to be played."
- "Atari Lynx with a UI that doesn't look like 1998."
- "The MSX2 library Westerners never got to see — Konami's golden age."
- "Vectrex with the colored plastic overlays, restored."
- "Play Virtual Boy in actual VR for the first time."

## Target screenshot

A modern desktop app: TG-16 system page with a curated library — Lords of Thunder cover art prominent, recommended-starting-points strip across the top, era context in a sidebar. Click a game and it goes full-bleed with optional CRT post shader. Looks like something released in 2026, not 2003. Per-system theming makes each system page feel like its own place.
