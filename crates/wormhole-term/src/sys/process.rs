//! What the system says of a process: its folder, who leads its terminal's foreground, its command
//! line, its children and the TCP ports it listens on. A pid that is gone, or not ours to read, gives
//! `None` or nothing, never an error: callers poll every few seconds and a process may end between
//! two of their calls.

use std::path::PathBuf;
use std::time::SystemTime;

#[cfg(target_os = "linux")]
use super::linux::process as os;
#[cfg(target_os = "macos")]
use super::macos::process as os;

/// A process's current folder.
pub fn cwd(pid: u32) -> Option<PathBuf> {
    os::cwd(pid)
}

/// The process group that owns the terminal of the process `shell_pid` right now.
pub fn foreground_group(shell_pid: u32) -> Option<u32> {
    os::foreground_group(shell_pid)
}

/// The process that started `pid`.
pub fn parent(pid: u32) -> Option<u32> {
    os::parent(pid)
}

/// When the process started.
pub fn start_time(pid: u32) -> Option<SystemTime> {
    os::start_time(pid)
}

/// The device of the process's controlling terminal.
pub fn tty(pid: u32) -> Option<u32> {
    os::tty(pid)
}

/// The processes in the group `pgid`.
pub fn group_members(pgid: u32) -> Vec<u32> {
    os::group_members(pgid)
}

/// A process's executable path and arguments.
pub fn command_line(pid: u32) -> Option<(String, Vec<String>)> {
    os::command_line(pid)
}

/// The processes `pid` started and that still run.
pub fn children(pid: u32) -> Vec<u32> {
    os::children(pid)
}

/// The ports `pid` has a TCP socket listening on, looking at no more than `limit` descriptors.
pub fn listening_tcp_ports(pid: u32, limit: usize) -> Vec<u16> {
    os::listening_tcp_ports(pid, limit)
}
