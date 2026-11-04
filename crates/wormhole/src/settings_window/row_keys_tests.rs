//! A row's config key and menu: the key shows with ⌥ held and for a moment after a copy, a click on it
//! copies it and changes nothing, and the row's menu (right click, ⌃↩, VoiceOver's Show menu) copies the
//! key or its value as JSON and puts the default back. Their cost is held by `frame_tests`.
//!
//! The window runs in a child process with a scratch data dir (`isolate_in`), like the whole-window tests.
//! The row is Terminal's Minimap, a switch with one key; the clipboard is the app's own (`platform::clipboard`).

use std::time::Duration;

use gpui_kit::accesskit::{Action, ActionRequest, NodeId, Role, TreeId};
use gpui_kit::{
    AnyWindowHandle, Capslock, Modifiers, ModifiersChangedEvent, MouseButton, MouseDownEvent, MouseUpEvent,
    TestAppContext, VisualTestContext,
};
use wormhole_core::config::Settings;
use wormhole_core::config::settings_index;

use super::info_tests::{centre, draw, open};
use crate::settings::{settings, update_settings};
use crate::style::row_keys::SHOWN_KEYS;
use crate::window::test_harness::{isolate_in, start};

const KEY: &str = "minimap";
const ID: &str = "minimap";

fn label() -> &'static str {
    settings_index::entry(ID).expect("the row").2.label()
}

/// The a11y node of the row `ID` (a group named by its label).
fn row(cx: &mut TestAppContext, handle: AnyWindowHandle) -> NodeId {
    let update = cx
        .update(|cx| handle.update(cx, |_, window, _| window.last_a11y_tree_update()))
        .expect("open")
        .expect("a tree");
    update
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Group && node.label() == Some(label()))
        .map(|(id, _)| *id)
        .expect("the row is on the page")
}

/// The key's button of the row, found by what it says to VoiceOver: Copy Key, described by the key.
fn key_button(cx: &mut TestAppContext, handle: AnyWindowHandle) -> NodeId {
    let update = cx
        .update(|cx| handle.update(cx, |_, window, _| window.last_a11y_tree_update()))
        .expect("open")
        .expect("a tree");
    update
        .nodes
        .iter()
        .find(|(_, node)| {
            node.role() == Role::Button && node.label() == Some("Copy Key") && node.description() == Some(KEY)
        })
        .map(|(id, _)| *id)
        .expect("the row's key is a button for VoiceOver")
}

/// Change the row's value, so it is a changed row with a default to go back to.
fn change(cx: &mut TestAppContext, handle: AnyWindowHandle) -> bool {
    let default = Settings::default().minimap;
    cx.update(|cx| update_settings(cx, |s| s.minimap = !default));
    draw(cx, handle);
    default
}

/// Settings → Advanced → Show config keys, which is off until the person turns it on.
fn show_config_keys(cx: &mut TestAppContext, on: bool) {
    cx.update(|cx| update_settings(cx, |s| s.show_config_keys = on));
}

fn minimap(cx: &mut TestAppContext) -> bool {
    cx.update(|cx| settings(cx).minimap)
}

fn clipboard(cx: &mut TestAppContext) -> Option<String> {
    cx.update(|cx| crate::platform::clipboard::read(cx))
}

fn right_click(keys: &mut VisualTestContext, at: gpui_kit::Point<gpui_kit::Pixels>) {
    keys.simulate_event(MouseDownEvent {
        button: MouseButton::Right,
        position: at,
        modifiers: Modifiers::none(),
        click_count: 1,
        first_mouse: false,
    });
    keys.simulate_event(MouseUpEvent {
        button: MouseButton::Right,
        position: at,
        modifiers: Modifiers::none(),
        click_count: 1,
    });
}

/// Where a right click lands on the row without landing on its control: a little in from its left edge.
fn on_the_row(cx: &mut TestAppContext, handle: AnyWindowHandle) -> gpui_kit::Point<gpui_kit::Pixels> {
    let node = row(cx, handle);
    let mid = centre(cx, handle, node);
    let bounds = cx
        .update(|cx| handle.update(cx, |_, window, _| window.last_a11y_tree_update()))
        .expect("open")
        .expect("a tree")
        .nodes
        .iter()
        .find(|(id, _)| *id == node)
        .and_then(|(_, node)| node.bounds())
        .expect("bounds");
    let scale =
        f64::from(cx.update(|cx| handle.update(cx, |_, window, _| window.scale_factor())).expect("open"));
    gpui_kit::point(gpui_kit::px(((bounds.x0 / scale) + 12.0) as f32), mid.y)
}

/// What the open menu offers, as titles.
fn menu_titles() -> Vec<String> {
    crate::platform::record::open()
        .expect("a menu is open")
        .outline()
        .into_iter()
        .filter_map(|line| {
            line.strip_prefix("- ").map(|title| title.split(" [").next().unwrap_or("").to_string())
        })
        .collect()
}

fn pick(cx: &mut TestAppContext, handle: AnyWindowHandle, title: &str) {
    let (_, callbacks) = crate::platform::record::pick(title).expect("the item is in the menu");
    cx.update(|cx| {
        handle
            .update(cx, |_, window, cx| {
                for callback in &callbacks {
                    callback(window, cx);
                }
            })
            .expect("open");
    });
}

fn shown_keys(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Vec<String> {
    SHOWN_KEYS.with(|shown| shown.borrow_mut().clear());
    draw(cx, handle);
    SHOWN_KEYS.with(|shown| shown.borrow().clone())
}

#[gpui_kit::test]
fn a_right_click_opens_the_rows_menu_and_reset_puts_the_default_back(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_right_click_opens_the_rows_menu_and_reset_puts_the_default_back") {
        return;
    }
    start(cx, 1);
    let handle = open(cx);
    let default = change(cx, handle);
    assert_eq!(minimap(cx), !default);
    let at = on_the_row(cx, handle);
    let mut keys = VisualTestContext::from_window(handle, cx);
    right_click(&mut keys, at);
    assert_eq!(menu_titles(), ["Copy Key", "Copy as JSON", "Reset to Default"]);
    pick(&mut keys, handle, "Reset to Default");
    assert_eq!(minimap(&mut keys), default, "Reset to Default put the default back");
}

#[gpui_kit::test]
fn the_menu_copies_the_key_and_the_value_as_json_and_resets_only_a_changed_row(cx: &mut TestAppContext) {
    if isolate_in(
        module_path!(),
        "the_menu_copies_the_key_and_the_value_as_json_and_resets_only_a_changed_row",
    ) {
        return;
    }
    start(cx, 1);
    let handle = open(cx);
    let at = on_the_row(cx, handle);
    let mut keys = VisualTestContext::from_window(handle, cx);
    right_click(&mut keys, at);
    let reset = crate::platform::record::open().expect("open").outline();
    assert!(reset.iter().any(|line| line.contains("Reset to Default [off]")), "nothing to reset: {reset:?}");
    pick(&mut keys, handle, "Copy Key");
    assert_eq!(clipboard(&mut keys).as_deref(), Some(KEY));

    let default = change(&mut keys, handle);
    let at = on_the_row(&mut keys, handle);
    right_click(&mut keys, at);
    pick(&mut keys, handle, "Copy as JSON");
    assert_eq!(clipboard(&mut keys), Some(format!(r#"{{"{KEY}": {}}}"#, !default)), "the current value");
}

#[gpui_kit::test]
fn the_menu_opens_without_the_pointer_by_ctrl_enter_and_by_voiceovers_show_menu(cx: &mut TestAppContext) {
    if isolate_in(
        module_path!(),
        "the_menu_opens_without_the_pointer_by_ctrl_enter_and_by_voiceovers_show_menu",
    ) {
        return;
    }
    start(cx, 1);
    let handle = open(cx);
    // VoiceOver's action on the row.
    let target = row(cx, handle);
    cx.update(|cx| {
        handle.update(cx, |_, window, cx| {
            window.simulate_a11y_action(
                ActionRequest {
                    action: Action::ShowContextMenu,
                    target_tree: TreeId::ROOT,
                    target_node: target,
                    data: None,
                },
                cx,
            )
        })
    })
    .expect("open");
    cx.run_until_parked();
    assert!(menu_titles().contains(&"Copy Key".to_string()), "Show menu opens the row's menu");
    crate::platform::record::close();

    // ⌃↩ on a control of the row: Tab to the switch, then the keys.
    let mut keys = VisualTestContext::from_window(handle, cx);
    let mut opened = false;
    for _ in 0..80 {
        keys.simulate_keystrokes("tab");
        keys.simulate_keystrokes("ctrl-enter");
        if crate::platform::record::open().is_some() {
            opened = true;
            break;
        }
    }
    assert!(opened, "⌃↩ on a focused control opens its row's menu");
    assert!(menu_titles().contains(&"Copy as JSON".to_string()));
}

#[gpui_kit::test]
fn holding_option_shows_every_key_and_letting_go_hides_them(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "holding_option_shows_every_key_and_letting_go_hides_them") {
        return;
    }
    start(cx, 1);
    show_config_keys(cx, true);
    let handle = open(cx);
    assert!(shown_keys(cx, handle).is_empty(), "the keys wait for the pointer");
    let mut keys = VisualTestContext::from_window(handle, cx);
    let modifiers = Modifiers { alt: true, ..Modifiers::none() };
    keys.simulate_event(ModifiersChangedEvent { modifiers, capslock: Capslock::default() });
    let shown = shown_keys(&mut keys, handle);
    assert!(shown.len() > 3, "every row of the page shows its key: {shown:?}");
    assert!(shown.contains(&KEY.to_string()));
    keys.simulate_event(ModifiersChangedEvent {
        modifiers: Modifiers::none(),
        capslock: Capslock::default(),
    });
    assert!(shown_keys(&mut keys, handle).is_empty(), "letting go hides them again");
}

#[gpui_kit::test]
fn a_click_on_the_key_copies_it_and_says_copied_for_a_moment_and_changes_nothing(cx: &mut TestAppContext) {
    if isolate_in(
        module_path!(),
        "a_click_on_the_key_copies_it_and_says_copied_for_a_moment_and_changes_nothing",
    ) {
        return;
    }
    start(cx, 1);
    show_config_keys(cx, true);
    let handle = open(cx);
    let before = minimap(cx);
    let button = key_button(cx, handle);
    let at = centre(cx, handle, button);
    let mut keys = VisualTestContext::from_window(handle, cx);
    // The key is hidden until the pointer is over its row, and a hidden one takes no click.
    keys.simulate_mouse_move(at, None, Modifiers::none());
    draw(&mut keys, handle);
    keys.simulate_click(at, Modifiers::none());
    assert_eq!(clipboard(&mut keys).as_deref(), Some(KEY), "the click copied the key");
    assert_eq!(minimap(&mut keys), before, "and did not touch the row's value");
    assert_eq!(shown_keys(&mut keys, handle), [KEY], "only the copied row says Copied");
    keys.executor().advance_clock(Duration::from_secs(2));
    keys.run_until_parked();
    assert!(shown_keys(&mut keys, handle).is_empty(), "and only for a moment");
}

#[gpui_kit::test]
fn voiceover_can_press_the_key_while_it_is_hidden(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "voiceover_can_press_the_key_while_it_is_hidden") {
        return;
    }
    start(cx, 1);
    show_config_keys(cx, true);
    let handle = open(cx);
    let target = key_button(cx, handle);
    cx.update(|cx| {
        handle.update(cx, |_, window, cx| {
            window.simulate_a11y_action(
                ActionRequest {
                    action: Action::Click,
                    target_tree: TreeId::ROOT,
                    target_node: target,
                    data: None,
                },
                cx,
            )
        })
    })
    .expect("open");
    cx.run_until_parked();
    assert_eq!(clipboard(cx).as_deref(), Some(KEY), "pressing the key copies it without the pointer");
}

#[gpui_kit::test]
fn a_copied_left_over_from_a_closed_window_is_not_shown_in_the_next_one(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_copied_left_over_from_a_closed_window_is_not_shown_in_the_next_one") {
        return;
    }
    start(cx, 1);
    show_config_keys(cx, true);
    let handle = open(cx);
    let button = key_button(cx, handle);
    let at = centre(cx, handle, button);
    {
        let mut keys = VisualTestContext::from_window(handle, cx);
        keys.simulate_mouse_move(at, None, Modifiers::none());
        draw(&mut keys, handle);
        keys.simulate_click(at, Modifiers::none());
        assert_eq!(shown_keys(&mut keys, handle), [KEY], "the row says Copied");
    }
    // The window goes before the timer that would clear it, so that timer finds nothing to update.
    cx.update(|cx| handle.update(cx, |_, window, _| window.remove_window())).expect("open");
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_secs(2));
    cx.run_until_parked();
    let reopened = open(cx);
    assert!(shown_keys(cx, reopened).is_empty(), "a new window does not inherit the old Copied");
}

#[gpui_kit::test]
fn with_show_config_keys_off_no_row_draws_a_key_not_even_with_option_held_and_the_menu_still_works(
    cx: &mut TestAppContext,
) {
    if isolate_in(
        module_path!(),
        "with_show_config_keys_off_no_row_draws_a_key_not_even_with_option_held_and_the_menu_still_works",
    ) {
        return;
    }
    start(cx, 1);
    assert!(!cx.update(|cx| settings(cx).show_config_keys), "hidden by default");
    let handle = open(cx);
    let mut keys = VisualTestContext::from_window(handle, cx);
    let modifiers = Modifiers { alt: true, ..Modifiers::none() };
    keys.simulate_event(ModifiersChangedEvent { modifiers, capslock: Capslock::default() });
    assert!(shown_keys(&mut keys, handle).is_empty(), "⌥ shows no key while the setting is off");
    let at = on_the_row(&mut keys, handle);
    right_click(&mut keys, at);
    assert_eq!(menu_titles(), ["Copy Key", "Copy as JSON", "Reset to Default"]);
    pick(&mut keys, handle, "Copy Key");
    assert_eq!(clipboard(&mut keys).as_deref(), Some(KEY), "Copy Key still copies the key");
    show_config_keys(&mut keys, true);
    assert!(shown_keys(&mut keys, handle).contains(&KEY.to_string()), "on: ⌥ shows the keys again");
}
