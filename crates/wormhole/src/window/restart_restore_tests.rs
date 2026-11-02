//! Restoring after a restart of the Mac, in a real window (GPUI's test platform; see `test_harness`): a terminal
//! whose host is gone comes back from its host's checkpoint, the checkpoint is the app's to remove (a closed
//! terminal, a closed window), and none of it happens with the feature off. Checkpoints are written the way a
//! host writes them, into the instance's scratch data folder.

use gpui_kit::TestAppContext;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::storage::store::SessionRecord;
use wormhole_term::checkpoint::{Key, Snapshot, Store};

use super::restart_restore::sessions_dir;
use super::test_harness::{drive, isolate_in, start};
use crate::app_state::{OpenWindow, app, app_mut};
use crate::settings::update_settings;
use crate::terminal_view::Start;

const OLD: &str = "aaaaaaaaaaaa";
const OTHER: &str = "bbbbbbbbbbbb";

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

/// A window with the instance's scratch database open, which the saved text's key (and so the checkpoint's) comes
/// from: a development build keeps that key in a file beside the database, never in the Keychain.
fn start_with_store(cx: &mut TestAppContext, sessions: i64) -> OpenWindow {
    let window = start(cx, sessions);
    cx.update(|cx| app_mut(cx).store = Some(wormhole_core::storage::store::Store::open().expect("opens")));
    window
}

fn set_flags(cx: &mut TestAppContext, restore: bool) {
    cx.update(|cx| {
        update_settings(cx, |s| {
            s.feature_flags.insert(FeatureFlag::RestoreAfterRestart.key().into(), restore);
            s.feature_flags.insert(FeatureFlag::PersistentTerminals.key().into(), true);
        })
    });
}

/// The key the app derives for `host`, from the instance's own store.
fn key_for(cx: &mut TestAppContext, window: &OpenWindow, host: &str) -> Key {
    let profile = drive(cx, window, |ws, _, _| ws.profile().clone());
    cx.update(|cx| app(cx).store_of(&profile).and_then(|s| s.checkpoint_key(host)))
        .expect("the store has a key")
}

/// A checkpoint of `host` as its host wrote it.
fn write_checkpoint(host: &str, key: &Key, screen: &[u8]) {
    let mut store = Store::open(&sessions_dir(), host).expect("a store");
    store.write(key, &Snapshot { time: 1_793_023_327, ended: None, screen }).expect("written");
}

fn record(host: &str) -> SessionRecord {
    let mut record = SessionRecord::new(7, None, None);
    record.host_id = Some(host.into());
    record.scrollback = Some("the saved text".into());
    record
}

fn host_start() -> Start {
    Start::Host {
        binary: "/the/host".into(),
        data_dir: "/data".into(),
        id: "cccccccccccc".into(),
        restore: None,
    }
}

fn restore(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    record: &SessionRecord,
    start: &Start,
) -> (Option<std::sync::Arc<crate::terminal_view::PendingRestore>>, Start) {
    let mut start = start.clone();
    let done = cx.update(|cx| {
        window
            .workspace
            .read_with(cx, |ws, cx| ws.restore_from_checkpoint(record, &mut start, cx))
            .expect("window")
    });
    (done, start)
}

#[gpui_kit::test]
fn a_terminal_whose_host_is_gone_comes_back_from_its_checkpoint(cx: &mut TestAppContext) {
    if isolate("a_terminal_whose_host_is_gone_comes_back_from_its_checkpoint") {
        return;
    }
    let window = start_with_store(cx, 1);
    set_flags(cx, true);
    let key = key_for(cx, &window, OLD);
    write_checkpoint(OLD, &key, b"\x1b[1mbuilt it\x1b[0m\r\n$ cargo test\r\n");
    let (done, start) = restore(cx, &window, &record(OLD), &host_start());
    assert!(done.is_some());
    // What the host starts with is read in the background from the checkpoint, with the key named here.
    let Start::Host { restore: Some(from), .. } = start else { panic!("a host start that restores") };
    assert_eq!(from.old, OLD);
    let mut options = wormhole_core::terminal::SpawnOptions {
        restore: Some("the saved text".into()),
        ..Default::default()
    };
    let words = |t: u64| format!("restored at {t}");
    assert!(wormhole_core::terminal::checkpoint::seed_options(
        &sessions_dir(),
        OLD,
        Some(&from.key),
        &mut options,
        &words
    ));
    let drawn = options.restore.as_deref().unwrap();
    assert!(drawn.contains("built it") && drawn.contains("restored at 1793023327"), "{drawn:?}");
    assert!(!drawn.contains("the saved text"), "the screen replaces the text");
    // Not removed: the new host has not written its own yet, and a crash or another restart now must still find it.
    assert!(sessions_dir().join(OLD).is_dir(), "the old checkpoint stays until the new host's has landed");
    assert!(crate::terminal_view::pending_restores().contains(OLD), "and the sweep is told to spare it");
    // It can be restored from again (the app crashed before the new host wrote).
    let (again, _) = restore(cx, &window, &record(OLD), &host_start());
    assert!(again.is_some());
    // Once nothing waits on it any more, the sweep may have it.
    drop((done, again));
    assert!(!crate::terminal_view::pending_restores().contains(OLD));
}

#[gpui_kit::test]
fn it_is_for_a_terminal_that_starts_a_new_host_and_wanted_its_text_kept_and_only_with_the_feature_on(
    cx: &mut TestAppContext,
) {
    if isolate(
        "it_is_for_a_terminal_that_starts_a_new_host_and_wanted_its_text_kept_and_only_with_the_feature_on",
    ) {
        return;
    }
    let window = start_with_store(cx, 1);
    set_flags(cx, true);
    let key = key_for(cx, &window, OLD);
    write_checkpoint(OLD, &key, b"screen\r\n");
    // Attached to a live host, or in the app: no new shell to draw it above.
    let attach = Start::Attach { socket: "/s".into(), id: OLD.into(), visible: true };
    assert!(restore(cx, &window, &record(OLD), &attach).0.is_none());
    assert!(restore(cx, &window, &record(OLD), &Start::Local).0.is_none());
    // A terminal whose text the person asked not to keep.
    let mut forgotten = record(OLD);
    forgotten.forget_text = true;
    assert!(restore(cx, &window, &forgotten, &host_start()).0.is_none());
    // A row that never ran in a host.
    let mut fresh = record(OLD);
    fresh.host_id = None;
    assert!(restore(cx, &window, &fresh, &host_start()).0.is_none());
    // The feature off.
    set_flags(cx, false);
    assert!(restore(cx, &window, &record(OLD), &host_start()).0.is_none());
    assert!(sessions_dir().join(OLD).is_dir(), "none of that touched the checkpoint");
}

#[gpui_kit::test]
fn what_the_host_is_asked_for_follows_the_feature_the_text_setting_and_the_terminal(cx: &mut TestAppContext) {
    if isolate("what_the_host_is_asked_for_follows_the_feature_the_text_setting_and_the_terminal") {
        return;
    }
    let window = start_with_store(cx, 1);
    let plan = |cx: &mut TestAppContext, record: &SessionRecord| {
        cx.update(|cx| {
            window.workspace.read_with(cx, |ws, cx| ws.checkpoint_plan(record, OTHER, cx)).expect("window")
        })
    };
    set_flags(cx, false);
    assert!(plan(cx, &record(OLD)).is_none(), "off: the host is asked for nothing and writes nothing");
    set_flags(cx, true);
    let keyed = plan(cx, &record(OLD)).expect("asked");
    assert_eq!(keyed.key.as_bytes(), key_for(cx, &window, OTHER).as_bytes());
    // The terminal's text is not to be kept: no key, no plan, and the host writes nothing.
    let mut forgotten = record(OLD);
    forgotten.forget_text = true;
    assert!(plan(cx, &forgotten).is_none(), "no screen for a terminal whose text is not kept");
    // A profile that does not keep text after a restart.
    cx.update(|cx| update_settings(cx, |s| s.restore_terminal_text = false));
    assert!(plan(cx, &record(OLD)).is_none());
}

#[gpui_kit::test]
fn closing_a_row_removes_its_hosts_checkpoint_and_no_other(cx: &mut TestAppContext) {
    if isolate("closing_a_row_removes_its_hosts_checkpoint_and_no_other") {
        return;
    }
    let window = start_with_store(cx, 2);
    set_flags(cx, true);
    let key = key_for(cx, &window, OLD);
    write_checkpoint(OLD, &key, b"closed\r\n");
    write_checkpoint(OTHER, &key, b"kept\r\n");
    drive(cx, &window, |ws, _, cx| {
        ws.sessions[1].terminal().update(cx, |t, _| t.set_host_id_for_test(OLD));
        ws.sessions[0].terminal().update(cx, |t, _| t.set_host_id_for_test(OTHER));
    });
    drive(cx, &window, |ws, window, cx| ws.close_session(1, window, cx));
    assert!(!sessions_dir().join(OLD).exists(), "the closed terminal's checkpoint is gone");
    assert!(sessions_dir().join(OTHER).is_dir(), "the other's is not");
}

/// What the launch sweep must spare: running hosts, and the checkpoints terminals are being restored from.
#[test]
fn the_sweep_spares_running_hosts_and_pending_restores() {
    let running: std::collections::HashSet<String> = [OLD.to_owned()].into();
    let pending: std::collections::HashSet<String> = [OTHER.to_owned()].into();
    let spared = super::restart_restore::protected(running, pending);
    assert!(spared.contains(OLD) && spared.contains(OTHER) && spared.len() == 2);
}

/// Closing a terminal that was restored but whose new host had not written yet takes the old checkpoint too: no
/// one is going to restore from it.
#[gpui_kit::test]
fn closing_a_restored_terminal_before_its_new_host_wrote_removes_the_old_checkpoint_as_well(
    cx: &mut TestAppContext,
) {
    if isolate("closing_a_restored_terminal_before_its_new_host_wrote_removes_the_old_checkpoint_as_well") {
        return;
    }
    let window = start_with_store(cx, 2);
    set_flags(cx, true);
    let key = key_for(cx, &window, OLD);
    write_checkpoint(OLD, &key, b"old\r\n");
    let (pending, _) = restore(cx, &window, &record(OLD), &host_start());
    drive(cx, &window, |ws, _, cx| {
        ws.sessions[1].terminal().update(cx, |t, cx| {
            t.set_host_id_for_test(OTHER);
            t.set_checkpoint_plan(
                crate::terminal_view::CheckpointPlan { key: Key::new([3; 32]), replaces: pending },
                cx,
            );
        });
    });
    drive(cx, &window, |ws, window, cx| ws.close_session(1, window, cx));
    assert!(!sessions_dir().join(OLD).exists());
}

/// Turning the setting off tells every terminal's host to stop and removes what they wrote.
#[gpui_kit::test]
fn turning_the_setting_off_stops_the_hosts_and_removes_their_checkpoints(cx: &mut TestAppContext) {
    if isolate("turning_the_setting_off_stops_the_hosts_and_removes_their_checkpoints") {
        return;
    }
    let window = start_with_store(cx, 1);
    set_flags(cx, true);
    let key = key_for(cx, &window, OLD);
    write_checkpoint(OLD, &key, b"x\r\n");
    drive(cx, &window, |ws, _, cx| {
        ws.sessions[0].terminal().update(cx, |t, _| t.set_host_id_for_test(OLD));
    });
    set_flags(cx, false);
    drive(cx, &window, |ws, _, cx| {
        assert!(!ws.sessions[0].terminal().read(cx).has_checkpoint_plan_for_test(), "no plan any more");
    });
    assert!(!sessions_dir().join(OLD).exists());
}

/// The new host says its first checkpoint is on disk: the app reads it back once and then, and only then, removes
/// the old host's. With no word from the host the old one stays.
#[gpui_kit::test]
fn the_old_checkpoint_goes_when_the_new_host_says_its_own_is_written_and_not_before(cx: &mut TestAppContext) {
    if isolate("the_old_checkpoint_goes_when_the_new_host_says_its_own_is_written_and_not_before") {
        return;
    }
    let window = start_with_store(cx, 1);
    set_flags(cx, true);
    let old_key = key_for(cx, &window, OLD);
    let new_key = key_for(cx, &window, OTHER);
    write_checkpoint(OLD, &old_key, b"old\r\n");
    let (pending, _) = restore(cx, &window, &record(OLD), &host_start());
    drive(cx, &window, |ws, _, cx| {
        ws.sessions[0].terminal().update(cx, |t, cx| {
            t.set_host_id_for_test(OTHER);
            t.set_checkpoint_plan(
                crate::terminal_view::CheckpointPlan { key: new_key.clone(), replaces: pending },
                cx,
            );
        });
    });
    cx.run_until_parked();
    assert!(sessions_dir().join(OLD).is_dir(), "no word from the host: the old one stays");
    // The word comes, but the new host's files do not read (it wrote nothing): the old one still stays.
    drive(cx, &window, |ws, _, cx| {
        ws.sessions[0].terminal().update(cx, |t, cx| t.checkpoint_written_for_test(1, cx));
    });
    cx.run_until_parked();
    assert!(sessions_dir().join(OLD).is_dir(), "the new checkpoint is not there to read");
    // Now it is, and the word comes again (a fresh plan: the first was spent).
    write_checkpoint(OTHER, &new_key, b"new\r\n");
    let (pending, _) = restore(cx, &window, &record(OLD), &host_start());
    drive(cx, &window, |ws, _, cx| {
        ws.sessions[0].terminal().update(cx, |t, cx| {
            t.set_checkpoint_plan(
                crate::terminal_view::CheckpointPlan { key: new_key.clone(), replaces: pending },
                cx,
            );
            t.checkpoint_written_for_test(2, cx);
        });
    });
    cx.run_until_parked();
    assert!(!sessions_dir().join(OLD).exists(), "read back once, then the old one went");
    assert!(sessions_dir().join(OTHER).is_dir());
    assert!(!crate::terminal_view::pending_restores().contains(OLD));
}
