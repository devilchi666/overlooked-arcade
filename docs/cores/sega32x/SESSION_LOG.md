# sega32x Session Log

Per-core Shipped / Almost / Next log for Sega 32X. Project-wide log
lives at `docs/SESSION_LOG.md`.

---

## 2026-05-20 — Phase 0 onboarding (paired with segacd)

- **Shipped:**
  - `oa_core::SystemId::Sega32X` variant + `parse_system_id` arm
    accepting `"sega32x" | "32x" | "sega-32x"`.
  - `bindings.rs` — sega32x routes to the 6-button Mega Drive controller
    via shared dispatch arms (`"genesis" | "segacd" | "sega32x" => ...`).
    Same pattern PCE-CD uses to share TG-16's controller. Three new
    tests lock the dispatch: `defaults_cover_every_sega32x_button`,
    `sega32x_remap_is_identity`, `sega32x_dispatch_round_trips`.
  - `default_core_dll_for_system("sega32x") → "picodrive_libretro.dll"`
    — the only mainstream libretro core with 32X support.
  - `media::repo_for_system_id` → `Sega_-_32X` for
    libretro-thumbnails cover sync.
  - `rom_hashes::libretro_dat_refs_for_system` → `metadat/no-intro/Sega - 32X`.
  - Frontend `themes/registry.ts` — `SystemId` union extended with
    `"sega32x"`, `systemThemes.sega32x` entry (extensions `["32x"]`,
    landscape 4/3 tile, `crt-lite` default shader preset).
  - Frontend `themes/systems.css` — `[data-system="sega32x"]` block,
    neon orange at hue 42° + L=0.68 + C=0.22. Period-accurate to the
    1994 32X marketing palette; lands in the open 35-50° hue band.
  - Per-core docs scaffold (README + ROADMAP + this log + KNOWN_GAME_BUGS
    + DECISIONS).
- **Almost:** Phase 1 operator validation — needs a real `.32x` cart
  launched end-to-end. Knuckles' Chaotix, Virtua Racing Deluxe,
  Doom 32X, Star Wars Arcade are good test carts.
- **Next:** Operator drops `picodrive_libretro.dll` into `<exe_dir>/cores/`,
  scans a Sega 32X ROMs folder, confirms tiles appear with neon orange
  theme, and launches a known-good cart. Once Phase 1 ✅, Phase 2
  polish opens (region quirks compatibility map, per-system shader
  override).
