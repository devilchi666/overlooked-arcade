# Per-System Settings Hub

**Status:** In flight (planned + S1 shipped 2026-06-14). Builds on the merged
Unified Navigation Phase 1 spatial engine.

## What this is

Consolidates **all** per-system settings — today scattered across the
"Per-system", "Media", "Metadata" categories, the Library→Game-media grid, and
BIOS in System Health — into one card-based **Systems** hub inside Settings.

Two-level card model (operator's chosen IA):
1. **Systems grid** — one card per system (the Game-media card look), library-first
   with a Show-all toggle.
2. **System hub** — click a card → a grid of **domain cards** (Display & Video ·
   Input · Core/Launcher · Media · Metadata · BIOS) → click a domain → its editor.

Clean **replace** of the scattered surfaces; built on the spatial-nav engine
(zero per-control wiring); delivers the unified-nav **Pillar B** panel primitives
(`HubCard` / `HubGrid` / `PanelScaffold`) as a byproduct.

## Source of truth

Plan + slices + reuse map + risks:
**[../../PLANS/per-system-settings-hub.md](../../PLANS/per-system-settings-hub.md)**.
IA decisions: [DECISIONS.md](DECISIONS.md). Progress: [SESSION_LOG.md](SESSION_LOG.md).

Code lives under `frontend/src/engine/systemsHub/`.
