//! Where macOS draws a window's traffic lights. Their size and spacing differ between macOS
//! versions (12 pt lights 8 apart on older systems, 14 pt ones 9 apart on later systems), so the
//! chrome that sits after them asks a window instead of assuming one layout. (The close button's
//! dot goes through GPUI's `set_window_edited`, which puts the lights back in place after it.)

use std::sync::OnceLock;

use gpui_kit::Window;

#[cfg(target_os = "macos")]
use super::macos::appkit::ns_window;

/// From the first light's left edge to the last one's right edge, as AppKit lays them out. It is
/// the same for every window, so it is measured once.
pub fn span(window: &Window) -> Option<f32> {
    static SPAN: OnceLock<f32> = OnceLock::new();
    if let Some(span) = SPAN.get() {
        return Some(*span);
    }
    let span = measure(window)?;
    Some(*SPAN.get_or_init(|| span))
}

/// Show the window's three lights, or take them out of sight and out of reach: the intro window keeps
/// them away for as long as it is up.
pub fn set_shown(window: &Window, shown: bool) {
    #[cfg(target_os = "macos")]
    if let Some(ns_window) = ns_window(window) {
        set_opacity(&ns_window, if shown { 1.0 } else { 0.0 });
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (window, shown);
}

/// How opaque the three lights are: 0 takes them out of sight and out of reach, 1 is as AppKit
/// draws them. The intro's run fades them away as a replay begins.
#[cfg(target_os = "macos")]
pub fn set_opacity(window: &objc2_app_kit::NSWindow, opacity: f32) {
    use objc2_app_kit::NSWindowButton;

    for kind in [NSWindowButton::CloseButton, NSWindowButton::MiniaturizeButton, NSWindowButton::ZoomButton] {
        if let Some(button) = window.standardWindowButton(kind) {
            button.setAlphaValue(f64::from(opacity));
            // Hidden as well, so a light nobody can see can't be clicked or Tab'd to.
            button.setHidden(opacity <= 0.0);
        }
    }
}

/// How opaque the close light is and whether it is hidden (devctl's `state`), `None` off macOS.
#[cfg(all(feature = "devctl", target_os = "macos"))]
pub fn opacity(window: &Window) -> Option<(f32, bool)> {
    use objc2_app_kit::NSWindowButton;

    let button = ns_window(window)?.standardWindowButton(NSWindowButton::CloseButton)?;
    Some((button.alphaValue() as f32, button.isHidden()))
}

#[cfg(target_os = "macos")]
fn measure(window: &Window) -> Option<f32> {
    use objc2_app_kit::NSWindowButton;

    let ns_window = ns_window(window)?;
    let close = ns_window.standardWindowButton(NSWindowButton::CloseButton)?.frame();
    let zoom = ns_window.standardWindowButton(NSWindowButton::ZoomButton)?.frame();
    let span = zoom.origin.x + zoom.size.width - close.origin.x;
    (span > 0.0).then_some(span as f32)
}

#[cfg(not(target_os = "macos"))]
fn measure(_: &Window) -> Option<f32> {
    None
}
