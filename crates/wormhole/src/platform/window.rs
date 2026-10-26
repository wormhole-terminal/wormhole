//! What the app does to a window beyond GPUI's own calls: where the system draws its traffic
//! lights and whether they show, its place on the screen, minimising, full screen, and ordering
//! it in front of or behind other windows (a profile that isn't shown, background mode, a
//! program's window operations).
//!
//! The lights' size and spacing differ between macOS versions (12 pt lights 8 apart on older
//! systems, 14 pt ones 9 apart on later systems), so the chrome that sits after them asks a
//! window instead of assuming one layout. (The close button's dot goes through GPUI's
//! `set_window_edited`, which puts the lights back in place after it.)

use std::sync::OnceLock;

use gpui_kit::Window;

use super::os::window as os;

/// From the first light's left edge to the last one's right edge, as the system lays them out.
/// It is the same for every window, so it is measured once. `None` where the system draws none.
pub fn span(window: &Window) -> Option<f32> {
    static SPAN: OnceLock<f32> = OnceLock::new();
    if let Some(span) = SPAN.get() {
        return Some(*span);
    }
    let span = os::lights_span(window)?;
    Some(*SPAN.get_or_init(|| span))
}

/// Show the window's three lights, or take them out of sight and out of reach: the intro window keeps
/// them away for as long as it is up.
pub fn set_shown(window: &Window, shown: bool) {
    os::set_lights_shown(window, shown);
}

/// Draw `window`'s traffic lights in the theme's appearance (`vibrancy::set_material_appearance`),
/// not the system's: with a light theme in Dark Mode, or a dark one in Light Mode, AppKit would
/// otherwise draw the other mode's lights. Call it for each window that has lights, once it is
/// open; they follow the theme from then on, full screen included.
pub fn follow_theme(window: &Window) {
    os::follow_theme(window);
}

/// The appearance the lights are drawn in, by AppKit's name, and where the close light and its title
/// bar sit (devctl's `state`), `None` where the system draws no lights.
#[cfg(feature = "devctl")]
pub fn lights_state(window: &Window) -> Option<String> {
    os::lights_state(window)
}

/// How opaque the close light is and whether it is hidden (devctl's `state`), `None` where the
/// system draws no lights.
#[cfg(feature = "devctl")]
pub fn opacity(window: &Window) -> Option<(f32, bool)> {
    os::lights_opacity(window)
}

/// Put the window's top left corner at `x`, `y` of the screen it is on, in points from that screen's top
/// left (the space of `Window::bounds`). GPUI has no call for it.
pub fn set_top_left(window: &Window, x: f32, y: f32) {
    os::set_top_left(window, x, y);
}

/// Put the window's top left corner at `x`, `y` of the system's global space (`pointer::Pointer::global`:
/// from the primary screen's bottom left, y up, so `y` is the corner's height above that line). It goes to
/// whichever screen that is on.
pub fn set_global_top_left(window: &Window, x: f32, y: f32) {
    os::set_global_top_left(window, x, y);
}

/// Make `window` something the pointer passes through and that no one switches to: it takes no mouse event
/// and casts no shadow, stays out of the window cycle and out of Mission Control, and it is over a
/// full-screen Space. The chip that follows the pointer during a drag is one: the drag goes on under it.
pub fn make_ghost(window: &Window) {
    os::make_ghost(window);
}

/// Whether the window casts its shadow. A window that grows from a small rectangle in a transparent
/// frame has none while it does, or the shadow would outline the whole frame; the system draws it again
/// from the window's content once it is on.
pub fn set_shadow(window: &Window, shadow: bool) {
    os::set_shadow(window, shadow);
}

/// Whether the window is minimised. AppKit says so only once the minimise animation is done.
pub fn is_minimised(window: &Window) -> bool {
    os::is_minimised(window)
}

/// Let the window enter full screen: GPUI's windows don't say they may, and a program can ask
/// for it (XTWINOPS 10).
pub fn allow_fullscreen(window: &Window) {
    os::allow_fullscreen(window);
}

/// Bring a minimised window back.
pub fn deminiaturize(window: &Window) {
    os::deminiaturize(window);
}

/// Take `window` off the screen without closing it: its view, terminals and agents go on
/// running (a profile that is not shown, `AppState::switch_profile`). GPUI has no call for it.
pub fn hide(window: &Window) {
    os::hide(window);
}

/// Put a window [`hide`] took away back on the screen, in front but not necessarily keyed.
pub fn show(window: &Window) {
    os::show(window);
}

/// Put the window behind the app's other windows (XTWINOPS 6).
pub fn order_back(window: &Window) {
    os::order_back(window);
}

/// Put `window` on screen behind every other window at the normal level, without the focus: a
/// pop-up (the quick panel), which floats over every app's windows, comes down to that level
/// first. Background mode's windows open this way.
pub fn keep_behind(window: &Window) {
    os::keep_behind(window);
}
