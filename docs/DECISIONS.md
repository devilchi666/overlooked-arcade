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

## 2026-05-16 — CI scope: workspace minus `oa-shell` on non-Windows

**Decision:** GitHub Actions matrix runs `cargo build --workspace --locked` + `cargo test --workspace --locked` on Windows, and `cargo build/test --workspace --exclude oa-shell --locked` on macOS + Ubuntu. Linux runners install ALSA + udev + X11 dev headers (`libasound2-dev`, `libudev-dev`, `libx11-dev`, `libxi-dev`, `libxtst-dev`, `pkg-config`) for cpal/gilrs/device_query.

**Why:** Including the Tauri shell crate on non-Windows runners drags in webkit2gtk, glib, gio, librsvg, libxdo, libssl, ayatana-appindicator, etc. — 9+ apt packages on Linux, plus the macOS Tauri build path. The user can't validate or ship Tauri builds on macOS/Linux today (Windows is the primary platform until Phase 6+ distribution work), so the value of testing the shell binary on those runners is low and the maintenance cost of the system-dep list is real. The emulation crates (`oa-pce-sys`, `oa-render`, `oa-audio`, `oa-input`, `oa-core`, `oa-pce`, `oa-savestate`, `oa-cdrom`) DO get full cross-platform coverage — that's where the vendored-C-core breakage risk lives, and macOS's zlib 1.2.11 issue already proved the value of that coverage (first-pass CI caught it).

**Considered and rejected:**
- **Install the full Tauri Linux/macOS dep set.** Catches Tauri drift cross-platform, but the user can't run the result, so the value is mostly noise until Phase 6+.
- **Skip non-Windows CI entirely.** Loses cross-platform validation of the emulation crates — exactly where vendored-C-core breakage is most likely.
- **Maintain a separate workflow file for Tauri builds.** Premature; add when Phase 6 distribution work begins.

**When to revisit:** Phase 6+ when cross-platform distribution becomes a real goal. Drop the `--exclude` and add the full system dep matrix.

---

## 2026-05-16 — Default renderer scaling mode: aspect-correct fit

**Decision:** The renderer's default presentation is **aspect-correct fit** — the largest rectangle inside the surface that preserves the core's reported display aspect ratio, with the remaining surface area letterboxed/pillarboxed in the render-pass clear color (black). Cores report aspect via the new `oa_core::Framebuffer.display_aspect: f32` field; `0.0` falls back to `width:height`. Implementation is `wgpu::RenderPass::set_viewport()` — no shader uniform, no extra draw, no per-frame allocation.

**Why:** Phase 2 ROADMAP plans five scaling modes (pixel perfect / aspect-correct fit / stretched / 1:1 / explicit integer multiples) with per-game default. Before that UI exists, the renderer needs a sensible default for "what happens when the user resizes the window". Aspect-correct fit is what polished emulators do — it preserves the look the developer designed without exposing pixel-aspect quirks. The Phase 2 settings UI will add the toggles; this commit lays the foundation by plumbing display aspect end-to-end (shim → `OaPceFrame` → `Framebuffer` → `Renderer`).

**Considered and rejected:**
- **Stretched (pre-patch behavior).** Easy, looks awful on non-matching window aspects, hides bugs in core aspect reporting.
- **Pixel-perfect integer-scale only.** Best image quality but surprisingly small image at non-matching window sizes; bad first impression vs gentle letterboxing.
- **Defer aspect mode until Phase 2 UI exists.** Means shipping Phase 1 with the stretched look as the de facto default — bad screenshot material and a regression target for Phase 2.

---

## 2026-05-16 — Tailwind v4 (CSS-first via `@tailwindcss/vite`), no PostCSS

**Decision:** Use Tailwind CSS v4 (`tailwindcss` + `@tailwindcss/vite`, both ^4.1) for the `frontend/` shell. Theme tokens — TG-16 orange/cream, OA dark surface palette, font stack — live inline in `frontend/src/index.css` under the v4 `@theme` directive. No `tailwind.config.js`, no `postcss.config.js`.

**Why:** v4 is a ground-up rewrite that ships as a Vite plugin and reads theme tokens from CSS via `@theme { --color-… }`. For per-system theming (Phase 2's anchor feature), CSS-native tokens compose with our per-system palettes more naturally than a JS config object that the build pipeline needs to read and serialize. The plugin-only setup keeps the dep tree small (89 npm packages total, zero vulnerabilities), eliminates the PostCSS hop, and ships a 6.2 kB CSS bundle for the splash. Solid 1.9 + Vite 6 are both standard choices given the project-wide UI-framework decision; no surprises there worth their own entries.

**Considered and rejected:**
- **Tailwind v3 + `tailwind.config.ts`.** Familiar, but theme-in-JS means per-system palettes have to live in a JS object that round-trips through the build instead of being plain CSS tokens we can override per-system page via the cascade. v4 was designed for exactly this.
- **No Tailwind, hand-rolled CSS only.** Loses utility-class velocity for layout/spacing/typography — Heroic-tier polish takes too long without it.
- **Vanilla-extract / Panda / Stylex.** All capable, none widely-used in the Solid ecosystem; choosing a heavier tool here trades future-contributor familiarity for marginal type safety we don't need yet.

**One implementation note worth keeping (corrected 2026-05-16):** Tauri 2's `tauri-cli` runs `beforeDevCommand` / `beforeBuildCommand` from the **frontend project root** (the directory containing `package.json` referenced by the build), NOT from the directory containing `tauri.conf.json` as we originally assumed. So our commands are plain `npm run dev` / `npm run build` — already in the right cwd. Tauri's standalone `cargo build -p oa-shell` doesn't touch the frontend at all (no beforeBuildCommand step), so the discrepancy only surfaced when we actually invoked `cargo tauri dev` for the first time. The earlier `npm --prefix ../../frontend …` form was based on a wrong cwd assumption and was silently broken until cargo-tauri got installed.

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

---

## 2026-05-16 — Spike 4 outcome: single-window WebView2-over-wgpu — **PASS**

**Scratch:** `scripts/spikes/04-single-window/` — standalone Cargo project (own `[workspace]` table to escape the repo workspace), Tauri 2.11 + wgpu 23. Single `WebviewWindow` built with `.transparent(true)`; wgpu surface attached to its underlying HWND; clear-color HSV gradient on a dedicated render thread. `dist/index.html` sets `html, body { background: transparent; }` and frames the wgpu area with semi-transparent top + bottom chrome strips + a side callout (`rgba(8, 6, 18, 0.55)` with `backdrop-filter: blur(12px)`).

**Result:** Operator ran `cargo run` on 2026-05-16 — animated rainbow gradient is visible in the middle region of the WebviewWindow, between the top and bottom HTML chrome strips. WebView2's transparent CSS regions composite over the wgpu surface drawn into the parent HWND on Windows + DWM. The chrome strips and the callout overlay correctly on top of the gradient with their semi-transparent backdrops.

**What this confirms (non-obvious before the spike):** Tauri 2 with `.transparent(true)` on a `WebviewWindow` does *not* result in WebView2's child HWND painting opaque over wgpu pixels in the same parent HWND. The combination — `WebviewWindow::transparent(true)` plus `html, body { background: transparent; }` in the loaded document — lets DWM composite the parent HWND's client area (where wgpu draws) underneath the WebView2 child's transparent regions. wgpu doesn't fight WebView2 for the HWND.

**Decision:** Add a "single-window" mode toggle to `oa-shell`. **Two-window stays the default** — multi-monitor users prefer game on one display + library on another, and two-window is already validated for the full emulator path including audio, input, save-states, scaling modes. Single-window becomes an opt-in for users on a single screen or who want the more polished "game-with-overlay-UI" feel. Both modes share `oa-render` and the emu thread; the difference is in `oa-shell::setup` (one `WebviewWindow` with wgpu attached to its HWND vs separate `WebviewWindow` + `Window`).

**This supersedes** the "Phase 2 spike" line in the 2026-05-15 Tauri+wgpu integration decision: single-window mode is shippable, not just an experimental spike target.

**Implementation notes for the integration pass:**
- Mode chosen at app start (settings file read before Tauri Builder). Toggling at runtime is possible but would require recreating windows; treat it as a restart-required setting for the first cut.
- The library UI in single-window mode is the same Solid app, but a separate route / view-mode that lays out chrome (top bar, optional side panel) over the transparent center where wgpu draws. Reuse the existing components.
- `dist/index.html`'s body MUST stay `background: transparent` in single-window mode and CAN keep `background: var(--color-oa-bg-deep)` in two-window mode — the two-window library WebView is opaque on purpose. Probably cleanest as a body class set by the shell, or two separate index entry points.
- Spike binary stays around as the minimal reference for the compositing approach. Don't delete `scripts/spikes/04-single-window/`.

---

## 2026-05-16 — Per-system theming: cascade-override with neutral-OA default

**Decision:** The Solid frontend declares system-aware design tokens (`--color-system-accent`, `--color-system-accent-soft`, `--color-system-glow`) once in `index.css`'s `@theme` block, defaulted to a neutral OA palette (warm desaturated accent, OA cream, faint glow). Per-system palettes live in `frontend/src/themes/systems.css` as `[data-system="<id>"]` blocks that override the tokens via CSS cascade when `document.documentElement.dataset.system` is set. The active system is flipped from TS via `applySystemTheme(id)` in `frontend/src/themes/registry.ts`, which also exports the typed `SystemId` union + `systemThemes` registry. App code references only the system tokens (`text-(--color-system-accent)`, etc.), never a system-specific token like `--color-tg16-orange`.

**Why:**
1. **System-agnostic pages need their own brand identity.** The all-systems library grid, settings panel, and About screen don't belong to any one system — defaulting them to TG-16's orange would visually claim TG-16 is "the OA palette," which is wrong and would force a refactor the moment Lynx ships. A neutral OA default keeps system identity scoped to the system page.
2. **Tailwind v4-native.** `@theme` declares the token + generates the utility class (`text-system-accent`, `bg-system-accent`, etc.) in one place. `[data-system="..."]` overrides are plain CSS — Tailwind doesn't need to know about per-system palettes, so we stay clear of v4's regenerate-on-config-change semantics.
3. **Adding a system is a 3-line recipe.** Extend `SystemId`, add a `systemThemes` entry, add a `[data-system="<id>"]` block to `systems.css`. The recipe is the same shape as the 8-step per-core recipe in `feedback_multi_core_architecture_ready` — the theming layer is core-agnostic by design.
4. **No JS-driven CSS-var writes per frame.** Active system changes hit the cascade once via the `dataset.system` attribute write; no Solid effect rewriting CSS variables on the root element, no inline `style=""` props.

**Considered and rejected:**
- **TG-16 as the unbranded default.** Tempting because TG-16 is the only core today, but bakes a system assumption into the OA brand and forces a cleanup pass when the second system ships.
- **JS-driven CSS-var injection** (Solid effect writes `--color-system-accent` etc. onto `document.documentElement.style` when the active system changes). More flexible (dynamic palette tweaking, palette previews), but heavier and pulls the source-of-truth out of CSS — diagnosing "why is the accent wrong?" becomes a JS+CSS trace instead of just `data-system` + DevTools inspect.
- **One CSS file per system, dynamically imported.** Defers theme code-splitting but adds a network/IO step on system switch (jarring), and palettes are small (~120 bytes per system gzipped) — not worth the complexity.

**When to revisit:** If theming grows beyond palette (per-system fonts, custom decorative SVGs, animated backgrounds), revisit whether the registry should carry asset references too. Today: tokens only.

---

## 2026-05-16 — Architecture pivot: libretro frontend (dynamic .dll loading)

**Decision:** Overlooked Arcade becomes a libretro frontend. Cores are loaded at runtime from `.dll`/`.so`/`.dylib` files in `appDataDir/cores/` via `libloading`, wrapped as `oa_core::Core` impls by the new `crates/oa-libretro` crate. Users can use community-built libretro core nightlies (https://buildbot.libretro.com/nightly/) or .dlls we build ourselves from forked source. Per-game and per-system core selection (UI shipping in the next sessions).

**This supersedes** parts of the 2026-05-15 "Engine stack" decision — specifically, the "forked C cores wrapped via FFI" aspect. Cores are no longer statically linked into our binary; they live as separate files. The rest of that decision (Rust + Tauri 2 + wgpu + WGSL) stands.

**Why:**
1. **Multi-core per system**, which the user explicitly requested. The PC Engine alone has three credible cores (Beetle PCE Fast, Beetle PCE full Mednafen, Beetle SuperGrafx) covering different game subsets (HuCard, CD, SGX). Statically linking all three balloons the binary and means re-vendoring every upstream change. Dynamic loading lets users pick the right core per game with zero binary cost.
2. **Catalog instantly expands** to every libretro core (~150 cores across every retro platform). Users wanting Genesis, SNES, PSX, N64, etc. drop the .dll in cores/. Aligns with the original "premium emulator frontend" vision but at much larger scope than the original 10-system lineup.
3. **Community-built nightlies** mean updates are dragging in a new .dll, not re-vendoring source. The libretro project ships daily builds tested across platforms.
4. **License flexibility.** Dynamic loading severs GPLv2 propagation. The shell can ship under whatever license; GPL cores stay GPL in their .dll. Our forked .dll builds remain GPL (their source is GPL). This unblocks future considerations we don't have today but might want.
5. **No throw-away.** Our existing `shim.cpp` logic ported cleanly to Rust under `oa-libretro/src/state.rs` and `pixel.rs`. The same callback model, environment dispatcher, and pixel conversion that worked statically work dynamically; only the linkage method changed.

**Trade-offs accepted:**
- **Day-one install requires a cores/ folder.** We can't ship "double-click, play Bonk" without bundling at least one .dll in the installer. Acceptable: we'll ship a curated set of forked cores (built by us from modified source) in the installer's cores/ folder. Power users can add more.
- **Editing core source = building a .dll**, not editing in-tree files. This is a real workflow change — adding a new build pipeline (or using libretro's own Makefile/CMake) — but each core is independent and stays close to upstream.
- **Singleton constraint enforced in Rust.** libretro cores keep C globals, so only one `LibretroCore` exists per process. The `Mutex<Option<State>>` in `oa-libretro/src/state.rs` enforces this. To swap cores at runtime (when launching a different game), we drop the active core (which uninstalls the singleton) and load the next .dll.
- **Variadic log callback omitted.** `extern "C" fn(level, fmt, ...)` requires unstable `c_variadic`. We `return false` for GET_LOG_INTERFACE; cores fall back to stderr. Revisit if a core hard-requires the log interface.

**Considered and rejected:**
- **Stay static, multi-crate** (vendor `oa-pce-sys`, `oa-sgx-sys`, `oa-pcecd-sys`, ...). Matches the original CLAUDE.md vision exactly but binary grows linearly with each system and we lose the libretro catalog. User explicitly chose dynamic over this option.
- **Hybrid (static built-in cores + dynamic .dll for everything else).** Best UX (day-one install works) but largest engineering scope — we'd maintain two parallel core-integration paths forever. User chose pure dynamic; we'll ship curated .dlls in the installer instead to get the same day-one experience.

**Migration plan (multi-session):**
1. ✅ This session — `oa-libretro` crate exists, integrates into `oa-shell` with auto-detection of `appDataDir/cores/mednafen_pce_fast_libretro.dll`, falls back to static `oa-pce` if missing.
2. Next — operator drops the .dll, validates Bonk plays via libretro. Per-system + per-game core picker UI. Cores-folder scanner UI in Settings.
3. After — retire `oa-pce-sys` and `oa-pce` crates. Vendor source moved to a separate build pipeline that produces our forked Beetle PCE Fast .dll for distribution.
4. CD + SGX work by dropping the respective libretro .dlls in cores/.

**CLAUDE.md update pending.** The "Locked design pillars" section's "forked C cores via FFI" line needs to change to "libretro cores loaded dynamically via libloading; we ship our own .dll builds for cores we fork heavily." Not done in this session — left for user to confirm before editing the source-of-truth file.

---

## 2026-05-17 — Asset delivery: Tauri's built-in asset protocol, not custom URI schemes

**Decision:** All on-disk media served to the WebView (cover art, save-state thumbnails, eventual snapshots / title screens / video) goes through Tauri's **built-in asset protocol** via `convertFileSrc(absolutePath)`, not a custom-registered URI scheme. `app.security.assetProtocol.enable = true` with `scope.allow = ["$APPDATA/**"]` in `tauri.conf.json`; the `tauri` crate gets the `protocol-asset` feature.

**Why:** Custom URI schemes registered via `register_asynchronous_uri_scheme_protocol("foo", ...)` work in production (WebView loads from `tauri://localhost/`, same-origin with custom schemes) but **error with `net::ERR_UNKNOWN_URL_SCHEME` in `cargo tauri dev`** (WebView loads from Vite at `http://localhost:5173/`, which is cross-protocol to the custom scheme; Chromium blocks the fetch before the Rust handler even runs). The custom-scheme path was initially shipped in the cover-art slice (`oa-media://`); validation by the operator surfaced the dev-mode breakage. The asset protocol is the canonical Tauri 2 path for serving local files and is specifically configured cross-origin-friendly. See [[reference_tauri_custom_uri_schemes_blocked_in_dev]] for the full debugging story.

**Trade-offs accepted:**
- **Scope must be declared up-front in `tauri.conf.json`.** Adding a new media root requires a config change + app restart. We use the broad `$APPDATA/**` scope for v1 because all media lives under appData; tighter scopes (`$APPDATA/media/**`) are an option later if we want to lock down.
- **`convertFileSrc` returns the host-formatted URL** (`https://asset.localhost/...` on Windows; `asset://localhost/...` elsewhere) — so the frontend has to call it; we can't construct URLs by string-formatting on the Rust side. Acceptable: one helper call per cover.
- **Path resolution moves to the frontend.** The Rust MediaDb stores paths as `appData`-relative; the frontend resolves `appDataDir()` once at mount and joins them. Trade-off: frontend now has to know about appData layout. Mitigated by isolating in `MediaProvider`'s `joinAppData()` helper.

**Considered and rejected:**
- **Custom URI scheme (`oa-media://...`)** — what we shipped first. Cleanly scoped to media routing, includes our own region-priority resolution server-side, but fails in dev mode. Production-only would force every iteration to be `cargo tauri build`.
- **`windows[].useHttpsScheme: true`** to load dev HTML from `https://tauri.localhost/` instead of HTTP. Sidesteps the cross-protocol issue but breaks Vite HMR and adds cert friction. Asset protocol is cleaner with no dev-iteration impact.
- **Base64-embed in IPC payloads.** Works cross-origin trivially (data: URLs are always allowed) but explodes IPC size — 1000 covers × 80 KB = 80 MB JSON per refresh — and defeats the WebView's HTTP cache. Reserve `data:` URLs for tiny transient previews (save-state thumbnails, where we already use them because they're ≤32 KB and the modal closes after viewing).
- **Sidecar HTTP server** (run a `localhost:NNNN` static file server inside the Tauri process). Architecturally heavier and re-implements what the asset protocol gives us for free.

**Where this applies going forward:**
- Cover art (this session). ✅
- Save-state thumbnails currently use base64 data URLs (`list_save_slots` command). For 10 slots per game this is fine, but if we ever surface a "library of all save states" view it'd want to migrate to the asset protocol too.
- Future per-game metadata media (screenshots, fanart, video clips) goes through the asset protocol by default.
- Per-system theming assets that ship in the install bundle would use Tauri's `$RESOURCE` scope (a different prefix); same convention.

---

## 2026-05-18 — Rewind: byte-bounded ring of opaque save-state blobs, off by default

**Decision:** Phase 4 slice A implements rewind as a `RewindRing` in `oa-savestate` that holds uncompressed `Core::save_state` blobs in memory, capped by total bytes (default 64 MiB), captured every N forward frames (default 6 = ~100 ms at 60 fps). Holding Backspace pops the newest snapshot, `load_state`s it, then runs exactly one forward frame to repaint the framebuffer. Rewind is off by default at every tier; users opt in via OA Settings → Gameplay → Rewind, with per-system + per-game overrides through the standard inheritance chain.

**Why:**
1. **Byte-bounded, not snapshot-count-bounded.** Per-snapshot size varies wildly across cores (PCE Fast ~50 KB, SNES9x ~300 KB, Mednafen Saturn ~3 MB if/when it lands). A count-based cap means "10 s of history on PCE, 1.7 s on Saturn" with no warning to the user. A byte cap lets the seconds-held display surface system-specific reality.
2. **Always retain ≥1 snapshot during eviction.** If a single snapshot busts the cap (extreme: 100 MB Saturn save vs 64 MB cap), we keep it rather than emptying the ring. Losing all history to a momentary cap squeeze is worse than briefly exceeding the cap; the next push restores equilibrium.
3. **Capture interval lives in frames, not milliseconds.** Different cores run at different rates (PCE 59.83 Hz, SNES 60.10 Hz, Lynx 75 Hz, future Saturn 50/60 Hz). A frame-based interval is deterministic across cores; the UI converts to ms for display only.
4. **No compression on the capture path.** Compression would buy ~5× density but adds variable CPU cost on the emu thread, which has to fit inside the frame budget. We have plenty of RAM headroom; choose predictability over density. If memory pressure becomes real (Saturn-class state sizes), revisit with `zstd` level 1 (already a workspace dep, currently unused in `oa-savestate`).
5. **Rewind suppresses input + advances one frame after load_state.** Libretro's `cb_video_refresh` only fires from `retro_run`, so the framebuffer needs at least one `run_frame` after `retro_unserialize` to repaint. We dispatch ZERO input to that run — the user's holding Backspace, not steering. Net motion per render frame at default settings: 5 game frames backwards (one `pop_back` undoes ~6 frames; one `run_frame` advances 1). Acceptable RetroArch-equivalent UX.
6. **Off by default at every tier.** Rewind has non-zero RAM + CPU cost per frame. Many sessions don't want it, and quietly burning ~30 MB of RAM + a `save_state` every 100 ms surprises users who'd never explicitly enabled it. Three-tier inheritance + off default at the root means the cost is opt-in everywhere.

**Considered and rejected:**
- **Snapshot-count-bounded ring.** Simpler API but misleading across cores (see #1).
- **Compress snapshots with `zstd` level 1 on capture, decompress on rewind.** Quiet density win on the wire — `oa-savestate` already depends on `zstd` from Phase 1's stub. Rejected for slice A on predictability grounds (#4); revisit when state sizes outgrow the byte budget on a real core. Code path is small enough that adding it later is a half-day refactor, not a redesign.
- **Capture the framebuffer alongside the save state.** Lets us skip the extra `run_frame` after `load_state` and present the framebuffer that existed when the snapshot was taken. Cleaner semantically, but doubles the snapshot size (PCE: 50 KB save + 240 KB RGBA → 290 KB; 5.8× density loss). The framebuffer trailing by one frame during rewind is invisible at ~5× rewind speed.
- **One ring per `Core` instance (owned by the core itself).** Tempting from a "core owns its own state" perspective. Rejected because the cap policy is system-agnostic UX (operator sets 64 MB; that 64 MB belongs to whatever's loaded), and shell-side ownership lets the ring survive a brief core swap (slice B might want this). The trait stays minimal; the shell owns the ring.
- **Always-on rewind, controlled only by capture interval (0 = off).** Conflates two concerns (enable/disable + interval) into one knob. Worse: a fresh install without a real opt-in would burn RAM silently.

**Where this applies going forward:**
- Slice B (rewind scrubbing UI) consumes the same ring: it'll add a peek-by-index accessor for thumbnail strip rendering + a "set head to position N" that pops everything above and `load_state`s the target.
- Slice C (TAS recording + deterministic replay) shares the snapshot format — TAS recording dumps the ring's contents as initial state + per-frame input deltas; replay rewinds via the same mechanism.
- Per-game milestone tracking (slice F) can subscribe to memory regions per-snapshot; the ring becomes a value-history channel as well as a state-history channel.

---

## 2026-05-18 — Top-bar menu-bar replaces scattered settings entry points

**Decision:** Reorganize the shell's information architecture around a named menu bar at the top (Library · View · System ▾ · Game ▾ · Tools · Settings · Help), with each tier of the existing three-tier settings split (OA-wide / per-system / per-game) getting its own menu. Configuration surfaces become menu-launched modal dialogs; the two genuinely wide editors (library folders + media sync, and core install/uninstall) stay as full-page routes. The in-game `QuickSettings` overlay shrinks to verbs only (Resume / Save / Info / Exit) — its drill-in tools (Rewind / TAS / Video / Memory / Disc) move to the `Tools ▾` menu, which can deep-link the overlay to a specific panel.

**Why:**
1. **One door per room.** Pre-redesign the per-system settings were reachable five ways (sidebar bottom button, GridControls ⚙, SystemHeader's four quick-action chips, SystemContextMenu items, toolbar `⚙`), the OA-wide Settings page had seven tabs of mixed scope, and Cores lived in four places. Inconsistent doors made the IA feel improvised. The menu bar gives each tier exactly one canonical home.
2. **Tier shows in the menu name.** A user looking at `Game ▾ → Cheats…` knows the cheat lives at the per-game tier; `System ▾ → Shaders…` is per-system; `Settings → Shaders…` is OA-wide. Pre-redesign that distinction was implicit in which route the user took to get there.
3. **LaunchBox discipline, modern visual language.** Researched LaunchBox / BigBox / RetroArch Ozone / RetroArch XMB. LaunchBox's classic Windows menu bar (File / Edit / View / Tools / Help) is the most familiar IA for desktop emulator-frontend users — but its Win32 styling reads as legacy. Adopted the discipline (named menus dispatching to focused surfaces) and rendered it in our type system (stylized text, no pipes, accent-on-open) so it reads like Spotify's top nav, not Notepad's.
4. **Disabled rather than hidden for contextual menus.** `System ▾` and `Game ▾` stay visible at all times — dimmed when there's no active context — with tooltips explaining what to do. Hidden-when-empty hurts discoverability; the user shouldn't have to remember "the System menu only exists when I've clicked a system." This was explicit user feedback during the planning chat.
5. **`QuickSettings` should be a pause menu, not a power-user console.** Today's overlay tries to be both — 10 rows of mixed verbs + configuration drill-ins crammed into a ~600px-wide modal where memory inspector and TAS recorder fight for space. Splitting it: verbs stay on Esc (instant), tools open via menu-bar `Tools ▾` (one more click, but each drill-in gets the full overlay area).
6. **Three-tier inheritance preserved.** OA-wide → per-system → per-game chain is unchanged at the persistence layer; the redesign is purely UI. The "scaffold" per-system Display overrides that persist but don't yet take effect at runtime continue to persist (the override JSON is the same); the dialog's amber "Scaffold" banner tells users the runtime wiring is pending.

**Considered and rejected:**
- **RetroArch XMB-style horizontal tab row.** Familiar to RetroArch users, but its discrete top-level tabs (Main Menu / Settings / Favorites / History / playlists) don't map cleanly to our tier split; settings would all collapse into one Settings tab with 20+ sub-categories. Worse for "where am I" awareness.
- **RetroArch Ozone-style three-column sidebar.** Replaces our left sidebar entirely with a categorical column. Conflicts with the existing left sidebar being a *library* nav surface (per-system, per-playlist), which is the user's main navigation target — not the settings tree.
- **Single consolidated `Settings` page with all tabs (status quo, retired here).** Worked but couldn't surface tier scope in the UI; users had to learn that "Cores" in Settings means OA-wide while right-click a tile → Run with core means per-game. Both visible, no shared frame.
- **Icon-based menu bar (icon + label chips).** Faster to scan visually but our system accents already do the high-frequency theming work. Stylized text reads as more "premium" within our visual language (the Spotify / Heroic / Linear axis vs the Win32 / GIMP / desktop-utility axis).

**Where this applies going forward:**
- New systems plug into the existing menus automatically (the system loop in `LeftSidebar` and the per-system dialogs all dispatch by `SystemId`).
- New per-game tabs (when we wire genuine orphans like Cheat Search v2) become menu items + drawer tabs; menu items deep-link the drawer to the right tab via `initialTab`.
- New in-game tools (Performance HUD, Screenshot gallery) become `Tools ▾` items. If they're light, they live as inline `MenuItem`s with state toggles (Performance HUD = a `MenuCheckbox`); if they're heavy, they open dialogs.
- Step 10 polish (icon set replacement, type ramp tightening, accent usage in more places) is deferred to a separate pass with a live dev server in front of an actual human eye.

**Companion docs:** `docs/UI_AUDIT.md` is the surface-by-surface inventory of the pre-redesign UI; `docs/UI_MENU_BAR_PLAN.md` is the redesign proposal with the full field-to-menu mapping and visual treatment spec.

---

## 2026-05-18 — Three-output logger (stderr + file + ring) with frontend bridge

**Decision:** Replace bare `env_logger::init()` in `apps/oa-shell/src/main.rs` with a custom `MultiLogger` (`apps/oa-shell/src/logger.rs`) that fans every `log::*!` record to three sinks:

1. **stderr** — preserves the `cargo tauri dev` workflow (same log stream operators saw before).
2. **File** — two writers via a `TeeWriter`:
   - `appData/logs/oa-current.log` — truncated on every launch. **Stable path** so a debugger can always `Read` it without negotiating timestamps.
   - `appData/logs/oa-<YYYYMMDD-HHmmss>.log` — per-session archive. Last 5 retained; older pruned on startup.
3. **Ring** — bounded `VecDeque<LogEntry>` (capacity 2000). Tauri command `get_recent_logs(limit)` serves the live tail to the `Help → Debug log…` dialog at 1 Hz.

Frontend log bridge (`frontend/src/lib/logbridge.ts`, installed in `index.tsx` before SolidJS hydration) wraps `console.log/info/warn/error/debug` plus `window.onerror` and `unhandledrejection`. Bracket prefixes like `[oa-launch]` in existing `console.log` call sites are parsed and become the Rust `target` field — no source changes required.

**Why:**
1. **Stable on-disk path beats per-session paths for debugging.** A debugger using the `Read` tool needs a single path it can pull on demand. The timestamped archives are for sharing prior runs ("the bug repro from yesterday"); the stable `oa-current.log` is for "tell me what just happened." Both come for free via the `TeeWriter`.
2. **Two-phase init avoids losing early logs.** `app_data_dir` is not resolvable until Tauri `setup()` runs, but `main()` needs logging immediately (cli arg parse, shell-mode resolution). Solution: `logger::init_early()` at `main()` top installs stderr + ring; `logger::configure_file_output(&app_data_dir)` inside `setup()` enables file output once the path lands. The tiny window between those two points (about 5 log lines) hits stderr + ring only, not the file. Acceptable.
3. **Ring is bounded; file is unbounded.** A long session could fill the ring 100x over; oldest entries silently drop. The file captures everything for post-mortem. The dialog reads from the ring, not the file, so polling at 1 Hz with 2000 entries is bounded work.
4. **Unified Rust + frontend timeline.** Without the bridge, frontend logs live in WebView DevTools and Rust logs live in stderr/file — two consoles to correlate. With the bridge, every event goes through the same `log::Log` impl: same timestamps, same target conventions, same dialog. The original `console.*` is preserved so DevTools still works.
5. **Bracket-target parser is zero-cost ergonomics.** The codebase already had `console.log("[oa-launch] handleLaunch called", entry)` patterns everywhere. The bridge parses `[oa-launch]` as the Rust target without source changes; the convention is preserved as a first-class field.
6. **Crash-survival via level-driven flush.** `WARN` + `ERROR` records flush the `BufWriter` immediately so a panic mid-line still leaves a useful tail. `INFO` / `DEBUG` ride the buffer (flushed on drop or buffer fill). Best of both: cheap normal operation, durable error path.

**Considered and rejected:**
- **`env_logger` + dedicated file appender.** Two parallel loggers is doable but you lose the ring (which is the live-dialog substrate). Custom `Log` impl is about 150 lines and gives all three outputs in one place.
- **`fern` crate.** Standard choice for multi-output Rust logging; would have saved some code. Skipped because we would be importing a crate for a feature that is 150 lines hand-rolled, and the hand-rolled version lets us bake the bracket-target parser + ring snapshot semantics into the same impl.
- **Tauri events for every log record.** Tried mentally; rejected. Log volume can be per-frame in some paths (rewind capture, video frame submission); JSON-serializing each into an event payload adds up. Polling the ring at 1 Hz is cheaper and gives identical UX.
- **JSON Lines on-disk format.** Better for grep/jq but worse for human scanning. The current format (`ISO-8601 LEVEL [target] message`) is grep-friendly AND readable. If we ever need structured analysis, we still have the in-memory `LogEntry` shape.
- **Drop the timestamped archives.** Tried, kept. The "stable current path" alone breaks down when you want to compare two sessions ("yesterday's run had X but today's does not"). Retaining the last 5 archives is about 5 MB worst case.

**Where this applies going forward:**
- New Tauri commands needing a "current" log surface can add to the ring via `log::info!`; the dialog and file both pick them up.
- Any frontend module using `console.*` automatically participates — no per-module wiring.
- When debugging cross-thread issues, the unified ring shows Rust + frontend events in real-time order (same `OffsetDateTime` source). Useful for "did the renderer ack before the UI's `set_shader_preset` invoke?" questions.
- The `target` field is the primary filter axis. Convention: Rust modules use their `module_path!` (`oa_shell::media`); frontend uses bracket prefixes (`[oa-launch]` becomes `frontend::oa-launch`). Keep prefixes short + namespaced.

---

## 2026-05-18 — Shared emu-thread state pattern (Arc<Mutex<T>> + Tauri command)

**Decision (formalized after 5 instances):** State that needs to flow from the emu thread to the frontend uses one canonical shape:

1. **Struct** with `#[derive(Clone, Debug, Default, serde::Serialize)] #[serde(rename_all = "camelCase")]`. Implements `Copy` when small enough; otherwise `Clone`.
2. **`Arc<Mutex<T>>` field on `AppState`.** Threaded into `EmuLoopArgs` via the same chain through `setup_two_window` / `setup_single_window` / `run_emu_render`.
3. **Initialized to `T::default()` in the Tauri `setup()` closure** alongside the other shared states; cloned once for `AppState`, once for the emu thread.
4. **Writer: emu thread**, on a cadence appropriate to the field (every frame for memory snapshots; every N frames for perf stats; on every state transition for TAS / video / rewind).
5. **Readers: Tauri commands** named `get_<field>` that lock + clone + return. Pattern: `fn get_X(state: tauri::State<'_, AppState>) -> Result<X, String> { Ok(*state.X.lock().map_err(|_| "X poisoned")?) }`
6. **Reset on `UnloadRom`** so the next launch does not inherit stale data.

This pattern has now been applied to:
- `SharedRewindState` (Phase 4 slice B)
- `SharedTasState` (Phase 4 slice C)
- `SharedVideoState` (Phase 4 slice D)
- `MemorySnapshot` (Phase 4 slice E) — `Vec<u8>` body, not Copy
- `SharedPerfStats` (debug-console pass, 2026-05-18)

**Why this is the right shape (not events, not channels):**
1. **Idempotent reads.** The frontend polls when it needs the value; missing a frame's update never matters because the next read still gets the latest. Events would require buffering on the frontend side to handle "what was the value when I opened the dialog."
2. **Mutex is uncontended in practice.** Writer touches the lock at most once per frame; readers poll at 1–4 Hz from Tauri commands. The window between writer-release and reader-acquire is always non-overlapping. Tried `RwLock`; not worth the API overhead for this pattern.
3. **Per-frame copy is cheap.** Largest state in the set (`MemorySnapshot`) is about 30 KB; the others are under 500 bytes. Cloning under the lock and releasing fast keeps the writer side never blocked.
4. **Resetting on `UnloadRom` is mandatory.** Without it, the HUD shows "60 fps" forever after the game closes, the memory inspector dumps stale bytes, etc. The reset belongs with the rest of the unload cleanup (rewind ring clear + video flush + milestone evaluator clear). Centralizing it in one block is a maintenance anchor.

**Recipe for adding a new shared state:**
1. Define `SharedFoo` struct next to the existing ones (around line ~280 in `main.rs`).
2. Add `foo: Arc<Mutex<SharedFoo>>` to `AppState` (preserve doc-comment style).
3. Init `let foo = Arc::new(Mutex::new(SharedFoo::default()));` in the setup closure.
4. Thread through both setup functions + `run_emu_render` (4 signature sites).
5. Update from the emu loop at chosen cadence; reset in the `UnloadRom` handler.
6. Add `get_foo` Tauri command following the standard pattern; register in `invoke_handler`.
7. Frontend polls via `invoke<SharedFoo>("get_foo")` — typically inside a `createEffect` gated on a `visible` signal so we do not pay IPC cost when not displayed.

The pattern's mechanical reliability across 5 implementations is the evidence it is correct.

---

## 2026-05-19 — Keyboard-heavy systems: hybrid RetroPad + keyboard passthrough (MAME / MSX / future computers)

**Decision:** For systems that need more inputs than a 12-button RetroPad covers — MAME's long-tail (mahjong, pinball, system-level controls), MSX (keyboard-driven home computer), and the eventual vintage-computer wave — adopt a **three-phase hybrid** rather than expanding our bindings tables indefinitely or punting entirely to the core's native input UI:

1. **Phase 1 — minimal OA bindings + document the native escape hatch.** For MAME specifically: add `SERVICE`, `MAME_MENU`, `P2_START`, `P2_COIN` to the existing 12-button table so the operator + local-multiplayer common cases work through OA's bindings dialog. Document the **TAB workflow** in `docs/cores/mame/README.md` — pressing TAB in-game opens MAME's own input config, which is per-driver-aware and stores remaps natively. This covers ~95% of arcade games out of the box with no new plumbing.

2. **Phase 2 — keyboard passthrough infrastructure (`oa-libretro`).** Wire `RETRO_DEVICE_KEYBOARD` so a keyboard-shaped system can receive raw key events in parallel with the RetroPad. Add a **"Game focus" toggle** (Tools menu checkbox + hotkey, RetroArch convention is Scroll Lock) that switches OA's keyboard between "OA controls only" (F1-F8 / Esc trigger OA actions) and "everything passes to the core" (the core eats all key events). Unlocks mahjong, pinball-with-shift-flippers, MAME's TAB menu without conflicts, MSX BASIC input, and every future home-computer system.

3. **Phase 3 — analog input (deferred until a real game demands it).** Steering wheels (OutRun), trackballs (Marble Madness, Centipede), paddles (Arkanoid), yokes (After Burner II). Touches `oa-input` (gilrs axes), `bindings.rs` (analog isn't a bitmask — needs a separate axis-binding schema), and the libretro analog device declaration. Defer until the operator wants to play one of these.

**Why this hybrid wins:**
1. **The common case stays simple.** 80%+ of arcade games are stick + 1-6 buttons + Start + Coin — Phase 1 covers that with no new infrastructure.
2. **The long tail uses MAME's existing solution.** MAME already has a robust per-driver input config UI reached by TAB. Building OA-side mappers for 40,000 arcade drivers + cabinet hardware categories would be a massive duplication of MAME's work; passing raw keyboard through and pointing users at TAB delegates the long-tail to the tool that already solved it.
3. **Phase 2 is cross-system infrastructure, not a MAME thing.** MSX (already in the catalog as a queued system), eventual Amiga / Atari ST / C64 / ZX Spectrum / Apple II / MS-DOS via DOSBox — all need keyboard passthrough too. Treating Phase 2 as "raw keyboard support" not "MAME input" means the next computer-shaped system onboarding is free.
4. **The Game-focus toggle pattern is proven.** RetroArch ships exactly this — a toggle that gates whether OA hotkeys or the core eat keypresses. Picking up the same model means our hotkey muscle-memory transfers and we can borrow conventions directly.
5. **Analog is genuinely different — defer until forced.** The bit-mask model `bit_for(sys, button) -> u32` is wrong shape for axes (value 0-65535, not on/off). Doing analog right means a parallel axis-binding system, more crate surface, and `gilrs` analog stick handling we haven't built. Phase 3 lands when a real game demands it, not as speculative scaffolding.

**Considered and rejected:**
- **Bigger static MAME button table.** 30+ buttons (B1-B10, all SF moves, all flippers, etc.) clutters the per-system bindings editor for users playing Pac-Man, doesn't solve analog, and still doesn't help MAME drivers using literal keyboard letters. Wrong shape.
- **Per-game input profiles via MAME's listxml.** Ingesting MAME's ~80 MB listxml + categorizing each driver's input type would let OA auto-pick "2-button" vs "6-button" vs "lightgun" profiles. Heavy ingest, and it still needs Option C / Phase 2 underneath for the keyboard-y categories. Worth revisiting after Phase 2 lands and we have a year of MAME usage data to know if auto-profile would pay back the implementation cost.
- **Punt to MAME's native input menu only (no OA bindings for MAME at all).** Inconsistent with every other system in OA — users would learn one workflow for NES/SNES/etc. and a different one for MAME. The hybrid keeps the common path consistent and lets the long tail escape into MAME natively when it has to.

**Where this applies going forward:**
- Phase 1 is one diff (4 buttons + docs). Ship alongside MAME onboarding hardening.
- Phase 2's keyboard-passthrough work in `oa-libretro` lands once and unlocks every keyboard-shaped system. Plan it as `oa-libretro` infrastructure, not under MAME.
- When MSX onboarding lands, it inherits the Phase 2 infrastructure with zero additional work — the system just registers as wanting keyboard passthrough and the existing pipeline handles it.
- Phase 3 analog gets its own DECISIONS entry when it lands — likely paired with whichever system forces the issue (OutRun = Saturn / Dreamcast / arcade; Marble Madness = MAME; Arkanoid = NES via Vaus paddle).
