//! What the quick panel needs from AppKit and Core Graphics beyond GPUI's window: which display
//! the pointer is on, a panel without the title bar's buttons, and growing or shrinking from the
//! top (AppKit keeps a window's bottom edge where it is).

use std::ffi::c_void;

use crate::platform::macos::appkit::ns_window;
use gpui_kit::Window;
use objc2_app_kit::{NSWindow, NSWindowButton};
use objc2_core_foundation::CGPoint;
use objc2_foundation::NSRect;

/// No traffic lights on a panel: GPUI gives a pop-up window a (transparent) title bar.
pub(super) fn prepare(window: &Window) {
    let Some(ns_window) = ns_window(window) else { return };
    for button in [NSWindowButton::CloseButton, NSWindowButton::MiniaturizeButton, NSWindowButton::ZoomButton]
    {
        if let Some(button) = ns_window.standardWindowButton(button) {
            button.setHidden(true);
        }
    }
}

/// Make the panel `height` tall, keeping its top edge where it is. Call it outside any update
/// of the panel's window (from a task, as GPUI's own `resize` does): AppKit tells the view about
/// its new size at once, and GPUI only hears that while the window isn't being updated.
pub(super) fn set_height_keeping_top(ns_window: &NSWindow, height: f32) {
    let frame = ns_window.frame();
    let content = ns_window.contentRectForFrameRect(frame);
    let chrome = frame.size.height - content.size.height;
    let new_height = f64::from(height) + chrome;
    let top = frame.origin.y + frame.size.height;
    let mut next = NSRect::new(frame.origin, frame.size);
    next.size.height = new_height;
    next.origin.y = top - new_height;
    ns_window.setFrame_display(next, true);
}

/// The Core Graphics id of the display the pointer is on.
#[allow(unsafe_code)] // Core Graphics C calls; see the SAFETY note below.
pub(super) fn pointer_display_id() -> Option<u64> {
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventCreate(source: *const c_void) -> *mut c_void;
        fn CGEventGetLocation(event: *const c_void) -> CGPoint;
        fn CGGetDisplaysWithPoint(point: CGPoint, max: u32, displays: *mut u32, count: *mut u32) -> i32;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFRelease(cf: *const c_void);
    }
    // SAFETY: CGEventCreate(NULL) returns an owned event (or null) that is released below; its
    // location is in global display coordinates, which is what CGGetDisplaysWithPoint takes, and
    // it writes at most `max` (1) ids into `display`.
    unsafe {
        let event = CGEventCreate(std::ptr::null());
        if event.is_null() {
            return None;
        }
        let point = CGEventGetLocation(event);
        CFRelease(event);
        let (mut display, mut count) = (0u32, 0u32);
        let error = CGGetDisplaysWithPoint(point, 1, &mut display, &mut count);
        (error == 0 && count == 1).then_some(u64::from(display))
    }
}
