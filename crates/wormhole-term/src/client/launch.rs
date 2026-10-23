//! Starting a session host process: `wormhole-host --id <id> --dir <hosts dir>`, detached through
//! [`sys::detached`](crate::sys::detached) (a session of its own, so it outlives the app), with:
//!
//! - a near-empty environment: `WORMHOLE_LOG` (its log level) and `TMPDIR` (where its socket goes
//!   when the hosts folder is too deep for one) if the app has them. The program's environment is
//!   sent in `Spawn`, never on a command line or in the host's own environment;
//! - the readiness pipe, which says one line: `ok <pid>` once the program runs, or `err <text>`.
//!
//! The host is this process's child while the app runs, and until the [`Started`] is dropped its pid
//! cannot be anyone else's, so a host that failed to come up can be signalled safely.

use std::ffi::OsStr;
use std::io;
use std::path::Path;
use std::time::Instant;

use crate::sys::detached::{self, Detached, ReadyLine, Signal};

/// A host process just started. Dropped, it is left to the reaper.
pub(super) struct Started {
    detached: Detached,
}

impl Started {
    pub(super) fn pid(&self) -> u32 {
        self.detached.pid()
    }

    /// Send the host `signal` (one that failed to come up is not left behind). Sound until this
    /// is dropped: nobody has reaped the host, so its pid is still its own, even as a zombie.
    pub(super) fn signal(&self, signal: Signal) {
        self.detached.signal(signal);
    }

    /// What the readiness pipe says by `until` (a past `until` only looks).
    pub(super) fn ready(&mut self, until: Instant) -> Ready {
        match self.detached.read_ready(until) {
            Ok(ReadyLine::Line(line)) => parse_line(&line),
            Ok(ReadyLine::Closed) => Ready::Closed,
            Ok(ReadyLine::Silent) => Ready::Silent,
            Err(err) => Ready::Err(format!("the session host's readiness pipe: {err}")),
        }
    }
}

/// What the readiness pipe said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Ready {
    /// The program runs with this pid.
    Ok(u32),
    /// The host could not start it, or itself: the system's words for why.
    Err(String),
    /// The pipe closed without a line: the host is gone.
    Closed,
    /// Nothing yet.
    Silent,
}

/// Start the host binary at `binary` for `id` in `hosts_dir`.
pub(super) fn start(binary: &Path, hosts_dir: &Path, id: &str) -> io::Result<Started> {
    let args = [OsStr::new("--id"), OsStr::new(id), OsStr::new("--dir"), hosts_dir.as_os_str()];
    Ok(Started { detached: detached::spawn(binary, &args, &host_env())? })
}

/// What the host's environment holds: the log level and the temporary folder, if the app has them.
fn host_env() -> Vec<(&'static str, std::ffi::OsString)> {
    ["WORMHOLE_LOG", "TMPDIR"].into_iter().filter_map(|key| Some((key, std::env::var_os(key)?))).collect()
}

/// `ok <pid>` or `err <text>`.
pub(super) fn parse_line(line: &str) -> Ready {
    if let Some(pid) = line.strip_prefix("ok ") {
        return match pid.trim().parse() {
            Ok(pid) => Ready::Ok(pid),
            Err(_) => Ready::Err(format!("the session host said {line:?}")),
        };
    }
    match line.strip_prefix("err ") {
        Some(text) => Ready::Err(text.to_owned()),
        None => Ready::Err(format!("the session host said {line:?}")),
    }
}
