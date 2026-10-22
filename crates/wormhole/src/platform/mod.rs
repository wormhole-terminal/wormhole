//! What the platform draws or does itself, behind one API for the app, so a port to another
//! platform adds a sibling folder to `macos/` and nothing else changes: `menu` is what a
//! menu is (platform-neutral), `macos/` its AppKit implementation (the only code here with
//! `objc2` and `unsafe`), `fallback/` GPUI's own for platforms with no native one, which is also
//! what a window without a native view gets.
//!
//! Context menus and popovers (`popover`: the folder's hover card, a group's name card) live here, and
//! the system's colour panel (`pick_colour`). The dialogs, the status item's menu (`menubar`), the
//! window's blur (`vibrancy`), the traffic lights, Touch ID (`auth`) and the rest of what is
//! AppKit's are meant to move here one by one, each as a neutral file beside `menu.rs` and an
//! implementation in `macos/`.

pub mod a11y;
mod anchor;
pub mod auth;
pub mod dock;
pub mod fallback;
pub mod hotkey;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod menu;
pub mod popover;
#[cfg(any(test, feature = "devctl"))]
pub mod record;
pub mod scroller;
pub mod vibrancy;
pub mod window;

use gpui_kit::*;

pub use anchor::{Anchor, on_right_click};
pub use menu::{Icon, Item, Menu, Swatch, run_on};
pub use popover::{ColourChange, Handle, Popover};

/// What Finder calls the volume the system starts from, as the last line of a path menu.
pub fn boot_volume_name() -> String {
    #[cfg(target_os = "macos")]
    if let Some(name) = macos::appkit::boot_volume_name() {
        return name;
    }
    "/".to_string()
}

/// Open `menu` at `at` (window coordinates) as the platform's own menu, and run what is chosen.
///
/// Returns at once: the menu opens, and its item runs, a moment later, outside the borrow of the
/// app that the caller (a mouse handler, an update) is in. See `macos::menu` for why that matters.
/// Call it from anywhere with a `Window` and an `App`, once per menu; an empty menu opens nothing.
pub fn show_menu(menu: Menu, at: Point<Pixels>, window: &mut Window, cx: &mut App) {
    if menu.is_empty() {
        return;
    }
    #[cfg(any(test, feature = "devctl"))]
    if record::keep(&menu, window) {
        return;
    }
    #[cfg(target_os = "macos")]
    let menu = match macos::menu::show(menu, at, window, cx) {
        Ok(()) => return,
        Err(menu) => menu,
    };
    fallback::menu::show(menu, at, window, cx);
}

/// Open `popover` under `anchor` (the element's bounds in the window) as the platform's own, and
/// run what is done in it, a moment later and outside every borrow of the app (see `popover`).
/// The handle closes it from the app (a hover popover is closed by the app).
pub fn show_popover(popover: Popover, anchor: Bounds<Pixels>, window: &mut Window, cx: &mut App) -> Handle {
    #[cfg(any(test, feature = "devctl"))]
    if let Some(handle) = record::keep_popover(&popover, window) {
        return handle;
    }
    #[cfg(target_os = "macos")]
    let popover = match macos::popover::show(popover, anchor, window, cx) {
        Ok(handle) => return handle,
        Err(popover) => popover,
    };
    fallback::popover::show(popover, anchor, window, cx)
}

/// Open the system's Colours window showing `start`; each colour chosen in it reaches `on_change`,
/// the latest per tick, outside every borrow of the app. Nowhere to open it, nothing happens.
pub fn pick_colour(start: Hsla, on_change: ColourChange, window: &mut Window, cx: &mut App) {
    #[cfg(any(test, feature = "devctl"))]
    if record::keep_colour(&on_change) {
        return;
    }
    #[cfg(target_os = "macos")]
    macos::colour::pick(start, on_change, window, cx);
    #[cfg(not(target_os = "macos"))]
    let _ = (start, on_change, window, cx);
}
