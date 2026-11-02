//! The Changed page: the nav lists it first and only while a setting differs from its default, with their
//! count; its page has the rows that differ in the controls of their own pages; Copy as JSON copies what
//! differs; Reset All… asks with the system's alert (Cancel keeps everything) and puts the listed settings
//! back. Its cost is held by `frame_tests`.
//!
//! The window runs in a child process with a scratch data dir (`isolate_in`), like the whole-window tests.
//! The alert is the platform's own only in the app: a test window has none, so GPUI's prompt asks instead,
//! which a test answers by the button's name and never shows.

use gpui_kit::accesskit::Role;
use gpui_kit::{AnyWindowHandle, TestAppContext, VisualTestContext};

use super::info_tests::draw;
use super::menu_tests::click;
use super::{Section, current_window, open_settings};
use crate::settings::{settings, update_settings};
use crate::window::test_harness::{isolate_in, start};

fn open(cx: &mut TestAppContext, section: Section) -> AnyWindowHandle {
    // The harness sets a few settings of its own (how the list numbers terminals): start from the defaults.
    cx.update(|cx| update_settings(cx, |s| *s = crate::settings::Settings::default()));
    cx.update(|cx| open_settings(Some(section), cx));
    cx.run_until_parked();
    let handle = cx.update(|cx| current_window(cx)).expect("the settings window opened").0;
    VisualTestContext::from_window(handle, cx)
        .simulate_resize(gpui_kit::size(gpui_kit::px(840.0), gpui_kit::px(1400.0)));
    cx.update(|cx| handle.update(cx, |_, window, _| window.simulate_a11y_active(true))).expect("open");
    draw(cx, handle);
    handle
}

/// The labels of the last frame's nodes of a role.
fn labels(cx: &mut TestAppContext, handle: AnyWindowHandle, role: Role) -> Vec<String> {
    let update = cx
        .update(|cx| handle.update(cx, |_, window, _| window.last_a11y_tree_update()))
        .expect("open")
        .expect("a tree");
    update
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == role)
        .filter_map(|(_, node)| node.label().map(str::to_string))
        .collect()
}

fn button(cx: &mut TestAppContext, handle: AnyWindowHandle, name: &str) -> gpui_kit::accesskit::NodeId {
    let update = cx
        .update(|cx| handle.update(cx, |_, window, _| window.last_a11y_tree_update()))
        .expect("open")
        .expect("a tree");
    update
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some(name))
        .map(|(id, _)| *id)
        .unwrap_or_else(|| panic!("a {name} button is on the page"))
}

#[gpui_kit::test]
fn the_nav_lists_the_page_only_while_something_differs_with_how_many(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_nav_lists_the_page_only_while_something_differs_with_how_many") {
        return;
    }
    start(cx, 1);
    let handle = open(cx, Section::Terminal);
    let listed = |cx: &mut TestAppContext| -> Vec<String> {
        labels(cx, handle, Role::Button).into_iter().filter(|n| n.starts_with("Changed")).collect()
    };
    assert!(listed(cx).is_empty(), "nothing differs: no row");
    cx.update(|cx| update_settings(cx, |s| s.tab_width = 8));
    draw(cx, handle);
    assert_eq!(listed(cx), ["Changed, 1 setting changed"]);
    cx.update(|cx| update_settings(cx, |s| s.diff_wrap = true));
    draw(cx, handle);
    assert_eq!(listed(cx), ["Changed, 2 settings changed"]);
}

#[gpui_kit::test]
fn the_page_shows_what_differs_in_its_own_controls_and_what_the_defaults_are(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_page_shows_what_differs_in_its_own_controls_and_what_the_defaults_are")
    {
        return;
    }
    start(cx, 1);
    let handle = open(cx, Section::Changed);
    assert!(
        labels(cx, handle, Role::Group).iter().any(|n| n == "Every setting is at its default"),
        "with nothing changed the page says so"
    );
    cx.update(|cx| update_settings(cx, |s| s.tab_width = 8));
    cx.update(|cx| update_settings(cx, |s| s.blur = !crate::settings::Settings::default().blur));
    draw(cx, handle);
    assert!(labels(cx, handle, Role::Group).iter().any(|n| n == "2 settings changed"));
    // The rows are the pages' own: Tab width's menu, and Blur's row with its switch.
    assert!(labels(cx, handle, Role::ComboBox).iter().any(|n| n == "Tab width"));
    let rows = labels(cx, handle, Role::Group);
    assert!(rows.iter().any(|n| n.starts_with("Blur")), "Blur's row is on the page: {rows:?}");
}

#[gpui_kit::test]
fn copy_as_json_copies_only_what_differs(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "copy_as_json_copies_only_what_differs") {
        return;
    }
    start(cx, 1);
    let handle = open(cx, Section::Changed);
    cx.update(|cx| update_settings(cx, |s| s.tab_width = 8));
    cx.update(|cx| update_settings(cx, |s| s.diff_wrap = true));
    draw(cx, handle);
    let mut keys = VisualTestContext::from_window(handle, cx);
    let copy = button(&mut keys, handle, "Copy as JSON");
    click(&mut keys, handle, copy);
    let copied = keys.read_from_clipboard().and_then(|item| item.text()).expect("something was copied");
    assert!(copied.starts_with('{') && copied.ends_with('}'), "{copied}");
    assert!(copied.contains("\"tab_width\": 8") && copied.contains("\"diff_wrap\": true"), "{copied}");
    assert!(!copied.contains("blur"), "what is at its default is not copied: {copied}");
    draw(&mut keys, handle);
    assert!(labels(&mut keys, handle, Role::Button).iter().any(|n| n == "Copied"), "the button says so");
}

#[gpui_kit::test]
fn reset_all_asks_first_and_cancel_keeps_everything(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "reset_all_asks_first_and_cancel_keeps_everything") {
        return;
    }
    start(cx, 1);
    let handle = open(cx, Section::Changed);
    cx.update(|cx| update_settings(cx, |s| s.tab_width = 8));
    cx.update(|cx| update_settings(cx, |s| s.diff_wrap = true));
    draw(cx, handle);
    let mut keys = VisualTestContext::from_window(handle, cx);
    let reset = button(&mut keys, handle, "Reset All…");
    click(&mut keys, handle, reset);
    assert!(keys.has_pending_prompt(), "the question is asked");
    assert_eq!(keys.update(|_, cx| settings(cx).tab_width), 8, "asking resets nothing");
    keys.simulate_prompt_answer("Cancel");
    keys.run_until_parked();
    assert_eq!(keys.update(|_, cx| settings(cx).tab_width), 8, "Cancel keeps everything");

    let reset = button(&mut keys, handle, "Reset All…");
    click(&mut keys, handle, reset);
    keys.simulate_prompt_answer("Reset");
    keys.run_until_parked();
    let (width, wrap) = keys.update(|_, cx| (settings(cx).tab_width, settings(cx).diff_wrap));
    let defaults = crate::settings::Settings::default();
    assert_eq!((width, wrap), (defaults.tab_width, defaults.diff_wrap), "Reset puts every listed one back");
    draw(&mut keys, handle);
    assert!(
        labels(&mut keys, handle, Role::Group).iter().any(|n| n == "Every setting is at its default"),
        "and the page says so, still open"
    );
}
