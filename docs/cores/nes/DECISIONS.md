# nes — Decisions

Per-core integration choices. Project-wide architectural decisions live in `docs/DECISIONS.md`.

---

## 2026-05-18 — FCEUmm as the default, Mesen as the accuracy alternative

OA defaults to `fceumm_libretro.dll` because (a) it's the long-standing libretro NES default with broad mapper compatibility, (b) it's lighter on CPU than Mesen, (c) it's the .dll most casual operators already have from RetroArch installs. The PerSystemSettingsPage → Cores tab lets the user swap to `mesen_libretro.dll` (cycle-accurate, slightly heavier) without any further configuration.

**Alternative considered:** Default to Mesen since it's more accurate. Rejected — for a launcher targeting "playable, not cycle-accurate" (a CLAUDE.md locked design pillar), FCEUmm is the right baseline. Power users who care about cycle-accuracy already know to swap.

---

## 2026-05-18 — `.fds` extension included in the scanner, BIOS responsibility on the operator

Famicom Disk System games (`.fds`) need `disksys.rom` (SHA-1 `5C891EB05680B61438EDBC4C3D77F9C7DC4E8FCA`) in `<exe_dir>/system/`. The scanner picks up `.fds` without checking for the BIOS — if it's missing, the launch will fail with the core's own "BIOS not found" error rather than OA refusing to scan the ROM. This mirrors the tg16 BIOS-check pattern: ROMs are visible in the library even without their BIOS so the operator knows what to install.

---

## 2026-05-18 — NSF files excluded from the scanner

NES Sound Format (`.nsf` / `.nsfe`) files are audio-only — the NES's APU exposed as a portable music format. Including them in the launcher's ROM scan would surface them as launchable tiles, but the result would be "open the game window, hear chiptune, no visuals." That's not what users expect from a library tile. A future "Tracks" sidebar destination could host audio formats separately.
