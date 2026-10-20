//! Pieces of AppKit that more than one native feature needs: a GPUI window's view, SF Symbols,
//! colours and key equivalents. The status item's menu (`menubar::appkit`) uses them too.

use std::path::Path;

use gpui_kit::{Pixels, Point, Window};
use objc2::rc::Retained;
use objc2::{MainThreadMarker, Message as _};
use objc2_app_kit::{NSColor, NSEventModifierFlags, NSImage, NSView, NSWorkspace};
use objc2_foundation::{NSCopying as _, NSFileManager, NSPoint, NSSize, NSString};
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

/// The icon Finder shows for the file or folder at `path`, `side` points square. AppKit finds it
/// (the house for the home folder, a disk for a volume, the folder's own custom one); nothing is
/// read here, and a path that does not exist gets the generic folder's.
pub fn file_icon(path: &Path, side: f64) -> Option<Retained<NSImage>> {
    MainThreadMarker::new()?;
    let icon = NSWorkspace::sharedWorkspace().iconForFile(&NSString::from_str(&path.to_string_lossy()));
    // The workspace shares its images: size a copy, not the cached one.
    let icon = icon.copy();
    icon.setSize(NSSize::new(side, side));
    Some(icon)
}

/// What Finder calls the volume the system starts from ("Macintosh HD").
pub fn boot_volume_name() -> Option<String> {
    MainThreadMarker::new()?;
    let name = NSFileManager::defaultManager().displayNameAtPath(&NSString::from_str("/")).to_string();
    (!name.is_empty()).then_some(name)
}
