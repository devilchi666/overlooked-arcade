# HW-Render Pipeline — Decisions

Append-only log of implementation decisions made during the build.
Strategic decisions made in the planning conversation live in
[docs/PLANS/hw-render-pipeline.md](../../PLANS/hw-render-pipeline.md)
"Operator-locked decisions". This file captures the *why* as code
lands.

---

## 2026-06-07 — Planning decisions (locked)

Captured from the planning conversation that produced
[docs/PLANS/hw-render-pipeline.md](../../PLANS/hw-render-pipeline.md).

### D1 — Vulkan first, multi-backend abstraction underneath

RetroArch supports every API by keeping a set of video drivers with
**one active at a time**, matched to the loaded core — not by running
several graphics APIs at once (you can't composite across APIs without
interop). OA mirrors that with a `HwContext` trait, but builds it on
wgpu's existing multi-backend support + `texture_from_raw` import
rather than four hand-written renderers. We ship the **Vulkan** context
first and only.

**Why Vulkan first:** one Vulkan bridge unlocks the whole HW lineup at
its *best* renderer — and two of them are Vulkan-only (paraLLEl-N64's
accurate RDP; Beetle PSX HW's best renderer). GL would be a hard
quality ceiling on exactly those. DX12/GL contexts get added later only
if a core runs materially better on them or operators hit Vulkan-driver
problems (operator framing: "roll out Vulkan first, add the others if
we find other cores run better on it or people have issues with
Vulkan").

### D2 — Shared-device zero-copy is the destination

The "make OA itself run better" win is backend-agnostic: it comes from
the core sharing wgpu's device and OA importing the core's `VkImage`
directly, so OA's shaders/bezel/scaling sample it on-GPU with no CPU
roundtrip. M1 may stage through a readback to prove the handshake; the
end state is zero-copy.

### D3 — Commit wgpu to the Vulkan backend for the HW path

A clean shared device needs the core and compositor on one device, so
`oa-render` runs on wgpu's Vulkan backend (today: `Backends::PRIMARY`,
DX12 on most Windows hosts). Software cores are backend-agnostic
(CPU-upload path), so they're unaffected — but the global switch is
smoke-tested against the existing 46 cores before merge.

### D4 — Capability tiering, never a silent crash

Probe Vulkan availability at startup; HW cores on a host without a
usable Vulkan device fall back to a software-peer core (PSX/N64/Saturn
have peers) or surface as unavailable — never the Null-backend crash.

### D5 — No HW-render guard

Explicit operator call: do **not** add a deny-list that refuses HW
cores. Honest catalog *labeling* ("needs OA hardware-rendering — in
development") is allowed (it's not a guard); the real fix is this
pipeline.

---

## 2026-06-08 — M1 implementation decision

### D6 — M1 standalone ash device + readback (not wgpu-shared)

Surfaced once the libretro Vulkan negotiation ABI was in hand. M1
stands up a separate `VkInstance`/`VkDevice` via `ash`, isolated from
wgpu, built per the core's negotiation interface; the core's rendered
`VkImage` is read back to CPU and fed the existing software-framebuffer
present path. wgpu is untouched (stays DX12).

**Why:** lowest-risk route to "Dolphin actually renders." Keeps the 46
working software cores at zero regression risk (no global wgpu-Vulkan
switch in M1) and sidesteps the extension-compatibility wall (the core
builds the device it needs rather than inheriting wgpu's). The
wgpu-shared device + zero-copy (plan D3) move to M2. ~150 lines of
device setup are throwaway; the FFI + handshake + Vulkan interface
callbacks + run-loop wiring carry over. Recorded in the plan as D6 +
the refined M1 milestone.

### D7 — libretro Vulkan HW protocol (researched from source, 2026-06-08)

Captured after three failed Dolphin bring-up attempts, from
`libretro_vulkan.h`, DolphinLibretro `Vulkan.cpp`, and RetroArch
`gfx/common/vulkan_common.c`. The authoritative contract:

- **Frontend owns the instance; core builds the device.** The frontend
  creates the `VkInstance` (honoring `get_application_info`'s apiVersion,
  else 1.0 upgraded to 1.1), selects a GPU, then calls the core's
  `create_device` (v1) / `create_device2` (v2) passing that instance +
  GPU. The core creates the `VkDevice` + a GRAPHICS+COMPUTE queue and
  fills `retro_vulkan_context`. Fallback: if `create_device` is NULL or
  returns false, the frontend builds the device itself.
- **The frontend instance MUST carry the WSI/surface extensions**
  (`VK_KHR_surface` + platform surface, e.g. `VK_KHR_win32_surface`, plus
  `get_physical_device_properties2` / `get_surface_capabilities2`). HW
  cores create a surface + swapchain internally (headless/fake — they
  render into images, never to a real window) and will null-deref the
  surface/swapchain entry points if the instance lacks these. **This was
  the cont. 3 crash.**
- **The core renders headless; the frontend composites.** The core never
  presents to a window. It hands the frontend a finished image via
  `set_image` and signals `video_refresh(RETRO_HW_FRAME_BUFFER_VALID)`.
- **The `set_image` image is guaranteed `TRANSFER_SRC | SAMPLED`** (Dolphin
  also adds `TRANSFER_DST | COLOR_ATTACHMENT`), layout
  `SHADER_READ_ONLY_OPTIMAL`. So a `vkCmdCopyImageToBuffer` readback (M1)
  is valid, and an on-GPU sampled composite (M2) is the zero-copy path.
- **Surface passed to create_device:** RetroArch passes its real window
  surface; M1 passes `VK_NULL_HANDLE` (headless — we have no window for the
  core and read back to CPU). Contract-legal ("if surface is NULL, the
  core need not consider presentation when creating queues").
- **create_device timing:** RetroArch calls it from its own video-init
  flow (after load), not the env handler. M1 currently calls it from the
  env-43 handler (got the device built successfully); revisit timing only
  if a core needs it.
