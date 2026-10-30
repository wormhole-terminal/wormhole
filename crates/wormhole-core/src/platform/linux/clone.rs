use std::io;
use std::path::Path;

/// No copy-on-write folder clone here yet.
pub(crate) fn clone_dir(_from: &Path, _to: &Path, _same_volume_as: &Path) -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "cloning a folder is only supported on macOS"))
}
