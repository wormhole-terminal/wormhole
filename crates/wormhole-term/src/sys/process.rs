//! What the system says of a process: its folder, who leads its terminal's foreground, its command
//! line, its children and the TCP ports it listens on. A reading of a pid that is gone, or not ours to
//! read, is `None` or nothing, never an error: callers poll every few seconds and a process may end
//! between two of their calls.

use std::io;
use std::path::PathBuf;
use std::time::SystemTime;

use super::unix::process as unix;

#[cfg(target_os = "linux")]
use super::linux::process as os;
#[cfg(target_os = "macos")]
use super::macos::process as os;

/// Whether a process with this pid exists. One that exists but is not ours still counts: the pid is
/// taken.
pub fn exists(pid: u32) -> bool {
    unix::exists(pid)
}

/// Ask a process to end. Refuses pids 0 and 1, which would reach a process group or init: a pid read
/// from a file is never trusted that far.
pub fn terminate(pid: u32) -> io::Result<()> {
    unix::terminate(pid)
}

/// The path of a process's executable.
pub fn executable(pid: u32) -> Option<PathBuf> {
    os::executable(pid)
}

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

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    #[test]
    fn a_process_exists_when_it_runs_even_if_it_is_not_ours() {
        assert!(exists(std::process::id()));
        // Process 1 belongs to the system: signalling it is refused (EPERM) and it still counts.
        assert!(exists(1));
        assert!(!exists(0));
        assert!(!exists(u32::MAX), "a pid that does not fit the system's");
    }

    #[test]
    fn pids_0_and_1_are_never_signalled() {
        for pid in [0, 1, u32::MAX] {
            assert_eq!(terminate(pid).expect_err("refused").kind(), io::ErrorKind::InvalidInput, "{pid}");
        }
    }

    #[test]
    fn a_process_is_asked_to_end_and_its_executable_is_found() {
        let mut child = Command::new("/bin/sleep").arg("30").spawn().expect("spawns");
        let path = executable(child.id()).expect("an executable");
        assert_eq!(path.file_name().and_then(|n| n.to_str()), Some("sleep"));
        terminate(child.id()).expect("signalled");
        let status = child.wait().expect("ends");
        assert!(!status.success());
        assert!(executable(u32::MAX).is_none());
    }
}
