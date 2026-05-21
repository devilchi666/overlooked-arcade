# gba Session Log

Per-core Shipped / Almost / Next log for Nintendo Game Boy Advance.
Project-wide log lives at `docs/SESSION_LOG.md`.

---

## 2026-05-19 — Phase 0 onboarding

- **Shipped:**
  - `oa_core::SystemId::Gba` variant + `parse_system_id("gba" |
    "game-boy-advance" | "gameboyadvance") → Gba` arm.
  - `bindings.rs::gba` module — 10-button layout (4-way d-pad + A + B +
    L + R + START + SELECT), identity libretro remap, `GBA_BUTTONS`
    table, `default_gba_bindings()`, all dispatch arms (`bit_for` /
    `buttons_for` / `to_libretro_bits` / `defaults_for`).
  - `default_core_dll_for_system("gba") → "mgba_libretro.dll"` in
    main.rs.
  - `media::repo_for_system_id("gba") → "Nintendo_-_Game_Boy_Advance"`.
  - `rom_hashes::libretro_dat_refs_for_system("gba") →
    metadat/no-intro/Nintendo - Game Boy Advance`. Onboarded-systems
    test fixtures bumped in both media + rom_hashes.
  - Frontend `themes/registry.ts` — `SystemId` union extended with
    `"gba"`, `systemThemes.gba` entry (extension `["gba"]`, portrait
    3/4 tile, `crt-lite` default shader preset per the handheld
    convention).
  - Frontend `themes/systems.css` — `[data-system="gba"]` block, deep
    indigo at hue 285° / lightness 0.55 / chroma 0.20. Sits between
    SNES 270° (L=0.62) and Lynx 290° (L=0.65) in hue, but the lightness
    axis separates the three.
  - Per-core docs scaffold (README + ROADMAP + this log +
    KNOWN_GAME_BUGS + DECISIONS).
- **Almost:** Phase 1 operator validation — needs real `.gba` ROMs
  launched end-to-end (Pokémon FireRed, Minish Cap, Metroid Zero
  Mission, Advance Wars are all good test cases).
- **Next:** Operator drops `mgba_libretro.dll` into `<exe_dir>/cores/`,
  scans a GBA ROMs folder, confirms tiles appear with the deep indigo
  theme, and launches a known-good ROM. Once Phase 1 ✅, Phase 2
  polish opens (dedicated `lcd-handheld` shader preset, per-system 3:2
  aspect override, BIOS pre-check for BIOS-required titles).
