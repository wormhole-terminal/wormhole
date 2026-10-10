//! The bridge from coding agents' hooks to the app.
//!
//! Every shell gets `WORMHOLE_SOCKET`, `WORMHOLE_TAB_ID` and
//! `WORMHOLE_BIN`. Agent hooks (installed by `agents::integrations`) run
//! `$WORMHOLE_BIN hook [--agent <id>] <Event> [payload]`, which forwards
//! the event over the socket, so the sidebar knows for sure when an agent
//! is working, waiting for you, or done.
//!
//! The same socket doubles as the single-instance lock, and carries the control protocol
//! (`control`: `Wormhole list`, `status`, `notify`, `send-text`, `new-terminal`), whose requests
//! are the one kind of message the app answers, on the same connection.
//!
//! - `message`: the wire format;
//! - `control`: the control requests and replies, their rules, limits and command line;
//! - `instance`: who serves the socket;
//! - `server`: the listener thread (connections from this user only);
//! - `client`: the `hook` and `statusline` command lines;
//! - `test_support`: the bounded receive the tests share.

mod client;
pub mod control;
mod instance;
mod message;
mod server;
#[cfg(test)]
mod test_support;

use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use sha2::{Digest, Sha256};

use crate::store::data_dir;

pub use client::{Command, command, run_cli, run_status_line_cli};
pub use control::run as run_control_cli;
pub(crate) use instance::is_locked_instance_dir;
pub use instance::{Instance, InstanceLock, acquire, start_server};
pub use message::Message;

/// What the socket hands the app.
#[derive(Debug)]
pub enum Incoming {
    /// A hook event, an activate or a usage report: nothing to answer.
    Message(Message),
    /// A control request, answered through `reply` (the connection waits for it).
    Control { request: control::Request, reply: std::sync::mpsc::SyncSender<control::Reply> },
}

static SOCKET_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Where the running instance listens; see [`socket_path_in`].
/// What a tab's shell gets to reach its app, and the names versions up to 0.2 used for them (the
/// app's old codename). Both are exported; [`tab_var`] reads either.
pub const LEGACY_TAB_VARS: &[(&str, &str)] = &[
    ("WORMHOLE_TAB_ID", "ANDROMEDA_TAB_ID"),
    ("WORMHOLE_SOCKET", "ANDROMEDA_SOCKET"),
    ("WORMHOLE_BIN", "ANDROMEDA_BIN"),
];

/// One of a tab's variables ([`LEGACY_TAB_VARS`]), or its old name's value when only that is set
/// (a shell an earlier version started).
pub fn tab_var(name: &str) -> Option<String> {
    let old = LEGACY_TAB_VARS.iter().find(|(new, _)| *new == name).map(|(_, old)| *old);
    std::env::var(name).ok().or_else(|| old.and_then(|old| std::env::var(old).ok()))
}

pub fn socket_path() -> PathBuf {
    SOCKET_PATH.get_or_init(|| socket_path_in(&std::env::temp_dir(), &data_dir())).clone()
}

/// `<temp>/wormhole-<uid>-<hash of the data dir>.sock`: one per user and data
/// dir (isolated test instances don't collide), in the per-user temp folder
/// where the system has one (macOS `$TMPDIR`), named the same by every build
/// (unlike std's `DefaultHasher`). Unix socket paths are limited to about 104
/// bytes, so an unusually long temp folder falls back to `/tmp`.
fn socket_path_in(temp: &Path, data_dir: &Path) -> PathBuf {
    let digest = Sha256::digest(data_dir.as_os_str().as_bytes());
    // SAFETY: getuid has no preconditions and cannot fail.
    #[allow(unsafe_code)]
    let uid = unsafe { libc::getuid() };
    let name =
        format!("wormhole-{uid}-{:02x}{:02x}{:02x}{:02x}.sock", digest[0], digest[1], digest[2], digest[3]);
    let path = temp.join(&name);
    if path.as_os_str().len() < 100 { path } else { Path::new("/tmp").join(name) }
}

pub fn remove_socket() {
    let _ = std::fs::remove_file(socket_path());
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::socket_path_in;

    #[test]
    fn the_socket_name_is_stable_and_short() {
        let uid = {
            // SAFETY: getuid has no preconditions and cannot fail.
            #[allow(unsafe_code)]
            unsafe {
                libc::getuid()
            }
        };
        let path = socket_path_in(Path::new("/var/folders/ab/T"), Path::new("/Users/me/Library/Wormhole"));
        // The same name from every build, so a running instance is always found.
        assert_eq!(path, Path::new(&format!("/var/folders/ab/T/wormhole-{uid}-18a91d25.sock")));
        let other = socket_path_in(Path::new("/var/folders/ab/T"), Path::new("/tmp/hd-data"));
        assert_ne!(path, other, "isolated instances don't collide");
        let long = socket_path_in(&Path::new("/x").join("y".repeat(120)), Path::new("/d"));
        assert!(long.starts_with("/tmp") && long.as_os_str().len() < 100);
    }
}
