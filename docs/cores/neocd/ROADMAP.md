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

- ⬜ Operator validation. Suggested: **Samurai Shodown RPG**
  (CD-exclusive), **Metal Slug 1 CD**, **King of Fighters '96 CD**,
  **Last Blade CD**.
- ⬜ Save state F5/F8 round-trip mid-disc.
- ⬜ CDDA streaming validation — Neo Geo CD titles have CDDA
  soundtracks; Samurai Shodown RPG is the canonical test.
- ⬜ Cover sync via libretro-thumbnails — operator pass.

---

## ⬜ Phase 2 — Polish

- ⬜ Disc-id extraction — Neo Geo CD discs carry a game-id signature
  in IPL.TXT on the data track. Extend `cd_id.rs` + switch
  `rom_hashes` from `&[]` to redump dat ref.
- ⬜ Region-toggle UX — same Unibios CD-variant work as cart neogeo.
- ⬜ Load-time optimization — Neo Geo CD's CD-loading was notoriously
  slow on real hardware (multiple-minute pauses). NeoCD core options
  expose a "fast load" toggle that operators may want surfaced.

---

## ⬜ Phase 3+ — Stretch

- ⬜ Multi-disc Neo Geo CD titles via `.m3u` (rare — most Neo Geo CD
  titles fit on a single disc).
