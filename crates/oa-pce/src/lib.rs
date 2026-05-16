//! oa-pce — idiomatic Rust wrapper over `oa-pce-sys`, impls `oa_core::Core`.
//!
//! The shim layer in `oa-pce-sys/shim.cpp` exposes a small C surface on top of
//! Beetle PCE Fast's libretro lifecycle. `PceCore` wraps that surface in a
//! Rust-friendly object, handling pointer lifetime, audio buffering, and the
//! mapping between our `InputState::buttons` bitfield and libretro's joypad
//! button IDs.
//!
//! The vendored core is singleton-style (Mednafen owns module-globals). Don't
//! construct more than one `PceCore` at once — `oa_pce_sys::oa_pce_new` returns
//! the same handle each time.

#![deny(rust_2018_idioms)]

use std::ptr::NonNull;

use oa_core::{Core, CoreError, Framebuffer, InputState, PortIndex, SystemId, Timing};
use oa_pce_sys::OaPceCore;

/// PCE button bits in `oa_core::InputState::buttons`.
///
/// We define a stable Rust-side layout and translate it to libretro's joypad
/// button IDs inside `set_input`. That way the shell + input mapper don't have
/// to know about libretro's quirky numbering.
pub mod buttons {
    /// I (right action button).
    pub const I: u32      = 1 << 0;
    /// II (left action button).
    pub const II: u32     = 1 << 1;
    /// Select.
    pub const SELECT: u32 = 1 << 2;
    /// Run (start).
    pub const RUN: u32    = 1 << 3;
    /// D-pad up.
    pub const UP: u32     = 1 << 4;
    /// D-pad right.
    pub const RIGHT: u32  = 1 << 5;
    /// D-pad down.
    pub const DOWN: u32   = 1 << 6;
    /// D-pad left.
    pub const LEFT: u32   = 1 << 7;
}

/// libretro RETRO_DEVICE_ID_JOYPAD_* values for the eight bits we care about.
mod retro_id {
    pub const B: u16      = 0; // PCE II
    pub const SELECT: u16 = 2;
    pub const START: u16  = 3; // PCE RUN
    pub const UP: u16     = 4;
    pub const DOWN: u16   = 5;
    pub const LEFT: u16   = 6;
    pub const RIGHT: u16  = 7;
    pub const A: u16      = 8; // PCE I
}

fn pce_to_retro_bits(b: u32) -> u16 {
    let mut out: u16 = 0;
    if b & buttons::I      != 0 { out |= 1 << retro_id::A; }
    if b & buttons::II     != 0 { out |= 1 << retro_id::B; }
    if b & buttons::SELECT != 0 { out |= 1 << retro_id::SELECT; }
    if b & buttons::RUN    != 0 { out |= 1 << retro_id::START; }
    if b & buttons::UP     != 0 { out |= 1 << retro_id::UP; }
    if b & buttons::DOWN   != 0 { out |= 1 << retro_id::DOWN; }
    if b & buttons::LEFT   != 0 { out |= 1 << retro_id::LEFT; }
    if b & buttons::RIGHT  != 0 { out |= 1 << retro_id::RIGHT; }
    out
}

/// PCE / TurboGrafx-16 core (HuCard).
///
/// Live `oa_pce_sys` instance. The C side maintains static module-globals; this
/// struct mediates access and runs Drop cleanup.
pub struct PceCore {
    handle: NonNull<OaPceCore>,
    rom_loaded: bool,
    audio_buf: Vec<i16>,
}

// SAFETY: the underlying core is singleton and we own the only handle for its
// lifetime; all method invocations on the C side are non-reentrant from Rust.
unsafe impl Send for PceCore {}

impl PceCore {
    /// PCE NTSC native resolution (most common mode).
    pub const NATIVE_WIDTH: u32 = 256;
    /// PCE NTSC native vertical resolution.
    pub const NATIVE_HEIGHT: u32 = 239;
    /// PCE NTSC frame rate (Mednafen-canonical).
    pub const NATIVE_FPS: f64 = 59.826_113_28;
    /// PCE audio sample rate Mednafen produces at.
    pub const NATIVE_SAMPLE_RATE: u32 = 44_100;

    /// Construct a PCE core. Calls `oa_pce_new`, which initialises Mednafen
    /// PCE Fast on first call and is a no-op afterwards (singleton).
    pub fn new() -> Self {
        let ptr = unsafe { oa_pce_sys::oa_pce_new() };
        let handle = NonNull::new(ptr).expect("oa_pce_new returned null");
        Self {
            handle,
            rom_loaded: false,
            audio_buf: Vec::with_capacity(8192),
        }
    }

    /// Load a HuCard ROM image. Replaces any previously loaded game.
    pub fn load_rom(&mut self, data: &[u8]) -> Result<(), CoreError> {
        if data.is_empty() {
            return Err(CoreError::InvalidRom("empty ROM data".into()));
        }
        let status =
            unsafe { oa_pce_sys::oa_pce_load_rom(self.handle.as_ptr(), data.as_ptr(), data.len()) };
        if status == 0 {
            self.rom_loaded = true;
            Ok(())
        } else {
            self.rom_loaded = false;
            Err(CoreError::InvalidRom(format!(
                "oa_pce_load_rom failed (status {status}, {} bytes)",
                data.len()
            )))
        }
    }

    /// True once `load_rom` has returned Ok.
    pub fn has_rom(&self) -> bool {
        self.rom_loaded
    }
}

impl Default for PceCore {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PceCore {
    fn drop(&mut self) {
        unsafe { oa_pce_sys::oa_pce_free(self.handle.as_ptr()) };
    }
}

impl Core for PceCore {
    fn system(&self) -> SystemId {
        SystemId::PcEngine
    }

    fn timing(&self) -> Timing {
        Timing {
            width: Self::NATIVE_WIDTH,
            height: Self::NATIVE_HEIGHT,
            fps: Self::NATIVE_FPS,
            sample_rate: Self::NATIVE_SAMPLE_RATE,
        }
    }

    fn reset(&mut self) {
        if self.rom_loaded {
            unsafe { oa_pce_sys::oa_pce_reset(self.handle.as_ptr()) };
        }
    }

    fn run_frame(&mut self) {
        if !self.rom_loaded {
            return;
        }
        unsafe { oa_pce_sys::oa_pce_run_frame(self.handle.as_ptr()) };
    }

    fn framebuffer(&self) -> Framebuffer<'_> {
        let f = unsafe { oa_pce_sys::oa_pce_framebuffer(self.handle.as_ptr()) };
        let len = (f.width as usize).saturating_mul(f.height as usize).saturating_mul(4);
        let pixels = if f.pixels.is_null() || len == 0 {
            &[][..]
        } else {
            // SAFETY: shim guarantees `f.pixels` points to at least len bytes,
            // valid until the next call to `oa_pce_run_frame`. The borrow is
            // tied to &self so the borrow checker enforces that.
            unsafe { std::slice::from_raw_parts(f.pixels, len) }
        };
        Framebuffer {
            width: f.width,
            height: f.height,
            pixels,
            display_aspect: f.display_aspect,
        }
    }

    fn drain_audio(&mut self) -> &[i16] {
        // Capacity 8192 stereo pairs = 16384 i16s. PCE at 44.1kHz / 60fps =
        // 735 frames/frame, so 16384 is ~22 frames of slack. Plenty.
        if self.audio_buf.capacity() < 16384 {
            self.audio_buf.reserve(16384 - self.audio_buf.capacity());
        }
        unsafe { self.audio_buf.set_len(self.audio_buf.capacity()); }
        let n = unsafe {
            oa_pce_sys::oa_pce_audio_samples(
                self.handle.as_ptr(),
                self.audio_buf.as_mut_ptr(),
                self.audio_buf.capacity(),
            )
        };
        unsafe { self.audio_buf.set_len(n); }
        &self.audio_buf
    }

    fn set_input(&mut self, port: PortIndex, input: InputState) {
        let port_idx = port as u32;
        let bits = pce_to_retro_bits(input.buttons);
        unsafe { oa_pce_sys::oa_pce_set_input(self.handle.as_ptr(), port_idx, bits) };
    }

    fn save_state(&self, writer: &mut dyn std::io::Write) -> Result<(), CoreError> {
        if !self.rom_loaded {
            return Err(CoreError::Internal("save_state called before load_rom".into()));
        }
        let size = unsafe { oa_pce_sys::oa_pce_serialize_size(self.handle.as_ptr()) };
        if size == 0 {
            return Err(CoreError::Internal(
                "core reported zero serialize size (save states unsupported in current state)".into(),
            ));
        }
        let mut buf = vec![0u8; size];
        let status = unsafe {
            oa_pce_sys::oa_pce_serialize(
                self.handle.as_ptr(),
                buf.as_mut_ptr().cast(),
                buf.len(),
            )
        };
        if status != 0 {
            return Err(CoreError::Internal(format!(
                "oa_pce_serialize failed (status {status}, {size} bytes requested)"
            )));
        }
        writer.write_all(&buf)?;
        Ok(())
    }

    fn load_state(&mut self, reader: &mut dyn std::io::Read) -> Result<(), CoreError> {
        if !self.rom_loaded {
            return Err(CoreError::Internal("load_state called before load_rom".into()));
        }
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        if buf.is_empty() {
            return Err(CoreError::SaveStateMalformed);
        }
        let status = unsafe {
            oa_pce_sys::oa_pce_unserialize(
                self.handle.as_ptr(),
                buf.as_ptr().cast(),
                buf.len(),
            )
        };
        if status != 0 {
            return Err(CoreError::Internal(format!(
                "oa_pce_unserialize failed (status {status}, {} bytes)",
                buf.len()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_native_timing() {
        let core = PceCore::new();
        let t = core.timing();
        assert_eq!(t.width, PceCore::NATIVE_WIDTH);
        assert_eq!(t.height, PceCore::NATIVE_HEIGHT);
        assert_eq!(t.sample_rate, 44_100);
    }

    #[test]
    fn rejects_empty_rom() {
        let mut core = PceCore::new();
        assert!(core.load_rom(&[]).is_err());
        assert!(!core.has_rom());
    }

    #[test]
    fn pce_button_remap() {
        // Pressing PCE I should set libretro bit A (8), not bit 0.
        assert_eq!(pce_to_retro_bits(buttons::I), 1 << 8);
        assert_eq!(pce_to_retro_bits(buttons::II), 1 << 0);
        assert_eq!(pce_to_retro_bits(buttons::RUN), 1 << 3);
        assert_eq!(
            pce_to_retro_bits(buttons::UP | buttons::I | buttons::RUN),
            (1 << 4) | (1 << 8) | (1 << 3)
        );
    }
}
