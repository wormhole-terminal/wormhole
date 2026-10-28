//! Reading the start of a file that may be larger than is wanted: what a viewer shows of a big file.

use std::io::{self, Read};
use std::path::Path;

use super::{LinkPolicy, open_regular};

/// The first bytes of a regular file, and how big the file was.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prefix {
    /// At most the cap.
    pub bytes: Vec<u8>,
    /// The file's size when it was opened.
    pub size: u64,
}

impl Prefix {
    /// The file has more than `bytes` holds.
    #[must_use]
    pub fn truncated(&self) -> bool {
        self.size > self.bytes.len() as u64
    }
}

/// Reads at most `cap` bytes from the start of a regular file; a larger file is read up to there, not
/// refused ([`super::read_capped`] refuses it). The same regular-file and link checks apply.
pub fn read_prefix_capped(path: &Path, cap: u64, policy: LinkPolicy<'_>) -> io::Result<Prefix> {
    let file = open_regular(path, policy)?;
    let size = file.metadata()?.len();
    let mut bytes = Vec::with_capacity(usize::try_from(size.min(cap).min(1 << 20)).unwrap_or(0));
    file.take(cap).read_to_end(&mut bytes)?;
    Ok(Prefix { bytes, size })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{LinkPolicy, read_prefix_capped};

    #[test]
    fn a_big_file_gives_its_start_and_its_size() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("big");
        fs::write(&file, b"0123456789").unwrap();
        let prefix = read_prefix_capped(&file, 4, LinkPolicy::Within(dir.path())).unwrap();
        assert_eq!((prefix.bytes.as_slice(), prefix.size, prefix.truncated()), (&b"0123"[..], 10, true));
        let whole = read_prefix_capped(&file, 10, LinkPolicy::Follow).unwrap();
        assert_eq!((whole.bytes.len(), whole.truncated()), (10, false));
    }

    #[test]
    fn a_folder_or_a_path_out_of_the_root_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        fs::write(other.path().join("f"), "x").unwrap();
        assert!(read_prefix_capped(dir.path(), 4, LinkPolicy::Follow).is_err());
        assert!(read_prefix_capped(&other.path().join("f"), 4, LinkPolicy::Within(dir.path())).is_err());
    }
}
