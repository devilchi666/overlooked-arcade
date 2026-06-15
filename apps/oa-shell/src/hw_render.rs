//! HW-render shell-side policy — capability observability + software-peer
//! fallback suggestions (HW-Render M3, feature DECISIONS D4 "capability
//! tiering, never a silent crash").
//!
//! The libretro/Vulkan plumbing lives in `oa-libretro` (`state.rs` +
//! `hw_vulkan.rs`); this module is the thin shell-side layer that turns the
//! core's negotiated [`HwRenderStatus`](oa_libretro::HwRenderStatus) into:
//!   1. an unambiguous per-launch log line (so an operator validating a new
//!      HW core sees exactly what the handshake did), and
//!   2. a software-peer suggestion when we declined HW for a core whose
//!      system has a genuine software-only sibling core — so the operator
//!      isn't left guessing at a black screen.
//!
//! These are **suggestions, not an auto-swap.** Auto-swapping cores mid-load
//! is deferred to the per-core validation loop (M3 Half 1): we don't yet know
//! which cores crash on a declined HW context versus fall back to their own
//! software/GL renderer cleanly, and a blind auto-swap risks a load loop if
//! the peer also fails. Telling the operator is the safe, honest behavior.

use oa_libretro::HwRenderStatus;

/// Map a hardware-render core (by base filename, no extension) to a
/// software-only sibling core that emulates the same system on the CPU.
///
/// Only the systems with a *genuine separate software-only core DLL* are
/// listed — PSX (Beetle/Mednafen PSX HW → the SW build) and Saturn
/// (Kronos / YabaSanshiro GL cores → Beetle Saturn, software-only). N64,
/// Dreamcast, and PSP have no separate SW core: their cores fall back to an
/// internal software/GL renderer via a core option when we decline Vulkan,
/// so there's nothing to suggest (the core handles it itself). Returns the
/// peer's base filename (no `.dll`/`.so`) or `None`.
pub fn software_peer_core(hw_core_base: &str) -> Option<&'static str> {
    // Normalize away any platform extension the caller might pass.
    let base = hw_core_base
        .trim_end_matches(".dll")
        .trim_end_matches(".so")
        .trim_end_matches(".dylib");
    match base {
        // PSX — Beetle PSX HW (a.k.a. Mednafen PSX HW) → the software build.
        "mednafen_psx_hw_libretro" | "beetle_psx_hw_libretro" => {
            Some("mednafen_psx_libretro")
        }
        // Saturn — the GL/HW cores → Beetle Saturn (Mednafen), software-only.
        "kronos_libretro" | "yabasanshiro_libretro" => Some("mednafen_saturn_libretro"),
        _ => None,
    }
}

/// One-line log summary of the HW-render handshake outcome for this launch.
/// Always safe to log; `NotRequested` (software cores, the common case) is
/// summarized too so the log is unambiguous rather than silent.
pub fn hw_status_log_line(status: HwRenderStatus, core_dll: &str) -> String {
    match status {
        HwRenderStatus::NotRequested => {
            format!("HW-render: {core_dll} is a software core (no SET_HW_RENDER)")
        }
        HwRenderStatus::AcceptedVulkan => {
            format!("HW-render: {core_dll} accepted — Vulkan HW path active")
        }
        HwRenderStatus::DeclinedNonVulkan(ctx) => format!(
            "HW-render: {core_dll} requested non-Vulkan context_type {ctx} — declined; \
             core falls back to its own software/GL renderer"
        ),
        HwRenderStatus::DeclinedInstanceError => format!(
            "HW-render: {core_dll} requested Vulkan but VkInstance create failed — declined; \
             core falls back to its own software/GL renderer"
        ),
    }
}

/// Operator-facing toast for a *declined* HW handshake — `None` when HW was
/// accepted or never requested (nothing to surface). On a decline, names the
/// software-peer core when one exists so the operator has a concrete next
/// step instead of a black screen.
pub fn hw_decline_toast(status: HwRenderStatus, core_dll: &str) -> Option<String> {
    let reason = match status {
        HwRenderStatus::NotRequested | HwRenderStatus::AcceptedVulkan => return None,
        HwRenderStatus::DeclinedNonVulkan(_) => {
            "needs a non-Vulkan GPU API OA doesn't drive yet"
        }
        HwRenderStatus::DeclinedInstanceError => "couldn't initialize Vulkan",
    };
    let base = core_dll
        .trim_end_matches(".dll")
        .trim_end_matches(".so")
        .trim_end_matches(".dylib");
    Some(match software_peer_core(base) {
        Some(peer) => format!(
            "{core_dll} {reason}. If the screen stays black, install {peer}.dll (software) \
             via Browse cores and set it as this system's core."
        ),
        None => format!(
            "{core_dll} {reason}; it will use its own software/GL renderer. \
             If the screen stays black, pick a software renderer in this system's core options."
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn psx_and_saturn_have_software_peers() {
        assert_eq!(
            software_peer_core("mednafen_psx_hw_libretro"),
            Some("mednafen_psx_libretro")
        );
        assert_eq!(
            software_peer_core("beetle_psx_hw_libretro"),
            Some("mednafen_psx_libretro")
        );
        assert_eq!(
            software_peer_core("kronos_libretro"),
            Some("mednafen_saturn_libretro")
        );
        assert_eq!(
            software_peer_core("yabasanshiro_libretro"),
            Some("mednafen_saturn_libretro")
        );
    }

    #[test]
    fn peer_lookup_tolerates_a_dll_extension() {
        assert_eq!(
            software_peer_core("mednafen_psx_hw_libretro.dll"),
            Some("mednafen_psx_libretro")
        );
    }

    #[test]
    fn cores_without_a_software_sibling_return_none() {
        // N64 / Dreamcast / PSP HW cores fall back via their own renderer
        // option — no separate SW core to suggest.
        for base in [
            "parallel_n64_libretro",
            "mupen64plus_next_libretro",
            "flycast_libretro",
            "ppsspp_libretro",
            "mednafen_pce_libretro", // an ordinary software core
        ] {
            assert_eq!(software_peer_core(base), None, "{base} should have no peer");
        }
    }

    #[test]
    fn accepted_and_not_requested_produce_no_toast() {
        assert!(hw_decline_toast(HwRenderStatus::AcceptedVulkan, "x_libretro.dll").is_none());
        assert!(hw_decline_toast(HwRenderStatus::NotRequested, "x_libretro.dll").is_none());
    }

    #[test]
    fn declined_toast_names_the_peer_when_one_exists() {
        let msg = hw_decline_toast(
            HwRenderStatus::DeclinedNonVulkan(2),
            "mednafen_psx_hw_libretro.dll",
        )
        .expect("decline should toast");
        assert!(msg.contains("mednafen_psx_libretro.dll"), "got: {msg}");
    }

    #[test]
    fn declined_toast_without_peer_points_at_core_options() {
        let msg = hw_decline_toast(HwRenderStatus::DeclinedInstanceError, "flycast_libretro.dll")
            .expect("decline should toast");
        assert!(msg.contains("core options"), "got: {msg}");
    }
}
