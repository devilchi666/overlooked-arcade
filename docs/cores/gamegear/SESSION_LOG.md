# gamegear Session Log

Per-core Shipped / Almost / Next log for Sega Game Gear.
Project-wide log lives at `docs/SESSION_LOG.md`.

---

## 2026-05-19 — Phase 0 onboarding

- **Shipped:**
  - `bindings.rs::gamegear` module — 7-button GG layout (4-way d-pad +
    B1 + B2 + START), identity libretro remap, `GAMEGEAR_BUTTONS`
    table, `default_gamegear_bindings()`, all dispatch arms (`bit_for`
    / `buttons_for` / `to_libretro_bits` / `defaults_for`).
  - `default_core_dll_for_system("gamegear") → "genesis_plus_gx_libretro.dll"`
    in main.rs (`parse_system_id("gamegear" | "game-gear")` was
    already wired). One .dll services both `gamegear` and `sms`.
  - `media::repo_for_system_id` arm already present (pre-wired); test
    fixture bumped to include `gamegear` in the onboarded set.
  - `rom_hashes::libretro_dat_refs_for_system("gamegear") →
    metadat/no-intro/Sega - Game Gear`.
  - Frontend `themes/registry.ts` — `SystemId` union extended with
    `"gamegear"`, `systemThemes.gamegear` entry (extensions `["gg"]`,
    landscape 4/3 tile, `crt-lite` default shader preset).
  - Frontend `themes/systems.css` — `[data-system="gamegear"]` block,
    yellow-green at hue 130° + chroma 0.18 (GG launch packaging palette,
    no near-hue collisions).
  - Per-core docs scaffold (README + ROADMAP + this log +
    KNOWN_GAME_BUGS + DECISIONS).
- **Almost:** Phase 1 operator validation — needs a real `.gg` ROM
  launched end-to-end (Sonic the Hedgehog (GG), Shinobi, Tails Adventure,
  Streets of Rage GG port are good test cases).
- **Next:** Operator drops `genesis_plus_gx_libretro.dll` into
  `<exe_dir>/cores/` (one install services both `sms` and `gamegear`),
  scans a Game Gear ROMs folder, confirms tiles appear with the
  yellow-green theme, and launches a known-good ROM. Once Phase 1 ✅,
  Phase 2 polish opens (dedicated LCD shader preset, GG bezel art).
