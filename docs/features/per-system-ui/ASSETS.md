# Per-System UI — Asset Catalog

Operator-facing reference for where every sound, image, and shader
file lives. The Rust resolver is path-based — drop a file at the
right path with the right name and the per-system custom UI feature
picks it up on next launch. No registry edits, no rebuild.

Slices that ship the consumers wire the asset paths in:

- Slice 2 ✅ — Sounds (`<system>/sounds/`)
- Slice 3 — Backgrounds (`<system>/backgrounds/`)
- Slice 4 — Boot animations (`<system>/boot-animation/`)
- Slice 5 — Tile flourishes (in-config, not asset-bundled)
- Slices 6-8 — Pilot full builds populate every directory above for
  GB / NES / Vectrex specifically

---

## Root layout

```
<exe_dir>/assets/system-ui/
├── _baseline/           ← Universal fallback for systems without a
│   ├── sounds/             dedicated bank. Every system reaches here
│   ├── backgrounds/        when its own <systemId>/ doesn't have the
│   └── boot-animation/     file the consumer is asking for.
└── <systemId>/          ← Per-system bank. Per plan §8, the 3 Stage 1
    ├── sounds/             pilots (gb, nes, vectrex) populate every
    ├── backgrounds/        directory; the other 40 baseline systems
    └── boot-animation/     stay empty and inherit from _baseline.
```

`<exe_dir>` is whichever directory holds `oa-shell.exe`:

- **Installed builds:** the install root (e.g. `C:\Program Files\Overlooked Arcade\`).
- **Portable mode:** the directory containing `portable.txt`.
- **Dev (`cargo tauri dev`):** `target/debug/` — drop assets there for
  local testing.

Resolution cascade (Slice 2): operator override in
`SystemSettings.ui_sound_<event>` → per-system bundle →
`_baseline` → silence. Same shape applies to backgrounds + boot
animations as those slices land.

---

## System slugs

The `<systemId>` directory name MUST match `SystemId` in
`frontend/src/themes/registry.ts`. Current set (43 systems):

```
tg16, pce-cd, lynx, nes, snes, mame, atari7800, genesis, segacd,
sega32x, saturn, psx, neogeo, neocd, ngp, jaguar, 3do, pcfx, n64,
gamecube, dreamcast, psp, ps2, nds, sms, gamegear, gb, gbc, gba,
2600, 5200, coleco, intv, o2, channelf, vectrex, virtualboy,
wonderswan, pokemini, msx, msx2, scummvm, dosbox
```

Adding a new system to OA extends the `SystemId` union — the
`Record<SystemId, SystemUIConfig>` shape forces a matching
`systemUIConfigs` entry; the asset directory is optional (falls back
to `_baseline`).

---

## Sounds (Slice 2 ✅)

Path: `<exe_dir>/assets/system-ui/<systemId>/sounds/`

| Event | Filename | When it plays | Wired |
| --- | --- | --- | --- |
| `navigate` | `navigate.<ext>` | DPad cursor tile-to-tile move in the library grid | Slice 2 |
| `click` | `click.<ext>` | Tile picked / generic confirm | Slice 2 helper exists; library-grid wires `launch` only in v1 — sidebar / context menus wire `click` later |
| `back` | `back.<ext>` | B / cancel | Slice 2 helper exists; call sites wire as surfaces are touched |
| `launch` | `launch.<ext>` | Game starts loading | Slice 2 |
| `boot-intro` | `boot-intro.<ext>` | Boot animation accompaniment on system entry | Slice 4 |
| `boot-outro` | `boot-outro.<ext>` | Exit-system accompaniment | Stage 3 |
| `error` | `error.<ext>` | Error toast / failed action | Wired with the error toast slice (TBD) |
| `scroll-tick` | `scroll-tick.<ext>` | Fast-scroll repeat sub-tick (quieter than `navigate`) | Wired when DPad auto-repeat tuning lands |

**Extension priority order:** `ogg`, `opus`, `wav`, `mp3`, `flac`, `m4a`
(matches rodio's `symphonia-all` decoder set). The resolver picks the
first extension that exists; ship one format per event.

**Mouse vs gamepad in v1:** library-grid mouse interactions stay
silent — sounds are gamepad-centric for the couch-gamer primary
audience. Slices 6-8 may wire mouse paths if pilot playtest surfaces
the asymmetry.

---

## Backgrounds (Slice 3 — building now)

Path: `<exe_dir>/assets/system-ui/<systemId>/backgrounds/`

The `SystemUIConfig.background` enum picks which renderer path runs
for a system:

| Config value | Filename | Renderer | When wired |
| --- | --- | --- | --- |
| `static` | `default.{png,jpg,jpeg,webp}` (optional) | CSS image fill; falls back to a soft accent-color gradient driven by `systemThemes[id]` if no file is present | Slice 3 |
| `animated` | `animated.{webm,mp4}` | Looping `<video autoplay muted>` element behind the library | Slice 3 |
| `shader` | `shader.wgsl` | Shader-driven background (WebGPU canvas) | Slice 3 ships a fallback-to-static; Slice 8 (Vectrex) lands the real shader renderer |

`backgroundAsset` in `SystemUIConfig` lets a system specify a non-
default filename (e.g. for variants). Empty / null means use the
default filename above.

**Static gradient default:** When `background: "static"` and no
`default.*` file exists, the renderer falls back to a radial gradient
using `--color-system-accent` from the existing `systemThemes`
registry. This makes Slice 3 visually equivalent to today for every
system that doesn't ship a custom asset.

---

## Boot animations (Slice 4 — future)

Path: `<exe_dir>/assets/system-ui/<systemId>/boot-animation/`

| Filename | Purpose | Notes |
| --- | --- | --- |
| `keyframes.css` | CSS-keyframe-driven animation overlay on system entry | Default path |
| `effects.wgsl` | Shader-driven animation (optional, signature systems) | Vectrex uses this; most systems don't |

Slice 4 ships a sub-toggle "Boot animations" in Settings → Display
(visible only when "Per-system experiences" is ON). Honors
`prefers-reduced-motion` — shortcuts to a 200ms fade when the OS
preference is set. Skippable on any input.

---

## Tile flourishes (Slice 5 — future)

Not asset-bundled. Driven entirely by `SystemUIConfig`:

- `interactionStyle: "instant" | "delayed" | "physical"` — selection feel
- `tileShape: "auto" | "square" | "portrait-3:4" | ...` — geometry override
- `transitionTiming: "instant" | "fast" | "standard" | "slow" | "cinematic"` — motion budget

No file drops needed for Slice 5; tuning happens in the
`systemUIConfigs.ts` registry.

---

## Pilot system asset specs (Stage 1, plan §8)

Locked at planning time; per-pilot slices (6-8) populate these.

### Pilot 1 — Game Boy (`gb`, Slice 6)

Soft / minimal / personal. LCD-persistence feel.

```
gb/
├── sounds/
│   ├── navigate.ogg     ← soft "tap" (original recording or curated CC0)
│   ├── click.ogg        ← soft confirm
│   ├── back.ogg         ← soft cancel
│   ├── launch.ogg       ← short LCD-style chime
│   └── boot-intro.ogg   ← accompanies the LCD fade-in
├── backgrounds/
│   └── default.png      ← single soft DMG-greenish gradient
└── boot-animation/
    └── keyframes.css    ← LCD fade-in (~1s)
```

### Pilot 2 — NES (`nes`, Slice 7)

Classic / bright / instant. Toy-piano character.

```
nes/
├── sounds/
│   ├── navigate.ogg     ← toy-piano "boop"
│   ├── click.ogg        ← bright confirm
│   ├── back.ogg         ← bright cancel
│   ├── launch.ogg       ← palette-flash chime
│   └── boot-intro.ogg   ← accompanies the zoom-in
├── backgrounds/
│   └── animated.webm    ← subtle scrolling NES-palette pattern
└── boot-animation/
    └── keyframes.css    ← quick zoom-in (~800ms) with palette flash
```

### Pilot 3 — Vectrex (`vectrex`, Slice 8)

Vector phosphor signature. Synthesized blips. Custom-component
escape hatch (`customComponent: "vectrex"` in
`systemUIConfigs.ts`).

```
vectrex/
├── sounds/
│   ├── navigate.ogg     ← synthesized vector blip (AI-generated)
│   ├── click.ogg        ← louder vector pulse
│   ├── back.ogg         ← vector unwind
│   ├── launch.ogg       ← vector draw + bloom
│   └── boot-intro.ogg   ← vector lines drawing in
├── backgrounds/
│   └── shader.wgsl      ← phosphor-screen WGSL (low-intensity glow + scanline-blur)
└── boot-animation/
    └── effects.wgsl     ← vector-lines-draw-in WGSL animation (~1.5s)
```

### `_baseline` universal fallback

Stage 1 ships at minimum:

```
_baseline/
└── sounds/
    └── navigate.{ogg|opus|wav}  ← universal soft click (CC0)
```

The other 40 systems inherit from `_baseline` until a pilot or
showcase-tier slice extends them. Backgrounds + boot-animation are
optional under `_baseline`; the renderers fall back to CSS gradient /
no animation when the directory is empty.

---

## File size + bundling targets

From plan §9:

- **Sounds:** ≤500 KB per system, ≤100 KB per individual file.
- **Visuals (backgrounds):** ≤2 MB per system.
- **Total installer addition over 40 systems:** ≤100 MB.

Per-pilot systems will run near the upper end (full SFX bank +
custom background); baseline systems will be empty (~0 bytes
addition).

**Encoding recommendations:**

- Audio: OGG Vorbis q3-q5 (~64-96 kbps) for short SFX; opus q3-q5
  for higher-quality signature sounds.
- Static backgrounds: WebP 80% quality or PNG-optimized.
- Animated backgrounds: VP9 WebM at low bitrate (this is a subtle
  background loop, not a feature video).
- Boot-animation: CSS keyframes preferred over video for size +
  responsiveness; WGSL shaders for signature systems where the
  visual effect is the point.

---

## Licensing

Stage 1 assets MUST be CC0 (Creative Commons Zero) or original work
produced for OA. No theme-ecosystem submission flow on the desktop
normal version — every asset that ships with the installer is
owned, public domain, or CC0-licensed.

**Recommended CC0 sources:**

- [Freesound.org](https://freesound.org/) (filter: License → CC0)
- [Kenney.nl](https://kenney.nl/) (game asset packs, all CC0)
- [Sonniss GDC pack](https://sonniss.com/gameaudiogdc) (royalty-free
  for game use; check the year's specific license)
- [BBC Sound Effects Archive](https://sound-effects.bbcrewind.co.uk/)
  (mixed licensing; filter for usable subset)

**Image sources:**

- Operator's own captures / renders (preferred for hardware-accurate
  console aesthetics)
- [Unsplash](https://unsplash.com/) (Unsplash License — usable but
  prefer CC0 where possible)
- AI-generated with CC0-friendly tools for synthesized visuals
  (Vectrex shader patterns, etc.)

---

## Verification

After dropping an asset, the easiest verification is a `cargo tauri
dev` session — Slice 2's library-grid wiring plays `navigate.ogg`
on DPad moves, so a fresh CC0 click at
`target/debug/assets/system-ui/_baseline/sounds/navigate.ogg`
audibly confirms the cascade is working end-to-end.

For non-audio assets (Slices 3+), look for the system's background
to switch from the default accent gradient to the asset image / video
when you focus a tile from that system.

---

## Changelog

- 2026-05-26 — Slice 3 ships the backgrounds cascade. The
  `<systemId>/backgrounds/` directories are now consumed by the
  `<SystemBackground>` component. Static + animated paths live; the
  shader path falls back to static until Slice 8 (Vectrex pilot)
  ships the shader-driven render path.
- 2026-05-26 — Slice 2 ships the sounds cascade; this file initialized
  alongside it.
