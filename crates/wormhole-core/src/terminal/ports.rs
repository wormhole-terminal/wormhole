//! The TCP ports a terminal's programs listen on (the title row's port chips):
//! the shell's process tree, each process's open sockets, and the ones that are listening.
//!
//! libproc only, no process is started and nothing is connected to. A poll is one
//! `proc_listpids` per process for the tree, one `proc_pidinfo` for each one's descriptors and one
//! `proc_pidfdinfo` for each of its sockets: a shell with a dev server under it is a few dozen
//! calls. It is meant for a background thread every few seconds, for the terminal on screen.

use std::collections::BTreeSet;

/// The most processes looked at under one shell, and sockets looked at in one process: a fork bomb
/// or a process holding a hundred thousand descriptors must not make a poll expensive.
const MAX_PROCESSES: usize = 256;
const MAX_FDS: usize = 4096;

/// The ports the processes under the shell `shell_pid` (the shell itself included) have a TCP
/// socket listening on, lowest first, each once. A listener on any address counts: the chip opens
/// `localhost`, which is where a development server is looked for.
pub fn listening_ports(shell_pid: u32) -> Vec<u16> {
    let ports: BTreeSet<u16> = sys::tree(shell_pid, MAX_PROCESSES)
        .into_iter()
        .flat_map(|pid| sys::listening(pid, MAX_FDS))
        .collect();
    ports.into_iter().collect()
}

/// The layout of `struct socket_fdinfo` (`<sys/proc_info.h>`) that matters here, as byte offsets
/// into what `proc_pidfdinfo(PROC_PIDFDSOCKETINFO)` fills in. The libc crate has no binding for
/// it; the offsets were taken with `offsetof` on the SDK's header, and a test on macOS listens on
/// a socket of its own and finds it, which fails if they ever move. Only macOS reads sockets this way;
/// the tests check the parsing everywhere.
#[cfg(any(target_os = "macos", test))]
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
#[cfg(any(target_os = "macos", test))]
fn listener_port(raw: &[u8]) -> Option<u16> {
    let int = |offset: usize| -> Option<i32> {
        Some(i32::from_ne_bytes(raw.get(offset..offset + 4)?.try_into().ok()?))
    };
    if int(socket::KIND)? != socket::KIND_TCP || int(socket::STATE)? != socket::STATE_LISTEN {
        return None;
    }
    Some(u16::from_be(int(socket::LOCAL_PORT)? as u16)).filter(|&port| port != 0)
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)] // libproc FFI; every buffer size is checked.
mod sys {
    use std::collections::HashSet;
    use std::ffi::c_void;

    use super::{listener_port, socket};

    /// `PROC_PPID_ONLY` from `<libproc.h>` (not exported by the libc crate).
    const PROC_PPID_ONLY: u32 = 6;
    /// `PROC_PIDFDSOCKETINFO` from `<sys/proc_info.h>`.
    const PROC_PIDFDSOCKETINFO: libc::c_int = 3;

    /// `pid` and everything started under it, as far as `limit` allows, `pid` first.
    pub fn tree(pid: u32, limit: usize) -> Vec<u32> {
        let mut seen: HashSet<u32> = HashSet::from([pid]);
        let mut all = vec![pid];
        let mut next = 0;
        while let Some(&parent) = all.get(next) {
            next += 1;
            for child in children(parent) {
                if all.len() < limit && seen.insert(child) {
                    all.push(child);
                }
            }
        }
        all
    }

    fn children(pid: u32) -> Vec<u32> {
        let mut pids = vec![0 as libc::c_int; 128];
        let bytes = (pids.len() * size_of::<libc::c_int>()) as libc::c_int;
        // SAFETY: the buffer holds `bytes` bytes; the call writes at most that much.
        let written =
            unsafe { libc::proc_listpids(PROC_PPID_ONLY, pid, pids.as_mut_ptr().cast::<c_void>(), bytes) };
        if written <= 0 {
            return Vec::new();
        }
        let count = (written as usize / size_of::<libc::c_int>()).min(pids.len());
        pids[..count].iter().filter(|&&child| child > 0).map(|&child| child as u32).collect()
    }

    /// The ports `pid` listens on over TCP, looking at no more than `limit` descriptors.
    pub fn listening(pid: u32, limit: usize) -> Vec<u16> {
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

    fn descriptors(pid: u32, limit: usize) -> Vec<libc::proc_fdinfo> {
        // SAFETY: a null buffer only asks how many bytes the list needs.
        let needed = unsafe {
            libc::proc_pidinfo(pid as libc::c_int, libc::PROC_PIDLISTFDS, 0, std::ptr::null_mut(), 0)
        };
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
}

#[cfg(not(target_os = "macos"))]
mod sys {
    pub fn tree(_: u32, _: usize) -> Vec<u32> {
        Vec::new()
    }
    pub fn listening(_: u32, _: usize) -> Vec<u16> {
        Vec::new()
    }
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

    #[cfg(target_os = "macos")]
    mod libproc {
        use std::net::TcpListener;
        use std::process::{Command, Stdio};

        use super::super::{listening_ports, sys};

        /// The offsets above are the SDK's: a socket of our own is found by the real calls.
        #[test]
        fn a_listener_of_this_process_is_found_and_a_closed_one_is_not() {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
            let port = listener.local_addr().expect("address").port();
            assert!(sys::listening(std::process::id(), 4096).contains(&port));
            drop(listener);
            assert!(!sys::listening(std::process::id(), 4096).contains(&port));
        }

        /// A connection that is open is not a listener, on either end.
        #[test]
        fn a_connected_socket_is_not_listening() {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
            let port = listener.local_addr().expect("address").port();
            let client = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
            let (_server_side, _) = listener.accept().expect("accept");
            let client_port = client.local_addr().expect("address").port();
            let found = sys::listening(std::process::id(), 4096);
            assert!(found.contains(&port) && !found.contains(&client_port), "{found:?}");
        }

        /// The tree is the shell's children and theirs, and no more.
        #[test]
        fn the_tree_follows_children_of_children() {
            // sh runs a subshell that runs sleep: sleep is a grandchild.
            let mut shell = Command::new("/bin/sh")
                .args(["-c", "(sleep 30; true) & wait"])
                .stdout(Stdio::null())
                .spawn()
                .expect("spawn");
            let shell_pid = shell.id();
            let mut grandchildren = Vec::new();
            for _ in 0..100 {
                grandchildren = sys::tree(shell_pid, 256);
                if grandchildren.len() >= 3 {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            let _ = shell.kill();
            let _ = shell.wait();
            assert_eq!(grandchildren[0], shell_pid, "the shell first");
            assert!(grandchildren.len() >= 3, "{grandchildren:?}");
            // The limit holds however deep or wide it goes.
            assert!(sys::tree(std::process::id(), 1).len() == 1);
        }

        /// What a poll costs: the test run's own tree (a shell's is smaller).
        /// `cargo test -p wormhole-core poll_cost -- --ignored --nocapture`
        #[test]
        #[ignore = "benchmark"]
        fn poll_cost() {
            let _listener = TcpListener::bind("127.0.0.1:0").expect("bind");
            let started = std::time::Instant::now();
            let tree = sys::tree(std::process::id(), 256);
            let ports = listening_ports(std::process::id());
            eprintln!("{} processes, {ports:?}: {:?}", tree.len(), started.elapsed());
        }

        #[test]
        fn a_listener_under_this_process_is_found_through_the_tree() {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
            let port = listener.local_addr().expect("address").port();
            assert!(listening_ports(std::process::id()).contains(&port));
            assert!(listening_ports(u32::MAX - 1).is_empty(), "a process that isn't there");
        }
    }
}
