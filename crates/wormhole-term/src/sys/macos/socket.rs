use std::io;
use std::os::fd::AsRawFd;

/// The effective uid of the process at the other end of a Unix socket.
#[allow(unsafe_code)] // libc FFI; the descriptor is open for the call, the outputs are locals.
pub(crate) fn peer_uid(socket: &impl AsRawFd) -> io::Result<u32> {
    let (mut uid, mut gid) = (0, 0);
    // SAFETY: getpeereid writes two integers into locals; the descriptor is borrowed and open.
    if unsafe { libc::getpeereid(socket.as_raw_fd(), &mut uid, &mut gid) } == 0 {
        Ok(uid)
    } else {
        Err(io::Error::last_os_error())
    }
}

/// The process that connected the socket, as the system recorded it then (`LOCAL_PEERPID`).
#[allow(unsafe_code)] // libc FFI; the pid goes into a local of the size given.
pub(crate) fn peer_pid(socket: &impl AsRawFd) -> io::Result<u32> {
    let mut pid: libc::pid_t = 0;
    let mut len = std::mem::size_of::<libc::pid_t>() as libc::socklen_t;
    // SAFETY: getsockopt writes at most `len` bytes into the local integer; the descriptor is open.
    let rc = unsafe {
        libc::getsockopt(
            socket.as_raw_fd(),
            libc::SOL_LOCAL,
            libc::LOCAL_PEERPID,
            (&mut pid as *mut libc::pid_t).cast(),
            &mut len,
        )
    };
    if rc == 0 {
        u32::try_from(pid).map_err(|_| io::ErrorKind::InvalidData.into())
    } else {
        Err(io::Error::last_os_error())
    }
}
