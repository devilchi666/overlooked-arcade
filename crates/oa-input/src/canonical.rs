//! The canonical controller model — the single normalization target for the
//! Controller Identity arc. Mirrors `frontend/src/platform/nav/canonical.ts`;
//! the two MUST stay in sync.
//!
//! Per decision D4 this IS the SDL / Xbox standard layout. Phase 2 maps each
//! physical pad's raw buttons/axes onto these variants (via `controllers.json`);
//! once normalized, the per-system gameplay maps (`default-maps.json`) operate
//! on canonical names and "just work" for any pad — including non-standard
//! ones like the wired Switch Pro.
//!
//! This is the CONTRACT only. Phase 1 introduces no normalization logic; it
//! just pins the vocabulary both layers and all later phases build against.

/// Canonical digital buttons. Face buttons use SDL position names
/// (south/east/west/north) rather than letters to stay unambiguous across
/// Nintendo's physically-swapped A/B/X/Y — the letter→position mapping is a
/// per-profile concern (`controllers.json`), not part of this vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CanonicalButton {
    /// Bottom face (Xbox A).
    South,
    /// Right face (Xbox B).
    East,
    /// Left face (Xbox X).
    West,
    /// Top face (Xbox Y).
    North,
    Up,
    Down,
    Left,
    Right,
    /// Left bumper.
    L1,
    /// Right bumper.
    R1,
    /// Left trigger (also a [`CanonicalAxis`] when analog).
    L2,
    /// Right trigger (also a [`CanonicalAxis`] when analog).
    R2,
    /// Left stick click.
    L3,
    /// Right stick click.
    R3,
    Start,
    /// Back / share.
    Select,
    /// Home / system button.
    Guide,
}

/// Canonical analog axes. Sticks are bipolar (-1..1); triggers are unipolar
/// (0..1) but reuse the `L2`/`R2` names so a digital-only pad can map them to
/// the [`CanonicalButton`] of the same name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CanonicalAxis {
    LeftX,
    LeftY,
    RightX,
    RightY,
    L2,
    R2,
}
