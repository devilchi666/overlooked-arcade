# tg16 — TurboGrafx-16 / PC Engine (HuCard)

First core online. Wraps **Beetle PCE Fast** (libretro port of Mednafen's PCE Fast module) via `crates/oa-pce-sys` (raw FFI + C++ shim) and `crates/oa-pce` (safe wrapper, `oa_core::Core` impl).

## Upstream

- **Source:** https://github.com/libretro/beetle-pce-fast-libretro
- **Vendored:** 2026-05-15 (shallow clone)
- **License:** GPL-2.0-or-later
- **Tree:** `crates/oa-pce-sys/vendor/` — see `vendor/ORIGIN.md` for layout notes.
- **Local patches:** `crates/oa-pce-sys/vendor/PATCHES/` — empty at vendor time. Add numbered `.patch` files as we modify upstream.

## Build surface

- `crates/oa-pce-sys/build.rs` compiles ~93 object files from `vendor/mednafen/**` + `vendor/libretro-common/**` + `deps/libchdr` + zlib/zstd/lzma/tremor → 9.6 MB C++ archive + 3.4 MB C archive linked into the binary.
- `crates/oa-pce-sys/shim.cpp` (~290 LOC) implements libretro's frontend callbacks (`retro_set_video_refresh` etc.), pixel-format conversion (RGB565 / 0RGB1555 / XRGB8888 → RGBA8), and exposes a small `oa_pce_*` C surface: `new / free / load_rom / reset / run_frame / framebuffer / audio_samples / set_input / save_state` (last is TODO).
- `crates/oa-pce-sys/src/lib.rs` — hand-written `extern "C"` bindings (no bindgen — see project `DECISIONS.md`).
- `crates/oa-pce/src/lib.rs` — `PceCore: Core`. Owns the singleton handle, runs `oa_pce_free` on Drop, translates our PCE button bitfield to libretro's `RETRO_DEVICE_ID_JOYPAD_*` numbering. Button constants in `oa_pce::buttons::{I, II, SELECT, RUN, UP, DOWN, LEFT, RIGHT}`.

Required `#define` shims captured at the top of `build.rs`: `INLINE=__inline`, `MEDNAFEN_VERSION_NUMERIC=931`, `STDC_HEADERS`, `__STDC_LIMIT_MACROS`, `_LOW_ACCURACY_`, and ~15 others. MSVC also needs `/EHsc /std:c++14` for Mednafen's pre-C++11 throw/try idioms.

## Native timing

- **Resolution:** 256×239 NTSC default (most games); shim reallocates the framebuffer up to 565×242 when the game switches modes (Bonk's Adventure runs 256×243).
- **Frame rate:** 59.826 Hz Mednafen-canonical.
- **Audio:** 44.1 kHz stereo i16.

## Current status (2026-05-16)

**Works:**
- Bonk's Adventure (USA, 384 KB HuCard) — 60.05 fps measured, audio at correct pitch, keyboard + gamepad input both live.
- Framebuffer mode-change detection (256×239 → 256×243 on ROM load).
- Audio: cpal default output via `oa-audio`, SPSC ringbuf, linear-interp resampler from 44.1 kHz to host rate (Windows typically 48 kHz). Zero drops over multi-minute runs.
- Input: keyboard via `device_query` (arrows / Z / X / Enter / RShift), gamepad via `gilrs` (dpad / east=I / south=II / start=RUN / select=SELECT). Gamepads bind to ports in connection order.

**Not working / not implemented:**
- Save states — shim doesn't expose `retro_serialize` / `retro_unserialize` yet; `PceCore::save_state` returns `CoreError::Internal`.
- Multitap (ports 2-5) — Mednafen supports it; we wire it through the bitfield but haven't tested.
- Focus-gated input — Tauri's `is_focused()` returns false for our no-WebView game window, so input polls unconditionally for now (see `feedback_tauri_no_webview_is_focused_unreliable` memory).
- PCE-CD — out of scope for this core; Phase 5 may vendor full Mednafen PCE separately.

## Per-core docs

- `ROADMAP.md` — phase tracking for tg16 specifically.
- `SESSION_LOG.md` — Shipped / Almost / Next per session.
- `KNOWN_GAME_BUGS.md` — per-game compatibility issues as they surface.
- `DECISIONS.md` — tg16-specific integration choices and the *why* behind them.

Project-wide context (license, stack, multi-core architecture) lives in `docs/DECISIONS.md` and `CLAUDE.md`.
