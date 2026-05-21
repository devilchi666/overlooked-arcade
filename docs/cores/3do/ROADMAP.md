# 3do — Roadmap

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

- ✅ `oa_core::SystemId::ThreeDo` variant + `parse_system_id` arm
  (`3do | threedo | panasonic-3do`).
- ✅ `bindings.rs::threedo` module — 11-button (d-pad + A/B/C + L/R +
  STOP/PLAY + START). All dispatch arms wired.
- ✅ `default_core_dll_for_system("3do") → "opera_libretro.dll"`.
- ✅ `rom_hashes` → `&[]` with NO_DAT_SYSTEMS entry.
- ✅ `media::repo_for_system_id` → `The_3DO_Company_-_3DO`.
- ✅ `check_3do_bios` + `THREEDO_BIOS_KNOWN_HASHES` (4 entries:
  FZ-1, FZ-10, GoldStar, Sanyo). Slotted into CD-launch dispatch.
- ✅ Frontend SystemId / systemThemes / CSS (Plan A: deep 3DO purple-
  magenta `oklch(0.55 0.22 297)` in tight Lynx→WS gap).
- ✅ Per-core docs scaffold.

---

## ⬜ Phase 1 — First 3DO game running

- ⬜ Operator validation: Star Control II / Road Rash / The Need for
  Speed / Lemmings 3DO / Crash 'n Burn.
- ⬜ Multi-region BIOS testing (FZ-1 + FZ-10 + GoldStar + Sanyo) — operator-driven.
- ✅ Save state F5/F8 round-trip mid-disc — closed by cross-system Phase 1.5 + Phase 4 save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).

---

## ✅ Phase 2 — Polish

- ✅ Disc-id extraction — closed by design: 3DO discs lack a standardized catalog serial; `apps/oa-shell/src/cd_id.rs::dispatch_extractor` intentionally returns None for 3DO (libretro-database's 3DO dat has zero serial fields).
- ✅ Per-game core option surface — closed by cross-system per-game settings drawer (slice 2.8.D) + per-system settings page (slice 2.8.C).
