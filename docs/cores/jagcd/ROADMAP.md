# Atari Jaguar CD — Roadmap

Onboarding status for the `jagcd` slug. The slug was added in branch
`feat/new-systems-jagcd-32xcd-stv` (2026-05-27) — code-side scaffolding
complete; operator validation in flight.

## Phase 0 — slug wiring (✅ shipped 2026-05-27)

- ✅ `oa_core::SystemId::JaguarCd` variant in `crates/oa-core/src/lib.rs`
- ✅ `parse_system_id` accepts `"jagcd" | "jaguar-cd" | "atari-jaguar-cd"`
- ✅ `default_core_dll_for_system` returns `virtualjaguar_libretro.dll`
- ✅ Frontend `SystemId` union, `systemThemes`, `systemUIConfigs` entries
- ✅ `systems.css` data-system block (gold-amber palette)
- ✅ Bindings re-use `default_jaguar_bindings()` via the
  `"jaguar" | "jagcd"` defaults_for arm
- ✅ `repos_for_system_id` → `Atari_-_Jaguar_CD` thumbnails repo
- ✅ `check_jagcd_bios` + CD-shape BIOS dispatch arm

## Phase 1 — operator playtest (⬜)

- ⬜ Verify `jagboot.rom` + `jagcd.rom` are both detected from
  `<exe_dir>/system/` and the launch path doesn't abort.
- ⬜ Launch a Jaguar CD title (Hover Strike: Unconquered Lands or
  Battlemorph are the most-playable retail recommendations) and
  confirm the game boots through both BIOSes correctly.
- ⬜ Confirm save-state save / load works (Virtual Jaguar's save state
  format is identical between cart + CD).
- ⬜ Confirm `Atari_-_Jaguar_CD` thumbnails repo syncs cover art
  cleanly (operator-triggered sync from per-system Media tab).
- ⬜ KNOWN_GAME_BUGS triage — populate the per-title quirks file from
  operator playtest notes.

## Phase 2+ — polish (deferred until Phase 1 surfaces a need)

- ⬜ Per-game `core_option` overrides for FMV-heavy titles if the
  default Virtual Jaguar settings underperform (Dragon's Lair, Vid
  Grid).
- ⬜ Disc-swap UI integration — Jaguar CD multi-disc games are rare
  but exist; verify the `set_disc_image` / `set_disc_eject` flow
  works against Virtual Jaguar.

## Known limitations

Virtual Jaguar's CD support has historically been considered the
weakest part of the core. Some games (Highlander I, Black Ice/White
Noise) are notorious for booting issues even on real hardware;
expect a similar compatibility floor here. Use upstream MAME's
Jaguar CD driver as the accuracy reference if a specific title
fails to boot.
