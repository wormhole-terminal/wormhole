//! The menu bar as text, to hold against the menus' spec: what the system has where it has a native
//! menu bar (`platform::inspect`: titles, the key equivalents GPUI gave the items from the
//! bindings, check marks, and what the system adds); elsewhere the model the menus were built from.
//! And the About and Licenses windows the Wormhole menu opens, the command palette and the
//! keyboard-shortcuts sheet.

use anyhow::Result;
use gpui_kit::{Bounds, point, px, size};

use super::Ctx;
use crate::platform::inspect;

/// `about`, `licenses`, `about-shot [name]`, `licenses-shot [name]`, `licenses-search <query>`,
/// `licenses-select <name>`.
pub(super) fn about(ctx: &mut Ctx, cmd: &str, arg: &str) -> Result<String> {
    // A shot's name; empty leaves the window's own.
    let arg = if cmd.ends_with("-shot") { super::file_name(arg, "")? } else { arg };
    Ok(crate::workspace::devctl_about(cmd, arg, ctx.dir, ctx.cx)?.unwrap_or_default())
}

/// `action <name>`: dispatch a registered action by name (`workspace::ToggleSidebar`) through this
/// window, as a click on its menu item does.
pub(super) fn action(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let action = ctx.cx.build_action(arg.trim(), None)?;
    ctx.window.dispatch_action(action, ctx.cx);
    Ok(String::new())
}

/// `available <name>`: whether something on the focused element's path handles the action (the
/// menu also counts the app-wide handlers, such as Hide's).
pub(super) fn available(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let action = ctx.cx.build_action(arg.trim(), None)?;
    Ok(format!("{}: {}\n", arg.trim(), ctx.window.is_action_available(action.as_ref(), ctx.cx)))
}

pub(super) fn menus(ctx: &mut Ctx, _: &str) -> Result<String> {
    Ok(inspect::menu_bar().unwrap_or_else(|| crate::workspace::menu_dump(ctx.cx)))
}

/// `palette [query]` / `palette close`: the command palette over this window, `query` typed.
pub(super) fn palette(ctx: &mut Ctx, arg: &str) -> Result<String> {
    crate::workspace::devctl_palette(ctx.workspace, arg, ctx.window, ctx.cx);
    Ok(String::new())
}

/// `sheet [filter]` / `sheet close`: the keyboard-shortcuts sheet over this window, filtered.
pub(super) fn sheet(ctx: &mut Ctx, arg: &str) -> Result<String> {
    crate::workspace::devctl_sheet(ctx.workspace, arg, ctx.window, ctx.cx);
    Ok(String::new())
}

/// `menu`: the last menu a right-click or a menu button opened, one row a line (items numbered).
/// A native menu can't be screenshotted, so the scripted checks read the model instead.
pub(super) fn menu(_: &mut Ctx, _: &str) -> Result<String> {
    Ok(crate::platform::record::dump())
}

/// `menu-pick <n|title>`: choose an item of that menu as a click on it would, by its number in
/// `menu` or its title (`Submenu > Title` inside a submenu): the menu closes, then the item runs.
pub(super) fn menu_pick(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let (title, callbacks) = crate::platform::record::pick(arg).map_err(anyhow::Error::msg)?;
    // Deferred like a click: the item updates the workspace, which a command may hold.
    ctx.window.defer(ctx.cx, move |window, cx| {
        for run in callbacks {
            run(window, cx);
        }
    });
    Ok(format!("picked {title}\n"))
}

/// `menu-close`: dismiss that menu as Esc does.
pub(super) fn menu_close(ctx: &mut Ctx, _: &str) -> Result<String> {
    let Some(callbacks) = crate::platform::record::close() else { anyhow::bail!("no menu is open") };
    ctx.window.defer(ctx.cx, move |window, cx| {
        for run in callbacks {
            run(window, cx);
        }
    });
    Ok(String::new())
}

/// `menu-native [press <n>]`: the open menu built as the real AppKit `NSMenu` (not shown), as AppKit
/// has it: items and, for a row of colour dots, each dot's tooltip and VoiceOver label; `press <n>`
/// presses the n-th dot as VoiceOver does and tells which one the menu took as chosen. The check of
/// what `menu` can't see: the AppKit side of a menu, short of a person's click in its loop.
pub(super) fn menu_native(_: &mut Ctx, arg: &str) -> Result<String> {
    let menu = crate::platform::record::open().ok_or_else(|| anyhow::anyhow!("no menu is open"))?;
    let press = match arg.trim().strip_prefix("press") {
        Some(n) => Some(n.trim().parse::<usize>()?),
        None => None,
    };
    inspect::menu(&menu, press)
}

// ---------------------------------------------------------------------------- popovers

/// Run what a popover command says is to run, deferred like a click: it updates the workspace,
/// which a command may hold.
fn run_deferred(ctx: &mut Ctx, callbacks: Vec<crate::platform::menu::Callback>) {
    ctx.window.defer(ctx.cx, move |window, cx| {
        for run in callbacks {
            run(window, cx);
        }
    });
}

/// `popover`: the last popover a hover, a click or a button opened, one row a line.
pub(super) fn popover(_: &mut Ctx, _: &str) -> Result<String> {
    Ok(crate::platform::record::dump_popover())
}

/// `popover-press <title>`: press that button or action row as a click does.
pub(super) fn popover_press(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let (title, callbacks) = crate::platform::record::popover_press(arg).map_err(anyhow::Error::msg)?;
    run_deferred(ctx, callbacks);
    Ok(format!("pressed {title}\n"))
}

/// `popover-type <text>`: type into the popover's field (it replaces what is there).
pub(super) fn popover_type(_: &mut Ctx, arg: &str) -> Result<String> {
    crate::platform::record::popover_type(arg).map_err(anyhow::Error::msg)?;
    Ok(String::new())
}

/// `popover-key <enter|escape>`: ↩ keeps what is in the field and closes, ⎋ drops it and closes.
pub(super) fn popover_key(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let callbacks = crate::platform::record::popover_key(arg).map_err(anyhow::Error::msg)?;
    run_deferred(ctx, callbacks);
    Ok(String::new())
}

/// `popover-colour <name|#rrggbb>`: a dot of the popover by name, or a colour chosen in the
/// Colours window (`#rrggbb`: the popover's last dot, or the window `Other colour…` asked for).
pub(super) fn popover_colour(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let callbacks = crate::platform::record::popover_colour(arg).map_err(anyhow::Error::msg)?;
    run_deferred(ctx, callbacks);
    Ok(String::new())
}

/// `popover-close`: close it as a click outside does: the field's text is kept.
pub(super) fn popover_close(ctx: &mut Ctx, _: &str) -> Result<String> {
    let Some(callbacks) = crate::platform::record::popover_close(true) else {
        anyhow::bail!("no popover is open")
    };
    run_deferred(ctx, callbacks);
    Ok(String::new())
}

/// `popover-native [name]`: the open popover built as the real AppKit controls (not shown) and
/// drawn to `<dir>/<name>.png`, as AppKit lays them out; prints its size and what it holds.
pub(super) fn popover_native(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let popover =
        crate::platform::record::open_popover().ok_or_else(|| anyhow::anyhow!("no popover is open"))?;
    let name = super::file_name(arg, "popover-native")?;
    let path = ctx.dir.join(format!("{name}.png"));
    let summary = inspect::popover(&popover, &path)?;
    Ok(format!("{summary}\nsaved {}\n", path.display()))
}

thread_local! {
    static REAL: std::cell::RefCell<Option<crate::platform::Handle>> = const { std::cell::RefCell::new(None) };
}

/// `popover-real`: open the recorded popover for real, as an `NSPopover` under the window's top left
/// corner, for a moment: the one check of AppKit's side (opening, the events a control sends, and
/// closing) that a background instance can make. `popover-real-close` closes it again. Use it with
/// a hover popover (`path-card open`), which doesn't take the keyboard.
pub(super) fn popover_real(ctx: &mut Ctx, _: &str) -> Result<String> {
    let popover =
        crate::platform::record::open_popover().ok_or_else(|| anyhow::anyhow!("no popover is open"))?;
    let anchor = Bounds::new(point(px(520.0), px(8.0)), size(px(120.0), px(24.0)));
    let handle = inspect::show_popover(popover, anchor, ctx.window, ctx.cx)?;
    REAL.with_borrow_mut(|real| *real = Some(handle));
    Ok(String::new())
}

/// `popover-real-close`: close what `popover-real` opened, as the app does.
pub(super) fn popover_real_close(ctx: &mut Ctx, _: &str) -> Result<String> {
    let handle = REAL.with_borrow_mut(Option::take).ok_or_else(|| anyhow::anyhow!("nothing was opened"))?;
    handle.close(ctx.cx);
    Ok(String::new())
}
