use std::io;
use std::os::fd::AsRawFd;

/// The credentials of the process at the other end of a Unix socket.
#[allow(unsafe_code)] // libc FFI; the credentials go into a local struct of the size given.
fn peer_cred(socket: &impl AsRawFd) -> io::Result<libc::ucred> {
    let mut cred = libc::ucred { pid: 0, uid: 0, gid: 0 };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: getsockopt writes at most `len` bytes into the local struct; the descriptor is open.
    let rc = unsafe {
        libc::getsockopt(
            socket.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut cred as *mut libc::ucred).cast(),
            &mut len,
        )
    };
    if rc == 0 { Ok(cred) } else { Err(io::Error::last_os_error()) }
}

/// The effective uid of the process at the other end of a Unix socket.
pub(crate) fn peer_uid(socket: &impl AsRawFd) -> io::Result<u32> {
    peer_cred(socket).map(|cred| cred.uid)
}

/// The process that connected the socket (`SO_PEERCRED`).
pub(crate) fn peer_pid(socket: &impl AsRawFd) -> io::Result<u32> {
    peer_cred(socket).and_then(|cred| u32::try_from(cred.pid).map_err(|_| io::ErrorKind::InvalidData.into()))
}
