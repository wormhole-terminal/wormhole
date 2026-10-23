//! The user's home folder, for showing a path as `~/Projects/app`.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// The user's home folder, read from the environment once: asking costs an environment lock and
/// an allocation, and terminal names are worked out on every frame.
pub(crate) fn home_dir() -> Option<&'static Path> {
    static HOME: LazyLock<Option<PathBuf>> = LazyLock::new(|| std::env::var_os("HOME").map(PathBuf::from));
    HOME.as_deref()
}
