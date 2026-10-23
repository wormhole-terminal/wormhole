use std::io;
use std::path::Path;

/// Only macOS has Time Machine.
pub(crate) fn exclude(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
pub(crate) fn is_excluded(_path: &Path) -> bool {
    false
}
