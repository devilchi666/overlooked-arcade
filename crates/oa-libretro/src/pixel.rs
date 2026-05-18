//! Pixel format conversion: libretro source format → RGBA8 packed.
//!
//! Ported from `apps/oa-shell/../crates/oa-pce-sys/shim.cpp` (cb_video_refresh).
//! Same conversion arithmetic, just rewritten as safe Rust against borrowed
//! buffers instead of C raw pointers.

use crate::ffi::PixelFormat;

/// Convert a single source frame into RGBA8 packed format, writing into
/// `dst`. `dst` must be at least `width * height * 4` bytes.
///
/// `src_ptr` points to the source pixel data; `pitch` is the source byte-stride
/// per row (not pixels — libretro can use over-aligned strides).
///
/// # Safety
///
/// Caller guarantees `src_ptr` points to at least `pitch * height` valid
/// bytes and `dst` is at least `width * height * 4` bytes.
pub unsafe fn convert(
    pix_fmt: PixelFormat,
    src_ptr: *const u8,
    pitch: usize,
    width: u32,
    height: u32,
    dst: &mut [u8],
) {
    match pix_fmt {
        PixelFormat::Xrgb8888 => unsafe { convert_xrgb8888(src_ptr, pitch, width, height, dst) },
        PixelFormat::Rgb565 => unsafe { convert_rgb565(src_ptr, pitch, width, height, dst) },
        PixelFormat::Rgb1555 => unsafe { convert_0rgb1555(src_ptr, pitch, width, height, dst) },
    }
}

unsafe fn convert_xrgb8888(src_ptr: *const u8, pitch: usize, w: u32, h: u32, dst: &mut [u8]) {
    let row_pixels = w as usize;
    let mut di = 0usize;
    for y in 0..h as usize {
        let row_ptr = unsafe { src_ptr.add(y * pitch) } as *const u32;
        for x in 0..row_pixels {
            let p = unsafe { *row_ptr.add(x) };
            dst[di]     = ((p >> 16) & 0xFF) as u8; // R
            dst[di + 1] = ((p >>  8) & 0xFF) as u8; // G
            dst[di + 2] = ( p        & 0xFF) as u8; // B
            dst[di + 3] = 0xFF;
            di += 4;
        }
    }
}

unsafe fn convert_rgb565(src_ptr: *const u8, pitch: usize, w: u32, h: u32, dst: &mut [u8]) {
    let row_pixels = w as usize;
    let mut di = 0usize;
    for y in 0..h as usize {
        let row_ptr = unsafe { src_ptr.add(y * pitch) } as *const u16;
        for x in 0..row_pixels {
            let p = unsafe { *row_ptr.add(x) };
            let r5 = ((p >> 11) & 0x1F) as u8;
            let g6 = ((p >>  5) & 0x3F) as u8;
            let b5 = ( p        & 0x1F) as u8;
            // 5/6-bit → 8-bit with bit-replication (same as shim.cpp).
            dst[di]     = (r5 << 3) | (r5 >> 2);
            dst[di + 1] = (g6 << 2) | (g6 >> 4);
            dst[di + 2] = (b5 << 3) | (b5 >> 2);
            dst[di + 3] = 0xFF;
            di += 4;
        }
    }
}

unsafe fn convert_0rgb1555(src_ptr: *const u8, pitch: usize, w: u32, h: u32, dst: &mut [u8]) {
    let row_pixels = w as usize;
    let mut di = 0usize;
    for y in 0..h as usize {
        let row_ptr = unsafe { src_ptr.add(y * pitch) } as *const u16;
        for x in 0..row_pixels {
            let p = unsafe { *row_ptr.add(x) };
            let r5 = ((p >> 10) & 0x1F) as u8;
            let g5 = ((p >>  5) & 0x1F) as u8;
            let b5 = ( p        & 0x1F) as u8;
            dst[di]     = (r5 << 3) | (r5 >> 2);
            dst[di + 1] = (g5 << 3) | (g5 >> 2);
            dst[di + 2] = (b5 << 3) | (b5 >> 2);
            dst[di + 3] = 0xFF;
            di += 4;
        }
    }
}
