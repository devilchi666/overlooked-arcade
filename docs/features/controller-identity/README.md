# Controller Identity & Auto-Config

Cross-cutting input-infrastructure arc. Gives every controller a **stable
identity** (VID/PID) and **auto-configures** both shell-nav and per-system
gameplay bindings from shared data files, with a wizard for unknown pads.

- **Plan:** [../../PLANS/controller-identity-substrate.md](../../PLANS/controller-identity-substrate.md)
  (full design, phases, decisions D1–D8, risks).
- **Status:** Planning locked 2026-06-12; execution queued **foundation-first**
  (Phase 0 = the Rust VID/PID spike). Not yet started in code.

## The one-paragraph why

OA has **no controller identity layer** — both input pipelines (menu
WebView poller + emulator gilrs poller) are positional and assume the Web
Gamepad "standard layout," so non-standard pads (e.g. a wired Switch Pro)
get their buttons mapped wrong, and a replug shuffles the ports. The fix:
keep the two pollers, give them **one shared config** — three data files
(`controllers.json` layout DB, `systems-input.json` schema,
`default-maps.json` canonical→system defaults) plus a press-the-buttons
wizard. Normalize every pad to one canonical layout once, then the existing
nav verb map + per-system gameplay maps "just work" for any controller.

## Composes with (does not merge)

- [../../PLANS/dynamic-controller-info.md](../../PLANS/dynamic-controller-info.md)
  — the core's *supported* device list (device-type dropdown).
- [../../PLANS/dynamic-input-descriptors.md](../../PLANS/dynamic-input-descriptors.md)
  — per-game *labels* ("B = Whip").

This arc is the PHYSICAL-pad side; those are the emulated-console side.

## Files

- `README.md` (this) · `DECISIONS.md` (the why) · `SESSION_LOG.md`
  (Shipped/Almost/Next per session).
