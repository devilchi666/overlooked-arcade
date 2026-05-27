# Per-System Custom UI — Roadmap

Stage 1 of the per-system-ui arc. Plan locked at
[docs/PLANS/per-system-ui.md](../../PLANS/per-system-ui.md). Plan §11
lists the full Stage 1 deliverables (11 code items + 6 content items +
2 doc items, ~5-7 weeks). This roadmap groups them into shippable
slices.

## Stage 1 slice plan

| Slice | Scope | Branch | Status |
| --- | --- | --- | --- |
| 1 | `SystemUIConfig` type + registry baseline for all 40+ systems + Settings → Display "Per-system experiences" master toggle + `prefers-reduced-motion` plumbing + feature-folder scaffold. No consumers yet — pure data model + toggle infrastructure. | `feat/per-system-ui-stage-1-slice-1` | shipping |
| 2 | Per-system SFX wiring. Universal CC0 click sound on nav / select / back / launch routed through the existing `ui-sounds` mixer bus. Asset bundle structure at `<exe_dir>/assets/system-ui/<system>/sounds/`. Honors master toggle + reduced-motion. | TBD | pending |
| 3 | Per-system background renderer. Static gradient default (driven by existing `systemThemes[id].accent`); `background` enum gates which renderer path runs. | TBD | pending |
| 4 | Boot animation framework + "Boot animations" Settings sub-toggle. Skippable on any input. Reduced-motion shortcuts to 200 ms fade. | TBD | pending |
| 5 | Tile flourish system. `interactionStyle` (instant / delayed / physical) wires into focus animations; `tileShape` overrides the existing `tileAspect`. | TBD | pending |
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
