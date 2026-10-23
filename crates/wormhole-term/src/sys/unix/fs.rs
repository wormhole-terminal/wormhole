use std::fs::{DirBuilder, FileType, Metadata, OpenOptions, Permissions};
use std::io;
use std::os::unix::fs::{
    DirBuilderExt as _, FileTypeExt as _, MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _,
};
use std::path::Path;

pub(crate) fn mode(meta: &Metadata) -> u32 {
    meta.mode()
}

pub(crate) fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    std::fs::set_permissions(path, Permissions::from_mode(mode))
}

pub(crate) fn inode(meta: &Metadata) -> u64 {
    meta.ino()
}

pub(crate) fn is_socket(kind: &FileType) -> bool {
    kind.is_socket()
}

pub(crate) fn private_dir(builder: &mut DirBuilder) -> &mut DirBuilder {
    builder.mode(0o700)
}

pub(crate) fn private_file(options: &mut OpenOptions) -> &mut OpenOptions {
    options.mode(0o600)
}

pub(crate) fn symlink(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}
