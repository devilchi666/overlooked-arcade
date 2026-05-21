# 2600 Session Log

Per-core Shipped / Almost / Next log for Atari 2600 / VCS. Project-wide
log lives at `docs/SESSION_LOG.md`.

---

## 2026-05-19 — Phase 0 onboarding

- **Shipped:**
  - `oa_core::SystemId::Atari2600` variant + `parse_system_id("2600" |
    "atari2600" | "vcs") → Atari2600` arm.
  - `bindings.rs::atari2600` module — 7-button layout (4-way d-pad +
    FIRE + SELECT + RESET), identity libretro remap, `ATARI2600_BUTTONS`
    table, `default_atari2600_bindings()`, all dispatch arms (`bit_for`
    / `buttons_for` / `to_libretro_bits` / `defaults_for`) keyed by
    string `"2600"`.
  - `default_core_dll_for_system("2600") → "stella_libretro.dll"` in
    main.rs.
  - `media::repo_for_system_id("2600") → "Atari_-_2600"`.
  - `rom_hashes::libretro_dat_refs_for_system("2600") →
    metadat/no-intro/Atari - 2600`. Both onboarded-systems test
    fixtures bumped to include `"2600"`.
  - Frontend `themes/registry.ts` — `SystemId` union extended with
    `"2600"`; `systemThemes["2600"]` entry (extensions `["a26"]`,
    portrait 3/4 tile, `crt-lite` default shader preset).
  - Frontend `themes/systems.css` — `[data-system="2600"]` block,
    muted wood-grain brown at hue 60° + chroma 0.07 (decisively
    distinct from TG-16's bright orange 55°/0.18 by chroma).
  - First single-button system on the lineup: documented the
    `z_is_the_primary_action_button_on_every_system` test exception
    + added the Z=FIRE assertion to `defaults_cover_every_2600_button`.
  - Per-core docs scaffold (README + ROADMAP + this log +
    KNOWN_GAME_BUGS + DECISIONS).
- **Almost:** Phase 1 operator validation — needs a real `.a26` ROM
  launched end-to-end (Adventure, Pitfall!, Yars' Revenge, River
  Raid, Asteroids all good joystick-only test cases).
- **Next:** Operator drops `stella_libretro.dll` into
  `<exe_dir>/cores/`, scans a 2600 ROMs folder (or configures a
  `*.bin → 2600` per-folder rule for `.bin`-shaped libraries),
  confirms wood-grain themed tiles appear, and launches a known-good
  ROM. Once Phase 1 ✅, Phase 2 polish opens (paddle controller
  support — paddle-required games like Breakout / Kaboom! are
  documented unplayable until shared analog-input infra lands).
