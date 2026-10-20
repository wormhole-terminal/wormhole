//! devctl and tests: the last menu that was opened, kept as a model. A native menu can't be screenshotted or
//! clicked, so a scripted check reads it (`menu`), chooses an item (`menu-pick`) or dismisses it
//! (`menu-close`) here instead. In a background instance (and in tests) the menu is not shown at all:
//! a real one would block the scripted instance in AppKit's loop and take the keys.

use std::cell::RefCell;

use gpui_kit::Window;

use super::menu::{Callback, Menu};

thread_local! {
    static LAST: RefCell<Option<Menu>> = const { RefCell::new(None) };
}

/// Remember `menu` as the open one. True when it is not to be shown.
pub fn keep(menu: &Menu, _window: &Window) -> bool {
    LAST.with(|last| *last.borrow_mut() = Some(menu.clone()));
    crate::background::on() || cfg!(test)
}

/// The menu that was opened last and not yet chosen from or closed.
pub fn open() -> Option<Menu> {
    LAST.with(|last| last.borrow().clone())
}

/// Close the open menu as Esc does; what is to run for it, in order (its `on_close`). `None` when
/// none is open.
pub fn close() -> Option<Vec<Callback>> {
    let menu = LAST.with(|last| last.borrow_mut().take())?;
    Some(menu.on_close.into_iter().collect())
}

/// Choose an item of the open menu, by its number as `menu` prints it (from 0) or its title
/// (`Title`, or `Submenu > Title`), as a click on it would: the menu closes. Returns the item's
/// title and what is to run, in order: the menu's `on_close`, then the item's callback. The caller
/// runs them where nothing is borrowed (a deferred callback of the window).
pub fn pick(selector: &str) -> Result<(String, Vec<Callback>), String> {
    let menu = open().ok_or("no menu is open")?;
    let leaves = menu.leaves();
    let selector = selector.trim();
    let found = match selector.parse::<usize>() {
        Ok(n) => leaves.get(n),
        Err(_) => leaves.iter().find(|leaf| leaf.full_title() == selector || leaf.item.title == selector),
    }
    .ok_or_else(|| format!("no item {selector:?}"))?;
    if !found.item.enabled {
        return Err(format!("{:?} is disabled", found.full_title()));
    }
    let title = found.full_title();
    let run = found.item.on_select.clone();
    let mut callbacks = close().unwrap_or_default();
    callbacks.extend(run);
    Ok((title, callbacks))
}

/// The open menu as `menu` prints it: one line per row, the items numbered as `menu-pick` counts.
pub fn dump() -> String {
    let Some(menu) = open() else { return "no menu open\n".into() };
    let mut number = 0;
    let mut out = String::new();
    for line in menu.outline() {
        if line.trim_start().starts_with("- ") {
            out.push_str(&format!("{number:>2} {line}\n"));
            number += 1;
        } else {
            out.push_str(&format!("   {line}\n"));
        }
    }
    out
}
