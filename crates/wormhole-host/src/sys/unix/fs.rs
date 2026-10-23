use std::ffi::CString;
use std::io;
use std::os::unix::ffi::OsStrExt as _;
use std::path::Path;

#[allow(unsafe_code)] // libc FFI; a NUL-terminated path and a null times pointer.
pub(crate) fn touch(path: &Path) -> io::Result<()> {
    let path = CString::new(path.as_os_str().as_bytes()).map_err(|_| io::ErrorKind::InvalidInput)?;
    // SAFETY: the path is NUL-terminated and outlives the call; a null `times` means now.
    if unsafe { libc::utimes(path.as_ptr(), std::ptr::null()) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
