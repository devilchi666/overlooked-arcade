# Per-System Custom UI — Roadmap

Stage 1 of the per-system-ui arc. Plan locked at
[docs/PLANS/per-system-ui.md](../../PLANS/per-system-ui.md). Plan §11
lists the full Stage 1 deliverables (11 code items + 6 content items +
2 doc items, ~5-7 weeks). This roadmap groups them into shippable
slices.

## Stage 1 slice plan

| Slice | Scope | Branch | Status |
| --- | --- | --- | --- |
| 1 | `SystemUIConfig` type + registry baseline for all 40+ systems + Settings → Display "Per-system experiences" master toggle + `prefers-reduced-motion` plumbing + feature-folder scaffold. No consumers yet — pure data model + toggle infrastructure. | merged | ✅ shipped |
| 2 | Per-system SFX wiring. Rust `resolve_ui_sound` resolver cascades operator override → per-system bundle at `<exe_dir>/assets/system-ui/<systemId>/sounds/<event>.<ext>` → universal baseline at `<exe_dir>/assets/system-ui/_baseline/sounds/<event>.<ext>` → silence. Frontend `playSystemUiSound` helper gates on the master toggle + per-system `audioProfile === "none"` opt-out; wired into `VirtualLibraryGrid` for gamepad-driven navigate + launch events. | merged | ✅ shipped |
| 3 | Per-system background renderer. New `apps/oa-shell/src/system_ui_assets.rs` module owns the disk-only resolver. `resolve_background_asset(systemId, kind)` cascades per-system bundle → `_baseline` → None. Frontend `<SystemBackground>` component renders three paths (`static` = CSS gradient + optional image overlay; `animated` = looping `<video>`; `shader` falls back to `static` until Slice 8). Honors master toggle. Source chain (refined during playtest): hover → focused → activeView → pinned. Static-path operator-validated; animated-path code-complete pending Slice 7 NES content. | merged | ✅ shipped |
| 4 | Boot animation framework + "Boot animations" Settings sub-toggle. New `SystemBootAnimation` component triggered by `activeSystemId` transition (sidebar-driven system entry). Sub-toggle ON + no reduced-motion → `oa-boot-fade` CSS keyframe over 1 s with radial gradient tinted by `--color-system-accent`. Sub-toggle OFF → no overlay (instant). `prefers-reduced-motion` → 200 ms cross-fade (accessibility floor, orthogonal to the toggle). Dispatches per-system `boot-intro` SFX whenever the visual fires. Skippable on any input (mouse / keyboard / gamepad). | merged | ✅ shipped |
| 5 | Tile flourish system. `interactionStyle` (instant / delayed / physical) drives the tile button's transition timing + hover transform via a `data-oa-interaction` attribute + CSS rules in `index.css`. `tileShape` (auto / square / portrait-3:4 / landscape-4:3 / wide-16:9 / circle) overrides the existing `tileAspect`. Both honor the master toggle — uniform plain library mode falls back to today's behaviour. | merged | ✅ shipped |
| 6 | Game Boy pilot — full SFX bank + static DMG gradient + custom boot animation + nintendo-handheld button labels. | TBD | pending |
| 7 | NES pilot — full SFX bank + animated palette background + custom boot animation + nintendo-console button labels. | TBD | pending |
| 8 | Vectrex pilot — phosphor shader background + custom-component escape hatch + synthesized vector-blip SFX + physical interaction. | TBD | pending |
| 9 | Per-core README updates — every `docs/cores/<id>/README.md` gets a "Per-system UI" section noting its config + signature character. | TBD | pending |

## Gate from Stage 1 → Stage 2

Per plan §11 "Ship criteria":

- Toggle works. Default ON. Operator can disable to get plain uniform library.
- All 40 systems have a `SystemUIConfig` (baseline for 37, showcase for 3).
- All 40 systems play SFX from the SFX bank on nav / select / back / launch.
- All 40 systems show a background asset (gradient default; pilots have more).
- All 3 pilots have a full custom boot animation.
- Vectrex has its custom component live (not just config).
- `cargo test --workspace` green.
- Operator playtest: launches GB → boots into themed GB → can
  navigate and launch a game → exits back to library. Same for NES
  and Vectrex.

Stage 2 design exists in plan §12 but isn't shippable until Stage 1's
playtest confirms the foundation feels right.

## Open questions before each slice

Plan §14 holds the ten open implementation questions. Each slice
answers the questions relevant to its scope at slice-planning time;
the answers land in [DECISIONS.md](DECISIONS.md).
