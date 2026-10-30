//! The scripted select opens the same platform menu as the visible control.

use gpui_kit::accesskit::Role;
use gpui_kit::{Modifiers, MouseButton, TestAppContext, VisualTestContext};
use wormhole_core::theme::fonts::FontSlot;

use super::{Section, current_window, devctl, open_settings};
use crate::platform::record;
use crate::settings::settings;
use crate::window::test_harness::{isolate_in, start};

#[gpui_kit::test]
fn settings_switches_have_their_visible_names_in_the_accessibility_tree(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "settings_switches_have_their_visible_names_in_the_accessibility_tree") {
        return;
    }
    start(cx, 1);
    cx.update(|cx| open_settings(Some(Section::Terminal), cx));
    cx.run_until_parked();
    let handle = cx.update(|cx| current_window(cx).expect("settings is open").0);
    let nodes = cx.update(|cx| {
        handle
            .update(cx, |_, window, cx| {
                window.simulate_a11y_active(true);
                window.draw(cx).clear(cx);
                window.last_a11y_tree_update().expect("the tree was built").nodes
            })
            .expect("settings stays open")
    });
    let name = wormhole_text::text(wormhole_text::keys::SETTINGS_TERMINAL_RESTORE_TEXT);
    assert!(
        nodes.iter().any(|(_, node)| node.role() == Role::Switch && node.label() == Some(name)),
        "the switch for {name:?} should have its visible label"
    );
    assert!(
        nodes
            .iter()
            .filter(|(_, node)| node.role() == Role::Switch)
            .all(|(_, node)| node.label().is_some_and(|label| !label.is_empty())),
        "each visible terminal setting switch should have a name"
    );
}

#[gpui_kit::test]
fn settings_segments_are_exposed_as_named_radio_choices(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "settings_segments_are_exposed_as_named_radio_choices") {
        return;
    }
    start(cx, 1);
    cx.update(|cx| open_settings(Some(Section::Sessions), cx));
    cx.run_until_parked();
    let handle = cx.update(|cx| current_window(cx).expect("settings is open").0);
    let nodes = cx.update(|cx| {
        handle
            .update(cx, |_, window, cx| {
                window.simulate_a11y_active(true);
                window.draw(cx).clear(cx);
                window.last_a11y_tree_update().expect("the tree was built").nodes
            })
            .expect("settings stays open")
    });
    for label in [wormhole_text::keys::SETTINGS_SESSIONS_SIDEBAR, wormhole_text::keys::SETTINGS_SESSIONS_TABS]
    {
        let name = wormhole_text::text(label);
        assert!(
            nodes.iter().any(|(_, node)| node.role() == Role::RadioButton && node.label() == Some(name)),
            "the {name:?} segment should be an accessible radio choice"
        );
    }
    let sidebar = wormhole_text::text(wormhole_text::keys::SETTINGS_SESSIONS_SIDEBAR);
    let node = nodes
        .iter()
        .find(|(_, node)| node.role() == Role::RadioButton && node.label() == Some(sidebar))
        .unwrap();
    assert_eq!(node.1.is_selected(), Some(true));
}

#[gpui_kit::test]
fn scripted_select_uses_the_platform_menu_and_updates_settings(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "scripted_select_uses_the_platform_menu_and_updates_settings") {
        return;
    }
    start(cx, 1);
    cx.update(|cx| open_settings(Some(Section::Terminal), cx));
    cx.run_until_parked();
    cx.update(|cx| devctl::select("scrollback", cx));
    cx.run_until_parked();
    // The test platform doesn't necessarily draw a background window on its own.
    cx.update(|cx| {
        let (handle, _) = current_window(cx).expect("settings is open");
        handle.update(cx, |_, window, cx| window.draw(cx).clear(cx)).expect("draws settings");
    });
    cx.run_until_parked();
    let menu = record::open().expect("the platform recorded the select menu");
    let choice = menu
        .leaves()
        .into_iter()
        .find(|leaf| leaf.item.title.contains("200,000"))
        .expect("scrollback choice")
        .item
        .title
        .to_string();
    let (_, callbacks) = record::pick(&choice).expect("choice is enabled");
    cx.update(|cx| {
        let (handle, _) = current_window(cx).expect("settings is open");
        handle
            .update(cx, |_, window, cx| {
                for callback in callbacks {
                    callback(window, cx);
                }
            })
            .expect("runs the selection");
    });
    assert_eq!(cx.update(|cx| settings(cx).scrollback_lines), 200_000);
}

#[gpui_kit::test]
fn clicking_a_setting_select_opens_the_platform_menu(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "clicking_a_setting_select_opens_the_platform_menu") {
        return;
    }
    start(cx, 1);
    cx.update(|cx| open_settings(Some(Section::Terminal), cx));
    cx.run_until_parked();
    let handle = cx.update(|cx| current_window(cx).expect("settings is open").0);
    cx.update(|cx| handle.update(cx, |_, window, cx| window.draw(cx).clear(cx)).expect("draws"));
    record::close();
    let mut pointer = VisualTestContext::from_window(handle, cx);
    let at = pointer.debug_bounds("select-scrollback-trigger").expect("select is visible").center();
    pointer.simulate_mouse_move(at, None, Modifiers::none());
    pointer.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    pointer.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    assert!(
        record::open()
            .expect("the click opened a platform menu")
            .outline()
            .iter()
            .any(|line| line.contains("200,000"))
    );
}

#[gpui_kit::test]
fn navigating_and_opening_another_picker_dismisses_the_previous_one(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "navigating_and_opening_another_picker_dismisses_the_previous_one") {
        return;
    }
    start(cx, 1);
    cx.update(|cx| open_settings(Some(Section::Fonts), cx));
    cx.run_until_parked();
    let (handle, page) = cx.update(|cx| current_window(cx).expect("settings is open"));
    cx.update(|cx| {
        handle
            .update(cx, |_, window, cx| {
                page.update(cx, |page, cx| {
                    page.toggle_font_picker(FontSlot::Terminal, window, cx);
                    assert_eq!(page.font_picker.open, Some(FontSlot::Terminal));
                    page.show_section(Section::Projects, cx);
                    assert!(page.font_picker.open.is_none());
                    page.open_project_picker(window, cx);
                    assert!(page.projects.devctl_state(cx).contains("fallback open true"));
                    page.toggle_font_picker(FontSlot::Code, window, cx);
                    assert!(page.projects.devctl_state(cx).contains("fallback open false"));
                    assert_eq!(page.font_picker.open, Some(FontSlot::Code));
                    page.show_section(Section::Terminal, cx);
                    assert!(page.font_picker.open.is_none());
                })
            })
            .expect("settings stays open");
    });
}

#[gpui_kit::test]
fn return_in_font_search_chooses_the_first_matching_font(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "return_in_font_search_chooses_the_first_matching_font") {
        return;
    }
    start(cx, 1);
    cx.update(|cx| open_settings(Some(Section::Fonts), cx));
    cx.run_until_parked();
    let (handle, page) = cx.update(|cx| current_window(cx).expect("settings is open"));
    cx.update(|cx| {
        crate::settings::update_settings(cx, |settings| {
            settings.set_slot_family(FontSlot::Code, "Menlo".into())
        });
        handle
            .update(cx, |_, window, cx| {
                page.update(cx, |page, cx| {
                    page.toggle_font_picker(FontSlot::Code, window, cx);
                    page.font_picker
                        .search
                        .update(cx, |input, cx| input.set_value("Same as terminal", window, cx));
                })
            })
            .expect("settings stays open");
    });
    cx.run_until_parked();
    cx.update(|cx| {
        page.update(cx, |page, cx| {
            page.font_picker.search.update(cx, |_, cx| {
                cx.emit(gpui_kit::component::input::InputEvent::PressEnter {
                    secondary: false,
                    shift: false,
                });
            });
        })
    });
    cx.run_until_parked();
    assert_eq!(cx.update(|cx| settings(cx).slot_family(FontSlot::Code).to_string()), "");
    assert!(cx.update(|cx| page.read(cx).font_picker.open.is_none()));
}
