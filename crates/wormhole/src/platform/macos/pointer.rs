//! Core Graphics' side of `platform::pointer`: the pointer and the screens in one global space, which
//! `wormhole_core::view::tear_off` turns into a screen and a place on it.

use std::ffi::c_void;

use objc2::MainThreadMarker;
use objc2_app_kit::NSWindow;
use objc2_core_foundation::{CGPoint, CGRect};
use objc2_foundation::NSPoint;
use wormhole_core::view::panes::Rect;
use wormhole_core::view::tear_off::{Screen, locate};

use crate::platform::pointer::Pointer;

/// The most displays asked about: no Mac has more connected.
const MAX_DISPLAYS: usize = 32;

#[allow(unsafe_code)] // Core Graphics C calls; the SAFETY notes are where they are made.
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventCreate(source: *const c_void) -> *mut c_void;
    fn CGEventGetLocation(event: *const c_void) -> CGPoint;
    fn CGGetActiveDisplayList(max: u32, displays: *mut u32, count: *mut u32) -> i32;
    fn CGDisplayBounds(display: u32) -> CGRect;
    fn CGMainDisplayID() -> u32;
}

#[allow(unsafe_code)] // As above.
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(cf: *const c_void);
}

/// The active displays, in the global space (y up).
#[allow(unsafe_code)] // Core Graphics C calls; see the SAFETY notes below.
pub fn screens() -> Vec<Screen> {
    let mut ids = [0u32; MAX_DISPLAYS];
    let mut count = 0u32;
    // SAFETY: writes at most `MAX_DISPLAYS` ids into `ids` and the number written into `count`.
    let error = unsafe { CGGetActiveDisplayList(MAX_DISPLAYS as u32, ids.as_mut_ptr(), &mut count) };
    if error != 0 {
        return Vec::new();
    }
    // SAFETY: plain queries by display id, which can be no longer valid only if it was unplugged since the
    // list above, when Core Graphics answers with an empty rectangle.
    let (main_height, bounds) = unsafe {
        let main = CGDisplayBounds(CGMainDisplayID());
        let bounds: Vec<(u32, CGRect)> =
            ids[..(count as usize).min(MAX_DISPLAYS)].iter().map(|&id| (id, CGDisplayBounds(id))).collect();
        (main.size.height, bounds)
    };
    // Core Graphics' space counts y down from the primary screen's top, AppKit's up from its bottom.
    bounds
        .iter()
        .map(|(id, b)| Screen {
            id: u64::from(*id),
            frame: Rect::new(
                b.origin.x as f32,
                (main_height - b.origin.y - b.size.height) as f32,
                b.size.width as f32,
                b.size.height as f32,
            ),
        })
        .collect()
}

#[allow(unsafe_code)] // Core Graphics C calls; see the SAFETY notes below.
pub fn on_screen() -> Option<Pointer> {
    // SAFETY: CGEventCreate(NULL) returns an owned event (or null) that is released here, once; its
    // location is in Core Graphics' global space (points from the primary screen's top left, y down).
    let at = unsafe {
        let event = CGEventCreate(std::ptr::null());
        if event.is_null() {
            return None;
        }
        let at = CGEventGetLocation(event);
        CFRelease(event);
        at
    };
    let screens = screens();
    // The primary screen is the one at the origin, whose height turns y round.
    let main_height = screens.iter().find(|screen| screen.frame.x == 0.0 && screen.frame.y == 0.0)?.frame.h;
    let global = (at.x as f32, main_height - at.y as f32);
    Some(Pointer { global, spot: locate(global.0, global.1, &screens)? })
}

/// The number of the front-most window at the global point, of any application.
pub fn window_number_at(global: (f32, f32)) -> Option<i64> {
    let mtm = MainThreadMarker::new()?;
    let at = NSPoint::new(f64::from(global.0), f64::from(global.1));
    Some(NSWindow::windowNumberAtPoint_belowWindowWithWindowNumber(at, 0, mtm) as i64)
}
