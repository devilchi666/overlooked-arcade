# mame Decisions

Per-core integration decisions. Project-wide architecture lives in `docs/DECISIONS.md`.

---

## 2026-05-19 — `mame` slug, not `arcade-mame`

**Decision:** Use `mame` as the SystemId slug. The buildbot catalog originally used `arcade-mame` (to disambiguate from `arcade-fbneo`); both got renamed to bare names (`mame` / `fbneo`) when MAME got wired in.

**Why:** Kebab-case `arcade-mame` reads as a category prefix, but MAME isn't a category — it's one specific emulator core (and one specific subset of arcade hardware). FinalBurn Neo wraps a different set of boards (Neo Geo + CPS1/2/3 + Cave + …). Calling them sibling slugs without the `arcade-` prefix matches how the rest of the project speaks about systems (`tg16`, not `console-tg16`).

When future arcade subsystems get added (Naomi via Flycast, Sega Saturn-arcade boards via Mednafen Saturn, etc.), they'll get their own short slug too.

---

## 2026-05-19 — Z is primary, not SF-native top-row punches

**Decision:** Default MAME bindings put **Z = B1 (weak punch)** to match the cross-system "Z is primary action" rule, even though Street Fighter veterans expect the punches on the top keyboard row (A/S/D = LP/MP/HP) and kicks on the bottom (Z/X/C = LK/MK/HK).

**Why:** Consistency across systems wins over preserving any one game's tradition. PCE established Z = primary in Phase 1; every subsequent system (Lynx, NES, SNES) keeps it. Adding an exception for MAME would break the `z_is_the_primary_action_button_on_every_system` regression test and confuse users muscle-memorying their way between consoles.

SF purists can remap via the per-system Bindings dialog. We'll consider shipping an "SF-native" alternate default if Phase 1.5 validation finds users routinely remapping all 6 buttons.

---

## 2026-05-19 — Extensions: `.zip` + `.chd`

**Decision:** MAME claims `.zip` and `.chd` in the frontend's `systemThemes.mame.extensions`.

**Why:** MAME ROM-sets are universally distributed as `.zip` archives. The library scanner peeks inside `.zip` files first; archives containing recognized inner extensions (`.nes`, `.smc`, etc.) get reclassified to that system. MAME zips contain hardware-specific binary blobs without a recognized inner extension, so they fall through to MAME by elimination.

`.chd` is included because some arcade boards (Killer Instinct, Atari System 2 disk-backed games, late Capcom CD-backed boards) ship as CHD images. PCE-CD also claims `.chd` — but PCE-CD's archive peek isn't applicable here (PCE-CD already filters on `.cue`/`.toc` filename patterns alongside `.chd`); a bare arcade `.chd` falls through to MAME.

If this dual-claim ever produces a misclassification in practice, the fix is to add a "MAME zip" heuristic to the scanner (e.g. detect that the zip contains files like `pacman.6e` with hardware-style naming) — defer until real misclassification surfaces.

---

## 2026-05-19 — Single core, no FBNeo at onboarding

**Decision:** Ship MAME alone first; FBNeo (`fbneo` slug) stays in the "Not yet wired" section.

**Why:** Both cover arcade, but they're not interchangeable — FBNeo covers Neo Geo + CPS1/2/3 + Cave + Toaplan with higher accuracy than MAME's equivalents, while MAME's coverage is broader (Williams, Atari, Nintendo VS, etc.). Wiring both at the same time forces an immediate decision tree per-game ("which arcade core for THIS game?"). Land MAME alone first, run a few weeks of real arcade play, then decide whether FBNeo deserves its own SystemId or whether it should be a per-game `core_override` against MAME-tagged ROMs.
