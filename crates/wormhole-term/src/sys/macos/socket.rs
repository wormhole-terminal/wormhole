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
