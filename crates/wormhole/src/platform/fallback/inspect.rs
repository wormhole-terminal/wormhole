//! `platform::inspect` without native menus or popovers: there is nothing native to look at.

use std::path::Path;

use anyhow::Result;
use gpui_kit::{App, Bounds, Pixels, Window};

use crate::platform::{Handle, Menu, Popover};

pub fn menu_bar() -> Option<String> {
    None
}

pub fn menu(_: &Menu, _: Option<usize>) -> Result<String> {
    anyhow::bail!("only on macOS")
}

pub fn popover(_: &Popover, _: &Path) -> Result<String> {
    anyhow::bail!("only on macOS")
}

pub fn show_popover(_: Popover, _: Bounds<Pixels>, _: &mut Window, _: &mut App) -> Result<Handle> {
    anyhow::bail!("only on macOS")
}
