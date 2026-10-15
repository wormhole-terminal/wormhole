//! Learning that the child has exited without waiting for its terminal to say so.
//!
//! The master reports the end of the child's output only when every holder of the slave has
//! closed it, and a shell that leaves a background job behind (a job that ignores the hangup,
//! `trap '' HUP; sleep 100 &`) exits while the job still holds the slave. The system can say when
//! a process exits, though: kqueue's `EVFILT_PROC` with `NOTE_EXIT`, which the reader's poller
//! takes next to the master.
//!
//! Elsewhere there is no such filter in the poller, so the reader looks at the child with
//! `try_wait` every [`POLL_EVERY`] while it waits, which is late by at most that.

use std::process::Child;
use std::time::Duration;

use polling::Poller;

use super::master::Key;

/// How often the reader looks for the child's exit where the poller cannot tell it.
pub(super) const POLL_EVERY: Duration = Duration::from_millis(100);

/// Who tells the reader that the child exited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Watch {
    /// The poller reports it as an event.
    ByPoller,
    /// Nothing does: the reader looks every [`POLL_EVERY`].
    ByPolling,
}

/// Have `poller` report [`Key::Exit`] when `child` exits, if it can.
#[cfg(target_os = "macos")]
#[allow(unsafe_code)] // polling's kqueue filter for a child takes the child's pid on trust.
pub(super) fn watch(poller: &Poller, child: &Child) -> Watch {
    use polling::PollMode;
    use polling::os::kqueue::{PollerKqueueExt as _, Process, ProcessOps};
    // SAFETY: the reader owns the child and takes the filter out again (`unwatch`) before the
    // child is dropped, so the registration never names a pid that is no longer ours.
    let filter = unsafe { Process::new(child, ProcessOps::Exit) };
    match poller.add_filter(filter, Key::Exit.raw(), PollMode::Oneshot) {
        Ok(()) => Watch::ByPoller,
        Err(err) => {
            tracing::debug!("cannot watch the child for its exit, polling for it: {err}");
            Watch::ByPolling
        }
    }
}

/// Stop watching `child`: the registration of [`watch`].
#[cfg(target_os = "macos")]
#[allow(unsafe_code)] // as `watch`.
pub(super) fn unwatch(poller: &Poller, child: &Child) {
    use polling::os::kqueue::{PollerKqueueExt as _, Process, ProcessOps};
    // SAFETY: as in `watch`; the child is still ours.
    let filter = unsafe { Process::new(child, ProcessOps::Exit) };
    // Already gone once the exit was reported (the registration is one-shot), which is fine.
    let _ = poller.delete_filter(filter);
}

/// Where the poller has no filter for a process, the reader polls.
#[cfg(not(target_os = "macos"))]
pub(super) fn watch(_poller: &Poller, _child: &Child) -> Watch {
    Watch::ByPolling
}

/// Nothing was registered.
#[cfg(not(target_os = "macos"))]
pub(super) fn unwatch(_poller: &Poller, _child: &Child) {}
