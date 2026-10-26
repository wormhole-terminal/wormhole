//! AppKit's side of `platform::window`: the traffic lights, the window's frame, minimising, full
//! screen and ordering.

use std::cell::Cell;
use std::ptr::NonNull;

use gpui_kit::Window;
use objc2_app_kit::{
    NSNormalWindowLevel, NSWindow, NSWindowButton, NSWindowCollectionBehavior,
    NSWindowDidEnterFullScreenNotification, NSWindowDidExitFullScreenNotification,
    NSWindowWillEnterFullScreenNotification, NSWindowWillExitFullScreenNotification,
};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSOperationQueue, NSPoint};

use super::appearance;
use super::appkit::ns_window;

pub fn lights_span(window: &Window) -> Option<f32> {
    let ns_window = ns_window(window)?;
    let close = ns_window.standardWindowButton(NSWindowButton::CloseButton)?.frame();
    let zoom = ns_window.standardWindowButton(NSWindowButton::ZoomButton)?.frame();
    let span = zoom.origin.x + zoom.size.width - close.origin.x;
    (span > 0.0).then_some(span as f32)
}

pub fn set_lights_shown(window: &Window, shown: bool) {
    if let Some(ns_window) = ns_window(window) {
        set_lights_opacity(&ns_window, if shown { 1.0 } else { 0.0 });
    }
}

/// How opaque the three lights are: 0 takes them out of sight and out of reach, 1 is as AppKit
/// draws them. The intro's run fades them away as a replay begins.
pub fn set_lights_opacity(window: &NSWindow, opacity: f32) {
    for kind in [NSWindowButton::CloseButton, NSWindowButton::MiniaturizeButton, NSWindowButton::ZoomButton] {
        if let Some(button) = window.standardWindowButton(kind) {
            button.setAlphaValue(f64::from(opacity));
            // Hidden as well, so a light nobody can see can't be clicked or Tab'd to.
            button.setHidden(opacity <= 0.0);
        }
    }
}

#[cfg(feature = "devctl")]
pub fn lights_opacity(window: &Window) -> Option<(f32, bool)> {
    let button = ns_window(window)?.standardWindowButton(NSWindowButton::CloseButton)?;
    Some((button.alphaValue() as f32, button.isHidden()))
}

pub fn set_top_left(window: &Window, x: f32, y: f32) {
    let Some(ns_window) = ns_window(window) else { return };
    // The screen the window is on, not "the display at the origin", which every display is for GPUI: AppKit's
    // y grows up from the bottom of the primary screen, so another screen's frame says where its own top is.
    let Some(screen) = ns_window.screen() else { return };
    let frame = screen.frame();
    let top = frame.origin.y + frame.size.height - f64::from(y);
    ns_window.setFrameTopLeftPoint(NSPoint::new(frame.origin.x + f64::from(x), top));
}

pub fn is_minimised(window: &Window) -> bool {
    ns_window(window).is_some_and(|w| w.isMiniaturized())
}

pub fn allow_fullscreen(window: &Window) {
    if let Some(ns_window) = ns_window(window) {
        ns_window.setCollectionBehavior(
            ns_window.collectionBehavior() | NSWindowCollectionBehavior::FullScreenPrimary,
        );
    }
}

pub fn deminiaturize(window: &Window) {
    if let Some(ns_window) = ns_window(window) {
        ns_window.deminiaturize(None);
    }
}

pub fn hide(window: &Window) {
    if let Some(ns_window) = ns_window(window) {
        ns_window.orderOut(None);
    }
}

pub fn show(window: &Window) {
    if let Some(ns_window) = ns_window(window) {
        ns_window.orderFront(None);
    }
}

pub fn order_back(window: &Window) {
    if let Some(ns_window) = ns_window(window) {
        ns_window.orderBack(None);
    }
}

pub fn keep_behind(window: &Window) {
    let Some(ns_window) = ns_window(window) else { return };
    // A pop-up (the quick panel) floats over every app's windows at its own level.
    if ns_window.level() > NSNormalWindowLevel {
        ns_window.setLevel(NSNormalWindowLevel);
    }
    ns_window.orderBack(None);
}

/// Draw `window`'s lights in the theme's appearance from now on (see `appearance`), full screen
/// included.
pub fn follow_theme(window: &Window) {
    let Some(ns_window) = ns_window(window) else { return };
    appearance::follow_titlebar(&ns_window);
    watch_full_screen();
}

/// Full screen moves the buttons into another title bar, which has none of the theme's appearance.
///
/// Unverified: that AppKit makes that title bar a new view (so that the appearance is lost) rather
/// than keeping one. A full screen window can't be tried in a test instance, which would take over
/// the display; so the title bars are put right when a transition begins as well as when it ends,
/// which costs nothing if the view stays and keeps the system's look from showing during the
/// transition if it doesn't.
#[allow(unsafe_code)]
fn watch_full_screen() {
    thread_local! {
        /// Whether the observers are in; they live as long as the app, held by the notification
        /// centre, which keeps a block observer until it is removed.
        static WATCHING: Cell<bool> = const { Cell::new(false) };
    }
    if WATCHING.replace(true) {
        return;
    }
    let center = NSNotificationCenter::defaultCenter();
    // SAFETY: notification names AppKit defines, immutable statics.
    let names = unsafe {
        [
            NSWindowWillEnterFullScreenNotification,
            NSWindowDidEnterFullScreenNotification,
            NSWindowWillExitFullScreenNotification,
            NSWindowDidExitFullScreenNotification,
        ]
    };
    for name in names {
        let block = block2::RcBlock::new(|_: NonNull<NSNotification>| appearance::refresh_titlebars());
        // SAFETY: no sender filter, and the main queue, so the block runs on the main thread, where
        // the windows are kept.
        let _observer = unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(name),
                None,
                Some(&NSOperationQueue::mainQueue()),
                &block,
            )
        };
    }
}

/// The appearance the lights are drawn in, by AppKit's name, then where the close light and its
/// title bar container sit (devctl's `state`): AppKit may lay the lights out again when an
/// appearance is applied, and this is how that shows.
#[cfg(feature = "devctl")]
pub fn lights_state(window: &Window) -> Option<String> {
    use objc2_app_kit::NSAppearanceCustomization as _;

    let ns_window = ns_window(window)?;
    let container = appearance::titlebar_container(&ns_window)?;
    let close = ns_window.standardWindowButton(NSWindowButton::CloseButton)?.frame();
    let at = container.frame();
    Some(format!(
        "{} close=({},{}) container=({},{} {}x{})",
        container.effectiveAppearance().name(),
        close.origin.x,
        close.origin.y,
        at.origin.x,
        at.origin.y,
        at.size.width,
        at.size.height
    ))
}
