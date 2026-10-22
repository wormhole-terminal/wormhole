//! `platform::keys` with nothing to ask beyond GPUI's modifiers: neither side is known.

#[inline]
pub fn option_sides() -> (bool, bool) {
    (false, false)
}
