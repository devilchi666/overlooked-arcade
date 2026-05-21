# neogeo Session Log

Per-core Shipped / Almost / Next log for SNK Neo Geo. Project-wide log
lives at `docs/SESSION_LOG.md`.

---

## 2026-05-20 — Phase 0 onboarding (paired with neocd + ngp)

- **Shipped:**
  - `oa_core::SystemId::NeoGeo` variant + parse_system_id arm
    accepting `"neogeo" | "neo-geo" | "aes" | "mvs"`.
  - `bindings.rs::neogeo` module — 10-button arcade face pad (A/B/C/D
    + START + COIN + d-pad). `NEOGEO_BUTTONS` table +
    `default_neogeo_bindings()` (Z=A primary via cross-system rule,
    A/B/C/D mapped to East/South/West/North, COIN on Key5 matching
    MAME convention) + `neogeo_to_libretro_bits` identity remap +
    all 4 dispatch arms. Neo Geo CD shares the controller via
    `"neogeo" | "neocd" => ...` dispatch arms.
  - `default_core_dll_for_system("neogeo") → "fbneo_libretro.dll"`.
  - `media::repo_for_system_id` → `SNK_-_Neo_Geo` for thumbnails.
  - `rom_hashes::libretro_dat_refs_for_system` → no-intro
    `SNK - Neo Geo` dat.
  - `check_neogeo_bios` function in main.rs — existence-only Phase 0
    check for `neogeo.zip` in `<exe_dir>/system/`. Cart-launch BIOS
    pre-check arm lives next to the CD-launch dispatch.
  - **`archive.rs::peek_zip_for_neogeo`** — new content-peek function
    that scans a .zip's inner files for the Neo Geo signature (`.p1`
    + `.s1` extensions together). Returns true for Neo Geo ROM-sets;
    MAME zips fall through.
  - `scan_service.rs` — extended ScannedRom struct with optional
    `system_hint` field. Scanner integration: .zip files matching the
    Neo Geo signature emit a single ScannedRom for the whole zip
    with `system_hint = "neogeo"`. MAME zips fall through to the
    normal inner-file enumeration path.
  - Frontend `library/ingest.ts` — ScannedRom type extended with
    optional `systemHint?`. Ingest paths (both `ingestFolderPath` and
    `rescanFolders`) prefer the hint over generic extension mapping
    when present.
  - Frontend `themes/registry.ts` — `SystemId` union extended with
    `"neogeo"`, `systemThemes.neogeo` entry (extensions
    `["neo", "zip"]`, landscape 4/3 tile, crt-lite default shader).
  - Frontend `themes/systems.css` — `[data-system="neogeo"]` block,
    deepest+most-saturated red at hue 18° + L=0.50 + C=0.27 (cluster
    bottom alongside VB 7° / MAME 12° / NES 28°). Period-correct to
    SNK arcade marketing.
  - Per-core docs scaffold.
- **Almost:** Phase 1 operator validation — needs a real ROM-set
  launch end-to-end with `neogeo.zip` BIOS. Metal Slug / KOF '97 /
  Samurai Shodown II / Garou good test sets.
- **Next:** Operator drops `fbneo_libretro.dll` + `neogeo.zip`, scans
  a Neo Geo folder, confirms `.neo` + `.zip` files classify correctly
  (content-peek catches Neo Geo zips; MAME zips remain MAME), launches
  a known-good set. Phase 2 polish queues Universe BIOS support +
  AES/MVS mode toggle surface + ROM-set content validation upgrade.
