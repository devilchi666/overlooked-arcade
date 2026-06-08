//! Vulkan HW-render bring-up for libretro hardware cores (Dolphin,
//! paraLLEl-N64, Beetle PSX HW, …). See docs/PLANS/hw-render-pipeline.md.
//!
//! **M1 probe phase (2026-06-08, operator decision "probe-first").** This
//! module does NOT yet stand up the standalone `VkInstance`/`VkDevice`.
//! Its job right now is to *report* exactly what the loaded HW core
//! requests so the device/queue code in the next commit can be written
//! against real numbers instead of guessed ABI:
//!
//!   * the Vulkan negotiation interface version + which of its callbacks
//!     the core actually provides (v1 `create_device` vs v2
//!     `create_device2`), and
//!   * the Vulkan `apiVersion` the core wants (drives which version our
//!     instance must request), and
//!   * the host's Vulkan physical devices, so we confirm M1 will pick the
//!     dedicated GPU (e.g. a GeForce RTX 4090) over any integrated GPU.
//!
//! Per D6 the M1 device is a standalone `ash` device isolated from wgpu,
//! with CPU readback into `State.fb_rgba`; wgpu is left untouched.

use std::ffi::CStr;

use ash::vk;

use crate::ffi::*;

/// Log the Vulkan context-negotiation interface a core handed us via
/// `RETRO_ENVIRONMENT_SET_HW_RENDER_CONTEXT_NEGOTIATION_INTERFACE` (env
/// 43). Records the interface version, which callbacks are non-NULL (so we
/// know whether to drive the v1 `create_device` path the plan assumes or
/// the v2 `create_device2` wrapper path), and — if the core provides
/// `get_application_info` — the Vulkan `apiVersion` it prefers. Calling
/// `get_application_info` is safe with no device present; it only returns
/// the core's static `VkApplicationInfo`.
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
            "oa-libretro HW: core provides no get_application_info — instance will default to Vulkan 1.1"
        );
        return;
    };
    // SAFETY: the core owns this fn pointer (registered via env 43); it
    // returns a pointer to its static VkApplicationInfo or NULL. No device
    // state is touched.
    let app_ptr = unsafe { get_app() };
    if app_ptr.is_null() {
        log::info!("oa-libretro HW: get_application_info returned NULL (core has no apiVersion preference)");
        return;
    }
    // SAFETY: non-null per the check above; lifetime is the core's (static).
    let app = unsafe { &*app_ptr };
    let api = app.api_version;
    let name = unsafe { read_cstr_opt(app.p_application_name) }.unwrap_or_else(|| "<none>".into());
    let engine = unsafe { read_cstr_opt(app.p_engine_name) }.unwrap_or_else(|| "<none>".into());
    if api == 0 {
        log::info!(
            "oa-libretro HW: core apiVersion=0 (means Vulkan 1.0) — app=\"{name}\" engine=\"{engine}\""
        );
    } else {
        log::info!(
            "oa-libretro HW: core wants Vulkan apiVersion {}.{}.{} — app=\"{name}\" engine=\"{engine}\"",
            vk::api_version_major(api),
            vk::api_version_minor(api),
            vk::api_version_patch(api),
        );
    }
}

/// Enumerate the host's Vulkan physical devices and log each with its type
/// + driver Vulkan version, then report which one the M1 device-build will
/// select: the first discrete GPU (the dedicated card — e.g. a GeForce RTX
/// 4090), falling back to the first available device if the host has no
/// discrete GPU.
///
/// Uses a throwaway `VkInstance` destroyed before returning — the real
/// instance/device come in the next commit. Any failure here (no Vulkan
/// loader, instance/enumeration error) is logged and swallowed; this is a
/// diagnostic, not a hard dependency yet.
pub(crate) fn probe_physical_devices() {
    // SAFETY: loads the system Vulkan loader (vulkan-1.dll on Windows).
    let entry = match unsafe { ash::Entry::load() } {
        Ok(e) => e,
        Err(e) => {
            log::error!(
                "oa-libretro HW: failed to load the Vulkan loader ({e}) — this host has no usable Vulkan runtime; HW cores will need the M3 software-peer fallback"
            );
            return;
        }
    };

    let app_info = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_1);
    let create_info = vk::InstanceCreateInfo::default().application_info(&app_info);
    // SAFETY: create_info is valid for the duration of the call; no
    // extensions/layers requested (headless enumeration only).
    let instance = match unsafe { entry.create_instance(&create_info, None) } {
        Ok(i) => i,
        Err(e) => {
            log::error!("oa-libretro HW: vkCreateInstance failed: {e:?}");
            return;
        }
    };

    // SAFETY: instance is live until destroy_instance below.
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
        // SAFETY: pd came from this instance's enumeration.
        let props = unsafe { instance.get_physical_device_properties(pd) };
        // SAFETY: device_name is a NUL-terminated fixed C-char array.
        let name = unsafe { CStr::from_ptr(props.device_name.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        let type_str = match props.device_type {
            vk::PhysicalDeviceType::DISCRETE_GPU => "discrete",
            vk::PhysicalDeviceType::INTEGRATED_GPU => "integrated",
            vk::PhysicalDeviceType::VIRTUAL_GPU => "virtual",
            vk::PhysicalDeviceType::CPU => "cpu",
            _ => "other",
        };
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
            // No discrete GPU — fall back to the first available device.
            if let Some(&pd) = pds.first() {
                let props = unsafe { instance.get_physical_device_properties(pd) };
                let name = unsafe { CStr::from_ptr(props.device_name.as_ptr()) }
                    .to_string_lossy()
                    .into_owned();
                log::warn!(
                    "oa-libretro HW: no discrete GPU found — M1 will fall back to [0] {name}"
                );
            } else {
                log::error!(
                    "oa-libretro HW: zero Vulkan physical devices — HW cores cannot run on this host"
                );
            }
        }
    }

    // SAFETY: no child objects outlive the instance (we only read props).
    unsafe { instance.destroy_instance(None) };
}

/// Read an optional NUL-terminated C string pointer to an owned `String`,
/// returning `None` for a NULL pointer.
///
/// # Safety
/// `p` must be NULL or point to a valid NUL-terminated C string.
unsafe fn read_cstr_opt(p: *const std::os::raw::c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    Some(unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
}
