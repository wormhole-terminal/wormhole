//! Over temp data folders with a real `wormhole-host` process: start, quit-keep, attach again, end.
//! Few terminals on purpose: PTYs are a shared system limit.

mod support;

use std::time::Duration;

use wormhole_core::ipc::hosts;

use self::support::{Fixture, wait_for};
use super::{HostedStart, HostingError, HostingProgram, TerminalHosting};

/// The one `cat` of a test: what is typed comes back, on the live screen and after a reattach.
#[test]
fn a_terminal_outlives_the_app_and_comes_back_with_its_screen() {
    let first = Fixture::new();
    let desk = first.desk();
    assert!(matches!(desk.program(), HostingProgram::Found { .. }));
    let terminal = desk.start(first.request(1), support::listener()).expect("starts in a host");
    let host_id = terminal.host_id().expect("hosted");
    assert_eq!(terminal.hosting(), TerminalHosting::Persistent { host_id: host_id.clone() });
    terminal.write(b"remember-me\n".to_vec());
    wait_for("the echo", || terminal.screen_text().contains("remember-me"));

    // Quit and keep: let go of the host, then the app's objects are dropped (the process would be gone).
    desk.finish(false);
    assert_eq!(terminal.hosting(), TerminalHosting::Detached { host_id: host_id.clone() });
    terminal.close();
    drop((terminal, desk));
    assert_eq!(first.live_hosts(), vec![host_id.clone()], "the program runs on without the app");

    // The next launch: its own object over the same folder.
    let second = first.desk();
    let plan = second.plan_at_launch().expect("plans");
    assert_eq!(
        plan.saved.iter().map(|s| (s.host_id.as_str(), s.running)).collect::<Vec<_>>(),
        [(host_id.as_str(), true)]
    );
    assert!(plan.orphans.is_empty());
    let saved = &plan.saved[0];
    assert_eq!(saved.cwd.as_deref(), Some(first.cwd()));
    let again =
        second.attach(host_id.clone(), first.request(7), true, support::listener()).expect("attaches");
    wait_for("the repaint", || again.screen_text().contains("remember-me"));
    again.write(b"and-more\n".to_vec());
    wait_for("the new echo", || again.screen_text().contains("and-more"));
    second.finish_restore().expect("prunes");

    // The rows were renumbered: the old id (1) is gone, the new one (7) names the host, nothing is doubled.
    let third = first.desk();
    let plan = third.plan_at_launch().expect("plans");
    assert_eq!(plan.saved.len(), 1, "one row for one host: {plan:?}");

    // End All: the program ends with its host and no row names it.
    drop(third);
    second.finish(true);
    wait_for("the host to end", || first.live_hosts().is_empty());
    assert!(first.desk().plan_at_launch().expect("plans").saved.is_empty());
}

/// Closing an attached terminal on purpose ends its program, so nothing is left to attach to.
#[test]
fn closing_a_terminal_ends_its_host_and_forgets_its_row() {
    let fixture = Fixture::new();
    let desk = fixture.desk();
    let terminal = desk.start(fixture.request(3), support::listener()).expect("starts");
    assert_eq!(fixture.live_hosts().len(), 1);
    terminal.close();
    wait_for("the host to end", || fixture.live_hosts().is_empty());
    assert!(fixture.desk().plan_at_launch().expect("plans").saved.is_empty());
}

/// "Move this window to <profile>": a terminal started after the move is saved in the profile the window is in, and
/// closing a terminal that ran before the move forgets its row there.
#[test]
fn a_window_that_moved_keeps_its_terminals_rows_in_its_new_profile() {
    let fixture = Fixture::new();
    let desk = fixture.desk();
    let before = desk.start(fixture.request(1), support::listener()).expect("starts");
    assert_eq!(fixture.hosted_rows_in("default"), vec![1]);

    desk.follow_profile("work".into()).expect("follows");
    let after = desk.start(fixture.request(2), support::listener()).expect("starts");
    assert_eq!(fixture.hosted_rows_in("work"), vec![2], "the new terminal's row is in the profile the window moved to");
    assert_eq!(fixture.hosted_rows_in("default"), vec![1], "nothing new in the profile it left");

    after.close();
    before.close();
    wait_for("the hosts to end", || fixture.live_hosts().is_empty());
    assert!(fixture.hosted_rows_in("work").is_empty());
}

/// A saved row whose host is gone starts again; a live host no row names is an orphan that can be adopted.
#[test]
fn gone_hosts_replay_and_unnamed_hosts_are_orphans() {
    let fixture = Fixture::new();
    let desk = fixture.desk();
    let (kept, lost) = (
        desk.start(fixture.request(1), support::listener()).expect("starts"),
        desk.start(fixture.request(2), support::listener()).expect("starts"),
    );
    let (kept_id, lost_id) = (kept.host_id().expect("hosted"), lost.host_id().expect("hosted"));
    desk.finish(false);
    drop((kept, lost, desk));

    // One host ended behind the app's back (killed from the Session Hosts window, say).
    let meta = hosts::find(&fixture.hosts_dir(), &lost_id).expect("meta");
    hosts::end_host(&meta, Duration::from_secs(5)).expect("ends");
    wait_for("it to go", || fixture.live_hosts() == vec![kept_id.clone()]);
    let plan = fixture.desk().plan_at_launch().expect("plans");
    let standing: Vec<(String, bool)> = plan.saved.iter().map(|s| (s.host_id.clone(), s.running)).collect();
    assert_eq!(
        standing,
        [(kept_id.clone(), true), (lost_id.clone(), false)],
        "saved order, each with its standing"
    );

    // Attaching to the one that is gone says so, which is the cue to start a new shell.
    let desk = fixture.desk();
    desk.plan_at_launch().expect("plans");
    let err = desk.attach(lost_id, fixture.request(5), true, support::listener()).err().expect("gone");
    assert!(matches!(err, HostingError::Gone), "{err:?}");

    // Forget the row of the live one: its host is nobody's, so it is offered, and takes the app.
    fixture.forget_rows();
    let desk = fixture.desk();
    let plan = desk.plan_at_launch().expect("plans");
    assert_eq!(plan.orphans.iter().map(|o| o.host_id.as_str()).collect::<Vec<_>>(), [kept_id.as_str()]);
    let adoptable = desk.adoptable_orphans();
    assert_eq!(adoptable.len(), 1);
    assert!(desk.adoptable_orphans().is_empty(), "asked once");
    let adopted = desk.attach(kept_id, fixture.request(9), true, support::listener()).expect("adopts");
    desk.finish(true);
    drop(adopted);
    wait_for("the host to end", || fixture.live_hosts().is_empty());
}

#[test]
fn without_a_host_program_nothing_starts_in_one() {
    let fixture = Fixture::new();
    let desk = fixture.desk_without_program();
    assert_eq!(desk.program(), HostingProgram::Missing);
    let err = desk.start(fixture.request(1), support::listener()).err().expect("no program");
    assert!(matches!(err, HostingError::NoProgram { .. }), "{err:?}");
}

/// The switch is core's flag rules over the folder's `config.json`: a test build is a dev build, where a
/// flag with no entry is on and the map can turn it off.
#[test]
fn the_flag_is_read_from_the_folders_settings() {
    let fixture = Fixture::new();
    assert!(fixture.desk().enabled(), "a dev build defaults a flag on");
    std::fs::write(fixture.config_file(), r#"{"feature_flags": {"persistent_terminals": false}}"#)
        .expect("writes");
    assert!(!fixture.desk().enabled());
}

#[test]
fn a_folder_that_is_not_the_experiments_is_refused() {
    let err = super::HostedTerminals::new(Some("/tmp/Wormhole".into()), None, 1).err().expect("refused");
    assert!(matches!(err, HostingError::NotIsolated { .. }), "{err:?}");
}

/// What a terminal whose host is gone is drawn from: the checkpoint the host kept (a restart of the Mac), which the new
/// host's own first checkpoint then replaces; or, with none, its saved text.
#[test]
fn a_terminal_whose_host_is_gone_is_drawn_from_its_checkpoint_then_its_text() {
    let fixture = Fixture::new();
    let desk = fixture.desk();
    let first = desk.start(fixture.request(1), support::listener()).expect("starts");
    let old = first.host_id().expect("hosted");
    first.write(b"kept-on-screen\n".to_vec());
    wait_for("the echo", || first.screen_text().contains("kept-on-screen"));
    let checkpoint = fixture.sessions_dir().join(&old);
    wait_for("the host's checkpoint", || checkpoint.exists());
    assert_eq!(desk.save_texts(), 1, "the screen is kept for a host that will not be there");
    desk.finish(false);
    drop((first, desk));
    // The Mac restarted: the host is gone, its checkpoint and the saved row are not.
    let meta = hosts::find(&fixture.hosts_dir(), &old).expect("meta");
    hosts::end_host(&meta, Duration::from_secs(5)).expect("ends");
    wait_for("it to go", || fixture.live_hosts().is_empty());
    assert!(checkpoint.exists(), "the host never deletes its checkpoint");

    let again = fixture.desk();
    let plan = again.plan_at_launch().expect("plans");
    let saved = &plan.saved[0];
    assert!(!saved.running);
    assert!(saved.text.as_deref().is_some_and(|text| text.contains("kept-on-screen")), "{saved:?}");
    let request = HostedStart {
        old_host: Some(old.clone()),
        saved_text: saved.text.clone(),
        ..fixture.request(saved.terminal)
    };
    let restored = again.start(request, support::listener()).expect("starts again");
    wait_for("the screen above the shell", || {
        let screen = restored.screen_text();
        screen.contains("kept-on-screen") && screen.contains("restored")
    });
    wait_for("the new host's checkpoint to replace the old", || !checkpoint.exists());

    // Without a checkpoint the saved text is what is drawn.
    let third = fixture.desk();
    let request = HostedStart {
        old_host: Some("nohostatall1".into()),
        saved_text: Some("from-the-saved-text".into()),
        ..fixture.request(8)
    };
    let text = third.start(request, support::listener()).expect("starts");
    wait_for("the saved text", || text.screen_text().contains("from-the-saved-text"));
    again.finish(true);
    third.finish(true);
}
