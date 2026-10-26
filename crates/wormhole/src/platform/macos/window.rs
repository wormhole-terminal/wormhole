//! AppKit's side of `platform::window`: the traffic lights, the window's frame, minimising, full
//! screen and ordering.

use std::cell::Cell;
use std::ptr::NonNull;

use gpui_kit::Window;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSNormalWindowLevel, NSView, NSWindow, NSWindowAnimationBehavior, NSWindowButton,
    NSWindowCollectionBehavior, NSWindowDidEnterFullScreenNotification,
    NSWindowDidExitFullScreenNotification, NSWindowStyleMask, NSWindowWillEnterFullScreenNotification,
    NSWindowWillExitFullScreenNotification,
};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSOperationQueue, NSPoint};
use wormhole_core::view::motion::live_resize::{Dragged, settled_frame, tiled};
use wormhole_core::view::motion::window_animation::Behaviour;

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

pub fn set_global_top_left(window: &Window, x: f32, y: f32) {
    if let Some(ns_window) = ns_window(window) {
        ns_window.setFrameTopLeftPoint(NSPoint::new(f64::from(x), f64::from(y)));
    }
}

pub fn make_ghost(window: &Window) {
    let Some(ns_window) = ns_window(window) else { return };
    ns_window.setIgnoresMouseEvents(true);
    ns_window.setHasShadow(false);
    // Transient and IgnoresCycle keep it out of Mission Control and the window cycle; FullScreenAuxiliary
    // lets it sit over a full-screen Space; CanJoinAllSpaces keeps it on the Space the drag is on.
    ns_window.setCollectionBehavior(
        NSWindowCollectionBehavior::Transient
            | NSWindowCollectionBehavior::IgnoresCycle
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::CanJoinAllSpaces,
    );
}

pub fn number(window: &Window) -> Option<i64> {
    ns_window(window).map(|ns_window| ns_window.windowNumber() as i64)
}

pub fn set_shadow(window: &Window, shadow: bool) {
    if let Some(ns_window) = ns_window(window) {
        ns_window.setHasShadow(shadow);
        if shadow {
            ns_window.invalidateShadow();
        }
    }
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
        cut_document_animation(&ns_window);
        ns_window.orderOut(None);
    }
}

pub fn show(window: &Window) {
    if let Some(ns_window) = ns_window(window) {
        cut_document_animation(&ns_window);
        ns_window.orderFront(None);
    }
}

/// Taking windows off the screen and putting them back (a profile that is switched) is not their opening or
/// closing: a window that has the document animation is given none first. It is not given back here, since
/// AppKit may read it after this returns; the window's tick gives it again (`Workspace::sync_window_animation`).
fn cut_document_animation(window: &NSWindow) {
    if window.animationBehavior() == NSWindowAnimationBehavior::DocumentWindow {
        window.setAnimationBehavior(NSWindowAnimationBehavior::None);
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

pub const HAS_SYSTEM_ANIMATION: bool = true;

pub fn set_animation(window: &Window, behaviour: Behaviour) {
    let Some(ns_window) = ns_window(window) else { return };
    let wanted = match behaviour {
        Behaviour::Default => NSWindowAnimationBehavior::Default,
        Behaviour::System => NSWindowAnimationBehavior::DocumentWindow,
        Behaviour::Cut => NSWindowAnimationBehavior::None,
    };
    if ns_window.animationBehavior() != wanted {
        ns_window.setAnimationBehavior(wanted);
    }
}

#[cfg(feature = "devctl")]
pub fn scaling_state(window: &Window) -> Option<String> {
    let ns_window = ns_window(window)?;
    let steps = ns_window.contentResizeIncrements();
    let behaviour = ns_window.animationBehavior();
    let frame = ns_window.frame();
    Some(format!(
        "increments=({},{}) animation={:?} frame={}x{} zoomed={} live={}",
        steps.width,
        steps.height,
        behaviour,
        frame.size.width,
        frame.size.height,
        ns_window.isZoomed(),
        ns_window.inLiveResize()
    ))
}

/// A window's AppKit handle, for what must happen outside an update (see `platform::window::Native`).
pub struct Native {
    window: Retained<NSWindow>,
    view: Retained<NSView>,
}

impl Native {
    pub fn of(window: &Window) -> Option<Self> {
        Some(Self { window: ns_window(window)?, view: super::appkit::ns_view(window)? })
    }

    /// Draw and present the window's first frame while it is off the screen.
    ///
    /// GPUI draws a frame when AppKit asks the view to display its layer, which a window nobody can see never
    /// does; the display link that would also stops for it. The same request is made here, by hand, from
    /// outside any update of the window or the app (the frame takes both).
    ///
    /// Unverified on a real display: that Metal presents a drawable for a window that is ordered out. If it
    /// doesn't, the first frame is drawn after the window is on the screen, as a window opens without this.
    #[allow(unsafe_code)]
    pub fn paint(&self) {
        let Some(layer) = self.view.layer() else { return };
        // SAFETY: `displayLayer:` is the method GPUI's view class implements for exactly this request (it
        // runs the window's frame callback, as AppKit's own call does); the layer is the view's own, and we
        // are on the main thread (the marker `ns_view` needed).
        unsafe {
            let _: () = objc2::msg_send![&*self.view, displayLayer: &*layer];
        }
    }

    /// Put the window on the screen, in front, with the keys when `focus`.
    pub fn show(&self, focus: bool) {
        if focus {
            self.window.makeKeyAndOrderFront(None);
        } else {
            self.window.orderFront(None);
        }
    }

    pub fn settle(&self, shrink: (f32, f32), dragged: Dragged) -> bool {
        // Full screen and a zoomed window are the system's size, not the person's.
        if self.window.isZoomed() || self.window.styleMask().contains(NSWindowStyleMask::FullScreen) {
            return false;
        }
        let Some(screen) = self.window.screen() else { return false };
        let frame = super::live_resize::frame_of(self.window.frame());
        // Tiled and filled windows are the system's size too.
        if tiled(frame, super::live_resize::frame_of(screen.visibleFrame())) {
            return false;
        }
        let settled = settled_frame(frame, shrink, dragged);
        // Not animated: the person has just let go, and a window moving by itself after it would be a second
        // resize. AppKit lines the frame up with the screen's pixels.
        self.window.setFrame_display(super::live_resize::rect_of(settled), true);
        true
    }
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
