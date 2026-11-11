//! Ending a session host from its terminal's bar (`terminal_view/host_problem.rs`): `Kill` over its socket, then a
//! signal. `hard` is for a host that does not answer: a process checked to be the host is killed outright.

use std::time::Duration;

use wormhole_core::ipc::hosts;

use super::desk::HostedTerminals;

/// How long ending a host waits for it to take `Kill` before it is signalled.
const END_WAIT: Duration = Duration::from_secs(4);

#[uniffi::export]
impl HostedTerminals {
    /// End host `host_id`. `None` when it worked (the caller closes the terminal), else why it did not. Takes seconds:
    /// not on the main thread.
    pub fn end_host(&self, host_id: String, hard: bool) -> Option<String> {
        let ended = hosts::find(&self.hosts_dir, &host_id).and_then(|meta| {
            if hard {
                hosts::end_unresponsive_host(&meta, END_WAIT)
            } else {
                hosts::end_host(&meta, END_WAIT)
            }
        });
        ended.err()
    }
}
