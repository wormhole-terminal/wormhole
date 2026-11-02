//! A terminal whose session host runs but could not be attached to keeps the host's id and says why;
//! only a host that is gone gives up its id. And such a host is not adopted again as a stranger at
//! the next launch, nor given a second row when a terminal already names it. (Real window, GPUI's
//! test platform; see `test_harness`.)

use gpui_kit::TestAppContext;
use wormhole_core::ipc::hosts::{AttachFailure, HostMeta, LaunchHosts, Unavailable, UnavailableKind};

use super::Workspace;
use super::reported_tests::terminal;
use super::test_harness::{drive, isolate_in, read, read_cx, start};
use crate::app_state::{OpenWindow, app_mut};
use crate::terminal_view::Hosting;

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

fn refused() -> AttachFailure {
    AttachFailure::Unavailable(Unavailable {
        kind: UnavailableKind::Refused,
        detail: "the app's signature".into(),
    })
}

fn fail(cx: &mut TestAppContext, window: &OpenWindow, id: &str, failure: AttachFailure) {
    let terminal = terminal(cx, window, 0);
    cx.update(|cx| terminal.update(cx, |t, cx| t.attach_failed_for_test(id, failure, cx)));
    cx.run_until_parked();
}

fn meta(id: &str) -> HostMeta {
    HostMeta {
        id: id.into(),
        pid: 1,
        child_pid: None,
        started_at: 1,
        host_version: "1.0.0".into(),
        proto_min: 1,
        proto_max: 1,
        socket: format!("/h/{id}.sock").into(),
        data_dir: "/d".into(),
        proc_start: None,
    }
}

#[gpui_kit::test]
fn a_host_that_runs_but_refused_the_app_keeps_its_id(cx: &mut TestAppContext) {
    if isolate("a_host_that_runs_but_refused_the_app_keeps_its_id") {
        return;
    }
    let window = start(cx, 1);
    fail(cx, &window, "aaaaaaaaaaaa", refused());
    let t = terminal(cx, &window, 0);
    t.read_with(cx, |t, _| {
        assert_eq!(t.host_id(), Some("aaaaaaaaaaaa"), "the row still names its host");
        assert_eq!(t.host_unavailable().map(|u| u.kind), Some(UnavailableKind::Refused));
        assert_eq!(t.hosting_for_test(), Hosting::Unavailable);
        assert!(!t.has_pty_for_test(), "no shell replaces it: the program is in the host");
    });
}

#[gpui_kit::test]
fn only_a_host_that_is_gone_gives_up_its_id(cx: &mut TestAppContext) {
    if isolate("only_a_host_that_is_gone_gives_up_its_id") {
        return;
    }
    let window = start(cx, 1);
    fail(cx, &window, "bbbbbbbbbbbb", AttachFailure::Gone);
    terminal(cx, &window, 0).read_with(cx, |t, _| {
        assert_eq!(t.host_id(), None, "gone: the saved text comes back as a new shell");
        assert!(t.host_unavailable().is_none());
        assert!(t.has_pty_for_test());
    });
}

/// The next launch must not make a ghost: a host a terminal names is not adopted as a second row,
/// one that does not take the app is left alone, and one that does is adopted once.
#[gpui_kit::test]
fn an_unreachable_host_does_not_get_a_new_row_at_the_next_launch(cx: &mut TestAppContext) {
    if isolate("an_unreachable_host_does_not_get_a_new_row_at_the_next_launch") {
        return;
    }
    let window = start(cx, 1);
    fail(cx, &window, "aaaaaaaaaaaa", refused());
    let rows = |cx: &mut TestAppContext| read(cx, &window, |ws: &Workspace| ws.sessions.len());
    assert_eq!(rows(cx), 1);
    // The launch finds three hosts nobody's saved row names: the one a terminal here names (it
    // could not attach), one that refuses the app, and one that is fine.
    let found = vec![meta("aaaaaaaaaaaa"), meta("cccccccccccc"), meta("dddddddddddd")];
    cx.update(|cx| {
        app_mut(cx).launch_hosts = LaunchHosts::plan([], found.clone());
        super::hosting::adopt_orphans_with(cx, |m| m.id == "dddddddddddd");
    });
    cx.run_until_parked();
    assert_eq!(rows(cx), 2, "one row more, for the host that takes the app");
    // And once more, with a host that now takes the app but is named by this window's terminal.
    cx.update(|cx| {
        app_mut(cx).launch_hosts = LaunchHosts::plan([], vec![meta("aaaaaaaaaaaa")]);
        super::hosting::adopt_orphans_with(cx, |_| true);
    });
    cx.run_until_parked();
    assert_eq!(rows(cx), 2, "a host a terminal names never gets a second row");
}

/// Closing it asks first (the host runs on unseen otherwise); Cancel keeps it, Leave It Running closes only
/// the terminal.
#[gpui_kit::test]
fn closing_a_terminal_whose_host_cannot_be_shown_asks_first(cx: &mut TestAppContext) {
    if isolate("closing_a_terminal_whose_host_cannot_be_shown_asks_first") {
        return;
    }
    let window = start(cx, 2);
    fail(cx, &window, "aaaaaaaaaaaa", refused());
    // The close button's dot says a question would come.
    read_cx(cx, &window, |ws: &Workspace, cx| {
        assert!(ws.sessions[0].needs_close_confirmation(cx), "it needs a confirmation");
    });
    drive(cx, &window, |ws, window, cx| ws.request_close_session(0, window, cx));
    assert!(cx.has_pending_prompt(), "a sheet");
    read(cx, &window, |ws: &Workspace| {
        assert_eq!(ws.sessions.len(), 2, "nothing closed before the answer");
        assert!(ws.closing_session_prompt);
    });
    cx.simulate_prompt_answer("Cancel");
    cx.run_until_parked();
    read(cx, &window, |ws: &Workspace| {
        assert_eq!(ws.sessions.len(), 2, "cancelled");
        assert!(!ws.closing_session_prompt);
    });
    drive(cx, &window, |ws, window, cx| ws.request_close_session(0, window, cx));
    cx.simulate_prompt_answer("Leave It Running");
    cx.run_until_parked();
    read(cx, &window, |ws: &Workspace| assert_eq!(ws.sessions.len(), 1, "the terminal is closed"));
}

/// Ending the host failed (there is no such host): the terminal stays and says why. When it worked, the
/// window closes that session by its id, shown or not.
#[gpui_kit::test]
fn a_host_that_could_not_be_ended_keeps_its_terminal(cx: &mut TestAppContext) {
    if isolate("a_host_that_could_not_be_ended_keeps_its_terminal") {
        return;
    }
    let window = start(cx, 2);
    fail(cx, &window, "aaaaaaaaaaaa", refused());
    let t = terminal(cx, &window, 0);
    cx.update(|cx| t.update(cx, |t, cx| t.end_host_then_close(false, cx)));
    cx.run_until_parked();
    read(cx, &window, |ws: &Workspace| assert_eq!(ws.sessions.len(), 2, "the row stays"));
    t.read_with(cx, |t, _| {
        assert!(t.host_end_failure().is_some(), "and says why");
        assert!(t.host_unavailable().is_some(), "it still keeps the host");
    });
    // Ended for real: the event closes it, though it is not the shown one.
    drive(cx, &window, |ws, window, cx| {
        ws.on_terminal_event(1, crate::terminal_view::TerminalEvent::HostEnded, window, cx)
    });
    read(cx, &window, |ws: &Workspace| assert_eq!(ws.sessions.len(), 1, "closed by its id"));
}

/// A failure that may pass is tried once more before the bar says anything; a second one shows it.
#[gpui_kit::test]
fn a_failure_that_may_pass_is_tried_again_once_before_the_bar(cx: &mut TestAppContext) {
    if isolate("a_failure_that_may_pass_is_tried_again_once_before_the_bar") {
        return;
    }
    let window = start(cx, 1);
    let timed_out =
        || AttachFailure::Unavailable(Unavailable { kind: UnavailableKind::TimedOut, detail: "slow".into() });
    fail(cx, &window, "aaaaaaaaaaaa", timed_out());
    terminal(cx, &window, 0).read_with(cx, |t, _| {
        assert!(t.host_unavailable().is_none(), "no bar yet: it is trying again");
        assert_eq!(t.hosting_for_test(), Hosting::Connecting);
    });
    // The retry fails the same way: now the bar.
    let t = terminal(cx, &window, 0);
    cx.update(|cx| t.update(cx, |t, cx| t.attach_failed_for_test("aaaaaaaaaaaa", timed_out(), cx)));
    cx.run_until_parked();
    t.read_with(cx, |t, _| {
        assert_eq!(t.host_unavailable().map(|u| u.kind), Some(UnavailableKind::TimedOut));
    });
}

/// A terminal restored from text (its host is gone) starts a new host, which seeds the text; one that
/// may not be hosted, or has no host program, stays in the app.
#[gpui_kit::test]
fn a_terminal_restored_after_its_host_is_gone_runs_in_a_new_host(cx: &mut TestAppContext) {
    use wormhole_core::storage::store::SessionRecord;
    use wormhole_core::terminal::SpawnOptions;

    use crate::terminal_view::Start;
    if isolate("a_terminal_restored_after_its_host_is_gone_runs_in_a_new_host") {
        return;
    }
    let window = start(cx, 1);
    let mut record = SessionRecord::new(7, None, None);
    record.scrollback = Some("what the shell printed last time\n".into());
    // Its row names a host that is not there any more: no attach, but a new host with the text.
    record.host_id = Some("aaaaaaaaaaaa".into());
    let options = SpawnOptions { restore: record.scrollback.clone(), ..SpawnOptions::default() };
    let plan = |cx: &mut TestAppContext, may_host: bool, binary: Option<&str>| {
        let (record, options) = (record.clone(), options.clone());
        let binary = binary.map(std::path::PathBuf::from);
        cx.update(|cx| {
            window
                .workspace
                .update(cx, |ws, cx| ws.start_with(&record, &options, may_host, binary, cx))
                .expect("window")
        })
    };
    assert!(
        matches!(plan(cx, true, Some("/the/host")), Start::Host { .. }),
        "a new host, with the text as its seed"
    );
    assert!(matches!(plan(cx, false, Some("/the/host")), Start::Local), "not allowed to host: in the app");
    assert!(matches!(plan(cx, true, None), Start::Local), "no host program: in the app");
}
