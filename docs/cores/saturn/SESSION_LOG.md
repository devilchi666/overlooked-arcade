# saturn Session Log

Per-core Shipped / Almost / Next log for Sega Saturn. Project-wide log
lives at `docs/SESSION_LOG.md`.

---

## 2026-05-20 — Phase 0 onboarding (paired with psx)

- **Shipped:**
  - `oa_core::SystemId::Saturn` variant + `parse_system_id` arm
    accepting `"saturn" | "sat" | "ss" | "sega-saturn"`.
  - `bindings.rs::saturn` module — 13-button 6-button MD-style face
    pad (A/B/C bottom + X/Y/Z top + L/R shoulders + START + d-pad).
    `SATURN_BUTTONS` table + `default_saturn_bindings()` (Z=A primary
    via cross-system rule, D/S/F top row, Q/W shoulders) +
    `saturn_to_libretro_bits` identity remap + all 4 dispatch arms.
    Three new tests lock the dispatch.
  - `default_core_dll_for_system("saturn") → "mednafen_saturn_libretro.dll"`
    in main.rs. Alternates documented (kronos_libretro,
    yabasanshiro_libretro) for per-system Cores override.
  - `media::repo_for_system_id` → `Sega_-_Saturn` for
    libretro-thumbnails cover sync.
  - `rom_hashes::libretro_dat_refs_for_system` → `&[]` with
    NO_DAT_SYSTEMS entry (Saturn disc-id extraction is Phase 2 polish).
  - Frontend `themes/registry.ts` — `SystemId` union extended with
    `"saturn"`, `systemThemes.saturn` entry (CD container extensions,
    landscape 4/3 tile, `crt-lite` default shader preset).
  - Frontend `themes/systems.css` — `[data-system="saturn"]` block,
    deepest purple at hue 275° + L=0.45 + C=0.18 (bottom of the
    SNES/Lynx/GBA violet cluster; period-accurate to Saturn launch
    marketing).
  - `check_saturn_bios` function + `SATURN_BIOS_KNOWN_HASHES` table
    in main.rs — five canonical SHA-1 entries (JP v1.00 / v1.01,
    US/EU v1.00, EU PAL v1.01, generic alias). CD-launch path's BIOS
    dispatch arm extended with `"saturn"` branch.
  - Per-core docs scaffold (README + ROADMAP + this log + KNOWN_GAME_BUGS
    + DECISIONS).
- **Almost:** Phase 1 operator validation — needs a real Saturn CD
  image + matching regional BIOS launched end-to-end. NiGHTS into
  Dreams / Guardian Heroes / Radiant Silvergun / Saturn Bomberman are
  good test discs.
- **Next:** Operator drops `mednafen_saturn_libretro.dll` into
  `<exe_dir>/cores/`, a regional Saturn BIOS into `<exe_dir>/system/`,
  marks a Saturn folder via Import Wizard, confirms tiles appear with
  deepest-purple theme, and launches a known-good disc. Once Phase 1
  ✅, Phase 2 polish opens (disc-id extraction, 3D Pad analog stick
  via shared analog-input infra, cart RAM expansion validation).
