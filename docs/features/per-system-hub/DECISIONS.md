# Per-System Settings Hub — Decisions

Append-only. Newest at the bottom. Each entry: date + the *why*.

---

## D1 — Two-level card model (2026-06-14)

Per-system settings were scattered across five surfaces. The operator chose a
**card-fronted hub**: a Systems grid (card per system) → a per-system hub of
**domain cards** (Display & Video / Input / Core-Launcher / Media / Metadata /
BIOS) → a domain editor. Rejected alternatives: a single long scrolling detail
page, and a sub-tab strip. Rationale: cards are "the neatest and most logical
way to group settings for systems," read well on controller/kiosk, and reuse the
Game-media card look the operator already loves. Cost: one extra click to reach a
setting vs a flat page — accepted.

## D2 — Clean replace, not additive (2026-06-14)

The hub **replaces** the separate "Per-system", "Media", "Metadata" Settings
categories AND the Library→Game-media grid, rather than living alongside them.
Rationale: a single home is the whole point; two paths to the same setting would
drift. Library keeps folders/views only. Done as the final slice (S5) once each
domain reaches parity in the hub — old surfaces stay live until then so nothing
regresses mid-arc.

## D3 — Library-first grid with Show-all (2026-06-14)

The Systems grid defaults to systems that have games (matching the Game-media
grid), with a Show-all toggle revealing all ~45 known systems. Rationale:
relevant by default, but you can pre-configure a system (art/bindings/BIOS)
before importing its ROMs.

## D4 — Built on the spatial engine, no new nav model (2026-06-14)

The hub renders inside SettingsPanel's existing `data-nav-region="settings-content"`
and uses native `<button>` cards, so the Phase-1 spatial engine drives it by
geometry with **zero per-control wiring**. The only nav integration is the
in-pane Back: one `pushBackHandler` registered while drilled in pops a hub level
before the takeover's onCancel closes Settings. The hub's `HubCard`/`HubGrid`/
`PanelScaffold` are the unified-nav **Pillar B** panel primitives — this arc
delivers them and supersedes the standalone "Pillar B on Settings" task.

## D5 — Per-game metadata stays separate (2026-06-14)

The Metadata domain card edits **system** facts only. Per-game metadata
(`MetadataGamePane`) is NOT pulled into the hub — it stays reachable from its own
entry point (game detail). The hub is per-system by design.
