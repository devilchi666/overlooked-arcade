# neocd — Roadmap

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

- ✅ `oa_core::SystemId::NeoGeoCd` variant + `parse_system_id` arm
  (`neocd | neo-geo-cd | neogeocd`).
- ✅ Shares the cart-Neo-Geo controller via `"neogeo" | "neocd" => ...`
  dispatch arms in bindings.rs.
- ✅ `default_core_dll_for_system("neocd") → "neocd_libretro.dll"`.
- ✅ `rom_hashes` → `&[]` with NO_DAT_SYSTEMS entry (CD disc-id
  extraction Phase 2).
- ✅ `media::repo_for_system_id` → `SNK_-_Neo_Geo_CD`.
- ✅ `check_neocd_bios` + `NEOCD_BIOS_KNOWN_HASHES` (3 entries:
  CDZ top-loader, CD front-loader, front-loader alias). Slotted into
  the CD-launch BIOS dispatch arm next to pce-cd/segacd/saturn/psx.
- ✅ Frontend SystemId / systemThemes / CSS (Plan A: muted SNK gold
  50°, family-cousin to cart neogeo via warm zone).
- ✅ Per-core docs scaffold.

**Acceptance gate:** Operator drops `neocd_libretro.dll` +
`neocd_z.rom`, marks a Neo Geo CD folder via Import Wizard, launches
a known-good disc.

---

## ⬜ Phase 1 — First Neo Geo CD game running

- ⬜ Operator validation: **Samurai Shodown RPG** (CD-exclusive), **Metal Slug 1 CD**, **KOF '96 CD**, **Last Blade CD** — operator playtest.
- ✅ Save state F5/F8 round-trip mid-disc — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ⬜ CDDA streaming validation — operator playtest (Samurai Shodown RPG canonical test).
- ✅ Cover sync via libretro-thumbnails — closed by cross-system media sync (`media::sync_media_for_system`).

---

## ⬜ Phase 2 — Polish

- ✅ Disc-id extraction — shipped via `apps/oa-shell/src/cd_id.rs::extractors::neo_geo_cd` (reads IPL.TXT-area NGCD/ADK prefixes); `rom_hashes` points at `metadat/redump/SNK - Neo Geo CD`.
- ⬜ Region-toggle UX — operator-driven Unibios CD-variant curation (same shape as cart neogeo).
- ⬜ Load-time optimization — operator-driven NeoCD "fast load" core-option surface.

---

## ⬜ Phase 3+ — Stretch

- ⬜ Multi-disc Neo Geo CD titles via `.m3u` — deferred (rare, most titles fit on a single disc).
