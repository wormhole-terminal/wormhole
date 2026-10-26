//! AppKit's side of `platform::live_resize`: the window's resize steps, and hearing a person take hold of a
//! window's edge and let go of it (`NSWindowWillStartLiveResizeNotification` and
//! `NSWindowDidEndLiveResizeNotification`), which is also how a frame knows a window is being dragged.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ptr::NonNull;
use std::rc::Rc;

use gpui_kit::Window;
use objc2_app_kit::{
    NSWindow, NSWindowDidEndLiveResizeNotification, NSWindowWillStartLiveResizeNotification,
};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSPoint, NSRect, NSSize};
use wormhole_core::view::motion::live_resize::{Event, Frame, dragged};

use super::appkit::ns_window;

thread_local! {
    /// The windows being dragged, by their address, with the frame each had when it began.
    static LIVE: RefCell<HashMap<usize, Frame>> = RefCell::new(HashMap::new());
    /// Whether the observers are in.
    static WATCHING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn key(window: &NSWindow) -> usize {
    std::ptr::from_ref(window) as usize
}

pub(super) fn frame_of(rect: NSRect) -> Frame {
    Frame::new(rect.origin.x as f32, rect.origin.y as f32, rect.size.width as f32, rect.size.height as f32)
}

pub(super) fn rect_of(frame: Frame) -> NSRect {
    NSRect::new(
        NSPoint::new(f64::from(frame.x), f64::from(frame.y)),
        NSSize::new(f64::from(frame.w), f64::from(frame.h)),
    )
}

pub fn set_increments(window: &Window, steps: Option<(f32, f32)>) {
    let Some(ns_window) = ns_window(window) else { return };
    // AppKit keeps whole points only (8.4 is 8, 8.6 is 9, 0.5 is 0, which would be "none"; checked on this
    // system): the steps are rounded here, at least a point, which is also what "no increments" is.
    let step = |points: f32| f64::from(points.round().max(1.0));
    let wanted = steps.map_or(NSSize::new(1.0, 1.0), |(w, h)| NSSize::new(step(w), step(h)));
    if ns_window.contentResizeIncrements() != wanted {
        ns_window.setContentResizeIncrements(wanted);
    }
}

pub fn in_progress(window: &Window) -> bool {
    ns_window(window).is_some_and(|window| LIVE.with(|live| live.borrow().contains_key(&key(&window))))
}

/// Tell `changed` when a window's live resize begins and ends. Once for the app's life; it runs on the main
/// thread, at once, in whatever AppKit is doing (a mouse down on a window's edge): it must not touch the app.
#[allow(unsafe_code)]
pub fn watch(changed: impl Fn(Event) + 'static) {
    if WATCHING.with(|watching| watching.replace(true)) {
        return;
    }
    let changed = Rc::new(changed);
    let center = NSNotificationCenter::defaultCenter();
    // SAFETY: notification names AppKit defines, immutable statics.
    let names = unsafe {
        [(NSWindowWillStartLiveResizeNotification, true), (NSWindowDidEndLiveResizeNotification, false)]
    };
    for (name, started) in names {
        let changed = changed.clone();
        let block = block2::RcBlock::new(move |notification: NonNull<NSNotification>| {
            // SAFETY: the notification AppKit hands the block, alive for the call.
            let notification = unsafe { notification.as_ref() };
            let Some(object) = notification.object() else { return };
            let Some(window) = object.downcast_ref::<NSWindow>() else { return };
            let frame = frame_of(window.frame());
            let number = window.windowNumber() as i64;
            if started {
                LIVE.with(|live| live.borrow_mut().insert(key(window), frame));
                changed(Event::Started { window: number });
            } else {
                let began = LIVE.with(|live| live.borrow_mut().remove(&key(window)));
                let dragged = began.map(|began| dragged(began, frame)).unwrap_or_default();
                changed(Event::Ended { window: number, dragged });
            }
        });
        // SAFETY: no sender filter and no queue: the block runs where AppKit posts, the main thread, where the
        // windows are kept. The centre keeps a block observer until it is removed, which is never.
        let _observer =
            unsafe { center.addObserverForName_object_queue_usingBlock(Some(name), None, None, &block) };
    }
}
