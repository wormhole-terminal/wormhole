//! The menu bar as text, to hold against the menus' spec: on macOS what AppKit has (titles, the key
//! equivalents GPUI gave the items from the bindings, check marks, and what the system adds);
//! elsewhere the model the menus were built from. And the About and Licenses windows the
//! Wormhole menu opens, the command palette and the keyboard-shortcuts sheet.

use anyhow::Result;

use super::Ctx;

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
    Ok(native(ctx).unwrap_or_else(|| crate::workspace::menu_dump(ctx.cx)))
}

#[cfg(target_os = "macos")]
fn native(_: &mut Ctx) -> Option<String> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSControlStateValueOn, NSEventModifierFlags, NSMenu};

    fn key_text(key: &str) -> String {
        // AppKit writes arrows and F-keys as characters of Unicode's private use area.
        let mut chars = key.chars();
        match (chars.next().map(u32::from), chars.next()) {
            (Some(0xF700), None) => "↑".into(),
            (Some(0xF701), None) => "↓".into(),
            (Some(0xF702), None) => "←".into(),
            (Some(0xF703), None) => "→".into(),
            (Some(code @ 0xF704..=0xF71B), None) => format!("F{}", code - 0xF704 + 1),
            (Some(0x08), None) | (Some(0x7F), None) => "⌫".into(),
            (Some(0x0D), None) => "↩".into(),
            (Some(0x20), None) => "Space".into(),
            _ => key.to_uppercase(),
        }
    }

    fn dump(menu: &NSMenu, depth: usize, out: &mut String) {
        let indent = "  ".repeat(depth);
        for item in menu.itemArray() {
            if item.isSeparatorItem() {
                out.push_str(&format!("{indent}---\n"));
                continue;
            }
            let key = item.keyEquivalent().to_string();
            let mut shortcut = String::new();
            if !key.is_empty() {
                let mask = item.keyEquivalentModifierMask();
                shortcut.push_str("  ");
                for (flag, glyph) in [
                    (NSEventModifierFlags::Control, '⌃'),
                    (NSEventModifierFlags::Option, '⌥'),
                    (NSEventModifierFlags::Shift, '⇧'),
                    (NSEventModifierFlags::Command, '⌘'),
                ] {
                    if mask.contains(flag) {
                        shortcut.push(glyph);
                    }
                }
                shortcut.push_str(&key_text(&key));
            }
            let check = if item.state() == NSControlStateValueOn { "✓ " } else { "" };
            let submenu = item.submenu();
            let arrow = if submenu.is_some() && depth > 0 { " ▸" } else { "" };
            let disabled = if item.isEnabled() { "" } else { "  (disabled)" };
            out.push_str(&format!("{indent}{check}{}{arrow}{shortcut}{disabled}\n", item.title()));
            if let Some(submenu) = submenu {
                dump(&submenu, depth + 1, out);
            }
        }
    }

    let mtm = MainThreadMarker::new()?;
    let main = NSApplication::sharedApplication(mtm).mainMenu()?;
    let mut out = String::new();
    dump(&main, 0, &mut out);
    Some(out)
}

#[cfg(not(target_os = "macos"))]
fn native(_: &mut Ctx) -> Option<String> {
    None
}

/// `palette [query]` / `palette close`: the command palette over this window, `query` typed.
pub(super) fn palette(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.devctl_palette(arg, ctx.window, ctx.cx);
    Ok(String::new())
}

/// `sheet [filter]` / `sheet close`: the keyboard-shortcuts sheet over this window, filtered.
pub(super) fn sheet(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.devctl_sheet(arg, ctx.window, ctx.cx);
    Ok(String::new())
}

/// `menu`: the last menu a right-click or a menu button opened, one row a line (items numbered).
/// A native menu can't be screenshotted, so the scripted checks read the model instead.
pub(super) fn menu(_: &mut Ctx, _: &str) -> Result<String> {
    Ok(crate::native::record::dump())
}

/// `menu-pick <n|title>`: choose an item of that menu as a click on it would, by its number in
/// `menu` or its title (`Submenu > Title` inside a submenu): the menu closes, then the item runs.
pub(super) fn menu_pick(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let (title, callbacks) = crate::native::record::pick(arg).map_err(anyhow::Error::msg)?;
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
    let Some(callbacks) = crate::native::record::close() else { anyhow::bail!("no menu is open") };
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
    let menu = crate::native::record::open().ok_or_else(|| anyhow::anyhow!("no menu is open"))?;
    let press = match arg.trim().strip_prefix("press") {
        Some(n) => Some(n.trim().parse::<usize>()?),
        None => None,
    };
    #[cfg(target_os = "macos")]
    {
        crate::native::macos::menu::inspect(&menu, press)
            .ok_or_else(|| anyhow::anyhow!("not on the main thread"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (menu, press);
        anyhow::bail!("only on macOS")
    }
}
