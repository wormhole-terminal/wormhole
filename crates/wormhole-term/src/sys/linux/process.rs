//! `/proc` for a process's folder. The rest of what macOS reads from libproc has no Linux reading
//! yet: those answer "nothing known", which is what the app showed on Linux before the edge moved
//! (no agent, no foreground command, no ports). A port writes them from `/proc`.

use std::path::PathBuf;
use std::time::SystemTime;

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

pub(crate) fn start_time(_pid: u32) -> Option<SystemTime> {
    None
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
