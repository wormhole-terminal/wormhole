//! A terminal whose session host runs but could not be attached to keeps the host's id and says why;
//! only a host that is gone gives up its id. And such a host is not adopted again as a stranger at
//! the next launch, nor given a second row when a terminal already names it. (Real window, GPUI's
//! test platform; see `test_harness`.)

use gpui_kit::TestAppContext;
use wormhole_core::ipc::hosts::{AttachFailure, HostMeta, LaunchHosts, Unavailable, UnavailableKind};

use super::Workspace;
use super::reported_tests::terminal;
use super::test_harness::{isolate_in, read, start};
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
