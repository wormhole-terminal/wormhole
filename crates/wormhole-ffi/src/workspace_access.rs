//! Where a program runs, for the agent access sheet's "Runs in <terminal>" line (`agent_access/sheet.rs`
//! `terminal_of`): the terminal whose shell the program is under, by its place in the process tree.

use wormhole_core::ipc::{ANCESTORS_MAX, Peer};
use wormhole_term::sys::process::ancestors;

use crate::workspace::Workspace;

#[uniffi::export]
impl Workspace {
    /// The id of the terminal of this workspace that process `pid` runs inside of, if any.
    pub fn terminal_running(&self, pid: u32) -> Option<i64> {
        let peer = Peer::new(pid, ancestors(pid, ANCESTORS_MAX));
        self.shared()
            .registered()
            .into_iter()
            .find(|(_, terminal)| terminal.shell_pid().is_some_and(|shell| peer.runs_in(shell)))
            .map(|(id, _)| id)
    }
}
