# pce-cd Roadmap

Per-core phase tracking for TurboGrafx-CD / PC Engine CD-ROM². Mirrors the
project-wide Phase 5 entry in `docs/ROADMAP.md`.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 5 — PCE-CD bringup (closed 2026-05-18)

- ✅ **Core picked.** Beetle PCE Fast (`mednafen_pce_fast_libretro.dll`)
  handles CD. Operator-validated 2026-05-18 with Castlevania: Rondo of Blood
  (CHD). Full Beetle PCE Mednafen kept available as a per-game fallback.
- ✅ **End-to-end validation** against the existing infrastructure: BIOS
  check, .chd path-based load, title-screen video, CDDA music, gameplay,
  audio.
- ✅ **System registry split.** Frontend: `SystemId` extended to include
  `pce-cd`; CD extensions moved off `tg16`. CSS: `[data-system="pce-cd"]`
  block with cyan-blue palette. Rust bindings.rs: pce-cd routes to the PCE
  controller table (same hardware as TG-16). Library DB v4→v5 migration
  retags existing tg16 rows whose file_path/archive_inner_path ends in a
  CD container extension.

**Acceptance gate:** Rondo of Blood boots from CHD on the existing
infrastructure, CDDA plays, gameplay starts. **Met.**

---

## ⬜ Phase 5.5 — Hardening (post-split, pre-Phase 6 work)

Small follow-ups that don't gate the next core but should land before
PCE-CD is considered "shipped" quality.

- ⬜ **Save-state round-trip mid-disc.** Verify save_state taken during
  gameplay reloads cleanly with the CD read-pointer intact. Should work
  via libretro `retro_serialize` / `retro_unserialize` (same path as
  HuCard save states), but CD adds disc-state machinery that's worth
  explicit smoke-testing.
- ⬜ **Multi-disc title via .m3u.** Pick a real multi-disc release
  (Cosmic Fantasy 4, Tengai Makyō II, etc.), confirm disc swap works
  through the libretro disc-control extension.
- ⬜ **`oa-cdrom` build-out — only if validation surfaces real gaps.**
  Plausible API surface if needed: CHD/CUE metadata parser for track
  count + disc count (library tile metadata); audio-track name surface
  for the right sidebar widgets. Don't pre-build — let real validation
  drive the API.
- ⬜ **TG-CD-specific theming polish.** The cyan-blue palette ships
  v1; per-system page header art / sidebar icon / cover-art frame may
  need TG-CD-specific tweaks once we have a few titles in the library.
- ⬜ **Per-game cover sync via libretro-thumbnails — infra ready 2026-05-19, needs operator validation.** Mapping `pce-cd → NEC_-_PC_Engine_CD_-_TurboGrafx-CD` shipped in `apps/oa-shell/src/media.rs::repo_for_system_id`. Operator: run `Settings → Library → Sync media for PC Engine CD` and confirm covers download.

---

## ⬜ Phase 6+ contributions

- ⬜ Per-game shader preset overrides for CD-FMV-heavy titles (some
  CDDA-era games have specific palette quirks the default Phosphor
  preset doesn't flatter).
- ⬜ KNOWN_GAME_BUGS triage as the library grows. Expected hotspots:
  CDDA timing edge cases, FMV stutter on slower CHDs, region-specific
  BIOS mismatches.

---

## 2026-05-21 — Stale-cleanup audit

The Phase 1+ items above were written when this system onboarded, before cross-system infrastructure (Phases 1.5 / 2.5–2.8 / 3 / 4 + direct-launch CLI) landed. Many `⬜` items are actually shipped — see `docs/cores/AUDIT_2026-05-21.md` for the per-item breakdown (stale vs open-code vs open-operator) for this system.
