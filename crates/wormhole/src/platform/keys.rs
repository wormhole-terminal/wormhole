//! What a key event says beyond GPUI's modifiers.

use super::os::keys as os;

/// Which Option keys are held right now: (left, right). GPUI's modifiers say only that one is,
/// and "Option as Meta" can be set per side, so this asks the system for the device-dependent
/// flags of the key event being handled. Neither side (no such flags, a synthetic key) leaves the
/// choice to `OptionAsMeta::is_meta`. Asked for each key that has Option: a flag read, no more.
#[inline]
pub fn option_sides() -> (bool, bool) {
    os::option_sides()
}
