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

**Companion docs:** `docs/features/ui-polish/UI_AUDIT.md` is the surface-by-surface inventory of the pre-redesign UI; `docs/features/ui-polish/UI_MENU_BAR_PLAN.md` is the redesign proposal with the full field-to-menu mapping and visual treatment spec.

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

---

## 2026-05-19 — Internal drag-reorder as a UX pattern (scope + library choice)

**Context.** External OS→app drag-drop (files from Explorer into the library window) is parked as unreliable — see `docs/PARKING_LOT.md` 2026-05-19 entry. Internal HTML5 drag-drop (within the WebView page, between DOM elements) is a different code path that lives entirely inside Chromium and is unaffected by the WebView2/transparency issues. After a UX-audit pass, drag-reorder slots into the app as the right interaction for several lists currently using up/down buttons or fixed orders.

**Decision: ship internal drag-reorder across Tier 1, Tier 2, and Tier 4 surfaces; explicitly leave Tier 3 alone.** Reference implementation is the Region priority list — if the pattern feels right there we roll through the rest in priority order.

**Tier 1 — high value, exists today, quick to wire (Diff B scope):**
- **Region priority** — Settings → Library → Region & version priority. Today four ↑/↓ + Reset buttons per row; ordering is the only thing this list does. Reference implementation.
- **Library folders list** — Settings → Library. Today Add/Remove only; scan-priority order matters (first-folder-wins on filename collisions) but is undocumented. Adding drag-reorder exposes the ordering control without inventing a separate setting.
- **Sidebar systems list** — Phase 2.6 already ships flat drag-reorder via native HTML5 API (`draggable` + `onDragStart/Over/Leave/Drop/End`). Auditing 2026-05-19: implementation is solid (drop indicator line above target, source dims to 40% opacity, end-of-list drop zone for append, hit detection on upper/lower half of each row for insert-above-vs-below, draggable-only-when-expanded). Converting to `solid-dnd` for cosmetic consistency would be churn without obvious benefit; keeping native HTML5 here.
- **Right sidebar widgets** — Hero / Title / Metadata reorder. Highest perceived polish per minute of work; visible to every operator every session.

**Tier 2 — exists today, real but secondary (one-off follow-ups when each surface comes up next):**
- **Per-system core priority list** — when multiple `.dll` cores are present for one system, give the user "try X first, fall back to Y" ordering instead of the current single "default core" dropdown.
- **Per-game milestones list** — display order matters for visual hierarchy.
- **Per-folder import rules** — when multiple rules apply, order resolves which wins.

**Tier 3 — explicitly skip:**
- Save state slots (numbered 0-9 by F5/F8 muscle memory; reordering breaks that).
- Per-game settings drawer tabs (fixed order is convention).
- Quick Settings menu items (fixed order is convention).
- Cheats list (toggle-based, order is cosmetic).
- TAS recordings / video clips (organizational only, users rarely curate).
- Bindings (one slot per button, no reorder concept).

**Tier 4 — speculative; revisit when the underlying feature ships:**
- **Library tiles into playlists/collections** — drag-drop tiles between sidebar entries. Needs the "Custom playlists" feature first (in VISION).
- **Nested system groups** ("PC Engine family" containing TG-16 + PCE-CD as children, per Phase 6 plans). Drag systems into/out of groups becomes the obvious affordance once the grouping data model exists.
- **Shader preset chain editor** — drag-reorder passes when user-defined multi-pass shader chains arrive (Phase 3 polish).

**Library: `solid-dnd`.** ~30 KB Solid-native drag-drop lib with sortable + draggable primitives + accessible keyboard reorder out of the box. Chosen over `@neodrag/solid` (no sortable primitive — would need manual implementation per surface) and rolling our own (~150 LOC base + the per-surface accessibility cost that becomes work-per-surface).

**Why this scope wins:**
1. **Tier 1 surfaces are the "ordering as primary interaction" cases.** That's exactly where drag pays for itself over buttons.
2. **Tier 2 has real users but lower frequency.** Including them now means the same lib + pattern gets reused; the marginal cost is small.
3. **Tier 4 is genuine future work** the data model isn't ready for. Including it in the decision doc commits us to the pattern when those features build out.
4. **Tier 3 explicitly out** to prevent the inevitable scope drift to "well couldn't we drag-reorder cheats too?" Each item there has a specific reason it doesn't fit.
5. **`solid-dnd` matches our framework choice.** No React/Vue baggage; works with Solid's reactivity model.

**Where this applies going forward:**
- Diff B ships Region priority as the reference implementation. If it feels right, Library Folders + Right Sidebar Widgets + Sidebar Systems audit land in the same diff.
- Tier 2 items land as one-off follow-ups when the relevant context comes up (e.g., per-system core priority during the next core onboarding that has two cores).
- Tier 4 items don't land standalone; they ship with the feature that builds the underlying data model.

---

## 2026-05-20 — Direct-launch CLI: forced single-window, hash-lookup, --system for ambiguous extensions

**Context.** External frontends (LaunchBox, BigBox, EmulationStation) launch standalone emulators by spawning the .exe with a ROM path. OA only opened to its library, so it needed a wrapper to slot into these frontends. This entry covers the three load-bearing decisions in the direct-launch CLI mode that ship `feat/direct-launch-cli`.

**Decision 1: Direct-launch forces single-window mode at runtime; operator's persisted preference (`OA_SHELL_MODE` env / `shell.json`) is left untouched on disk.**

**Why:** Single-window mode already hosts both the wgpu game surface and the transparent WebView for QuickSettings / toasts in the same HWND. Closing the window cleanly exits the process via the existing `CloseRequested → graceful_exit` path. Two-window mode would require spawning a "ghost" library WebView (just to host overlays) on top of the native game window — extra plumbing for no win.

**Considered and rejected:**
- **Respect `OA_SHELL_MODE` in direct-launch.** Would require a second WebView in two-window mode, invisible-until-Esc, with input-routing and focus-handling complications. Not worth the flexibility nobody asked for.
- **Persist single-window for the user.** Would surprise operators who deliberately set two-window mode for their library workflow. Runtime-only override keeps the dev experience right.

**Decision 2: SHA-1 hash lookup against `library_db` only for cart-shaped ROMs; CD images skip.**

**Why:** Hashing a .nes/.pce/.sfc cart is microseconds. Hashing a 4-8 GB CHD / ISO at boot is seconds-to-minutes — operator notices and the launch feels broken. And libretro-database doesn't canonicalize CD images by content hash (it uses disc IDs / track signatures); the lookup wouldn't match anything useful anyway. For carts, the lookup populates `matched_entry_id`, the frontend pulls the matched RomEntry, and the existing per-game-overrides cascade applies (patches, custom core options, shader, rewind config, analog routing, bezel).

**Considered and rejected:**
- **Skip the lookup entirely; always treat direct-launch as ad-hoc.** Loses per-game overrides — operators who carefully tuned a specific game in OA's library would see the wrong settings when launching from LaunchBox. Bad ergonomics.
- **Hash all ROMs including CDs.** Pays multi-GB hash cost for ~0 hit-rate gain.
- **Look up by file path instead of hash.** Fragile — operator moves a ROM, lookup misses. Hash is content-addressable; that's the point.

**Decision 3: Ambiguous extensions (`.cue`, `.chd`, `.iso`, `.m3u`, `.pbp`, `.zip`, `.7z`) require explicit `--system`; the CLI errors with a candidate list rather than guessing.**

**Why:** A `.cue` could be PCE-CD, Sega CD, Saturn, PSX, Neo Geo CD, 3DO, or PCFX. There's no reliable way to know from the filename alone, and guessing wrong loads the wrong core (no game, just a confusing error). Errors-up-front is the correct UX: LaunchBox / BigBox can configure `--system <slug>` per-platform emulator entry; EmulationStation users wrap with a shell snippet per platform. Cart extensions stay auto-inferred because they're unambiguous.

**Considered and rejected:**
- **Default to current `ACTIVE_CORE.md` system.** Surprising and silent. Operator launches a Saturn `.cue` while atari7800 is the active core, gets atari7800-flavored failure.
- **Try library DB hash lookup for ambiguous extensions before failing.** Multi-GB hash cost just to disambiguate; not worth it given that launchers can configure `--system` once per-platform.
- **Read CD-image headers to detect system.** Possible but fragile — Sega CD vs PSX vs Saturn header signatures vary across image formats (`.cue`/`.chd`/`.iso` each store sectors differently). Defer to v2 if real-world LaunchBox configs make `--system` annoying.

**Why these three decisions sit together:**

They form a coherent ergonomic contract:
1. The shell window behavior is predictable and reversible (decision 1).
2. Tuning carried into OA via the library applies automatically (decision 2).
3. The CLI fails loudly when it can't be safe (decision 3).

Operators who set OA up once via LaunchBox / BigBox / EmulationStation get the right system, the right core, the right per-game overrides, the right window behavior — without needing a wrapper script. That's the bar this feature had to clear.

---

## 2026-05-21 — `cargo tauri build` does NOT bundle `cores/` or `system/`

**Decision:** The release build emits `oa-shell.exe` with empty `cores/` and `system/` folders next to it. Operators populate both folders themselves from libretro buildbot nightlies (https://buildbot.libretro.com/) for cores and from their own legally-acquired BIOS dumps for `system/`. We never ship cores or BIOS bundled with the installer, and we won't add a build script that copies them from a developer's local `target/debug/` over.

**Why:** Both folders may contain copyrighted material that we have no right to redistribute.

- **Cores** are GPL-2.0 (libretro-frontend builds of Mednafen / MAME / etc.). Redistributing the .dll bundled with our installer means our installer becomes a GPL-2.0 derivative bundle, which forces the GPL on parts of the shell that don't need it. The 2026-05-16 libretro pivot (see "Architecture pivot: libretro frontend") was specifically driven by this concern — dynamic loading via `libloading` severs GPL propagation, but that severance only holds if we DON'T ship the cores in the installer.
- **BIOS files** (PCE-CD `syscard3.pce`, PSX `scph5500.bin`, Saturn `sega_100.bin`, NDS `bios7.bin`/`bios9.bin`/`firmware.bin`, etc.) are all proprietary console firmware. Operators acquire them legally from their own hardware / legitimate dump archives; shipping any of them is straightforwardly copyright infringement.

**Considered and rejected:**
- **`build.rs` script that copies `target/debug/cores/` → `target/release/cores/`.** Convenient for the dev loop but the .exe would still ship empty to end users (different machine, no local cores folder). And in CI it'd silently package whatever cores the build machine happened to have.
- **Bundle empty cores/system folders with a README pointing at the buildbot.** Acceptable in principle but the operator still has to do the work, so the README + folders add little vs. a single docs page.
- **Bundle a "stock cores pack" via Tauri's resources mechanism.** Same GPL + BIOS-copyright problems.
- **Ship a downloader UI that fetches cores at first run.** Already exists (`apps/oa-shell/src/core_installer.rs` + the buildbot catalog UI from commit `3f22eac`). That's the right delivery vector — operator-initiated, opt-in download, no installer-bundled cores.

**How it manifests in practice:**
- New installs see "no libretro core found" in `oa-current.log` on first launch until the operator opens the in-app Cores page and installs cores via the buildbot catalog UI.
- The dev loop works because the developer manually populates `target/debug/cores/` once and the .exe-relative `cores/` resolution picks them up. `cargo tauri dev` runs against `target/debug/` so cores live there.
- When iterating with `cargo tauri build`, the developer manually copies the same cores into `target/release/cores/` once.

**Implication for future PRs:** Don't add cores-bundling automation. If a contributor (or a future Claude session) suggests it as a quality-of-life dev convenience, point them at this entry — the licensing rationale survives the convenience argument.

---

## 2026-05-21 — Honor libretro `SET_CORE_OPTIONS_DISPLAY` + `UPDATE_DISPLAY_CALLBACK`

**Decision:** Implement libretro's option-visibility envs (55 + 69). Cores that hide options based on the current values of other options (e.g. Beetle PSX's "Lightgun crosshair color" disappearing when "Lightgun" is off) now get their wish: the per-system / per-game settings panel filters those keys out at render time and re-runs the visibility check whenever a value changes.

**Why:** The accept-and-ignore stub at `state.rs:820` predates the per-system core-options surface shipping. Once that surface existed, ignoring the visibility hint meant users would see options that wouldn't take effect (because the dependent flag was off) — a worse UX than RetroArch parity. The marginal complexity is small: a `HashSet<String>` on `State`, one extra trait method (`refresh_option_visibility`) called after `set_option`, and a frontend `.filter()`.

**How it works:**
- `cb_environment::SET_CORE_OPTIONS_DISPLAY` flips `State.hidden_options` per (key, visible) pair the core pushes.
- `SET_CORE_OPTIONS_UPDATE_DISPLAY_CALLBACK` stores the core's `update_display_t` function pointer in `State.update_display_cb`.
- The emu thread calls `core.refresh_option_visibility()` after every `SetCoreOption` / `ApplyCoreOptions`, which invokes the stored callback. The core synchronously fires `SET_CORE_OPTIONS_DISPLAY` re-entrantly for every key whose visibility changed.
- `State.hidden_options` is snapshotted into `CoreOptionsFile.hidden_keys` on disk (alongside schema + values) and surfaced via `CoreOptionsSnapshot.hiddenKeys` to the frontend.
- The mutex on `State` is released BEFORE calling the core's callback to dodge the obvious deadlock — the core's re-entry into `cb_environment` needs to acquire that same lock.

**Considered and rejected:**
- **Store `visible: bool` on `CoreOption` itself.** Cleaner shape but the schema captured to disk is then mutated per session (visibility is dynamic; schema definitions are not). Keeping the visibility set as a sibling field of the schema preserves the "schema is the immutable core declaration" invariant.
- **Emit a Tauri event when visibility changes, instead of having the frontend re-fetch.** The frontend already calls `refetch()` after `set_system_core_option` / `set_game_core_option`. An event would let the panel update if visibility changed because of an emu-thread side-effect (e.g. core flipped visibility mid-frame from a game-state predicate), but no shipped core does that — every UPDATE_DISPLAY_CALLBACK invocation is frontend-initiated. Adding the event surface for hypothetical cores is YAGNI.
- **Don't bother with the UPDATE_DISPLAY_CALLBACK env (just honor the initial visibility set during load).** Tempting given the complexity delta, but the whole point of dynamic visibility is dependent options — e.g. "Lightgun crosshair color" should appear the moment the user flips "Lightgun" on, not on next core reload. Without env 69 we'd ship a stale visibility set.

---

## 2026-05-21 — Library folders: SQLite is the single source of truth

**Decision:** Drop the localStorage `oa.settings.v1.libraryFolders` array. The SQLite `folders` table (created in slice 2.7.C alongside the Import Wizard) is now the single source of truth for tracked library folders. The Settings → Library tab, the Rescan-all menu item, the file-system watcher (`set_watched_folders`), and the ImportWizard's "pick a tracked folder" dropdown all read through `list_folders` via the settings store's SQLite-backed signal.

**Why:** Operator reported "no folders tracked" in Settings even though 5 folders were actively imported with hundreds of ROMs. Diagnosis: the SQLite `folders` table held the 5 paths correctly but the localStorage `libraryFolders` array was empty. The ImportWizard's commit step writes to both stores (`add_folder` + `setLibraryFolders([...tracked, f])`), but the two had diverged at some point — most likely a manual "remove all" click on the Settings list, a write race during the v0→v1 settings schema upgrade, or a clean of WebView2 storage between dev sessions. The mirror line in ImportWizard.tsx:530 was always a known half-measure: its own comment said *"Slice C leaves both stores live; a future slice can migrate the watcher to read from SQLite."*

A single source of truth removes the class entirely: ANY divergence between localStorage and SQLite means the UI lies about reality. The fix that actually closes that gap is dropping one of the two stores.

**Why SQLite and not localStorage:**
- SQLite already carries the richer schema: scan-subfolders + subfolders-are-systems + watch-enabled + last-scanned-at + folder_rules (FK-cascaded). localStorage only stored a string array.
- SQLite persists across WebView reinstalls / profile clears (it lives in `appData/.../library/games.sqlite`, not `localAppData/.../EBWebView/`). localStorage routinely gets nuked when WebView2 reprovisions a profile — particularly common in `cargo tauri dev` iteration.
- SQLite is shared across windows (single-window vs two-window mode) without origin-isolation concerns; localStorage isn't guaranteed to be in all WebView configurations.

**Migration shape:**
- Schema v12: new `folders.display_order INTEGER NOT NULL DEFAULT 0`. `list_folders` orders by it. `add_folder` sets it to `MAX(display_order) + 1` so adds go to the end. New `reorder_folders(orderedIds)` bulk-update for drag-reorder. Backfilled from `rowid` so existing folders keep insertion order on first launch after upgrade.
- New `migrate_folders_from_local_storage(paths)` Tauri command — idempotent: paths already in `folders` are skipped. Called once on settings-store init with whatever's in the legacy localStorage payload, then the field is dropped from the persisted JSON forever.
- Frontend `settings` API: `libraryFolders()` still returns `string[]` (drop-in compat for the watcher + Rescan-all). `libraryFolderRows()` returns the full Folder rows for UI that needs ids. `addLibraryFolderPath`, `removeLibraryFolderById`, `reorderLibraryFolderIds`, `refreshLibraryFolders` replace the old `setLibraryFolders` setter — each one writes through to SQLite then refreshes the signal.

**Considered and rejected:**
- **Keep both stores, fix the mirror.** That keeps the divergence class alive — a single dropped `setLibraryFolders` call somewhere in the wizard flow is all it takes to break Settings again. Single source of truth is the only fix that survives further refactors.
- **Move to a sidecar JSON file in `appData/.../library/folders.json`.** Better than localStorage (persistent + cross-window) but worse than SQLite because (a) we already have the SQLite table, (b) JSON loses the `folder_rules` FK relationship, and (c) two persistent stores is exactly what we're escaping.
- **Keep localStorage as the source of truth.** Would mean migrating folder rules + scan settings OUT of SQLite. Strictly worse: localStorage doesn't survive WebView reprovisioning, doesn't have transactions, and doesn't enforce relational invariants.
- **Make the migration a one-way "settings store calls SQLite on init, then keeps the localStorage mirror updated on writes."** Two writes per mutation = same divergence risk on partial failure.

---

## 2026-05-21 — Shared analog input infra: Phases E + F + G close the umbrella

**Decision:** Wire libretro's three remaining input-infra envs — multi-port device-type (extension of Phase A from port-0-only to all 5 ports), rumble interface (env 23, `GET_RUMBLE_INTERFACE`), and sensor interface (env 25, `GET_SENSOR_INTERFACE`) — and flip the per-core ROADMAP bullets they close. The "Phase 3 shared analog input infrastructure" umbrella entry in NEXT.md DEFERRED is removed; ~12 specific items across 8 cores now ship.

**Why:** Audit during the 2026-05-21 follow-up review (after the operator asked "I thought analog input infra was done") showed Phases A–D shipped substantively (per-game device-type override, per-button analog pressure, mouse-as-stick analog source, per-game UI) but the project's own NEXT.md still listed the umbrella as DEFERRED, and per-core ROADMAPs still had ⬜ bullets citing "gated on shared analog-input infra" for items that the shipped infra actually closes. Three genuinely-still-open siblings remained:
- **Multi-port device-type** — `arm_libretro_device` only wrote port 0, so 7800 twin-stick / SNES Mouse on port 2 / arcade coop light-gun were still blocked.
- **Rumble interface** — declined with `false` in `cb_environment` (state.rs:975), so cores requesting rumble silently got no haptic feedback even though gilrs already supports it.
- **Sensor interface** — same `false` decline, so GBA tilt / solar / NDS gyroscope cores either ignored sensors or crashed reading null function pointers.

**Phase E — multi-port device-type:**
- `GameOverrides` gains `libretro_device_port1..4: Option<u32>` siblings to the existing `libretro_device` (port 0 kept for back-compat). `GameOverrides::libretro_device_ports()` returns the 5-element array.
- `arm_libretro_device` walks ports 0..=4 and dispatches `SetPortDevice` for each.
- `set_libretro_device_for_game` takes an optional `port: Option<u32>` so the same Tauri command writes to any port.
- `PerGameSettingsDrawer` Input tab adds a collapsible "+ Additional ports (1–4)" section. Auto-expands when any port-1..4 override is non-null so an operator returning to a multi-port game sees their config without hunting.

**Phase F — rumble interface:**
- New FFI types in `oa-libretro/src/ffi.rs`: `retro_rumble_effect`, `retro_rumble_interface`, `retro_set_rumble_state_t`.
- `State.rumble: [[u16; 2]; 5]` — per (port × strong/weak), 0..=65535.
- `cb_set_rumble_state` trampoline writes the cell; env 23 hands the core our interface struct.
- `LibretroCore::rumble_snapshot()` reads the array.
- `InputPoller::dispatch_rumble(strengths)` builds long-lived gilrs `Effect` per (port × kind) lazily on first non-zero write, varies magnitude via `Effect::set_gain` (continuous-rumble polls don't rebuild), stops on strength=0, rebuilds on gamepad rotation.
- Shell's emu thread calls `core.rumble_snapshot()` + `input.dispatch_rumble(...)` after each NORMAL forward-play `run_frame`. The all-zeros snapshot fast-paths to a no-op for cores that don't use the env.

**Phase G — sensor interface:**
- FFI types: `retro_sensor_interface`, `retro_set_sensor_state_t`, `retro_sensor_get_input_t`, RETRO_SENSOR_* enable/axis constants.
- `State.sensor_enabled: [[bool; 3]; 5]` (accel/gyro/illum per port), `State.sensor_values: [[f32; 7]; 5]` (per axis).
- Phase 1 fallback: arrow-keys-as-tilt feed `sensor_values[0][ACCEL_X|Y|Z]` so GBA Boktai / Kirby Tilt 'n' Tumble / WarioWare Twisted! are playable without OS-level accelerometer access. Real motion (Windows Sensor API / Linux iio / macOS Core Motion) is deferred until operator hardware demands it.
- `core_ref.sensors_enabled()` guards the per-frame sensor pump so the 95% of cores that don't use sensors pay nothing.

**Considered and rejected:**
- **Trackball-delta dispatch.** Libretro spec says `RETRO_DEVICE_MOUSE` is delta-based; our existing pointer-as-mouse path may already produce delta values via the per-frame coordinate diff. Verify-as-needed when an operator tests an actual MAME arcade trackball game (Marble Madness, Centipede). Listed in NEXT.md DEFERRED rather than shipped because the verification + potential 80 LOC fix is operator-triggered.
- **Real accelerometer access.** Windows Sensor API + Linux iio + macOS Core Motion. Postponed because (a) keyboard-tilt fallback covers the playable bar today, (b) the actual sensor hardware most users have isn't connected to their PC (tablet IMUs and phones aren't typically plumbed), and (c) the gesture vocabulary GBA tilt games use is binary-ish ("tilt left to roll Kirby left") which keyboard arrows model fine.
- **Real solar-sensor input.** Same reasoning as accelerometer plus an additional concern: real ambient light readings during emulation are usually wrong anyway (operators play indoors at night; Boktai requires sunlight to recharge). The mock-zero illuminance the env returns is honest about "no sensor"; operators use mGBA's per-game core-options to fix illuminance to a target level instead.
- **Effect rebuild per dispatch tick.** Simpler than `set_gain` + lazy build, but cores call set_rumble_state every frame for continuous rumble (RetroArch logs confirm); rebuilding 60×/sec per port-effect is wasteful when `set_gain` keeps the same effect alive.
- **Fire-and-forget rumble channel (mpsc emu → input poller thread).** Adds latency + Send concerns for the gilrs Effect handle. The snapshot-pull pattern is simpler and the per-frame cost of `core_ref.rumble_snapshot()` is one mutex acquisition + a 5×2 u16 copy.

---

## 2026-05-21 — Multi-core CPU awareness (rayon + tokio blocking + parallel boot)

**Decision:** Adopt rayon as the workspace work-stealing pool for embarrassingly-parallel CPU work, and use it (plus tokio::task::spawn_blocking, plus std::thread::spawn for boot-time loaders) to parallelize the four cold-path bottlenecks in OA: media sync, ROM hashing, rewind buffer compression, and boot-time I/O.

**Why:** Pre-survey of the codebase confirmed there was zero rayon, zero `par_iter`, and zero work-stealing pool anywhere. Long-running workers (emu thread, audio callback, renderer present, video capture) each had their own dedicated `std::thread::spawn`, which is correct for their hot-path latency requirements — but **none of the cold paths used parallelism at all**. Library scan, media sync, image decode/resize/encode, ROM hashing were all single-threaded. On modern desktops with 8–16 cores, OA was leaving 70–90% of CPU on the table during ingest.

**Why these four specifically:** They're the load-bearing UX moments where parallelism is most felt:
1. **Media sync** — first-scan-of-a-large-library is the moment a new user judges the product. Decode + resize + WebP-encode is pure CPU.
2. **ROM hashing** — reads + SHA-1s thousands of files; the bottleneck of library identification.
3. **Rewind buffer** — bounded by RAM; compression lifts the rewind-depth ceiling by 5–10× for free.
4. **Boot-time I/O** — felt as "the wait between double-click and first paint." Four independent loads ran sequentially.

**Mechanism choices (different tool for each phase):**
- **rayon** for the ROM hash pre-pass: `par_iter().filter_map(...).collect()` over a deduped vec is the natural shape; rayon's stealing pool is ideal for variable-cost CPU work. Called inside `tokio::task::spawn_blocking` because the surrounding `resolve_rom_hashes_for_system` is async — keeps the rayon work off the async runtime threads.
- **tokio::task::spawn_blocking** for media-sync per-image work: the call site is inside an `async fn` running under `futures::stream::iter().buffer_unordered(8)`. spawn_blocking is the idiomatic bridge from a single async task to a CPU pool; equivalent multi-core utilization to rayon here, but cleaner integration with the existing async control flow.
- **std::thread::spawn** for boot-time loaders: four genuinely independent disk-bound jobs (sweep_temp, read_media_db, read_media_prefs, library_db::open) where we want each to run to completion on its own thread and join at point-of-use. No work stealing needed; no async runtime spun up yet at this point in setup.
- **In-place zstd** for the rewind ring: per-snapshot synchronous compression at zstd level 1 (fast preset, 300–600 MB/s). No worker thread because the per-call cost on the emu thread is acceptable (sub-1ms for PCE/Lynx states; ~30–50ms on PS2 once every 100ms of wall clock under default settings). A future revision can move compression to a worker if PS2/Saturn rewind sees real-world stutter complaints.

**Considered and rejected:**
- **Wholesale tokio runtime for everything.** OA's existing model is async-on-demand (only the Tauri commands and a few async I/O paths). Forcing the whole app into tokio just to get the blocking pool would balloon the dep tree and inject latency everywhere. Mixed-mode is fine — rayon for sync CPU bursts, spawn_blocking for async-bound CPU bursts, std::thread::spawn for one-shot independent boot work.
- **Two-instance run-ahead (related context: see the latency feature survey).** Mednafen-derived cores have global C state. Loading the same `.dll` twice in one process is not survivable for ~95% of OA's lineup. Single-instance run-ahead (a future Phase) is fine; multi-process parallelism for run-ahead is out of scope for this slice.
- **Rayon's global pool with no thread limit.** rayon's default pool spawns ~num_cpus threads. Fine for our cold-path bursts. Considered capping it to leave headroom for the emu thread, but the cold paths and the emu thread never run concurrently (no game is running during ingest), so the cap would only hurt.
- **Per-image worker thread pool managed by OA itself.** Tokio's blocking pool already exists, already has ~512 threads, already integrates with the async runtime. Reinventing it would be ceremony with no upside.
- **Async file I/O via tokio::fs for the boot loaders.** Mixing async with `tauri::Builder::default().setup()` (which is sync) requires a runtime handle to block_on. std::thread::spawn is simpler and the I/O cost is wall-clock-bounded by the disk, not by thread count.
- **Background compression for the rewind ring on a dedicated worker.** Cleaner long-term, but adds Send + Arc<Mutex> + mpsc bookkeeping for marginal benefit at current state sizes. Defer until real-world PS2/Saturn rewind playtest surfaces stutter.

---

## 2026-05-21 — Post-import sync ordering: identify before media + metadata

**Decision:** In the Import Wizard's post-commit flow, `resolve_rom_hashes_for_system` must complete (awaited) before `sync_media_for_system` and `sync_metadata_for_system` are fired. Media + metadata syncs may then fire in parallel — they don't depend on each other, only on stamped sha1s being present in `library_db`.

**Why:** `sync_media_for_system` reads its `only_identified` filter once at the top of the function (`apps/oa-shell/src/media.rs::sync_media_for_system`, ~line 1541). It enumerates `entries.filter(|e| canonical_by_id.contains_key(&e.id))` where `canonical_by_id` is populated by looking up `library.lookup_rom_hash(sha1)` for each entry. If sha1 is `None` on the row (the normal state after a fresh import, before identification runs), the filter retains zero entries and the sync no-ops.

`sync_metadata_for_system` doesn't have an equivalent hard filter, but its primary match path is sha1-based; without stamped sha1s it falls back to fuzzy title matching, which catches roughly 10% of a real Genesis library vs. >90% with sha1s.

Confirmed in the field on a 1160-entry Genesis import (see `appData/logs/oa-current.log` around 2026-05-21 13:52:24): `sync_media` ran at T+0ms with `keeping 0 hash-matched entries`; `sync_metadata` ran at T+2ms with `matched 120, unmatched 1040`; `resolve_rom_hashes` finished at T+387ms with `matched 1081`. Operator saw "found matches but nothing changed in library and no metadata."

**How (frontend):** `frontend/src/components/ImportWizard.tsx::commit()` step 4 is `await invoke("resolve_rom_hashes_for_system", ...)` per touched system; step 5 fires `sync_media_for_system` + `sync_metadata_for_system` as fire-and-forget. The serial await adds ~200–500ms to wizard completion time depending on library size — acceptable given the correctness benefit (cover art + metadata actually populate on first sync, instead of needing a manual second pass through `Settings → Library → Identify ROMs / Sync media / Sync metadata`).

**Considered and rejected:**
- **Make sync_media re-evaluate the filter mid-walk.** Would let it pick up sha1s as resolve_rom_hashes stamps them. But the filter is upfront-by-design — it determines the total work unit count emitted to the progress event stream. Reactive filtering would complicate the progress UI more than it helps.
- **Drop the `only_identified` filter entirely.** Was considered but the filter exists to prevent fuzzy-matching against the wrong region of a multi-region game (the typical false-positive cause for unidentified ROMs). Keeping it is correct; ordering the calls is the right fix.
- **Have resolve_rom_hashes auto-trigger sync_media on completion (server-side).** Would solve it transparently but couples two concerns the operator may want decoupled (e.g. identify without ever syncing covers, for headless library audits). Frontend-side ordering preserves the operator's ability to opt out via the wizard's "Skip sync" button.
- **Parallel resolve across multiple systems.** Currently sequential — `resolve_rom_hashes_for_system` holds the LibraryDb write lock for its DB applies. Parallelizing wouldn't speed up a multi-system wizard (they'd queue), and the existing per-system progress UI matches the sequential model. Revisit if a real multi-system import becomes painful.

**Other paths with similar shape (audited 2026-05-21):**
- `App.tsx::autoIdentifyAfterIngest` (drag-drop / "Add library folder" / "Rescan tracked folders"): only fires `resolve_rom_hashes_for_system`, never the media/metadata syncs. Pre-existing gap — operator sees ROMs identified but no covers. Tracked as a separate fix; the wizard race fix doesn't address it.
- `SettingsPage.tsx` per-system "Sync media" / "Sync metadata" / "Identify ROMs" buttons: each is its own awaited handler. No race within each, but a user who clicks "Sync media" without first clicking "Identify ROMs" hits the same `only_identified` filter and sees zero matches. User-triggered, lower priority — the UI could grey out "Sync media" until "Identify ROMs" has run at least once, but that's a UX call rather than a correctness one.

**Follow-up — server-side authoritative sha1 lookup (added in the same fix branch after operator confirmed a second bug):**

The frontend ordering fix above isn't enough on its own. After awaiting `resolve_rom_hashes_for_system`, the wizard still calls `sync_media_for_system(systemId, entries)` with the **original `entries` array constructed at scan time**, before identification stamped any sha1s. Every `entry.sha1` is `None` even though the matching `library_db.games` rows now have sha1 populated. `canonical_by_id` stayed empty and the `only_identified` filter still kept 0 entries.

Server-side fix: `sync_media_for_system` and `sync_metadata_for_system` now look up `library.find_sha1_by_id(&e.id)` for each entry, falling back to `entry.sha1` only if the DB has nothing. The DB is the authoritative source — the frontend payload is just a list of which entries to consider.

Net effect:
- `sync_media`: `keeping 0 hash-matched entries` → `keeping ~95% hash-matched entries` on a typical Genesis library.
- `sync_metadata`: gains a parallel improvement — it now uses the **canonical no-intro title** from `rom_hashes.canonical_title` as the match key when the entry is hash-identified, falling back to the user's filename-derived title only for unidentified ROMs. Filename-fuzzy was matching ~10% (120 of 1160 on the Genesis confirmation case); canonical-exact matches ~95%.

New library_db method: `find_sha1_by_id(id) -> Result<Option<String>>` — cheap single-column lookup, no full row materialization. Used by both sync paths.

**Why server-side and not frontend re-fetch:** the same bug would lurk in any other call site that constructs `SyncRomEntry` from scan data instead of from a freshly-queried library row. Server-side hydration makes correctness independent of caller discipline — any caller can pass `sha1: None` on the payload and the server picks up the authoritative value from the DB.

**Third follow-up — metadata system-name routing (after the user reported sync_metadata still showing matched 120 / unmatched 1040 on Genesis):**

`metadat_system_name_for_extension(ext)` in `apps/oa-shell/src/metadata.rs` was the function that mapped an entry's extension to its libretro-database metadat system name (used to construct the cache path + fetch URL). It hadn't been touched since the TG-16-only first-bringup days:

```rust
fn metadat_system_name_for_extension(ext: &str) -> &'static str {
    match ext {
        "sgx" => "NEC - PC Engine SuperGrafx",
        "cue" | "chd" | ... => "NEC - PC Engine CD - TurboGrafx-CD",
        _ => "NEC - PC Engine - TurboGrafx 16",   // <-- wildcard fall-through
    }
}
```

Every non-CD non-SGX extension fell through to PC Engine. So `.md`, `.gen`, `.smd`, `.nes`, `.sfc`, `.gba`, `.sms`, every other system's extensions, all got bucketed into the PCE catalog (442 upstream entries). The metadata sync had only ever worked properly for PC Engine titles. The "matched 120" we kept seeing was an accidental fuzzy-overlap: 120 of the 1160 Genesis ROMs had generic enough names that they fuzzy-matched something in PCE's catalog. Cross-system data contamination — operator was getting PC Engine metadata stamped on every game from every other system that happened to fuzzy-match.

Fix: replaced with `metadat_system_name_for(system_id, ext) -> Option<&'static str>` that mirrors `rom_hashes::libretro_dat_refs_for_system`'s mapping — every system OA supports routes to its correct libretro-database name. TG-16-family extensions (`.sgx` / CD containers / cart) remain the only place where extension matters (they share `tg16` system_id but split into three upstream dats). `mame` and `3do` return `None` (no upstream metadat — MAME is set-based, 3DO's upstream dat has no metadata fields); those entries are counted as unmatched in the per-entry loop without attempting a fetch.

Net effect on Genesis: sync_metadata now fetches `Sega - Mega Drive - Genesis.dat` (~2400 entries) instead of `NEC - PC Engine - TurboGrafx 16.dat` (442 entries). Combined with the canonical-no-intro-title match key from the previous follow-up, match rate jumps from 10% to ~95% on a typical fully-identified library.

**Why duplicated mapping (vs sharing rom_hashes::libretro_dat_refs_for_system):** Considered. Rejected for now because rom_hashes uses `DatRef` (subdir + basename, supporting metadat/no-intro vs metadat/redump split + multi-dat merges like gb DMG+CGB), while metadata only needs a single basename per (system, ext). The duplication is small (one match arm per system) and the type shapes don't align cleanly. Worth revisiting if a third call site grows that needs the same mapping — at that point promoting to a shared `system_id → libretro-database name` table earns its keep.

---

## 2026-05-25 — Strategic locks from external-advisor planning session

**Context:** Operator + Claude collaborated with an external LLM advisor (ChatGPT) on strategic direction. Advisor consumed `docs/EXTERNAL_ADVISOR_BRIEF.md` and surfaced 6 feature concepts + 2 direct answers to the brief's open questions. Operator made decisions; this entry captures the strategic locks before implementation planning.

Full implementation plan landed at [`docs/PLANS/guided-setup.md`](PLANS/guided-setup.md).

### Decision A — Audience priority

**Decision:** Primary audience is **couch gamers** (controller in hand, monitor/TV across the room, want OA to work and stay out of the way). Secondary is **cabinet builders** (kiosk shell audience, served eventually by the kiosk shell plan). Tertiary is **desktop users** (already served by the existing UI).

**Onboarding order:** desktop → couch → cabinet. Every operator encounters OA first on a desktop install; from there they may move to couch or cabinet. The guided-setup arc is the first step every new operator walks regardless of where they end up.

**Why:** OA's premium-feel positioning + per-system theming already implies "I want this to feel like a curated home, not a launcher tool" — that maps to the couch audience more naturally than the desktop-power-user audience. Desktop users keep what they have; couch becomes the priority for new investment.

**Considered and rejected:**
- **Desktop primary, couch secondary.** The status quo. Easier to ship but cedes the audience-expansion opportunity to LaunchBox/BigBox.
- **Cabinet primary.** Would force kiosk shell ahead of guided-setup. Multi-month commitment for a smaller audience than couch.

### Decision B — Two-tier UX model (smart defaults + power-user escape hatch)

**Decision:** Every operator-facing decision in the guided-setup arc — folder layout, core selection, bindings, BIOS — ships with a smart default that handles the 80% case, and an explicit "customize / override" path for power users.

**Why:** Apple + Steam both operate this way successfully. Aligns with "guided automation, not magic" — operator sees what was decided and why, can always override. Serves both defectors (skip past defaults) AND first-timers (accept defaults, learn over time).

**Considered and rejected:**
- **Fully automatic, no override surface.** Magic vibe; risks alienating power users who want to tune.
- **Fully manual, no defaults.** Status quo for the existing wizard. Defeats the guided-setup goal.

### Decision C — Controller-navigable from day one, scope = wizard + library browse/launch

**Decision:** The guided-setup wizard is navigable with a controller from the day it ships. Same for library browsing and launching games. Per-game settings drawer, cheat editor, complex configuration screens stay mouse + keyboard until kiosk shell Phase 1 ships properly later.

**Model:** D-pad + focus rings (Steam Big Picture style). A confirms, B cancels, Y customizes, Select shows help overlay. On-screen hint bar persistent at footer.

**Why:** Couch-primary audience can't grab a keyboard mid-onboarding. Controller-nav is load-bearing for that audience. Scoping to wizard + browse/launch lets us ship without absorbing the entire kiosk Phase 1 effort.

**Considered and rejected:**
- **Mouse + keyboard only; add controller polish later.** Half-experience for the priority audience.
- **Whole desktop UI controller-navigable.** Effectively absorbs kiosk Phase 1; multi-month addition to the scope.
- **Stick-driven cursor instead of DPad focus.** Slower for grid navigation; less aligned with BigBox-class UX conventions. Cursor mode parked for the kiosk shell library browser later.

### Decision D — Curated decision tree for core selection (not benchmarking)

**Decision:** Per-system core recommendations driven by a hardcoded `(system, power_tier) → core` table. CPU power detected via the `sysinfo` crate (brand + base clock + core count) and bucketed into three tiers: low, mid, high. Operator can override the auto-detected tier in Settings.

**Why:** Predictable, maintainable, low-magic. The table encodes our per-system knowledge directly rather than hoping a benchmark generalizes. Operators see "we picked Beetle PSX HW because we detected your system as high-tier" — visible automation, not silent decisions.

**Considered and rejected:**
- **Detected + benchmarked.** sysinfo reports the hardware, one-time benchmark frame at first launch buckets the operator into a power tier. More accurate, more failure modes, more complex. Benchmark results can mislead on systems with thermal throttling, integrated GPUs, etc.
- **Operator picks core per-system manually.** Status quo. Loses the "smart default" win that defines the guided-setup pitch.

### Decision E — Folder model: read-in-place default, canonical layout opt-in

**Decision:** OA continues to read ROMs from wherever they live by default. The wizard OFFERS a canonical `<root>/<system>/` layout; operator can have OA copy or move ROMs there. Default state of the "organize my ROMs" toggle is OFF (opt-in only). Default root is mode-aware:
- Portable mode → `<exe_dir>\roms\<system>\`
- AppData mode → `~\Documents\OverlookedArcade\roms\<system>\`
- Operator can pick a different location; persisted to settings.

**Why:** ROM collections are personal. Many operators have years of organization invested in their existing layout (No-Intro, TOSEC, ROMVault-managed). Forcing a canonical layout would alienate that audience. Auto-moving ROMs is also expensive (500GB+ collections take hours) and risky (CD `.cue + .bin` siblings, archive inner-path encoding, watcher conflicts during move). Opt-in only.

**Considered and rejected:**
- **Force canonical layout.** Alienates existing-collection operators. Risk of file-move corruption for niche edge cases (archives, CD multi-file sets).
- **No canonical layout offer at all.** Loses a real win for new users without a system — they benefit from OA suggesting a clean layout out of the gate.

### Decision F — Voice / tone: warm + curator/enthusiast

**Decision:** OA's copy voice is warm-but-not-saccharine, knowledgeable-but-not-condescending. Acknowledges the operator as someone who knows what they're doing without assuming they remember every system's quirks. Treats emulation as a cultural pursuit, not a task to grind through.

**Why:** The Jaguar gold theme, per-system care, and "premium home for overlooked systems" positioning all imply this voice already. The copy should match. Plain/utilitarian feels generic; saccharine feels condescending; wry-only feels exclusive. Warm-plus-curator is the middle that serves both first-timers (welcoming) and defectors (respectful).

**Considered and rejected:**
- **Plain / utilitarian (Apple HIG style).** Generic, doesn't match the per-system theming personality.
- **Wry / understated only.** Risks excluding first-time-emulation users who don't get the joke.

### Decision G — Theme ecosystem: WAIT (deferred to PARKING_LOT)

**Decision:** Do NOT build the Rhai-scripted theme ecosystem currently designed in `docs/features/kiosk-shell/KIOSK_PLAN.md` Phase 2. Per-system CSS hardcoded in `frontend/src/themes/registry.ts` remains the only theming surface for now. Reconsider if/when (a) the kiosk shell launches AND (b) there's clear community pull (multiple operators independently asking how to share themes).

**Why:** Classic dead-ecosystem trap. Theme ecosystems require simultaneous critical mass on demand (users wanting themes) AND supply (theme authors producing them). OA's current user count meets neither. Author would maintain the entire ecosystem alone with no community contributions, locking in maintenance cost without product value.

Full reasoning logged in `docs/PARKING_LOT.md` (2026-05-25 entry).

### Decision H — License pivot: GPL-2.0 → permissive (timing deferred)

**Decision:** Eventually move the OA shell from GPL-2.0 to MIT or Apache 2.0. Timing condition: pivot when (a) the dynamic-load architecture pivot is complete (✅ vendored static crates retired per 2026-05-16 entry) AND (b) the installer ships only our own DLL builds of any forked cores (in-progress; we currently use community-built nightlies for most cores).

**Why:** Mission-aligned with "gift to the retro community" — permissive licensing encourages contributions + forks + downstream ecosystem use. GPL cores stay GPL inside their .dll regardless; the shell license is independent post-pivot. Commercial-actor copying is not a meaningful risk — they'll copy regardless of license; OA's defense is vision + execution speed + non-commercial intent, not legal walls.

**Supersedes:** the 2026-05-15 "License: GPLv2 binary-wide" decision, **once timing conditions are met**. Until then, the original decision stands. When the pivot ships, file a new entry that names the chosen permissive license (MIT vs Apache 2.0) and updates `workspace.package.license` in the root `Cargo.toml`.

**Considered and rejected:**
- **Stay GPL-2.0 forever.** Keeps the original alignment with copyleft cores but unnecessarily limits downstream use post-pivot.
- **Pivot to permissive immediately.** Premature; the "ship our own DLLs" condition isn't met yet, and the binary-wide GPL story is still technically accurate while community-built core nightlies are the default.

### Adjacent features mentioned but not in scope

These came up in the advisor session as future-feature ideas. Catalogued in `docs/PLANS/guided-setup.md` §15 for follow-up arcs after guided-setup ships:

- **System Mode** — immersive per-system experience (boot animation, ambient music, UI transforms per system).
- **Game Context System** — rich hover info ("if you like X try this", fun facts, dev/year, known issues). ChatGPT's "highest ROI" pick at 2-3 weeks of work.
- **Play History Intelligence** — original-to-the-space feature. Track plays, surface "you seem to like SNES RPGs", dormant favorites.
- **RetroAchievements integration** — close one of two big RetroArch gaps. Pending strategic decision after guided-setup ships.
- **Netplay** — close the other RetroArch gap. Multi-month effort with real risk of shipping a worse version for years. Pending strategic decision.

None decided in scope yet. Operator opted to commit to guided-setup as the next major arc; the rest stay future-work.

---

## 2026-05-26 — Strategic locks for per-system custom UI (second major arc)

**Context:** Operator + Claude planned the per-system custom UI feature in a follow-up session to the guided-setup planning. ChatGPT advisor had pitched "every system feels alive" as the killer differentiator vs LaunchBox / BigBox / RetroArch / Pegasus / ES-DE. Operator opted to plan it in depth; full implementation plan landed at [`docs/PLANS/per-system-ui.md`](PLANS/per-system-ui.md).

The strategic locks here capture decisions made before any code is written. Some entries clarify or supersede earlier locks (specifically the kiosk shell relationship and the theme ecosystem boundary).

### Decision I — Per-system custom UI is the DEFAULT OA experience

**Decision:** Per-system custom UI (per-system audio, boot animations, navigation behavior, layout structure, tile flourishes) ships as the default user experience for desktop normal OA — not as a power-user feature, not opt-in. First-launch defaults the "Per-system experiences" toggle to ON. Most operators see this without configuring anything.

**Why:** OA's positioning + per-system theming (since day 1) + curator-audience focus all point at this being the IDENTITY of the product, not a feature on top of it. Treating it as opt-in would bury the differentiator under defaults. ChatGPT's framing — "the only frontend where every system feels alive" — only works if every user actually experiences it.

**Considered and rejected:**
- **Opt-in feature.** Bury the differentiator; defeats the positioning.
- **Power-user only.** Same problem; the audience for whom this matters most is the casual curator, not the configurator.

### Decision J — Three-mode user separation: Themed / No theme / Kiosk

**Decision:** Three top-level user paths, picked via Settings (and eventually kiosk shell's own mode):

1. **Themed** (default): per-system custom UI as designed in this plan.
2. **No theme**: uniform plain library, no per-system flair. Single toggle in Settings → Display.
3. **Kiosk** (future, separate plan): full themable mode with built-in per-system experiences as starting defaults, plus optional theme authoring via the kiosk Theme Studio.

**Why:** Three distinct audiences with three distinct needs. Themed for the curator default; No theme for the operator who wants OA's library + emulation without flair (accessibility, personal taste, low-end hardware); Kiosk for cabinet builders + power users who want full theming control. Trying to serve all three with one mode forces compromises that please none.

**Considered and rejected:**
- **Two modes (Themed + Kiosk).** Loses the operators who want a plain library without going full kiosk.
- **One mode with many sub-toggles.** Configuration sprawl; default behavior becomes ambiguous; first-launch confusion.

### Decision K — No theme editor on desktop normal — period

**Decision:** Theme editing (TOML / Rhai / Theme Studio / `.oatheme` archives / federated index) is a kiosk-shell-exclusive feature. Desktop normal OA never gets a theme editor. Desktop operators get the choice of: Themed (built-in per-system experiences), No theme (uniform plain library), or eventually Kiosk (full editor with built-ins as defaults).

**Why:** Confirms and concretizes the 2026-05-25 Decision G theme-ecosystem WAIT lock. Theme authoring is a separate audience need from "I just want OA to look great per-system." Cleaving them keeps the desktop experience predictable and the kiosk experience flexible. Avoids the maintenance trap of supporting power-user themes on the casual-user surface.

**Considered and rejected:**
- **Limited theme editing on desktop.** Half-measure that satisfies nobody. Adds complexity without unlocking the kiosk audience.
- **Defer until kiosk shell ships.** Already the plan; this lock just makes it explicit.

### Decision L — Hybrid architecture: SystemUIConfig DSL + per-system component escape hatch

**Decision:** Most systems use a config-driven `SystemUIConfig` interface (enum-based DSL: layout, navigation, emphasis, background, audioProfile, interactionStyle, tileShape, transitionTiming, buttonLabels). A handful of "signature" systems (Vectrex confirmed; others TBD) override the config-driven library view with a custom Solid component when they need to render something the DSL can't express (e.g. Vectrex's vector-stroke library tiles).

**Why:** Config-driven keeps 95% of systems trivially extensible (adding a system = filling out the config). Component escape hatch lets the 5% of systems that need genuinely unique rendering escape the DSL's limits without forcing every system through component-level code. Pure config would constrain creativity (Vectrex can't be expressed in enums); pure components would inflate the per-system maintenance burden 10×.

**Considered and rejected:**
- **Pure config-driven.** Can't express Vectrex; loses the signature feature.
- **Pure per-system components.** Every system needs custom code; maintenance cost compounds with each system added.

### Decision M — Pilot order: Game Boy → NES → Vectrex

**Decision:** Stage 1 ships full showcase configurations for three pilot systems in this order: Game Boy first (smallest scope, "soft / minimal / personal" end of spectrum), NES second (validates pattern at medium complexity, "classic / bright / instant"), Vectrex third (escape-hatch escalation, "vector-rendered / signature").

**Why:** Game Boy first lets us nail the minimal case before scaling up; if the project derails after Pilot 1, GB alone is shippable. NES second validates the config-driven pattern at medium complexity without yet needing the escape hatch. Vectrex third because it needs `customComponent` — proving config-driven on the prior two first means we know when we're escalating deliberately, not by accident.

**Considered and rejected:**
- **Jaguar first** (max aggressive). High-risk start; if the "loud" theming feels wrong, we'd lose Stage 1 momentum debugging it.
- **Vectrex first** (max signature). Forces the escape hatch on day 1 before the config pattern exists; harder to know what's "config DSL" vs "Vectrex special."

### Decision N — Multi-source audio asset strategy

**Decision:** Per-system SFX combines three sources: CC0/royalty-free pack (baseline universal sounds for all 37 non-pilot systems), original recordings or commissioned audio (signature character for the 3 pilots), AI-generated procedural sounds (Vectrex synthesized vector-blips; future systems where hardware sound character is procedurally easier than recordable).

**Explicitly excluded:** community-sourced sound packs on the desktop normal version. Community submission stays parked alongside the theme ecosystem (Decision G).

**Why:** No single source produces good results across the spectrum. CC0 is cheap but generic; originals are best but expensive per pilot; AI is good for synthesized character but feels canned for natural sounds. Combining gives the right tool for each system.

**Considered and rejected:**
- **CC0 only.** Sounds generic; pilots can't communicate signature character.
- **Originals only.** ~1-2 hours of recording per system × 40 systems = months of content production.
- **Community sourcing.** Mission-aligned but slow + uneven + adds curation burden; defers to kiosk-shell theme substrate maturity.

### Decision O — Boot animation policy: medium length, every entry, always skippable

**Decision:** Boot animation when entering a system runs ~1-1.5 seconds, plays on every system entry (including switching back to a recently-left system), and is always skippable via any nav input mid-animation. Reduced-motion preferences (CSS media query or dedicated accessibility toggle) downgrade to a 200ms fade. Two related Settings toggles: master "Per-system experiences" + sub-toggle "Boot animations" for operators who want themes but not transitions.

**Why:** Medium length feels deliberate without becoming annoying on repeated entry. Every-entry frequency keeps the experience predictable; once-per-session would feel arbitrary when operators switch back. Always-skippable respects operators who know what they're doing. Reduced-motion honoring is an accessibility floor.

**Considered and rejected:**
- **Long cinematic boot (3-5s, BigBox-style).** Annoying on repeat entry; cinematic effect doesn't justify the time cost for a library-launcher use case.
- **Once-per-session.** Feels arbitrary when switching back to a recently-played system.
- **Never skippable.** Defensible artistically; user-hostile in practice.

### Decision P — Navigation pattern: BOTH (flat grid + explicit system-entry)

**Decision:** Two coexisting library navigation paths. (1) **Flat grid** (existing behavior + enhanced) — sidebar can filter by system; tile-focus triggers light per-system retheme but no boot animation. (2) **Explicit system-entry** — operator deliberately "enters" a system via Sidebar manufacturer view or future system selector; boot animation plays; operator lands in full per-system themed library.

Both modes coexist; operator chooses path moment-to-moment. The per-system theming is more pronounced in the entered state.

**Why:** Flat grid is the muscle-memory pattern; removing it would break existing operators' flow. System-entry is where the per-system experience truly lands. Both is the kindest answer — existing flow preserved, new experience accessible deliberately.

**Considered and rejected:**
- **Flat grid only** (theming follows focused tile). Invisible to existing users but loses the boot-animation + full-immersion moment.
- **System-entry only.** Forces a nav change on every operator; breaks the flat-library muscle memory.

### Decision Q — Kiosk shell relationship — clarified, not changed

**Decision:** The kiosk shell (planned at `docs/features/kiosk-shell/KIOSK_PLAN.md`) becomes the **theme editor + power-user mode** that consumes the built-in per-system experiences as its starting defaults. Kiosk shell still ships its Phase 2 theme substrate (TOML / Rhai / Theme Studio / `.oatheme` archive) but as power-user authoring ON TOP of the built-ins, not as the source of all theming.

**Updated kiosk shell positioning:**
- Operator in kiosk mode can: (a) use built-in per-system experiences as-is, (b) author new themes via the Theme Studio, (c) use NO theme (plain kiosk shell), or (d) import community themes (eventually).
- Desktop normal users never see the kiosk theme editor.

**Why:** Pre-this-plan, kiosk shell Phase 2 was framed as "the substrate that provides all theming." After this plan, that framing was wrong — the built-in per-system experiences in THIS plan provide the default theming, and kiosk shell becomes the editor for users who want to deviate. Cleaner separation; respects the audience cleavage between desktop curator and kiosk cabinet builder.

**Supersedes:** the implicit framing in `docs/features/kiosk-shell/KIOSK_PLAN.md` Phase 2 that the theme substrate is the only theming source. Phase 2 still ships as designed (Rhai, TOML, Theme Studio, `.oatheme`), but the kiosk shell consumes built-in per-system experiences as default starting points rather than starting from blank.

**Considered and rejected:**
- **Move all theming to kiosk shell.** Locks desktop users out of per-system experience; would have made desktop OA strictly worse.
- **Duplicate the work — per-system experiences in desktop AND in kiosk.** Wasted effort; both surfaces benefit from sharing the built-in catalogue.

### Decision R — Staged ship: each stage fully working, next stage builds on top

**Decision:** Per-system custom UI ships in three stages (polish layer → behavior layer → experience layer). Each stage is a complete, shippable product. Later stages add capability without rebuilding prior stages — `SystemUIConfig` is additive across stages; pilots tuned in Stage 1 deepen in Stage 2; Stage 3 in-game theming layers on top.

**Why:** Multi-month features that ship in one big chunk risk slipping forever. Staged ship lets us validate the architecture + audience response at Stage 1 before committing to Stages 2-3. Operator can change priority between stages if the early stage reveals something unexpected.

**Considered and rejected:**
- **One big ship.** ~15-23 weeks before any user feedback; risks building the wrong thing.
- **Single stage scope.** Loses the deeper "behavior" and "experience" wins; pilot would be 80% of the experience but stop at polish.

### Additional decisions captured implicitly

- **Coverage at Stage 1:** baseline `SystemUIConfig` for ALL ~40 wired systems + showcase tier for 3 pilots. No system stays at the pre-this-plan visual default once Stage 1 ships; every system gets at least the baseline themed treatment.
- **Per-system asset budget:** ≤500 KB sounds + ≤2 MB visuals per system. Total addition ~100 MB worst case across 40 systems. Assets bundled with the installer; no first-launch download.
- **Audio routing:** new per-system SFX flows through the existing 4-bus mixer (shipped 2026-05-24 in media-taxonomy) on the `ui-sounds` bus. No new audio infrastructure needed.

---

## 2026-05-26 — Strategic locks for the Game Info Panel (third major arc)

**Context:** Operator + Claude planned the Game Info Panel feature as a tighter alternative to ChatGPT's "Game Context System" pitch. Reframed from editorial-and-recommendations to structured-factual-reference. Full implementation plan landed at [`docs/PLANS/game-info-panel.md`](PLANS/game-info-panel.md).

The strategic locks here capture decisions made before any code is written. The scope is deliberately tight for v1 with the full distribution + scraper + contribution architecture fully designed but deferred to v2.

### Decision S — Reframe: Game Info PANEL, not Context SYSTEM

**Decision:** The feature surfaces **structured factual reference data per game** — date, publisher, region, version, player count, controls supported, known bugs, best-emulator recommendations, and an operator-editable short summary. Explicitly NOT editorial commentary, NOT "fun facts" or "why it's important," and NOT a recommendations engine ("if you like X try this").

**Why:** Operator's actual use case is "should I launch this specific game right now?" — answered by version + region + works-with-my-controller + known-issues + best-core. Editorial content has unclear sourcing, IP concerns, and feels like enthusiast-blog territory rather than launcher-tool territory. Recommendations belong in a future Play History Intelligence feature (the other ChatGPT pitch); they need the play-data layer this plan doesn't touch.

**Considered and rejected:**
- **Full ChatGPT framing** (importance + dev/year + recommendations + fun facts + known issues). Too sprawling; mixes editorial and reference; "fun facts" needs content sourcing that doesn't have a clean answer.
- **Pure factual stripped of all narrative.** Operator wants a short-summary field for personal use; making it operator-editable + default-empty solves the IP question without giving up the slot.

### Decision T — Field schema locked (9 fields, mostly already in OA)

**Decision:** Nine structured fields:
1. **Date** (release year) — sourced from existing metadata sync
2. **Publisher** — sourced from existing metadata sync
3. **Region** — sourced from existing metadata sync + DAT region tags
4. **Version** — sourced from existing metadata sync + DAT rev tags
5. **Player count** — sourced from existing metadata sync
6. **Short summary** — operator-editable, default empty in v1
7. **Controls supported** — empty by default v1; hand-curated v2
8. **Game bugs** — migrated from `KNOWN_GAME_BUGS.md` files for v1
9. **Best emulator per game** — migrated from KNOWN_GAME_BUGS where mentioned for v1; hand-curated v2

**Why:** Five fields already exist in OA via metadata sync — v1 mostly surfaces them. Four fields need new infrastructure (structured per-game data format) but minimal new CONTENT (KNOWN_GAME_BUGS migration produces meaningful seed data). Tight v1 scope (~3-4 weeks) by deliberately scoping to what's already mostly there.

**Considered and rejected:**
- **Adding "genre" / "ESRB rating" / "approximate playtime"** to v1. Defer — easy to add later if requested; not load-bearing.
- **Dropping "short summary"** entirely. Operator-editable + default-empty makes it cheap and operator-valuable; keep.

### Decision U — Three-layer data architecture (scraper / hand-curated / operator local)

**Decision:** Game info data lives in three layers at runtime:
1. **Scraper output** (deferred to v2) — auto-generated objective fields from libretro-database DATs + future scraped sources
2. **Hand-curated content** (deferred to v2) — project maintainer + community contributions; narrative summaries, refined best-emulator notes
3. **Operator local overrides** (v1) — per-install SQLite table; never leaves the operator's machine unless they explicitly Submit correction

**v1 ships layers 1 + 3.** Layer 1 in v1 is "what OA's existing metadata sync produces" + "KNOWN_GAME_BUGS migration result" — no separate scraper running. Layer 3 is the operator's local SQLite override table. Layer 2 ships in v2 alongside the data repo.

**Why:** The three-layer model anticipates the v2 distribution architecture without forcing it into v1. Field-typed precedence (Decision W) defines how the three layers merge cleanly.

**Considered and rejected:**
- **Two layers (project + operator).** Loses the distinction between scraper-managed and curator-managed fields; makes the v2 scraper architecture harder to retrofit.
- **One layer (everyone edits the same files).** Doesn't allow operator local edits without polluting upstream.

### Decision V — Data format: YAML front-matter in per-system markdown

**Decision:** Structured per-game entries live as YAML front-matter blocks separated by `---` in per-system markdown files. v1 location: `docs/cores/<id>/games-info.md` in the main OA repo. v2 location: separate `overlooked-arcade-game-info` data repo (move announced; not in v1).

**Why:** YAML is human-readable for hand-edits; front-matter blocks parse cleanly into a structured index at OA startup; markdown wrapper keeps room for prose context per-system (system-wide notes that aren't per-game). Most retro-frontend data projects use similar formats; lower contribution bar than custom JSON/SQLite-only formats.

**Considered and rejected:**
- **Pure JSON files per system.** Less human-friendly for hand-edits; no room for prose; harder for non-technical contributors.
- **SQLite database checked into the repo.** Fast to query but invisible to PR reviewers; opaque diffs; high friction for contributions.
- **Per-game files in per-system folders.** Cleaner PR diffs (one file per game changed) but explodes the file count (40 systems × hundreds of games = thousands of files). Defer; revisit if PR-diff noise becomes a problem.

### Decision W — Field-typed precedence for conflict resolution

**Decision:** When the three layers disagree on a field's value, precedence depends on the field's nature:

- **Always local wins** (narrative + operator preferences): short summary, controls supported, best emulator, operator-added bugs. Operator's words and discovered preferences are sacred.
- **Always project / scraper wins** (objective facts): date, publisher, region, version, player count. These are read-only in the UI; if a scraper update finds a corrected publisher name, it overrides any stale local data.
- **Three-way merge** (currently no fields, reserved for future).

**Why:** Different fields have different ownership semantics. One-size-fits-all (always-local OR always-master) loses information; per-field precedence captures real-world ownership distinction between "facts the project curates" and "preferences the operator owns."

**Considered and rejected:**
- **Always local wins.** Loses scraper updates to corrected facts. Operator might have stale local data hiding important corrections.
- **Always master wins on next sync.** Operator's effort on local edits gets clobbered; terrible UX.
- **Three-way merge with operator approval per conflict.** Too disruptive; operator gets prompts on every sync. Reserved for future cases where neither precedence rule fits.

### Decision X — v1 tight scope: supplied DATs only, no scraper, no community pipeline

**Decision:** v1 ships:
- Data model + parser (YAML front-matter from per-system markdown)
- KNOWN_GAME_BUGS migration into structured entries (one-time pass)
- Tile-hover compact card + long-press / `i` full panel + tile badge
- Operator local edits via SQLite override table
- Inline "Apply best emulator" + "Apply controls" actions wiring to existing `GameOverrides`
- "Submit correction" surface stubbed for v1 (clipboard copy + "coming soon" toast)

v1 does NOT ship:
- Scraper running anywhere
- Separate data repo
- Daily auto-sync mechanism
- Wikipedia / IGDB / TheGamesDB / ScreenScraper integration
- GitHub Issue → auto-PR community contribution flow

**Why:** Full distribution + scraper + contribution stack would dominate v1 implementation (estimated ~3-5 weeks ON TOP of v1's ~3-4 week scope). Better to ship the UI + data model with what we already have, prove the value, then layer distribution on later when actual operator demand justifies the infrastructure.

**Considered and rejected:**
- **Full v1 with scraper + data repo + contribution flow.** Doubles the v1 timeline; risks shipping nothing for ~2 months instead of something useful in ~3-4 weeks.
- **Skip v1 entirely and only ship v2.** Loses the "ship something useful fast" win.

### Decision Y — v2 architecture fully designed, deferred

**Decision:** All v2 components are designed (and recorded in `docs/PLANS/game-info-panel.md` §11) but not in v1 scope:
- Scheduled scraper running on GitHub Actions on the data repo
- Separate `overlooked-arcade-game-info` GitHub data repo
- Daily auto-sync with manual "Check now" button + off toggle
- GitHub Issue → auto-PR community contribution flow with maintainer review
- Field-source tagging evolution (PR-only initially → per-field auto-merge for tagged scraper fields)
- Wikipedia / TheGamesDB / ScreenScraper richer-source integration paths

**Why:** Locking the architecture now prevents relitigation when v2 starts. Future-Claude or future-you reads the plan, understands what was decided and why, and can pick up implementation without redesign.

**Considered and rejected:**
- **Leave v2 as an open design exercise for later.** Risks expensive redesign work later; current operator + Claude planning context fades.

### Decision Z — Scheduling: Game Info Panel v1 as polish for Per-System UI Stage 1

**Decision:** Game Info Panel v1 ships immediately after Per-System UI Stage 1 in the strict-sequence portion of the pipeline. Updated pipeline:

```
Phase 0 (controller-nav, ~2-3w)
   → Per-System UI Stage 1 (polish layer, ~5-7w)
   → Game Info Panel v1 (per-game depth, ~3-4w)
   → [INFLECTION POINT — ~10-14 weeks from green-light]
   → interleave Guided Setup Track + Per-System Stage 2+3 + Game Info Panel v2
```

**Why:** Per-System UI Stage 1 makes OA's library feel alive. Game Info Panel v1 is the practical complement — once every system has its own personality, the natural next ask is "what is THIS specific game about?" Shipping them adjacent lands the operator's first complete-feeling experience: themed library + per-game depth.

Also: shared infrastructure makes adjacency cheaper. The structured per-game data Game Info Panel defines is consumed by Guided Setup Phase 2D (auto-apply per-game core overrides) and Per-System UI Stage 3 (`metadataPriority` field). Doing Game Info Panel v1 right after Per-System Stage 1 means Stage 2 and Stage 3 can lean on the data already populated.

**Updated inflection point estimate:** ~10-14 weeks from green-light (was ~7-10 weeks before this scheduling decision). Larger inflection but richer inflection — operator gets identity + depth at the same milestone.

**Considered and rejected:**
- **Game Info Panel v1 first, before Per-System UI Stage 1.** Loses the "identity moment" first-impression. Per-system personality is the killer differentiator; lead with that.
- **Game Info Panel v1 in the post-inflection interleave bucket.** Misses the benefit of having structured per-game data populated before Guided Setup Phase 2D and Per-System Stage 3 consume it. Adjacency saves rework.
- **Skip Game Info Panel entirely until v2 distribution is ready.** Loses the ~3-4 week v1 win; operator waits months for any per-game depth.

### Additional decisions captured implicitly

- **v1 effort: ~3-4 weeks.** Significantly tighter than the full pitch because of deliberate scope deferral.
- **Migration approach for KNOWN_GAME_BUGS:** scripted parser pass that reads the existing free-form markdown and emits structured front-matter entries. Imperfect but better than abandoning the existing knowledge. One-time at v1 build; subsequent edits happen in the new structured format.
- **Action buttons in panel — "Apply best emulator" + "Apply controls":** both write to existing `GameOverrides` table (`libretro_core` + `libretro_device_port1..4`). No new override fields needed; just consumption.
- **Tile badge styling:** subtle ⚠ N for known issues; single small icon for operator-has-local-edits. Don't crowd the tile.
- **"Submit correction" v1 stub behavior:** clipboard copy of operator's local edits as JSON + informational toast ("Your changes are copied. We're not yet set up to receive submissions automatically — coming soon"). Makes the UI surface visible without committing to v2 backend infrastructure.

---

## 2026-06-03 — Reversal: OA supports external standalone emulators via a Launcher abstraction

**Decision:** Overlooked Arcade will support **both libretro cores AND external
standalone emulators** via a `Launcher` trait abstraction in `oa-core`. Two
trait impls: today's `LibretroLauncher` (wrapping the existing `LibretroCore`)
and a new `ExternalProcessLauncher` (spawns a configured emulator binary via
`tokio::process::Command`). Per-emulator profile YAMLs live in
`config/emulators/<id>.yaml` (mirroring the per-system descriptor pattern from
the 2026-06-02 consolidation arc).

**This supersedes** the 2026-05-16 "Architecture pivot: libretro frontend"
decision's framing as "libretro is the only FFI / launch boundary."
That decision's *core* — libretro `.dll` loading via `libloading`, dynamic
core swap, per-system + per-game core selection — all stands. What's
reversed is the *exclusivity* claim. OA is now a frontend for retro emulation,
period; libretro is the primary path, external standalone emulators are the
secondary path.

**Phase C of the new arc (see `docs/PLANS/virtual-library-and-launcher-arc.md`)
implements the trait refactor.** Phase D ships the install pipeline. v1 pilot
emulator set: Cemu (Wii U), RPCS3 (PS3), Lime3DS (3DS).

**Why (reversed rationale):**

1. **Vendor coverage gap.** Wii U, PS3, 3DS, Switch, recent PS2-via-PCSX2, and
   modern Mac emulation targets all lack production-grade libretro paths today.
   The libretro-only stance left those systems either unreachable from OA or
   reachable only via a separate launcher app — which contradicts the "premium
   frontend for retro emulation" pitch. The operator explicitly named this as
   "needed for the future to not bite us" during the 2026-06-03 planning round.
2. **Plugin-style install profile shape is constrained, not open-ended.** The
   2026-06-02 PARKING_LOT entry rejected a generic "Plugin / Extension API"
   for good reasons (security, version-compat, SDK contract burden). The
   launcher install pipeline is NOT a generic plugin API — it's a closed set
   of operator-editable per-emulator profile YAMLs with constrained semantics
   (download URL, launch args template, install location, capability flags).
   Reusing the per-system-descriptor pattern's discipline avoids the open-SDK
   trap.
3. **Variant model + per-game settings need launcher-agnostic shape from day
   one.** The new arc's Phase A → E → B → C ordering deliberately puts the
   launcher refactor (Phase C) before the variant tree + Casual/Preservation
   UX (Phase B) crystallizes on libretro-only assumptions. Postponing the
   reversal risks UX work that doesn't generalize.
4. **The 2026-05-16 trade-off "Day-one install requires a cores/ folder" no
   longer holds for all systems.** Wii U operators with a Cemu install
   should be able to drop OA in and launch — no `.dll` curation step. The
   install pipeline (Phase D) handles emulator delivery automatically for
   profiles with clean redistribution stance.

**Legal posture (unchanged from 2026-06-03 lock):**

- OA downloads + sets up emulator binaries where legally clean (each profile
  points at the emulator's official release endpoint — GitHub Releases for
  Cemu / RPCS3 / Lime3DS / etc.).
- OA **never** downloads or installs ROMs or BIOS files. "Emulation is legal;
  redistribution of copyrighted ROMs and BIOS is not unless the user owns
  them, and OA cannot guarantee that."
- Emulators with ambiguous redistribution stance ship as profile-without-fetch
  (operator points OA at an existing install rather than triggering an
  auto-download).

**Considered and rejected:**

- **Stay libretro-only.** Cleanest architecture; leaves Wii U / PS3 / 3DS /
  Switch operators stranded. Rejected by the operator on 2026-06-03 with the
  framing "OA will have to take on the role of a front end for other emulators
  eventually anyway — plan for it now so it doesn't bite us at the end."
- **Generic plugin / SDK API.** The 2026-06-02 PARKING_LOT entry rejected this
  for the right reasons (version-compat burden, security surface, contract
  ossification). The launcher abstraction is **not** a plugin API — it's a
  closed set of trait impls + operator-editable per-emulator profile YAMLs.
  Different shape, narrower scope.
- **Defer external-emulator support until after Phase E (schema promotion).**
  Risks crystallizing the variant model + per-game settings shape on
  libretro-only assumptions and forcing a second refactor later. The phase
  order locks in: A (identification) → E (schema) → B (UX) → **C (launcher)**
  → D (install) → F (Vault) → G (crate split). The launcher refactor lands
  before the UX layer hardens.

**CLAUDE.md update accompanies this decision:** the "libretro is the only FFI
boundary" line in the "Architectural rules" section softens to "libretro is
the primary launcher boundary; external standalone emulators reach the shell
via the `Launcher` trait + `ExternalProcessLauncher` impl."

---

## 2026-06-03 — Reversal-partial: 2026-06-02 Plugin / Extension API parking-lot entry

**Decision:** Un-park the 2026-06-02 PARKING_LOT entry on "Plugin / Extension
API" — **partially**. The launcher abstraction + external-emulator install
pipeline (recorded in the new DECISIONS entry above) DOES surface
operator-editable per-emulator profile YAMLs that look superficially like
"plugins" from the operator's POV. But the shape is constrained: closed set
of trait impls, no third-party Rust-side extensibility, no SDK contract, no
dynamic loading of arbitrary plugin code.

**What stays parked:** the original parking-lot entry's rejection of a generic
third-party Rust SDK for custom views / custom systems / arbitrary plugin
code stays in force. That kind of plugin API still carries the security +
version-compat + SDK-ossification costs the 2026-06-02 entry called out.

**What's un-parked:** the narrow case of "operator points OA at additional
emulator profiles." This is a configuration surface, not a code surface — same
shape as `config/systems/<id>/system.yaml` editability.

**Cross-ref:** [2026-06-03 Launcher abstraction reversal entry](#2026-06-03--reversal-oa-supports-external-standalone-emulators-via-a-launcher-abstraction);
[docs/PLANS/virtual-library-and-launcher-arc.md](PLANS/virtual-library-and-launcher-arc.md).



---

## 2026-06-03 — Pivot: filename-fuzzy primary, per-track SHA-1 experimental

**Decision:** Per-track SHA-1 disc identification (Phase A1 Sub-phases 1–3 of the virtual library + launcher arc) is moved from primary to opt-in behind a `LibraryPrefs.disc_track_experimental_enabled` flag (default OFF). Filename-fuzzy matching against `rom_hashes_tracks.game_name` becomes the new primary disc-shape identification path.

**Why:** Operator playtest 2026-06-03 measured **0% per-track match rate** on the real library. Two stacking architectural limitations made the per-track architecture fundamentally inert on the operator's actual data:

1. **CHD round-trips lose bytes.** `chdman extractcd` on Dreamcast GD-ROM "4 Wheel Thunder (USA)" Track 18 produced 338,704,464 bytes vs redump's cataloged 339,233,664 — exactly **225 sectors short** (225 × 2352 = 529,200 byte delta). The diff is a fixed convention difference in how chdman handles the SD/HD area boundary that no amount of fixing our SHA-1 path catches; redump's per-track SHA-1 was computed over DiscImageCreator's source .bin, which CHD does not preserve byte-for-byte. Likely consistent across all GD-ROM CHDs; possibly different but similarly fatal offsets for other system CHDs (untested).
2. **Archived disc images are skipped.** Sub-phase 3 deferred per-track-through-archive to v2 because multi-GB streaming through `oa-shell`'s archive reader OOMs. The operator's PSX library is 100% .zip-archived (common to save space), so per-track was inert there from day one.

After 1 + 2: per-track works only for **raw, unarchived, DiscImageCreator-shape .cue+.bin** — a niche of operators. The plan's expected use case (broad identification across normal libraries) doesn't materialize.

**What replaces it:** filename-fuzzy match. Build an in-memory `HashMap<normalized_filename_key, RomTrackRow>` per resolve call from `rom_hashes_tracks` distinct game_names; compare operator filenames via `disc_filename_fuzzy_key` (strip extension, lowercase, separator punctuation → space, preserve regional brackets). On hit: stamp canonical title + serial + canonical's first-track SHA-1 as marker. Works on any container shape, any storage shape — only the filename matters.

**What we considered and rejected:**

- **Fix the CHD/redump byte offset.** Requires hand-investigating chdman's GD-ROM SD/HD boundary handling for each system + reverse-engineering DiscImageCreator's source byte layout. Wouldn't fix archived-disc skip. Multi-week effort for diminishing returns.
- **Implement archive-aware per-track.** Streaming a 700 MB .bin through `oa-shell`'s archive reader requires either loading the whole file into memory (OOMs at PS2 scale) or a streaming archive crate that supports seek-by-byte-range. Doable but multi-week, and doesn't address the CHD case.
- **Match against chdman's own embedded per-track `data_sha1` field** (added MAME 2023). Would require a public database catalogging those values; redump doesn't publish them, and no community database exists. Would have to fork a hash-catalog effort, which is well outside our scope.
- **Drop disc identification entirely.** Keep peek_disc_id at ~12% Dreamcast / 0% archived PSX hit rates as the only path. Rejected as worse than what fuzzy delivers.

**What we keep from Sub-phases 1+2+3:**

- ✅ Schema v18→v19 (`rom_hashes_tracks`, `game_disc_tracks`, `disc_sets`) — used by fuzzy to source canonical titles, and necessary for opt-in per-track.
- ✅ `disc_track_hash` engine — correct implementation, just gated.
- ✅ Sync flow + `JobKind::DiscTrackHash` + `LibraryPrefs.disc_track_strictness` — functional behind the flag.
- ⚠️ Sub-phase 4 (multi-disc disc-set wiring on `games.disc_set_id`) — deferred. The fuzzy path's canonical names carry the "(Disc N)" suffix; grouping moves to display-time rather than data-model-time. Re-evaluate once fuzzy hit-rate is measured operator-side.

**Cross-ref:** [docs/PLANS/disc-track-sha1-matching.md "Pivot 2026-06-03" section](PLANS/disc-track-sha1-matching.md); operator playtest in oa-current.log 2026-06-03 17:27–20:20.

---

## 2026-06-04 — JobScope RAII guard for background-jobs lifecycles (option B; C deferred)

**Decision:** Replace the verbose `create_job → mark_running → progress×N → mark_completed/mark_failed` boilerplate that every background-jobs consumer carried with a `JobScope` RAII guard. Drop auto-fails on dropped-without-finalize so a `?` early-return between create_job and mark_completed can no longer leak a `running` row.

**Why:** Operator-reported bug 2026-06-04 — media sync's progress bar "shows but doesn't advance and then sits there even after the work is done." Audit found three structural problems:

1. **Per-consumer boilerplate fragility.** Every consumer manually wired `if let (Some(reg), Some(id)) = (registry_state.as_ref(), registry_job_id) { ... }` at every progress tick and every completion path. A `?` between create_job and the final mark_completed silently left the row in `running` until next launch's `promote_running_rows_to_interrupted` sweep.
2. **Media-sync progress granularity.** Ticks only fired at per-repo boundaries (typically 1–3 events per multi-thousand-rom sync). The per-rom inner loop emitted `oa://library-sync` (a separate Tauri event) but never ticked the registry. Bar visibly stuck.
3. **Two parallel event streams.** `oa://library-sync` + `JobEvent::Progressed` carried the same media-sync progress through different channels; the BackgroundJobsBar reads only the latter.

**What `JobScope` does:**
- `start(reg, kind, label, ..., total) -> Self` calls `create_job` + `mark_running` atomically. Returns a no-op scope when `reg` is None or create_job fails.
- `tick(done)`, `set_total(total)` — update progress; both bypass the SQL debounce so the bar UI sees the values immediately.
- `complete()` — force-flushes done=total then `mark_completed`. Idempotent.
- `fail(error)` — `mark_failed`. Idempotent.
- `Drop` — auto-fails with `"JobScope dropped without explicit complete() or fail() — likely a `?` early-return in the consumer"` if neither was called.

**Migrated consumers (RAII path):**
- `media.rs::sync_media_for_system` (THE bug). Per-rom tick added inside `buffer_unordered` via `app.try_state::<JobRegistry>()` (the scope's `&JobRegistry` borrow can't move into the stream closure).
- `rom_hashes.rs::sync_rom_hashes_for_system` + `resolve_rom_hashes_for_system` + `auto_sync_rom_hashes_if_empty`.
- `main.rs::refresh_mame_system_info` (simple 2-state finalization).

**Not migrated (kept manual):**
- `core_installer.rs::download_core` — tri-state finalization (`mark_cancelled` vs `mark_completed` vs `mark_failed`) AND the cancel branch does per-kind cleanup (deletes `.zip.partial` + `.dll.partial` before marking cancelled). JobScope's binary `complete()` / `fail()` model would lose the cancellation distinction.
- `main.rs::start_background_scan` — tri-state finalization across a `tokio::task::spawn_blocking` boundary that clones the `JobRegistry` into the blocking task. JobScope holds `&JobRegistry` (lifetime-bound) and can't cross that boundary.
- These two stay on the manual pattern. Their hand-rolled finalization is fully exercised today and the operator has not reported issues with either.

**Considered and rejected (deferred to "(c)" if needed):**
- **`run_job_over_iter<I, F, Fut>` higher-order helper** that wraps the entire "create + iterate + complete" pattern with automatic per-item progress. Cleaner for the simple-loop consumers (hash_resolve, dat_sync) but forces every consumer into one iterator shape. `media.rs`'s repo-grouped concurrent work and `scan_service`'s rayon-based parallel classification have legitimate reasons for hand-rolled iteration; constraining them to a single iterator shape would either bloat the helper API or leave those consumers on the manual path anyway.

**Trigger criteria for adopting (c):** If progress-bar bugs keep surfacing in the consumers that JobScope already covers — i.e. the RAII finalization isn't enough and we need to push more of the wiring into a shared helper — revisit. Specifically, watch for: bars that fail to start (suggests we need a `run_job_over` style that owns the iteration too), bars that fail to tick (suggests we need a built-in periodic ticker), or bars that fail to finalize despite Drop (suggests the closure isn't dropping reliably). Until any of those land, JobScope is the right ceiling.

**Cross-ref:** `apps/oa-shell/src/job_registry.rs::JobScope` doc comment; operator playtest report 2026-06-04.
