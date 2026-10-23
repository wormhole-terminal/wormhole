//! The TCP ports a process listens on, from libproc: one `proc_pidinfo` for its descriptors and one
//! `proc_pidfdinfo` for each of its sockets.

use std::ffi::c_void;

/// `PROC_PIDFDSOCKETINFO` from `<sys/proc_info.h>`.
const PROC_PIDFDSOCKETINFO: libc::c_int = 3;

/// The layout of `struct socket_fdinfo` (`<sys/proc_info.h>`) that matters here, as byte offsets
/// into what `proc_pidfdinfo(PROC_PIDFDSOCKETINFO)` fills in. The libc crate has no binding for
/// it; the offsets were taken with `offsetof` on the SDK's header, and a test listens on a socket
/// of its own and finds it, which fails if they ever move.
mod socket {
    /// `sizeof(struct socket_fdinfo)`: the buffer the call needs.
    pub(super) const SIZE: usize = 792;
    /// `psi.soi_kind`, an `int`: `SOCKINFO_TCP` for TCP over IPv4 or IPv6.
    pub(super) const KIND: usize = 256;
    pub(super) const KIND_TCP: i32 = 2;
    /// `psi.soi_proto.pri_tcp.tcpsi_ini.insi_lport`, an `int` holding the port in network byte
    /// order in its low 16 bits.
    pub(super) const LOCAL_PORT: usize = 268;
    /// `psi.soi_proto.pri_tcp.tcpsi_state`, an `int`: `TSI_S_LISTEN` is 1.
    pub(super) const STATE: usize = 344;
    pub(super) const STATE_LISTEN: i32 = 1;
}

/// The port a socket listens on, from what `proc_pidfdinfo` wrote (`raw`), or `None` for anything
/// that isn't a TCP socket in the listen state (or a buffer too short to say).
fn listener_port(raw: &[u8]) -> Option<u16> {
    let int = |offset: usize| -> Option<i32> {
        Some(i32::from_ne_bytes(raw.get(offset..offset + 4)?.try_into().ok()?))
    };
    if int(socket::KIND)? != socket::KIND_TCP || int(socket::STATE)? != socket::STATE_LISTEN {
        return None;
    }
    Some(u16::from_be(int(socket::LOCAL_PORT)? as u16)).filter(|&port| port != 0)
}

/// The ports `pid` listens on over TCP, looking at no more than `limit` descriptors.
#[allow(unsafe_code)] // libproc FFI; every buffer size is checked.
pub(crate) fn listening_tcp_ports(pid: u32, limit: usize) -> Vec<u16> {
    let mut ports = Vec::new();
    for fd in descriptors(pid, limit) {
        if fd.proc_fdtype != libc::PROX_FDTYPE_SOCKET as u32 {
            continue;
        }
        let mut raw = Raw([0; socket::SIZE]);
        // SAFETY: `raw` is `SIZE` writable bytes and the call is told so.
        let written = unsafe {
            libc::proc_pidfdinfo(
                pid as libc::c_int,
                fd.proc_fd,
                PROC_PIDFDSOCKETINFO,
                raw.0.as_mut_ptr().cast::<c_void>(),
                socket::SIZE as libc::c_int,
            )
        };
        if written > 0
            && let Some(port) = listener_port(&raw.0[..(written as usize).min(socket::SIZE)])
        {
            ports.push(port);
        }
    }
    ports
}

/// The buffer a socket's description is written into, aligned for the `int`s read from it.
#[repr(align(8))]
struct Raw([u8; socket::SIZE]);

#[allow(unsafe_code)] // libproc FFI; every buffer size is checked.
fn descriptors(pid: u32, limit: usize) -> Vec<libc::proc_fdinfo> {
    // SAFETY: a null buffer only asks how many bytes the list needs.
    let needed =
        unsafe { libc::proc_pidinfo(pid as libc::c_int, libc::PROC_PIDLISTFDS, 0, std::ptr::null_mut(), 0) };
    if needed <= 0 {
        return Vec::new();
    }
    // A little room for the descriptors opened between the two calls.
    let room = (needed as usize / size_of::<libc::proc_fdinfo>() + 8).min(limit);
    let mut fds = vec![libc::proc_fdinfo { proc_fd: 0, proc_fdtype: 0 }; room];
    let bytes = (room * size_of::<libc::proc_fdinfo>()) as libc::c_int;
    // SAFETY: the buffer holds `bytes` bytes; the call writes at most that much.
    let written = unsafe {
        libc::proc_pidinfo(
            pid as libc::c_int,
            libc::PROC_PIDLISTFDS,
            0,
            fds.as_mut_ptr().cast::<c_void>(),
            bytes,
        )
    };
    if written <= 0 {
        return Vec::new();
    }
    fds.truncate(written as usize / size_of::<libc::proc_fdinfo>());
    fds
}

#[cfg(test)]
mod tests {
    use super::{listener_port, socket};

    /// A socket description with the fields `listener_port` reads, as the kernel lays them out.
    fn raw(kind: i32, state: i32, port: u16) -> Vec<u8> {
        let mut bytes = vec![0u8; socket::SIZE];
        let mut put =
            |offset: usize, value: i32| bytes[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
        put(socket::KIND, kind);
        put(socket::STATE, state);
        put(socket::LOCAL_PORT, i32::from(port.to_be()));
        bytes
    }

    #[test]
    fn a_listening_tcp_socket_gives_its_port() {
        assert_eq!(listener_port(&raw(socket::KIND_TCP, socket::STATE_LISTEN, 3000)), Some(3000));
        assert_eq!(listener_port(&raw(socket::KIND_TCP, socket::STATE_LISTEN, 65535)), Some(65535));
    }

    #[test]
    fn nothing_else_is_a_port_to_open() {
        // An established connection (state 4), a UDP or unix socket, and port 0.
        assert_eq!(listener_port(&raw(socket::KIND_TCP, 4, 3000)), None);
        assert_eq!(listener_port(&raw(1, socket::STATE_LISTEN, 3000)), None);
        assert_eq!(listener_port(&raw(socket::KIND_TCP, socket::STATE_LISTEN, 0)), None);
        assert_eq!(listener_port(&[0; 16]), None, "a buffer too short to say");
        assert_eq!(listener_port(&[]), None);
    }
}
