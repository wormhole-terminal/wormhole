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
//! - `control`: the control requests and replies, their rules, limits and the command line's verbs;
//! - `exchange`: the command line's round trip over the control socket, and the welcome's;
//! - `mcp`: `wh mcp`, the same requests served to coding agents over MCP (stdin and stdout);
//! - `instance`: who serves the socket;
//! - `read_job`: a read of a terminal's screen, run by the socket thread;
//! - `access`: who may control the app: the identity behind a request, what the user allowed, and the decision;
//! - `ask_job`: a request that waits for the user's answer to a question;
//! - `wait_job`: a `wait` or a `run`, which a thread of its own carries out and answers once;
//! - `bus`: the events the app publishes and the subscriptions that hear them;
//! - `server`: the listener thread (connections from this user only) and the threads of the streams;
//! - `client`: the `hook` and `statusline` command lines;
//! - `hosts`: the session hosts seen from outside (ids, their folder, which are alive, ending one);
//! - `host_binary`: where the session host's executable is;
//! - `test_support`: the bounded receive the tests share.
//!
//! Tier 5: it uses `storage`, `agents`, `update`, `config` and `notify`. `terminal` and
//! `onboarding` sit above it, so a terminal's environment can name the socket without the socket
//! knowing about terminals.

pub mod access;
mod ask_job;
pub mod bus;
mod client;
pub mod control;
mod doctor;
pub(crate) mod exchange;
pub mod host_binary;
pub mod hosts;
mod instance;
mod mcp;
mod message;
mod read_job;
mod server;
#[cfg(test)]
mod skill_guards;
#[cfg(test)]
mod test_support;
mod wait_job;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use sha2::{Digest, Sha256};

use crate::platform::paths;
use crate::platform::socket::socket_in_with;
use crate::platform::user::UserId;
use crate::storage::paths::data_dir;

pub use ask_job::{AskJob, Decision};
pub use client::{Command, command, run_cli, run_status_line_cli};
pub use instance::{Instance, InstanceLock, Served, acquire, start_server};
pub use message::Message;
pub use read_job::ReadJob;
pub use server::{ANCESTORS_MAX, Peer};
pub use wait_job::WaitJob;

/// `Wormhole <verb> …`: the command line's round trip to the app, or, for `mcp`, the MCP server (which is not
/// a round trip: it serves its own stdin and stdout, and makes the round trips its callers ask for). The
/// exit code. Here, above both, so that `exchange` and `mcp` (which uses it) do not need each other.
pub fn run_control_cli(args: &[String]) -> i32 {
    match args.split_first() {
        Some((verb, rest)) if verb == "mcp" => mcp::run(rest),
        // Asks the app like any verb, and adds what the command line sees itself.
        Some((verb, _)) if verb == "doctor" => doctor::run(args),
        _ => exchange::run(args),
    }
}

/// What the socket hands the app.
#[derive(Debug)]
pub enum Incoming {
    /// A hook event, an activate or a usage report: nothing to answer.
    Message(Message),
    /// A control request, answered through `reply` (the connection waits for it), with who sent it
    /// as the system sees it.
    Control { request: control::Request, peer: Peer, reply: std::sync::mpsc::SyncSender<Answer> },
}

/// What the app gives the socket thread for a control request.
#[derive(Debug)]
pub enum Answer {
    /// Written back at once: every action that answers in one line, and every refusal.
    Reply(control::Reply),
    /// A read of a terminal's screen, which the socket thread carries out itself (the copy is not
    /// for the UI thread) and writes back as one line.
    Read(ReadJob),
    /// A `wait` or a `run`: a thread of its own carries it out and writes the one reply line when it is
    /// over, however long that takes (the connection is its own, so the hooks never wait for it). The
    /// app made the subscription it listens on while it answered, as for a stream, so it counts among
    /// the streams and is given back when the job ends or is dropped.
    Wait(WaitJob),
    /// The reply line now, then the connection is handed to a stream thread that writes what the
    /// subscription yields. The app makes the subscription while it answers, so nothing published
    /// after the reply is missed; if the socket gave up waiting, dropping the answer drops the
    /// subscription and gives its place back.
    Stream { reply: control::Reply, subscription: bus::Subscription },
    /// The request needs the user's approval and its caller said it will wait (`Request::ask`): the app
    /// queued the question. A thread of its own holds the connection until the user answers, then runs the
    /// request again (or writes the refusal the app decided on).
    Ask(AskJob),
}

impl From<control::Reply> for Answer {
    fn from(reply: control::Reply) -> Self {
        Answer::Reply(reply)
    }
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
    SOCKET_PATH.get_or_init(|| socket_path_in(&paths::temp(), &data_dir())).clone()
}

/// `<temp>/wormhole-<uid>-<hash of the data dir>.sock`: one per user and data
/// dir (isolated test instances don't collide), in the per-user temp folder
/// where the system has one (macOS `$TMPDIR`), named the same by every build
/// (unlike std's `DefaultHasher`). A path too long for a socket
/// falls back to `/tmp` (`platform::socket::socket_in`, which the hosts' sockets follow too).
fn socket_path_in(temp: &Path, data_dir: &Path) -> PathBuf {
    let digest = Sha256::digest(data_dir.as_os_str().as_encoded_bytes());
    let uid = UserId::current();
    let name =
        format!("wormhole-{uid}-{:02x}{:02x}{:02x}{:02x}.sock", digest[0], digest[1], digest[2], digest[3]);
    // The temp folder is the preferred one, so the fallback's own step through it adds nothing.
    socket_in_with(&temp.join(&name), &name, None)
}

pub fn remove_socket() {
    let _ = std::fs::remove_file(socket_path());
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::socket_path_in;
    use crate::platform::user::UserId;

    #[test]
    fn the_socket_name_is_stable_and_short() {
        let uid = UserId::current();
        let path = socket_path_in(Path::new("/var/folders/ab/T"), Path::new("/Users/me/Library/Wormhole"));
        // The same name from every build, so a running instance is always found.
        assert_eq!(path, Path::new(&format!("/var/folders/ab/T/wormhole-{uid}-18a91d25.sock")));
        let other = socket_path_in(Path::new("/var/folders/ab/T"), Path::new("/tmp/hd-data"));
        assert_ne!(path, other, "isolated instances don't collide");
        let long = socket_path_in(&Path::new("/x").join("y".repeat(120)), Path::new("/d"));
        assert!(long.starts_with("/tmp") && long.as_os_str().len() < 100);
    }
}
