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

- ⬜ **Save-state round-trip mid-disc** — operator smoke-test (Castlevania Rondo); save-state infra itself is shipped cross-system.
- ⬜ **Multi-disc title via .m3u** — operator playtest (Cosmic Fantasy 4, Tengai Makyō II).
- ⬜ **`oa-cdrom` build-out** — deferred-until-forced; only if validation surfaces real gaps.
- ⬜ **TG-CD-specific theming polish** — operator-driven UI polish (per-system theming infra shipped cross-system).
- ✅ **Per-game cover sync via libretro-thumbnails** — closed by cross-system media sync (`media::sync_media_for_system`).

---

## ⬜ Phase 6+ contributions

- ⬜ Per-game shader preset overrides for CD-FMV-heavy titles — operator-driven curation (per-game shader override infra shipped cross-system).
- ⬜ KNOWN_GAME_BUGS triage as the library grows — operator-driven data curation.
