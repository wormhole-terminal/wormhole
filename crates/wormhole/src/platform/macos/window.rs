//! AppKit's side of `platform::window`: the traffic lights, the window's frame, minimising, full
//! screen and ordering.

use gpui_kit::{App, Window, point, px};
use objc2_app_kit::{NSNormalWindowLevel, NSWindow, NSWindowButton, NSWindowCollectionBehavior};
use objc2_foundation::NSPoint;

use super::appkit::ns_window;

#[cfg(feature = "devctl")]
pub use lights_look::lights_appearance;
pub use lights_look::{follow_theme, refresh_theme};

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

/// The appearance the traffic lights are drawn in. AppKit takes it from the title bar the buttons
/// sit in, and from the window above that; the window's own is left alone (GPUI hears the system's
/// Light or Dark Mode change through it, and the app's system-look reading depends on that). So the
/// theme's appearance goes on the title bar's container, the same one the blur and glass views
/// follow (`vibrancy::material`), and the lights match the theme when it differs from the system's.
mod lights_look {
    use std::cell::RefCell;
    use std::ptr::NonNull;

    use gpui_kit::Window;
    use objc2::rc::{Retained, Weak};
    use objc2::runtime::{NSObjectProtocol, ProtocolObject};
    use objc2_app_kit::{
        NSAppearance, NSAppearanceCustomization as _, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
        NSWindow, NSWindowButton, NSWindowDidEnterFullScreenNotification,
        NSWindowDidExitFullScreenNotification,
    };
    use objc2_foundation::{NSNotification, NSNotificationCenter, NSOperationQueue};
    use wormhole_core::theme::tokens::Appearance;

    use super::ns_window;
    use crate::platform::macos::vibrancy::material_appearance;

    type Observer = Retained<ProtocolObject<dyn NSObjectProtocol>>;

    thread_local! {
        /// Every window with lights (AppKit lives on the main thread), forgotten as windows close.
        static WINDOWS: RefCell<Vec<Weak<NSWindow>>> = const { RefCell::new(Vec::new()) };
        /// Whether the full screen observers are in; they live as long as the app.
        static WATCHING: RefCell<Vec<Observer>> = const { RefCell::new(Vec::new()) };
    }

    /// Draw `window`'s lights in the theme's appearance from now on, and again whenever it changes.
    pub fn follow_theme(window: &Window) {
        let Some(ns_window) = ns_window(window) else { return };
        apply(&ns_window);
        WINDOWS.with_borrow_mut(|windows| {
            if !windows.iter().any(|weak| weak.load().is_some_and(|known| known == ns_window)) {
                windows.push(Weak::new(&ns_window));
            }
        });
        watch_full_screen();
    }

    /// The theme's appearance changed: every window's lights follow.
    pub fn refresh_theme() {
        WINDOWS.with_borrow_mut(|windows| {
            windows.retain(|weak| {
                let Some(window) = weak.load() else { return false };
                apply(&window);
                true
            });
        });
    }

    /// Full screen moves the buttons into another title bar, which has no appearance of ours.
    #[allow(unsafe_code)]
    fn watch_full_screen() {
        if WATCHING.with_borrow(|tokens| !tokens.is_empty()) {
            return;
        }
        let center = NSNotificationCenter::defaultCenter();
        // SAFETY: notification names AppKit defines, immutable statics.
        let names =
            unsafe { [NSWindowDidEnterFullScreenNotification, NSWindowDidExitFullScreenNotification] };
        let tokens = names.map(|name| {
            let block = block2::RcBlock::new(|_: NonNull<NSNotification>| refresh_theme());
            // SAFETY: no sender filter, and the main queue, so the block runs on the main thread,
            // where the windows are kept.
            unsafe {
                center.addObserverForName_object_queue_usingBlock(
                    Some(name),
                    None,
                    Some(&NSOperationQueue::mainQueue()),
                    &block,
                )
            }
        });
        WATCHING.with_borrow_mut(|watching| watching.extend(tokens));
    }

    /// The view that holds the three buttons' title bar: the close button's grandparent.
    #[allow(unsafe_code)]
    fn container(window: &NSWindow) -> Option<Retained<objc2_app_kit::NSView>> {
        let close = window.standardWindowButton(NSWindowButton::CloseButton)?;
        // SAFETY: AppKit's own view hierarchy, read on the main thread.
        unsafe { close.superview()?.superview() }
    }

    #[allow(unsafe_code)]
    fn apply(window: &NSWindow) {
        let (Some(appearance), Some(container)) = (material_appearance(), container(window)) else {
            return;
        };
        // SAFETY: AppKit's appearance names are immutable statics, set before main runs.
        let name = unsafe {
            match appearance {
                Appearance::Light => NSAppearanceNameAqua,
                Appearance::Dark => NSAppearanceNameDarkAqua,
            }
        };
        container.setAppearance(NSAppearance::appearanceNamed(name).as_deref());
    }

    /// The name of the appearance the lights are drawn in (devctl's `state`).
    #[cfg(feature = "devctl")]
    pub fn lights_appearance(window: &Window) -> Option<String> {
        let container = container(&*ns_window(window)?)?;
        Some(container.effectiveAppearance().name().to_string())
    }
}
