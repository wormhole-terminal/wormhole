//! The TCP ports a terminal's programs listen on (the title row's port chips):
//! the shell's process tree, each process's open sockets, and the ones that are listening.
//!
//! Nothing is started and nothing is connected to. A poll is one process listing for each process
//! in the tree and, for each, one descriptor list and one look at each of its sockets (`sys::process`):
//! a shell with a dev server under it is a few dozen calls. It is meant for a background thread every
//! few seconds, for the terminal on screen.

use std::collections::{BTreeSet, HashSet};

use crate::sys::process;

/// The most processes looked at under one shell, and sockets looked at in one process: a fork bomb
/// or a process holding a hundred thousand descriptors must not make a poll expensive.
const MAX_PROCESSES: usize = 256;
const MAX_FDS: usize = 4096;

/// The ports the processes under the shell `shell_pid` (the shell itself included) have a TCP
/// socket listening on, lowest first, each once. A listener on any address counts: the chip opens
/// `localhost`, which is where a development server is looked for.
pub fn listening_ports(shell_pid: u32) -> Vec<u16> {
    let ports: BTreeSet<u16> = tree(shell_pid, MAX_PROCESSES)
        .into_iter()
        .flat_map(|pid| process::listening_tcp_ports(pid, MAX_FDS))
        .collect();
    ports.into_iter().collect()
}

/// `pid` and everything started under it, as far as `limit` allows, `pid` first.
fn tree(pid: u32, limit: usize) -> Vec<u32> {
    let mut seen: HashSet<u32> = HashSet::from([pid]);
    let mut all = vec![pid];
    let mut next = 0;
    while let Some(&parent) = all.get(next) {
        next += 1;
        for child in process::children(parent) {
            if all.len() < limit && seen.insert(child) {
                all.push(child);
            }
        }
    }
    all
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;
    use std::process::{Command, Stdio};

    use super::{listening_ports, tree};
    use crate::sys::process;

    /// The offsets in the system's socket description are the SDK's: a socket of our own is found by
    /// the real calls.
    #[test]
    fn a_listener_of_this_process_is_found_and_a_closed_one_is_not() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("address").port();
        assert!(process::listening_tcp_ports(std::process::id(), 4096).contains(&port));
        drop(listener);
        assert!(!process::listening_tcp_ports(std::process::id(), 4096).contains(&port));
    }

    /// A connection that is open is not a listener, on either end.
    #[test]
    fn a_connected_socket_is_not_listening() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("address").port();
        let client = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
        let (_server_side, _) = listener.accept().expect("accept");
        let client_port = client.local_addr().expect("address").port();
        let found = process::listening_tcp_ports(std::process::id(), 4096);
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
            grandchildren = tree(shell_pid, 256);
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
        assert!(tree(std::process::id(), 1).len() == 1);
    }

    /// What a poll costs: the test run's own tree (a shell's is smaller).
    /// `cargo test -p wormhole-term poll_cost -- --ignored --nocapture`
    #[test]
    #[ignore = "benchmark"]
    fn poll_cost() {
        let _listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let started = std::time::Instant::now();
        let tree = tree(std::process::id(), 256);
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
