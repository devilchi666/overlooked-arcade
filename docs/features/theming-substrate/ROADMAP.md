# Theming Substrate — Roadmap

ARC 1 of the theming-substrate arc. Plan locked at
[docs/PLANS/theming-substrate.md](../../PLANS/theming-substrate.md).
Plan §6 holds the full phase breakdown (acceptance gates, critical
files, decisions). This roadmap is the slice-by-slice status board.

## ARC 1 phase plan

| Phase | Scope | Branch | Status |
| --- | --- | --- | --- |
| 1 | Engine/Theme surface separation. Extract Settings + Library Manager + Import Wizard + BIOS + Core installer + System Health + Background Jobs into engine-owned fullscreen-takeover surface summoned via `F12` / `Select+Start` / top-right corner icon. Retroverse drops from 6 → 5 tabs. Write SURFACES.md. | `feat/theming-substrate-phase-1` | ✅ shipped 2026-06-06 (in `engine/`, `platform/engineSurface.ts`, `platform/dialogs.ts`, `engine/SettingsPanel.tsx` + App.tsx wiring; SETTINGS dropped from `routing/currentRoute.ts`; SettingsPage.tsx deleted; 744 oa-shell tests pass; typecheck silent) |
| 2 | Platform / Theme SDK foundation. Move shared layer to `frontend/src/platform/` (`@oa/platform` alias). Define `theme.toml` manifest schema + `ThemeContext` interface. Rename Retroverse* → Theme* generics. ESLint boundary rule. Cleanup `HOTSPOT_SYSTEMS` triplicate + `customComponent` orphan. | (multi-slice) | ✅ Slice A shipped 2026-06-06. Slice B shipped 2026-06-07 (stores + lib + themes moves: `platform/{lib,themes,settings,library,layout/state,views}/` with all importers rewritten to `@oa/platform/*` alias; `SidebarView` type extracted to `platform/layout/types.ts`; ~130 import sites rewritten). Slice C shipped 2026-06-07 (13 component moves into `platform/components/` — the 6 declared + the 7-file private sub-component cluster per operator decision; `RetroverseContext` → `ThemeContext` / `useRetroverse()` → `useTheme()` rename; `ThemeManifest` type in `platform/theme/manifest.ts`; 790 tests pass; typecheck silent; awaiting operator playtest before merge). Phase 2 complete modulo the ESLint boundary rule, deferred to Phase 4 by operator decision. Residual wrong-direction imports: `platform/components/*` → `../../nav/{focus,back,HintBar}` (nav becomes platform-owned in Phase 3) + SystemHeader → `../../components/SystemCoresStrip` (core-installer surface, engine territory). |
| 3 | Theme substrate — layout + palette + assets. Per-system palette extracted to JSON. Asset resolver generalized to take `theme_id`. 5 nav primitives (grid / wheel / list / carousel / custom). Toy 2nd theme proves substrate. | TBD | ⬜ pending |
| 4 | Tauri bridge hardening. Drain ~150 `invoke()` leakage sites into `platform/api/` domain modules. SystemId parity test (CI). Hand-written typed wrappers (defer tauri-specta). | TBD | ⬜ pending |
| 5 | `.oatheme` distribution + loading. Zip + loose-folder dev mode. Rust loader (`theme_loader.rs`). Theme picker UI (Manager → Appearance). Conflict + fallback policies. Build-time bundling only (runtime load deferred to ARC 2). | TBD | ⬜ pending |
| 6 | Retroverse rebuilt + 2nd pilot theme. **ARC 1 ACCEPTANCE GATE.** Retroverse-as-theme (same UX) + Wheel pilot (BigBox-inspired coverflow) proves SDK supports >1 IA shape. Both ship in binary. Operator switches in Appearance. | TBD | ⬜ pending |

## Gate from Phase 2 → Phase 3 (sequencing pause)

Per plan §7, ARC 1 pauses at end of Phase 2 to let the Virtual
Library arc's Phase E (game_identities schema, ~3-4 weeks) +
Phase C (Launcher trait, ~2-3 weeks) land. Resume Phase 3 after
both VL phases ship.

## Gate from ARC 1 → ARC 2

Per plan §11:

- All 6 phases' acceptance gates green
- `cargo test -p oa-shell` green
- `npm run typecheck` + `npm run lint` silent
- SystemId parity test green
- Operator dogfood: Retroverse + Wheel switchable; Retroverse
  feels indistinguishable from pre-arc behavior; Wheel boots /
  browses / launches
- Third hand-built theme folder dropped at `<exe_dir>/themes/test/`
  → appears in Appearance picker → loads

ARC 2 (Behaviors + Shaders — Rhai + WGSL) gets its own plan when
scheduled. KIOSK_PLAN §2.2 is the source spec; doesn't need to be
written from scratch.
