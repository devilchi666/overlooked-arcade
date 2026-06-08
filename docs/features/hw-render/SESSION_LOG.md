# HW-Render Pipeline — Session Log

Each session that touches this feature appends a 3-line entry:
**Shipped / Almost / Next.** Older entries roll to
`SESSION_LOG_ARCHIVE.md` when this file passes ~150 lines.

---

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
