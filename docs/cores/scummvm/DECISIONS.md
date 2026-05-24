# scummvm Decisions

Per-core architectural decisions. Date entries.

---

## 2026-05-24 — Ship as an ordinary OA system, not a separate launcher

Locked in the [features/dosbox-and-scummvm/README.md](../../features/dosbox-and-scummvm/README.md) plan. ScummVM (and Phase 2's DOSBox) appear in the same sidebar / library / per-game settings model as console systems. No separate "PC games" surface, no new launcher app, no new top-level concept. The engine-launcher difference is contained inside the launch dispatch (descriptor file → `RomSource::Path` → core opens game data in same directory) and the per-system bindings module. Everything downstream — cover art, save states, per-game overrides, audio overrides — works the same as for console games.

## 2026-05-24 — `.scummvm` descriptor scan, no auto-detection in v1

OA scans for `.scummvm` descriptor files at any depth under the library folder. The descriptor is the operator-curated "this is a game" marker. v1 does NOT wrap ScummVM's `--detect` auto-detector — operators (or LaunchBox's ScummVM importer) create descriptors manually. Auto-detect wrapping is a Phase 2 polish item; for now the manual-create flow matches how every other LaunchBox-aware tool handles ScummVM and keeps the v1 scope bounded.

## 2026-05-24 — Per-core `system_dir` subdirectory

Engine-launcher cores ship their own engine plugins / themes / runtime config that live in dedicated subdirectories rather than alongside console BIOSes. ScummVM gets `<exe_dir>/system/scummvm/` (created on first launch by `system_dir_for` in `apps/oa-shell/src/main.rs`) so the top-level `<exe_dir>/system/` doesn't accumulate engine-specific files (`theme/`, `extra/`, etc.) that would clutter the cart-shape BIOS folder. Console cores keep the install-wide `<exe_dir>/system/` they've always used.

## 2026-05-24 — `RomSource::Path` always (descriptor extension)

ScummVM's libretro core sets `need_fullpath = true` because it opens additional files (game data, save state files, engine config) relative to the descriptor's path. The shell's launch dispatch routes `.scummvm` through `RomSource::Path` via the new `is_descriptor_extension` helper — parallel to how CD images go through `RomSource::Path` because the core opens .bin tracks relative to the .cue. The descriptor file itself is tiny (~50 bytes) but the path matters, not the bytes.

## 2026-05-24 — No BIOS check, no SHA-1 dat

ScummVM has no BIOS (it ships its own engine plugins). Game data files vary by release — different revisions, language packs, fan translations — so libretro-database doesn't ship a canonical SHA-1 set for ScummVM games. Cover sync falls back to fuzzy filename matching at the shared 0.95 threshold; this works fine because LaunchBox's ScummVM art pack keys covers on the canonical game title and operators (or the LaunchBox importer) name `.scummvm` files after the canonical title ("Monkey Island.scummvm"). Both `rom_hashes::libretro_dat_refs_for_system` and the cart-shape BIOS dispatch return no-op for `"scummvm"`.

## 2026-05-24 — Keyboard passthrough on by default

`default_keyboard_passthrough("scummvm") = true`. Text input drives a meaningful slice of the canonical library — the Monkey Island sword-fighting insults, password prompts in Indiana Jones / Loom / Zak McKracken, typed verb input in SCI titles. Without passthrough those games are stuck. Same precedent as `mame` / `msx` / `5200` — systems where the keyboard is part of the game, not an operator-mode UI affordance.

## 2026-05-24 — Mouse-primary bindings; pointer via shared POINTER infra

Per-system Bindings exposes 8 fallback bits (d-pad cursor + LMB + RMB + ESCAPE + PAUSE) for users without a mouse. The actual mouse cursor flows through OA's shared POINTER input infrastructure (the same path NDS stylus + DC pointer + PSP touch use). The bindings table is the RetroPad fallback surface; pointer cursor is platform-level. Keeps the per-system Bindings page UI consistent across systems without piling pointer-specific controls into the bindings model.

## 2026-05-24 — Theme: teal-cyan 195° L=0.62 C=0.16

Period-correct to the adventure-game era's dominant teal-cyan UI palette — Sierra's classic GUI panels, ScummVM's own GUI theme defaults, the Monkey Island ocean. Hue 195° sits in the open band between coleco cyan (also 195° at L=0.72) and PCE-CD silver-cyan (220° at L=0.72); ScummVM's lower L=0.62 separates it from coleco at a glance, AND its unique tileAspect "1/1" (the only square tile in the OA lineup) makes mixed library views distinguish it without relying solely on hue.
