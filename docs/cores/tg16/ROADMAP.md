# tg16 Roadmap

Per-core phase tracking for TurboGrafx-16 / PC Engine. Mirrors the project-wide phases in `docs/ROADMAP.md` but only the tg16 slice.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 1 — HuCard runs end-to-end (completed 2026-05-16)

- ✅ Vendor Beetle PCE Fast into `crates/oa-pce-sys/vendor/`.
- ✅ `build.rs` compiles the full Mednafen PCE Fast tree (93 objects) + libretro-common + libchdr/zlib/zstd/lzma/tremor.
- ✅ `shim.cpp` exposes the `oa_pce_*` surface; pixel-format conversion for RGB565 / 0RGB1555 / XRGB8888 → RGBA8.
- ✅ `oa-pce::PceCore` impls `oa_core::Core`. Drop runs `oa_pce_free`. PCE-bit → libretro-joypad-bit remap with unit test.
- ✅ HuCard ROM loads via `OA_ROM` env var (uses `RETRO_ENVIRONMENT_GET_GAME_INFO_EXT` because the frontend doesn't pass a file path).
- ✅ Pixels at 60 fps in the game window (Bonk's Adventure measured 60.05 fps).
- ✅ Audio: 44.1 kHz core output → cpal at host rate via linear-interp resampler, no drops.
- ✅ Input — keyboard: `device_query` global polling, default map arrows + Z/X + Enter/RShift.
- ✅ Input — gamepad: `gilrs` polling, connection-order port binding, default map dpad + east=I/south=II/start=RUN/select=SELECT.
- ✅ CI matrix (Windows / macOS / Linux × `cargo build` + `cargo test`). Non-Windows runners use `--exclude oa-shell` to skip Tauri's gtk/webkit deps; emulation crates get full cross-platform coverage. macOS required one local patch to vendored zlib 1.2.11 — see `crates/oa-pce-sys/vendor/PATCHES/0001-zutil-skip-fdopen-NULL-on-modern-macOS.patch` and tg16 `DECISIONS.md`.

**Acceptance gate:** Bonk's Adventure (HuCard) at 60 fps with audio + controller. **Met** — keyboard + gamepad both wired, CI green on all three OSes.

---

## ⬜ Phase 1.5 — Hardening (post-Phase-1, pre-Phase-2)

Small follow-ups that don't gate Phase 2 but should land before Phase 5 (PCE-CD) opens.

- ⬜ Save state — wire `retro_serialize` / `retro_unserialize` through the shim, replace the `CoreError::Internal` stubs in `PceCore::save_state` / `load_state`.
- ⬜ Multitap exercise — pull controllers 2-5 into a test ROM that reads them, verify the per-port bit pipeline.
- ✅ Focus-gated input (shipped earlier than logged; closed 2026-05-18). `WindowEvent::Focused` events drive an `Arc<AtomicBool>` that the emu thread reads each frame to flip `InputPoller::set_enabled`. Picks the right window in each shell mode via `focus_target_label(mode)` (label "game" in two-window, "main" in single-window). Initial value `true` so first-launch input works before any focus event fires.
- ⬜ Pixel aspect ratio — PCE has non-square pixels (varies by horizontal mode). Wire per-system aspect ratio through `oa-render` viewport math so "Aspect-correct fit" mode in Phase 2 gets it right.

---

## ⬜ Phase 2 contributions

The Phase 2 work that lives in this core (everything else is in shared crates):

- ⬜ TG-16 theme (Solid + Tailwind) — system page, library tile styling, default shader preset slot.
- ⬜ PCE button glyphs (I / II / RUN / SELECT) for the input-mapping UI.
- ⬜ Per-system aspect-ratio entry in the system registry (whatever shape that takes when it's designed).

---

## ✅ Phase 5 — PCE-CD bringup (closed 2026-05-18)

**Resolved.** Beetle PCE Fast (`mednafen_pce_fast_libretro.dll`) handles
both HuCard AND CD — same .dll, no full-Mednafen vendor needed. The 2026-05-15
question "separate core or this one?" was made obsolete by the 2026-05-16
libretro pivot anyway: both options are now "drop a .dll." Operator validated
Castlevania: Rondo of Blood (CHD) end-to-end 2026-05-18.

The frontend split — PC Engine CD-ROM² gets its own SystemId (`pce-cd`),
its own sidebar entry, its own theme, its own per-system settings — lives
in `docs/cores/pce-cd/`. TG-16 keeps `.pce` HuCards; everything CD-shaped
lives under pce-cd now.

See `docs/cores/pce-cd/ROADMAP.md` for the post-split Phase 5.5 hardening
list (save-state mid-disc, multi-disc .m3u, `oa-cdrom` build-out if real
gaps surface).

---

## ⬜ Beyond

- ⬜ Per-game shader presets (Phase 3 — TG-16 default preset + per-game overrides for specific titles).
- ⬜ Rewind / TAS support (Phase 4 — requires save-state from Phase 1.5).
- ⬜ KNOWN_GAME_BUGS triage pass once we have a larger test ROM sample.
