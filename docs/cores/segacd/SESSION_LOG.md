# segacd Session Log

Per-core Shipped / Almost / Next log for Sega CD / Mega-CD. Project-wide
log lives at `docs/SESSION_LOG.md`.

---

## 2026-05-20 — Phase 0 onboarding (paired with sega32x)

- **Shipped:**
  - `oa_core::SystemId::SegaCd` variant + `parse_system_id` arm
    accepting `"segacd" | "sega-cd" | "mega-cd" | "megacd" | "mcd"`.
  - `bindings.rs` — segacd routes to the 6-button Mega Drive controller
    via shared dispatch arms (`"genesis" | "segacd" | "sega32x" => ...`).
    Same pattern PCE-CD uses to share TG-16's controller. Three new
    tests lock the dispatch: `defaults_cover_every_segacd_button`,
    `segacd_remap_is_identity`, `segacd_dispatch_round_trips`.
  - `default_core_dll_for_system("segacd") → "genesis_plus_gx_libretro.dll"`
    — same .dll already shipping for SMS + Game Gear.
  - `media::repo_for_system_id` → `Sega_-_Mega-CD_-_Sega_CD` for
    libretro-thumbnails cover sync.
  - `rom_hashes::libretro_dat_refs_for_system` → `&[]` with NO_DAT_SYSTEMS
    entry (CD images aren't single-file SHA-1 matched).
  - Frontend `themes/registry.ts` — `SystemId` union extended with
    `"segacd"`, `systemThemes.segacd` entry (CD container extensions,
    landscape 4/3 tile, `plain` default shader preset for FMV-heavy
    library).
  - Frontend `themes/systems.css` — `[data-system="segacd"]` block,
    sapphire blue at hue 235° + L=0.55 + C=0.20 (family-cousin to
    Genesis cobalt 245°, visually distinct via lightness axis).
  - `check_sega_cd_bios` function + `SEGA_CD_BIOS_KNOWN_HASHES` table
    in main.rs — six canonical SHA-1 entries across US / JP / EU
    regional variants. CD-launch path branches by system_id to call the
    right BIOS check.
  - Per-core docs scaffold (README + ROADMAP + this log + KNOWN_GAME_BUGS
    + DECISIONS).
- **Almost:** Phase 1 operator validation — needs a real Sega CD image
  + matching regional BIOS launched end-to-end. Sonic CD (US v2.00),
  Lunar: The Silver Star Complete, Snatcher, Popful Mail are good
  test discs.
- **Next:** Operator drops `genesis_plus_gx_libretro.dll` into
  `<exe_dir>/cores/`, a regional `bios_CD_*.bin` into `<exe_dir>/system/`,
  marks a Sega CD folder via Import Wizard, confirms tiles appear with
  sapphire theme, and launches a known-good disc. Once Phase 1 ✅,
  Phase 2 polish opens (disc-id extraction via cd_id.rs, redump dat
  switch, per-system theming).
