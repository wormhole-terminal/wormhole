use std::io;
use std::os::fd::AsRawFd;

/// The effective uid of the process at the other end of a Unix socket.
#[allow(unsafe_code)] // libc FFI; the credentials go into a local struct of the size given.
pub(crate) fn peer_uid(socket: &impl AsRawFd) -> io::Result<u32> {
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
    if rc == 0 { Ok(cred.uid) } else { Err(io::Error::last_os_error()) }
}
