//! Keeping a file out of the system's backups (Time Machine on macOS).

use std::io;
use std::path::Path;

#[cfg(target_os = "linux")]
use super::linux::backup as os;
#[cfg(target_os = "macos")]
use super::macos::backup as os;

/// Keep `path` out of backups. It follows the file, not its path, and needs no privileges.
pub fn exclude(path: &Path) -> io::Result<()> {
    os::exclude(path)
}

/// Whether `path` is kept out of backups (for the test that the real call works).
pub fn is_excluded(path: &Path) -> bool {
    os::is_excluded(path)
}
