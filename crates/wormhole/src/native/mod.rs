//! What the platform draws or does itself, behind one API for the app, so a port to another
//! platform adds a sibling folder to `macos/` and nothing else changes: `menu` is what a
//! menu is (platform-neutral), `macos/` its AppKit implementation (the only code here with
//! `objc2` and `unsafe`), `fallback/` GPUI's own for platforms with no native one, which is also
//! what a window without a native view gets.
//!
//! Context menus are the first to live here. The dialogs, the status item's menu (`menubar`), the
//! window's blur (`vibrancy`), the traffic lights, Touch ID (`auth`) and the rest of what is
//! AppKit's are meant to move here one by one, each as a neutral file beside `menu.rs` and an
//! implementation in `macos/`.

mod anchor;
pub mod fallback;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod menu;
#[cfg(any(test, feature = "devctl"))]
pub mod record;

use gpui_kit::*;

pub use anchor::{Anchor, on_right_click};
pub use menu::{Icon, Item, Menu, run_on};

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
