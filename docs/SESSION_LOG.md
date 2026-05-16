# Session Log

Project-wide milestone log. Per-core day-to-day work goes in `docs/cores/<core>/SESSION_LOG.md`. This file is for cross-cutting milestones (phase boundaries, shell-level shipped features, new systems coming online).

Format: date + three lines — **Shipped / Almost / Next**.

---

## 2026-05-15 — Project bootstrap + Spike 1

- **Shipped:** Approved setup plan (`C:\Users\Devilchi\.claude\plans\jazzy-spinning-blum.md`); Day-1 docs scaffolding (`CLAUDE.md`, full `docs/`, `LICENSE` stub, `NOTICES.md`, `.gitignore`); memory bootstrapped (3 entries); Rust 1.95.0 + LLVM 22.1.5 installed via winget. **All three Phase 0 spikes passed same day:** (1) two-window Tauri+wgpu @ 60.2 fps; (2) `cc-rs` builds Beetle PCE Fast Mednafen C on MSVC with one shim define; (3) hand-written FFI chosen over bindgen (4.8× faster builds, no libclang dep, idiomatic enum names). **Phase 1 skeleton landed same day:** full Cargo workspace (10 crates + `oa-shell` binary); `oa-core` trait designed (Core, Framebuffer, Timing, InputState, PortIndex, SystemId, CoreError); Beetle PCE Fast vendored at `crates/oa-pce-sys/vendor/` (5.5 MB) with ORIGIN.md + PATCHES/; `oa-pce-sys` compiles the proven Mednafen endian helpers; `oa-pce::PceCore` stubs `Core` with native timing (256×239 @ 59.826 Hz, 44.1 kHz). Workspace `cargo build` clean in 78s; `cargo test` 4/4 pass; `oa-shell` boots cleanly, opens Tauri WebView, ticks PceCore on emu thread at observed 59.9 Hz for 1320+ frames.
- **Almost:** Real PCE emulation. Today's PceCore is a stub that paints a frame-counter gradient — the renderer, audio sink, and input poller are all wired but produce no output yet. Beetle PCE Fast's full `pce_fast/*.cpp` set is not yet in the cc-rs build.
- **Next:** Two parallel tracks. (a) Expand `oa-pce-sys/build.rs` to compile the full pce_fast core (huc6280.c, vdc.c, psg.cpp, input.c, huc.h... -driven .cpp set), discover and document each integration shim as it surfaces, and write the C++ shim layer exposing `oa_pce_new` / `oa_pce_load_rom` / `oa_pce_run_frame` / `oa_pce_framebuffer` (per the Spike 3 surface). (b) Replace the `oa-render` stub with a real wgpu pipeline that takes a `Framebuffer<'_>` and draws it on the game window from Spike 1.

---

## 2026-05-15 — Renderer + two-window integration (still same day)

- **Shipped:** Real `oa-render::Renderer` (wgpu Surface + RGBA8 texture cache + fullscreen-triangle blit pipeline + nearest-neighbour sampler + WGSL shader at `crates/oa-render/shaders/blit.wgsl`). `oa-shell` now opens BOTH the library WebView AND a native game window with a wgpu surface attached — combined emu+render thread ticks `PceCore` at native rate and presents each frame. Steady **59.8 fps for 3360+ frames** observed; PCE-stub gradient visible in the game window. Three small bugs found and fixed during integration: wgpu 23 still uses the old `ImageCopyTexture`/`ImageDataLayout` names (not the `TexelCopy*` names from 24+); `raw-window-handle` must be a direct dep of `oa-shell`; `RawWindowHandle` is `!Send` so handle extraction must happen INSIDE the spawned thread, not before the spawn.
- **Almost:** Real PCE emulation. The renderer is now production-grade for the blit path; the wrapped C core is still a Rust stub painting a gradient. Audio sink and input poller are still stubs.
- **Next:** The PCE C-core build. Two sub-steps: (a) expand `oa-pce-sys/build.rs` to compile the full `vendor/mednafen/pce_fast/*.cpp` set — discover and document each integration shim define as it surfaces; (b) write a C++ shim layer (`crates/oa-pce-sys/src/shim.cpp`) wrapping Mednafen's globals into the `oa_pce_*` surface from Spike 3, and rewrite `oa-pce::PceCore` to call through that.

---

## 2026-05-15 — Beetle PCE Fast full C/C++ build compiles (still same day)

- **Shipped:** `oa-pce-sys/build.rs` now compiles the **entire** Beetle PCE Fast / Mednafen PCE Fast core tree — 93 object files, 9.6 MB C++ archive + 3.4 MB C archive linked into the Rust binary. Covers `pce_fast/*`, all mednafen helpers, `libretro.cpp` (the core engine), `libretro-common/*` portability layer, `mednafen/cdrom/*` for CD support, `mednafen/sound/Blip_Buffer.c` for audio resampling, `mednafen/tremor/*` for integer Vorbis (CD audio), and the full `deps/libchdr` + `lzma-19.00` + `zstd/lib` + `zlib-1.2.11` chain for CHD CD-image support. The 2 endian FFI bridge tests still pass through the much-larger native lib. **Two real iteration findings** captured in build.rs comments: (1) Mednafen-derived trees need `MEDNAFEN_VERSION_NUMERIC=931` + `STDC_HEADERS` + `__STDC_LIMIT_MACROS` + `_LOW_ACCURACY_` defines, not just `INLINE`; (2) MSVC C++ wants `/EHsc` + `/std:c++14` for Mednafen's pre-C++11 idioms with throw/try/catch. Build is incremental-fast (~16s) after the cold compile.
- **Almost:** A real PCE ROM running. The library compiles and links but no symbols are exposed to Rust yet — `oa-pce-sys/src/lib.rs` still binds only the endian helpers from the spike. The C++ shim layer translating libretro's frontend-callback API into our `oa_pce_*` surface is the next step.
- **Next:** Write `crates/oa-pce-sys/src/shim.cpp`. Provides the 5 libretro frontend callbacks (`retro_set_video_refresh` etc.), wires `retro_init` / `retro_load_game` / `retro_run` / `retro_serialize` into `oa_pce_new` / `oa_pce_load_rom` / `oa_pce_run_frame` / `oa_pce_save_state` (from the Spike-3 surface), and stashes the latest video frame + audio samples for the wrapper to read. Then update `oa-pce::PceCore` to call through, and oa-shell loads a real HuCard ROM from disk.

---

## 2026-05-15 — Bonk's Adventure runs 🎉 (still same day)

- **Shipped:** `crates/oa-pce-sys/shim.cpp` (~280 LOC) — full libretro frontend implementation: video/audio/input/environment/log callbacks, RGB565→RGBA8 + 0RGB1555→RGBA8 + XRGB8888→RGBA8 pixel conversion, retro_init/retro_load_game/retro_run/retro_unload_game/retro_deinit lifecycle wired into the `oa_pce_*` surface from Spike 3. Hand-written `extern "C"` bindings in `oa-pce-sys/src/lib.rs`. `oa-pce::PceCore` rewritten to call through (handle pointer wrapped in `NonNull`, Drop runs `oa_pce_free`, button-bit remap from our PCE layout to libretro's RETRO_DEVICE_ID_JOYPAD_* numbering). `oa-shell` reads `OA_ROM` env var and pipes the bytes through. **Bonk's Adventure (USA), 384 KB HuCard, runs at steady ~60 fps for 813 frames** — Beetle reports back `Samples / Frame: 734.3` (== 44100/60.05) and `Estimated FPS: 60.05`. The renderer detected the 256×243 PCE mode automatically and reallocated its FB texture from the 256×239 pre-load default. The Phase 1 acceptance gate is functionally hit.
- **One integration gotcha worth recording:** Beetle's `retro_load_game` doesn't accept `info->data` directly — when the frontend doesn't provide a file path, the core takes the `RETRO_ENVIRONMENT_GET_GAME_INFO_EXT` path, expecting the frontend to return a `retro_game_info_ext` struct via that environment callback with the data+size pointer and extension string. First load returned status 2 (rejection); adding `GET_GAME_INFO_EXT` support fixed it. Comments in `shim.cpp` flag the requirement for next-system bring-up.
- **Almost:** Audio + input. Beetle generates the samples (`g_audio_count` ticking up each `retro_run`) but `oa-audio` is still a stub — they're not piped to cpal yet. `oa-input` reports zeros so the player can't actually play.
- **Next:** (a) Wire cpal in `oa-audio` and pump the core's audio samples each frame so we get sound. (b) Wire real keyboard input from Tauri into `oa-input` mapped to the PCE button layout. (c) CI matrix.

---

## 2026-05-15 — Audio working (still same day)

- **Shipped:** Real `oa-audio::AudioSink` — cpal default output stream, SPSC ring buffer via `ringbuf` 0.4 between emu thread and audio callback, sample-format conversion (i16/f32/u16) inside the callback. First pass had the right wiring but wrong rate: Windows opened the device at 48 kHz while we push 44.1 kHz, producing ~9% pitch shift + underrun gaps. Added a stateful linear-interpolation resampler on the producer side that carries the last source frame across calls (click-free at batch boundaries). Bonk now sounds correct ("much better" — user-verified). `oa-shell` calls `sink.push(core.drain_audio())` after each frame; stats show 0 ring-buffer drops over multi-minute runs.
- **Almost:** Real input. The button bit-remap from our PCE layout to libretro's joypad numbering is wired through `oa_pce_set_input`; the only missing piece is reading actual keyboard/gamepad events. `oa-input` still returns zeros.
- **Next:** Tauri keyboard events → `oa-input::InputPoller` → PCE buttons. Then add gamepad via `gilrs`. Then CI matrix.

---

## 2026-05-15 — Keyboard input working: Bonk is playable 🎮 (still same day)

- **Shipped:** Real `oa-input::InputPoller` — cross-platform global keyboard polling via `device_query` 4.x. Generic `KeyboardMapping` table (32 bit slots × 5 ports, `Option<Keycode>` each) keeps `oa-input` system-agnostic — the shell binds keys to bit positions using the `oa_pce::buttons::*` constants. Default PCE map: arrow keys = d-pad, Z = I, X = II, Enter = RUN, RShift = SELECT. `Bonk's Adventure runs end-to-end with pixels, audio at correct pitch, AND playable keyboard input` — Phase 1 acceptance gate fully met.
- **Two real integration bugs caught while debugging** (both important to record):
  - **`tauri::Window::is_focused()` returns false for native (no-WebView) windows even when they have user focus.** Our focus gate prevented any input from reaching the core. For now `set_enabled(true)` unconditionally; proper focus tracking needs Tauri-event routing in Phase 2.
  - **`retro_set_controller_port_device` MUST be called AFTER `retro_load_game`, not before.** Beetle's `MDFNI_LoadGame` re-initializes the core and resets `pce_fast/input.c`'s `data_ptr[]` array, disconnecting any pre-load wiring. With pre-load wiring, the input pipeline reaches `g_input_bits[]` and `cb_input_state` returns the right values, but Mednafen never sees the data because its `INPUT_Frame` reads through `data_ptr[]` which points nowhere. Took 4 iterations of diagnostic logging in `cb_input_state` to localise.
- **Almost:** Gamepad. `gilrs` is in the dep graph but not wired. CI matrix is still empty.
- **Next:** Optional gamepad polling via `gilrs` (already in dep graph), then CI matrix (Windows / macOS / Linux × `cargo test` + `cargo tauri build`). That's the last open Phase 1 item; after CI green, Phase 2 (Solid UI shell + library + per-system theming) opens.

---

## End-of-session retrospective (2026-05-15)

**One session, one day, greenfield to a playable PCE emulator.** Built:

| Layer | Status | Notes |
|---|---|---|
| Workspace | ✅ | 10 crates + binary; clean `cargo build` (78s cold) |
| `oa-core` trait | ✅ | Designed for N systems; `SystemId` non-exhaustive |
| `oa-pce-sys` + `oa-pce` | ✅ | 93 C/C++ objects, 13 MB native libs; shim.cpp ~290 LOC |
| `oa-render` | ✅ | wgpu, RGBA8 texture cache, fullscreen-triangle WGSL, FIFO vsync |
| `oa-audio` | ✅ | cpal default output, SPSC ringbuf, linear-interp resampler |
| `oa-input` | ✅ keyboard | `device_query` polling, generic mapping table |
| `oa-shell` | ✅ | Two-window flow (library WebView + native game window), emu+render thread, audio + input wired |
| Phase 0 spikes | ✅✅✅ | All three passed |
| Phase 1 gate | 🟨 | Pixels + audio + keyboard live; gamepad + CI remaining for clean closure |

**Real lessons captured in auto-memory:**

- `reference_libretro_controller_after_load_game` — `retro_set_controller_port_device` must run AFTER `retro_load_game`. Applies to every libretro-style core we wrap.
- `feedback_tauri_no_webview_is_focused_unreliable` — Tauri Window's `is_focused()` returns false on no-WebView windows. Affects every future window-event wiring.
- `feedback_multi_core_architecture_ready` — every workspace crate except the PCE-specific pair is core-agnostic. Adding a new system follows an 8-step recipe, not a refactor.
- `project_current_state` updated end-of-session.

**Build artefact sizes (debug profile):**

- `oa-pce-sys` native libs: 9.6 MB (C++) + 3.4 MB (C) = ~13 MB
- `target/debug/oa-shell.exe`: not yet measured for release-profile but expect 30-50 MB debug, 15-25 MB release-stripped

**Stack quirks worth remembering:**

- wgpu 23 still uses `ImageCopyTexture` / `ImageDataLayout`; renamed to `TexelCopy*` in 24+.
- Tauri 2's `WindowBuilder` (no-WebView) is behind the `"unstable"` feature flag.
- `RawWindowHandle` is `!Send`; extract inside the spawned thread, never across.
- Beetle PCE Fast's `retro_load_game` needs `RETRO_ENVIRONMENT_GET_GAME_INFO_EXT` support when the frontend doesn't pass a file path — `info.data` alone isn't enough.
- Mednafen-derived headers need `INLINE=__inline` + `MEDNAFEN_VERSION_NUMERIC=931` + `STDC_HEADERS` + `_LOW_ACCURACY_` + the other ~15 shim defines catalogued in `crates/oa-pce-sys/build.rs`.

**Scope clarification (end of session):** the documented 10-system lineup in `docs/VISION.md` is the first wave, not the project ceiling. User clarified: "I want to do a lot more cores than what the documents show. I want to be able to run almost all of them plus new ones we work on." Captured in memory `project_expanded_scope_all_systems.md`; VISION + ROADMAP updated to reflect the broader ambition. The multi-core architecture wired in day one (`feedback_multi_core_architecture_ready.md`) means scope expansion is cheap per-system, so this is a tonal shift rather than a re-plan.

**Phase 2 backlog additions (end of session):** window modes (windowed / windowed-fullscreen / exclusive-fullscreen / monitor selection) and video scaling modes (pixel-perfect / aspect-correct fit / stretched / original 1:1 / explicit integer multiples). Recorded in `docs/ROADMAP.md` Phase 2; per-system aspect quirks + per-game scaling override parked in `docs/PARKING_LOT.md`.
