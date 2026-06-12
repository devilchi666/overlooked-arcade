//! Controller identity — derive a stable cross-layer device-key from a
//! gamepad's VID/PID. Phase 0 of the Controller Identity arc.
//! See docs/PLANS/controller-identity-substrate.md.
//!
//! This MUST stay byte-for-byte compatible with the frontend derivation in
//! `frontend/src/platform/nav/deviceKey.ts`: a pad that reports VID/PID on
//! both layers produces the identical `vidpid:<vid>:<pid>` key (decision D2).
//! `gilrs` 0.11 already exposes `Gamepad::vendor_id()` / `product_id()`
//! natively (no upgrade needed — the spike's headline finding), so Rust reads
//! VID/PID directly rather than parsing the SDL GUID by hand.
//!
//! When VID/PID is absent we fall back to `name:<slug>` (risk R5). Cross-layer
//! agreement of the name fallback is NOT guaranteed (the Web `id` name and the
//! OS name can differ) — that case is handled by frontend-as-identity-authority
//! (decision (c)), not by matching the fallback keys.

/// Build the cross-layer device-key. `vid` / `pid` come from
/// `gilrs::Gamepad::vendor_id()` / `product_id()`; `name` from `os_name()`.
pub fn device_key(vid: Option<u16>, pid: Option<u16>, name: &str) -> String {
    match (vid, pid) {
        (Some(v), Some(p)) => format!("vidpid:{v:04x}:{p:04x}"),
        _ => format!("name:{}", slug(name)),
    }
}

/// Lowercase, collapse runs of non-alphanumerics to a single dash, trim
/// leading/trailing dashes. Mirrors the frontend `slug()` exactly.
fn slug(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_dash = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "unknown".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vidpid_key_is_lowercase_zero_padded_hex() {
        // Switch Pro: VID 0x057e, PID 0x2009 — the operator's canonical pad.
        assert_eq!(device_key(Some(0x057e), Some(0x2009), "Pro Controller"), "vidpid:057e:2009");
        // Zero-padding: a low VID/PID must still be 4 hex digits.
        assert_eq!(device_key(Some(0x45), Some(0x8), "x"), "vidpid:0045:0008");
    }

    #[test]
    fn missing_vid_or_pid_falls_back_to_name_slug() {
        assert_eq!(device_key(None, None, "Xbox 360 Controller"), "name:xbox-360-controller");
        // Either half missing → fallback (we never emit a half key).
        assert_eq!(device_key(Some(0x057e), None, "Pad"), "name:pad");
        assert_eq!(device_key(None, Some(0x2009), "Pad"), "name:pad");
    }

    #[test]
    fn slug_collapses_and_trims_punctuation() {
        assert_eq!(device_key(None, None, "  Microsoft  X-Box 360 pad!! "), "name:microsoft-x-box-360-pad");
        assert_eq!(device_key(None, None, ""), "name:unknown");
        assert_eq!(device_key(None, None, "***"), "name:unknown");
    }
}
