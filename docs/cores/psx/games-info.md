# PlayStation — Game Info

Per-game structured reference data for the Game Info Panel. Format reference:
`docs/cores/SCHEMA.md`. v1 seed entries — most coverage arrives in Phase 10
via the `KNOWN_GAME_BUGS.md` migration pass.

# ============================================================================

---
id_key:
  system_id: psx
  rom_hash: 7a4b00112233445566778899aabbccddeeff0011
  rom_title: "Tomb Raider (USA)"

date: 1996
publisher: Eidos Interactive
region: USA
version: "1.0"
player_count: 1
genre: Action-Adventure

short_summary: ""
controls_supported:
  - "Standard gamepad"
  - "DualShock vibration"
best_emulator:
  recommended: "beetle_psx_hw_libretro.dll"
  reason: "PGXP + Vulkan renderer eliminates the depth-buffering glitches of the SW renderer."

bugs:
  - description: "Crashes when entering Caves of Kaliya without prior save."
    severity: blocker
    workaround: "Save in the previous room first."
  - description: "Audio cuts in pre-rendered cutscene at start of Egypt level."
    severity: minor

meta:
  schema_version: 1
  last_updated: "2026-05-30"
  contributors: []

---
# Final Fantasy VII — Phase 10 migration will likely add the multi-disc-
# specific notes from KNOWN_GAME_BUGS once the migration script runs.
id_key:
  system_id: psx
  rom_hash: 1111222233334444555566667777888899990000
  rom_title: "Final Fantasy VII (USA)"

date: 1997
publisher: SCEA
region: USA
version: "1.1"
player_count: 1
genre: RPG

controls_supported:
  - "Standard gamepad"
  - "Analog stick"
best_emulator:
  recommended: "beetle_psx_hw_libretro.dll"
  reason: "HW renderer + 4× internal-resolution scaling cleans up pre-rendered backgrounds without warping FMV cutscenes."

meta:
  schema_version: 1
  last_updated: "2026-05-30"
  contributors: []
