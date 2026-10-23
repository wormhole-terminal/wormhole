//! The signals that end the host. They are blocked in every thread and received by one thread that
//! waits for them, so none runs a handler in the middle of the driver.

use super::unix::signals as os;

/// Why the host is asked to end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ending {
    /// The app, a user or the system asked it to stop (`SIGTERM`).
    Terminate,
    /// An interrupt from the terminal it was started in (`SIGINT`).
    Interrupt,
}

/// Block the ending signals in this thread and every thread it starts afterwards, so only
/// [`wait_ending_signal`] receives them. A hangup is blocked too and ignored: the host has no
/// terminal to lose.
pub fn block_ending_signals() {
    os::block_ending_signals();
}

/// Wait for a blocked signal; the ending it asks for, or `None` for one the host ignores and for a
/// wait that failed.
pub fn wait_ending_signal() -> Option<Ending> {
    os::wait_ending_signal()
}
