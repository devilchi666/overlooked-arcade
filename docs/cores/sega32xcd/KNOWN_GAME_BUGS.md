# Sega 32X CD — Known Game Bugs

Per-title quirks discovered during operator playtest. Append as
issues surface; cite the libretro core version (`picodrive_libretro`
build date / commit) and the specific behaviour seen.

## Format

```
### Game Title (Region)
- **Symptom:** what goes wrong
- **Reproducer:** how to trigger it
- **Workaround:** core option / per-game override / etc., if any
- **Upstream tracking:** issue / commit link if applicable
- **First seen:** YYYY-MM-DD, picodrive build / sha
```

## Entries

(None yet — populate during Phase 1 playtest once the operator
has a Sega CD BIOS + 32X-CD game image in hand. See
[ROADMAP.md](ROADMAP.md) Phase 1 for the gating bullets.)

## Reference

PicoDrive's 32X+CD support is functional but not historically as
polished as its plain-32X cart mode. Common quirks to watch for:

- FMV-decoding glitches on Slam City / Supreme Warrior — both are
  heavy FMV titles that exercise the CD+32X combined pipeline most
  intensely.
- Save-state collisions with plain segacd or cart sega32x — verify
  the save-state directory is sega32xcd-specific (the slug stays
  distinct in `sanitize_stem` even though oa-core routes through
  SegaCd).
