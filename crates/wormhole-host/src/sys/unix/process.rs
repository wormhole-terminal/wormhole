use std::os::fd::AsRawFd as _;

/// Out of every folder, a private umask, and no core files.
#[allow(unsafe_code)] // libc FFI; constant arguments and a NUL-terminated path.
pub(crate) fn leave_folders_and_cores() {
    // SAFETY: chdir gets a NUL-terminated literal; umask and setrlimit take plain values and a
    // pointer to a local struct.
    unsafe {
        libc::chdir(c"/".as_ptr());
        libc::umask(0o077);
        let none = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
        libc::setrlimit(libc::RLIMIT_CORE, &none);
    }
}

/// Point standard input, output and error at `/dev/null`: nothing reads them, and a descriptor
/// 0 to 2 that is closed would be handed to the next socket.
#[allow(unsafe_code)] // libc FFI on descriptor numbers.
pub(crate) fn null_stdio() {
    let Ok(null) = std::fs::OpenOptions::new().read(true).write(true).open("/dev/null") else { return };
    for fd in 0..=2 {
        // SAFETY: dup2 onto a standard descriptor; `null` stays open until after the loop.
        unsafe { libc::dup2(null.as_raw_fd(), fd) };
    }
}

#[allow(unsafe_code)] // `remove_var` races with other threads reading the environment.
pub(crate) fn clear_environment() {
    let names: Vec<_> = std::env::vars_os().map(|(name, _)| name).collect();
    for name in names {
        // SAFETY: called first thing in `main`, before the host starts any thread, so nothing can
        // read the environment while it changes.
        unsafe { std::env::remove_var(name) };
    }
}
