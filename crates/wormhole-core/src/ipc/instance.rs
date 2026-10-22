//! The single-instance lock: the first launch holds a lock on a file in the
//! data dir and serves the hook socket; later launches ask it to come to the
//! front and exit, unless they started inside one of its tabs.
//!
//! The lock used to be the socket itself ("connect, else remove and bind"),
//! which two launches close together could both pass: both became primary on
//! the same database, and the second bind took the socket from the first, so
//! its agents' hooks silently reached the wrong instance. The kernel lock can
//! only be held once, and it goes away with the process, crashes included, so
//! whoever holds it may remove a socket file left behind.

use std::fs::{self, File, TryLockError};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use super::{Incoming, Message, server, socket_path};
use crate::storage::paths::{INSTANCE_LOCK_FILE, data_dir};

/// Held while this process is the running instance (closing the file unlocks).
#[derive(Debug)]
pub struct InstanceLock {
    _file: File,
}

#[derive(Debug)]
pub enum Instance {
    /// This process runs the app; hook messages and control requests arrive on `events`.
    Primary { events: smol::channel::Receiver<Incoming>, lock: InstanceLock },
    /// Another process does (and was asked to come to the front, if this launch may ask).
    Secondary,
}

/// Become the running instance for this data dir, or find the one that is.
pub fn acquire() -> std::io::Result<Instance> {
    let tab = super::tab_var("WORMHOLE_TAB_ID");
    acquire_in(&data_dir(), &socket_path(), asks_to_come_forward(tab.as_deref()))
}

/// Whether a launch that finds the app running asks it to come forward. Not from inside one of
/// its tabs (`tab_id`: that tab's `WORMHOLE_TAB_ID`): there the app is in front already when a
/// person launches it, and otherwise it is a program running in the tab (an agent's `cargo run`,
/// a script) that must not take the screen from whatever the person is doing.
pub fn asks_to_come_forward(tab_id: Option<&str>) -> bool {
    tab_id.is_none_or(|id| id.trim().is_empty())
}

pub(super) fn acquire_in(dir: &Path, socket: &Path, come_forward: bool) -> std::io::Result<Instance> {
    crate::storage::fsutil::create_dir_all_private(dir)?;
    let file = File::options()
        .create(true)
        .write(true)
        .truncate(false)
        .mode(crate::storage::fsutil::PRIVATE_FILE)
        .open(dir.join(INSTANCE_LOCK_FILE))?;
    match file.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => {
            if come_forward {
                activate(socket);
            }
            return Ok(Instance::Secondary);
        }
        Err(TryLockError::Error(err)) => return Err(err),
    }
    let lock = InstanceLock { _file: file };
    let events = listen(socket).unwrap_or_else(|err| {
        // Keep the lock all the same: giving it up would let the next launch become primary on
        // the same database. This one runs without hooks instead (nothing ever arrives).
        tracing::warn!("hook socket {} unavailable, agent hooks are off: {err}", socket.display());
        smol::channel::unbounded().1
    });
    Ok(Instance::Primary { events, lock })
}

/// Serve the hook socket (the lock is held, so a socket file there is a leftover).
fn listen(socket: &Path) -> std::io::Result<smol::channel::Receiver<Incoming>> {
    let _ = fs::remove_file(socket);
    if let Some(parent) = socket.parent() {
        fs::create_dir_all(parent)?;
    }
    let listener = UnixListener::bind(socket)?;
    // Only this user's hooks may talk to us; a socket that can't be made private is not served.
    let served = fs::set_permissions(socket, fs::Permissions::from_mode(0o600)).and_then(|()| {
        let (tx, rx) = smol::channel::unbounded();
        server::spawn(listener, tx)?;
        Ok(rx)
    });
    if served.is_err() {
        let _ = fs::remove_file(socket);
    }
    served
}

/// Ask the running instance to come to the front. It may hold the lock but
/// not listen yet (it is starting), so keep trying for a moment.
fn activate(socket: &Path) {
    for _ in 0..20 {
        if let Ok(mut stream) = UnixStream::connect(socket) {
            if let Ok(json) = serde_json::to_string(&Message::Activate) {
                let _ = writeln!(stream, "{json}");
            }
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Become the running instance, or return `None` if one already exists
/// (after asking it to activate). Without a lock or a socket (an unwritable
/// data dir) the app still runs, without hooks.
pub fn start_server() -> Option<smol::channel::Receiver<Incoming>> {
    static HELD: OnceLock<InstanceLock> = OnceLock::new();
    match acquire() {
        Ok(Instance::Primary { events, lock }) => {
            let _ = HELD.set(lock);
            Some(events)
        }
        Ok(Instance::Secondary) => None,
        Err(err) => {
            tracing::warn!("single-instance lock or hook socket unavailable: {err}");
            Some(smol::channel::unbounded().1)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixStream;
    use std::sync::{Arc, Barrier};
    use std::time::Duration;

    use super::{Instance, acquire_in, asks_to_come_forward};
    use crate::ipc::Message;
    use crate::ipc::test_support::{recv_message_within, recv_within};

    /// Long enough for a loaded CI runner; a message that is not coming fails the test after this
    /// instead of hanging it.
    const WAIT: Duration = Duration::from_secs(5);

    #[test]
    fn two_launches_at_once_make_one_primary() {
        let dir = tempfile::tempdir().expect("tempdir");
        let socket = dir.path().join("hd.sock");
        let barrier = Arc::new(Barrier::new(2));
        let launches: Vec<_> = (0..2)
            .map(|_| {
                let (dir, socket, barrier) = (dir.path().to_path_buf(), socket.clone(), barrier.clone());
                std::thread::spawn(move || {
                    barrier.wait();
                    acquire_in(&dir, &socket, true).expect("acquire")
                })
            })
            .collect();
        let instances: Vec<Instance> = launches.into_iter().map(|t| t.join().expect("thread")).collect();
        let primaries: Vec<&Instance> =
            instances.iter().filter(|i| matches!(i, Instance::Primary { .. })).collect();
        assert_eq!(primaries.len(), 1);
        let Instance::Primary { events, .. } = primaries[0] else { unreachable!() };

        // The second launch asked the first to come forward, and hooks reach it.
        let activate = recv_message_within(events, WAIT);
        assert!(matches!(activate, Some(Message::Activate)), "no Activate within {WAIT:?}: {activate:?}");
        let mut hook = UnixStream::connect(&socket).expect("connect");
        writeln!(hook, r#"{{"kind":"hook","tab":"7","event":"Stop"}}"#).expect("write");
        let message = recv_message_within(events, WAIT);
        assert!(
            matches!(message, Some(Message::Hook { ref tab, .. }) if tab == "7"),
            "no hook within {WAIT:?}: {message:?}"
        );
        let mode = std::fs::metadata(&socket).expect("socket").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    /// The next launch after the primary is gone. The lock is released when the last copy of its
    /// file closes, and a child process forked by a test running in parallel (the terminal tests
    /// spawn shells) holds a copy until its `exec`; on macOS CI that window was long enough to
    /// make an instant retry fail. Wait a bounded time instead of asserting at once.
    fn acquire_once_free(dir: &std::path::Path, socket: &std::path::Path) -> Instance {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let instance = acquire_in(dir, socket, true).expect("acquire");
            if matches!(instance, Instance::Primary { .. }) || std::time::Instant::now() > deadline {
                return instance;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    /// The bug: a socket that couldn't be served returned the error and dropped the lock with
    /// it, so the next launch became primary on the same database as this one.
    #[test]
    fn without_a_socket_the_primary_keeps_the_lock_and_runs_without_hooks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let not_a_dir = dir.path().join("not-a-dir");
        std::fs::write(&not_a_dir, "").expect("write");
        let socket = not_a_dir.join("hd.sock");
        let first = acquire_in(dir.path(), &socket, true).expect("a missing socket is not an error");
        let Instance::Primary { events, .. } = &first else { panic!("not primary: {first:?}") };
        assert!(recv_within(events, Duration::from_millis(200)).is_none(), "no hooks ever arrive");
        assert!(matches!(acquire_in(dir.path(), &socket, true).expect("second"), Instance::Secondary));
        drop(first);
        assert!(matches!(acquire_once_free(dir.path(), &socket), Instance::Primary { .. }));
    }

    #[test]
    fn the_next_launch_takes_over_once_the_primary_is_gone() {
        let dir = tempfile::tempdir().expect("tempdir");
        let socket = dir.path().join("hd.sock");
        let first = acquire_in(dir.path(), &socket, true).expect("first");
        assert!(matches!(first, Instance::Primary { .. }));
        assert!(matches!(acquire_in(dir.path(), &socket, true).expect("second"), Instance::Secondary));
        drop(first);
        // Its socket file is still there (as after a crash); the new primary replaces it.
        assert!(socket.exists());
        let third = acquire_once_free(dir.path(), &socket);
        let Instance::Primary { events, .. } = third else { panic!("not primary: {third:?}") };
        let mut hook = UnixStream::connect(&socket).expect("connect");
        writeln!(hook, r#"{{"kind":"activate"}}"#).expect("write");
        let message = recv_message_within(&events, WAIT);
        assert!(matches!(message, Some(Message::Activate)), "no Activate within {WAIT:?}: {message:?}");
    }

    #[test]
    fn a_launch_from_inside_a_tab_never_brings_the_app_forward() {
        assert!(asks_to_come_forward(None), "the Dock, the Finder, a shell elsewhere");
        assert!(asks_to_come_forward(Some("")));
        assert!(!asks_to_come_forward(Some("42")), "an agent's `cargo run` in a tab");

        let dir = tempfile::tempdir().expect("tempdir");
        let socket = dir.path().join("hd.sock");
        let first = acquire_in(dir.path(), &socket, true).expect("first");
        let Instance::Primary { events, .. } = &first else { panic!("not primary: {first:?}") };
        assert!(matches!(acquire_in(dir.path(), &socket, false).expect("second"), Instance::Secondary));
        assert!(recv_within(events, Duration::from_millis(300)).is_none(), "it was not asked to activate");
    }
}
