# Unified Navigation & Panel System — Decisions

Append-only. Newest at the bottom. Each entry: date + the *why*.

---

## D1 — Movement model = region-bias hybrid (2026-06-14, Phase 1)

The plan left the movement model open ("pure spatial geometry vs. hybrid —
validate feel on Settings before committing"). Built pure-spatial first; the
playtest log showed the whole Settings surface was **one flat plane of 73
focusables**, so a Down press from the Library sub-nav dove to the far-left
"Themes" category — geometrically valid, but disorienting.

**Decision:** region-bias hybrid — **UP/DOWN move within the focused element's
region; LEFT/RIGHT cross between regions** at an edge. This matches the
operator's already-locked nav spec (the Retroverse controller-nav contract:
up/down within region, left/right between sidebar↔center↔right pane), so the
engine and the rest of the shell speak the same model.

**Regions are derived, not hand-wired:** nearest `[data-nav-region]` (override
hook) → semantic landmark (`aside`/`nav`/`role=navigation`/`role=tablist`) →
the layer container (catch-all). This keeps "zero per-control wiring": sidebars
are already `<aside>`/`<nav>`. The one explicit marker added in Phase 1 is
`data-nav-region="settings-content"` on the Settings center pane, because that
content column isn't a landmark and otherwise fell into the layer catch-all
(stranding the embedded Library sub-nav). Pillar B's PanelScaffold will set
`data-nav-region` structurally, retiring ad-hoc markers.

## D2 — `lastFocused` is the engine's source of truth (2026-06-14, Phase 1)

A hidden `<select>` inside a collapsed `<details>` could be *discovered*
(non-zero box in the layout) but *refuse* `.focus()`. With `document.activeElement`
as the "current element", focus stayed on the prior control, so every press
recomputed from the same origin and re-picked the hidden select → an infinite
wall.

**Decision:** the engine's own `lastFocused` (painted ring) is the source of
truth, kept in sync with mouse/Tab via a `focusin` listener; `document.active
Element` is only a fallback. Worst case a press lands on a non-focusable
element once and the next press moves on — movement can never get pinned.
Belt-and-suspenders: discovery also skips focusables inside a collapsed
`<details>` (everything but the summary).

## D3 — Engine-first; Pillar B (panel scaffold) follows (2026-06-14, Phase 1)

Phase 1's plan names both the spatial engine (Pillar A) and a unified panel
structure (Pillar B). **Decision:** ship Pillar A and prove it on the live
Settings surface first, validate feel, THEN do the Pillar-B restructure. The
engine is the risky, feel-dependent half; restructuring panels before knowing
the movement model felt right would have been rework. Trade-off accepted: the
Settings HintBar labels are stale ("Switch region"/"Prev tab") until Pillar B.

## D4 — Dialogs + custom modals covered in Phase 1 (2026-06-14, Phase 1)

The plan sequences `Dialog` scope-integration into Phase 2. But Settings-reachable
dialogs (Bindings, Core options) and custom modals (Import Wizard, Game-media
panel) would have *regressed* (un-navigable) under a Settings-only engine.

**Decision:** pull the container integration forward for the Settings-reachable
set — `Dialog` branches to a `SpatialDialogLayer` when the engine is active, and
custom modals adopt a reusable `SpatialModalScope` wrapper ("wire the scope once
into the container") + a z-lift above the takeover. Phase 2's job narrows to
formalizing this and retiring the legacy `Dialog.navigate` markers.
