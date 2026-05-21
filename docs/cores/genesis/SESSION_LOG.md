# genesis Session Log

Per-core Shipped / Almost / Next log for Sega Mega Drive / Genesis.
Project-wide log lives at `docs/SESSION_LOG.md`.

---

## 2026-05-19 — Phase 0 onboarding

- **Shipped:**
  - `oa_core::SystemId::Genesis` variant + `parse_system_id("genesis")` arm.
  - `bindings.rs::genesis` module — 10-button 6-button-MD layout
    (A/B/C + X/Y/Z + Start + Mode + d-pad), identity libretro remap,
    `GENESIS_BUTTONS` table, `default_genesis_bindings()`, all dispatch
    arms (`bit_for` / `buttons_for` / `to_libretro_bits` / `defaults_for`).
  - `default_core_dll_for_system("genesis") → "clownmdemu_libretro.dll"`
    in main.rs.
  - `media::repo_for_system_id` → `Sega_-_Mega_Drive_-_Genesis` for
    libretro-thumbnails cover sync.
  - `rom_hashes::libretro_dat_refs_for_system` → `metadat/no-intro/Sega - Mega Drive - Genesis`.
  - Frontend `themes/registry.ts` — `SystemId` union extended with
    `"genesis"`, `systemThemes.genesis` entry (extensions
    `["md", "smd", "gen", "68k"]`, landscape 4/3 tile, `crt-lite` default
    shader preset).
  - Frontend `themes/systems.css` — `[data-system="genesis"]` block,
    cobalt blue at hue 245° + chroma 0.22 (distinct from PCE-CD's
    cyan-blue at 220°).
  - Per-core docs scaffold (README + ROADMAP + this log +
    KNOWN_GAME_BUGS + DECISIONS).
- **Almost:** Phase 1 operator validation — needs a real `.md` ROM
  launched end-to-end (Sonic the Hedgehog, Streets of Rage 2,
  Phantasy Star IV are good test cases).
- **Next:** Operator drops `clownmdemu_libretro.dll` into
  `<exe_dir>/cores/`, scans a Genesis ROMs folder, confirms tiles appear
  with cobalt theme, and launches a known-good ROM. Once Phase 1 ✅,
  Phase 2 polish (3-button vs 6-button game map, MD glyphs) opens.
