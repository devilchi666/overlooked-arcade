//! Vulkan HW-render bring-up for libretro hardware cores (Dolphin,
//! paraLLEl-N64, Beetle PSX HW, …). See docs/PLANS/hw-render-pipeline.md.
//!
//! **M1 (D6): standalone `ash` device + CPU readback.** We stand up a
//! `VkInstance`/`VkDevice` here that is completely isolated from wgpu, hand
//! it to the core via the libretro Vulkan HW-render interface, let the core
//! render into a `VkImage`, then copy that image back to a host-visible
//! buffer and feed the existing `State.fb_rgba` software-framebuffer path.
//! wgpu is left untouched. Zero-copy + a wgpu-shared device are M2.
//!
//! The 2026-06-08 probe run (oa-current.log) showed the operator's Dolphin
//! .dll does NOT register a context-negotiation interface (it only would
//! after we accept a HW context, and even then this build doesn't provide
//! `create_device`). So M1 **self-builds** the device with all available
//! device extensions + features enabled — the universal path. The core
//! consumes whatever device we hand it via `retro_hw_render_interface_vulkan`.

use std::ffi::{c_void, CStr};
use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use ash::vk;

use crate::ffi::*;

// ============================ probe helpers ============================
// (Kept from the probe commit — still called from the HW env handlers to
//  log what a core requests + which GPU we select.)

/// Log the Vulkan context-negotiation interface a core handed us via env 43.
pub(crate) fn log_negotiation_interface(
    iface: &retro_hw_render_context_negotiation_interface_vulkan,
) {
    log::info!(
        "oa-libretro HW: negotiation iface — type={} version={} (we implement v{}); \
         callbacks: get_application_info={} create_device(v1)={} destroy_device={} \
         create_instance(v2)={} create_device2(v2)={}",
        iface.interface_type,
        iface.interface_version,
        RETRO_HW_RENDER_CONTEXT_NEGOTIATION_INTERFACE_VULKAN_VERSION,
        iface.get_application_info.is_some(),
        iface.create_device.is_some(),
        iface.destroy_device.is_some(),
        iface.create_instance.is_some(),
        iface.create_device2.is_some(),
    );
    let Some(get_app) = iface.get_application_info else {
        log::info!(
            "oa-libretro HW: core provides no get_application_info — instance defaults to Vulkan 1.1"
        );
        return;
    };
    // SAFETY: core-owned fn pointer; returns its static VkApplicationInfo or NULL.
    let app_ptr = unsafe { get_app() };
    if app_ptr.is_null() {
        log::info!("oa-libretro HW: get_application_info returned NULL (no apiVersion preference)");
        return;
    }
    // SAFETY: non-null per the check; lifetime is the core's (static).
    let app = unsafe { &*app_ptr };
    let api = app.api_version;
    let name = unsafe { read_cstr_opt(app.p_application_name) }.unwrap_or_else(|| "<none>".into());
    let engine = unsafe { read_cstr_opt(app.p_engine_name) }.unwrap_or_else(|| "<none>".into());
    if api == 0 {
        log::info!("oa-libretro HW: core apiVersion=0 (Vulkan 1.0) — app=\"{name}\" engine=\"{engine}\"");
    } else {
        log::info!(
            "oa-libretro HW: core wants Vulkan apiVersion {}.{}.{} — app=\"{name}\" engine=\"{engine}\"",
            vk::api_version_major(api),
            vk::api_version_minor(api),
            vk::api_version_patch(api),
        );
    }
}

/// Enumerate the host's Vulkan physical devices and log each, reporting
/// which one M1 will select (first discrete GPU — the dedicated card).
/// Throwaway instance, destroyed before return; diagnostic only.
pub(crate) fn probe_physical_devices() {
    // SAFETY: loads the system Vulkan loader (vulkan-1.dll on Windows).
    let entry = match unsafe { ash::Entry::load() } {
        Ok(e) => e,
        Err(e) => {
            log::error!("oa-libretro HW: failed to load the Vulkan loader ({e}) — no usable Vulkan runtime");
            return;
        }
    };
    let app_info = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_1);
    let create_info = vk::InstanceCreateInfo::default().application_info(&app_info);
    // SAFETY: create_info valid for the call; no extensions/layers requested.
    let instance = match unsafe { entry.create_instance(&create_info, None) } {
        Ok(i) => i,
        Err(e) => {
            log::error!("oa-libretro HW: vkCreateInstance failed: {e:?}");
            return;
        }
    };
    // SAFETY: instance live until destroy_instance below.
    let pds = match unsafe { instance.enumerate_physical_devices() } {
        Ok(p) => p,
        Err(e) => {
            log::error!("oa-libretro HW: vkEnumeratePhysicalDevices failed: {e:?}");
            unsafe { instance.destroy_instance(None) };
            return;
        }
    };
    log::info!("oa-libretro HW: {} Vulkan physical device(s):", pds.len());
    let mut selected: Option<(usize, String)> = None;
    for (i, &pd) in pds.iter().enumerate() {
        let props = unsafe { instance.get_physical_device_properties(pd) };
        let name = unsafe { CStr::from_ptr(props.device_name.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        let type_str = device_type_str(props.device_type);
        let api = props.api_version;
        log::info!(
            "oa-libretro HW:   [{i}] {name} ({type_str}, Vulkan {}.{}.{})",
            vk::api_version_major(api),
            vk::api_version_minor(api),
            vk::api_version_patch(api),
        );
        if props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU && selected.is_none() {
            selected = Some((i, name));
        }
    }
    match selected {
        Some((i, name)) => log::info!(
            "oa-libretro HW: M1 will select [{i}] {name} (first discrete GPU — the dedicated card)"
        ),
        None => {
            if let Some(&pd) = pds.first() {
                let props = unsafe { instance.get_physical_device_properties(pd) };
                let name = unsafe { CStr::from_ptr(props.device_name.as_ptr()) }
                    .to_string_lossy()
                    .into_owned();
                log::warn!("oa-libretro HW: no discrete GPU — M1 falls back to [0] {name}");
            } else {
                log::error!("oa-libretro HW: zero Vulkan physical devices — HW cores cannot run here");
            }
        }
    }
    // SAFETY: no child objects outlive the instance (we only read props).
    unsafe { instance.destroy_instance(None) };
}

fn device_type_str(t: vk::PhysicalDeviceType) -> &'static str {
    match t {
        vk::PhysicalDeviceType::DISCRETE_GPU => "discrete",
        vk::PhysicalDeviceType::INTEGRATED_GPU => "integrated",
        vk::PhysicalDeviceType::VIRTUAL_GPU => "virtual",
        vk::PhysicalDeviceType::CPU => "cpu",
        _ => "other",
    }
}

/// # Safety
/// `p` must be NULL or a valid NUL-terminated C string.
unsafe fn read_cstr_opt(p: *const c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    Some(unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
}

// ============================ device build ============================

/// A tiny spin-lock backing the libretro `lock_queue` / `unlock_queue`
/// contract. The core wraps every `vkQueueSubmit` on our queue with this
/// pair (it may submit from background threads — Dolphin compiles shaders
/// async). `lock_queue`/`unlock_queue` are always paired on the SAME
/// thread, so a non-reentrant binary lock suffices; we spin+yield rather
/// than carry a std `MutexGuard` across the FFI boundary (which is `!Send`
/// and can't live in a `Sync` field). Our own readback submit takes the
/// same lock so it never races the core's submits.
struct QueueLock(AtomicBool);
impl QueueLock {
    const fn new() -> Self {
        Self(AtomicBool::new(false))
    }
    fn lock(&self) {
        while self.0.swap(true, Ordering::Acquire) {
            std::thread::yield_now();
        }
    }
    fn unlock(&self) {
        self.0.store(false, Ordering::Release);
    }
}

/// The most recent frame the core handed us via `set_image` (+ the extent
/// learned from the `video_refresh` sentinel call). Consumed by the
/// per-frame readback.
struct FrameImage {
    image: vk::Image,
    layout: vk::ImageLayout,
    format: vk::Format,
    /// Semaphores the core signals when the image is ready; we must wait
    /// on them before reading the image back.
    wait_sems: Vec<vk::Semaphore>,
    /// Extent, filled from the `video_refresh` sentinel call (not carried
    /// by `set_image`).
    width: u32,
    height: u32,
    /// Set true once `video_refresh(RETRO_HW_FRAME_BUFFER_VALID, …)` lands
    /// for this image — readback only fires for a ready frame.
    ready: bool,
}

/// Readback GPU resources, reused across frames. Behind a `Mutex` because
/// `readback_into` takes `&self` (the interface handle aliases the whole
/// struct across threads).
struct Readback {
    cmd_pool: vk::CommandPool,
    cmd_buf: vk::CommandBuffer,
    fence: vk::Fence,
    /// Host-visible staging buffer; (re)allocated when a frame needs more.
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    cap: vk::DeviceSize,
}

/// Standalone Vulkan device the core renders on (M1, isolated from wgpu).
/// Boxed and stored in `State`; the libretro interface `handle` points at
/// this allocation so the 8 frontend callbacks reach it without touching
/// the STATE mutex (avoids re-entrancy deadlock with `retro_run`).
pub(crate) struct VulkanHw {
    entry: ash::Entry,
    instance: ash::Instance,
    device: ash::Device,
    queue: vk::Queue,
    mem_props: vk::PhysicalDeviceMemoryProperties,
    queue_lock: QueueLock,
    frame: Mutex<Option<FrameImage>>,
    /// Semaphore the core asked us to signal when we're done with the image
    /// (`set_signal_semaphore`); `null` when none set this frame.
    signal_sem: Mutex<vk::Semaphore>,
    readback: Mutex<Readback>,
    /// The interface struct we hand the core (env 41). Its `handle` is set
    /// to point back at this `VulkanHw` after boxing (see `finalize_handle`).
    interface: retro_hw_render_interface_vulkan,
    /// Core renders bottom-left origin (Dolphin reports this) → flip on
    /// readback so the image is top-left for our renderer.
    flip_y: bool,
    /// Device extensions we required when building the device (the list we
    /// pass to `create_device` / enable on self-build). M2 hands this to
    /// wgpu's `device_from_raw` so it uses the same enabled set. Empty in
    /// M1 (we required none); populated once M2 requests VK_KHR_swapchain +
    /// wgpu's needed extensions.
    device_extensions: Vec<String>,
    /// Vulkan instance apiVersion we created the instance with (for wgpu
    /// `Instance::from_raw` when M2 adopts this device).
    api_version: u32,
}

/// Raw handles for adopting the core's Vulkan device into the renderer (M2
/// zero-copy). All handles are raw `u64` (`vk::Handle::as_raw`) so this
/// type carries no `ash` dependency across the crate boundary — the
/// renderer reconstructs the `vk::*` types. None are owned here; the
/// `VulkanHw` (in `State`) owns the lifetimes.
#[derive(Clone, Debug)]
pub struct LoadedHwVulkan {
    pub instance: u64,
    pub physical_device: u64,
    pub device: u64,
    pub queue: u64,
    pub queue_family_index: u32,
    pub queue_index: u32,
    /// Device extensions enabled on the core's device (so the renderer can
    /// tell wgpu which to use in `device_from_raw`).
    pub device_extensions: Vec<String>,
    /// Vulkan instance apiVersion.
    pub api_version: u32,
}

// SAFETY: ash Entry/Instance/Device + vk handles are Send+Sync; the only
// non-auto-Send fields are the raw `handle`/fn pointers in `interface`,
// which are immutable after `finalize_handle`. All per-frame mutation goes
// through the internal locks. The core uses the documented same-thread
// lock_queue pairing.
unsafe impl Send for VulkanHw {}
unsafe impl Sync for VulkanHw {}

/// Instance-level Vulkan resources, created when we accept a Vulkan
/// `SET_HW_RENDER` — *before* we know whether the core will build the
/// device for us (negotiation `create_device`) or we self-build it. Held in
/// `State` until the device is finalized into a [`VulkanHw`].
///
/// The split matters: the libretro Vulkan contract is that the FRONTEND
/// creates the instance, then the CORE builds the device on it via
/// `create_device`. Doing so tells the core the frontend is driving Vulkan
/// (so it renders headless into images for us) instead of spinning up its
/// own context + swapchain — which is exactly the deadlock the first
/// device-build hit (Dolphin tried to own a swapchain with no window).
pub(crate) struct VulkanInstance {
    entry: ash::Entry,
    instance: ash::Instance,
    phys: vk::PhysicalDevice,
    flip_y: bool,
}

impl VulkanInstance {
    /// Create the `VkInstance` (Vulkan 1.1) + select the first discrete GPU
    /// (the dedicated card). Done at `SET_HW_RENDER` accept time.
    pub(crate) fn create(flip_y: bool) -> Result<VulkanInstance, String> {
        // SAFETY: loads the Vulkan loader.
        let entry = unsafe { ash::Entry::load() }
            .map_err(|e| format!("Vulkan loader unavailable: {e}"))?;
        let app_info = vk::ApplicationInfo::default()
            .application_name(c"Overlooked Arcade")
            .api_version(vk::API_VERSION_1_1);

        // Enable the WSI/surface INSTANCE extensions, matching RetroArch's
        // frontend instance. HW cores (Dolphin) create a surface + swapchain
        // internally (headless/fake — they render into images they hand us
        // via set_image, never to a real window) and dereference the
        // vkCreate*SurfaceKHR / swapchain entry points. With a bare instance
        // those resolve to NULL and the core crashes during its OWN context
        // init, right after it selects the Vulkan backend and before it ever
        // queries our render interface (observed 2026-06-08). Add each only
        // if the loader reports it available. Verified against RetroArch
        // gfx/common/vulkan_common.c (vulkan_context_create_instance_wrapper).
        let avail = unsafe { entry.enumerate_instance_extension_properties(None) }
            .unwrap_or_default();
        let has = |name: &CStr| {
            avail
                .iter()
                .any(|e| unsafe { CStr::from_ptr(e.extension_name.as_ptr()) } == name)
        };
        let wanted: [&CStr; 4] = [
            ash::khr::surface::NAME,
            ash::khr::win32_surface::NAME,
            ash::khr::get_physical_device_properties2::NAME,
            ash::khr::get_surface_capabilities2::NAME,
        ];
        let ext_ptrs: Vec<*const c_char> =
            wanted.iter().copied().filter(|n| has(n)).map(|n| n.as_ptr()).collect();
        log::info!(
            "oa-libretro HW: enabling {} instance extension(s): {}",
            ext_ptrs.len(),
            wanted
                .iter()
                .filter(|n| has(n))
                .map(|n| n.to_string_lossy())
                .collect::<Vec<_>>()
                .join(", "),
        );
        let instance_ci = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&ext_ptrs);
        // SAFETY: instance_ci + ext_ptrs valid for the call.
        let instance = unsafe { entry.create_instance(&instance_ci, None) }
            .map_err(|e| format!("vkCreateInstance failed: {e:?}"))?;

        let pds = match unsafe { instance.enumerate_physical_devices() } {
            Ok(p) if !p.is_empty() => p,
            Ok(_) => {
                unsafe { instance.destroy_instance(None) };
                return Err("no Vulkan physical devices".into());
            }
            Err(e) => {
                unsafe { instance.destroy_instance(None) };
                return Err(format!("enumerate_physical_devices failed: {e:?}"));
            }
        };
        let phys = pds
            .iter()
            .copied()
            .find(|&pd| {
                unsafe { instance.get_physical_device_properties(pd) }.device_type
                    == vk::PhysicalDeviceType::DISCRETE_GPU
            })
            .unwrap_or(pds[0]);
        let props = unsafe { instance.get_physical_device_properties(phys) };
        let name = unsafe { CStr::from_ptr(props.device_name.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        log::info!(
            "oa-libretro HW: instance up; selected GPU {name} ({})",
            device_type_str(props.device_type)
        );
        Ok(VulkanInstance {
            entry,
            instance,
            phys,
            flip_y,
        })
    }

    /// Ask the core to build the device on our instance + GPU (libretro
    /// Vulkan negotiation v1, the canonical path). Non-consuming: returns
    /// the wrapped device + queue on success, `None` on failure (so the
    /// caller can fall back to self-build with the instance intact).
    pub(crate) fn try_create_device(
        &self,
        create_device: retro_vulkan_create_device_t,
    ) -> Option<(ash::Device, vk::Queue, u32, vk::PhysicalDevice)> {
        let mut ctx = retro_vulkan_context {
            gpu: vk::PhysicalDevice::null(),
            device: vk::Device::null(),
            queue: vk::Queue::null(),
            queue_family_index: 0,
            presentation_queue: vk::Queue::null(),
            presentation_queue_family_index: 0,
        };
        let gipa = self.entry.static_fn().get_instance_proc_addr;
        // Pass a ZEROED VkPhysicalDeviceFeatures, not NULL. The libretro
        // spec permits NULL ("no required features"), but Granite (the
        // engine behind paraLLEl-RDP) dereferences this pointer inside
        // create_device to merge the frontend's required features into the
        // device it builds — NULL faults. Same null-deref class as the
        // GET_GAME_INFO_EXT strings (see reference memory); RetroArch also
        // passes a real (zeroed) features struct here. Must outlive the call.
        let required_features = vk::PhysicalDeviceFeatures::default();
        // M2: require VK_KHR_swapchain on the device the core builds, so the
        // renderer (wgpu, adopting this device) can present to our window.
        // M1 passed none (readback didn't present on this device). The core
        // enables this PLUS whatever it needs (e.g. VK_EXT_external_memory_host).
        let mut req_ext_ptrs: [*const c_char; 1] = [ash::khr::swapchain::NAME.as_ptr()];
        log::info!(
            "oa-libretro HW: calling core create_device (gpu set, surface=null, features=zeroed, require=[VK_KHR_swapchain])…"
        );
        // SAFETY: core-provided fn. We pass our live instance + GPU, a null
        // surface (headless — the core renders into images for us, never to a
        // window), our required device extensions ([swapchain]), no layers,
        // and a zeroed required-features struct. The core fills `ctx`.
        let ok = unsafe {
            create_device(
                &mut ctx,
                self.instance.handle(),
                self.phys,
                vk::SurfaceKHR::null(),
                gipa,
                req_ext_ptrs.as_mut_ptr(),
                req_ext_ptrs.len() as u32,
                std::ptr::null_mut(),
                0,
                &required_features,
            )
        };
        log::info!(
            "oa-libretro HW: core create_device returned ok={ok} device_null={}",
            ctx.device == vk::Device::null(),
        );
        if !ok || ctx.device == vk::Device::null() {
            log::warn!("oa-libretro HW: core create_device returned false / null device");
            return None;
        }
        let gpu = if ctx.gpu != vk::PhysicalDevice::null() {
            ctx.gpu
        } else {
            self.phys
        };
        log::info!(
            "oa-libretro HW: core built device via create_device (queue_family={})",
            ctx.queue_family_index
        );
        // Wrap the core-created device handle with ash (loads device fns via
        // our instance's vkGetDeviceProcAddr).
        // SAFETY: ctx.device is a valid device created on our instance.
        let device = unsafe { ash::Device::load(self.instance.fp_v1_0(), ctx.device) };
        Some((device, ctx.queue, ctx.queue_family_index, gpu))
    }

    /// Self-build the device (fallback when the core provides no negotiation
    /// `create_device`). Enables all available device extensions + features.
    /// Consuming; destroys the instance on failure.
    pub(crate) fn into_hw_self_build(self) -> Result<VulkanHw, String> {
        let qfps = unsafe {
            self.instance
                .get_physical_device_queue_family_properties(self.phys)
        };
        let queue_family = match qfps
            .iter()
            .position(|q| q.queue_flags.contains(vk::QueueFlags::GRAPHICS))
        {
            Some(i) => i as u32,
            None => {
                unsafe { self.instance.destroy_instance(None) };
                return Err("no graphics queue family".into());
            }
        };
        let ext_props =
            match unsafe { self.instance.enumerate_device_extension_properties(self.phys) } {
                Ok(p) => p,
                Err(e) => {
                    unsafe { self.instance.destroy_instance(None) };
                    return Err(format!("enumerate_device_extension_properties failed: {e:?}"));
                }
            };
        let ext_ptrs: Vec<*const c_char> =
            ext_props.iter().map(|p| p.extension_name.as_ptr()).collect();
        let features = unsafe { self.instance.get_physical_device_features(self.phys) };
        let priorities = [1.0f32];
        let queue_ci = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family)
            .queue_priorities(&priorities)];
        let device = {
            let dci = vk::DeviceCreateInfo::default()
                .queue_create_infos(&queue_ci)
                .enabled_extension_names(&ext_ptrs)
                .enabled_features(&features);
            match unsafe { self.instance.create_device(self.phys, &dci, None) } {
                Ok(d) => {
                    log::info!("oa-libretro HW: self-built device with {} extension(s)", ext_ptrs.len());
                    d
                }
                Err(e) => {
                    log::warn!("oa-libretro HW: self-build w/ all extensions failed ({e:?}); retrying bare");
                    let dci = vk::DeviceCreateInfo::default()
                        .queue_create_infos(&queue_ci)
                        .enabled_features(&features);
                    match unsafe { self.instance.create_device(self.phys, &dci, None) } {
                        Ok(d) => d,
                        Err(e) => {
                            unsafe { self.instance.destroy_instance(None) };
                            return Err(format!("vkCreateDevice failed: {e:?}"));
                        }
                    }
                }
            }
        };
        let queue = unsafe { device.get_device_queue(queue_family, 0) };
        let phys = self.phys;
        self.finalize(device, queue, queue_family, phys)
    }

    /// Build readback resources + the interface and assemble the
    /// [`VulkanHw`]. Consuming. On failure, destroys the device + instance.
    pub(crate) fn finalize(
        self,
        device: ash::Device,
        queue: vk::Queue,
        queue_family: u32,
        gpu: vk::PhysicalDevice,
    ) -> Result<VulkanHw, String> {
        let mem_props = unsafe { self.instance.get_physical_device_memory_properties(gpu) };
        let (cmd_pool, cmd_buf, fence, interface) =
            match build_resources(&self.entry, &self.instance, &device, gpu, queue, queue_family) {
                Ok(v) => v,
                Err(e) => {
                    // SAFETY: device + instance live; nothing references them yet.
                    unsafe {
                        device.destroy_device(None);
                        self.instance.destroy_instance(None);
                    }
                    return Err(e);
                }
            };
        Ok(VulkanHw {
            entry: self.entry,
            instance: self.instance,
            device,
            queue,
            mem_props,
            queue_lock: QueueLock::new(),
            frame: Mutex::new(None),
            signal_sem: Mutex::new(vk::Semaphore::null()),
            readback: Mutex::new(Readback {
                cmd_pool,
                cmd_buf,
                fence,
                buffer: vk::Buffer::null(),
                memory: vk::DeviceMemory::null(),
                cap: 0,
            }),
            interface,
            flip_y: self.flip_y,
            // VK_KHR_swapchain is enabled on both device paths (required of
            // the create_device path; included in the self-build path's
            // all-extensions set). The renderer hands this to wgpu's
            // device_from_raw as the extension it will use beyond Vulkan-1.1
            // core. Instance is created at Vulkan 1.1.
            device_extensions: vec![
                ash::khr::swapchain::NAME.to_string_lossy().into_owned(),
            ],
            api_version: vk::API_VERSION_1_1,
        })
    }

    /// Destroy the instance without finalizing a device — for cleanup when a
    /// HW core was accepted but no device was ever built (e.g. load failed
    /// before `context_reset`).
    pub(crate) fn destroy(self) {
        // SAFETY: instance live; no children created.
        unsafe { self.instance.destroy_instance(None) };
    }
}

impl VulkanHw {
    /// Point the interface `handle` back at this (now-boxed, address-stable)
    /// allocation. MUST be called after the `VulkanHw` is in its final
    /// location (the `State` box). Returns a `*const` to the interface for
    /// the env-41 reply.
    pub(crate) fn finalize_handle(&mut self) {
        self.interface.handle = self as *mut VulkanHw as *mut c_void;
    }

    /// Pointer to our persistent interface struct, for the
    /// GET_HW_RENDER_INTERFACE reply (`*data = this`).
    pub(crate) fn interface_ptr(&self) -> *const retro_hw_render_interface_vulkan {
        &self.interface
    }

    /// Raw handles for the renderer to adopt this device (M2 zero-copy).
    /// All from the interface we built — instance/gpu/device/queue +
    /// queue family (the interface's `queue_index` field holds the family
    /// index per libretro_vulkan.h), plus the required device-extension
    /// list + instance apiVersion wgpu needs for `from_raw`.
    pub(crate) fn adopted_handles(&self) -> LoadedHwVulkan {
        use ash::vk::Handle;
        LoadedHwVulkan {
            instance: self.interface.instance.as_raw(),
            physical_device: self.interface.gpu.as_raw(),
            device: self.interface.device.as_raw(),
            queue: self.interface.queue.as_raw(),
            queue_family_index: self.interface.queue_index,
            queue_index: 0,
            device_extensions: self.device_extensions.clone(),
            api_version: self.api_version,
        }
    }

    /// Record the extent + mark the most-recent `set_image` frame ready.
    /// Called from `cb_video_refresh` when the HW sentinel lands.
    pub(crate) fn mark_frame_ready(&self, width: u32, height: u32) {
        if let Ok(mut g) = self.frame.lock() {
            if let Some(f) = g.as_mut() {
                f.width = width;
                f.height = height;
                f.ready = true;
            }
        }
    }

    /// Copy the core's rendered `VkImage` back to CPU and convert into
    /// `fb_rgba` (R,G,B,A; alpha forced opaque), flipping vertically when
    /// the core renders bottom-left origin. Updates `fb_w`/`fb_h`. No-op
    /// when no ready frame is pending. Fully synchronous (M1): submit →
    /// fence wait → map → convert.
    pub(crate) fn readback_into(
        &self,
        fb_rgba: &mut Vec<u8>,
        fb_w: &mut u32,
        fb_h: &mut u32,
    ) -> Result<(), String> {
        // Hold the frame lock across the GPU work so a concurrent set_image
        // can't swap the image mid-readback (M1 is synchronous anyway).
        let mut fg = self.frame.lock().map_err(|_| "frame lock poisoned".to_string())?;
        let Some(frame) = fg.as_mut() else {
            return Ok(());
        };
        if !frame.ready || frame.width == 0 || frame.height == 0 {
            return Ok(());
        }
        if frame.image == vk::Image::null() {
            frame.ready = false;
            return Ok(());
        }
        let (w, h) = (frame.width, frame.height);
        let needed = (w as vk::DeviceSize) * (h as vk::DeviceSize) * 4;

        let signal = {
            let mut s = self.signal_sem.lock().map_err(|_| "sig lock".to_string())?;
            std::mem::replace(&mut *s, vk::Semaphore::null())
        };

        let mut rb = self.readback.lock().map_err(|_| "readback lock".to_string())?;
        self.ensure_buffer(&mut rb, needed)?;

        let dev = &self.device;
        // SAFETY: all handles belong to `dev`; recorded + submitted below,
        // then host-waited on the fence before we read the mapped memory.
        unsafe {
            dev.reset_command_buffer(rb.cmd_buf, vk::CommandBufferResetFlags::empty())
                .map_err(|e| format!("reset_command_buffer: {e:?}"))?;
            let begin = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            dev.begin_command_buffer(rb.cmd_buf, &begin)
                .map_err(|e| format!("begin_command_buffer: {e:?}"))?;

            let range = vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);
            let to_src = vk::ImageMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
                .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
                .old_layout(frame.layout)
                .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(frame.image)
                .subresource_range(range);
            dev.cmd_pipeline_barrier(
                rb.cmd_buf,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[to_src],
            );

            let subres = vk::ImageSubresourceLayers::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .mip_level(0)
                .base_array_layer(0)
                .layer_count(1);
            let region = vk::BufferImageCopy::default()
                .buffer_offset(0)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_subresource(subres)
                .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                .image_extent(vk::Extent3D { width: w, height: h, depth: 1 });
            dev.cmd_copy_image_to_buffer(
                rb.cmd_buf,
                frame.image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                rb.buffer,
                &[region],
            );

            let back = vk::ImageMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::TRANSFER_READ)
                .dst_access_mask(vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE)
                .old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                .new_layout(frame.layout)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(frame.image)
                .subresource_range(range);
            dev.cmd_pipeline_barrier(
                rb.cmd_buf,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[back],
            );

            dev.end_command_buffer(rb.cmd_buf)
                .map_err(|e| format!("end_command_buffer: {e:?}"))?;

            // Submit (under the queue lock the core also honors). Wait on the
            // core's image-ready semaphores; signal the core's done-semaphore.
            let cmd_bufs = [rb.cmd_buf];
            let wait_stages: Vec<vk::PipelineStageFlags> =
                vec![vk::PipelineStageFlags::ALL_COMMANDS; frame.wait_sems.len()];
            let sig = [signal];
            let mut submit = vk::SubmitInfo::default().command_buffers(&cmd_bufs);
            if !frame.wait_sems.is_empty() {
                submit = submit
                    .wait_semaphores(&frame.wait_sems)
                    .wait_dst_stage_mask(&wait_stages);
            }
            if signal != vk::Semaphore::null() {
                submit = submit.signal_semaphores(&sig);
            }
            self.queue_lock.lock();
            let submit_res = dev.queue_submit(self.queue, &[submit], rb.fence);
            self.queue_lock.unlock();
            submit_res.map_err(|e| format!("queue_submit: {e:?}"))?;

            dev.wait_for_fences(&[rb.fence], true, u64::MAX)
                .map_err(|e| format!("wait_for_fences: {e:?}"))?;
            dev.reset_fences(&[rb.fence])
                .map_err(|e| format!("reset_fences: {e:?}"))?;

            // Map + convert.
            let ptr = dev
                .map_memory(rb.memory, 0, needed, vk::MemoryMapFlags::empty())
                .map_err(|e| format!("map_memory: {e:?}"))? as *const u8;
            let src = std::slice::from_raw_parts(ptr, needed as usize);
            convert_into(src, w, h, self.flip_y, frame.format, fb_rgba);
            dev.unmap_memory(rb.memory);
        }

        // One-shot: confirms the full readback path works end-to-end.
        static FIRST_READBACK: AtomicBool = AtomicBool::new(false);
        if !FIRST_READBACK.swap(true, Ordering::Relaxed) {
            log::info!(
                "oa-libretro HW: first readback OK — {w}x{h} format={:?} flip_y={} (frame on screen)",
                frame.format,
                self.flip_y,
            );
        }

        *fb_w = w;
        *fb_h = h;
        frame.ready = false;
        Ok(())
    }

    /// (Re)allocate the host-visible staging buffer when a frame needs more
    /// than the current capacity.
    fn ensure_buffer(&self, rb: &mut Readback, needed: vk::DeviceSize) -> Result<(), String> {
        if rb.cap >= needed && rb.buffer != vk::Buffer::null() {
            return Ok(());
        }
        let dev = &self.device;
        // SAFETY: handles belong to `dev`; old buffer (if any) is not in use
        // (we only get here between fully-fenced readbacks).
        unsafe {
            if rb.buffer != vk::Buffer::null() {
                dev.destroy_buffer(rb.buffer, None);
                dev.free_memory(rb.memory, None);
                rb.buffer = vk::Buffer::null();
                rb.memory = vk::DeviceMemory::null();
                rb.cap = 0;
            }
            let bci = vk::BufferCreateInfo::default()
                .size(needed)
                .usage(vk::BufferUsageFlags::TRANSFER_DST)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            let buffer = dev
                .create_buffer(&bci, None)
                .map_err(|e| format!("create_buffer: {e:?}"))?;
            let req = dev.get_buffer_memory_requirements(buffer);
            let mem_type = find_memory_type(
                &self.mem_props,
                req.memory_type_bits,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )
            .ok_or_else(|| "no host-visible coherent memory type".to_string())?;
            let mai = vk::MemoryAllocateInfo::default()
                .allocation_size(req.size)
                .memory_type_index(mem_type);
            let memory = dev
                .allocate_memory(&mai, None)
                .map_err(|e| format!("allocate_memory: {e:?}"))?;
            dev.bind_buffer_memory(buffer, memory, 0)
                .map_err(|e| format!("bind_buffer_memory: {e:?}"))?;
            rb.buffer = buffer;
            rb.memory = memory;
            rb.cap = needed;
        }
        Ok(())
    }
}

impl Drop for VulkanHw {
    fn drop(&mut self) {
        // SAFETY: wait for the device to go idle, then destroy everything we
        // own in dependency order. The core's context_destroy has already
        // run (state::hw_teardown calls it before dropping us), so the core
        // no longer references these objects.
        unsafe {
            let _ = self.device.device_wait_idle();
            if let Ok(rb) = self.readback.lock() {
                if rb.buffer != vk::Buffer::null() {
                    self.device.destroy_buffer(rb.buffer, None);
                    self.device.free_memory(rb.memory, None);
                }
                self.device.destroy_fence(rb.fence, None);
                self.device.destroy_command_pool(rb.cmd_pool, None);
            }
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
        // `entry` drops last (unloads the loader).
        let _ = &self.entry;
    }
}

/// Create the readback command pool/buffer/fence and assemble the
/// `retro_hw_render_interface_vulkan` we hand the core. Borrows the
/// device/entry/instance so the caller can still destroy the device on a
/// failure here (the device isn't owned by a `VulkanHw` yet). The
/// interface `handle` is left null — patched in `finalize_handle` once the
/// `VulkanHw` is boxed (address-stable).
fn build_resources(
    entry: &ash::Entry,
    instance: &ash::Instance,
    device: &ash::Device,
    phys: vk::PhysicalDevice,
    queue: vk::Queue,
    queue_family: u32,
) -> Result<
    (vk::CommandPool, vk::CommandBuffer, vk::Fence, retro_hw_render_interface_vulkan),
    String,
> {
    // SAFETY: all handles belong to `device`/`instance`; freed by the
    // caller's device teardown on error.
    unsafe {
        let pool_ci = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let cmd_pool = device
            .create_command_pool(&pool_ci, None)
            .map_err(|e| format!("create_command_pool failed: {e:?}"))?;
        let alloc_ci = vk::CommandBufferAllocateInfo::default()
            .command_pool(cmd_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let cmd_buf = device
            .allocate_command_buffers(&alloc_ci)
            .map_err(|e| format!("allocate_command_buffers failed: {e:?}"))?[0];
        let fence = device
            .create_fence(&vk::FenceCreateInfo::default(), None)
            .map_err(|e| format!("create_fence failed: {e:?}"))?;

        let interface = retro_hw_render_interface_vulkan {
            interface_type: RETRO_HW_RENDER_INTERFACE_VULKAN,
            interface_version: RETRO_HW_RENDER_INTERFACE_VULKAN_VERSION,
            handle: std::ptr::null_mut(),
            instance: instance.handle(),
            gpu: phys,
            device: device.handle(),
            get_device_proc_addr: instance.fp_v1_0().get_device_proc_addr,
            get_instance_proc_addr: entry.static_fn().get_instance_proc_addr,
            queue,
            queue_index: queue_family,
            set_image: vk_set_image,
            get_sync_index: vk_get_sync_index,
            get_sync_index_mask: vk_get_sync_index_mask,
            set_command_buffers: vk_set_command_buffers,
            wait_sync_index: vk_wait_sync_index,
            lock_queue: vk_lock_queue,
            unlock_queue: vk_unlock_queue,
            set_signal_semaphore: vk_set_signal_semaphore,
        };
        Ok((cmd_pool, cmd_buf, fence, interface))
    }
}

/// Find a memory type index satisfying `type_bits` + `flags`.
fn find_memory_type(
    props: &vk::PhysicalDeviceMemoryProperties,
    type_bits: u32,
    flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    (0..props.memory_type_count).find(|&i| {
        (type_bits & (1 << i)) != 0
            && props.memory_types[i as usize]
                .property_flags
                .contains(flags)
    })
}

/// Convert a mapped BGRA8/RGBA8 staging buffer into `fb_rgba` (R,G,B,A,
/// opaque alpha), flipping vertically when `flip_y`. `src` is tightly
/// packed `w*h*4` bytes.
fn convert_into(src: &[u8], w: u32, h: u32, flip_y: bool, format: vk::Format, fb_rgba: &mut Vec<u8>) {
    let wu = w as usize;
    let hu = h as usize;
    let need = wu * hu * 4;
    if fb_rgba.len() < need {
        fb_rgba.resize(need, 0);
    }
    let bgra = is_bgra(format);
    for y in 0..hu {
        let sy = if flip_y { hu - 1 - y } else { y };
        let srow = sy * wu * 4;
        let drow = y * wu * 4;
        for x in 0..wu {
            let si = srow + x * 4;
            let di = drow + x * 4;
            let (r, g, b) = if bgra {
                (src[si + 2], src[si + 1], src[si])
            } else {
                (src[si], src[si + 1], src[si + 2])
            };
            fb_rgba[di] = r;
            fb_rgba[di + 1] = g;
            fb_rgba[di + 2] = b;
            fb_rgba[di + 3] = 0xFF;
        }
    }
}

fn is_bgra(format: vk::Format) -> bool {
    matches!(
        format,
        vk::Format::B8G8R8A8_UNORM | vk::Format::B8G8R8A8_SRGB
    ) || {
        // Log unexpected formats once so we can add a swizzle if needed.
        if !matches!(
            format,
            vk::Format::R8G8B8A8_UNORM | vk::Format::R8G8B8A8_SRGB
        ) {
            static WARNED: AtomicBool = AtomicBool::new(false);
            if !WARNED.swap(true, Ordering::Relaxed) {
                log::warn!(
                    "oa-libretro HW: unexpected core image format {format:?} — treating as RGBA8 (colors may be off)"
                );
            }
        }
        false
    }
}

// ---- the 8 frontend callbacks the core drives each frame ----
// Each receives the opaque `handle` (= our boxed VulkanHw). They use the
// struct's internal locks only — never the STATE mutex — so the core can
// call them re-entrantly from inside retro_run / its worker threads.

/// # Safety: `handle` is the VulkanHw pointer we set on the interface.
unsafe fn hw_from_handle<'a>(handle: *mut c_void) -> Option<&'a VulkanHw> {
    if handle.is_null() {
        None
    } else {
        Some(unsafe { &*(handle as *const VulkanHw) })
    }
}

unsafe extern "C" fn vk_set_image(
    handle: *mut c_void,
    image: *const retro_vulkan_image,
    num_semaphores: u32,
    semaphores: *const vk::Semaphore,
    _src_queue_family: u32,
) {
    let Some(hw) = (unsafe { hw_from_handle(handle) }) else {
        return;
    };
    if image.is_null() {
        return;
    }
    let img = unsafe { &*image };
    // One-shot: confirms the core actually handed us a rendered image
    // (the pipeline is alive past context_reset) + reports its format so
    // we can verify the readback swizzle.
    static FIRST_SET_IMAGE: AtomicBool = AtomicBool::new(false);
    if !FIRST_SET_IMAGE.swap(true, Ordering::Relaxed) {
        log::info!(
            "oa-libretro HW: first set_image — format={:?} layout={:?} num_semaphores={}",
            img.create_info.format,
            img.image_layout,
            num_semaphores,
        );
    }
    let mut wait = Vec::new();
    if !semaphores.is_null() && num_semaphores > 0 {
        wait.extend_from_slice(unsafe {
            std::slice::from_raw_parts(semaphores, num_semaphores as usize)
        });
    }
    if let Ok(mut g) = hw.frame.lock() {
        *g = Some(FrameImage {
            image: img.create_info.image,
            layout: img.image_layout,
            format: img.create_info.format,
            wait_sems: wait,
            width: 0,
            height: 0,
            ready: false,
        });
    }
}

unsafe extern "C" fn vk_get_sync_index(_handle: *mut c_void) -> u32 {
    // M1: single synchronous buffer.
    0
}

unsafe extern "C" fn vk_get_sync_index_mask(_handle: *mut c_void) -> u32 {
    // One buffer in flight → bit 0 only.
    0x1
}

unsafe extern "C" fn vk_set_command_buffers(
    _handle: *mut c_void,
    _num_cmd: u32,
    _cmd: *const vk::CommandBuffer,
) {
    // M1: cores we target (Dolphin) present via set_image, not pre-submitted
    // command buffers. Log once if a core actually uses this so we wire it.
    static WARNED: AtomicBool = AtomicBool::new(false);
    if !WARNED.swap(true, Ordering::Relaxed) {
        log::warn!("oa-libretro HW: core called set_command_buffers — ignored in M1");
    }
}

unsafe extern "C" fn vk_wait_sync_index(_handle: *mut c_void) {
    // M1 is fully synchronous (we host-wait the fence each readback), so the
    // single buffer is always free by the time the core renders again.
}

unsafe extern "C" fn vk_lock_queue(handle: *mut c_void) {
    if let Some(hw) = unsafe { hw_from_handle(handle) } {
        hw.queue_lock.lock();
    }
}

unsafe extern "C" fn vk_unlock_queue(handle: *mut c_void) {
    if let Some(hw) = unsafe { hw_from_handle(handle) } {
        hw.queue_lock.unlock();
    }
}

unsafe extern "C" fn vk_set_signal_semaphore(handle: *mut c_void, semaphore: vk::Semaphore) {
    if let Some(hw) = unsafe { hw_from_handle(handle) } {
        if let Ok(mut g) = hw.signal_sem.lock() {
            *g = semaphore;
        }
    }
}
