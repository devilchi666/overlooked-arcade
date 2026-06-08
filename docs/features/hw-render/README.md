# HW-Render Pipeline — Feature

OA can host libretro cores that render on the GPU — Dolphin
(GameCube/Wii), paraLLEl-N64, Beetle PSX HW, Flycast (Dreamcast),
PPSSPP, Beetle Saturn HW — instead of only cores that hand back a CPU
framebuffer.

The unlock is implementing the libretro **HW render interface**
(`SET_HW_RENDER` + the Vulkan HW negotiation), which OA has hard-denied
since its first libretro commit. A HW-rendered core is a GPU emulator;
when OA declines, the core falls back to a Null video backend and
crashes the whole app (it runs in-process). The fix is to give the core
a real GPU context that shares OA's wgpu device.

The architecture mirrors RetroArch's video-driver model — a
**`HwContext` abstraction with exactly one backend active at a time**,
matched to the loaded core — but built on top of wgpu's existing
multi-backend support (Vulkan / DX12 / Metal / GL) and `texture_from_raw`
import, rather than four hand-written renderers. **Vulkan ships first
and only**; DX12/GL contexts are added later only if a core runs better
on them or operators hit Vulkan-driver issues. Once the core shares
wgpu's device, its output stays GPU-resident and OA's own
shaders/bezel/scaling sample it with zero copy — which makes OA itself
faster, not just the HW cores.

## Source of truth

[docs/PLANS/hw-render-pipeline.md](../../PLANS/hw-render-pipeline.md)
holds the locked design: the crash analysis, the operator-locked
decisions (Vulkan-first multi-backend abstraction, zero-copy end state,
no HW-render guard), the four milestones (M1 handshake → M2 zero-copy →
M3 lineup + reinit → M4 more backends if needed), sequencing relative
to the Theming + Virtual Library arcs, and risks. This folder records
what's actually implemented vs what's still on paper.

## Integration points (real files)

- `crates/oa-libretro/src/ffi.rs` — env constants present; HW-render +
  Vulkan negotiation structs to add.
- `crates/oa-libretro/src/state.rs` — the `SET_HW_RENDER => false` arm
  (to replace) + `cb_video_refresh` `RETRO_HW_FRAME_BUFFER_VALID`
  sentinel handling.
- `crates/oa-render/src/lib.rs` — wgpu instance/device creation
  (`Backends::PRIMARY` → Vulkan for the HW path) + the new `HwContext`
  trait / `VulkanHwContext` impl.
- `apps/oa-shell/src/main.rs` — `LoadRom` run loop / framebuffer push
  (HW-aware present branch).

## Status

Planning locked 2026-06-07. No code yet. Slotted after VL Phase C3,
before Theming ARC 2 — see the plan's Sequencing section.
