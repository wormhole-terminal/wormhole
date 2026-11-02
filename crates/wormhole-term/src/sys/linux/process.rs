//! `/proc` for a process's folder. The rest of what macOS reads from libproc has no Linux reading
//! yet: those answer "nothing known", which is what the app showed on Linux before the edge moved
//! (no agent, no foreground command, no ports). A port writes them from `/proc`.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

pub(crate) fn cwd(pid: u32) -> Option<PathBuf> {
    std::fs::read_link(format!("/proc/{pid}/cwd")).ok()
}

pub(crate) fn executable(pid: u32) -> Option<PathBuf> {
    std::fs::read_link(format!("/proc/{pid}/exe")).ok()
}

pub(crate) fn foreground_group(_shell_pid: u32) -> Option<u32> {
    None
}

pub(crate) fn parent(_pid: u32) -> Option<u32> {
    None
}

/// [`start_time`] for a process of another user: `/proc` shows the same to everyone.
pub(crate) fn start_time_any_user(pid: u32) -> Option<SystemTime> {
    start_time(pid)
}

/// When the process started: its start in clock ticks after boot (`/proc/<pid>/stat`, field 22)
/// from the boot time (`btime` in `/proc/stat`).
pub(crate) fn start_time(pid: u32) -> Option<SystemTime> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // The name (field 2) may hold spaces and parentheses: fields count from the last `)`.
    let after_name = stat.get(stat.rfind(')')? + 1..)?;
    let ticks: u64 = after_name.split_whitespace().nth(19)?.parse().ok()?;
    let btime: u64 = std::fs::read_to_string("/proc/stat")
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("btime ")?.trim().parse().ok())?;
    let per_second = clock_ticks_per_second()?;
    SystemTime::UNIX_EPOCH
        .checked_add(Duration::from_secs(btime) + Duration::from_secs_f64(ticks as f64 / per_second))
}

#[allow(unsafe_code)] // sysconf takes a constant and returns a number.
fn clock_ticks_per_second() -> Option<f64> {
    // SAFETY: `sysconf` reads nothing of ours.
    let ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    (ticks > 0).then_some(ticks as f64)
}

pub(crate) fn tty(_pid: u32) -> Option<u32> {
    None
}

pub(crate) fn group_members(_pgid: u32) -> Vec<u32> {
    Vec::new()
}

pub(crate) fn environment_names(_pid: u32) -> Option<Vec<String>> {
    None
}

pub(crate) fn command_line(_pid: u32) -> Option<(String, Vec<String>)> {
    None
}

pub(crate) fn children(_pid: u32) -> Vec<u32> {
    Vec::new()
}

pub(crate) fn listening_tcp_ports(_pid: u32, _limit: usize) -> Vec<u16> {
    Vec::new()
}

pub(crate) fn command_name(pid: u32) -> Option<String> {
    std::fs::read_to_string(format!("/proc/{pid}/comm")).ok().map(|name| name.trim_end().to_string())
}

/// The effective user id (the second of `Uid:` in `/proc/<pid>/status`).
pub(crate) fn owner(pid: u32) -> Option<u32> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    status.lines().find_map(|line| line.strip_prefix("Uid:")?.split_whitespace().nth(1)?.parse().ok())
}

pub(crate) fn resident_bytes(_pid: u32) -> Option<u64> {
    None
}
