# Atari Jaguar CD — Known Game Bugs

Per-title quirks discovered during operator playtest. Append as
issues surface; cite the libretro core version (`virtualjaguar_libretro`
build date / commit) and the specific behaviour seen.

## Format

```
### Game Title (Region)
- **Symptom:** what goes wrong
- **Reproducer:** how to trigger it
- **Workaround:** core option / per-game override / etc., if any
- **Upstream tracking:** issue / commit link if applicable
- **First seen:** YYYY-MM-DD, virtualjaguar build / sha
```

## Entries

(None yet — populate during Phase 1 playtest. See
[ROADMAP.md](ROADMAP.md) Phase 1 for the gating bullet.)

## Reference

Virtual Jaguar's CD support is the most fragile part of the core
upstream. Some retail titles (Highlander I, Black Ice/White Noise)
have boot issues even on real hardware; verify against MAME's
Jaguar CD driver if a specific title refuses to launch and the
behaviour seems hardware-faithful.
