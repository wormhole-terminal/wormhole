use std::ffi::{CStr, CString};
use std::io;
use std::os::unix::ffi::OsStrExt as _;
use std::path::Path;

/// Time Machine skips a file that carries this attribute (what `tmutil addexclusion` sets: it
/// follows the file, not its path, and needs no privileges).
const EXCLUDE_ATTR: &CStr = c"com.apple.metadata:com_apple_backup_excludeItem";

pub(crate) fn exclude(path: &Path) -> io::Result<()> {
    // The attribute's value is a binary property list holding the string Time Machine looks for.
    let mut value = Vec::new();
    plist::Value::String("com.apple.backupd".into())
        .to_writer_binary(&mut value)
        .map_err(|err| io::Error::other(format!("encoding the backup exclusion: {err}")))?;
    let c_path = CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidInput, format!("{} has a NUL in its name", path.display()))
    })?;
    // SAFETY: `c_path` and `EXCLUDE_ATTR` are NUL-terminated and outlive the call, `value` points
    // at `value.len()` readable bytes, and XATTR_NOFOLLOW keeps a link at `path` from being followed.
    #[allow(unsafe_code)]
    let rc = unsafe {
        libc::setxattr(
            c_path.as_ptr(),
            EXCLUDE_ATTR.as_ptr(),
            value.as_ptr().cast(),
            value.len(),
            0,
            libc::XATTR_NOFOLLOW,
        )
    };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub(crate) fn is_excluded(path: &Path) -> bool {
    let Ok(c_path) = CString::new(path.as_os_str().as_bytes()) else { return false };
    // SAFETY: a size query (null buffer) of an attribute by NUL-terminated names.
    #[allow(unsafe_code)]
    let size = unsafe {
        libc::getxattr(
            c_path.as_ptr(),
            EXCLUDE_ATTR.as_ptr(),
            std::ptr::null_mut(),
            0,
            0,
            libc::XATTR_NOFOLLOW,
        )
    };
    size > 0
}
