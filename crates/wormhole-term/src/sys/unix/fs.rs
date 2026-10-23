use std::fs::{Metadata, OpenOptions};
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _};

pub(crate) fn mode(meta: &Metadata) -> u32 {
    meta.mode()
}

pub(crate) fn private_file(options: &mut OpenOptions) -> &mut OpenOptions {
    options.mode(0o600)
}
