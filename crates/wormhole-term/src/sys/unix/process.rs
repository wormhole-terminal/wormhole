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

/// How to end a process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndSignal {
    /// Ask it to end (`SIGTERM`): a process that is stopped or stuck keeps it pending.
    Terminate,
    /// End it at once (`SIGKILL`), which nothing can ignore.
    Kill,
}

/// Send `how` to the process. Refuses pid 0 and 1, which would signal a process group or init: a pid
/// read from a file is never trusted that far.
#[allow(unsafe_code)] // libc FFI; plain integers.
pub(crate) fn signal(pid: u32, how: EndSignal) -> io::Result<()> {
    let pid = i32::try_from(pid).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "pid"))?;
    if pid <= 1 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a host's pid"));
    }
    let signal = match how {
        EndSignal::Terminate => libc::SIGTERM,
        EndSignal::Kill => libc::SIGKILL,
    };
    // SAFETY: kill with a positive pid signals that one process; the pid was checked above.
    let result = unsafe { libc::kill(pid, signal) };
    if result == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}
