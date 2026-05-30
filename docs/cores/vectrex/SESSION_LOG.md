# vectrex Session Log

Per-core Shipped / Almost / Next log for GCE Vectrex. Project-wide log lives at `docs/SESSION_LOG.md`.

---

## 2026-05-29 — `vector-phosphor` shader preset (Phase 2 polish)

Closes the Phase 2 ROADMAP item dedicated to giving the Vectrex its
own visual identity. Branch `feat/vectrex-vector-phosphor-shader`,
2 phase commits.

- **Shipped (P1 `7509ee1` + P2 `e23f74c`):**
  - New `ShaderPreset::VectorPhosphor` variant (id=5; `is_multipass`
    + new `uses_persistence` both return true).
  - `crates/oa-render/shaders/vector_blur.wgsl` — 9-tap σ≈2.5
    separable Gaussian with a luminance bright-pass on the H pass
    (smoothstep over the 0.5 threshold band; default tuned for
    Vectrex's pure-black background). V pass blurs the already-
    filtered output without re-thresholding.
  - `crates/oa-render/shaders/persistence.wgsl` — ping-pong
    accumulator that reads (current_glow, history_prev) and writes
    history_curr = current + history_prev * 0.866. The 0.866 decay
    constant gives a ~80ms half-life at 60fps; vector strokes leave
    a visible echo for ~5-10 frames.
  - Renderer extensions: `history_textures` pair allocated lazily,
    cleared to black on (re)alloc, ping-ponged each frame via
    `history_write_index`; `persistence_pass` built lazily when the
    preset becomes VectorPhosphor; final-blit secondary input
    samples history[just_written] when persistence is active.
  - `blit.wgsl` gains a preset_id == 5 branch that composites
    `source + glow * bloom_amount` additively (clamped at 1.5).
  - `shaders/presets/vector-phosphor.preset.toml` shipped (default
    bloom_amount = 1.0).
  - `themes/registry.ts`: vectrex `defaultShaderPreset` flipped
    `crt-lite` → `vector-phosphor`.
  - Frontend FALLBACK_PRESETS gains both vector-phosphor + the
    missing lcd-handheld entry.

- **Almost:** Operator playtest. Drop a Vectrex .vec ROM in (e.g.
  Mine Storm, Berzerk, Star Castle) and verify the halo + ghosting
  reads on dark-mostly screens. Tune `bloom_amount` per taste via
  the existing per-system bloom slider; per-system override saved
  via SystemSettings.

- **Next:** Phase 2 — translucent overlay rendering for per-game
  plastic-color-strip recreation. Independent feature; shader work
  doesn't gate it.

---

## 2026-05-20 — Phase 0 onboarding

- **Shipped:** `bindings.rs::vectrex` 8-button module (D-pad + B1/B2/B3/B4 in a horizontal-row 4-face-button layout), identity remap, all dispatch arms. `default_core_dll_for_system("vectrex") → "vecx_libretro.dll"`. `rom_hashes::libretro_dat_refs_for_system` arm. Frontend `systemThemes.vectrex` (extensions `["vec", "gam"]`, landscape 4/3, crt-lite) + `[data-system="vectrex"]` block (bright phosphor-green 165° / L=0.80 / C=0.16 — period-correct for the vector-display CRT). Per-core docs scaffold.
- **Almost:** Phase 1 operator validation. Mine Storm (BIOS pack-in), Berzerk, Star Trek good test cases.
- **Next:** Operator installs `vecx_libretro.dll` (+ optional `vectrex.bin` BIOS for Mine Storm pack-in), scans Vectrex folder, confirms phosphor-green themed tiles, launches a known-good ROM.
