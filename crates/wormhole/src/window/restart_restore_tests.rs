//! Restoring after a restart of the Mac, in a real window (GPUI's test platform; see `test_harness`): a terminal
//! whose host is gone comes back from its host's checkpoint, the checkpoint is the app's to remove (a closed
//! terminal, a closed window), and none of it happens with the feature off. Checkpoints are written the way a
//! host writes them, into the instance's scratch data folder.

use gpui_kit::TestAppContext;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::storage::store::SessionRecord;
use wormhole_core::terminal::SpawnOptions;
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
fn key_for(cx: &mut TestAppContext, window: &OpenWindow, host: &str) -> [u8; 32] {
    let profile = drive(cx, window, |ws, _, _| ws.profile().clone());
    cx.update(|cx| app(cx).store_of(&profile).and_then(|s| s.checkpoint_key(host)))
        .expect("the store has a key")
}

/// A checkpoint of `host` as its host wrote it.
fn write_checkpoint(host: &str, key: [u8; 32], screen: &[u8]) {
    let mut store = Store::open(&sessions_dir(), host).expect("a store");
    let snapshot = Snapshot {
        time: 1_793_023_327,
        cols: 80,
        rows: 24,
        cwd: Some("/tmp"),
        shell: Some("login"),
        title: None,
        ended: None,
        screen: Some(screen),
    };
    store.write(Some(&Key::new(key)), &snapshot).expect("written");
}

fn record(host: &str) -> SessionRecord {
    let mut record = SessionRecord::new(7, None, None);
    record.host_id = Some(host.into());
    record.scrollback = Some("the saved text".into());
    record
}

fn host_start() -> Start {
    Start::Host { binary: "/the/host".into(), data_dir: "/data".into(), id: "cccccccccccc".into() }
}

fn restore(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    record: &SessionRecord,
    start: &Start,
) -> (Option<std::sync::Arc<crate::terminal_view::PendingRestore>>, SpawnOptions) {
    let mut options = SpawnOptions { restore: record.scrollback.clone(), ..Default::default() };
    let done = cx.update(|cx| {
        window
            .workspace
            .read_with(cx, |ws, cx| ws.restore_from_checkpoint(record, start, &mut options, cx))
            .expect("window")
    });
    (done, options)
}

#[gpui_kit::test]
fn a_terminal_whose_host_is_gone_comes_back_from_its_checkpoint(cx: &mut TestAppContext) {
    if isolate("a_terminal_whose_host_is_gone_comes_back_from_its_checkpoint") {
        return;
    }
    let window = start_with_store(cx, 1);
    set_flags(cx, true);
    let key = key_for(cx, &window, OLD);
    write_checkpoint(OLD, key, b"\x1b[1mbuilt it\x1b[0m\r\n$ cargo test\r\n");
    let (done, options) = restore(cx, &window, &record(OLD), &host_start());
    assert!(done.is_some());
    assert!(options.restore_styled);
    let drawn = options.restore.as_deref().unwrap();
    assert!(drawn.contains("built it") && drawn.contains("restored 20"), "{drawn:?}");
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
    write_checkpoint(OLD, key, b"screen\r\n");
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
    assert_eq!(keyed.shell, "login");
    assert_eq!(keyed.key.as_ref().map(|k| *k.as_bytes()), Some(key_for(cx, &window, OTHER)));
    let mut shell = record(OLD);
    shell.shell = Some("fish".into());
    assert_eq!(plan(cx, &shell).unwrap().shell, "fish");
    let mut forgotten = record(OLD);
    forgotten.forget_text = true;
    let plan_forgotten = plan(cx, &forgotten).expect("asked");
    assert!(plan_forgotten.key.is_none(), "no screen for a terminal whose text is not kept");
}

#[gpui_kit::test]
fn closing_a_row_removes_its_hosts_checkpoint_and_no_other(cx: &mut TestAppContext) {
    if isolate("closing_a_row_removes_its_hosts_checkpoint_and_no_other") {
        return;
    }
    let window = start_with_store(cx, 2);
    set_flags(cx, true);
    let key = key_for(cx, &window, OLD);
    write_checkpoint(OLD, key, b"closed\r\n");
    write_checkpoint(OTHER, key, b"kept\r\n");
    drive(cx, &window, |ws, _, cx| {
        ws.sessions[1].terminal().update(cx, |t, _| t.set_host_id_for_test(OLD));
        ws.sessions[0].terminal().update(cx, |t, _| t.set_host_id_for_test(OTHER));
    });
    drive(cx, &window, |ws, window, cx| ws.close_session(1, window, cx));
    assert!(!sessions_dir().join(OLD).exists(), "the closed terminal's checkpoint is gone");
    assert!(sessions_dir().join(OTHER).is_dir(), "the other's is not");
}

/// Clear Buffer forgets what a checkpoint holds of the buffer: it is gone before the host writes the next, so a
/// restart cannot bring back what the person cleared.
#[gpui_kit::test]
fn clearing_the_buffer_removes_the_checkpoint_that_holds_it(cx: &mut TestAppContext) {
    if isolate("clearing_the_buffer_removes_the_checkpoint_that_holds_it") {
        return;
    }
    let window = start_with_store(cx, 1);
    set_flags(cx, true);
    let key = key_for(cx, &window, OLD);
    write_checkpoint(OLD, key, b"secret scrollback\r\n");
    drive(cx, &window, |ws, _, cx| {
        ws.sessions[0].terminal().update(cx, |t, _| t.set_host_id_for_test(OLD));
    });
    drive(cx, &window, |ws, _, cx| {
        let terminal = ws.sessions[0].terminal().clone();
        terminal.update(cx, |t, cx| t.clear_buffer_for_test(cx));
    });
    assert!(!sessions_dir().join(OLD).exists());
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
    write_checkpoint(OLD, key, b"old\r\n");
    let (pending, _) = restore(cx, &window, &record(OLD), &host_start());
    drive(cx, &window, |ws, _, cx| {
        ws.sessions[1].terminal().update(cx, |t, cx| {
            t.set_host_id_for_test(OTHER);
            t.set_checkpoint_plan(
                crate::terminal_view::CheckpointPlan { key: None, shell: "login".into(), replaces: pending },
                cx,
            );
        });
    });
    drive(cx, &window, |ws, window, cx| ws.close_session(1, window, cx));
    assert!(!sessions_dir().join(OLD).exists());
}
