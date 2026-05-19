//! `device_query::Keycode` → libretro `retro_key` translation.
//!
//! libretro cores that register a keyboard callback expect events tagged
//! with their own `retro_key` codespace (see the `RETROK_*` constants in
//! [`crate::ffi`]). The shell polls keys via the `device_query` crate which
//! returns its own enum; this module bridges between the two.
//!
//! Unmapped variants return `RETROK_UNKNOWN` so cores get a benign event
//! they can ignore instead of a garbage keycode that could trigger
//! something unrelated. F16-F20 land here because libretro only defines
//! F1-F15.

use device_query::Keycode;

use crate::ffi::*;

/// Translate a `device_query::Keycode` to its libretro `retro_key` value.
/// Returns [`RETROK_UNKNOWN`] for variants without a libretro equivalent.
pub fn keycode_to_retro_key(k: Keycode) -> u32 {
    match k {
        // Letters — libretro uses lowercase ASCII codepoints (97..=122)
        // even though `device_query` capitalizes the variant names. There
        // is no separate "shifted A" in libretro's keycode space; modifiers
        // ride alongside via the `key_modifiers` bitmask.
        Keycode::A => RETROK_a,
        Keycode::B => RETROK_b,
        Keycode::C => RETROK_c,
        Keycode::D => RETROK_d,
        Keycode::E => RETROK_e,
        Keycode::F => RETROK_f,
        Keycode::G => RETROK_g,
        Keycode::H => RETROK_h,
        Keycode::I => RETROK_i,
        Keycode::J => RETROK_j,
        Keycode::K => RETROK_k,
        Keycode::L => RETROK_l,
        Keycode::M => RETROK_m,
        Keycode::N => RETROK_n,
        Keycode::O => RETROK_o,
        Keycode::P => RETROK_p,
        Keycode::Q => RETROK_q,
        Keycode::R => RETROK_r,
        Keycode::S => RETROK_s,
        Keycode::T => RETROK_t,
        Keycode::U => RETROK_u,
        Keycode::V => RETROK_v,
        Keycode::W => RETROK_w,
        Keycode::X => RETROK_x,
        Keycode::Y => RETROK_y,
        Keycode::Z => RETROK_z,

        // Number row.
        Keycode::Key0 => RETROK_0,
        Keycode::Key1 => RETROK_1,
        Keycode::Key2 => RETROK_2,
        Keycode::Key3 => RETROK_3,
        Keycode::Key4 => RETROK_4,
        Keycode::Key5 => RETROK_5,
        Keycode::Key6 => RETROK_6,
        Keycode::Key7 => RETROK_7,
        Keycode::Key8 => RETROK_8,
        Keycode::Key9 => RETROK_9,

        // Function keys. libretro stops at F15; F16-F20 fall through to
        // UNKNOWN so cores get a benign no-op.
        Keycode::F1  => RETROK_F1,
        Keycode::F2  => RETROK_F2,
        Keycode::F3  => RETROK_F3,
        Keycode::F4  => RETROK_F4,
        Keycode::F5  => RETROK_F5,
        Keycode::F6  => RETROK_F6,
        Keycode::F7  => RETROK_F7,
        Keycode::F8  => RETROK_F8,
        Keycode::F9  => RETROK_F9,
        Keycode::F10 => RETROK_F10,
        Keycode::F11 => RETROK_F11,
        Keycode::F12 => RETROK_F12,
        Keycode::F13 => RETROK_F13,
        Keycode::F14 => RETROK_F14,
        Keycode::F15 => RETROK_F15,
        // F16-F20 have no libretro equivalent — return UNKNOWN so a stray
        // press becomes a benign no-op instead of misrouting into the
        // RETROK numeric space (where 297..=301 collides with NUMLOCK /
        // CAPSLOCK / SCROLLOCK / RSHIFT). Tested in the F16/F20 test below.
        Keycode::F16 => RETROK_UNKNOWN,
        Keycode::F17 => RETROK_UNKNOWN,
        Keycode::F18 => RETROK_UNKNOWN,
        Keycode::F19 => RETROK_UNKNOWN,
        Keycode::F20 => RETROK_UNKNOWN,

        // Numpad — libretro uses a parallel KP* keyspace so cores that
        // care about distinguishing the main row from the keypad get the
        // right answer. (Most cores treat them identically, but MAME has
        // drivers that bind to the keypad specifically.)
        Keycode::Numpad0 => RETROK_KP0,
        Keycode::Numpad1 => RETROK_KP1,
        Keycode::Numpad2 => RETROK_KP2,
        Keycode::Numpad3 => RETROK_KP3,
        Keycode::Numpad4 => RETROK_KP4,
        Keycode::Numpad5 => RETROK_KP5,
        Keycode::Numpad6 => RETROK_KP6,
        Keycode::Numpad7 => RETROK_KP7,
        Keycode::Numpad8 => RETROK_KP8,
        Keycode::Numpad9 => RETROK_KP9,
        Keycode::NumpadAdd       => RETROK_KP_PLUS,
        Keycode::NumpadSubtract  => RETROK_KP_MINUS,
        Keycode::NumpadMultiply  => RETROK_KP_MULTIPLY,
        Keycode::NumpadDivide    => RETROK_KP_DIVIDE,
        Keycode::NumpadDecimal   => RETROK_KP_PERIOD,
        Keycode::NumpadEnter     => RETROK_KP_ENTER,
        Keycode::NumpadEquals    => RETROK_KP_EQUALS,

        // Navigation cluster.
        Keycode::Up       => RETROK_UP,
        Keycode::Down     => RETROK_DOWN,
        Keycode::Left     => RETROK_LEFT,
        Keycode::Right    => RETROK_RIGHT,
        Keycode::PageUp   => RETROK_PAGEUP,
        Keycode::PageDown => RETROK_PAGEDOWN,
        Keycode::Home     => RETROK_HOME,
        Keycode::End      => RETROK_END,
        Keycode::Insert   => RETROK_INSERT,
        Keycode::Delete   => RETROK_DELETE,

        // Modifier keys. macOS's LOption / ROption map to ALT; Command /
        // RCommand map to META (Windows uses Win key as META too). LMeta
        // / RMeta on Windows are the Win keys directly.
        Keycode::LShift   => RETROK_LSHIFT,
        Keycode::RShift   => RETROK_RSHIFT,
        Keycode::LControl => RETROK_LCTRL,
        Keycode::RControl => RETROK_RCTRL,
        Keycode::LAlt     => RETROK_LALT,
        Keycode::RAlt     => RETROK_RALT,
        Keycode::LMeta    => RETROK_LMETA,
        Keycode::RMeta    => RETROK_RMETA,
        Keycode::LOption  => RETROK_LALT,
        Keycode::ROption  => RETROK_RALT,
        Keycode::Command  => RETROK_LMETA,
        Keycode::RCommand => RETROK_RMETA,

        // Editing + whitespace.
        Keycode::Backspace => RETROK_BACKSPACE,
        Keycode::Tab       => RETROK_TAB,
        Keycode::CapsLock  => RETROK_CAPSLOCK,
        Keycode::Enter     => RETROK_RETURN,
        Keycode::Escape    => RETROK_ESCAPE,
        Keycode::Space     => RETROK_SPACE,

        // Punctuation on the main keyboard. The names differ between the
        // two crates (`Apostrophe` vs `RETROK_QUOTE`, `Dot` vs `RETROK_
        // PERIOD`, `Grave` vs `RETROK_BACKQUOTE`) but the keys are the
        // same physical position.
        Keycode::Apostrophe   => RETROK_QUOTE,
        Keycode::BackSlash    => RETROK_BACKSLASH,
        Keycode::Comma        => RETROK_COMMA,
        Keycode::Dot          => RETROK_PERIOD,
        Keycode::Equal        => RETROK_EQUALS,
        Keycode::Grave        => RETROK_BACKQUOTE,
        Keycode::LeftBracket  => RETROK_LEFTBRACKET,
        Keycode::Minus        => RETROK_MINUS,
        Keycode::RightBracket => RETROK_RIGHTBRACKET,
        Keycode::Semicolon    => RETROK_SEMICOLON,
        Keycode::Slash        => RETROK_SLASH,
    }
}

/// Build the libretro `key_modifiers` bitmask from a list of currently-
/// held keys. Combines `RETROKMOD_*` flags for any modifier key present.
/// Includes the lock-key flags (NumLock / CapsLock / ScrollLock) so cores
/// that care about lock state get accurate reads — `device_query` reports
/// the lock keys via their dedicated variants.
pub fn modifiers_from_held(held: &[Keycode]) -> u16 {
    let mut m: u16 = RETROKMOD_NONE;
    for k in held {
        match k {
            Keycode::LShift | Keycode::RShift           => m |= RETROKMOD_SHIFT,
            Keycode::LControl | Keycode::RControl       => m |= RETROKMOD_CTRL,
            Keycode::LAlt | Keycode::RAlt
            | Keycode::LOption | Keycode::ROption       => m |= RETROKMOD_ALT,
            Keycode::LMeta | Keycode::RMeta
            | Keycode::Command | Keycode::RCommand      => m |= RETROKMOD_META,
            Keycode::CapsLock                           => m |= RETROKMOD_CAPSLOCK,
            _ => {}
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_map_to_lowercase_ascii() {
        // libretro uses the lowercase ASCII codepoint for letter keys.
        assert_eq!(keycode_to_retro_key(Keycode::A), 97);
        assert_eq!(keycode_to_retro_key(Keycode::Z), 122);
        // Round-trip: every letter is exactly 'a' + offset.
        for (i, k) in [
            Keycode::A, Keycode::B, Keycode::C, Keycode::D, Keycode::E,
            Keycode::F, Keycode::G, Keycode::H, Keycode::I, Keycode::J,
            Keycode::K, Keycode::L, Keycode::M, Keycode::N, Keycode::O,
            Keycode::P, Keycode::Q, Keycode::R, Keycode::S, Keycode::T,
            Keycode::U, Keycode::V, Keycode::W, Keycode::X, Keycode::Y,
            Keycode::Z,
        ].iter().enumerate() {
            assert_eq!(keycode_to_retro_key(*k), 97 + i as u32);
        }
    }

    #[test]
    fn digits_map_to_ascii() {
        assert_eq!(keycode_to_retro_key(Keycode::Key0), 48);
        assert_eq!(keycode_to_retro_key(Keycode::Key9), 57);
    }

    #[test]
    fn function_keys_step_by_one() {
        assert_eq!(keycode_to_retro_key(Keycode::F1),  RETROK_F1);
        assert_eq!(keycode_to_retro_key(Keycode::F2),  RETROK_F2);
        assert_eq!(keycode_to_retro_key(Keycode::F15), RETROK_F15);
        // F16+ have no libretro equivalent. They fall through to UNKNOWN
        // (== 0). Locking this prevents a future contributor from blindly
        // extrapolating F16 = 297 etc., which would silently misroute keys
        // a few drivers actually use.
        assert_eq!(keycode_to_retro_key(Keycode::F16), RETROK_UNKNOWN);
        assert_eq!(keycode_to_retro_key(Keycode::F20), RETROK_UNKNOWN);
    }

    #[test]
    fn numpad_maps_to_kp_keyspace() {
        // The whole point of having a separate KP* range is to distinguish
        // the keypad from the main row. Lock these so a future "simplify"
        // pass doesn't collapse them.
        assert_eq!(keycode_to_retro_key(Keycode::Numpad0), RETROK_KP0);
        assert_eq!(keycode_to_retro_key(Keycode::Numpad9), RETROK_KP9);
        assert_ne!(keycode_to_retro_key(Keycode::Numpad0), keycode_to_retro_key(Keycode::Key0));
        assert_eq!(keycode_to_retro_key(Keycode::NumpadEnter), RETROK_KP_ENTER);
        assert_ne!(keycode_to_retro_key(Keycode::NumpadEnter), keycode_to_retro_key(Keycode::Enter));
    }

    #[test]
    fn modifiers_distinguish_left_and_right() {
        assert_eq!(keycode_to_retro_key(Keycode::LShift), RETROK_LSHIFT);
        assert_eq!(keycode_to_retro_key(Keycode::RShift), RETROK_RSHIFT);
        assert_ne!(keycode_to_retro_key(Keycode::LShift), keycode_to_retro_key(Keycode::RShift));
        // macOS Option key folds into ALT — the OS-side mapping for the
        // same physical key is different across platforms, but the
        // libretro side normalizes to ALT either way.
        assert_eq!(keycode_to_retro_key(Keycode::LOption), RETROK_LALT);
        assert_eq!(keycode_to_retro_key(Keycode::Command), RETROK_LMETA);
    }

    #[test]
    fn navigation_cluster_maps() {
        assert_eq!(keycode_to_retro_key(Keycode::Up), RETROK_UP);
        assert_eq!(keycode_to_retro_key(Keycode::Down), RETROK_DOWN);
        assert_eq!(keycode_to_retro_key(Keycode::PageUp), RETROK_PAGEUP);
        assert_eq!(keycode_to_retro_key(Keycode::Delete), RETROK_DELETE);
    }

    #[test]
    fn punctuation_maps_to_ascii_codepoints() {
        // Sanity-check the symbols where the two crates disagree on the name.
        assert_eq!(keycode_to_retro_key(Keycode::Apostrophe), 39); // '
        assert_eq!(keycode_to_retro_key(Keycode::Dot), 46);        // .
        assert_eq!(keycode_to_retro_key(Keycode::Grave), 96);      // `
        assert_eq!(keycode_to_retro_key(Keycode::Comma), 44);      // ,
    }

    #[test]
    fn modifier_bitmask_combines() {
        // Empty set → no modifiers.
        assert_eq!(modifiers_from_held(&[]), RETROKMOD_NONE);
        // Shift alone.
        assert_eq!(modifiers_from_held(&[Keycode::LShift]), RETROKMOD_SHIFT);
        // Ctrl+Alt combo lights both bits.
        let m = modifiers_from_held(&[Keycode::LControl, Keycode::RAlt]);
        assert_eq!(m & RETROKMOD_CTRL, RETROKMOD_CTRL);
        assert_eq!(m & RETROKMOD_ALT, RETROKMOD_ALT);
        assert_eq!(m & RETROKMOD_SHIFT, 0);
        // Mac Option folds into ALT alongside RAlt (same bit).
        let m = modifiers_from_held(&[Keycode::LOption, Keycode::RAlt]);
        assert_eq!(m, RETROKMOD_ALT);
        // Non-modifier keys don't contribute bits.
        assert_eq!(modifiers_from_held(&[Keycode::A, Keycode::Space]), RETROKMOD_NONE);
        // CapsLock contributes its lock bit.
        assert_eq!(modifiers_from_held(&[Keycode::CapsLock]), RETROKMOD_CAPSLOCK);
    }

    #[test]
    fn unknown_returns_zero_not_garbage() {
        // Variants without a libretro mapping must return RETROK_UNKNOWN
        // (== 0). A core seeing keycode 0 will treat it as "no key" — safe.
        // A core seeing a random non-zero value could fire an unrelated
        // game action, which would be hard to debug.
        assert_eq!(RETROK_UNKNOWN, 0);
        assert_eq!(keycode_to_retro_key(Keycode::F18), RETROK_UNKNOWN);
    }
}
