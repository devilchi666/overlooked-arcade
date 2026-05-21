# gb Session Log

Per-core Shipped / Almost / Next log for Nintendo Game Boy / Game Boy
Color. Project-wide log lives at `docs/SESSION_LOG.md`.

---

## 2026-05-19 — Phase 0 onboarding

- **Shipped:**
  - `oa_core::SystemId::Gb` variant + `parse_system_id("gb" | "gbc" |
    "gameboy" | "game-boy" | "game-boy-color") → Gb` arm.
  - `bindings.rs::gb` module — 8-button NES-shape layout (4-way d-pad +
    A + B + START + SELECT), identity libretro remap, `GB_BUTTONS`
    table, `default_gb_bindings()`, all dispatch arms (`bit_for` /
    `buttons_for` / `to_libretro_bits` / `defaults_for`).
  - `default_core_dll_for_system("gb") → "gambatte_libretro.dll"`
    in main.rs.
  - `media::repo_for_system_id("gb") → "Nintendo_-_Game_Boy"` as
    primary cover repo; GBC-specific cover gap from
    `Nintendo_-_Game_Boy_Color` documented as a follow-up.
  - `rom_hashes::libretro_dat_refs_for_system("gb") → &[
    no-intro/Nintendo - Game Boy, no-intro/Nintendo - Game Boy Color]`
    — two DatRefs, merged into one local corpus by `fetch_and_parse_all`.
  - Frontend `themes/registry.ts` — `SystemId` union extended with
    `"gb"`, `systemThemes.gb` entry (extensions `["gb", "gbc"]`,
    portrait 3/4 tile, `crt-lite` default shader preset).
  - Frontend `themes/systems.css` — `[data-system="gb"]` block, muted
    DMG pea-green at hue 145° + chroma 0.13 (distinct from GG's bright
    yellow-green at 130° / 0.18 by both hue AND chroma).
  - Per-core docs scaffold (README + ROADMAP + this log +
    KNOWN_GAME_BUGS + DECISIONS).
- **Almost:** Phase 1 operator validation — needs real `.gb` + `.gbc`
  ROMs launched end-to-end (Tetris, Super Mario Land, Link's Awakening,
  Pokémon Red are good DMG cases; Pokémon Crystal, Link's Awakening
  DX, Wario Land 3 for CGB).
- **Next:** Operator drops `gambatte_libretro.dll` into
  `<exe_dir>/cores/`, scans a Game Boy ROMs folder, confirms tiles
  appear with the DMG pea-green theme, and launches a known-good ROM.
  Once Phase 1 ✅, Phase 2 polish opens (dedicated `lcd-handheld`
  shader preset, DMG palette presets, multi-repo cover sync).
