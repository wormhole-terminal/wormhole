//! A polite `kill`: SIGTERM asks the app to quit as ⌘Q does (every window saved, session hosts
//! left running) instead of ending it on the spot, which lost the terminals' text since the last
//! minute's save. The handler only sets a flag; the app's tick reads it with [`take`].

use std::io;

use super::unix::signal as os;

/// Catch SIGTERM from now on. Programs the app starts don't inherit the handler: `exec` puts a
/// caught signal back to its default.
pub fn catch_sigterm() -> io::Result<()> {
    os::catch_sigterm()
}

/// Whether SIGTERM arrived since the last call.
pub fn take() -> bool {
    os::take()
}

/// End the process at once, with exit code 0, when ctrl-c (SIGINT) arrives: for a command line that
/// streams until it is stopped, where being stopped is how it is meant to end. Nothing is lost by
/// not unwinding: what it printed was written line by line.
pub fn exit_on_interrupt() -> io::Result<()> {
    os::exit_on_interrupt()
}
