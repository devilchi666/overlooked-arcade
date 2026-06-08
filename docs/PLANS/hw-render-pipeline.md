# HW-Render Pipeline — libretro hardware-rendered cores in OA

Plan locked 2026-06-07 with the operator after the first internal
Dolphin playtest crashed. Branch family: `feat/hw-render-*`.
Feature folder: [features/hw-render/](../features/hw-render/).

## Goal in one line

Teach OA's wgpu renderer to host a libretro core that renders on the
GPU (Dolphin, paraLLEl-N64, Beetle PSX HW, Flycast, PPSSPP, Beetle
Saturn HW, …) instead of handing back a CPU framebuffer — and do it on
a backend abstraction so each core can eventually use the API it runs
best on, the way RetroArch's video drivers do.

## What's broken today (the crash)

OA's libretro layer hard-returns `false` to
`RETRO_ENVIRONMENT_SET_HW_RENDER` (`crates/oa-libretro/src/state.rs`,
the `… | RETRO_ENVIRONMENT_SET_HW_RENDER => false` arm) and always has,
since the first libretro pivot commit `e134fee` (2026-05-18). OA only
ever consumed software framebuffers via `cb_video_refresh`
(`state.rs::cb_video_refresh`, the `pixel::convert` path).

A HW-rendered core is a GPU emulator — it produces no CPU framebuffer.
When OA declines `SET_HW_RENDER`, Dolphin's libretro core falls back to
its **Null** video backend and crashes the moment it's asked to render,
taking OA down with it (the core runs in-process on the emu thread).
Observed in the 2026-06-07 log:

```
env cmd 14 (raw 0xe)   ← RETRO_ENVIRONMENT_SET_HW_RENDER
W[Video]: SetHWRender - failed to set hw renderer for OpenGL Core / 3.0 / ES 3.2 / 3.1 / 3.0
N[Video]: Using GFX backend: Null    ← process dies here
```

This is **architecture, not a regression** — the internal HW path was
never implemented. The fix is to implement it. Per operator decision
(2026-06-07) we do **not** guard/deny HW-render cores; we make them
work.

## Why this matters beyond Dolphin

The same `SET_HW_RENDER` wall blocks every HW-accelerated core OA has
queued. The PSX README already flags it ("Beetle PSX HW needs to obtain
a Vulkan/OpenGL surface from the wgpu host"). One pipeline unlocks:

| System | Core | Best renderer |
|---|---|---|
| GameCube / Wii | Dolphin | Vulkan (also GL, D3D) |
| N64 | paraLLEl-N64 | **paraLLEl-RDP = Vulkan-only** |
| PSX | Beetle PSX HW | **Vulkan** (most accurate; GL is the weaker sibling) |
| Dreamcast | Flycast | Vulkan (also GL, D3D11) |
| PSP | PPSSPP | Vulkan (also GL, D3D11) |
| Saturn | Beetle Saturn HW / Kronos | Vulkan / GL |

The single most accurate N64 renderer (paraLLEl-RDP) and the best PSX
renderer (Beetle PSX HW Vulkan) are **Vulkan-only** — which is why the
abstraction targets Vulkan first (see §Decisions).

## Operator-locked decisions (2026-06-07)

### D1 — Vulkan first, multi-backend abstraction underneath

RetroArch "supports everything" by having a **set of independent video
drivers with exactly one active at a time**, matched to the loaded core
(it answers `GET_PREFERRED_HW_RENDER` with the active driver's API, and
reinitializes the driver when a core needs a different one). It does
**not** run multiple graphics APIs simultaneously — you can't composite
a GL texture and a Vulkan texture in one present without interop.

OA mirrors that model, but starts wgpu's job ahead: wgpu is already a
multi-backend abstraction (Vulkan / DX12 / Metal / GL) and wgpu-hal
exposes the raw handles plus `texture_from_raw` to import a texture
another API produced. So OA builds a thin **`HwContext` trait** with one
impl per backend, exactly one active at a time, on top of what wgpu
already gives — not four renderers from scratch.

**We ship the Vulkan `HwContext` first and only.** One Vulkan bridge
unlocks essentially the whole lineup above at their best renderer. DX12
and GL contexts are added later **only if** a core runs materially
better on them, or operators hit Vulkan-driver problems (operator
decision 2026-06-07 — "roll out Vulkan first, add the others if we find
other cores run better on it or people have issues with Vulkan").

### D2 — Shared-device zero-copy is the end state

The "make OA itself run better" payoff is backend-agnostic and comes
from the shared-device design, not the API: the core shares wgpu's
`VkDevice`/queue, renders into a `VkImage`, and OA imports that image
directly (`wgpu_hal::vulkan::Device::texture_from_raw`) so OA's own
shaders / bezel / scaling sample the core's output on-GPU with no CPU
roundtrip. Milestone 1 may stage through a simpler bridge to prove the
handshake; the destination is zero-copy (Milestone 2).

### D3 — Backend = Vulkan commits wgpu to the Vulkan backend

For a clean shared device, OA's wgpu instance must run on its **Vulkan
backend** so the core and the compositor share one device. Today
`oa-render/lib.rs` creates the instance with `Backends::PRIMARY` (picks
DX12 on most Windows machines). The HW path forces/prefers
`Backends::VULKAN`. Software-only cores upload a CPU buffer and work on
any backend, so they're unaffected by the choice — but the global
backend switch is re-validated against the existing 46 working cores
(see Risks).

### D4 — Capability tiering + fallback, not a hard crash

Probe whether the host's wgpu can stand up a Vulkan device at startup.
If not (weak/absent Vulkan driver, e.g. some integrated GPUs, macOS
without MoltenVK), HW cores fall back to a **software-peer core** where
one exists (PSX / N64 / Saturn all have SW peers) or are surfaced as
unavailable with a clear message — never the silent Null-backend crash.

### D5 — No HW-render guard (explicit operator call)

We do not add a deny-list that refuses HW cores. The catalog may
honestly *label* a core as "needs OA hardware-rendering (in
development)" until M1 lands — labeling is not a guard — but the real
fix is this pipeline.

### D6 — M1 uses a standalone ash device, not the wgpu-shared device

For M1 only, OA stands up a **separate `VkInstance`/`VkDevice` via
`ash`, isolated from wgpu**, purely for the core, and reads the core's
rendered image back to CPU for the existing present path. wgpu is left
entirely alone (stays DX12 on Windows). D3's "commit wgpu to the Vulkan
backend" + true device sharing move to **M2**, alongside the zero-copy
import they enable.

**Why (operator decision 2026-06-08):** M1's goal is to prove the full
libretro HW handshake and get Dolphin actually rendering, at the lowest
risk. The standalone device (a) keeps the 46 working software cores at
zero regression risk (wgpu untouched), and (b) sidesteps the
extension-compatibility wall — the core's negotiation interface gets to
build the device exactly as it needs, instead of being handed wgpu's
pre-made device that may lack required extensions/features. The cost is
~150 lines of standalone device setup thrown away in M2; the reusable
parts (FFI, handshake, the 8 Vulkan interface callbacks, run-loop
wiring) are built here and carry over. Keeps M1 a safely mergeable
milestone that can't break the working renderer.

## Architecture — integration points (real files)

1. **`crates/oa-libretro/src/ffi.rs`** — the env constants already
   exist (`RETRO_ENVIRONMENT_SET_HW_RENDER` 14,
   `GET_HW_RENDER_INTERFACE` 41|EXPERIMENTAL,
   `SET_HW_RENDER_CONTEXT_NEGOTIATION_INTERFACE` 43|EXPERIMENTAL,
   `GET_PREFERRED_HW_RENDER` 56). Add the structs:
   `retro_hw_render_callback`, and for Vulkan
   `retro_hw_render_context_negotiation_interface_vulkan`,
   `retro_vulkan_image`, `retro_hw_render_interface_vulkan`.

2. **`crates/oa-libretro/src/state.rs`** —
   - Replace the `SET_HW_RENDER => false` arm: accept it, store the
     core's `retro_hw_render_callback` (context type + the
     `context_reset` / `context_destroy` / `get_current_framebuffer` /
     `get_proc_address` pointers) in `State`.
   - Answer `GET_PREFERRED_HW_RENDER` (56) with the active backend's
     context type (Vulkan).
   - Answer `GET_HW_RENDER_INTERFACE` (41) with the
     `retro_hw_render_interface_vulkan` carrying our VkInstance /
     PhysicalDevice / Device / queue + the frontend callbacks
     (`set_image`, `get_sync_index`, `set_command_buffers`, …).
   - `cb_video_refresh`: special-case `data == RETRO_HW_FRAME_BUFFER_VALID`
     (the `(void*)-1` sentinel) — do NOT run `pixel::convert` (it would
     deref `0xFFFF…`); instead mark "HW frame ready" so the run loop
     pulls from the imported texture. NOTE: this sentinel is also a
     latent crash today (current null-check passes `-1`), fixed here.

3. **`crates/oa-render/src/lib.rs`** — the wgpu instance/device
   (`new_async`, the `Instance::new(... Backends::PRIMARY ...)` +
   `request_adapter` + `request_device` block). Add:
   - Backend selection (Vulkan for the HW path).
   - A `HwContext` trait + `VulkanHwContext` impl that extracts the
     ash handles via `wgpu_hal`, hands them to the libretro layer, and
     imports the core's `VkImage` each frame for compositing.

4. **`apps/oa-shell/src/main.rs`** — the `EmuCommand::LoadRom` handler
   + the per-frame run loop (the `core.run_frame()` → framebuffer push
   region, distinct from VL Phase C's launcher-dispatch region at the
   top of the same handler). Branch the present path: software core →
   today's CPU-upload; HW core → composite the imported texture.

## Milestones

### M1 — Vulkan context bring-up + handshake, core on screen

**Device strategy (operator decision 2026-06-08, D6): standalone ash
device for M1, not the wgpu-shared device.** See D6 below.

- ffi structs + `state.rs` handshake (D-section items 1-2).
- A **standalone `VkInstance`/`VkDevice` (via `ash`), isolated from
  wgpu**, stood up in `oa-libretro` purely for the core and built per
  the core's negotiation interface. `oa-render`/wgpu is NOT touched in
  M1 — it stays on its current backend (DX12 on Windows), so the 46
  working software cores are at zero regression risk.
- The 8 Vulkan HW-interface callbacks (`set_image`, `get_sync_index`,
  …) + the run-loop wiring — these are the reusable core, carried into
  M2 unchanged.
- Present bridge: after the core renders into the `VkImage` it hands us
  via `set_image`, copy/readback to a host-visible buffer and feed the
  existing `fb_rgba` CPU path — so wgpu presents it exactly like a
  software core's frame. Goal is a **frame of Dolphin on screen**,
  proving negotiation + `context_reset` + `get_proc_address` + the
  run-loop end-to-end.
- Throwaway in M2: only the standalone instance/device creation
  (~150 lines) is replaced by the wgpu-shared device. The FFI,
  handshake, interface callbacks, and run-loop wiring all carry over.
- **Exit:** the operator's GameCube game renders through the internal
  `dolphin_libretro` core instead of crashing.

### M2 — Zero-copy import + on-GPU compositing

- Replace the M1 bridge with direct `texture_from_raw` import of the
  core's `VkImage`; wire `set_image` / `get_sync_index` synchronization
  (semaphores, queue ownership, layout transitions).
- OA's compositor samples the core texture directly — no CPU roundtrip.
- **Exit:** Dolphin at full speed with OA's scaling/bezel/shader stack
  applied on-GPU; measurable win vs M1 readback.

### M3 — Backend-matching + reinit-on-core-switch + capability tiering

- Capability probe at startup (D4); software-peer fallback wiring.
- RetroArch-style renderer reinit when a loaded HW core needs a
  backend different from the live one (only fires for HW cores — SW
  cores are backend-agnostic).
- Validate the full HW lineup beyond Dolphin (paraLLEl-N64 Vulkan-RDP,
  Beetle PSX HW Vulkan, Flycast, PPSSPP, Beetle Saturn HW).
- Per-core ROADMAP `⬜→✅` flips for each system's HW renderer.

### M4 — Additional backends (only if needed) + cross-platform

- DX12 and/or GL `HwContext` impls behind the same trait — added
  **only** when a core runs materially better on them or Vulkan-driver
  issues surface in the field (per D1).
- Cross-platform: MoltenVK (Vulkan→Metal) on macOS; GL fallback tier.

## Sequencing relative to in-flight arcs

HW-render is a **Rust engine-layer arc** (`oa-render` + `oa-libretro` +
`main.rs` run loop). The two big in-flight arcs touch different layers:

- **Theming Substrate ARC 1** (current resume target) is pure
  frontend/TS — zero overlap. Its only renderer work is **ARC 2 (WGSL
  shader hooks)**, parked far out behind ARC 1 (itself gated on VL
  Phase E + C).
- **Virtual Library** (Phases C/D/F/G) is schema + frontend + launcher
  *lifecycle* (the `Launcher` trait sits above `oa_core::Core`); it
  never touches frame production.

**The one ordering constraint:** HW-render lands **before Theming ARC 2
(WGSL)** so shader hooks build on a renderer that already understands
GPU-resident core textures (otherwise ARC 2 assumes CPU framebuffers
and gets rewritten).

**The one coordination point:** both VL Phase C and HW-render edit
`main.rs`'s `LoadRom` handler — different regions (C = launcher
dispatch at the top; HW = run-loop / framebuffer push lower down). Let
**VL Phase C3 land first** to avoid a double-edit, and because C3 +
HW-render together complete the GameCube launch story (external +
internal).

**Recommended slot:** after VL Phase C3, before Theming ARC 2. Theming
ARC 1's frontend resume can interleave safely (different layer).

```
VL Phase C3  →  HW-RENDER (M1 Vulkan → M2 zero-copy → M3 lineup)  →  Theming ARC 2 (WGSL) builds on it
                 (Theming ARC 1 frontend resume interleaves safely)
```

## Risks

- **R1 (HIGH) — global Vulkan backend switch regresses working cores.**
  Moving wgpu from DX12 (Windows default) to Vulkan re-validates the 46
  software cores' present path. Bounded (CPU-upload path is
  backend-agnostic; risk is shader/format edge cases), but every
  shipped system gets a smoke-test pass before merge.
- **R2 (HIGH) — Vulkan sync correctness.** The libretro Vulkan HW
  interface (semaphores, queue-family ownership, image layout
  transitions, `get_sync_index` frame pacing) is notoriously fiddly.
  M1's readback bridge de-risks by proving the handshake before the
  sync work in M2.
- **R3 (MEDIUM) — wgpu-hal raw-handle API churn.** Extracting ash
  handles via `wgpu_hal` and importing with `texture_from_raw` depends
  on wgpu-hal internals that move between versions. Pin wgpu; isolate
  the unsafe interop in `VulkanHwContext`.
- **R4 (MEDIUM) — host Vulkan availability.** Weak/absent Vulkan
  drivers (some integrated GPUs; macOS without MoltenVK). D4's
  capability tiering + software-peer fallback is the mitigation.
- **R5 (LOW) — wgpu is DX12, not DX11.** A D3D11-only core (rare; most
  multi-API cores also do D3D12 or Vulkan) would need a D3D11↔D3D12
  bridge. Not in scope — those cores use their Vulkan/GL path.

## Exit criteria

- Internal `dolphin_libretro` renders a GameCube game in-process
  instead of crashing (M1).
- Zero-copy: core output composited on-GPU through OA's existing
  scaling/bezel/shader stack with no CPU readback (M2).
- The Vulkan-best lineup validated beyond Dolphin (M3).
- Software cores unaffected on the Vulkan backend; HW-unavailable hosts
  fall back cleanly, never crash (D4).
