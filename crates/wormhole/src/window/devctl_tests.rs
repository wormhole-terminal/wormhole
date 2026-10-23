//! devctl's batch runner in a real window (a devctl build): waiting for idle and the wheel over a point. The
//! harness is `test_harness`.

use std::path::PathBuf;

use gpui_kit::TestAppContext;

use crate::terminal_view::Status;
use crate::window::test_harness::{draw, drive, isolate_in, read_cx, start, with_window};

/// A devctl batch with `wait-idle` stops there until the window has settled, holds back the next
/// batch meanwhile, and ends with its `sync` tokens after everything else it printed.
#[cfg(feature = "devctl")]
#[gpui_kit::test]
fn a_devctl_batch_waits_for_idle_and_ends_with_its_sync_tokens(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_devctl_batch_waits_for_idle_and_ends_with_its_sync_tokens") {
        return;
    }
    let window = start(cx, 2);
    let dir = PathBuf::from(std::env::var_os("WORMHOLE_DATA_DIR").expect("a scratch data dir")).join("ctl");
    std::fs::create_dir_all(&dir).expect("creates the control dir");
    let (cmd, out) = (dir.join("cmd"), dir.join("out.txt"));
    let poll = |cx: &mut TestAppContext| {
        let dir = dir.clone();
        drive(cx, &window, move |ws, window, cx| crate::devctl::poll_dir(dir, ws, window, cx));
    };
    let script = "sync first\nreduce-motion on\nhook 2 Notification Allow Bash?\nhook 9 Stop\nwait-idle 5000\n\
                  reduce-motion\nsync second\n";
    std::fs::write(&cmd, script).expect("writes the batch");
    poll(cx);
    assert!(!out.exists(), "the batch waits for the window to settle");
    assert!(cx.update(|cx| cx.reduce_motion()), "what came before the wait ran");
    std::fs::write(&cmd, "state\n").expect("writes the next batch");
    poll(cx);
    assert!(cmd.exists(), "the next batch is not taken while one waits");

    for _ in 0..100 {
        if out.exists() {
            break;
        }
        cx.executor().advance_clock(std::time::Duration::from_millis(50));
        cx.run_until_parked();
    }
    let printed = std::fs::read_to_string(&out).expect("the batch finished");
    assert_eq!(
        printed,
        "hook failed: no terminal 9 in this window (`state` lists them)\nidle\nreduce-motion on\nsync first\nsync second\n"
    );
    let status = cx.update(|cx| window.workspace.read_with(cx, |ws, cx| ws.sessions[1].status(cx)));
    assert_eq!(status.expect("the window is open"), Status::Attention, "the hook arrived");
    poll(cx);
    assert!(!cmd.exists(), "the next batch runs once the first is done");
}

/// `scroll` turns the wheel over a point the way a trackpad does, a frame between the steps, so
/// a long virtual list scrolls past the rows it had never measured, and back.
#[cfg(feature = "devctl")]
#[gpui_kit::test]
fn a_devctl_scroll_turns_the_wheel_over_a_point(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_devctl_scroll_turns_the_wheel_over_a_point") {
        return;
    }
    let window = start(cx, 1);
    let dir = PathBuf::from(std::env::var_os("WORMHOLE_DATA_DIR").expect("a scratch data dir")).join("ctl");
    std::fs::create_dir_all(&dir).expect("creates the control dir");
    let cmd = dir.join("cmd");
    let poll = |cx: &mut TestAppContext| {
        let dir = dir.clone();
        drive(cx, &window, move |ws, window, cx| crate::devctl::poll_dir(dir, ws, window, cx));
    };
    std::fs::write(&cmd, "task-seed 80 0\npanel-tab notes\n").expect("writes the batch");
    poll(cx);
    draw(cx, &window);
    let top = |cx: &mut TestAppContext| {
        let top = read_cx(cx, &window, |ws, cx| {
            ws.devctl_panel_list(cx).expect("the notes list").logical_scroll_top()
        });
        (top.item_ix, f32::from(top.offset_in_item))
    };
    assert_eq!(top(cx), (0, 0.0));

    // Over the right panel, which ends at the window's edge.
    let x = f32::from(with_window(cx, &window, |window, _| window.viewport_size().width)) - 180.0;
    std::fs::write(&cmd, format!("scroll {x} 400 1500\n")).expect("writes the batch");
    poll(cx);
    let (row, _) = top(cx);
    assert!(row > 0, "the list scrolled, past the rows it had measured");
    std::fs::write(&cmd, format!("scroll {x} 400 -5000\n")).expect("writes the batch");
    poll(cx);
    assert_eq!(top(cx), (0, 0.0), "and back to the top");
}
