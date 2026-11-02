//! At launch: remove the files of every host that is certainly gone (its process is not the host
//! the meta describes: gone, or its pid reused, see `identity`), the socket in the temporary folder
//! included, so they do not pile up for ever. Those whose socket is missing (the process may run
//! on), whose meta is unreadable or that have no meta are left for the person to look at in the
//! Session Hosts window.

use std::path::Path;

use super::{StaleReason, forget, scan};

/// Remove the files of the gone hosts in `hosts_dir`. Returns the ids removed; each is logged.
pub fn forget_gone_hosts(hosts_dir: &Path) -> Vec<String> {
    let Ok(scanned) = scan(hosts_dir) else { return Vec::new() };
    let mut removed = Vec::new();
    for stale in scanned.stale.iter().filter(|s| s.reason == StaleReason::PidGone) {
        match forget(hosts_dir, &stale.id) {
            Ok(done) => {
                tracing::info!(
                    id = stale.id,
                    files = done.removed.len(),
                    "removed the files of a session host that is gone"
                );
                removed.push(stale.id.clone());
            }
            Err(err) => tracing::warn!(id = stale.id, "left the files of a stale session host: {err}"),
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::forget_gone_hosts;
    use crate::platform::socket::Listener;

    fn meta(id: &str, pid: u32, started_at: u64, socket: &Path) -> String {
        format!(
            "format=1\nid={id}\npid={pid}\nstarted_at={started_at}\nhost_version=1.0.0\nproto_min=1\n\
             proto_max=1\nsocket={}\ndata_dir=/d\n",
            socket.display()
        )
    }

    /// Hosts whose process is gone, or is another program now (a reused pid), have their files
    /// removed at launch; a running host and a file that only looks odd are left alone.
    #[test]
    fn stale_hosts_are_forgotten_at_launch() {
        let dir = tempfile::Builder::new().prefix("hl").tempdir_in("/tmp").expect("temp");
        let d = dir.path();
        let live = crate::ipc::hosts::spawn_fake_host(d);
        let live_sock = d.join("llllllllllll.sock");
        let _listener = Listener::bind(&live_sock).expect("binds");
        let live_meta = meta("llllllllllll", live.pid(), live.started_at, &live_sock);
        std::fs::write(d.join("llllllllllll.meta"), live_meta).expect("meta");
        // Gone: a pid nothing has (a reaped child).
        let mut child = std::process::Command::new("/usr/bin/true").spawn().expect("spawns");
        let gone = child.id();
        child.wait().expect("waits");
        let gone_sock = d.join("gggggggggggg.sock");
        drop(Listener::bind(&gone_sock).expect("binds"));
        std::fs::write(d.join("gggggggggggg.meta"), meta("gggggggggggg", gone, 1, &gone_sock)).expect("meta");
        // Reused: the pid is this test process, which is not a host.
        let reused_sock = d.join("rrrrrrrrrrrr.sock");
        drop(Listener::bind(&reused_sock).expect("binds"));
        let reused_meta = meta("rrrrrrrrrrrr", std::process::id(), 1, &reused_sock);
        std::fs::write(d.join("rrrrrrrrrrrr.meta"), reused_meta).expect("meta");
        // Slow: a live host whose process began long before it wrote its meta.
        let slow = crate::ipc::hosts::spawn_fake_host(d);
        let slow_sock = d.join("ssssssssssss.sock");
        let _slow_listener = Listener::bind(&slow_sock).expect("binds");
        let slow_meta = meta("ssssssssssss", slow.pid(), slow.started_at + 30, &slow_sock);
        std::fs::write(d.join("ssssssssssss.meta"), slow_meta).expect("meta");
        // Odd: a meta that does not read stays for a person to look at.
        std::fs::write(d.join("uuuuuuuuuuuu.meta"), "garbage").expect("write");

        let mut removed = forget_gone_hosts(d);
        removed.sort();
        assert_eq!(removed, ["gggggggggggg", "rrrrrrrrrrrr"]);
        for name in ["gggggggggggg.meta", "gggggggggggg.sock", "rrrrrrrrrrrr.meta", "rrrrrrrrrrrr.sock"] {
            assert!(!d.join(name).exists(), "{name} is removed");
        }
        for name in [
            "llllllllllll.meta",
            "llllllllllll.sock",
            "ssssssssssss.meta",
            "ssssssssssss.sock",
            "uuuuuuuuuuuu.meta",
        ] {
            assert!(d.join(name).exists(), "{name} stays");
        }
    }
}
