//! No watcher yet: the caller keeps polling.

use std::io;
use std::path::{Path, PathBuf};

pub(crate) struct Watcher;

pub(crate) fn watch(
    _roots: &[&Path],
    _on_batch: Box<dyn FnMut(Vec<PathBuf>, bool) + Send>,
) -> io::Result<Watcher> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "no file watcher on this system"))
}
