//! A terminal, alone or in a split, is replayed by frames it did not cause, except while the accessibility
//! tree is built: GPUI replays none of a reused view's nodes, so the pane bar's clipboard buttons would drop
//! out of the tree on those frames (`window::cached`).

use gpui_kit::TestAppContext;
use wormhole_core::storage::store::{SessionRecord, SplitRecord, WindowRecord};
use wormhole_core::view::panes::{Axis, TabPanes};

use crate::app_state::OpenWindow;
use crate::window::reported_tests::terminal;
use crate::window::test_harness::{draw_replaying as draw, isolate_in, start, start_with};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// How many times terminal `ix` was rendered.
fn renders(cx: &mut TestAppContext, window: &OpenWindow, ix: usize) -> usize {
    let terminal = terminal(cx, window, ix);
    cx.update(|cx| terminal.read(cx).renders_for_test())
}

/// Frames that nothing asked for, with the tree off and then on: the terminal is replayed, then drawn afresh.
fn frames_leave_it_alone_unless_the_tree_is_built(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    shown: &[usize],
) {
    for _ in 0..3 {
        draw(cx, window);
    }
    let before: Vec<usize> = shown.iter().map(|&ix| renders(cx, window, ix)).collect();
    for _ in 0..4 {
        draw(cx, window);
    }
    let after: Vec<usize> = shown.iter().map(|&ix| renders(cx, window, ix)).collect();
    assert_eq!(after, before, "frames nothing asked for replay the terminals");
    let tree = crate::platform::a11y::force_tree();
    // GPUI refreshes the window when the tree turns on.
    draw(cx, window);
    let before: Vec<usize> = shown.iter().map(|&ix| renders(cx, window, ix)).collect();
    for _ in 0..4 {
        draw(cx, window);
    }
    for (&ix, was) in shown.iter().zip(before) {
        let now = renders(cx, window, ix);
        assert!(
            now >= was + 4,
            "terminal {ix} is rendered in each of four frames with the tree built ({was} to {now})"
        );
    }
    drop(tree);
}

#[gpui_kit::test]
fn the_terminal_is_embedded_plainly_while_the_accessibility_tree_is_built(cx: &mut TestAppContext) {
    if isolate("the_terminal_is_embedded_plainly_while_the_accessibility_tree_is_built") {
        return;
    }
    let window = start(cx, 2);
    frames_leave_it_alone_unless_the_tree_is_built(cx, &window, &[0]);
}

#[gpui_kit::test]
fn the_panes_of_a_split_are_embedded_plainly_while_the_accessibility_tree_is_built(cx: &mut TestAppContext) {
    if isolate("the_panes_of_a_split_are_embedded_plainly_while_the_accessibility_tree_is_built") {
        return;
    }
    let mut layout = TabPanes::single(1);
    assert!(layout.split(Axis::Row, 2) && layout.focus(1));
    let member = |id| SessionRecord { split_id: Some(100), ..SessionRecord::new(id, None, None) };
    let record = WindowRecord {
        id: 1,
        sessions: vec![member(1), member(2)],
        splits: vec![SplitRecord { id: 100, layout: Some(layout.to_json()) }],
        ..Default::default()
    };
    let window = start_with(cx, record);
    frames_leave_it_alone_unless_the_tree_is_built(cx, &window, &[0, 1]);
}
