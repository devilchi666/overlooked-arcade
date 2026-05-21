# coleco Decisions Log

ColecoVision-specific integration choices. Project-wide decisions live in `docs/DECISIONS.md`. Append-only; newest at the bottom.

---

## 2026-05-19 — blueMSX as the default Coleco core

**Decision:** `default_core_dll_for_system("coleco")` returns `"bluemsx_libretro.dll"`.

**Why:** blueMSX is the long-standing libretro ColecoVision default — mature, broad compat across the Z80 systems family, well-tested. Per-system Cores alternate is `gearcoleco_libretro.dll` (Coleco-only, lighter — useful for low-spec hosts or when the operator wants a cleaner per-system install).

---

## 2026-05-19 — 16-button layout (D-pad + 2 fires + 10 keypad numbers)

**Decision:** `COLECO_BUTTONS` ships 16 entries: 4-way d-pad + L_FIRE (yellow side button) + R_FIRE (red side button) + KP0..KP9. Keypad * and # keys are NOT exposed (rare; per-game core options handle them when needed).

**Why:** The keypad is a critical part of the Coleco controller — many launch-era games (Donkey Kong, Zaxxon, Mouse Trap) require keypad input at start screens for game-mode / difficulty selection. Phase 0 ships full keypad coverage rather than deferring it. The 16-bit mapping uses every available libretro RetroPad bit (face + shoulders + L2/R2 + L3/R3 + Start + Select) per blueMSX's libretro convention. Operators with controllers that have limited button counts can configure per-system per-button rebinds via the OA bindings UI.

---

## 2026-05-19 — Bright cyan accent at hue 195°

**Decision:** `[data-system="coleco"]` ships `oklch(0.72 0.16 195)` — bright cyan in the open 160-215° teal-cyan range.

**Why:** ColecoVision's branding wasn't strongly color-coded (console was black/silver with a red logo on white). Picking the iconic logo red would collide with NES (28°) + MAME (12°) + 2600 (60° brown, low chroma) — too crowded. The unclaimed cyan range gives Coleco a fresh visual identity without competing for warm-color tile recognition.

---

## 2026-05-19 — `.col` + `.cv`; exclude `.bin` globally

**Decision:** `extensions = ["col", "cv"]`. `.bin` deliberately excluded.

**Why:** Same rationale as 2600. Both `.col` and `.cv` are real dump extensions in active use (`.col` is No-Intro standard; `.cv` shows up in older sets). Coleco's typical `.bin` dump shape collides with too many other systems to safely register globally; operators with `.bin`-shaped libraries configure per-folder `*.bin → coleco` rules in the Import Wizard.

---

## 2026-05-19 — BIOS required (coleco.rom)

**Decision:** Document `coleco.rom` (8 KB) as REQUIRED in `<exe_dir>/system/`. No BIOS pre-check yet — operator sees blueMSX's failure mode directly on launch.

**Why:** Same Phase-2 polish pattern as the planned PCE-CD syscard pre-check at `apps/oa-shell/src/main.rs::check_pce_cd_bios`. The Coleco BIOS contains the title screen + base I/O routines — no game can boot without it. Documenting it in README + ROADMAP keeps operator expectations correct until the pre-check ships.
