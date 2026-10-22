//! What the saved text asks of the operating system, behind one trait: tests use a fake that
//! records the calls (and never reaches the system), and the calls can move to the app's platform
//! edge as they are.

use std::path::Path;

use anyhow::Result;

use super::cipher::Key;
use super::keys;

/// The operating system's part in keeping the saved text.
pub trait Platform: Send + Sync {
    /// The key the text of the profile in `dir` is sealed with, made and kept on first use (see
    /// `keys`). An error (no Keychain, access denied, a locked Keychain) means the text isn't kept:
    /// it is never written unsealed.
    fn text_key(&self, dir: &Path) -> Result<Key>;
    /// Keep `path` out of backups (Time Machine). A failure is only worth a warning: the file is
    /// still the user's alone.
    fn exclude_from_backup(&self, path: &Path) -> Result<()>;
}

/// The real thing: the Keychain in a release build, a key file in a development build.
pub struct System;

impl Platform for System {
    fn text_key(&self, dir: &Path) -> Result<Key> {
        if cfg!(debug_assertions) { keys::dev_key_file(dir) } else { keys::keychain(dir) }
    }

    fn exclude_from_backup(&self, path: &Path) -> Result<()> {
        exclude_from_backup(path)
    }
}

/// Time Machine skips a file that carries this attribute (what `tmutil addexclusion` sets: it
/// follows the file, not its path, and needs no privileges).
#[cfg(target_os = "macos")]
const EXCLUDE_ATTR: &std::ffi::CStr = c"com.apple.metadata:com_apple_backup_excludeItem";

#[cfg(target_os = "macos")]
fn exclude_from_backup(path: &Path) -> Result<()> {
    use std::os::unix::ffi::OsStrExt as _;

    use anyhow::Context as _;

    // The attribute's value is a binary property list holding the string Time Machine looks for.
    let mut value = Vec::new();
    plist::Value::String("com.apple.backupd".into())
        .to_writer_binary(&mut value)
        .context("encoding the backup exclusion")?;
    let c_path = std::ffi::CString::new(path.as_os_str().as_bytes())
        .with_context(|| format!("{} has a NUL in its name", path.display()))?;
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
        return Err(std::io::Error::last_os_error())
            .with_context(|| format!("excluding {} from backups", path.display()));
    }
    Ok(())
}

/// Only macOS has Time Machine.
#[cfg(not(target_os = "macos"))]
fn exclude_from_backup(_: &Path) -> Result<()> {
    Ok(())
}

/// Whether `path` carries the exclusion (for the test that the real call works).
#[cfg(all(test, target_os = "macos"))]
pub(crate) fn is_excluded_from_backup(path: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt as _;
    let Ok(c_path) = std::ffi::CString::new(path.as_os_str().as_bytes()) else { return false };
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
