# Sega Titan Video (ST-V) — Known Game Bugs

Per-title quirks discovered during operator playtest. Append as
issues surface; cite the MAME core version (`mame_libretro` build
date / commit) and the specific behaviour seen.

## Format

```
### Game Title (Region)
- **Symptom:** what goes wrong
- **Reproducer:** how to trigger it
- **Workaround:** core option / per-game override / etc., if any
- **Upstream tracking:** MAME issue / commit link if applicable
- **First seen:** YYYY-MM-DD, mame build / sha
```

## Entries

(None yet — populate during Phase 1 playtest once the operator
has an ST-V BIOS + ROM set in hand. See [ROADMAP.md](ROADMAP.md)
Phase 1 for the gating bullets.)

## Reference

MAME's stv driver is mature; most quirks are at the level of
"this specific title has a known timing issue documented in MAME's
upstream tracker" rather than driver-wide regressions. Cross-reference
the MAME upstream issue tracker before adding entries here so the
notes link out to the canonical fix-or-status-tracking.
