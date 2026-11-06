//! The Components page: its specimens work on sample state kept in the page, and changing it writes nothing
//! to the settings (which `config.json` is saved from). Its cost is held by `frame_tests`.
//!
//! The window runs in a child process with a scratch data dir (`isolate_in`), like the whole-window tests.

use gpui_kit::accesskit::{NodeId, Toggled, TreeUpdate};
use gpui_kit::{AnyWindowHandle, Modifiers, TestAppContext, VisualTestContext};

use super::info_tests::{centre, draw};
use super::{Section, current_window, open_settings};
use crate::settings::{Settings, settings, update_settings};
use crate::window::test_harness::{isolate_in, start};

/// A checkbox of the page, by the words it says to VoiceOver.
const CHECKBOX: &str = "Show on every page";

fn open(cx: &mut TestAppContext) -> AnyWindowHandle {
    cx.update(|cx| update_settings(cx, |s| *s = Settings::default()));
    cx.update(|cx| open_settings(Some(Section::Components), cx));
    cx.run_until_parked();
    let handle = cx.update(|cx| current_window(cx)).expect("the settings window opened").0;
    // Tall, so the whole page is laid out inside the window and every specimen has bounds.
    VisualTestContext::from_window(handle, cx)
        .simulate_resize(gpui_kit::size(gpui_kit::px(840.0), gpui_kit::px(6000.0)));
    cx.update(|cx| handle.update(cx, |_, window, _| window.simulate_a11y_active(true))).expect("open");
    draw(cx, handle);
    handle
}

fn tree(cx: &mut TestAppContext, handle: AnyWindowHandle) -> TreeUpdate {
    cx.update(|cx| handle.update(cx, |_, window, _| window.last_a11y_tree_update()))
        .expect("open")
        .expect("a frame built the tree")
}

/// The node that says `words`, and whether it is switched on.
fn node_of(cx: &mut TestAppContext, handle: AnyWindowHandle, words: &str) -> (NodeId, Option<Toggled>) {
    let tree = tree(cx, handle);
    let (id, node) = tree
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some(words))
        .unwrap_or_else(|| panic!("nothing in the page says {words:?}"));
    (*id, node.toggled())
}

#[gpui_kit::test]
fn a_sample_control_flips_in_the_page_and_writes_no_setting(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_sample_control_flips_in_the_page_and_writes_no_setting") {
        return;
    }
    start(cx, 1);
    let handle = open(cx);
    let before = cx.update(|cx| serde_json::to_value(settings(cx)).expect("settings as JSON"));
    let (id, was) = node_of(cx, handle, CHECKBOX);
    let at = centre(cx, handle, id);
    let mut keys = VisualTestContext::from_window(handle, cx);
    keys.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    draw(cx, handle);
    let (_, now) = node_of(cx, handle, CHECKBOX);
    assert_ne!(was, now, "the checkbox flipped");
    let after = cx.update(|cx| serde_json::to_value(settings(cx)).expect("settings as JSON"));
    assert_eq!(before, after, "the sample state is the page's: no setting changed");
}
