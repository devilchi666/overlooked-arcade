# HW-Render Pipeline — Session Log

Each session that touches this feature appends a 3-line entry:
**Shipped / Almost / Next.** Older entries roll to
`SESSION_LOG_ARCHIVE.md` when this file passes ~150 lines.

---

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
