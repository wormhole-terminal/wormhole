//! devctl's look at what the platform draws natively, which a screenshot of GPUI's render can't
//! show: the menu bar as the system has it, a menu or popover built as the real controls, and a
//! popover opened for real for a moment.

use std::path::Path;

use anyhow::Result;
use gpui_kit::{App, Bounds, Pixels, Window};

use super::os::inspect as os;
use super::{Handle, Menu, Popover};

/// The app's menu bar as the system has it (titles, key equivalents, check marks, what the system
/// adds); `None` where there is no native menu bar to read.
pub fn menu_bar() -> Option<String> {
    os::menu_bar()
}

/// `menu` built as the native menu (not shown), as the system has it; with `press`, the colour dot
/// of that place pressed as VoiceOver does.
pub fn menu(menu: &Menu, press: Option<usize>) -> Result<String> {
    os::menu(menu, press)
}

/// `popover` built as the native controls (not shown) and drawn to `png`; what it holds.
pub fn popover(popover: &Popover, png: &Path) -> Result<String> {
    os::popover(popover, png)
}

/// Open `popover` for real under `anchor`, as the native one.
pub fn show_popover(
    popover: Popover,
    anchor: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
) -> Result<Handle> {
    os::show_popover(popover, anchor, window, cx)
}
