# Atari Jaguar CD — Session Log

## 2026-05-27 — Slug onboarded

Code-side scaffolding for `jagcd` shipped as part of
`feat/new-systems-jagcd-32xcd-stv` branch alongside `sega32xcd` and
`stv`. Lifts the system from the `docs/NEXT.md` DEFERRED band.

- **Shipped:**
  - `oa_core::SystemId::JaguarCd` variant.
  - `parse_system_id` aliases (`jagcd` / `jaguar-cd` / `atari-jaguar-cd`).
  - `default_core_dll_for_system` returns `virtualjaguar_libretro.dll`
    — shared with the cart `jaguar` system.
  - `JAGCD_BIOS_KNOWN_HASHES` + `check_jagcd_bios` helper. CD-side
    boot ROM (`jagcd.rom`) hash-checked against the libretro-database
    canonical dump; missing file blocks launch with an actionable
    error toast. Cart-side `jagboot.rom` still pre-checked via the
    existing `check_jaguar_bios` path.
  - CD-shape BIOS dispatch arm in `main.rs` so the launch path
    consults `check_jagcd_bios` before invoking the .dll.
  - Frontend SystemId union, `systemThemes` entry (gold-amber
    palette, 4/3 tile aspect, crt-lite default shader), and
    `systemUIConfigs` baseline entry.
  - `systems.css` `[data-system="jagcd"]` block — same hue family as
    cart Jaguar (gold-orange) but L 0.58 / hue 75° so the sidebar
    entries read distinctly when both are in the library at once.
  - Bindings reuse `default_jaguar_bindings()` via the
    `"jaguar" | "jagcd"` arm in `defaults_for` — Pro Controller
    layout shared 1:1 between cart + CD systems.
  - `repos_for_system_id` → `Atari_-_Jaguar_CD` libretro-thumbnails
    repo (separate from the cart-Jaguar thumbnails).
  - Per-core docs scaffolded — README + this SESSION_LOG + ROADMAP +
    KNOWN_GAME_BUGS shell.
- **Almost:** Operator playtest. Deferred until the operator has
  legally acquired the `jagcd.rom` + `jagboot.rom` BIOS pair plus
  a Jaguar CD image. Recommended first launch when content is in
  hand: Hover Strike: Unconquered Lands or Battlemorph (most-
  playable retail; Virtual Jaguar's CD support has historical
  compatibility quirks on some titles).
- **Next:** Phase 1 playtest when BIOSes + ROM are legally
  available. Per-title KNOWN_GAME_BUGS triage as playtest surfaces
  real issues. Phase 1 ROADMAP bullets close as each lands.
