# virtualboy Session Log

Per-core Shipped / Almost / Next log for Nintendo Virtual Boy. Project-wide log lives at `docs/SESSION_LOG.md`.

---

## 2026-05-30 — `vb-monochrome` shader preset (Phase 2 polish)

Closes the Phase 2 ROADMAP item for VB's dedicated shader.

- **Shipped (`addd47d`):** New `ShaderPreset::VbMonochrome` (id=6).
  Single-pass — branches in `blit.wgsl` after vector-phosphor (id=5).
  Three layered effects keyed off the operator design locked
  2026-05-30: pure-red palette enforcement (crushes any residual G/B
  out of the framebuffer so the era-correct monochrome look stays
  guaranteed); vertical scanline darken at the source-column rate
  (~0.82× on odd columns, mimicking the VB's spinning-mirror LED
  column scanner); soft circular vignette via smoothstep falloff
  (1.0 at center → ~0.7 at the corner, selling the headset
  eyepiece framing without obscuring gameplay). New
  `shaders/presets/vb-monochrome.preset.toml` (no tunable params
  today; defaults baked in WGSL). BUILTIN_PRESETS bumped 6→7 with
  test count updates. `themes/registry.ts` virtualboy
  `defaultShaderPreset` flipped `plain` → `vb-monochrome`. 506
  oa-shell tests stay green; frontend typecheck clean.

- **Note:** This session originally planned a "VB completion pack"
  (shader + Right D-pad bindings). The Right D-pad bindings turned
  out to be already shipped 2026-05-24 via the shared analog-routing
  infra (`bindings::analog_sticks_for("virtualboy")` returns Dual
  panels; `system_settings::default_analog_routing("virtualboy")`
  provides Numpad 8/2/4/6 keyboard fallback + right gamepad stick).
  A duplicate implementation via high bits 16-19 was written and
  reverted on the branch before push. Reinforces the audit-code-
  before-recommending lesson — the ROADMAP entry was accurate.

- **Almost:** Phase 1 operator playtest of the shader. Drop a VB ROM
  (Mario's Tennis, Galactic Pinball, VB Wario Land) and verify the
  vertical scanlines + vignette + red enforcement read on real
  content. Tune vignette strength via `vb-monochrome.preset.toml` if
  the corner falloff feels off.

- **Next:** Phase 2 remaining items — color-tinting palette options
  (Beetle VB Core-Option curation) + the deferred OpenXR VR support.

---

## 2026-05-20 — Phase 0 onboarding

- **Shipped:** `bindings.rs::virtualboy` 10-button module (LEFT D-pad + A + B + L + R + START + SELECT). `default_core_dll_for_system("virtualboy") → "mednafen_vb_libretro.dll"`. `rom_hashes` arm. Frontend `systemThemes.virtualboy` (extension `["vb"]`, landscape 4/3, **`plain` shader** — see DECISIONS) + `[data-system="virtualboy"]` block (deep VB red 7° / L=0.55 / C=0.26 — period-correct LED red, distinct from MAME scarlet + NES red by lightness + chroma). Per-core docs.
- **Almost:** Phase 1 operator validation — single-D-pad VB titles (Mario's Tennis, V-Tetris, Wario Cruise). Anaglyph 3D mode spot-check via Beetle VB core options.
- **Next:** Operator installs `mednafen_vb_libretro.dll` (no BIOS needed — VB never had one), scans VB folder, confirms VB-red themed tiles, launches a known-good single-D-pad ROM. Dual-D-pad games (Mario Clash, Wario Land VB, Teleroboxer, Red Alarm, Vertical Force) playable single-D-pad-only until Phase 2 right-D-pad work lands.
