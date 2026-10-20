//! Pieces of AppKit that more than one native feature needs: a GPUI window's view, SF Symbols,
//! colours and key equivalents. The status item's menu (`menubar::appkit`) uses them too.

use gpui_kit::{Pixels, Point, Window};
use objc2::rc::Retained;
use objc2::{MainThreadMarker, Message as _};
use objc2_app_kit::{NSColor, NSEventModifierFlags, NSImage, NSView};
use objc2_foundation::{NSPoint, NSString};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::menubar::model::KeyEquivalent;

/// GPUI's view of `window`, which a menu pops up in. `None` off the main thread and for a window
/// that isn't an AppKit one (a test window).
#[allow(unsafe_code)] // AppKit interop; see the SAFETY note below.
pub fn ns_view(window: &Window) -> Option<Retained<NSView>> {
    MainThreadMarker::new()?;
    let RawWindowHandle::AppKit(handle) = HasWindowHandle::window_handle(window).ok()?.as_raw() else {
        return None;
    };
    // SAFETY: GPUI hands out its live NSView, and we are on the main thread. The reference lives
    // for this call only; `retain` takes the view's own count.
    let view: &NSView = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
    Some(view.retain())
}

/// `at`, a point in the window as GPUI has it (logical pixels from the top left), in `view`'s own
/// coordinates.
pub fn location(view: &NSView, at: Point<Pixels>) -> NSPoint {
    let (x, y) = (f64::from(f32::from(at.x)), f64::from(f32::from(at.y)));
    let y = if view.isFlipped() { y } else { view.bounds().size.height - y };
    NSPoint::new(x, y)
}

pub fn symbol(name: &str, description: &str) -> Option<Retained<NSImage>> {
    NSImage::imageWithSystemSymbolName_accessibilityDescription(
        &NSString::from_str(name),
        Some(&NSString::from_str(description)),
    )
}

/// An sRGB colour, each channel 0–1.
pub fn ns_color(rgba: [f32; 4]) -> Retained<NSColor> {
    let [r, g, b, a] = rgba.map(f64::from);
    NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, a)
}

pub fn modifier_mask(key: &KeyEquivalent) -> NSEventModifierFlags {
    let mut mask = NSEventModifierFlags::empty();
    for (on, flag) in [
        (key.command, NSEventModifierFlags::Command),
        (key.option, NSEventModifierFlags::Option),
        (key.control, NSEventModifierFlags::Control),
        (key.shift, NSEventModifierFlags::Shift),
    ] {
        if on {
            mask |= flag;
        }
    }
    mask
}
