use std::fs::Metadata;
use std::os::unix::fs::MetadataExt as _;

#[allow(unsafe_code)] // libc FFI; no arguments.
pub(crate) fn effective_uid() -> u32 {
    // SAFETY: geteuid takes nothing and cannot fail.
    unsafe { libc::geteuid() }
}

pub(crate) fn owner(meta: &Metadata) -> u32 {
    meta.uid()
}
