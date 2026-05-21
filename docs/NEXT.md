# Next — cross-system priority queue

What to ship next across the project, ordered by leverage. **Per-system status lives in `docs/cores/<id>/ROADMAP.md`** — this file is just the cross-system view of what to pick up next when you have a fresh session.

Each item: short scope, rough line estimate, gating (operator-driven / blocked on infra / ready to ship), where the work lives.

When you close an item, the matching PR also flips the corresponding `⬜` to `✅` in the relevant per-core ROADMAP — see CLAUDE.md "ROADMAP hygiene" for the policy.

---

## HIGH — ready to ship next

These are operator-independent and the infrastructure they sit on already exists.

(Empty as of the last audit pass. The HIGH batch ran 2026-05-21 and closed: disc-id extraction across all CD systems, multi-repo cover sync for gb + wonderswan, N64 .v64/.n64 byte-swap, N64 WASD analog default.)

When something lands in this bucket, name it concretely (`apps/oa-shell/src/<path>` + scope + estimate) so the next session can pick it up without re-deriving.

---

## MEDIUM — Phase 3+ polish

1. **Dedicated `vector-phosphor` shader preset for Vectrex** (~250 lines WGSL + UI).
   - New `ShaderPreset::VectorPhosphor` variant + WGSL branch. Gaussian glow on bright pixels (vector lines), optional persistence trail.
   - Wants operator design input on glow radius + persistence half-life.

2. **Dedicated `vb-monochrome` shader for Virtual Boy** (~120 lines WGSL).
   - New `ShaderPreset::VbMonochrome` variant. LED-grain noise + red-on-black tint + optional visor reflection.
   - Wants operator design input on noise intensity + grain pattern.

3. **Per-system `lcd-handheld` default binding** (~30 lines, ready to ship).
   - The shader preset exists (`ShaderPreset::LcdHandheld`, id 4). Wire `gb`/`gba`/`gamegear`/`ngp`/`wonderswan` to default to it via `frontend/src/themes/registry.ts` `defaultShaderPreset` slot.
   - Wants operator validation against real handheld captures first.

4. **Jaguar KP8–KP_HASH keyboard-passthrough dispatch** (~80 lines).
   - Bits 16-20 in `bindings.rs::jaguar` already exist. Wire libretro `RETRO_DEVICE_KEYBOARD` events from the upper-bit presses through to Virtual Jaguar.
   - Wants VJ-specific RETROK keycode validation against a running core.

5. **Multi-system light-gun smoke-test validation** (~160 lines harness).
   - POINTER device dispatch is shipped; per-system validation across dreamcast (House of the Dead), saturn (Virtua Cop), nes (Zapper), psx (Time Crisis) is pending. Mostly operator playtest — code work is a test harness.

---

## LOWER — operator-driven or Phase 3+ stretch

1. **SNES Mouse + Super Multitap** (~200 lines, niche). Mario Paint, Bomberman.
2. **O2 per-game keyboard-layout overlay UI** (~150 lines). Quest for the Rings overlays. Frontend image picker + in-game overlay surface.
3. **Vectrex translucent overlay rendering + aspect override** (~250 lines combined). Plastic color-strip per-game PNG; Vectrex CRT portrait 3:4 default.
4. **NDS microphone input** (~200 lines). Blow/voice puzzles. Deferred until operator playtest forces it.
5. **NDS per-game touch overlay UI** (~250 lines). Visual stylus cursor.
6. **NDS multi-touch** (~80 lines, niche). POINTER index 1+.
7. **Sega CD 3-button vs 6-button pad mode override** (~100 lines + DATA work).
8. **SMS Light Phaser** (~120 lines, shared light-gun infra).
9. **Genesis MD-specific button glyphs polish** (UI). A/B/C diamond + 6-button shoulder visualization.
10. **NGP-mono vs NGPC library-tile differentiation** (~60 lines). Badge or subtitle.
11. **PCFX FMV streaming validation** (operator). PC-FX is FMV-heavy.

---

## DEFERRED — blocked on shared infra not yet triggered

These wait for a single, larger infrastructure pass that benefits many systems at once. Each line item below names what unlocks the deferred work.

- **Phase 3 shared analog input infrastructure** (~1200 lines across `oa-input` + bindings schema + per-game profile UI). Blocks: 2600/5200/7800 paddle controllers, ChannelF 3-axis plunger, Intellivision 16-direction disc, GBA tilt sensor, GBA solar sensor, SMS Light Phaser, PSX/Saturn analog triggers, N64 Memory Pak + Rumble Pak, Dreamcast analog triggers, GameCube analog triggers, MAME Phase 3 (steering wheel + trackball + paddle).
- **System-agnostic cheat code path** (~300 lines). Once Phase 4 milestone pattern is more established.
- **GameCube Wii Remote / Nunchuk / Classic Controller dispatch** (~500 lines, new libretro device type, Phase 2.5).
- **Dreamcast VMU peripheral** (~400 lines, secondary screen + device dispatch).
- **Pressure-sensitive analog triggers** (~150 lines shared GC/PS2/DC).
- **Custom-built Vectrex vector renderer** (~500 lines, Phase 3+). Replace vecx raster with native wgpu vector-stroke rendering.
- **Modern VR for Virtual Boy via OpenXR** (~800 lines, Phase 2+). Side-by-side dual-perspective to a headset.
- **Right D-pad bindings for Virtual Boy** (~150 lines, OR wait for shared analog infra). Unlocks Mario Clash, VB Wario Land, Teleroboxer, Red Alarm, Vertical Force.
- **Jaguar CD support** (~300 lines, Phase 3). Separate load path + BIOS.
- **32X-CD games** (~300 lines, Phase 3+). Shared between sega32x + segacd.
- **ST-V arcade variant** of Saturn (~250 lines, Phase 3+). Separate `stv` slug.

---

## DOC / DATA / TRIAGE

Not code — content / curation / validation work.

- **KNOWN_GAME_BUGS triage** for each system once playtime surfaces real issues.
- **Per-game shader curation** — opinionated per-title defaults for known-quirky titles across all systems.
- **Region badges + publisher / developer logos** (already in `docs/PARKING_LOT.md`).
- **2600 homebrew / hack tile distinction** — per-game source-of-origin tag.
- **NEC PC-FX cover-art curation** — Japan-only library; titles ship Japanese by default and need operator-set English aliases for searchability.
- **MAME ROM-set name resolution** — per-game metadata sync against MAME listxml.

---

## Cross-system infrastructure inventory

What's already shipped that future work can lean on. Cite these in PRs that close per-core ROADMAP items.

- **Save states** — `oa_libretro::LibretroCore::save_state` / `load_state` + multi-slot UI + thumbnails (Phase 1.5 + Phase 4).
- **Rewind / TAS / video / memory inspector / milestones** — Phase 4 slices A-F.
- **Shader pipeline** — `crates/oa-render/src/lib.rs::ShaderPreset` (Plain / Scanlines / CrtLite / Phosphor / LcdHandheld) + `shaders/presets/*.preset.toml` + hot-reload + per-game/per-system override.
- **Per-system settings page** — slice 2.8.C. Closes per-system core override, shader, bloom, aspect, overscan, bezel, region/revision priority, rewind config, analog routing, keyboard passthrough.
- **Per-game settings drawer** — slice 2.8.D. All of the above stack on top per-game; plus `core_options` map, `patch_path`, `keypad_layout_note`.
- **Quick settings overlay** — slice 2.8.B. In-game pause menu.
- **Window + scaling modes** — Phase 2.
- **Aspect override** — `system_settings::default_display_aspect` + `SystemSettings.display_aspect_override` + `GameOverrides.display_aspect_override`. Per-system defaults: GBA → 1.5.
- **Audio device picker** — shipped.
- **Library scan + Import Wizard + watcher** — Phase 2.7.
- **SQLite library** — Phase 2.5.
- **Hash ROM identification** — `rom_hashes::resolve_rom_hashes_for_system`. `HeaderRule` extended with `ByteSwap` for N64 .v64/.n64 normalization.
- **Media sync** — `media::sync_media_for_system` + `repos_for_system_id` (multi-repo: gb DMG+CGB, wonderswan WS+WSC) + `repos_for_entry` (gamecube → GC/Wii classifier via `is_wii_dump`).
- **Core installer + buildbot catalog UI**.
- **BIOS pre-checks** — CD-launch dispatch covers 9 CD systems; cart-shape covers nds/neogeo/lynx/atari7800. Neogeo BIOS flavour-tagged stock vs Universe.
- **Keyboard passthrough** + Game-Focus toggle + Ctrl+G. Default-on for `mame`, `msx`, `msx2`, `5200`.
- **Analog axes** — `InputState.axes` + `compute_stick_output` with keyboard fallback + deadzone + sensitivity + per-system default routing (`default_analog_routing("n64") → WASD`).
- **POINTER device** — `oa_core::InputState.pointer` + `cb_input_state` POINTER dispatch + `InputPoller::poll_pointer` with `PointerViewport` (window-relative mapping fed from `Renderer::last_viewport()` per frame).
- **Direct-launch CLI** — `--system` / `--core` / per-game lookup + bootstrap-hint so the emu thread loads the right .dll on first launch.
- **Disc-id extraction** — `cd_id.rs::extractors` covers pce-cd, segacd, saturn, psx/ps2, neocd, pcfx, gamecube, dreamcast; 3DO returns None by design.
- **Per-system theming** — `frontend/src/themes/systems.css` + `registry.ts`.
- **Bindings UI** — `SystemBindingsEditor.tsx` renders button-name chips per system.
- **CJK font fallbacks** — `frontend/src/index.css::--font-display` covers PC-FX + FDS Japanese-only libraries.

When you add new cross-system infrastructure, append it here so the next session knows it can be leaned on.
