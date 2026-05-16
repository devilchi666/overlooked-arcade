//! oa-pce-sys — raw FFI bindings to the vendored Beetle PCE Fast core.
//!
//! Hand-written `extern "C"` blocks (per the Spike 3 decision). The C++ shim
//! at `shim.cpp` exposes the `oa_pce_*` surface designed in Spike 3 on top of
//! libretro's `retro_*` lifecycle.

#![deny(rust_2018_idioms)]
#![allow(non_snake_case, non_camel_case_types)]

use std::os::raw::{c_char, c_void};

/// Opaque core handle. The shim is singleton-style internally; we still pass a
/// pointer for ABI symmetry and to give the wrapper a place to hang state.
#[repr(C)]
pub struct OaPceCore {
    _private: [u8; 0],
}

/// Borrow of the core's latest framebuffer (RGBA8, `width * height * 4` bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OaPceFrame {
    pub width: u32,
    pub height: u32,
    pub pixels: *const u8,
}

/// Static metadata about the vendored core.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OaPceCoreInfo {
    pub core_name: *const c_char,
    pub version_major: u32,
    pub version_minor: u32,
}

extern "C" {
    // ---- Spike 3 / production surface ----
    pub fn oa_pce_new() -> *mut OaPceCore;
    pub fn oa_pce_free(core: *mut OaPceCore);
    pub fn oa_pce_load_rom(core: *mut OaPceCore, data: *const u8, len: usize) -> i32;
    pub fn oa_pce_reset(core: *mut OaPceCore);
    pub fn oa_pce_run_frame(core: *mut OaPceCore);
    pub fn oa_pce_framebuffer(core: *const OaPceCore) -> OaPceFrame;
    pub fn oa_pce_audio_samples(core: *const OaPceCore, out: *mut i16, out_cap: usize) -> usize;
    pub fn oa_pce_set_input(core: *mut OaPceCore, port: u32, bits: u16);
    pub fn oa_pce_info() -> OaPceCoreInfo;

    // ---- Mednafen-derived endian helpers (kept for backward compat with our spike tests) ----
    pub fn FlipByteOrder(src: *mut u8, count: u32);
    pub fn Endian_A16_Swap(src: *mut c_void, nelements: u32);
    pub fn Endian_A32_Swap(src: *mut c_void, nelements: u32);
    pub fn Endian_A64_Swap(src: *mut c_void, nelements: u32);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flip_reverses_bytes() {
        let mut buf = [0x01u8, 0x02, 0x03, 0x04];
        unsafe { FlipByteOrder(buf.as_mut_ptr(), buf.len() as u32) };
        assert_eq!(buf, [0x04, 0x03, 0x02, 0x01]);
    }

    #[test]
    fn endian_a16_swaps_halfwords() {
        let mut buf = [0x0102u16, 0x0304u16];
        unsafe { Endian_A16_Swap(buf.as_mut_ptr() as *mut std::ffi::c_void, buf.len() as u32) };
        assert_eq!(buf, [0x0201u16, 0x0403u16]);
    }

    #[test]
    fn info_returns_non_null_name() {
        // Smoke-test the new shim is linked.
        let info = unsafe { oa_pce_info() };
        assert!(!info.core_name.is_null());
        assert_eq!(info.version_major, 0);
    }
}
