//! What the system says of a process.

use std::path::PathBuf;

#[cfg(target_os = "linux")]
use super::linux::process as os;
#[cfg(target_os = "macos")]
use super::macos::process as os;

/// A process's current folder, or `None` when it can't be read (it ended, or isn't ours).
pub fn cwd(pid: u32) -> Option<PathBuf> {
    os::cwd(pid)
}
