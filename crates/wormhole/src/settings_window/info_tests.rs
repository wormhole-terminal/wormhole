//! The (i) after a setting's label: it is a tab stop that ↩ opens and Esc closes, a press outside its
//! card or on itself closes it, so does Tab away from it, one is open at a time (the second press opens
//! the second, not only closes the first), a search's mark is in the card, and VoiceOver can press it.
//! Its cost is held by `frame_tests`.
//!
//! The window runs in a child process with a scratch data dir (`isolate_in`), like the whole-window tests.

use gpui_kit::accesskit::{Action, ActionRequest, NodeId, Role, TreeId};
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{
    AnyWindowHandle, ElementId, KeyUpEvent, Keystroke, Modifiers, TestAppContext, VisualTestContext, point,
    px,
};
use wormhole_core::config::settings_index::Query;

use super::{Section, current_window, open_settings};
use crate::window::test_harness::{isolate_in, start};

const NAME: &str = "More information";

pub(super) fn open(cx: &mut TestAppContext) -> AnyWindowHandle {
    cx.update(|cx| open_settings(Some(Section::Terminal), cx));
    cx.run_until_parked();
    let handle = cx.update(|cx| current_window(cx)).expect("the settings window opened").0;
    cx.update(|cx| handle.update(cx, |_, window, _| window.simulate_a11y_active(true))).expect("open");
    draw(cx, handle);
    handle
}

pub(super) fn draw(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    cx.update(|cx| handle.update(cx, |_, window, cx| window.render_frame(cx))).expect("open");
}

fn shown(cx: &mut TestAppContext, handle: AnyWindowHandle) -> bool {
    cx.update(|cx| {
        handle.update(cx, |_, window, cx| crate::style::is_open_in(window.window_handle().window_id(), cx))
    })
    .expect("open")
}

/// The row whose card is open.
fn open_row(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Option<ElementId> {
    cx.update(|cx| {
        handle.update(cx, |_, window, cx| crate::style::open_row_in(window.window_handle().window_id(), cx))
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
pub(super) fn centre(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    id: NodeId,
) -> gpui_kit::Point<gpui_kit::Pixels> {
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

    // Another (i) takes over, and a press outside closes. An open card hangs under its (i) and covers
    // what lies below it, so the card is opened on the lower of two (i)s and the press is on the upper,
    // which it does not cover: that press must open the upper one's card, not only close the lower's.
    keys.simulate_click(at, Modifiers::none());
    assert!(!shown(&mut keys, handle), "closed again");
    let (ids, _) = infos(&mut keys, handle);
    let mut by_height: Vec<_> = ids.iter().map(|id| (centre(&mut keys, handle, *id).y, *id)).collect();
    by_height.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("heights compare"));
    let (upper, lower) = (by_height[0].1, by_height[1].1);
    let down = centre(&mut keys, handle, lower);
    keys.simulate_click(down, Modifiers::none());
    let first = open_row(&mut keys, handle).expect("the lower (i)'s card is open");
    let up = centre(&mut keys, handle, upper);
    keys.simulate_click(up, Modifiers::none());
    let second = open_row(&mut keys, handle).expect("a card is open after the press on the upper (i)");
    assert_ne!(second, first, "a press on another (i) opens that one's card");
    keys.simulate_click(point(px(2.0), px(400.0)), Modifiers::none());
    assert!(!shown(&mut keys, handle), "a press outside closes it");
}

#[gpui_kit::test]
fn tabbing_away_from_the_info_closes_its_card(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "tabbing_away_from_the_info_closes_its_card") {
        return;
    }
    start(cx, 1);
    let handle = open(cx);
    // GPUI tells a window's focus listeners about a move only while the window is the active one.
    cx.update(|cx| handle.update(cx, |_, window, _| window.activate_window())).expect("open");
    draw(cx, handle);
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
    keys.simulate_keystrokes("tab");
    // Focus listeners run as the frame that follows the move is drawn.
    draw(&mut keys, handle);
    assert!(!shown(&mut keys, handle), "Tab away closes its card, which Esc could no longer reach");
}

#[gpui_kit::test]
fn a_card_opened_from_a_search_marks_what_was_found_in_it(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_card_opened_from_a_search_marks_what_was_found_in_it") {
        return;
    }
    start(cx, 1);
    let handle = open(cx);
    // A word of the descriptions that no label has, so the (i)s of the results hold the found part.
    let word = "scrollback";
    let page = cx.update(|cx| current_window(cx)).expect("the window").1;
    cx.update(|cx| {
        handle
            .update(cx, |_, window, cx| {
                page.update(cx, |page, cx| {
                    page.search.update(cx, |search, cx| search.set_value(word.to_string(), window, cx));
                    page.show_results = !Query::new(word).is_empty();
                    cx.notify();
                })
            })
            .expect("open")
    });
    cx.run_until_parked();
    draw(cx, handle);
    let (ids, _) = infos(cx, handle);
    // The results are live rows; press each (i) until a card holds the found word.
    let mut marked = None;
    for id in ids {
        let at = centre(cx, handle, id);
        let mut keys = VisualTestContext::from_window(handle, cx);
        keys.simulate_click(at, Modifiers::none());
        if shown(&mut keys, handle)
            && let Some((words, Some(found))) = crate::style::LAST_CARD.with(|c| c.borrow().clone())
        {
            marked = Some(words[found].to_lowercase());
            break;
        }
        keys.simulate_click(point(px(2.0), px(400.0)), Modifiers::none());
    }
    assert_eq!(marked.as_deref(), Some(word), "the card marks the part of its words the search found");
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
