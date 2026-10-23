//! Files the host keeps alive.

use std::io;
use std::path::Path;

use super::unix::fs as os;

/// Set a file's access and modification times to now (the long-path socket in `$TMPDIR` is kept
/// from the temporary-file cleaner by this).
pub fn touch(path: &Path) -> io::Result<()> {
    os::touch(path)
}
