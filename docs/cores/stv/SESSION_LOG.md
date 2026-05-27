# Sega Titan Video (ST-V) — Session Log

## 2026-05-27 — Slug onboarded (code-only ship)

Code-side scaffolding for `stv` shipped as part of
`feat/new-systems-jagcd-32xcd-stv` branch alongside `jagcd` and
`sega32xcd`. Lifts the system out of the `docs/NEXT.md` DEFERRED
band. Operator picked the "BIOS + ROM ready for jagcd only" path
during slice planning — Phase 1 playtest deferred until an ST-V
BIOS + ROM set is in hand.

- **Shipped:**
  - Frontend SystemId union, `systemThemes` entry (cyan-blue
    arcade palette, .zip/.7z extensions, crt-lite default shader),
    `systemUIConfigs` baseline entry.
  - `systems.css` `[data-system="stv"]` block — hue 220° / L 0.55,
    distinct from Saturn purple + lynx cyan in the sidebar at
    once.
  - `parse_system_id` aliases (`stv` / `titan` / `sega-titan-video`)
    routing to `oa_core::SystemId::Mame`. Pure alias — no new
    oa-core variant. The frontend slug stays distinct for sidebar
    + theming purposes; the launch path is fully MAME's.
  - `default_core_dll_for_system` returns `mame_libretro.dll` for
    the stv slug (shared with the parent mame system). No
    separate BIOS pre-check; MAME handles arcade BIOS lookup
    internally.
  - Bindings reuse `default_mame_bindings()` — same arcade
    6-button panel layout MAME ships.
  - `repos_for_system_id` → `[Sega_-_Titan_Video, MAME]` — the
    dedicated ST-V thumbnails repo with MAME as the fallback for
    titles missing from the ST-V-specific catalog.
  - Per-core docs — README + this SESSION_LOG + ROADMAP +
    KNOWN_GAME_BUGS shell.
- **Almost:** Operator playtest with a real ST-V ROM set + the
  `stvbios.zip` BIOS in `<exe_dir>/system/`. Recommended first
  launch when content is in hand: Radiant Silvergun (the Japan-only
  arcade original — predates the Saturn home port), Cotton 2, or
  Steep Slope Sliders. MAME's stv driver should handle BIOS lookup
  + ROM-set loading without operator intervention.
- **Next:** Phase 1 playtest. ROADMAP Phase 1 bullets close as
  each step completes.
