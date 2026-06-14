# Unified Navigation & Panel System

**Status:** Planned 2026-06-14 (operator's chosen direction). Supersedes the
per-panel Controller-Nav Coverage approach for the engine surface.

## Why this exists

Per-panel nav wiring doesn't scale: every surface had to mount a focus group AND
tag each control, so most engine panels (Library sub-tabs, Media, Metadata,
System Health, Profile, About, most dialogs) stayed inert. The operator's call:
build a **unified system so any panel just works** across keyboard / controller /
kiosk-arcade controls, and **restructure the panels themselves** into a
consistent, input-agnostic structure and look.

## Two pillars

- **A — Spatial navigation engine:** universal focusable auto-discovery (native
  selector, no markers) + geometry-based movement + layer/modal scoping, reusing
  the Slice-1/2/3 activate layer (button/checkbox/radio/select-overlay/
  slider-adjust/Y-reset/OSK) + the input bus / back stack / HintBar / focus ring.
- **B — Unified panel structure & look:** a shared PanelScaffold + consistent
  control-row vocabulary + standard tab pattern, so panels read and traverse the
  same way and are kiosk-ready.

## Source of truth

Design + phases + the fold-in of nav-coverage Slices 1–3:
**[../../PLANS/unified-navigation-and-panels.md](../../PLANS/unified-navigation-and-panels.md)**.

Predecessor history (the activate layer this engine reuses, the audit, the OSK
deferral): [../nav-coverage/](../nav-coverage/).

## Phase 1 (next)

Engine + universal discovery, proven on the whole Settings surface (incl. the
embedded sub-pages), with the Pillar-B scaffold applied to Settings as the
reference adopter. Queued in [../../NEXT.md](../../NEXT.md) HIGH band.
