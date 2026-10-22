//! What the system says about how windows should look: light or dark, and the accessibility
//! display options, read and watched. Reduce Transparency and Increase Contrast are what the
//! settings store turns into an opacity of 1 (hiding every blur) and the high-contrast tokens;
//! Reduce Motion is what GPUI's animations follow. Tests never ask the real system: there,
//! nothing is known and nothing is watched.

use wormhole_core::tokens::Appearance;

use super::os::look as os;
pub use super::os::look::DisplayObserver;

/// The system's appearance, light or dark. `None` where it can't be asked (off macOS, in tests).
pub fn system_appearance() -> Option<Appearance> {
    if cfg!(test) { None } else { os::appearance() }
}

/// Whether Reduce Transparency is on (System Settings → Accessibility → Display).
pub fn reduce_transparency() -> bool {
    !cfg!(test) && os::reduce_transparency()
}

/// Whether Increase Contrast is on (System Settings → Accessibility → Display).
pub fn increase_contrast() -> bool {
    !cfg!(test) && os::increase_contrast()
}

/// Whether Reduce Motion is on (System Settings → Accessibility → Display); `None` where it
/// can't be asked (off macOS, in tests), so GPUI's own flag is left as it is.
pub fn reduce_motion() -> Option<bool> {
    if cfg!(test) { None } else { os::reduce_motion() }
}

/// Calls `changed` on the main thread each time the system's display options change (Reduce
/// Transparency, Increase Contrast, Reduce Motion, …), for as long as the observer it returns
/// is kept.
pub fn observe_display_options(changed: impl Fn() + Send + Sync + 'static) -> Option<DisplayObserver> {
    if cfg!(test) { None } else { os::observe(changed) }
}
