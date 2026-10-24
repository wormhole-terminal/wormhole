//! A terminal, alone or in a split, is replayed by frames it did not cause, also while the accessibility
//! tree is built: GPUI replays a reused view's nodes too, so the pane bar's clipboard buttons stay in the
//! tree on those frames (`window::cached`).

use gpui_kit::TestAppContext;
use gpui_kit::accesskit::Role;
use wormhole_core::storage::store::{SessionRecord, SplitRecord, WindowRecord};
use wormhole_core::view::panes::{Axis, TabPanes};
use wormhole_term::PtyEvent;

use crate::app_state::OpenWindow;
use crate::window::reported_tests::terminal;
use crate::window::test_harness::{
    activate, draw_replaying as draw, isolate_in, rebuild, simulate_tree, start, start_with, tree,
};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// How many times terminal `ix` was rendered.
fn renders(cx: &mut TestAppContext, window: &OpenWindow, ix: usize) -> usize {
    let terminal = terminal(cx, window, ix);
    cx.update(|cx| terminal.read(cx).renders_for_test())
}

/// Output as the PTY's reader delivers it, to terminal `ix`.
fn output(cx: &mut TestAppContext, window: &OpenWindow, ix: usize) {
    let terminal = terminal(cx, window, ix);
    cx.update(|cx| terminal.update(cx, |t, cx| t.event_for_test(PtyEvent::Output, cx)));
    cx.run_until_parked();
}

/// A program in terminal `ix` asks to read the clipboard: its pane bar asks, with three buttons.
fn ask(cx: &mut TestAppContext, window: &OpenWindow, ix: usize) {
    let terminal = terminal(cx, window, ix);
    cx.update(|cx| terminal.update(cx, |t, cx| t.ask_clipboard_for_test(cx)));
    cx.run_until_parked();
}

fn asking(cx: &mut TestAppContext, window: &OpenWindow, ix: usize) -> bool {
    let terminal = terminal(cx, window, ix);
    cx.update(|cx| terminal.read(cx).asking_for_test())
}

/// Frames that nothing asked for, with the tree off and then on: the terminals are replayed both times.
fn frames_leave_it_alone_with_or_without_the_tree(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    shown: &[usize],
) {
    for on in [false, true] {
        simulate_tree(cx, window, on);
        // A view prepainted without the tree has none of its nodes to replay: it is drawn once more.
        for _ in 0..3 {
            draw(cx, window);
        }
        let before: Vec<usize> = shown.iter().map(|&ix| renders(cx, window, ix)).collect();
        for _ in 0..4 {
            draw(cx, window);
        }
        let after: Vec<usize> = shown.iter().map(|&ix| renders(cx, window, ix)).collect();
        assert_eq!(after, before, "frames nothing asked for replay the terminals (tree built: {on})");
    }
}

/// With the tree built and a question out in the last of the `shown` terminals: the tree of a window whose
/// terminals are replayed is the tree a full rebuild gives (the same nodes, links, labels and focus), the
/// pane bar's buttons are in it, and pressing one on a replayed frame does what it says.
fn replayed_frames_give_the_tree_of_a_rebuild(cx: &mut TestAppContext, window: &OpenWindow, shown: &[usize]) {
    let asker = *shown.last().expect("a terminal");
    simulate_tree(cx, window, true);
    for _ in 0..3 {
        draw(cx, window);
    }
    ask(cx, window, asker);
    assert!(asking(cx, window, asker));
    rebuild(cx, window);
    let reference = tree(cx, window);
    assert!(reference.whole(), "the rebuilt tree: {:?}", reference.problems);
    for label in ["Allow once", "Always", "Deny"] {
        let found = reference.labelled(Role::Button, label);
        assert_eq!(found.len(), 1, "the pane bar's {label:?} button is in the tree: {:?}", reference.nodes);
    }

    // Frames nothing asked for: the terminals are replayed, with their nodes.
    let before: Vec<usize> = shown.iter().map(|&ix| renders(cx, window, ix)).collect();
    for round in 0..4 {
        draw(cx, window);
        assert_eq!(tree(cx, window), reference, "replayed frame {round}");
    }
    let after: Vec<usize> = shown.iter().map(|&ix| renders(cx, window, ix)).collect();
    assert_eq!(after, before, "the terminals were replayed, not drawn");

    // Frames a terminal's output asks for: it is drawn again, the rest is replayed.
    for ix in shown {
        output(cx, window, *ix);
        draw(cx, window);
        assert_eq!(tree(cx, window), reference, "a frame of terminal {ix}'s output");
    }

    // The button answers on a replayed frame.
    let once = reference.one(Role::Button, "Allow once");
    draw(cx, window);
    activate(cx, window, once);
    assert!(!asking(cx, window, asker), "the question was answered");
    draw(cx, window);
    let answered = tree(cx, window);
    assert!(answered.whole(), "{:?}", answered.problems);
    assert!(answered.labelled(Role::Button, "Allow once").is_empty(), "and the bar is gone");
    assert_eq!(answered.nodes.len() + 3, reference.nodes.len(), "its three buttons and nothing else");
}

#[gpui_kit::test]
fn the_terminal_is_replayed_while_the_accessibility_tree_is_built(cx: &mut TestAppContext) {
    if isolate("the_terminal_is_replayed_while_the_accessibility_tree_is_built") {
        return;
    }
    let window = start(cx, 2);
    frames_leave_it_alone_with_or_without_the_tree(cx, &window, &[0]);
    replayed_frames_give_the_tree_of_a_rebuild(cx, &window, &[0]);
}

#[gpui_kit::test]
fn the_panes_of_a_split_are_replayed_while_the_accessibility_tree_is_built(cx: &mut TestAppContext) {
    if isolate("the_panes_of_a_split_are_replayed_while_the_accessibility_tree_is_built") {
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
    frames_leave_it_alone_with_or_without_the_tree(cx, &window, &[0, 1]);
    replayed_frames_give_the_tree_of_a_rebuild(cx, &window, &[0, 1]);
}
