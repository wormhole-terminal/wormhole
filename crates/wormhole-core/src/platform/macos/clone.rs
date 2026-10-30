use std::ffi::{CStr, CString};
use std::io;
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::fs::MetadataExt as _;
use std::path::Path;

/// `COPYFILE_STATE_FORBID_CROSS_MOUNT` of `<copyfile.h>` (a `bool`: do not descend into a directory on
/// another device during a recursive copy), which `libc` lacks.
const COPYFILE_STATE_FORBID_CROSS_MOUNT: libc::c_int = 14;

fn c_path(path: &Path) -> io::Result<CString> {
    CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidInput, format!("{} has a NUL in its name", path.display()))
    })
}

/// The device of `path` when it is on an APFS volume (the only one here that clones), else `None`.
fn apfs_volume(path: &Path) -> io::Result<Option<u64>> {
    let c = c_path(path)?;
    let mut stat = std::mem::MaybeUninit::<libc::statfs>::uninit();
    // SAFETY: `c` is NUL-terminated and outlives the call, and `stat` is the size statfs writes.
    #[allow(unsafe_code)]
    let rc = unsafe { libc::statfs(c.as_ptr(), stat.as_mut_ptr()) };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: statfs returned 0, so it filled the whole struct.
    #[allow(unsafe_code)]
    let stat = unsafe { stat.assume_init() };
    // SAFETY: `f_fstypename` is a NUL-terminated array filled by statfs.
    #[allow(unsafe_code)]
    let name = unsafe { CStr::from_ptr(stat.f_fstypename.as_ptr()) };
    if name.to_bytes() != b"apfs" {
        return Ok(None);
    }
    Ok(Some(std::fs::metadata(path)?.dev()))
}

/// A `copyfile_state_t` that is freed when it goes out of scope, on every path.
struct State(libc::copyfile_state_t);

impl State {
    /// A state that makes a recursive copy stay on the device it began on.
    fn forbidding_cross_mount() -> io::Result<Self> {
        // SAFETY: allocates; a null return is checked below.
        #[allow(unsafe_code)]
        let raw = unsafe { libc::copyfile_state_alloc() };
        if raw.is_null() {
            return Err(io::Error::last_os_error());
        }
        let state = Self(raw);
        let forbid = true;
        // SAFETY: `state.0` is a live state, and for this flag the value is a pointer to a `bool`, which
        // `forbid` is and which outlives the call (the value is copied into the state).
        #[allow(unsafe_code)]
        let rc = unsafe {
            libc::copyfile_state_set(
                state.0,
                COPYFILE_STATE_FORBID_CROSS_MOUNT as u32,
                std::ptr::from_ref(&forbid).cast(),
            )
        };
        if rc == 0 { Ok(state) } else { Err(io::Error::last_os_error()) }
    }
}

impl Drop for State {
    fn drop(&mut self) {
        // SAFETY: the state came from `copyfile_state_alloc` and is freed once, here.
        #[allow(unsafe_code)]
        unsafe {
            libc::copyfile_state_free(self.0);
        }
    }
}

/// `copyfile(3)` with `COPYFILE_CLONE | COPYFILE_ACL | COPYFILE_RECURSIVE | COPYFILE_NOFOLLOW`.
/// `COPYFILE_CLONE` clones each item with `clonefile(2)`, falling back to copying the bytes for an item
/// that can't be cloned, and implies `COPYFILE_EXCL | COPYFILE_STAT | COPYFILE_XATTR | COPYFILE_DATA |
/// COPYFILE_NOFOLLOW_SRC` (so permissions, owners, times and extended attributes, such as the executable
/// bits of `node_modules/.bin`, come along); `COPYFILE_ACL` adds the ACLs. `COPYFILE_RECURSIVE` takes the
/// whole tree (and, with the implied `EXCL`, needs a `to` that does not exist, not an existing folder to
/// nest in); `COPYFILE_NOFOLLOW` adds `NOFOLLOW_DST` to the implied `NOFOLLOW_SRC`, so links are copied as
/// links. `COPYFILE_STATE_FORBID_CROSS_MOUNT` keeps a volume mounted inside the tree from being copied.
///
/// `COPYFILE_CLONE_FORCE`, which would fail instead of copying, is refused by `copyfile` together with
/// `COPYFILE_RECURSIVE` (`EINVAL`), so it is checked beforehand that `from`, the parent of `to` and
/// `same_volume_as` are on one APFS volume, the case where cloning works; otherwise this fails with
/// [`io::ErrorKind::Unsupported`] and copies nothing.
pub(crate) fn clone_dir(from: &Path, to: &Path, same_volume_as: &Path) -> io::Result<()> {
    let to_parent = to.parent().ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    // Not `io::Error::from_raw_os_error(ENOTSUP)`: Rust maps that to `ErrorKind::Uncategorized` (only
    // `EOPNOTSUPP` becomes `Unsupported`), and callers tell "this can't work here" by the kind.
    let refused =
        || io::Error::new(io::ErrorKind::Unsupported, "the folders are not all on the same APFS volume");
    let Some(volume) = apfs_volume(from)? else { return Err(refused()) };
    for other in [to_parent, same_volume_as] {
        if apfs_volume(other)? != Some(volume) {
            return Err(refused());
        }
    }
    let (from, to) = (c_path(from)?, c_path(to)?);
    let state = State::forbidding_cross_mount()?;
    let flags =
        libc::COPYFILE_CLONE | libc::COPYFILE_ACL | libc::COPYFILE_RECURSIVE | libc::COPYFILE_NOFOLLOW;
    // SAFETY: both paths are NUL-terminated and outlive the call, which only reads them, and `state` is a
    // live state that is freed only after the call.
    #[allow(unsafe_code)]
    let rc = unsafe { libc::copyfile(from.as_ptr(), to.as_ptr(), state.0, flags) };
    if rc == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}
