//! AppKit's side of `platform::inspect`: the main menu read back, and devctl's native menus and
//! popovers.

use std::path::Path;

use anyhow::Result;
use gpui_kit::{App, Bounds, Pixels, Window};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSControlStateValueOn, NSEventModifierFlags, NSMenu};

use crate::platform::{Handle, Menu, Popover};

pub fn menu_bar() -> Option<String> {
    let mtm = MainThreadMarker::new()?;
    let main = NSApplication::sharedApplication(mtm).mainMenu()?;
    let mut out = String::new();
    dump(&main, 0, &mut out);
    Some(out)
}

pub fn menu(menu: &Menu, press: Option<usize>) -> Result<String> {
    super::menu::inspect(menu, press).ok_or_else(|| anyhow::anyhow!("not on the main thread"))
}

pub fn popover(popover: &Popover, png: &Path) -> Result<String> {
    super::popover::render(popover, png).ok_or_else(|| anyhow::anyhow!("not on the main thread"))
}

pub fn show_popover(
    popover: Popover,
    anchor: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
) -> Result<Handle> {
    super::popover::show(popover, anchor, window, cx)
        .map_err(|_| anyhow::anyhow!("the window has no native view"))
}

/// A key equivalent as the menus' spec writes it.
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

/// `menu` one item a line, submenus indented under theirs.
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
