//! The window reads each terminal through its summary (`summaries`): output that changes no fact the
//! window shows doesn't wake it, a change that does shows at once. The harness is `test_harness`.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{Entity, TestAppContext};
use wormhole_term::PtyEvent;

use crate::app_state::OpenWindow;
use crate::terminal_view::{Status, TerminalView};
use crate::window::reported_tests::terminal;
use crate::window::test_harness::{draw, drive, hook, isolate_in, read_cx, start};

/// Output as the PTY's reader delivers it, to terminal `ix`.
fn output(cx: &mut TestAppContext, terminal: &Entity<TerminalView>) {
    cx.update(|cx| terminal.update(cx, |t, cx| t.event_for_test(PtyEvent::Output, cx)));
    cx.run_until_parked();
}

/// How many times the workspace, and the sidebar, have been notified from now on.
fn count_wakes(cx: &mut TestAppContext, window: &OpenWindow) -> (Rc<Cell<usize>>, Rc<Cell<usize>>) {
    let (window_wakes, sidebar_wakes) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
    let (counted, counted_sidebar) = (window_wakes.clone(), sidebar_wakes.clone());
    cx.update(|cx| {
        let workspace = window.workspace.upgrade().expect("the window is open");
        let sidebar = workspace.read(cx).sidebar.clone();
        cx.observe(&workspace, move |_, _| counted.set(counted.get() + 1)).detach();
        cx.observe(&sidebar, move |_, _| counted_sidebar.set(counted_sidebar.get() + 1)).detach();
    });
    (window_wakes, sidebar_wakes)
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

    let (wakes, sidebar_wakes) = count_wakes(cx, &window);
    for _ in 0..20 {
        output(cx, &shown);
    }
    assert_eq!(wakes.get(), 0, "twenty frames of output, no fact changed: the window is left alone");
    assert_eq!(sidebar_wakes.get(), 0, "and the sidebar");
    assert_eq!(state(cx), busy);

    // A fact that changes shows at once, in the row and by waking the window.
    hook(cx, &window, 1, "PermissionRequest:Bash", Some("Approve cargo test?"));
    assert!(wakes.get() >= 1, "an agent asking for you wakes the window");
    assert!(sidebar_wakes.get() >= 1, "and tells the sidebar");
    let told = crate::window::test_harness::sidebar_of(cx, &window, |sidebar| sidebar.input().rows[0].state);
    assert_eq!(told, state(cx), "what it was told is the row now");
    let asked = read_cx(cx, &window, |ws, _| ws.sessions[0].pane.summary.status);
    assert_eq!(asked, Status::Attention);
    assert_ne!(state(cx), busy, "and its row says so");
}

/// The times on the rows move on by themselves: the tick draws the sidebar when one would read
/// differently, and draws nothing while none does. (The window around it is not drawn for that, nor for a
/// hover or a rename in the list: only for a change of what it reads of the list, `regions_tests`.)
#[gpui_kit::test]
fn a_time_that_reads_differently_draws_the_sidebar_on_the_tick(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_time_that_reads_differently_draws_the_sidebar_on_the_tick") {
        return;
    }
    let window = start(cx, 2);
    // The first tick finds the terminals' folders and looks for agents (a hook's is not one the
    // process tree has): settled before the hook, so the ones that follow find nothing new.
    drive(cx, &window, |ws, window, cx| ws.tick(window, cx));
    hook(cx, &window, 1, "UserPromptSubmit", None);
    let working = terminal(cx, &window, 0);
    // Just short of the minute, which a few moments later is reached.
    cx.update(|cx| {
        working.update(cx, |t, cx| {
            t.backdate_status_for_test(Duration::from_millis(59_900));
            cx.notify();
        })
    });
    cx.run_until_parked();
    draw(cx, &window);
    draw(cx, &window);
    let times = |cx: &mut TestAppContext| {
        read_cx(cx, &window, |ws, cx| {
            crate::sidebar::row_times(&ws.row_infos(cx), ws.profile(), cx).into_iter().next().flatten()
        })
    };
    let tick = |cx: &mut TestAppContext| {
        // Not one that looks for agents again.
        drive(cx, &window, |ws, window, cx| {
            ws.set_ticks(1);
            ws.tick(window, cx)
        });
    };
    assert_eq!(times(cx), None, "not yet a minute");
    let (wakes, sidebar_wakes) = count_wakes(cx, &window);
    tick(cx);
    assert_eq!((wakes.get(), sidebar_wakes.get()), (0, 0), "a tick with nothing new tells nobody");

    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(times(cx), Some("1m".to_string()), "a minute now");
    tick(cx);
    assert!(sidebar_wakes.get() >= 1, "the sidebar is told the time reads differently");
    draw(cx, &window);
    let after = sidebar_wakes.get();
    tick(cx);
    assert_eq!(sidebar_wakes.get(), after, "and once it has drawn it, not again");
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
