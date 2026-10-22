//! AppKit's side of `platform::window`: the traffic lights, the window's frame, minimising, full
//! screen and ordering.

use gpui_kit::{App, Window, point, px};
use objc2_app_kit::{NSNormalWindowLevel, NSWindow, NSWindowButton, NSWindowCollectionBehavior};
use objc2_foundation::NSPoint;

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

pub fn set_top_left(window: &Window, cx: &App, x: f32, y: f32) {
    let Some(ns_window) = ns_window(window) else { return };
    // AppKit counts y up from the bottom of the primary screen; GPUI down from its top.
    let primary = cx.displays().into_iter().find(|d| d.bounds().origin == point(px(0.0), px(0.0)));
    let Some(primary) = primary else { return };
    let top = f64::from(f32::from(primary.bounds().size.height)) - f64::from(y);
    ns_window.setFrameTopLeftPoint(NSPoint::new(f64::from(x), top));
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
