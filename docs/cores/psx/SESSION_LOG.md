# psx Session Log

Per-core Shipped / Almost / Next log for Sony PlayStation. Project-wide
log lives at `docs/SESSION_LOG.md`.

---

## 2026-05-20 — Phase 0 onboarding (paired with saturn)

- **Shipped:**
  - `oa_core::SystemId::Playstation` variant + `parse_system_id` arm
    accepting `"psx" | "ps1" | "ps" | "playstation"`.
  - `bindings.rs::psx` module — 14-button digital DualPad (d-pad +
    Triangle/Circle/Cross/Square + L1/R1/L2/R2 + START + SELECT).
    `PSX_BUTTONS` table + `default_psx_bindings()` (Z=Cross primary
    via cross-system rule, Q/W front shoulders, E/R rear triggers) +
    `psx_to_libretro_bits` identity remap + all 4 dispatch arms.
    Three new tests lock the dispatch. DualShock analog sticks + L3/R3
    deferred to Phase 2.
  - `default_core_dll_for_system("psx") → "mednafen_psx_hw_libretro.dll"`
    in main.rs (Beetle PSX HW). Beetle PSX SW
    (`mednafen_psx_libretro.dll`) documented as a pre-registered
    catalog peer alternate.
  - `media::repo_for_system_id` → `Sony_-_PlayStation` for
    libretro-thumbnails cover sync.
  - `rom_hashes::libretro_dat_refs_for_system` → `&[]` with
    NO_DAT_SYSTEMS entry (PSX disc-id extraction is Phase 2 polish).
  - Frontend `themes/registry.ts` — `SystemId` union extended with
    `"psx"`, `systemThemes.psx` entry (CD container extensions +
    `.pbp`, landscape 4/3 tile, `crt-lite` default shader preset).
  - Frontend `themes/systems.css` — `[data-system="psx"]` block,
    teal cyan at hue 180° + L=0.65 + C=0.16 (open band, evokes PS1
    launch cool-blue/cyan/silver palette).
  - `check_psx_bios` function + `PSX_BIOS_KNOWN_HASHES` table in
    main.rs — six canonical SHA-1 entries spanning JP / US / EU v3.0
    + US v4.x revisions + SCPH-100x PSone alias. CD-launch path's
    BIOS dispatch arm extended with `"psx"` branch.
  - `is_cd_extension` extended to include `.pbp` — the PSP-format
    PS1 EBOOT container needs BIOS pre-check + path-based loading
    like other CD images.
  - Per-core docs scaffold (README + ROADMAP + this log + KNOWN_GAME_BUGS
    + DECISIONS).
- **Almost:** Phase 1 operator validation — needs a real PSX CD image
  + matching regional BIOS launched end-to-end. Castlevania: Symphony
  of the Night / Final Fantasy VII / Metal Gear Solid / Crash Bandicoot
  / Resident Evil are good test discs.
- **Next:** Operator drops `mednafen_psx_hw_libretro.dll` into
  `<exe_dir>/cores/`, a regional PSX BIOS into `<exe_dir>/system/`,
  marks a PSX folder via Import Wizard, confirms tiles appear with
  teal-cyan theme, and launches a known-good disc. If HW core fails
  to obtain a Vulkan/OpenGL surface from wgpu host, operator swaps to
  Beetle PSX SW via per-system Cores. Once Phase 1 ✅, Phase 2 polish
  opens (DualShock analog sticks via shared analog-input infra,
  disc-id extraction via cd_id.rs, PGXP geometry correction surface).
