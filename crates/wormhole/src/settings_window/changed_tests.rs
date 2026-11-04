//! The Changed page: the nav's badge counts the settings that differ from their defaults; the page lists
//! them in the controls of their own pages, each under its page's name, with a reset arrow of its own; Copy
//! as JSON copies what differs; Reset All… asks first (Cancel keeps everything) and puts back the listed
//! settings and nothing else. Which rows differ is core's (`settings_index`'s tests); this is the page.
//! Its cost is held by `frame_tests`.
//!
//! The window runs in a child process with a scratch data dir (`isolate_in`), like the whole-window tests.

use std::path::Path;

use gpui_kit::accesskit::{NodeId, Role, TreeUpdate};
use gpui_kit::{AnyWindowHandle, Modifiers, TestAppContext, VisualTestContext};
use wormhole_core::config::settings_index;

use super::info_tests::{centre, draw};
use super::{Section, current_window, open_settings};
use crate::settings::{Settings, settings, update_settings};
use crate::window::test_harness::{isolate_in, start};

fn open(cx: &mut TestAppContext, section: Section) -> AnyWindowHandle {
    // The harness sets a few settings of its own (how the list numbers terminals): start from the defaults.
    cx.update(|cx| update_settings(cx, |s| *s = Settings::default()));
    cx.update(|cx| open_settings(Some(section), cx));
    cx.run_until_parked();
    let handle = cx.update(|cx| current_window(cx)).expect("the settings window opened").0;
    VisualTestContext::from_window(handle, cx)
        .simulate_resize(gpui_kit::size(gpui_kit::px(840.0), gpui_kit::px(1400.0)));
    cx.update(|cx| handle.update(cx, |_, window, _| window.simulate_a11y_active(true))).expect("open");
    draw(cx, handle);
    handle
}

fn tree(cx: &mut TestAppContext, handle: AnyWindowHandle) -> TreeUpdate {
    cx.update(|cx| handle.update(cx, |_, window, _| window.last_a11y_tree_update()))
        .expect("open")
        .expect("a tree")
}

/// The first node of a role with this label in the last frame.
fn find(update: &TreeUpdate, role: Role, label: &str) -> Option<NodeId> {
    update
        .nodes
        .iter()
        .find(|(_, node)| node.role() == role && node.label() == Some(label))
        .map(|(id, _)| *id)
}

/// Every node below `root`, in the tree's order.
fn below(update: &TreeUpdate, root: NodeId) -> Vec<NodeId> {
    let mut found = Vec::new();
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        let Some((_, node)) = update.nodes.iter().find(|(node, _)| *node == id) else { continue };
        for child in node.children().iter().rev() {
            pending.push(*child);
        }
        if id != root {
            found.push(id);
        }
    }
    found
}

fn button(cx: &mut TestAppContext, handle: AnyWindowHandle, name: &str) -> NodeId {
    let update = tree(cx, handle);
    find(&update, Role::Button, name).unwrap_or_else(|| panic!("a {name} button is on the page"))
}

fn click(keys: &mut VisualTestContext, handle: AnyWindowHandle, node: NodeId) {
    let at = centre(keys, handle, node);
    keys.simulate_click(at, Modifiers::none());
    keys.run_until_parked();
    draw(keys, handle);
}

/// The label of a setting's row in the index.
fn label(id: &str) -> &'static str {
    settings_index::entry(id).expect("the row").2.label()
}

/// The title of the page a setting lives on.
fn page_of(id: &str) -> &'static str {
    settings_index::entry(id).expect("the row").0.title()
}

fn differing(cx: &mut TestAppContext) -> usize {
    cx.update(|cx| super::changed::differing(cx))
}

fn differ(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    let defaults = Settings::default();
    cx.update(|cx| {
        update_settings(cx, |s| {
            s.tab_width = 8;
            s.blur = !defaults.blur;
        })
    });
    draw(cx, handle);
}

#[gpui_kit::test]
fn the_nav_counts_the_settings_that_differ_and_the_page_with_none_says_so(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_nav_counts_the_settings_that_differ_and_the_page_with_none_says_so") {
        return;
    }
    start(cx, 1);
    let handle = open(cx, Section::Changed);
    assert_eq!(differing(cx), 0, "nothing differs");
    let update = tree(cx, handle);
    assert!(find(&update, Role::Group, "No settings changed").is_some(), "the card says so");
    assert!(find(&update, Role::Button, "Reset All…").is_none(), "with nothing to reset: no button");
    assert!(find(&update, Role::Button, "Copy as JSON").is_none(), "nor anything to copy");

    cx.update(|cx| update_settings(cx, |s| s.tab_width = 8));
    assert_eq!(differing(cx), 1);
    cx.update(|cx| update_settings(cx, |s| s.diff_wrap = !Settings::default().diff_wrap));
    assert_eq!(differing(cx), 2);
    draw(cx, handle);
    let update = tree(cx, handle);
    assert!(find(&update, Role::Group, "2 settings changed").is_some(), "the card counts them");
    cx.update(|cx| update_settings(cx, |s| s.diff_wrap = Settings::default().diff_wrap));
    draw(cx, handle);
    assert!(
        find(&tree(cx, handle), Role::Group, "1 setting changed").is_some(),
        "and says it in the singular for one"
    );
}

#[gpui_kit::test]
fn a_changed_setting_is_listed_in_its_own_row_under_its_page_and_its_arrow_puts_it_back(
    cx: &mut TestAppContext,
) {
    if isolate_in(
        module_path!(),
        "a_changed_setting_is_listed_in_its_own_row_under_its_page_and_its_arrow_puts_it_back",
    ) {
        return;
    }
    start(cx, 1);
    let handle = open(cx, Section::Changed);
    differ(cx, handle);
    let update = tree(cx, handle);
    // Each changed setting is the row of its page, under a group named for that page.
    for id in ["blur", "tab-width"] {
        let page =
            find(&update, Role::Group, page_of(id)).unwrap_or_else(|| panic!("a group for {id}'s page"));
        let row = find(&update, Role::Group, label(id)).unwrap_or_else(|| panic!("{id}'s row is listed"));
        assert!(below(&update, page).contains(&row), "{id} is under {}", page_of(id));
    }
    let places: Vec<&str> = [page_of("blur"), page_of("tab-width")].to_vec();
    assert_ne!(places[0], places[1], "two pages, two headings");
    let at = |title: &str| {
        update.nodes.iter().position(|(_, n)| n.role() == Role::Group && n.label() == Some(title))
    };
    assert!(at(page_of("blur")) < at(page_of("tab-width")), "in the order of the pages");
    assert!(find(&update, Role::Group, label("minimap")).is_none(), "a setting at its default is not listed");

    // The arrow at the end of a row puts back that setting and no other.
    let row = find(&update, Role::Group, label("tab-width")).expect("the row");
    let arrow = below(&update, row)
        .into_iter()
        .find(|id| {
            update.nodes.iter().any(|(node, n)| {
                node == id && n.role() == Role::Button && n.label() == Some("Reset to default")
            })
        })
        .expect("the row has a reset arrow, in view without the pointer");
    let mut keys = VisualTestContext::from_window(handle, cx);
    click(&mut keys, handle, arrow);
    let (width, blur) = keys.update(|_, cx| (settings(cx).tab_width, settings(cx).blur));
    let defaults = Settings::default();
    assert_eq!(width, defaults.tab_width, "the arrow put the default back");
    assert_eq!(blur, !defaults.blur, "and left the other change alone");
    assert_eq!(differing(&mut keys), 1);
    let update = tree(&mut keys, handle);
    assert!(find(&update, Role::Group, label("tab-width")).is_none(), "its row is gone from the page");
    assert!(find(&update, Role::Group, label("blur")).is_some());
    assert!(find(&update, Role::Group, "1 setting changed").is_some());
}

#[gpui_kit::test]
fn copy_as_json_copies_only_what_differs(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "copy_as_json_copies_only_what_differs") {
        return;
    }
    start(cx, 1);
    let handle = open(cx, Section::Changed);
    differ(cx, handle);
    let mut keys = VisualTestContext::from_window(handle, cx);
    let copy = button(&mut keys, handle, "Copy as JSON");
    click(&mut keys, handle, copy);
    let copied = keys.update(|_, cx| crate::platform::clipboard::read(cx)).expect("something was copied");
    assert!(copied.starts_with('{') && copied.ends_with('}'), "{copied}");
    assert!(copied.contains("\"tab_width\": 8"), "{copied}");
    assert!(copied.contains("\"blur\""), "{copied}");
    assert!(!copied.contains("diff_wrap"), "what is at its default is not copied: {copied}");
    assert!(find(&tree(&mut keys, handle), Role::Button, "Copied").is_some(), "the button says so");
}

#[gpui_kit::test]
fn reset_all_asks_first_and_puts_back_the_listed_settings_and_nothing_else(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "reset_all_asks_first_and_puts_back_the_listed_settings_and_nothing_else") {
        return;
    }
    start(cx, 1);
    let handle = open(cx, Section::Changed);
    differ(cx, handle);
    // A pinned folder differs from the default too, but has no row in Settings: not listed, so not reset.
    cx.update(|cx| update_settings(cx, |s| assert!(s.set_pinned(Path::new("/tmp"), true))));
    draw(cx, handle);
    assert_eq!(differing(cx), 2, "the pin is not one of the listed settings");
    let defaults = Settings::default();

    let mut keys = VisualTestContext::from_window(handle, cx);
    let reset_all = button(&mut keys, handle, "Reset All…");
    click(&mut keys, handle, reset_all);
    assert_eq!(keys.update(|_, cx| settings(cx).tab_width), 8, "asking resets nothing");
    assert!(
        find(&tree(&mut keys, handle), Role::Button, "Reset").is_some(),
        "the question is asked, with a Reset button"
    );
    let cancel = button(&mut keys, handle, "Cancel");
    click(&mut keys, handle, cancel);
    assert_eq!(keys.update(|_, cx| settings(cx).tab_width), 8, "Cancel keeps everything");
    assert!(find(&tree(&mut keys, handle), Role::Button, "Reset").is_none(), "and withdraws the question");

    let reset_all = button(&mut keys, handle, "Reset All…");
    click(&mut keys, handle, reset_all);
    let reset = button(&mut keys, handle, "Reset");
    click(&mut keys, handle, reset);
    let (width, blur, pinned) =
        keys.update(|_, cx| (settings(cx).tab_width, settings(cx).blur, settings(cx).files_pinned.clone()));
    assert_eq!((width, blur), (defaults.tab_width, defaults.blur), "Reset puts every listed one back");
    assert_eq!(pinned, ["/tmp"], "and nothing else");
    assert_eq!(differing(&mut keys), 0);
    assert!(
        find(&tree(&mut keys, handle), Role::Group, "No settings changed").is_some(),
        "the page says so, still open"
    );
}
