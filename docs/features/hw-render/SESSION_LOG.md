# HW-Render Pipeline — Session Log

Each session that touches this feature appends a 3-line entry:
**Shipped / Almost / Next.** Older entries roll to
`SESSION_LOG_ARCHIVE.md` when this file passes ~150 lines.

---

## 2026-06-08 (cont. 15d) — ✅ M2 MERGED to main

- **Shipped:** operator confirmed core-switching works (last validation item)
  → merged `feat/hw-render-m2` → `main` (--no-ff), tagged `hw-render-m2-proven`.
  Bookkeeping: plan M2 milestone marked ✅; n64 ROADMAP Phase 1 → 🟨 with a
  HW-render note; ACTIVE_WORK M2 → shipped+merged, M3/M4 flagged future stretch;
  NES audio clipping/clicking filed in NEXT.md MEDIUM.
- **Almost:** nothing half-done for M2. M3 (rest of the Vulkan lineup — Beetle
  PSX HW / Flycast / PPSSPP / Saturn HW — + capability tiering + software-peer
  fallback) and M4 (DX12/GL backends, MoltenVK) are untouched future stretch.
- **Next (when the arc resumes):** M3 — validate a second HW core (Beetle PSX
  HW Vulkan is the natural next, needs PSX BIOS + `beetle_psx_hw_renderer =
  vulkan`); the device-adoption + zero-copy path is core-agnostic so it should
  largely "just work," but each core's set_image semaphore use + sync-index
  behavior may differ (paraLLEl-N64 was `num_semaphores=0`). Separately: the
  NES/cross-system audio clipping pass (NEXT.md).

## 2026-06-08 (cont. 15c) — audio: honor SET_SYSTEM_AV_INFO (env 32) — fixes paraLLEl-N64 buzz

- **Symptom (playtest):** speed + aspect/flip good, but sound "horrible".
- **Diagnosed from the log math (NOT a zero-copy regression):** the sink was
  fed at ~48,065 i16/s while the 48 kHz device drains 96,000 i16/s — **exactly
  half** → the cpal callback zero-fills the gap every callback = buzzing. The
  audio collection (`cb_audio_sample_batch` frames×2, `drain_audio`) + the
  resampler are provably correct and shared with working software cores, so the
  CORE was producing ~22050 stereo frames/run_frame (half of the declared
  44100@60).
- **Root cause:** paraLLEl-N64 calls `RETRO_ENVIRONMENT_SET_SYSTEM_AV_INFO`
  (env 32) AFTER load (log line 504, post the line-142 core-load) to publish its
  REAL timing once the game's VI/region is known — but our env-32 handler only
  updated `display_aspect` and **threw away the revised `sample_rate`/`fps`**.
  The sink kept the placeholder 44100 captured at `finish_load` → chronic
  under/over-feed. A general libretro gap (any env-32 core), surfaced by the M2
  playtest.
- **Fix:** env-32 handler now captures + logs `fps`/`sample_rate`/geometry and
  stashes `(fps, sample_rate)` in `State.pending_av_info`; `take_pending_av_info()`
  drains it; the shell run loop applies it each frame — rebuilds the audio sink
  at the new rate (mirrors the core-swap rebuild path + re-applies min-latency)
  and retimes the frame limiter. oa-libretro 49 tests pass; workspace clean.
- **Next (operator):** rebuild + relaunch paraLLEl-N64. The log should now show
  `SET_SYSTEM_AV_INFO — fps=… sample_rate=…` (this tells us the core's REAL
  rate) and `core revised audio rate 44100 -> N Hz; rebuilding sink`, and the
  pushed-samples/s should match the device drain (no more half-feed) → clean
  audio. If env 32 reports sample_rate=44100 unchanged, the half-feed is a
  different (deeper) core quirk and the log will say so.

## 2026-06-08 (cont. 15b) — ✅ zero-copy PROVEN at 60 fps; "small image" was the `original` scaling setting

- **Playtest log (oa-current.log 15:10–15:11) confirms task 1 WORKS:**
  `adopted HW core Vulkan device (RTX 4090)` → `first set_image
  R8G8B8A8_UNORM` → `first zero-copy HW import — 640x240 … (no readback)`;
  the `readback avg ms/frame` line is **GONE** (import engaged). Steady-state
  fps = **60.0** (frame 480→600 = 120 frames / 2.001 s; same every interval
  thereafter), audio **0 dropped**. The on-screen "~55 fps" was a
  cumulative-since-launch average (dragged down by the tg16 bootstrap + load
  gap), not the instantaneous rate. **So zero-copy hit the M2 speed goal on
  paraLLEl-N64 — tasks 2 & 3 (multi-buffer sync / lock-narrowing) are NOT
  needed for this core.**
- **The three playtest complaints, resolved:**
  1. *"~1/4 screen / much smaller, centered"* — the per-game override is
     `scaling: "original"` (log line 96/100: `set_scaling_mode -> original`)
     = native 640×240 centered on the 1920×1009 surface, no scaling. Working
     as designed; readback fed the identical 640×240. Operator fix: set this
     game's scaling to Aspect-Correct Fit / Pixel-Perfect. NOT a zero-copy bug.
  2. *"still slow"* — actually 60 fps steady (see above); the average display
     just hadn't caught up (~3–4 s after load it's full speed).
  3. *"sound off"* — audio pushes steadily with 0 drops at 60 fps; the off-ness
     was the sub-60 startup ramp.
- **One real (minor) fix this pass:** import mode skipped the readback that
  used to refresh `fb_width/fb_height`, so the frame-stat log read a stale
  `fb 256x240` (the tg16 bootstrap size) and pointer-viewport mapping would
  use stale dims. `hw_after_run` now refreshes `fb_width/fb_height` from the
  current import frame even in import mode (cheap scalar copy; no readback).
  `display_aspect` was already correct (sourced from av_info at load, not the
  readback) so Aspect-Correct Fit will aspect the HW core properly.
- **Status:** M2 zero-copy is functionally DONE + proven at full speed on
  paraLLEl-N64, shader (CrtLite) intact, software-core swap path unchanged.
  Mergeable once the operator confirms the scaling-mode change looks right +
  re-checks a software core + swap-back. Tasks 2/3 deferred indefinitely
  (not needed unless a future HW core is sync-capped).

## 2026-06-08 (cont. 15) — M2 zero-copy import shipped (task 1) — awaiting playtest

- **Shipped (compiles clean, 49 tests pass; NOT yet runtime-tested):** the
  zero-copy import path that replaces the M1 synchronous readback.
  - **Researched first (the gate):** wgpu 23.0.1 source confirms a
    `texture_from_raw` texture is tracked as `TextureUses::UNINITIALIZED`
    (`wgpu-core/device/resource.rs:729`) → `derive_image_layout` →
    `vk::ImageLayout::UNDEFINED` (`wgpu-hal/vulkan/conv.rs:227`), with **no
    public way to seed the layout**. So a direct-import-and-sample WOULD discard
    the core's pixels. Confirmed `Texture::as_hal` / `CommandEncoder::as_hal_mut`
    / `Device::as_hal` + the hal `raw_handle()`/`raw_device()` accessors exist →
    Approach A is viable. (Operator-confirmed: build task 1, then measure.)
  - **`oa-render::Renderer::present_hw_image(frame, aspect)`** — records a raw
    `vkCmdBlitImage` (NEAREST, inverted dst-Y when `flip_y`) from the core's
    `set_image` `VkImage` into a wgpu-native `fb_texture` (its `VkImage` via
    `Texture::as_hal`), in a DEDICATED wgpu encoder (`as_hal_mut` → raw cmd
    buffer) with hand-managed barriers. Leaves `fb_texture` in
    `SHADER_READ_ONLY_OPTIMAL` = exactly where wgpu's `RESOURCE` tracker expects
    it, so from frame ≥2 wgpu emits NO transition and the blit's own
    `TRANSFER_WRITE→SHADER_READ` back-barrier supplies copy→sample visibility
    (frame 1 may be blank — wgpu's first-use `UNDEFINED→SHADER_READ`). Then runs
    the existing composite/scale/shader/bezel chain (extracted into a shared
    `composite_and_present`). dst format mirrors the core format (Rgba8/Bgra8)
    so the blit is identity. Source NEVER enters wgpu's tracker.
  - **Queue sync (D7 crux):** `oa-libretro::hw_queue_lock/unlock` expose the
    SAME `VulkanHw::queue_lock` the core's `lock_queue` callback honors; oa-shell
    brackets each `present_hw_image` (wgpu blit + present) with it so wgpu's
    submit can't race the core's worker-thread submits on the shared `VkQueue`.
  - **oa-shell wiring:** `set_hw_import_mode(adopted)` on adopt / `false` on
    restore (so `hw_after_run` skips the readback when importing); both present
    sites (main + run-ahead) route through a `present_current` helper that does
    HW-import when adopted (falling back to `present(framebuffer())` if no HW
    frame is pending / import fails → adoption-failed cores stay on readback).
- **Deferred (tasks 2 & 3), per the staged plan:** real multi-buffer
  `get_sync_index` (still mask=0x1 single-buffer) + narrowing the queue lock /
  GPU-side sync for CPU run-ahead. Do these only if fps is still capped after
  measuring task 1 (removing the 31 ms readback should be the main win —
  the on-GPU blit folds into wgpu's normal Fifo present, no host fence wait).
- **Next (operator):** rebuild + launch **paraLLEl-N64** (parallel-n64-gfxplugin
  = parallel, upscaling 1×). SUCCESS = the `readback avg … ms/frame` log
  DISAPPEARS (import mode), a `first zero-copy HW import …` line appears, fps
  climbs toward 60, audio correct, **CrtLite shader still applied**. Then
  confirm a software core + swap-back still render. Do NOT merge to main until
  this playtest passes.

## 2026-06-08 (cont. 14) — measurement: readback ~31ms is serialization, NOT upscale → zero-copy justified

- **Measured (paraLLEl-N64, oa-current.log 14:12):** readback steady ~31ms/
  frame; shader CrtLite **preserved** on the adopted renderer (4a works);
  adopt/restore/re-adopt all clean.
- **Operator confirmed upscaling is already 1× (native).** So the ~31ms is
  NOT GPU render cost (native 640×240 on a 4090 is ~1-2ms) — it's
  **serialization in the synchronous readback path**: ~31ms ≈ two 16.6ms
  vsync intervals; the `wait_for_fences` blocks the CPU each frame behind the
  core's pacing + wgpu's Fifo/vsync present on the shared queue, with no
  CPU/GPU overlap. Compounded by our M1 `get_sync_index_mask = 0x1` telling
  paraLLEl-RDP it has ONE in-flight buffer ("Using 1 sync frames" = minimal
  pipelining).
- **Verdict: zero-copy is the right fix** (measurement ruled out upscale/core
  GPU cost). The remaining M2 work:
  1. **Zero-copy import** — replace readback with sampling the core's VkImage.
     Hits the wgpu `texture_from_raw` layout-discard wall (no layout param →
     wgpu treats imported image as UNDEFINED → discards). Fix: raw-Vulkan
     image→image copy into a wgpu-native texture with hand-managed barriers
     (src SHADER_READ_ONLY→TRANSFER_SRC, dst restored to wgpu's expected
     layout), OR research wgpu-core's from-hal initial-state handling for a
     direct-sample path. The `present_hw_image` + `set_hw_import_mode(true)`
     wiring is scaffolded (4b) and ready.
  2. **Real `get_sync_index` multi-buffering** — return a multi-bit mask +
     cycle the index so the core pipelines (vs the M1 single-buffer stub).
  3. Drop the host fence wait (GPU-side sync via the core's semaphores / queue
     ordering) so the CPU runs ahead.
- **Banked + working on `feat/hw-render-m2`:** device adoption, renderer
  lifecycle (swap/restore/re-adopt), settings preservation, readback timing.
  M1 stays at tag `hw-render-m1-proven`.
- **Next session (focused):** the zero-copy import + multi-buffer sync (D7).
  Best done fresh + carefully — sync bugs are intermittent + miserable to
  debug rushed.

## 2026-06-08 (cont. 13) — M2 Stage 4: settings preservation + readback timing; zero-copy wall

- **Shipped:** 4a (settings preservation — shader/scaling/etc. carried across
  the renderer rebuild) + 4b (frame exposure `loaded_core_hw_frame` +
  `set_hw_import_mode` skip-readback flag) + a **readback timing probe**
  (`hw_after_run` logs avg readback ms/frame every 120 frames).
- **Stage 4c wall (researched):** true zero-copy (import the core's VkImage +
  sample it) hits a confirmed wgpu limitation — `wgpu_hal::vulkan::Device::
  texture_from_raw` has **no layout parameter** and the docs don't define the
  initial-state assumption, so wgpu will most likely treat the imported image
  as UNDEFINED and **discard the core's pixels** on first use. The correct fix
  is the intricate D7 sync work: a raw-Vulkan image→image copy from the core's
  VkImage into a wgpu-native texture with hand-managed barriers (src
  SHADER_READ_ONLY→TRANSFER_SRC, dst restored to wgpu's expected layout) +
  coordination with wgpu's state tracker. Not a guess-and-go.
- **Measure-first call:** before investing in that, confirm the readback IS
  the bottleneck. paraLLEl-RDP logged "Using 1 sync frames" — it may be
  self-synchronizing / slow regardless of our path, in which case zero-copy
  won't help and a core option (`parallel-n64-parallel-rdp-synchronous` off,
  upscale 1×) would. The readback timing probe + the existing fps line will
  tell us: if readback ≈ most of the ~34 ms/frame → zero-copy is the fix; if
  readback is small and the core run dominates → it's a core-config/perf issue.
- **Next (operator):** rebuild + launch paraLLEl-N64; report the
  `readback avg X ms/frame` line + the `~NN fps` line; and try toggling the
  parallel-RDP synchronous / upscaling core options to see if fps changes.

## 2026-06-08 (cont. 12) — ✅ M2 Stage 3 VALIDATED (adoption + lifecycle on hardware)

- **Operator confirmed:** paraLLEl-N64 loads on the adopted device AND swaps
  cleanly between it and software cores. So the M2 foundation is proven on
  real hardware: wgpu adopts the core's `VkDevice` (`from_raw` end-to-end),
  the HW core renders (via Stage-3 readback), and the renderer
  rebuild-on-core-switch lifecycle is crash-free (R1 holds). No queue-race
  glitches reported in this pass.
- **Next — Stage 4 (the zero-copy speed win), 3 parts:**
  1. **Skip readback when adopted** — add an oa-libretro flag so
     `hw_after_run` stops the CPU copy (the slow part) when the renderer
     adopted the device; just keep the frame marked for import.
  2. **Import + composite** — expose the core's current `set_image` VkImage
     (handle/format/extent/layout) from oa-libretro; oa-render imports it via
     `texture_from_raw` + `create_texture_from_hal` and runs the existing
     scaling/bezel/shader chain on it directly (no fb_rgba upload). New
     `Renderer::present_hw_image(...)`; oa-shell calls it (instead of
     `present(framebuffer)`) when adopted.
  3. **Settings preservation** — re-apply shader preset / scaling / bloom /
     overscan / rotation after a renderer rebuild (they reset today). Track
     the latest values in run_emu_render locals; bezel RGBA source isn't
     retained (minor; re-applied if the frontend re-sends).
- **Stage 4 risk (D7 sync):** the adopted wgpu + the core's Granite threads
  share one VkQueue; wgpu's present doesn't take the core's `lock_queue`.
  paraLLEl-N64 host-syncs (num_semaphores=0, "1 sync frame") so it may be
  fine, but if zero-copy glitches/tears, the fix is coordinating wgpu's
  submits with `lock_queue` (or waiting the core's semaphores).

## 2026-06-08 (cont. 11) — M2 Stage 3 smoke test: adoption WORKS; fixed surface-in-use

- **Smoke test (oa-current.log 13:30): the device adoption SUCCEEDED at
  runtime** — `oa-render: adopted HW core Vulkan device (RTX 4090) — zero-copy
  path active`. The whole novel chain ran: create_device(require swapchain) ok
  → context_reset → GET_HW_RENDER_INTERFACE → wgpu `from_raw`/`device_from_raw`/
  `create_device_from_hal`. The biggest M2 risk (wgpu adopting a device it
  didn't create) is PROVEN on real hardware.
- **Crash cause (fixed): `surface configuration failed: Native window is in
  use`.** My swap built the new adopted renderer (new surface on the HWND)
  while the OLD renderer's surface was still alive — Windows allows one
  surface per HWND. Fix: route all 3 swap sites through `hw_swap_to_adopted` /
  `hw_restore_normal` helpers that **`drop(old)` FIRST** (freeing the window)
  before building the new renderer; adoption failure falls back to a normal
  renderer (M1 readback). Compiles; workspace clean.
- **Known next risk (queue sync):** the adopted wgpu and oa-libretro's
  readback + the core's Granite threads all submit to the SAME VkQueue.
  wgpu's present (emu/render thread) isn't taking the core's `lock_queue`, so
  it can race the core's background submits (VkQueue needs external sync).
  May glitch/crash intermittently in Stage 3; the proper fix is the Stage 4
  sync work (D7). Re-testing Stage 3 first to see if it renders.
- **Next:** operator re-tests paraLLEl-N64 (expect: renders via readback on
  the adopted device, no surface crash; watch for queue-race glitches).

## 2026-06-08 (cont. 10) — M2 Stage 3: device adoption + lifecycle wired (compiles)

- **Stage 3a (committed `855bbf7`) — adoption machinery, de-risked:** the
  wgpu-hal `from_raw` path COMPILES on wgpu 23.0.1 (the central M2
  assumption). `oa-render`: extracted `Renderer::finish_build` (shared
  pipeline build) + `Renderer::new_adopting_vulkan` (wraps the core's raw
  VkInstance/device → `Instance::from_raw`/`expose_adapter`/`device_from_raw`
  → wgpu `from_hal`/`create_*_from_hal`, no-op drop callbacks so oa-libretro
  stays the sole owner) + `AdoptedVulkanDevice` + `RenderError::VulkanAdopt`;
  added `ash` (workspace 0.38 → handle types unify). `oa-libretro`:
  `try_create_device` now requires `[VK_KHR_swapchain]`.
- **Stage 3b (this commit) — oa-shell lifecycle wired:** `run_emu_render`
  (emu + render on ONE thread) now: on a Vulkan HW-core load swaps `renderer`
  to `new_adopting_vulkan` (`renderer_adopted` flag); on core-swap / UnloadRom
  rebuilds the normal renderer BEFORE the core drops (safe ordering — the core
  teardown destroys the device); adoption failure keeps the normal renderer →
  HW core falls back to the M1 readback path. **Stage 3 keeps readback**, so
  no speed change yet — it proves adoption + lifecycle. Workspace clean (zero
  warnings).
- **Known gap (to fix before/with Stage 4):** rebuilding the renderer resets
  display settings (shader preset / scaling / bloom / bezel) to defaults —
  they arrive via separate `ApplyShaderPreset`/`SetScalingMode` EmuCommands,
  not LoadRom. Affects HW cores only (software cores never rebuild). Simple
  scalar settings are easy to transfer via getters; the bezel's source RGBA
  isn't retained (minor). Will handle alongside Stage 4.
- **Runtime status:** compiles; NOT yet runtime-tested (the adoption could
  fail/crash at runtime). Decision pending: smoke-test Stage 3 (no speed
  change) vs roll straight into Stage 4 (zero-copy import = the speed win) +
  settings preservation, then one playtest.

## 2026-06-08 (cont. 9) — M2 started: architecture confirmed + Stage 1/2

- **Branch `feat/hw-render-m2`** (off `feat/hw-render-m1`; M1 stays bankable
  at tag `hw-render-m1-proven`).
- **Architecture confirmed with operator (DECISIONS D9/D10):** reinit-per-core
  (software cores stay on their current backend, untouched → can't regress;
  only Vulkan HW cores rebuild the renderer onto the core's device). HwContext
  trait + per-launch picker is the path to per-core backend selection
  (Vulkan now; GL/D3D = M4, one trait impl each — the operator's "cores pick
  what they run on" goal). Operator confirmed zero-copy is a framerate/timing
  WIN with no added input lag, given we keep the pipeline shallow (no
  latency-for-throughput buffering).
- **Stage 1 (done) — wgpu-hal 23 API verified** for Approach A: single
  `ash 0.38` in the lock ⇒ handle types unify (no transmute);
  `Instance::from_raw` + `expose_adapter` + `Adapter::required_device_extensions`
  + `Adapter::device_from_raw` + wgpu `create_*_from_hal` all present. Clean
  ordering: build hal Adapter first → ask it which device exts it needs →
  feed those into `create_device`. Key files confirmed: `oa-render/src/lib.rs`
  `new_async` (Backends::PRIMARY); **`run_emu_render` (main.rs:3538) owns BOTH
  the emu loop AND the renderer on ONE thread** → no cross-thread handoff for
  the device adoption (big simplification); `renderer` is a swappable
  `let mut` (main.rs:3583).
- **Stage 2 (shipped) — exposure scaffolding, NO behavior change:**
  oa-libretro exposes the core's adopted-device handles via
  `loaded_core_hw_vulkan() -> Option<LoadedHwVulkan>` (raw u64 handles +
  device-extension list + apiVersion, no ash leak across crates) + the
  `VulkanHw::adopted_handles` accessor + two new VulkanHw fields
  (`device_extensions`, `api_version`; empty/1.1 in M1, real values threaded
  in Stage 3). Readback path untouched → paraLLEl-N64 still renders exactly
  as M1. Workspace clean (zero warnings); 49 tests pass.
- **Next — Stage 3 (the gated interop):** build wgpu from the core's adopted
  device + move the surface onto our instance + `create_device` requests
  VK_KHR_swapchain + swap the `Renderer` in `run_emu_render` on HW-core load.
  CONFIRM with operator before this (R1: software cores re-validated after a
  software⇄HW switch).

## 2026-06-08 (cont. 8) — 🎉 M1 PIPELINE PROVEN (paraLLEl-N64 renders in-process)

- **The null-features fix worked.** Full pipeline confirmed end-to-end with
  **paraLLEl-N64 / paraLLEl-RDP (Vulkan)**:
  `create_device returned ok=true` → `device built … ready` →
  `GET_HW_RENDER_INTERFACE -> Vulkan interface` → `context_reset returned` →
  `first set_image — format=R8G8B8A8_UNORM layout=SHADER_READ_ONLY_OPTIMAL
  num_semaphores=0` → `first readback OK — 640x240 … (frame on screen)`.
  A GPU-rendering libretro core runs **in-process instead of crashing** — the
  M1 architecture (standalone ash VkDevice + create_device + the 8 interface
  callbacks + CPU readback into fb_rgba) is validated. wgpu untouched; the 46
  software cores unaffected.
  - NOTE on the exit gate: the plan's literal M1 gate named *Dolphin/GameCube*.
    We proved the pipeline on paraLLEl-N64 instead (Dolphin is a separate,
    harder problem — it builds its own windowless context and silent-crashes;
    parked for ground-truth Vulkan validation). The *architecture* M1 set out
    to prove is done.
- **Known M1 limitation — speed:** ~29 fps (half of N64's 60). Audio sounds
  off purely because emulation runs at ~half speed (audio produced at
  half-rate); not an audio bug. Root cause = M1's synchronous readback: every
  frame we submit a copy and host-wait the fence (`wait_for_fences u64::MAX`),
  fully draining the GPU and killing CPU/GPU overlap. `num_semaphores=0` (the
  core host-synchronizes its image) compounds it. **This is exactly what M2
  (zero-copy import, no readback, no host wait) removes** — speed was always
  an M2 deliverable (D2/D6).
- **Shipped:** docs only (this entry). Code unchanged since `824b38a`.
- **Next:** (a) operator can try paraLLEl-RDP core options that affect speed
  (`parallel-n64-parallel-rdp-synchronous` off; upscaling = 1x) — may help but
  our full-drain readback caps the ceiling regardless; (b) optional per-frame
  timing instrumentation (run_frame vs readback) to confirm readback is the
  dominant cost; (c) the real fix is **M2 zero-copy** — import the core's
  VkImage via wgpu (commits wgpu to the Vulkan backend per D3) and composite
  on-GPU. Consider whether M1 (renders, slow) is a mergeable checkpoint before
  starting M2.

## 2026-06-08 (cont. 7) — M1: past the option wall; create_device null-features fix

- **The cont. 6 core-option fix WORKED.** paraLLEl-N64 now requests
  `SET_HW_RENDER context_type=6` (Vulkan), we accept, and it sends the
  negotiation interface (`version=1`, `create_device(v1)`, app="paraLLEl-RDP"
  engine="Granite", apiVersion 1.1). We're past every prior wall and into the
  real device negotiation. **The crash is now precisely at our call into the
  core's `create_device`** (log ends right after `get_application_info`).
- **Researched the create_device contract** (libretro_vulkan.h + a libretro
  forum/repo search). Findings:
  - We passed `required_features = NULL`. Spec permits NULL, but **Granite
    (paraLLEl-RDP's engine) dereferences it** to merge features into the
    device — NULL faults. Same null-deref class as the GET_GAME_INFO_EXT
    strings. RetroArch passes a real (zeroed) `VkPhysicalDeviceFeatures`.
  - Null surface IS valid headless (paraLLEl-RDP is compute-only) — fine.
  - Version dispatch confirmed: v1 → `create_device` (correct); the
    `create_instance/create_device2 = true` we logged is an out-of-bounds
    garbage read of a v1-sized struct (harmless — we don't call them).
  - paraLLEl-RDP needs `VK_EXT_external_memory_host` (device ext) but the
    CORE adds that in create_device; NVIDIA-on-Windows supports it.
- **Shipped (branch `feat/hw-render-m1`, not merged):** `try_create_device`
  now passes `&VkPhysicalDeviceFeatures::default()` (zeroed) instead of NULL,
  plus pinpoint logs immediately before/after the create_device call
  (`calling core create_device…` / `create_device returned ok=… device_null=…`)
  so the next run is decisive regardless. oa-libretro checks clean.
- **Next (operator):** rebuild + relaunch the N64 game. If the null-features
  theory is right: `create_device returned ok=true` → `device built … ready`
  → `GET_HW_RENDER_INTERFACE` → `context_reset` → `first set_image` →
  `first readback OK`. If `calling core create_device…` logs but `returned`
  does NOT, create_device still faults on something else (then it's the next
  lead). Sources: libretro_vulkan.h; libretro forums parallel-n64 Vulkan.

## 2026-06-08 (cont. 6) — M1: fixed core-option timing (renderer option applied too late)

- **Playtest of paraLLEl-N64:** crashed — but NOT in our code. Log: core
  requested `SET_HW_RENDER context_type=1` (OpenGL), we correctly declined
  (Vulkan-only), core logged `mupen64plus: libretro frontend doesn't have
  OpenGL support` and bailed. So paraLLEl-N64 ran its GL plugin, never Vulkan.
- **Operator set `parallel-n64-gfxplugin = parallel`** (verified persisted in
  `core-options/n64.json` → `values: {parallel-n64-gfxplugin: parallel}`) and
  relaunched — **still `context_type=1`**. Root cause found:
  - `main.rs` applied per-system core-option overrides only in the **post-load**
    block (~line 4654), which the comment at ~4684 confirms runs AFTER
    `retro_load_game`. But cores gate their HW-render API on a core option read
    **during** `retro_load_game` (paraLLEl-N64 `gfxplugin`, Beetle PSX HW
    `renderer`, Flycast, PPSSPP). So the override landed too late → core saw
    the default (GL) → requested OpenGL → declined → crash. **A blocker for
    the whole Vulkan HW lineup, not just N64.**
- **Shipped (branch `feat/hw-render-m1`, not merged):** `main.rs` now
  pre-applies the operator's stored per-system core-option overrides via
  `core_ref.set_option` **immediately before `core_ref.load_rom`** (new block
  just above the stem precompute, ~line 4544). `set_option` only stages the
  value in `State.option_values`, so the core's `GET_VARIABLE` poll during
  load returns it even before the schema is captured. The post-load block
  still runs (full effective merge + visibility). oa-shell checks clean (zero
  warnings).
- **Next (operator):** **rebuild** (release) so the fix is in the binary, then
  relaunch the N64 game (option already set). Expect `SET_HW_RENDER
  context_type=6` → `accepted (Vulkan)` → device → `GET_HW_RENDER_INTERFACE`
  → `context_reset` → `first set_image` → `first readback OK`. If it STILL
  shows `context_type=1`, paraLLEl-N64 reads gfxplugin during retro_init
  (before our pre-apply) and we'd need to push overrides inside
  `LibretroCore::load` before set_environment — but the log timing
  (SET_HW_RENDER fires during load_game, after load() returns) says
  pre-load_rom should be in time.

## 2026-06-08 (cont. 5) — M1: WSI ext didn't fix it; pivot to a simpler Vulkan core

- **Playtest of cont. 4 (oa-current.log 11:23):** instance extensions now
  enabled (log confirms all 4), but **same death point** — Dolphin reaches
  `Using GFX backend: Vulkan` then silent-crashes. New detail: after our
  `create_device` succeeds, Dolphin runs `VulkanContext.cpp:317/:334/:214` —
  it **creates its OWN VkInstance** ("Using Vulkan 1.2") and crashes there,
  **before ever calling GET_HW_RENDER_INTERFACE (env 41)**. So this Dolphin
  build sets up its own complete Vulkan context (not rendering into the
  device we provide) and dies in a raw access violation with zero error text.
- **Operator decision:** stop blind-iterating on Dolphin (hardest possible
  core — full standalone Dolphin, windowless in-process). **Prove the M1
  pipeline on a simpler Vulkan core first** (paraLLEl-N64 / Beetle PSX HW),
  then treat Dolphin as a separate, harder integration. Our code is fully
  core-agnostic so no changes were needed for this.
- **Shipped (branch `feat/hw-render-m1`, not merged):** added one-shot
  diagnostic logs — `first set_image` (format/layout/semaphores) and
  `first readback OK` (WxH/format/flip) — so the next run with a working
  core confirms the device → create_device → set_image → readback path
  end-to-end. Workspace checks clean; 49 tests pass.
- **Next (operator):** grab a clean Vulkan HW core from buildbot.libretro.com
  + a ROM, set its renderer to Vulkan, launch. Recommended: **paraLLEl-N64**
  (`parallel_n64_libretro.dll`, no BIOS, written by the libretro-Vulkan
  author — the cleanest possible contract test; set core option
  `parallel-n64-gfxplugin = parallel` for the Vulkan RDP) with any `.z64`.
  Alt: Beetle PSX HW (`mednafen_psx_hw_libretro.dll`, needs PSX BIOS, set
  `beetle_psx_hw_renderer = vulkan`). Watch the log for the new
  `first set_image` / `first readback OK` lines = pipeline proven.

## 2026-06-08 (cont. 4) — M1: researched the protocol; instance needs WSI extensions

- **Playtest of cont. 3 (oa-current.log 10:55):** create_device **worked**
  (`core built device via create_device — ready`), but Dolphin then ran its
  OWN `VulkanContext::Create`, reached `Using GFX backend: Vulkan`, and
  hard-crashed — *before* ever calling GET_HW_RENDER_INTERFACE (env 41). Same
  death point as cont. 2.
- **Researched the actual contract** (operator chose research-first) by
  fetching `libretro_vulkan.h`, DolphinLibretro `Vulkan.cpp`, and RetroArch
  `gfx/common/vulkan_common.c`. Findings (recorded in DECISIONS D7):
  - The core renders **headless** into images it hands us via `set_image`;
    it never presents to a real window. Our readback model is correct.
  - The `set_image` image is created with **TRANSFER_SRC | SAMPLED |
    TRANSFER_DST | COLOR_ATTACHMENT**, layout `SHADER_READ_ONLY_OPTIMAL` —
    so our `vkCmdCopyImageToBuffer` readback is valid. **The "TRANSFER_SRC
    risk" is resolved** (it's guaranteed present).
  - **Root cause of the crash:** RetroArch creates its `VkInstance` WITH the
    WSI/surface instance extensions (`VK_KHR_surface`, `VK_KHR_win32_surface`,
    `get_physical_device_properties2`, `get_surface_capabilities2`). We
    created ours bare. Dolphin's video backend creates a surface + (fake)
    swapchain internally and dereferences `vkCreate*SurfaceKHR` / swapchain
    entry points — which are **NULL without those instance extensions** →
    crash in the core's own init, exactly where we saw it.
- **Shipped (branch `feat/hw-render-m1`, not merged):**
  `VulkanInstance::create` now enables the four WSI/surface instance
  extensions (each only if the loader reports it available), matching
  RetroArch's `vulkan_context_create_instance_wrapper`. Workspace checks
  clean (zero warnings); oa-libretro 49 tests pass.
- **Next (operator):** launch again. If the missing-extension theory is
  right, Dolphin's own context init now succeeds and it proceeds past
  `Using GFX backend: Vulkan` to call `GET_HW_RENDER_INTERFACE` (env 41) →
  `context_reset` → frames. Paste the new `oa-libretro HW:` lines + what's on
  screen. If it still dies at the same spot, the next lever is matching
  RetroArch's create_device *timing* (it calls create_device from video init
  after load, not from the env handler) and/or passing a real/headless
  surface.

## 2026-06-08 (cont. 3) — M1 fix: use the core's create_device (was deadlocking)

- **Playtest result of cont. 2 (oa-current.log 10:30):** black screen + hard
  lock (force-killed). Log got MUCH further — `SET_HW_RENDER accepted
  (Vulkan)`, `standalone Vulkan device ready`, and the **negotiation
  interface DID arrive** (`create_device(v1)=true`, app="Dolphin-Emu",
  apiVersion 1.0). But then Dolphin spun up its **own** VkInstance
  (VulkanContext: "Loading system driver", `VK_KHR_surface`/`win32_surface`,
  "Using Vulkan 1.2") and hung right after `Using GFX backend: Vulkan` — no
  env 41, no context_reset.
- **Root cause:** we **self-built** the device and never called the core's
  `create_device`. Without that call the core doesn't know the frontend owns
  the Vulkan context, so Dolphin fell back to creating its own context +
  swapchain — which needs a window it doesn't have → deadlock. (Two separate
  devices would also make any handed-back image unreadable on our device.)
- **Shipped (branch `feat/hw-render-m1`, not merged):** restructured the
  device bring-up to the canonical libretro path.
  - `hw_vulkan.rs`: split into `VulkanInstance` (instance + GPU select, made
    at SET_HW_RENDER accept) and `VulkanHw` (the device). New
    `VulkanInstance::try_create_device` calls the core's `create_device` with
    OUR instance + GPU + null surface (headless) and wraps the device the
    core returns (`ash::Device::load`). `into_hw_self_build` is the fallback
    (cores with no negotiation). `finalize` builds readback + interface;
    `destroy` cleans an un-finalized instance.
  - `state.rs`: SET_HW_RENDER (14) now creates the **instance** only (device
    deferred). SET_HW_RENDER_CONTEXT_NEGOTIATION_INTERFACE (43) **calls
    create_device** then and there (before the core can fall back), finalizes
    the device. `hw_after_load` self-builds only if no device exists yet
    (no-negotiation cores), then drives context_reset. `hw_teardown` also
    destroys a leftover instance. New `State.hw_instance`.
  - Workspace checks clean (zero warnings); oa-libretro 49 tests pass.
- **Almost / still the KNOWN RISK:** readback still uses
  `vkCmdCopyImageToBuffer` (needs the core image to have TRANSFER_SRC usage).
  Now at least the device is shared so handles are valid. If the copy is
  rejected we pivot to a sampled-blit readback.
- **Next (operator):** launch again. Success trail now extends to:
  `device built via core create_device — ready` → `Using GFX backend: Vulkan`
  → `GET_HW_RENDER_INTERFACE -> Vulkan interface` → `calling context_reset` →
  `context_reset returned` → then frames. Paste the new `oa-libretro HW:`
  lines + what's on screen.

## 2026-06-08 (cont. 2) — M1 device-build: standalone ash device + readback

- **Probe-run findings (oa-current.log 10:02):** GET_PREFERRED_HW_RENDER=Vulkan
  worked; Dolphin offered Vulkan **first** in its backend list. GPU select
  correct (only device = RTX 4090, Vulkan 1.4.329). Core wants Vulkan
  **apiVersion 1.0** (version_major=4194304 = `VK_MAKE_API_VERSION(0,1,0,0)`).
  `bottom_left_origin=true` → readback flips vertically. **No negotiation
  interface (env 43) and no env 41** were sent — because we declined every
  SET_HW_RENDER (Dolphin only registers those after a HW context is
  accepted). ⇒ M1 decision refined: **self-build the device** (this Dolphin
  build provides no `create_device`); D6 intent (standalone ash device +
  CPU readback, wgpu untouched) preserved.
- **Shipped (branch `feat/hw-render-m1`, not merged):**
  - `hw_vulkan.rs`: `VulkanHw` — self-builds VkInstance (Vulkan 1.1) → first
    discrete GPU → graphics queue → device with **all** available device
    extensions + features (retry bare on failure) → readback cmd
    pool/buffer/fence. The 8 frontend interface callbacks (`set_image`,
    `get_sync_index`≡0, `get_sync_index_mask`≡0x1, `set_command_buffers`
    no-op+warn, `wait_sync_index` no-op, `lock_queue`/`unlock_queue` via an
    AtomicBool spin-lock, `set_signal_semaphore`). Per-frame **synchronous
    readback**: barrier core image→TRANSFER_SRC, `vkCmdCopyImageToBuffer`,
    barrier back, submit (wait on core sems, signal core's done-sem) under
    the queue lock, host-wait the fence, map, swizzle BGRA/RGBA + vertical
    flip into `fb_rgba`. `VulkanHw::Drop` waits idle + destroys everything.
  - `state.rs`: SET_HW_RENDER (14) now **accepts Vulkan** — eagerly builds
    the device (so the interface is ready whenever the core queries env 41,
    which can fire mid-load), nulls the GL-style callback fields, returns
    true; declines non-Vulkan. GET_HW_RENDER_INTERFACE (41) returns our
    persistent `retro_hw_render_interface_vulkan`. `cb_video_refresh`
    sentinel now marks the frame ready (extent from w/h). Free fns
    `hw_after_load` (drives context_reset), `hw_after_run` (readback),
    `hw_teardown` (context_destroy + device drop). State gained
    `hw_vulkan: Option<Box<VulkanHw>>` (boxed → stable handle address).
  - `core.rs`: `finish_load` → `hw_after_load`; `run_frame` → `hw_after_run`;
    `Drop` → `hw_teardown` before deinit/uninstall.
  - Workspace checks clean (zero warnings); oa-libretro 49 tests pass.
- **Almost / KNOWN RISK:** the readback uses `vkCmdCopyImageToBuffer`, which
  requires the core's presented image to have `VK_IMAGE_USAGE_TRANSFER_SRC_BIT`.
  The libretro Vulkan spec guarantees the image is **SAMPLED** (for the
  frontend to composite via a shader), not necessarily TRANSFER_SRC. If
  Dolphin's output image lacks TRANSFER_SRC, the copy is invalid → likely
  garbage or a validation/driver error rather than a frame. **Most likely
  next failure point.** Fix if hit: readback via a tiny sample-into-our-own-
  TRANSFER_SRC-image pass (render pass + sampler + fullscreen shader) — more
  code, deferred until the log shows it's needed.
- **Next (operator):** launch the GameCube game again. Look in
  `oa-current.log` for: `SET_HW_RENDER accepted (Vulkan)` →
  `standalone Vulkan device ready` → `GFX backend: Vulkan` (NOT Null) →
  `GET_HW_RENDER_INTERFACE -> Vulkan interface` → `calling context_reset`.
  If it renders (even imperfectly) the M1 handshake is proven. Paste the new
  `oa-libretro HW:` lines + describe what's on screen (frame / garbage /
  upside-down / crash) so we tune format/flip or pivot to the sampled
  readback.

## 2026-06-08 (cont.) — M1 probe phase: negotiation + GPU enumeration logging

- **Shipped (branch `feat/hw-render-m1`, not merged — commit `021bf74`):**
  Operator chose **probe-first** (de-risk the Vulkan negotiation ABI before
  writing the standalone device) and **dedicated-GPU** selection (the 4090,
  i.e. first discrete GPU over integrated). Landed a zero-device-setup commit
  that captures exactly what the operator's Dolphin .dll requests:
  - `state.rs`: split `SET_HW_RENDER` out of the camera/location decline arm;
    added the three HW-render env arms — `GET_PREFERRED_HW_RENDER` (56) now
    answers **Vulkan** (so the core sends its Vulkan negotiation interface
    instead of defaulting to OpenGL like the 2026-06-07 log showed),
    `SET_HW_RENDER` (14) logs context_type/version/flags + stores the callback
    but **still returns false** (probe phase — behavior unchanged, no device),
    `SET_HW_RENDER_CONTEXT_NEGOTIATION_INTERFACE` (43) type-checks + logs +
    stores the Vulkan negotiation struct.
  - `cb_video_refresh`: special-case the `RETRO_HW_FRAME_BUFFER_VALID`
    `(void*)-1` sentinel before the pixel path — **fixes a latent crash**
    (old null-check let `-1` through into `pixel::convert`).
  - `hw_vulkan.rs` (new): `log_negotiation_interface` (version, which
    callbacks the core provides — v1 `create_device` vs v2 `create_device2`,
    plus the core's preferred Vulkan apiVersion) + `probe_physical_devices`
    (enumerate GPUs via a throwaway ash instance, log each + report which one
    M1 selects).
  - `State` gained `hw_render_callback` + `hw_negotiation_vulkan` (stored now,
    consumed by the device-build commit).
  - Workspace checks clean (zero warnings); oa-libretro 49 tests pass; pushed.
- **Almost:** nothing half-done — this is a clean, mergeable diagnostic step.
- **Next (operator action, then device-build commit):** operator launches a
  GameCube game once with this build; the OA log (`oa-current.log`) will now
  print the negotiation version, which create_device callbacks are non-NULL,
  the core's wanted apiVersion, and the GPU list with the selected card.
  Paste those lines back, then write the device-build commit: stand up the
  standalone `VkInstance`/`VkDevice` (calling the core's `create_device`),
  the 8 Vulkan interface callbacks (M1 sync model: `get_sync_index`≡0,
  mask≡0x1, real `lock_queue` mutex, synchronous per-frame readback), the
  `VkImage`→CPU readback into `fb_rgba`, flip `SET_HW_RENDER` to accept +
  answer `GET_HW_RENDER_INTERFACE` (41), and drive `context_reset` from
  `finish_load`. `main.rs` likely unchanged (HW frame surfaces through the
  existing `fb_rgba` slice).

## 2026-06-08 — M1 started: FFI layer + ash + device decision

- **Shipped (branch `feat/hw-render-m1`, not merged):**
  - HW-render FFI foundation in `oa-libretro/ffi.rs` — the API-agnostic
    core (`retro_hw_render_callback`, `RETRO_HW_CONTEXT_*`,
    proc-address / context-reset / get-current-framebuffer typedefs,
    `RETRO_HW_FRAME_BUFFER_VALID` sentinel), verified byte-for-byte
    against libretro.h (commit `0e8efd8`).
  - Full Vulkan libretro FFI (`libretro_vulkan.h`) typed with `ash::vk`
    handles so it's ABI-correct: `retro_vulkan_image`,
    `retro_hw_render_interface_vulkan` (+ its 8 frontend callbacks),
    `retro_vulkan_context`, the negotiation interface + all callback
    typedefs (v1 `create_device` + v2 wrappers), version/enum constants.
    `ash 0.38` added as a workspace dep (commit `f6a77e5`). oa-libretro
    checks clean.
  - **Decision D6 (operator):** M1 uses a **standalone ash `VkDevice`**
    for the core, isolated from wgpu, with CPU readback into the
    existing present path — NOT the wgpu-shared device (that + zero-copy
    move to M2). Lowest-risk route to a rendered frame: wgpu untouched
    (zero regression to the 46 software cores) and the core builds the
    device it needs (sidesteps extension mismatch). Recorded in the plan
    (D6 + refined M1) + feature DECISIONS.
- **Almost:** nothing half-done — the FFI layer is complete + compiling;
  the device/runtime work hasn't started.
- **Next:** stand up the standalone `VkInstance`/`VkDevice` via ash
  (calling the core's negotiation `create_device`), implement the 8
  Vulkan interface callbacks (`set_image` + the sync-index machinery),
  the image→CPU readback into `State.fb_rgba`, the `state.rs` handshake
  (replace `SET_HW_RENDER => false`; answer `GET_PREFERRED_HW_RENDER` +
  `GET_HW_RENDER_INTERFACE`; special-case the `FRAME_BUFFER_VALID`
  sentinel in `cb_video_refresh`), and the `main.rs` `LoadRom` run-loop
  HW present branch. Substantial unsafe-Vulkan; own focused session.

## 2026-06-07 — Planning locked

- **Shipped:** Full plan + feature folder scaffold after the first
  internal Dolphin playtest crashed. Root cause confirmed from the OA
  log: the `dolphin_libretro` core requests `SET_HW_RENDER` (env cmd
  `0xe`), OA hard-returns `false` (always has, since `e134fee`
  2026-05-18), Dolphin falls back to its Null video backend and
  crashes in-process. Plan at
  [docs/PLANS/hw-render-pipeline.md](../../PLANS/hw-render-pipeline.md).
  Operator decisions: Vulkan-first multi-backend abstraction
  (RetroArch video-driver model on top of wgpu's existing backends);
  shared-device zero-copy as the end state; commit wgpu to the Vulkan
  backend; capability tiering + software-peer fallback; **no
  HW-render guard** (make the cores work, don't deny them). Four
  milestones M1 (handshake, core on screen) → M2 (zero-copy) → M3
  (lineup + reinit-on-switch + tiering) → M4 (DX12/GL only if needed).
- **Almost:** Nothing — pure planning session, no engine code touched.
- **Next:** M1 — add the HW-render + Vulkan negotiation structs to
  `oa-libretro/ffi.rs`, replace the `SET_HW_RENDER => false` arm +
  handle the `RETRO_HW_FRAME_BUFFER_VALID` sentinel in
  `cb_video_refresh`, stand up `oa-render` on the Vulkan backend with
  a `VulkanHwContext`, and get a frame of Dolphin on screen via the
  simplest present bridge. Gated behind VL Phase C3 landing first
  (avoids a `main.rs` `LoadRom` double-edit). See plan §Milestones.
