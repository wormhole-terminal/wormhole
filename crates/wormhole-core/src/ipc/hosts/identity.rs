//! Is the process a meta file names the host it says it is? A pid in a file proves nothing: pids
//! are reused (after a reboot above all), and the executable a host runs from may be gone (an
//! update replaces the app bundle while hosts keep running), which leaves its path empty. So
//! a host is known by what the system keeps of the process itself: the **name** it was started
//! as (`wormhole-host`), the **user** it runs as, and **when it started**, which must match the
//! meta's `started_at` (the host writes that within moments of starting) within a few seconds.

use std::time::SystemTime;

use crate::ipc::host_binary::HOST_BINARY_NAME;
use crate::platform::process;

/// How far a process's start may be from the meta's `started_at`: the host takes its time a few
/// moments after the process began, and the two clocks are read at different resolutions.
pub const START_SLACK_SECS: u64 = 10;

/// Whether process `pid` is the session host whose meta says it started at `started_at` (Unix
/// seconds): named like a host, running as this user, started when the meta says.
pub fn is_host_process(pid: u32, started_at: u64) -> bool {
    matches(
        process::command_name(pid).as_deref(),
        process::runs_as_current_user(pid),
        process::start_time(pid),
        started_at,
    )
}

/// [`is_host_process`] over what was read, so the rule is tested without processes.
fn matches(name: Option<&str>, ours: bool, start: Option<SystemTime>, started_at: u64) -> bool {
    let Some(start) = start.and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok()) else {
        return false;
    };
    name == Some(HOST_BINARY_NAME) && ours && start.as_secs().abs_diff(started_at) <= START_SLACK_SECS
}

/// A stand-in for a running host in tests: a copy of `sleep` named `wormhole-host`, in `dir`.
#[cfg(test)]
pub(crate) fn spawn_fake_host(dir: &std::path::Path) -> FakeHost {
    let binary = dir.join(HOST_BINARY_NAME);
    std::fs::copy("/bin/sleep", &binary).expect("copies sleep");
    // macOS kills a copy of a system binary at once (its signature names the old place): sign it again.
    #[cfg(target_os = "macos")]
    {
        let signed = std::process::Command::new("/usr/bin/codesign")
            .args(["-s", "-", "-f"])
            .arg(&binary)
            .output()
            .expect("codesign runs");
        assert!(signed.status.success(), "codesign: {}", String::from_utf8_lossy(&signed.stderr));
    }
    let child = std::process::Command::new(&binary).arg("60").spawn().expect("starts");
    // Let it load before a test deletes its file: a page not read yet cannot be read afterwards, and
    // the system ends a process that needs one.
    std::thread::sleep(std::time::Duration::from_millis(300));
    let started_at = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).expect("clock").as_secs();
    FakeHost { child, started_at }
}

/// A running process that is a host as far as `is_host_process` can tell; killed when dropped.
#[cfg(test)]
pub(crate) struct FakeHost {
    child: std::process::Child,
    /// What a meta file says to match it (Unix seconds).
    pub started_at: u64,
}

#[cfg(test)]
impl FakeHost {
    pub(crate) fn pid(&self) -> u32 {
        self.child.id()
    }
}

#[cfg(test)]
impl Drop for FakeHost {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn at(secs: u64) -> Option<SystemTime> {
        Some(SystemTime::UNIX_EPOCH + Duration::from_secs(secs))
    }

    #[test]
    fn a_host_has_the_name_the_user_and_the_start_the_meta_says() {
        assert!(matches(Some("wormhole-host"), true, at(1000), 1000));
        assert!(matches(Some("wormhole-host"), true, at(1000), 1000 + START_SLACK_SECS));
        assert!(matches(Some("wormhole-host"), true, at(1000), 1000 - START_SLACK_SECS));
    }

    #[test]
    fn anything_else_is_not_the_host() {
        // Another program under the pid.
        assert!(!matches(Some("zsh"), true, at(1000), 1000));
        assert!(!matches(None, true, at(1000), 1000));
        // Another user's.
        assert!(!matches(Some("wormhole-host"), false, at(1000), 1000));
        // A reused pid: the process is younger or older than the host the meta describes.
        assert!(!matches(Some("wormhole-host"), true, at(1000 + START_SLACK_SECS + 1), 1000));
        assert!(!matches(Some("wormhole-host"), true, at(5), 1000));
        assert!(!matches(Some("wormhole-host"), true, None, 1000));
    }

    #[test]
    fn this_process_and_a_gone_one_are_not_hosts() {
        assert!(!is_host_process(std::process::id(), 0));
        assert!(!is_host_process(0, 0));
        assert!(!is_host_process(u32::MAX, 0));
    }

    /// An update deletes the bundle a host runs from; the process lives on, with no path to its
    /// executable, and must still be known as the host (so `sessions end` can signal it).
    #[cfg(target_os = "macos")]
    #[test]
    fn a_host_whose_binary_was_deleted_is_still_known() {
        let dir = tempfile::tempdir().expect("temp");
        let host = spawn_fake_host(dir.path());
        let (pid, started) = (host.pid(), host.started_at);
        assert!(is_host_process(pid, started), "running from its binary");
        std::fs::remove_file(dir.path().join(HOST_BINARY_NAME)).expect("deletes the binary");
        assert!(is_host_process(pid, started), "its name and start are still there");
        assert!(!is_host_process(pid, started + 3600), "another start is another process");
        drop(host);
        assert!(!is_host_process(pid, started), "gone");
    }
}
