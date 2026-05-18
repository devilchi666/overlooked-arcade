# pce-cd Session Log

Append-only. Newest at the bottom. Three lines per session: **Shipped /
Almost / Next**.

---

## 2026-05-18 — Phase 5 close: PCE-CD registry split

- **Shipped:** PCE-CD lives as its own SystemId. Operator validated
  Castlevania: Rondo of Blood (CHD) running end-to-end on Beetle PCE Fast
  via the existing CD load infrastructure — BIOS SHA-1 check passed, CHD
  loaded via path-based RomSource, gameplay + audio confirmed. Frontend
  `SystemId` union extended; `systemThemes["pce-cd"]` added with the CD
  extensions (cue/chd/ccd/toc/m3u/iso) moved off `tg16` so .pce stays
  cart-only. `[data-system="pce-cd"]` palette: cyan-blue (220°) distinct
  from TG-16 orange, SNES violet, Lynx purple. Rust `bindings.rs` shares
  PCE_BUTTONS / pce_to_libretro_bits / default_pce_bindings between
  `tg16` and `pce-cd` (same controller — split is library/theme only).
  Library DB v4→v5 migration retags existing tg16 rows whose
  file_path/archive_inner_path ends in a CD container extension. One new
  oa-shell test (`v4_to_v5_retags_cd_games_to_pce_cd`) covers carts,
  bare CDs, archived inner-.cue, and the trick case of a .pce file
  whose path happens to contain the substring "cue".
- **Almost:** Save-state round-trip mid-disc smoke test (libretro path
  should Just Work but worth explicit validation). Multi-disc .m3u
  exercise. `oa-cdrom` build-out gated on those surfacing real gaps —
  don't pre-build.
- **Next:** Phase 5.5 hardening or jump to Phase 6+ next-system work
  (operator call). The first-wave order has 7800 / SMS / GG / MSX /
  Coleco / Vectrex / VB / WonderSwan queued, plus continued breadth
  beyond the first wave.
