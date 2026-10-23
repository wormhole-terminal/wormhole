//! devctl's hold on the settings window: show a section, search, open a select, confirm a
//! reset, and render the window to a PNG. devctl calls these from a window's tick; they ask
//! nothing of that window.

#![cfg(feature = "devctl")]

use std::path::PathBuf;

use gpui_kit::*;
use wormhole_core::config::settings_index::{Query, SectionId as Section};
use wormhole_core::theme::fonts::FontSlot;
use wormhole_core::view::sidebar::Density;

use super::{SettingsWindow, SidebarLook, current_window, open_settings, reveal_row, sections};
use crate::app_state::app;

pub(crate) fn section(name: &str, cx: &mut App) {
    let section = Section::ALL.into_iter().find(|s| format!("{s:?}").eq_ignore_ascii_case(name));
    open_settings(section, cx);
}

/// devctl `settings-search <text>`: type into the settings search (empty clears it). The
/// window opens first if it isn't (deferred: send this in the next batch then).
pub(crate) fn search(text: &str, cx: &mut App) {
    let text = text.to_string();
    with_page(cx, move |page, window, cx| {
        page.search.update(cx, |search, cx| search.set_value(text, window, cx));
        page.show_results = !Query::new(&page.search.read(cx).value()).is_empty();
    });
}

/// devctl `settings-record <id> [keys]`: start the recorder of row `id`, and press `keys`
/// (GPUI's keystroke syntax, e.g. `cmd-space`) into it.
pub(crate) fn record(arg: &str, cx: &mut App) {
    let (id, keys) = arg.trim().split_once(' ').unwrap_or((arg.trim(), ""));
    let Some(id) = super::sections::recorder_id(id) else { return };
    let keystroke = (!keys.trim().is_empty()).then(|| Keystroke::parse(keys.trim()).ok()).flatten();
    with_page(cx, move |page, _, cx| {
        page.set_recording(Some(super::recorder::Recording::Keys { id, held: Default::default() }), cx);
        if let Some(keystroke) = keystroke {
            page.record_key(&keystroke, cx);
        }
    });
}

/// devctl `settings-reveal <id>`: the settings window at that row, as the palette opens it
/// (`reveal_row`). The window opens on the next tick: `settings-shot` in a later batch.
pub(crate) fn reveal(id: &str, cx: &mut App) {
    reveal_row(id, cx);
}

/// devctl `settings-preview <regular|compact|dense|off>`: what Terminals & sidebar previews
/// beside the current look, as when the pointer is over that density.
pub(crate) fn preview(arg: &str, cx: &mut App) {
    let look = match arg.trim() {
        "regular" => Some(SidebarLook::Density(Density::Regular)),
        "compact" => Some(SidebarLook::Density(Density::Compact)),
        "dense" => Some(SidebarLook::Density(Density::Dense)),
        _ => None,
    };
    with_page(cx, move |page, _, cx| {
        page.show_section(Section::Sessions, cx);
        page.sidebar_preview = look;
    });
}

/// devctl `profile-sheet`: the "New profile" sheet over the settings window, as the File menu
/// opens it (the window opens on the next tick: `settings-shot` in a later batch).
pub(crate) fn profile_sheet(cx: &mut App) {
    sections::open_new_profile_sheet(cx);
}

/// devctl `settings-scroll <px>`: scroll the page down (negative: up).
pub(crate) fn scroll(by: f32, cx: &mut App) {
    with_page(cx, move |page, _, _| {
        let offset = page.content_scroll.offset();
        page.content_scroll.set_offset(point(offset.x, offset.y - px(by)));
    });
}

/// devctl `settings-resize <w> <h>`: the settings window's size.
pub(crate) fn resize(w: f32, h: f32, cx: &mut App) {
    with_page(cx, move |_, window, _| window.resize(size(px(w), px(h))));
}

/// devctl `settings-reset-confirm`: press Advanced → "Reset to defaults…" (asks, resets nothing).
pub(crate) fn reset_confirm(cx: &mut App) {
    with_page(cx, |page, _, cx| {
        page.show_section(Section::Advanced, cx);
        page.confirm_reset = true;
    });
}

/// devctl `settings-cli-tool [install|uninstall]`: the Command line tool row on its page, read
/// again, or with its Install / Uninstall pressed.
pub(crate) fn cli_tool(action: &str, cx: &mut App) {
    let action = action.trim().to_string();
    with_page(cx, move |page, _, cx| {
        page.show_section(Section::Agents, cx);
        match action.as_str() {
            "install" => page.install_cli_tool(cx),
            "uninstall" => page.uninstall_cli_tool(cx),
            _ => page.load_cli_tool(cx),
        }
    });
}

/// devctl `settings-select <id>`: open the select of the row `id` (scrollback, icon-theme).
pub(crate) fn select(id: &str, cx: &mut App) {
    let id = super::sections::select_id(id);
    with_page(cx, move |page, _, _| page.open_select = id);
}

/// devctl: pick a project in Settings → Project folders.
pub(crate) fn project(root: &str, cx: &mut App) {
    let root = PathBuf::from(root);
    with_page(cx, move |page, _, cx| {
        page.show_section(Section::Projects, cx);
        page.projects.select(root);
    });
}

/// devctl: open the project select's menu in Settings → Project folders.
pub(crate) fn project_menu(query: &str, cx: &mut App) {
    let query = query.to_string();
    with_page(cx, move |page, window, cx| {
        page.show_section(Section::Projects, cx);
        page.projects.devctl_open(query, window, cx);
    });
}

/// devctl: render the settings window to a PNG and append the result to
/// `out`. Deferred: sections read app state that devctl is updating.
pub(crate) fn shot(path: PathBuf, out: PathBuf, cx: &mut App) {
    let window = crate::app_state::live(&app(cx).settings_window);
    cx.defer(move |cx| {
        let result = window
            .ok_or_else(|| anyhow::anyhow!("settings window not open"))
            .and_then(|(handle, _)| {
                // Draw first: a window behind others may not have redrawn since its last change.
                handle.update(cx, |_, window, cx| {
                    window.draw(cx).clear(cx);
                    window.render_to_image()
                })?
            })
            .and_then(|image| Ok(image.save(&path)?));
        let line = match result {
            Ok(()) => format!("saved {}\n", path.display()),
            Err(err) => format!("settings shot failed: {err:#}\n"),
        };
        let appended = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&out)
            .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
        if let Err(err) = appended {
            tracing::warn!("devctl: {err}");
        }
    });
}

/// devctl: expand a font picker in the settings window, optionally searching
/// (the window opens deferred, so the first call on a closed window only opens it).
pub(crate) fn font_picker(arg: &str, cx: &mut App) {
    let (slot, query) = arg.trim().split_once(' ').unwrap_or((arg.trim(), ""));
    let Some(slot) = FontSlot::from_key(slot) else { return };
    let query = query.to_string();
    open_settings(Some(Section::Fonts), cx);
    let Some((handle, page)) = current_window(cx) else { return };
    let _ = handle.update(cx, |_, window, cx| {
        page.update(cx, |page, cx| {
            if page.font_picker.open != Some(slot) {
                page.toggle_font_picker(slot, window, cx);
            }
            page.font_picker.search.update(cx, |input, cx| input.set_value(query, window, cx));
        })
    });
}

/// Run `f` on the settings window's page after this update (devctl runs inside a workspace's).
fn with_page(
    cx: &mut App,
    f: impl FnOnce(&mut SettingsWindow, &mut Window, &mut Context<SettingsWindow>) + 'static,
) {
    let Some((handle, page)) = current_window(cx) else {
        open_settings(None, cx);
        return;
    };
    cx.defer(move |cx| {
        let updated = handle.update(cx, |_, window, cx| {
            page.update(cx, |page, cx| {
                f(page, window, cx);
                cx.notify();
            })
        });
        if let Err(err) = updated {
            tracing::warn!("devctl settings: {err:#}");
        }
    });
}
