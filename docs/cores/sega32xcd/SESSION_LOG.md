# Sega 32X CD — Session Log

## 2026-05-27 — Slug onboarded (code-only ship)

Code-side scaffolding for `sega32xcd` shipped as part of
`feat/new-systems-jagcd-32xcd-stv` branch alongside `jagcd` and
`stv`. Lifts the system out of the `docs/NEXT.md` DEFERRED band.
Operator picked the "BIOS + ROM ready for jagcd only" path during
slice planning — Phase 1 playtest deferred until they have a
Sega CD BIOS + 32X-CD game image in hand.

- **Shipped:**
  - Frontend SystemId union, `systemThemes` entry, `systemUIConfigs`
    baseline entry.
  - `systems.css` `[data-system="sega32xcd"]` block — orange-red
    32X family (hue 42°), L 0.60 so the sidebar reads distinctly
    from cart 32X.
  - `parse_system_id` aliases (`sega32xcd` / `sega-32x-cd` /
    `32xcd` / `32x-cd`) routing to `oa_core::SystemId::SegaCd`. The
    "stacked override" pattern (per the Sega32X doc comment) — no
    new oa-core variant; the divergence is at the slug level via
    `default_core_dll_for_system`.
  - `default_core_dll_for_system` returns `picodrive_libretro.dll`
    for the sega32xcd slug — PicoDrive is the only mainstream
    libretro core with 32X+CD combined mode.
  - CD-shape BIOS dispatch arm reusing `check_sega_cd_bios` (same
    regional Sega CD BIOS files suffice; 32X cart BIOSes optional).
  - Bindings reuse `default_genesis_bindings()` — Mega Drive 6-button
    layout shared across the genesis / segacd / sega32x / sega32xcd
    family.
  - `repos_for_system_id` → `Sega_-_32X` thumbnails repo (no
    dedicated 32X-CD repo exists; the FMV titles are cataloged with
    cart 32X upstream).
  - Per-core docs — README + this SESSION_LOG + ROADMAP +
    KNOWN_GAME_BUGS shell.
- **Almost:** Operator playtest with a real Sega 32X CD image +
  the regional Sega CD BIOS in `<exe_dir>/system/`. Recommended
  first launch when content is in hand: Night Trap (32X CD).
- **Next:** Phase 1 playtest. ROADMAP Phase 1 bullets close as
  each step completes.
