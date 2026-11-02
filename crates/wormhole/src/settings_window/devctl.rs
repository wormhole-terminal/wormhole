//! devctl's hold on the settings window: show a section, search, open a select, confirm a
//! reset, and render the window to a PNG. devctl calls these from a window's tick; they ask
//! nothing of that window.

#![cfg(feature = "devctl")]

use std::path::PathBuf;

use gpui_kit::*;
use wormhole_core::config::settings_index::{Query, SectionId as Section};
use wormhole_core::import::{self, Env, Source};
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

/// devctl `probe-approve <id>`: Settings → Extensions, the sheet of that Probe, approved.
pub(crate) fn probe_approve(id: String, cx: &mut App) {
    with_page(cx, move |page, _, cx| {
        page.show_section(Section::Extensions, cx);
        page.devctl_approve_probe(id, cx);
    });
}

/// devctl `probe-off <id>`.
pub(crate) fn probe_off(id: String, cx: &mut App) {
    with_page(cx, move |page, _, cx| {
        page.show_section(Section::Extensions, cx);
        page.turn_probe_off(id.clone(), id, cx);
    });
}

/// The `state` line for an open approval sheet (`probe-sheet <id> ready|waiting`), or nothing.
pub(crate) fn probe_sheet_line(cx: &App) -> String {
    let Some((_, page)) = current_window(cx) else { return String::new() };
    page.read(cx).probe_sheet_line()
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

/// devctl `settings-reset-confirm`: press Advanced → "Reset to defaults…" (the system's alert asks; answer
/// it with `sheet-answer`, and nothing is reset until the red button is).
pub(crate) fn reset_confirm(cx: &mut App) {
    with_page(cx, |page, window, cx| {
        page.show_section(Section::Advanced, cx);
        page.ask_reset_all(window, cx);
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

/// devctl `settings-select <id>`: open the menu of the row `id` (tab-width, theme-mode, scrollback,
/// density…), as a click on it does.
pub(crate) fn select(id: &str, cx: &mut App) {
    // The colour controls are menus of their own kind, with state in the window.
    if let Some(row) = ["primary-color", "branch-color"].into_iter().find(|known| *known == id) {
        with_page(cx, move |page, window, cx| {
            let state = if row == "primary-color" { &page.primary_pick } else { &page.branch_pick };
            crate::select::devctl_open_color(row, state, window, cx);
        });
        return;
    }
    let Some(row) = super::registry::menu_ids().find(|known| *known == id) else { return };
    with_page(cx, move |_, window, cx| crate::select::devctl_open(row, window, cx));
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

/// devctl `import-scan <fixture home>`: scan that folder as if it were the home (never the real one),
/// show the result as the Import page's, and open the page. Without a folder the page scans the
/// instance's own home, as when it is opened.
pub(crate) fn import_scan(home: Option<PathBuf>, cx: &mut App) {
    let Some(home) = home else {
        with_page(cx, |page, _, cx| {
            page.show_section(Section::Import, cx);
            page.scan_import(cx);
        });
        return;
    };
    // A fixed shell rule, so a fixture's shell lines read the same on every machine.
    let mut env = Env::new(home);
    env.login_shell = "/bin/zsh".into();
    env.system_shells = ["/bin/zsh", "/bin/bash", "/bin/sh"].map(String::from).to_vec();
    let mut found = import::scan(&env);
    crate::settings::import::resolve_fonts(&mut found);
    with_page(cx, move |page, _, cx| page.set_import(found, cx));
}

/// devctl `agents-open`: Coding agents' list of other agents and "How status works", open or shut.
pub(crate) fn agents_open(others: bool, status: bool, cx: &mut App) {
    with_page(cx, move |page, _, cx| {
        page.others_open = others;
        page.status_open = status;
        cx.notify();
    });
}

/// devctl `import-open <source key>`: open that terminal's card on the Import page.
pub(crate) fn import_open(source: Source, cx: &mut App) {
    with_page(cx, move |page, _, cx| page.import_open(source, cx));
}

/// devctl `import-apply <source key>`: press "Import selected" on that terminal's card, with the rows
/// ticked as shown. Returns whether the page has the terminal.
pub(crate) fn import_apply(source: Source, cx: &mut App) {
    with_page(cx, move |page, _, cx| {
        if page.import_has(source) {
            page.apply_import(source, cx);
        } else {
            tracing::warn!("devctl import-apply: {} was not found by the last scan", source.key());
        }
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

/// devctl `settings-pointer <move|click|right-click> <x> <y>`, `settings-pointer wheel <x> <y> <dy>` and
/// `settings-pointer alt-on|alt-off`: the pointer, the wheel over a point (a negative `dy` scrolls
/// down, as a trackpad), or ⌥ held, in the settings window, which has no command file of its own.
/// Positions are in the window's logical pixels, as in `settings-shot`.
pub(crate) fn pointer(arg: &str, cx: &mut App) -> Result<(), String> {
    let mut words = arg.split_whitespace();
    let what = words.next().unwrap_or("").to_string();
    let at = words
        .next()
        .zip(words.next())
        .and_then(|(x, y)| Some(point(px(x.parse().ok()?), px(y.parse().ok()?))));
    let dy: Option<f32> = words.next().and_then(|dy| dy.parse().ok());
    let Some((handle, _)) = current_window(cx) else { return Err("settings window not open".into()) };
    let (held, none) = (Modifiers { alt: true, ..Modifiers::none() }, Modifiers::none());
    let events: Vec<PlatformInput> = match (what.as_str(), at) {
        ("alt-on" | "alt-off", _) => vec![PlatformInput::ModifiersChanged(ModifiersChangedEvent {
            modifiers: if what == "alt-on" { held } else { none },
            capslock: Capslock::default(),
        })],
        ("wheel", Some(position)) => vec![PlatformInput::ScrollWheel(ScrollWheelEvent {
            position,
            delta: ScrollDelta::Pixels(point(px(0.0), px(dy.ok_or("wheel needs <x> <y> <dy>")?))),
            modifiers: none,
            touch_phase: TouchPhase::Moved,
        })],
        ("move", Some(position)) => {
            vec![PlatformInput::MouseMove(MouseMoveEvent { position, pressed_button: None, modifiers: none })]
        }
        ("click" | "right-click", Some(position)) => {
            let button = if what == "click" { MouseButton::Left } else { MouseButton::Right };
            vec![
                PlatformInput::MouseMove(MouseMoveEvent { position, pressed_button: None, modifiers: none }),
                PlatformInput::MouseDown(MouseDownEvent {
                    button,
                    position,
                    modifiers: none,
                    click_count: 1,
                    first_mouse: false,
                }),
                PlatformInput::MouseUp(MouseUpEvent { button, position, modifiers: none, click_count: 1 }),
            ]
        }
        _ => {
            return Err(
                "expected move|click|right-click <x> <y>, wheel <x> <y> <dy> or alt-on|alt-off".into()
            );
        }
    };
    cx.defer(move |cx| {
        let _ = handle.update(cx, |_, window, cx| {
            for event in events {
                window.dispatch_event(event, cx);
            }
        });
    });
    Ok(())
}
