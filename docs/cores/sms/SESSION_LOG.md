# sms Session Log

Per-core Shipped / Almost / Next log for Sega Master System.
Project-wide log lives at `docs/SESSION_LOG.md`.

---

## 2026-05-19 — Phase 0 onboarding

- **Shipped:**
  - `bindings.rs::sms` module — 7-button SMS layout (4-way d-pad + B1 +
    B2 + PAUSE), identity libretro remap, `SMS_BUTTONS` table,
    `default_sms_bindings()`, all dispatch arms (`bit_for` /
    `buttons_for` / `to_libretro_bits` / `defaults_for`).
  - `default_core_dll_for_system("sms") → "genesis_plus_gx_libretro.dll"`
    in main.rs (`parse_system_id("sms")` was already wired).
  - `media::repo_for_system_id` arm already present (pre-wired); test
    fixture bumped to include `sms` in the onboarded set.
  - `rom_hashes::libretro_dat_refs_for_system("sms") →
    metadat/no-intro/Sega - Master System - Mark III`.
  - Frontend `themes/registry.ts` — `SystemId` union extended with
    `"sms"`, `systemThemes.sms` entry (extensions `["sms"]`, landscape
    4/3 tile, `crt-lite` default shader preset).
  - Frontend `themes/systems.css` — `[data-system="sms"]` block, neon
    magenta at hue 340° + chroma 0.22 (distinct from every other
    claimed hue; closest neighbor is NES 28° at ~48° distance).
  - Per-core docs scaffold (README + ROADMAP + this log +
    KNOWN_GAME_BUGS + DECISIONS).
- **Almost:** Phase 1 operator validation — needs a real `.sms` ROM
  launched end-to-end (Alex Kidd in Miracle World, Phantasy Star,
  Wonder Boy III: The Dragon's Trap, Sonic the Hedgehog (SMS) all
  good test cases).
- **Next:** Operator drops `genesis_plus_gx_libretro.dll` into
  `<exe_dir>/cores/` (one install services both `sms` and `gamegear`),
  scans an SMS ROMs folder, confirms tiles appear with neon-magenta
  theme, and launches a known-good ROM. Once Phase 1 ✅, Phase 2
  polish opens (Japan-region FM sound surface, optional BIOS handling).
