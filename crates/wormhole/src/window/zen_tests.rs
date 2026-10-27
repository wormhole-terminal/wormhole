//! Zen mode in a real window: the chrome it hides, the panel it leaves be, and that the window comes back
//! as it was. The settings it layers are tested in the store (`settings::zen_tests`).

use std::path::PathBuf;

use gpui_kit::accesskit::Role;
use gpui_kit::{TestAppContext, VisualTestContext, px, size};
use wormhole_core::config::presets::{Preset, Shown};
use wormhole_core::view::layout::title_parts::{TitlePart, TitleRow};

use crate::app_state::OpenWindow;
use crate::settings::{self, SettingsStore, set_zen_mode};
use crate::window::test_harness::{draw, drive, isolate_in, press, read, simulate_tree, start, tree};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

fn drawn(cx: &mut TestAppContext, window: &OpenWindow, selector: &'static str) -> bool {
    VisualTestContext::from_window(window.handle, cx).debug_bounds(selector).is_some()
}

fn zen_mode(cx: &mut TestAppContext, on: bool) {
    cx.update(|cx| set_zen_mode(cx, on));
    cx.run_until_parked();
}

/// A wide window with the status bar and the minimap on, the panel open, a title row of the person's own
/// and a folder to show in it, drawn until it settles.
fn full_window(cx: &mut TestAppContext) -> OpenWindow {
    let window = start(cx, 1);
    cx.simulate_window_resize(window.handle, size(px(1800.0), px(900.0)));
    cx.update(|cx| {
        SettingsStore::update(cx, |s| {
            s.status_bar = true;
            s.minimap = true;
            s.title_row = Some(TitleRow::new([
                TitlePart::Folder,
                TitlePart::Terminal,
                TitlePart::Space,
                TitlePart::Ports,
                TitlePart::Splits,
            ]));
        })
    });
    drive(cx, &window, |ws, _, cx| {
        ws.panes.panel_wanted = true;
        ws.sessions[0].terminal().update(cx, |terminal, cx| {
            terminal.cwd = Some(PathBuf::from("/Volumes/Work/zen"));
            cx.notify();
        });
        ws.sync_summaries(cx);
    });
    for _ in 0..3 {
        draw(cx, &window);
    }
    window
}

/// Whether the status bar is in the frame: it has no selector of its own, but it is a toolbar in the
/// accessibility tree, which is built for the frame once an assistive app is connected.
fn status_bar_drawn(cx: &mut TestAppContext, window: &OpenWindow) -> bool {
    simulate_tree(cx, window, true);
    draw(cx, window);
    !tree(cx, window).labelled(Role::Toolbar, "Status bar").is_empty()
}

/// What the chrome shows now: the status bar, the panel's edge, the folder crumb.
fn chrome(cx: &mut TestAppContext, window: &OpenWindow) -> (bool, bool, bool) {
    (status_bar_drawn(cx, window), drawn(cx, window, "panel-resize"), drawn(cx, window, "crumb-folder"))
}

#[gpui_kit::test]
fn zen_mode_hides_the_chrome_and_ending_it_brings_back_all_of_it(cx: &mut TestAppContext) {
    if isolate("zen_mode_hides_the_chrome_and_ending_it_brings_back_all_of_it") {
        return;
    }
    let window = full_window(cx);
    assert_eq!(chrome(cx, &window), (true, true, true), "the window shows everything it was given");
    let own_row = cx.update(|cx| settings::settings(cx).title_row().into_owned());
    let panes = read(cx, &window, |ws| ws.panes);

    zen_mode(cx, true);
    // Drawn the way the display does (replaying what GPUI keeps; the chrome's check panics on a missed
    // notify there) and from scratch.
    draw(cx, &window);
    assert_eq!(
        chrome(cx, &window),
        (false, false, false),
        "no status bar, no panel, a title row of the name"
    );
    let zen = Shown::of(Preset::Zen);
    cx.update(|cx| {
        let settings = settings::settings(cx);
        assert_eq!(settings.title_row().into_owned(), zen.title.row());
        assert!(!settings.status_bar && !settings.minimap && !settings.column_dividers);
        assert_eq!(settings::ui(cx).divider_width, px(0.0), "no line between the columns");
    });
    assert!(!drawn(cx, &window, "minimap"));
    read(cx, &window, |ws| {
        assert!(!ws.notes_visible() && ws.columns().panel == 0.0, "the panel takes no room");
        assert_eq!(ws.panes, panes, "the window still wants it as it did");
    });

    zen_mode(cx, false);
    draw(cx, &window);
    assert_eq!(chrome(cx, &window), (true, true, true), "everything is back");
    cx.update(|cx| {
        assert_eq!(settings::settings(cx).title_row().into_owned(), own_row, "the person's own title row");
        assert!(settings::settings(cx).status_bar && settings::settings(cx).column_dividers);
        assert!(settings::ui(cx).divider_width > px(0.0));
    });
    read(cx, &window, |ws| assert!(ws.notes_visible() && ws.panes == panes));
}

#[gpui_kit::test]
fn a_panel_closed_by_the_person_stays_closed_when_zen_mode_ends(cx: &mut TestAppContext) {
    if isolate("a_panel_closed_by_the_person_stays_closed_when_zen_mode_ends") {
        return;
    }
    let window = full_window(cx);
    drive(cx, &window, |ws, window, cx| ws.toggle_notes(window, cx));
    draw(cx, &window);
    assert!(!drawn(cx, &window, "panel-resize"));

    zen_mode(cx, true);
    draw(cx, &window);
    zen_mode(cx, false);
    draw(cx, &window);
    assert!(!drawn(cx, &window, "panel-resize"), "Zen mode did not open what was closed");
    read(cx, &window, |ws| assert!(!ws.panes.panel_wanted));
}

#[gpui_kit::test]
fn the_panels_toggle_does_nothing_under_zen_mode_and_the_panel_is_as_it_was_after(cx: &mut TestAppContext) {
    if isolate("the_panels_toggle_does_nothing_under_zen_mode_and_the_panel_is_as_it_was_after") {
        return;
    }
    let window = full_window(cx);
    zen_mode(cx, true);
    draw(cx, &window);
    let panes = read(cx, &window, |ws| ws.panes);

    // The button and ⌥⌘I, and showing a tab (⌃⌘1), which would open the panel.
    drive(cx, &window, |ws, window, cx| ws.toggle_notes(window, cx));
    press(cx, &window, "alt-cmd-i");
    press(cx, &window, "ctrl-cmd-1");
    draw(cx, &window);
    read(cx, &window, |ws| {
        assert_eq!(ws.panes, panes, "nothing of the window's panel moved");
        assert!(!ws.notes_visible());
    });

    zen_mode(cx, false);
    draw(cx, &window);
    assert!(drawn(cx, &window, "panel-resize"), "the panel is back as it was");
}

#[gpui_kit::test]
fn the_key_toggles_zen_mode_and_it_is_not_saved(cx: &mut TestAppContext) {
    if isolate("the_key_toggles_zen_mode_and_it_is_not_saved") {
        return;
    }
    let window = full_window(cx);
    let path = SettingsStore::config_path();
    let saved = std::fs::read(&path).expect("config.json is saved");

    press(cx, &window, "ctrl-cmd-z");
    cx.run_until_parked();
    draw(cx, &window);
    assert!(cx.update(|cx| settings::zen(cx)), "⌃⌘Z turned it on");
    assert_eq!(chrome(cx, &window), (false, false, false));
    assert_eq!(std::fs::read(&path).expect("config.json"), saved, "turning it on wrote nothing");

    // A setting changed meanwhile saves the person's values, never Zen mode's.
    cx.update(|cx| SettingsStore::update(cx, |s| s.diff_wrap = !s.diff_wrap));
    let text = String::from_utf8(std::fs::read(&path).expect("config.json")).expect("text");
    let own = wormhole_core::config::Config::load_for(&crate::active_profile::paths()).settings;
    assert!(own.status_bar && own.minimap && own.column_dividers, "{text}");
    assert_eq!(own.title_row().parts().len(), 5, "the person's own title row is in the file");
    assert_ne!(std::fs::read(&path).expect("config.json"), saved, "the change itself was saved");

    press(cx, &window, "ctrl-cmd-z");
    cx.run_until_parked();
    draw(cx, &window);
    assert!(!cx.update(|cx| settings::zen(cx)), "⌃⌘Z turned it off");
    assert_eq!(chrome(cx, &window), (true, true, true));
}

#[gpui_kit::test]
fn the_status_bar_and_minimap_toggles_wait_for_the_end_of_zen_mode(cx: &mut TestAppContext) {
    if isolate("the_status_bar_and_minimap_toggles_wait_for_the_end_of_zen_mode") {
        return;
    }
    let window = full_window(cx);
    zen_mode(cx, true);
    press(cx, &window, "cmd-/");
    cx.run_until_parked();
    cx.update(|cx| assert!(settings::settings_without_zen(cx).status_bar, "⌘/ changed nothing"));
    zen_mode(cx, false);
    draw(cx, &window);
    press(cx, &window, "cmd-/");
    cx.run_until_parked();
    cx.update(|cx| assert!(!settings::settings(cx).status_bar, "and does again afterwards"));
}

#[gpui_kit::test]
fn a_profile_opened_while_zen_mode_is_on_is_in_it_before_any_tick(cx: &mut TestAppContext) {
    if isolate("a_profile_opened_while_zen_mode_is_on_is_in_it_before_any_tick") {
        return;
    }
    use wormhole_core::config::profile::{NewProfile, StartFrom};

    let window = full_window(cx);
    zen_mode(cx, true);
    let work = cx
        .update(|cx| {
            let new =
                NewProfile { name: "Work".into(), color: None, avatar: None, from: StartFrom::Defaults };
            let id = crate::app_state::AppState::create_profile(new, cx)?;
            crate::app_state::AppState::ensure_open(&id, cx);
            Ok::<_, String>(id)
        })
        .expect("makes the profile");
    // No tick has run: the store a window of the new profile reads already shows less.
    cx.update(|cx| {
        let store = settings::store_in(&work, cx);
        assert!(store.zen() && !store.settings.status_bar && !store.settings.minimap);
    });
    cx.update(|cx| crate::app_state::AppState::open_window_in_profile(&work, cx));
    cx.run_until_parked();
    draw(cx, &window);
}

#[gpui_kit::test]
fn the_panels_button_leaves_zen_mode_and_does_not_open_the_panel(cx: &mut TestAppContext) {
    if isolate("the_panels_button_leaves_zen_mode_and_does_not_open_the_panel") {
        return;
    }
    let window = full_window(cx);
    // This window's panel is closed; another window's would be open.
    drive(cx, &window, |ws, window, cx| ws.toggle_notes(window, cx));
    zen_mode(cx, true);
    draw(cx, &window);
    drive(cx, &window, |ws, window, cx| ws.panel_button(window, cx));
    draw(cx, &window);
    assert!(!cx.update(|cx| settings::zen(cx)), "the click left Zen mode");
    read(cx, &window, |ws| assert!(!ws.panes.panel_wanted && !ws.notes_visible(), "the panel as it was"));
    assert_eq!(chrome(cx, &window), (true, false, true));

    // Outside Zen mode the button is the toggle.
    drive(cx, &window, |ws, window, cx| ws.panel_button(window, cx));
    read(cx, &window, |ws| assert!(ws.panes.panel_wanted));
}
