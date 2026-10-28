use std::ffi::CString;
use std::io;
use std::os::unix::ffi::OsStrExt as _;
use std::path::Path;

fn c_path(path: &Path) -> io::Result<CString> {
    CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidInput, format!("{} has a NUL in its name", path.display()))
    })
}

/// `renamex_np` with `RENAME_EXCL`: the system refuses with `EEXIST` when `to` exists.
pub(crate) fn rename_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    let (from, to) = (c_path(from)?, c_path(to)?);
    // SAFETY: both paths are NUL-terminated and outlive the call; `renamex_np` only reads them.
    #[allow(unsafe_code)]
    let rc = unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_EXCL) };
    if rc == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}
