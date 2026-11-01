//! The nav: a sub-row under the open page for each titled group it draws, none while a search runs, the
//! keys that reach every row and the group a sub-row names becoming the current one, and what VoiceOver
//! hears. Its cost is held by `frame_tests`.
//!
//! The window runs in a child process with a scratch data dir (`isolate_in`), like the whole-window tests.

use gpui_kit::accesskit::{Role, TreeUpdate};
use gpui_kit::{AnyWindowHandle, KeyUpEvent, Keystroke, TestAppContext, VisualTestContext};
use wormhole_core::config::settings_index::Query;

use super::info_tests::{draw, open};
use super::{Section, current_window};
use crate::settings::settings;
use crate::window::test_harness::{isolate_in, start};

fn tree(cx: &mut TestAppContext, handle: AnyWindowHandle) -> TreeUpdate {
    cx.update(|cx| handle.update(cx, |_, window, _| window.last_a11y_tree_update()))
        .expect("open")
        .expect("a frame built the tree")
}

/// The names of the buttons in the last frame, and the name of the one with the keyboard.
fn buttons(cx: &mut TestAppContext, handle: AnyWindowHandle) -> (Vec<String>, Option<String>) {
    let update = tree(cx, handle);
    let name = |node: &gpui_kit::accesskit::Node| node.label().map(str::to_string);
    let focused = update.nodes.iter().find(|(id, _)| *id == update.focus).and_then(|(_, node)| name(node));
    let names = update
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button)
        .filter_map(|(_, node)| name(node))
        .collect();
    (names, focused)
}

/// The titled groups of `section` that its page draws: what the nav lists under it.
fn listed(cx: &mut TestAppContext, section: Section) -> Vec<&'static str> {
    cx.update(|cx| {
        let (_, page) = current_window(cx).expect("open");
        let settings = settings(cx);
        page.read(cx).nav_groups(section, settings).map(|(_, group)| group.title()).collect()
    })
}

fn search_for(cx: &mut TestAppContext, handle: AnyWindowHandle, word: &str) {
    let (_, page) = cx.update(|cx| current_window(cx)).expect("open");
    cx.update(|cx| {
        handle
            .update(cx, |_, window, cx| {
                page.update(cx, |page, cx| {
                    page.search.update(cx, |search, cx| search.set_value(word.to_string(), window, cx));
                    page.show_results = !Query::new(word).is_empty();
                    cx.notify();
                })
            })
            .expect("open");
    });
    cx.run_until_parked();
    draw(cx, handle);
}

#[gpui_kit::test]
fn the_open_page_lists_its_titled_groups_and_a_search_hides_them(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_open_page_lists_its_titled_groups_and_a_search_hides_them") {
        return;
    }
    start(cx, 1);
    let handle = open(cx);
    let titles = listed(cx, Section::Terminal);
    assert!(titles.len() >= 2, "the Terminal page has several groups: {titles:?}");
    assert!(titles.iter().all(|title| !title.is_empty()), "a group without a title gets no sub-row");
    assert!(listed(cx, Section::Profile).is_empty(), "the profile card has none");

    // Every page, then each titled group of the open one, is a button with a name.
    let (names, _) = buttons(cx, handle);
    let sections: Vec<Section> = cx.update(|cx| Section::visible(settings(cx)).collect());
    for section in sections {
        assert!(
            names.iter().any(|name| name.starts_with(section.title())),
            "{section:?} has a name: {names:?}"
        );
    }
    for title in &titles {
        assert_eq!(names.iter().filter(|name| *name == title).count(), 1, "one sub-row for {title}");
    }

    // A search shows its counts instead, said aloud, and no sub-rows.
    search_for(cx, handle, "line");
    let (names, _) = buttons(cx, handle);
    assert!(names.iter().any(|name| name.contains("match")), "a row says how many it found: {names:?}");
    for title in &titles {
        assert!(!names.contains(&(*title).to_string()), "{title} gives way to the search");
    }
}

#[gpui_kit::test]
fn tab_and_the_arrows_reach_the_sub_rows_and_a_press_picks_the_group(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "tab_and_the_arrows_reach_the_sub_rows_and_a_press_picks_the_group") {
        return;
    }
    start(cx, 1);
    // Sessions is the longest page: its last group is far below the top.
    cx.update(|cx| super::open_settings(Some(Section::Sessions), cx));
    cx.run_until_parked();
    let handle = cx.update(|cx| current_window(cx)).expect("opened").0;
    cx.update(|cx| handle.update(cx, |_, window, _| window.simulate_a11y_active(true))).expect("open");
    draw(cx, handle);
    let titles = listed(cx, Section::Sessions);
    let last = *titles.last().expect("the Sessions page has groups");

    let mut keys = VisualTestContext::from_window(handle, cx);
    // ⇥ walks the stops from the search field: the profile card, the rows, the sub-rows.
    let mut walked = Vec::new();
    for _ in 0..60 {
        keys.simulate_keystrokes("tab");
        let (_, focus) = buttons(&mut keys, handle);
        if let Some(name) = focus {
            walked.push(name.clone());
            if name == last {
                break;
            }
        }
    }
    assert_eq!(walked.last().map(String::as_str), Some(last), "⇥ reaches the last sub-row: {walked:?}");
    // ↑ goes back one stop, ↓ forward again.
    keys.simulate_keystrokes("up");
    let (_, back) = buttons(&mut keys, handle);
    assert_ne!(back.as_deref(), Some(last), "↑ leaves it");
    keys.simulate_keystrokes("down");
    assert_eq!(buttons(&mut keys, handle).1.as_deref(), Some(last), "↓ comes back");

    // ↩ presses it: the sub-row reads as the current one, and the first no longer does. (The scroll itself
    // is made by GPUI's next-frame callbacks, which the test platform doesn't run: `settings-shot` shows it.)
    let first = *titles.first().expect("a first group");
    assert_ne!(first, last);
    assert_eq!(selected(&mut keys, handle, last), Some(false));
    let keystroke = Keystroke::parse("enter").expect("a key");
    keys.simulate_keystrokes("enter");
    keys.simulate_event(KeyUpEvent { keystroke });
    draw(&mut keys, handle);
    assert_eq!(selected(&mut keys, handle, last), Some(true), "the picked group is current");
    assert_eq!(selected(&mut keys, handle, first), Some(false), "and the first is not");
}

/// Whether the button called `name` is marked selected in the last frame.
fn selected(cx: &mut TestAppContext, handle: AnyWindowHandle, name: &str) -> Option<bool> {
    let update = tree(cx, handle);
    update
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some(name))
        .map(|(_, node)| node.is_selected() == Some(true))
}
