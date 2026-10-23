//! What core asks of processes: [`wormhole_term::sys::process`] and what only core needs.

pub use wormhole_term::sys::process::*;

use std::io;
use std::process::{Command, ExitStatus};

use super::unix::process as os;

/// Replace this process with the command; only returns, with the reason, if that failed.
pub fn exec(command: &mut Command) -> io::Error {
    os::exec(command)
}

/// Start the command in a process group of its own, so a signal to ours (Ctrl-C in a dev terminal)
/// doesn't reach it.
pub fn own_process_group(command: &mut Command) -> &mut Command {
    os::own_process_group(command)
}

/// The exit status a system's raw wait status stands for (tests build the ones a command can't be
/// made to produce).
pub fn exit_status_from_raw(raw: i32) -> ExitStatus {
    os::exit_status_from_raw(raw)
}
