//! The theme's appearance on the AppKit views that draw it themselves: the blur and glass views
//! behind GPUI's view (`vibrancy`) and the title bar the traffic lights sit in (`window`). Both
//! follow the theme, not the system: a light theme in Dark Mode is drawn light, lights included.
//!
//! The window's own appearance is left alone: GPUI hears the system's Light or Dark Mode change
//! through it, and the app's system-look reading depends on that. So the theme's goes on these
//! views, and this module is the one place that knows which, so the two concerns don't need each
//! other.

use std::cell::{Cell, RefCell};
use std::ptr;

use objc2::rc::{Retained, Weak};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization as _, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSView,
    NSWindow, NSWindowButton,
};
use wormhole_core::theme::tokens::Appearance;

thread_local! {
    /// Every window's effect view (AppKit lives on the main thread), forgotten as windows close.
    static EFFECTS: RefCell<Vec<Weak<NSView>>> = const { RefCell::new(Vec::new()) };
    /// Every window with lights. The window, not its title bar container: AppKit swaps the
    /// container for another when the window changes its title bar (full screen), so it is found
    /// again each time.
    static TITLEBARS: RefCell<Vec<Weak<NSWindow>>> = const { RefCell::new(Vec::new()) };
    static CURRENT: Cell<Option<Appearance>> = const { Cell::new(None) };
}

/// AppKit's appearance for `appearance`.
#[allow(unsafe_code)]
pub(super) fn ns_appearance(appearance: Appearance) -> Option<Retained<NSAppearance>> {
    // SAFETY: AppKit's appearance names are immutable statics, set before main runs.
    let name = unsafe {
        match appearance {
            Appearance::Light => NSAppearanceNameAqua,
            Appearance::Dark => NSAppearanceNameDarkAqua,
        }
    };
    NSAppearance::appearanceNamed(name)
}

/// The appearance currently applied to the app's native window materials.
pub(super) fn current() -> Option<Retained<NSAppearance>> {
    CURRENT.get().and_then(ns_appearance)
}

/// The theme's appearance is now `appearance`: every view that follows it is drawn that way. Does
/// nothing when it is the one already set, which is most settings changes.
pub(super) fn set(appearance: Appearance) {
    if CURRENT.replace(Some(appearance)) == Some(appearance) {
        return;
    }
    EFFECTS.with_borrow_mut(|effects| {
        effects.retain(|weak| {
            let Some(effect) = weak.load() else { return false };
            apply(&effect, appearance);
            true
        });
    });
    refresh_titlebars();
}

/// A blur or glass view that follows the theme's appearance from now on.
pub(super) fn follow_effect(effect: &NSView) {
    if let Some(appearance) = CURRENT.get() {
        apply(effect, appearance);
    }
    EFFECTS.with_borrow_mut(|effects| {
        effects.retain(|weak| weak.load().is_some());
        effects.push(Weak::new(effect));
    });
}

/// `appearance` on `view` and everything in it, once: for a view that lives only a moment (the notice
/// chip), so it need not be remembered and followed.
pub(super) fn set_on(view: &NSView, appearance: Appearance) {
    apply(view, appearance);
}

/// `window`'s title bar (and with it the traffic lights) follows the theme's appearance from now on.
pub(super) fn follow_titlebar(window: &NSWindow) {
    apply_titlebar(window);
    TITLEBARS.with_borrow_mut(|windows| {
        windows.retain(|weak| weak.load().is_some());
        if !windows.iter().any(|weak| weak.load().is_some_and(|known| ptr::eq(&*known, window))) {
            windows.push(Weak::new(window));
        }
    });
}

/// Put the theme's appearance on every title bar again, where a window's changed: full screen moves
/// the buttons into another title bar, which has none of ours.
pub(super) fn refresh_titlebars() {
    TITLEBARS.with_borrow_mut(|windows| {
        windows.retain(|weak| {
            let Some(window) = weak.load() else { return false };
            apply_titlebar(&window);
            true
        });
    });
}

/// The view that holds the three buttons' title bar: the close button's grandparent.
#[allow(unsafe_code)]
pub(super) fn titlebar_container(window: &NSWindow) -> Option<Retained<NSView>> {
    let close = window.standardWindowButton(NSWindowButton::CloseButton)?;
    // SAFETY: AppKit's own view hierarchy, read on the main thread.
    unsafe { close.superview()?.superview() }
}

fn apply_titlebar(window: &NSWindow) {
    if let (Some(appearance), Some(container)) = (CURRENT.get(), titlebar_container(window)) {
        apply(&container, appearance);
    }
}

/// Set `appearance` on `view`, unless it has it: AppKit lays a view out and draws it again for any
/// appearance set, even the same one.
fn apply(view: &NSView, appearance: Appearance) {
    let Some(wanted) = ns_appearance(appearance) else { return };
    if view.appearance().is_some_and(|set| set == wanted) {
        return;
    }
    view.setAppearance(Some(&wanted));
}
