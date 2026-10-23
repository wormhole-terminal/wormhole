use std::fs::File;
use std::os::fd::{FromRawFd as _, OwnedFd, RawFd};

/// The descriptor the app hands the readiness pipe on.
const READY_FD: RawFd = 3;

/// Take descriptor 3 if it is open, and keep it from the program the host starts.
#[allow(unsafe_code)] // libc FFI on a descriptor number, and taking ownership of it.
pub(crate) fn take() -> Option<File> {
    // SAFETY: F_GETFD only asks about the descriptor; -1 means it is not open.
    let open = unsafe { libc::fcntl(READY_FD, libc::F_GETFD) } >= 0;
    if !open {
        return None;
    }
    // SAFETY: as above; FD_CLOEXEC keeps the shell from inheriting the pipe (it would hold
    // the app's read end open for as long as the shell runs).
    unsafe { libc::fcntl(READY_FD, libc::F_SETFD, libc::FD_CLOEXEC) };
    // SAFETY: descriptor 3 is open, and nothing else in this process owns it: it is taken
    // first thing at start-up, before anything opens a file.
    Some(File::from(unsafe { OwnedFd::from_raw_fd(READY_FD) }))
}
