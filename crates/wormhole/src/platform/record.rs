//! devctl and tests: the last menu that was opened, kept as a model. A native menu can't be screenshotted or
//! clicked, so a scripted check reads it (`menu`), chooses an item (`menu-pick`) or dismisses it
//! (`menu-close`) here instead. In a background instance (and in tests) the menu is not shown at all:
//! a real one would block the scripted instance in AppKit's loop and take the keys.

use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::{App, Hsla, Window};

use super::menu::{Callback, Menu};
use super::popover::{ColourChange, Handle, Popover, Row};

thread_local! {
    static LAST: RefCell<Option<Menu>> = const { RefCell::new(None) };
    static POPOVER: RefCell<Option<Recorded>> = const { RefCell::new(None) };
    static COLOUR: RefCell<Option<ColourChange>> = const { RefCell::new(None) };
}

/// The popover that was opened last, and the window it is in.
struct Recorded {
    popover: Popover,
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
    if let Some(name) =
        selector.trim().strip_prefix("colour ").or_else(|| selector.trim().strip_prefix("color "))
    {
        return pick_swatch(&menu, name.trim());
    }
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

/// `colour <n|name>`: a dot of the menu's row of swatches, by its place (from 0) or its name, as a
/// click on it would: the menu closes and the row's `on_pick` runs with its place.
fn pick_swatch(menu: &Menu, which: &str) -> Result<(String, Vec<Callback>), String> {
    let row = menu.swatch_row().ok_or("the menu has no row of colours")?;
    let ix = match which.parse::<usize>() {
        Ok(n) => (n < row.swatches.len()).then_some(n),
        Err(_) => row.swatches.iter().position(|swatch| swatch.name.eq_ignore_ascii_case(which)),
    }
    .ok_or_else(|| format!("no colour {which:?}"))?;
    let (title, on_pick) = (format!("colour {}", row.swatches[ix].name), row.on_pick.clone());
    let mut callbacks = close().unwrap_or_default();
    let run: Callback = Rc::new(move |window, cx| on_pick(ix, window, cx));
    callbacks.push(run);
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

// ---------------------------------------------------------------------------- popovers

/// Remember `popover` as the open one, as `keep` does a menu. `Some(handle)` when it is not to be
/// shown (a background instance, a test): the handle closes the record.
pub fn keep_popover(popover: &Popover, window: &Window) -> Option<Handle> {
    if !(crate::background::on() || cfg!(test)) {
        return None;
    }
    let handle = window.window_handle();
    POPOVER.with(|last| *last.borrow_mut() = Some(Recorded { popover: popover.clone() }));
    Some(Handle::new(move |cx: &mut App| {
        let Some(callbacks) = popover_close(false) else { return };
        // Not from inside whatever update called this.
        cx.defer(move |cx| {
            handle
                .update(cx, move |_, window, cx| {
                    for run in callbacks {
                        run(window, cx);
                    }
                })
                .ok();
        });
    }))
}

/// A colour panel that was asked for, instead of one that is shown.
pub fn keep_colour(on_change: &ColourChange) -> bool {
    if !(crate::background::on() || cfg!(test)) {
        return false;
    }
    COLOUR.with(|last| *last.borrow_mut() = Some(on_change.clone()));
    true
}

/// The popover that was opened last and not yet closed.
pub fn open_popover() -> Option<Popover> {
    POPOVER.with(|last| last.borrow().as_ref().map(|recorded| recorded.popover.clone()))
}

fn with_popover<R>(f: impl FnOnce(&mut Popover) -> R) -> Option<R> {
    POPOVER.with(|last| last.borrow_mut().as_mut().map(|recorded| f(&mut recorded.popover)))
}

/// The open popover as `popover` prints it.
pub fn dump_popover() -> String {
    match open_popover() {
        Some(popover) => popover.outline().into_iter().map(|line| format!("{line}\n")).collect(),
        None => "no popover open\n".into(),
    }
}

/// Close the open popover as a click outside does: a field's text is kept (committed), then its
/// `on_close`. `keep` false skips the commit, as ⎋ does. `None` when none is open.
pub fn popover_close(keep: bool) -> Option<Vec<Callback>> {
    let recorded = POPOVER.with(|last| last.borrow_mut().take())?;
    let mut callbacks: Vec<Callback> = Vec::new();
    if keep {
        for row in &recorded.popover.rows {
            if let Row::Field(field) = row {
                let (text, commit) = (field.value.to_string(), field.on_commit.clone());
                callbacks.push(Rc::new(move |window, cx| commit(&text, window, cx)));
            }
        }
    }
    callbacks.extend(recorded.popover.on_close);
    Some(callbacks)
}

/// Press the button or action row titled `title`, as a click does: what it says after, kept; a
/// button that closes the popover closes it first. Returns what is to run.
pub fn popover_press(title: &str) -> Result<(String, Vec<Callback>), String> {
    let popover = open_popover().ok_or("no popover is open")?;
    let title = title.trim();
    for row in &popover.rows {
        match row {
            Row::Buttons(buttons) => {
                if let Some(button) = buttons.iter().find(|button| button.title == title) {
                    if !button.enabled {
                        return Err(format!("{title:?} is off"));
                    }
                    let on_press = button.on_press.clone();
                    let mut callbacks = Vec::new();
                    if button.closes {
                        // The popover goes first, then the action.
                        callbacks.extend(popover_close(false).unwrap_or_default());
                        callbacks.push(on_press);
                    } else {
                        callbacks.push(on_press);
                        if let Some(done) = button.done.clone() {
                            with_popover(|open| says(open, title, &done.title));
                        }
                    }
                    return Ok((title.to_string(), callbacks));
                }
            }
            Row::Actions(items) => {
                if let Some(item) = items.iter().find(|item| item.title == title) {
                    if !item.enabled {
                        return Err(format!("{title:?} is off"));
                    }
                    let mut callbacks = popover_close(false).unwrap_or_default();
                    callbacks.extend(item.on_select.clone());
                    return Ok((title.to_string(), callbacks));
                }
            }
            _ => {}
        }
    }
    Err(format!("no button or action {title:?}"))
}

fn says(popover: &mut Popover, title: &str, done: &gpui_kit::SharedString) {
    for row in &mut popover.rows {
        if let Row::Buttons(buttons) = row
            && let Some(button) = buttons.iter_mut().find(|button| button.title == title)
        {
            button.title = done.clone();
        }
    }
}

/// Type `text` into the open popover's field (it replaces what is there).
pub fn popover_type(text: &str) -> Result<(), String> {
    let found = with_popover(|popover| {
        popover.rows.iter_mut().find_map(|row| match row {
            Row::Field(field) => {
                field.value = text.to_string().into();
                Some(())
            }
            _ => None,
        })
    })
    .ok_or("no popover is open")?;
    found.ok_or_else(|| "the popover has no field".to_string())
}

/// ↩ or ⎋ in the open popover's field: ↩ commits and closes, ⎋ runs the field's cancel and closes
/// without keeping the text.
pub fn popover_key(key: &str) -> Result<Vec<Callback>, String> {
    let popover = open_popover().ok_or("no popover is open")?;
    let field = popover
        .rows
        .iter()
        .find_map(|row| match row {
            Row::Field(field) => Some(field.clone()),
            _ => None,
        })
        .ok_or("the popover has no field")?;
    match key.trim() {
        "enter" | "return" => {
            let mut callbacks = Vec::new();
            let (text, commit) = (field.value.to_string(), field.on_commit.clone());
            callbacks.push(Rc::new(move |window: &mut Window, cx: &mut App| commit(&text, window, cx)) as Callback);
            callbacks.extend(popover_close(false).unwrap_or_default());
            Ok(callbacks)
        }
        "escape" | "esc" => {
            let mut callbacks: Vec<Callback> = field.on_cancel.into_iter().collect();
            callbacks.extend(popover_close(false).unwrap_or_default());
            Ok(callbacks)
        }
        other => Err(format!("popover-key enter|escape, not {other:?}")),
    }
}

/// A colour for the open popover's row of dots: a named one by name (the popover stays), or
/// `#rrggbb`, which a colour chosen in the panel is (what the panel would send). With no popover
/// open, the colour panel that was asked for last.
pub fn popover_colour(which: &str) -> Result<Vec<Callback>, String> {
    let which = which.trim();
    let hex = which.starts_with('#').then(|| wormhole_core::tokens::Color::from_hex(which));
    let colour = |color: wormhole_core::tokens::Color| -> Hsla {
        gpui_kit::Rgba { r: color.r, g: color.g, b: color.b, a: 1.0 }.into()
    };
    if let Some(popover) = open_popover() {
        for row in &popover.rows {
            let Row::Colours(colours) = row else { continue };
            return match hex {
                Some(parsed) => {
                    let custom = colours.custom.as_ref().ok_or("the colours have no custom dot")?;
                    let (color, on_change) = (colour(parsed?), custom.on_change.clone());
                    Ok(vec![Rc::new(move |window, cx| on_change(color, window, cx))])
                }
                None => {
                    let ix = colours
                        .swatches
                        .swatches
                        .iter()
                        .position(|swatch| swatch.name.eq_ignore_ascii_case(which))
                        .ok_or_else(|| format!("no colour {which:?}"))?;
                    let on_pick = colours.swatches.on_pick.clone();
                    Ok(vec![Rc::new(move |window, cx| on_pick(ix, window, cx))])
                }
            };
        }
        return Err("the popover has no colours".into());
    }
    let on_change = COLOUR.with(|last| last.borrow().clone()).ok_or("no popover or colour panel is open")?;
    let color = colour(hex.ok_or("a colour panel takes #rrggbb")??);
    Ok(vec![Rc::new(move |window, cx| on_change(color, window, cx))])
}
