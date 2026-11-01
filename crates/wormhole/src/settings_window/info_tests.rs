//! The (i) after a setting's label: it is a tab stop that ↩ opens and Esc closes, a press outside its
//! card or on itself closes it, one is open at a time, and VoiceOver can press it. Its cost is held by
//! `frame_tests`.
//!
//! The window runs in a child process with a scratch data dir (`isolate_in`), like the whole-window tests.

use gpui_kit::accesskit::{Action, ActionRequest, NodeId, Role, TreeId};
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{
    AnyWindowHandle, KeyUpEvent, Keystroke, Modifiers, TestAppContext, VisualTestContext, point, px,
};

use super::{Section, current_window, open_settings};
use crate::window::test_harness::{isolate_in, start};

const NAME: &str = "More information";

fn open(cx: &mut TestAppContext) -> AnyWindowHandle {
    cx.update(|cx| open_settings(Some(Section::Terminal), cx));
    cx.run_until_parked();
    let handle = cx.update(|cx| current_window(cx)).expect("the settings window opened").0;
    cx.update(|cx| handle.update(cx, |_, window, _| window.simulate_a11y_active(true))).expect("open");
    draw(cx, handle);
    handle
}

fn draw(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    cx.update(|cx| handle.update(cx, |_, window, cx| window.render_frame(cx))).expect("open");
}

fn shown(cx: &mut TestAppContext, handle: AnyWindowHandle) -> bool {
    cx.update(|cx| {
        handle.update(cx, |_, window, cx| crate::style::is_open_in(window.window_handle().window_id(), cx))
    })
    .expect("open")
}

/// ↩ or Space as the keyboard sends it: GPUI clicks a focused element when the key comes back up, which
/// `simulate_keystrokes` (key down only) never sends.
fn activate(keys: &mut VisualTestContext, key: &str) {
    keys.simulate_keystrokes(key);
    let keystroke = Keystroke::parse(key).expect("a key");
    keys.simulate_event(KeyUpEvent { keystroke });
}

/// The ids of the (i)s in the last frame's tree, in the tree's order, and the one that has focus.
fn infos(cx: &mut TestAppContext, handle: AnyWindowHandle) -> (Vec<NodeId>, NodeId) {
    let update = cx
        .update(|cx| handle.update(cx, |_, window, _| window.last_a11y_tree_update()))
        .expect("open")
        .expect("a frame built the tree");
    let ids = update
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.label() == Some(NAME))
        .map(|(id, _)| *id)
        .collect();
    (ids, update.focus)
}

/// Where the (i) `id` is drawn.
fn centre(cx: &mut TestAppContext, handle: AnyWindowHandle, id: NodeId) -> gpui_kit::Point<gpui_kit::Pixels> {
    let update = cx
        .update(|cx| handle.update(cx, |_, window, _| window.last_a11y_tree_update()))
        .expect("open")
        .expect("a tree");
    let (_, node) = update.nodes.iter().find(|(node, _)| *node == id).expect("the node");
    let bounds = node.bounds().expect("a drawn node has bounds");
    // The tree's bounds are in device pixels.
    let scale =
        f64::from(cx.update(|cx| handle.update(cx, |_, window, _| window.scale_factor())).expect("open"));
    point(
        px((((bounds.x0 + bounds.x1) / 2.0) / scale) as f32),
        px((((bounds.y0 + bounds.y1) / 2.0) / scale) as f32),
    )
}

#[gpui_kit::test]
fn the_info_opens_by_keyboard_and_pointer_and_closes_again(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_info_opens_by_keyboard_and_pointer_and_closes_again") {
        return;
    }
    start(cx, 1);
    let handle = open(cx);
    let (ids, _) = infos(cx, handle);
    assert!(ids.len() >= 2, "the Terminal page has rows with an (i): {ids:?}");
    assert!(!shown(cx, handle));

    // Tab reaches an (i), and ↩ opens it.
    let mut keys = VisualTestContext::from_window(handle, cx);
    let mut reached = false;
    for _ in 0..80 {
        keys.simulate_keystrokes("tab");
        let (ids, focus) = infos(&mut keys, handle);
        if ids.contains(&focus) {
            reached = true;
            break;
        }
    }
    assert!(reached, "Tab reaches an (i)");
    activate(&mut keys, "enter");
    assert!(shown(&mut keys, handle), "↩ opens it");
    let (_, focus) = infos(&mut keys, handle);
    keys.simulate_keystrokes("escape");
    assert!(!shown(&mut keys, handle), "Esc closes it");
    assert_eq!(infos(&mut keys, handle).1, focus, "and the keyboard stays on the (i)");
    activate(&mut keys, "space");
    assert!(shown(&mut keys, handle), "Space opens it too");

    // A press on the (i) itself closes it, and does not open it again.
    let at = centre(&mut keys, handle, focus);
    keys.simulate_click(at, Modifiers::none());
    assert!(!shown(&mut keys, handle), "a click on the open (i) closes it");
    keys.simulate_click(at, Modifiers::none());
    assert!(shown(&mut keys, handle), "a click on a closed one opens it");

    // Another (i) takes over, and a press outside closes.
    let (ids, _) = infos(&mut keys, handle);
    let other = *ids.iter().find(|id| **id != focus).expect("another (i)");
    let there = centre(&mut keys, handle, other);
    keys.simulate_click(there, Modifiers::none());
    assert!(shown(&mut keys, handle), "a second (i) opens");
    keys.simulate_click(point(px(2.0), px(400.0)), Modifiers::none());
    assert!(!shown(&mut keys, handle), "a press outside closes it");
}

#[gpui_kit::test]
fn voiceover_can_press_the_info(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "voiceover_can_press_the_info") {
        return;
    }
    start(cx, 1);
    let handle = open(cx);
    let (ids, _) = infos(cx, handle);
    let target = *ids.first().expect("an (i)");
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
    assert!(shown(cx, handle), "pressing the (i) opens its card");
}
