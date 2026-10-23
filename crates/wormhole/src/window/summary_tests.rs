//! The window reads each terminal through its summary (`summaries`): output that changes no fact the
//! window shows doesn't wake it, a change that does shows at once. The harness is `test_harness`.

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{Entity, TestAppContext};
use wormhole_term::PtyEvent;

use crate::app_state::OpenWindow;
use crate::terminal_view::{Status, TerminalView};
use crate::window::reported_tests::terminal;
use crate::window::test_harness::{draw, hook, isolate_in, read_cx, start};

/// Output as the PTY's reader delivers it, to terminal `ix`.
fn output(cx: &mut TestAppContext, terminal: &Entity<TerminalView>) {
    cx.update(|cx| terminal.update(cx, |t, cx| t.event_for_test(PtyEvent::Output, cx)));
    cx.run_until_parked();
}

/// How many times the workspace has been notified from now on.
fn count_wakes(cx: &mut TestAppContext, window: &OpenWindow) -> Rc<Cell<usize>> {
    let wakes = Rc::new(Cell::new(0));
    let counted = wakes.clone();
    cx.update(|cx| {
        let workspace = window.workspace.upgrade().expect("the window is open");
        cx.observe(&workspace, move |_, _| counted.set(counted.get() + 1)).detach();
    });
    wakes
}

#[gpui_kit::test]
fn output_that_changes_no_fact_does_not_wake_the_window_and_a_change_does_at_once(cx: &mut TestAppContext) {
    if isolate_in(
        module_path!(),
        "output_that_changes_no_fact_does_not_wake_the_window_and_a_change_does_at_once",
    ) {
        return;
    }
    let window = start(cx, 2);
    draw(cx, &window);
    // The one on screen: its output draws the terminal's own view for every frame.
    let shown = terminal(cx, &window, 0);
    // The first output turns the terminal busy; the window hears of that.
    output(cx, &shown);
    let state = |cx: &mut TestAppContext| read_cx(cx, &window, |ws, cx| ws.row_infos(cx)[0].state);
    let busy = state(cx);

    let wakes = count_wakes(cx, &window);
    for _ in 0..20 {
        output(cx, &shown);
    }
    assert_eq!(wakes.get(), 0, "twenty frames of output, no fact changed: the window is left alone");
    assert_eq!(state(cx), busy);

    // A fact that changes shows at once, in the row and by waking the window.
    hook(cx, &window, 1, "PermissionRequest:Bash", Some("Approve cargo test?"));
    assert!(wakes.get() >= 1, "an agent asking for you wakes the window");
    let asked = read_cx(cx, &window, |ws, _| ws.sessions[0].pane.summary.status);
    assert_eq!(asked, Status::Attention);
    assert_ne!(state(cx), busy, "and its row says so");
}

/// A frame of the window reads the terminals it doesn't show only through their summaries: drawn
/// while two of them are being updated (which makes a read of one panic), it still draws. What a
/// frame reads of a terminal is what it depends on, so a terminal's output must not be in it.
#[gpui_kit::test]
fn a_frame_reads_no_terminal_it_does_not_show(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_frame_reads_no_terminal_it_does_not_show") {
        return;
    }
    let window = start(cx, 3);
    // The rows, the bar and the tab bar are the parts that show every terminal.
    cx.update(|cx| crate::settings::update_settings(cx, |settings| settings.status_bar = true));
    hook(cx, &window, 2, "PermissionRequest:Bash", Some("Approve cargo test?"));
    hook(cx, &window, 3, "UserPromptSubmit", None);
    draw(cx, &window);
    draw(cx, &window);
    let (second, third) = (terminal(cx, &window, 1), terminal(cx, &window, 2));
    cx.update(|cx| {
        window
            .handle
            .update(cx, |_, window, cx| {
                second.update(cx, |_, cx| third.update(cx, |_, cx| window.render_frame(cx)))
            })
            .expect("the window is open")
    });
}
