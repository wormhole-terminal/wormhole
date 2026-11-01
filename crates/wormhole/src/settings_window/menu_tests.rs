//! A row's menu control (`select::menu`) on Changes & diffs' Tab width: a pop-up button that says its
//! value to VoiceOver, ⇥ reaches it, ↩ and Space open it, the arrows and ↩ choose, Esc closes and leaves
//! the keyboard on it, a click on an option chooses, and a width written in config.json by hand shows as
//! it is, in the list and checked, and stays until another is chosen. Its cost is held by `frame_tests`.
//!
//! The window runs in a child process with a scratch data dir (`isolate_in`), like the whole-window tests.

use gpui_kit::accesskit::{Action, ActionRequest, NodeId, Role, TreeId};
use gpui_kit::{AnyWindowHandle, KeyUpEvent, Keystroke, Modifiers, TestAppContext, VisualTestContext};

use super::info_tests::{centre, draw};
use super::{Section, current_window, open_settings};
use crate::settings::{settings, update_settings};
use crate::window::test_harness::{isolate_in, start};

const LABEL: &str = "Tab width";

fn open(cx: &mut TestAppContext) -> AnyWindowHandle {
    cx.update(|cx| open_settings(Some(Section::Git), cx));
    cx.run_until_parked();
    let handle = cx.update(|cx| current_window(cx)).expect("the settings window opened").0;
    cx.update(|cx| handle.update(cx, |_, window, _| window.simulate_a11y_active(true))).expect("open");
    draw(cx, handle);
    handle
}

/// What the last frame's tree holds of the menu control: its node, its value, whether it has focus.
fn control(cx: &mut TestAppContext, handle: AnyWindowHandle) -> (NodeId, String, bool, Option<bool>) {
    let update = cx
        .update(|cx| handle.update(cx, |_, window, _| window.last_a11y_tree_update()))
        .expect("open")
        .expect("a tree");
    let (id, node) = update
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::ComboBox && node.label() == Some(LABEL))
        .expect("the Tab width menu is on the page");
    (*id, node.value().unwrap_or_default().to_string(), update.focus == *id, node.is_expanded())
}

/// The options of the open menu as VoiceOver has them: node, name, whether it is the selected one.
fn options(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Vec<(NodeId, String, bool)> {
    let update = cx
        .update(|cx| handle.update(cx, |_, window, _| window.last_a11y_tree_update()))
        .expect("open")
        .expect("a tree");
    update
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::ListBoxOption)
        .map(|(id, node)| {
            (*id, node.label().unwrap_or_default().to_string(), node.is_selected() == Some(true))
        })
        .collect()
}

/// A click in the middle of the node.
fn click(keys: &mut VisualTestContext, handle: AnyWindowHandle, node: NodeId) {
    let at = centre(keys, handle, node);
    keys.simulate_click(at, Modifiers::none());
}

fn tab_width(cx: &mut TestAppContext) -> usize {
    cx.update(|cx| settings(cx).tab_width)
}

/// ↩ or Space as the keyboard sends it: GPUI clicks a focused element when the key comes back up.
fn activate(keys: &mut VisualTestContext, key: &str) {
    keys.simulate_keystrokes(key);
    keys.simulate_event(KeyUpEvent { keystroke: Keystroke::parse(key).expect("a key") });
}

fn focus_the_menu(keys: &mut VisualTestContext, handle: AnyWindowHandle) {
    for _ in 0..80 {
        keys.simulate_keystrokes("tab");
        if control(keys, handle).2 {
            return;
        }
    }
    panic!("Tab reaches the menu");
}

#[gpui_kit::test]
fn the_menu_says_its_value_and_is_chosen_from_by_keyboard(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_menu_says_its_value_and_is_chosen_from_by_keyboard") {
        return;
    }
    start(cx, 1);
    let handle = open(cx);
    let (_, value, _, expanded) = control(cx, handle);
    assert_eq!(value, "4 spaces", "the control tells VoiceOver its current value");
    assert_eq!(expanded, Some(false));
    let mut keys = VisualTestContext::from_window(handle, cx);
    focus_the_menu(&mut keys, handle);

    activate(&mut keys, "enter");
    assert_eq!(control(&mut keys, handle).3, Some(true), "↩ opens it");
    let listed = options(&mut keys, handle);
    assert_eq!(
        listed.iter().map(|(_, name, selected)| (name.as_str(), *selected)).collect::<Vec<_>>(),
        [("4 spaces", true), ("2 spaces", false), ("8 spaces", false)]
    );

    keys.simulate_keystrokes("escape");
    assert_eq!(control(&mut keys, handle).3, Some(false), "Esc closes it");
    assert!(control(&mut keys, handle).2, "and the keyboard stays on it");
    assert_eq!(tab_width(&mut keys), 4, "closing chooses nothing");

    activate(&mut keys, "space");
    assert_eq!(control(&mut keys, handle).3, Some(true), "Space opens it too");
    keys.simulate_keystrokes("down");
    keys.simulate_keystrokes("down");
    keys.simulate_keystrokes("up");
    keys.simulate_keystrokes("enter");
    keys.simulate_event(KeyUpEvent { keystroke: Keystroke::parse("enter").expect("a key") });
    assert_eq!(tab_width(&mut keys), 2, "↓ ↓ ↑ ↩ chooses the second option");
    let (_, value, focused, expanded) = control(&mut keys, handle);
    assert_eq!((value.as_str(), focused, expanded), ("2 spaces", true, Some(false)));
}

#[gpui_kit::test]
fn a_click_chooses_and_a_press_outside_closes(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_click_chooses_and_a_press_outside_closes") {
        return;
    }
    start(cx, 1);
    let handle = open(cx);
    let (id, ..) = control(cx, handle);
    let mut keys = VisualTestContext::from_window(handle, cx);
    click(&mut keys, handle, id);
    assert_eq!(control(&mut keys, handle).3, Some(true), "a click opens it");
    let eight = options(&mut keys, handle).into_iter().find(|(_, name, _)| name == "8 spaces").expect("8").0;
    click(&mut keys, handle, eight);
    assert_eq!(tab_width(&mut keys), 8, "a click on an option chooses it");
    assert_eq!(control(&mut keys, handle).3, Some(false), "and closes the menu");

    let (id, ..) = control(&mut keys, handle);
    let at = centre(&mut keys, handle, id);
    keys.simulate_click(at, Modifiers::none());
    assert_eq!(control(&mut keys, handle).3, Some(true));
    keys.simulate_click(at, Modifiers::none());
    assert_eq!(control(&mut keys, handle).3, Some(false), "a click on the open control closes it");
    keys.simulate_click(at, Modifiers::none());
    keys.simulate_click(gpui_kit::point(gpui_kit::px(2.0), gpui_kit::px(400.0)), Modifiers::none());
    assert_eq!(control(&mut keys, handle).3, Some(false), "a press outside closes it");
    assert_eq!(tab_width(&mut keys), 8, "and chooses nothing");
}

#[gpui_kit::test]
fn a_width_written_by_hand_shows_in_the_menu_and_stays_until_another_is_chosen(cx: &mut TestAppContext) {
    if isolate_in(
        module_path!(),
        "a_width_written_by_hand_shows_in_the_menu_and_stays_until_another_is_chosen",
    ) {
        return;
    }
    start(cx, 1);
    let handle = open(cx);
    cx.update(|cx| update_settings(cx, |s| s.tab_width = 3));
    draw(cx, handle);
    let (id, value, ..) = control(cx, handle);
    assert_eq!(value, "3 spaces", "the control shows the width as it is");
    let mut keys = VisualTestContext::from_window(handle, cx);
    click(&mut keys, handle, id);
    let listed = options(&mut keys, handle);
    assert_eq!(
        listed.iter().map(|(_, name, selected)| (name.as_str(), *selected)).collect::<Vec<_>>(),
        [("4 spaces", false), ("2 spaces", false), ("8 spaces", false), ("3 spaces", true)],
        "it is listed last and checked"
    );
    // Choosing it again is not a change; closing leaves it.
    let own = listed[3].0;
    click(&mut keys, handle, own);
    assert_eq!(tab_width(&mut keys), 3);
    let (id, ..) = control(&mut keys, handle);
    click(&mut keys, handle, id);
    let two = options(&mut keys, handle)[1].0;
    click(&mut keys, handle, two);
    assert_eq!(tab_width(&mut keys), 2, "another choice replaces it");
}

#[gpui_kit::test]
fn voiceover_can_press_the_menu(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "voiceover_can_press_the_menu") {
        return;
    }
    start(cx, 1);
    let handle = open(cx);
    let (target, ..) = control(cx, handle);
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
    draw(cx, handle);
    assert_eq!(control(cx, handle).3, Some(true), "pressing the control opens its menu");
}
