use std::io;

/// Whether a process with this pid exists: signal 0 checks without sending anything. `EPERM` means
/// it exists but is not ours, which still counts (the pid is taken).
#[allow(unsafe_code)] // libc FFI; plain integers.
pub(crate) fn exists(pid: u32) -> bool {
    let Ok(pid) = i32::try_from(pid) else { return false };
    if pid <= 0 {
        return false;
    }
    // SAFETY: kill with signal 0 only checks that the process exists and may be signalled.
    let result = unsafe { libc::kill(pid, 0) };
    result == 0 || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// Ask the process to end (`SIGTERM`). Refuses pid 0 and 1, which would signal a process group or init.
#[allow(unsafe_code)] // libc FFI; plain integers.
pub(crate) fn terminate(pid: u32) -> io::Result<()> {
    let pid = i32::try_from(pid).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "pid"))?;
    if pid <= 1 {
        // 0 and 1 would signal a process group or init; never from a file's say-so.
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a host's pid"));
    }
    // SAFETY: kill with a positive pid signals that one process; the pid was checked above.
    let result = unsafe { libc::kill(pid, libc::SIGTERM) };
    if result == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}
