# neocd Session Log

---

## 2026-05-20 — Phase 0 onboarding (paired with neogeo + ngp)

- **Shipped:** SystemId variant, parse_system_id arm, shared
  controller dispatch via `"neogeo" | "neocd" => ...` (same precedent
  PCE-CD/TG-16 set), default core (`neocd_libretro.dll`), media +
  rom_hashes arms (NO_DAT_SYSTEMS entry for CD-shape), CSS theme
  (muted SNK gold 50° — family-cousin to cart neogeo's deep red),
  `check_neocd_bios` + `NEOCD_BIOS_KNOWN_HASHES` table (3 entries),
  CD-launch BIOS dispatch arm extended with `"neocd"`. Per-core docs
  scaffold.
- **Almost:** Phase 1 operator validation.
- **Next:** Operator drops `neocd_libretro.dll` +
  `neocd_z.rom`/`neocd_t.rom` BIOS, marks a Neo Geo CD folder via
  Import Wizard, launches Samurai Shodown RPG / Metal Slug CD / KOF
  '96 / Last Blade.
