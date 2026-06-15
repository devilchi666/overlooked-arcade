# Sega Titan Video (ST-V) — Roadmap

Onboarding status for the `stv` slug. Added in branch
`feat/new-systems-jagcd-32xcd-stv` (2026-05-27). Code-side wiring
complete; operator playtest deferred (operator picked the
"have BIOS + ROMs for jagcd only" path during slice planning).

## Phase 0 — slug wiring (✅ shipped 2026-05-27)

- ✅ Frontend `SystemId` union, `systemThemes` entry (cyan-blue
  hue 220° / L 0.55 arcade-weight palette, .zip/.7z extensions,
  crt-lite default shader).
- ✅ `systemUIConfigs` baseline entry.
- ✅ `SYSTEM_PALETTES` entry (`frontend/src/platform/themes/systemPalettes.ts`,
  injected as the `[data-system="stv"]` CSS block at boot) — Sega
  arcade cyan-blue distinct from Saturn purple (275°) and lynx (220°
  but L 0.72).
- ✅ `parse_system_id` accepts `"stv" | "titan" | "sega-titan-video"`
  and routes to `oa_core::SystemId::Mame` (pure alias — no new
  oa-core variant).
- ✅ `default_core_dll_for_system` returns `mame_libretro.dll`
  for the stv slug.
- ✅ Bindings re-use `default_mame_bindings()` via the
  `"mame" | "stv"` defaults_for arm.
- ✅ `repos_for_system_id` → `[Sega_-_Titan_Video, MAME]` — primary
  Sega_-_Titan_Video thumbnails repo, falls back to MAME if a
  title isn't cataloged separately.

## Phase 1 — operator playtest (⬜ awaiting BIOS + ROM set)

- ⬜ Drop a complete ST-V ROM set (`stvbios.zip` + a few playable
  games like Radiant Silvergun, Cotton 2, or Steep Slope Sliders)
  into `<exe_dir>/system/` + the library folder operator marks
  as `stv`.
- ⬜ Launch a title. MAME's stv driver should handle BIOS lookup +
  ROM-set loading without operator intervention.
- ⬜ Confirm save-state save / load works (MAME's save state
  format applies).
- ⬜ Confirm `Sega_-_Titan_Video` thumbnails repo syncs cover art
  cleanly (operator-triggered sync from per-system Media tab);
  fallback to MAME repo for titles missing in the dedicated repo.
- ⬜ KNOWN_GAME_BUGS triage — populate the per-title quirks file
  from playtest notes.

## Phase 2+ — polish (deferred until Phase 1 surfaces a need)

- ⬜ Per-game `core_option` overrides for any title where MAME's
  default settings underperform.
- ⬜ Evaluate adding Beetle Saturn STV experimental mode as an
  alternate core via per-game override — pick this if operators
  surface MAME-specific compatibility gaps.

## Known limitations

MAME's stv driver is mature but a few late-era ST-V titles
historically have minor visual or audio quirks. Cross-reference
the MAME upstream issue tracker if a specific title diverges
from real-hardware behaviour.
