use std::fs::{Metadata, OpenOptions};
use std::io;
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _};
use std::path::Path;

pub(crate) fn mode(meta: &Metadata) -> u32 {
    meta.mode()
}

pub(crate) fn private_file(options: &mut OpenOptions) -> &mut OpenOptions {
    options.mode(0o600)
}

pub(crate) fn symlink(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}
