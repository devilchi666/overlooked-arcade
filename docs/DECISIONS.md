# Decisions Log

Append-only. Newest at the bottom. Every entry: **what** we decided, **when**, **why**, and **what we considered and rejected**.

When a future session questions a decision, this is the source of truth for the original reasoning. If we reverse a decision, write a new entry that supersedes the old one — never edit history.

This file is **project-wide**. Per-core decisions live in `docs/cores/<core>/DECISIONS.md`.

---

## 2026-05-15 — Engine stack: Rust + Tauri 2 + wgpu + forked C cores

**Decision:** Build the new emulator frontend on Rust + Tauri 2 + wgpu (WGSL), with forked C cores wrapped via FFI.

**Why:** Three forces drove this:
1. **Forked-core philosophy.** We want to own and modify the emulation code (rewind, memory inspector, TAS, deterministic state). That argues for the C cores already battle-tested in Mednafen/Beetle/MAME, not C# rewrites.
2. **Premium UI ceiling.** Heroic Games Launcher visual tier requires HTML/CSS/JS speed of iteration with native binary distribution. Tauri delivers that.
3. **Multi-API rendering from one shader pipeline.** wgpu + WGSL translates to DX12/Vulkan/Metal/GL/WebGPU. No per-platform shader rewrites.

**Considered and rejected:**
- **Unity (the prior project at G:\Emultor).** 3-4 months in. Strong on UI ergonomics but bad fit for forked-C-core integration; URP shader pipeline doesn't compete with WGSL on multi-backend coverage; mono-repo ICore-in-C# meant we wrote emulation by hand rather than leveraging Mednafen.
- **Native C++/SDL2.** Too much non-emulator engineering for a solo dev.
- **Pure Rust emulators from scratch (rust-tinyboy etc).** Reinventing what Mednafen already nails.
- **Forking RetroArch.** Inherit the UX we're trying to escape.

---

## 2026-05-15 — UI framework: Solid + TypeScript + Tailwind

**Decision:** Solid (not Svelte, React, or egui) for the Tauri WebView frontend, with TypeScript + Tailwind + Vite.

**Why:** Smallest runtime among the modern reactive frameworks (~7KB), no virtual DOM, fine-grained reactivity. Per-system theming relies on CSS cascade and animated library grids — Solid renders these without VDOM diff overhead. TypeScript-first; closer ergonomically to React for any future contributors. Vite for instant HMR.

**Considered and rejected:**
- **Svelte 5.** Comparable size, more compile-time magic, separate `.svelte` file format. Solid won on simplicity.
- **React.** Bigger bundle, VDOM overhead matters for animated library grids.
- **egui / iced (native Rust UI).** Cannot hit Heroic's visual ceiling at solo-dev velocity; HTML/CSS is the only stack where Tailwind + designers' instincts work directly.
- **Frame streaming into a `<canvas>` via IPC.** Rejected as a rendering strategy; IPC at 60 fps wastes bandwidth and adds 1-2 frames latency.

---

## 2026-05-15 — License: GPLv2 binary-wide

**Decision:** Ship the entire binary under GPLv2.

**Why:** Beetle PCE Fast (and any Mednafen-derived core) is GPLv2. Statically linking it propagates GPLv2 to the whole binary. The project is non-commercial — a gift to the retro community — so GPLv2 aligns naturally with intent. Repo public from Day 1; About screen links to source; this satisfies distribution requirements with zero ongoing burden.

**Considered and rejected:**
- **Process-isolate cores via IPC to keep the shell proprietary.** Adds latency, ~2-5 MB binary overhead, significant engineering. Only worth it for a closed-shell commercial play — explicitly not our path.
- **Use only MAME-derived (BSD-3-like) cores.** Avoids GPL propagation but MAME's PCE driver is weaker than Mednafen's. Compatibility cost not worth the licensing flexibility we don't need.

---

## 2026-05-15 — Forked-core management: in-tree vendored copy + patch series

**Decision:** Each forked core lives in `crates/oa-<sys>-sys/vendor/` as in-tree source. Local modifications captured as numbered `.patch` files under `vendor/PATCHES/`. `vendor/ORIGIN.md` records upstream URL + commit SHA + date vendored.

**Why:** We modify cores extensively (state hooks for rewind, memory peek for inspector, libretro-glue stripping). Git submodules fight against in-tree edits. Patch-series-on-pristine-upstream slows iteration and breaks debugger step-through. In-tree edit is the fastest path; `scripts/vendor-update.ps1` re-derives the patch series by diffing against fresh upstream when we need to re-vendor.

**Considered and rejected:**
- **Git submodules.** Mechanical friction against the way we work; we modify, we don't pull.
- **Pristine upstream + patches applied at build time.** Build complexity, broken debugger, slower iteration.

---

## 2026-05-15 — Tauri + wgpu integration: two-window for Phase 1, single-window spike for Phase 2

**Decision:** Phase 1 ships with two windows — a Tauri WebviewWindow holding the Solid library UI, and a separate Tauri Window (no WebView attached) with a wgpu surface for the game. Phase 2 spikes a single-window mode where the WebView is transparent and the wgpu surface draws beneath; if it composites cleanly on Windows/macOS, we ship it as the primary mode and keep two-window as a settings toggle (multi-monitor users prefer it anyway).

**Why:** Two-window is the lowest-risk path to "first PCE ROM running." Zero compositing risk, clean lifecycle, wgpu surface tied to a dedicated window. Single-window UX is nicer but transparent WebView2 on Windows is the historical pain point — we don't want to gate Phase 1 on solving compositing artifacts.

**Considered and rejected:**
- **Single-window only.** Phase 1 risk too high.
- **`<canvas>` frame streaming via IPC.** Rejected as a rendering strategy — see DECISIONS entry on UI framework.

---

## 2026-05-15 — No per-core ARCHITECTURE.md

**Decision:** Per-core docs (`docs/cores/<sys>/`) will contain README, ROADMAP, SESSION_LOG, KNOWN_GAME_BUGS, and DECISIONS — but **not** an ARCHITECTURE.md shadowing chip behavior. The vendored C source's own comments and upstream documentation are the chip-level reference.

**Why:** We are wrapping forked cores, not writing emulation from scratch. Chip-level docs were load-bearing for the previous Unity project (which implemented HuC6280, VDC, PSG, etc. by hand in C#). With forked cores, restating chip behavior in our docs invites us to second-guess working upstream code. Worse, our restatement will drift from the C source as we patch it. Keep the truth in one place: upstream.

**Considered and rejected:**
- **Port the 42-entry hardware-reference memory and per-core ARCHITECTURE.md from `G:\Emultor`.** Dead weight given the forked-core approach; project owner pushed back explicitly during planning ("we dont need any of the architechual files or information. we are going to use cores and not any of our old code").

---

## 2026-05-15 — Spike 1 outcome: Tauri 2 + wgpu two-window works on Windows

**Decision:** Two-window architecture (library WebviewWindow + game Window with wgpu surface) is validated for Phase 1. Proceed with it as planned.

**What was tested** (scratch code at `scripts/spikes/01-tauri-wgpu-twowindow/`):
- Tauri 2.11.1 + wgpu 23.0.1 + raw-window-handle 0.6, MSVC toolchain, Windows 11.
- One Tauri process opens a `WebviewWindowBuilder` (HTML/CSS test page via WebView2) AND a `WindowBuilder` (native, no webview).
- `wgpu::Instance::create_surface_unsafe` against the native window's raw-window-handle returned a valid Surface.
- Render thread on `std::thread` (not Tauri's async runtime), surface configured `Bgra8UnormSrgb` + `PresentMode::Fifo`, drew animated HSV clear-color gradient.
- Ran for 50 seconds, 2880+ frames rendered at steady ~60.1-60.2 fps, no event-loop contention with WebView2.

**Why this matters:** Phase 1's plan-of-record was two-window for risk reduction. Spike confirms the approach is straightforward — no compositing hacks, no DPI gymnastics, no special features needed beyond Tauri's `"unstable"`. Single-window mode (Phase 2 spike) remains optional polish, not a blocker.

**One gotcha worth recording:** `tauri::WindowBuilder` (the no-WebView variant) is gated behind Tauri's `"unstable"` feature flag in 2.11.x. Our two-window architecture commits us to that flag until Tauri stabilizes the API. Acceptable; the alternative (running wgpu inside the WebviewWindow's HWND) is a much messier compositing fight.

**Considered and rejected during the spike:**
- **Frame-streaming into a `<canvas>`.** Not even attempted — already rejected in the UI-framework decision. Spike confirmed why: 60 fps × 640×480 RGBA bytes over Tauri IPC would burn ~36 MB/s for what wgpu does for free.
- **`wgpu::Surface<'window>` lifetimed against `&Window`.** Easier ownership story is `Arc<Window>` + `create_surface_unsafe` with explicit raw handles; surface and window co-live in the render thread.

---

## 2026-05-15 — Spike 2 outcome: Beetle PCE Fast (Mednafen C) builds via cc-rs on MSVC

**Decision:** `cc-rs` + vendored Beetle PCE Fast / Mednafen source is the production build approach for `oa-pce-sys`. No CMake, no separate build system — pure Rust-driven build.

**What was tested** (scratch code at `scripts/spikes/02-beetle-pce-build/`):
- Shallow-cloned `libretro/beetle-pce-fast-libretro` into `vendor/`.
- Compiled `vendor/mednafen/mednafen-endian.c` (smallest standalone .c in the core tree) via `cc::Build` against MSVC 14.44.
- Linked the resulting static archive into a tiny Rust binary.
- Called three exported C functions across the FFI boundary (`FlipByteOrder`, `Endian_A16_Swap`, `Endian_A32_Swap`) — all returned correct results.
- Incremental rebuild: 0.76s. Cold build: ~8s.

**Integration cost we now know about:** Mednafen-derived headers expect `INLINE` to be pre-defined when included. In a full libretro build, the glue layer pulls in `retro_inline.h` before anything else; without the glue we have to inject `INLINE` ourselves via `cc::Build::define("INLINE", Some("__inline"))`. There will likely be a small handful of similar shim defines as we add more files to the build (`PSS_STYLE`, `SHARED_INTERNAL`, paths, etc.) — they're catalogued as we encounter them, not pre-anticipated.

**Bonus finding (re-evaluates Phase 5 plan):** Beetle PCE Fast's `vendor/mednafen/pce_fast/` contains `pcecd.cpp` + `pcecd_drive.cpp`. Beetle PCE Fast appears to ship CD support after all — Phase 5 may not need to swap in full Mednafen PCE. Defer the final call until Phase 5 spike actually wires up the CD path; for now `docs/ROADMAP.md` Phase 5 stays as planned.

**Considered and rejected:**
- **CMake-orchestrated build.** Beetle ships Makefiles, not CMakeLists. Adding a CMake layer is one more moving part for no benefit; cc-rs handles MSVC discovery (VCINSTALLDIR, paths) automatically.
- **Pre-built static libs.** Defeats the forked-core philosophy and requires we build outside cargo.
- **bindgen for the endian helpers.** Hand-written `extern "C"` is fine for a handful of functions. Bindgen-vs-handwritten as a project-wide policy is the subject of Spike 3.

---

## 2026-05-15 — Spike 3 outcome: hand-written FFI surface (not bindgen) for `oa-<sys>-sys` crates

**Decision:** Production `oa-<sys>-sys` crates declare FFI surfaces by hand (`extern "C"` blocks + `#[repr(C)]` structs). Do not add `bindgen` to the build pipeline.

**What was tested** (scratch code at `scripts/spikes/03-bindgen-vs-handwritten/`):
- Synthetic but representative shim header `vendor/pce_shim.h` — opaque core handle, enum, plain struct, ~9 functions covering the init/run/framebuffer/audio/input surface we'll write atop Beetle PCE Fast.
- Two sibling crates (`handwritten-variant`, `bindgen-variant`) compiling the same `pce_shim.c` and exercising the same call sequence from Rust.
- Cold-build timing and dependency-graph inspection.

**Measured:**

| Metric | Hand-written | Bindgen |
|---|---|---|
| Cold build (incl. cargo registry pull) | **1.52s** | **7.34s** (4.8× slower) |
| Transitive crates introduced | 0 | 27 (clang-sys, libloading, nom, regex, syn, prettyplease, etc.) |
| External tool needed | none | libclang.dll — Windows requires LLVM install (~150 MB) |
| Hand-maintained binding LOC | 33 lines (1 ffi.rs) | 0 + ~10 lines build.rs glue |
| Enum naming at call site | `OaPceStatus::Ok` | `OaPceStatus::OA_PCE_OK` |

**Why hand-written wins for us:**

1. **The FFI surface is ours.** Each `oa-<sys>-sys` crate owns a small C/C++ shim layer wrapping the forked core. ~10-20 functions per system. Adding a function costs one line in `ffi.rs`; auto-generation buys nothing.
2. **Build determinism.** No libclang dependency on developer machines or CI runners. Onboarding stays at "install Rust + Node + MSVC."
3. **5× faster cold builds matter** when iterating on the shim during early system bring-up.
4. **Idiomatic Rust enum names** at every call site, without `bindgen` post-processing hooks.
5. **Drift is OK because the shim is small.** When the shim changes, the Rust side gets a compile error pointing at the exact line — auto-binding-regen would change call sites silently.

**Bindgen stays an option for these specific future cases:**
- Wrapping a much larger upstream surface directly (e.g., if we ever skip writing our own shim for a system).
- Wrapping a vendor SDK we don't control and that ships hundreds of functions.

Neither applies today. Re-evaluate per-system if it does.
