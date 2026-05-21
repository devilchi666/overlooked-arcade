# nds Decisions Log

Append-only.

---

## 2026-05-20 — melonDS as default

`melonds_libretro.dll`. DeSmuME is the per-system alternate (older,
less accurate).

---

## 2026-05-20 — POINTER infra shipped as part of NDS onboarding

**Decision:** The minimal cross-cutting POINTER input infrastructure
(RETRO_DEVICE_POINTER dispatch in oa-libretro + InputState.pointer +
device_query mouse polling in oa-input) ships as part of NDS Phase 0
rather than as standalone Phase 2 work.

**Why:** NDS games overwhelmingly depend on stylus / touch-screen
input — without POINTER dispatch, the platform's library is
essentially unplayable. Operator chose "Ship POINTER infra" over
"Defer touch entirely" during onboarding.

The infra is minimal: device_query mouse position → normalized i16
libretro POINTER range → stored per-port → returned via
cb_input_state on RETRO_DEVICE_POINTER queries. No window-relative
mapping at Phase 0 (assumes 1920×1080 screen for normalization).

Phase 2.5 polish wires window-relative pointer coordinates via Tauri
window context for pixel-perfect mapping to the game-output
rectangle. Operators with non-standard screen sizes may find touch
positioning offset until then but the input model is functional.

**Considered and rejected:**
- **Defer touch entirely.** NDS slug would ship with button-only
  bindings; stylus-dependent games (most of the library) unplayable.
  Defeated by operator preference for usable Phase 0.
- **Full per-game touch overlay UI at Phase 0.** Substantial UI work
  on top of the dispatch infra. Defeated by scope.

---

## 2026-05-20 — Multi-file BIOS check (new shape)

**Decision:** `check_nds_bios` requires ALL THREE files (bios7.bin +
bios9.bin + firmware.bin) to be present in `<exe_dir>/system/`, with
each file's SHA-1 checked against the canonical entry. Returns
OkCanonical only if all three hash-match.

**Why:** NDS BIOS is genuinely 3 separate files representing distinct
hardware components (ARM7 coprocessor BIOS, ARM9 main CPU BIOS, DS
firmware). melonDS needs all three for proper boot + region detection.

This is the first multi-file BIOS check in OA's lineup — previous
patterns either:
- Single-file BIOS (pce-cd, segacd, saturn, psx, neocd, 3do, pcfx,
  dreamcast, ps2): one .bin file → one check.
- Existence-only check (neogeo.zip): one .zip file → existence test.

NDS extends the pattern: three files → AND of three hash checks.

---

## 2026-05-20 — Pearl yellow-green 95° theme (Nintendo handheld pearl)

`oklch(0.78 0.14 95)` — pearl yellow-green in the open 90-100° band.
Matches the Nintendo handheld pearl convention (ngp 105°, also pearl
yellow-green; WonderSwan 305° pearl lavender for Bandai's competing
handheld).

DS Lite era shipped translucent shell variants (Polar White, Onyx
Black, Cobalt Blue, Mint Green, Crimson Red, Coral Pink); pearl
yellow-green is era-adjacent without committing to any single shell
color.

---

## 2026-05-20 — A is PRIMARY per Nintendo convention

NDS A button (east face, libretro A bit 8) is the primary action
button per Nintendo convention. Test fixture: `("nds", "A", "B")` —
matches nes/snes/gb/gba precedent. Keyboard Z → NDS A → libretro A
bit 8.
