//! Is the process a meta file names the host it says it is? A pid in a file proves nothing: pids
//! are reused (after a reboot above all), and the executable a host runs from may be gone (an
//! update replaces the app bundle while hosts keep running), which leaves its path empty. So
//! a host is known by what the system keeps of the process itself: the **name** it was started
//! as (`wormhole-host`), the **user** it runs as, and **when it started**.
//!
//! The answer is three-valued, because a wrong "no" is the expensive one (the files of a running
//! host get removed, and its terminal is lost): [`Identity::No`] only when something definite says
//! so (no such process, another program's name, another user, or a process that started after the
//! host did), [`Identity::Yes`] when everything fits, and [`Identity::Unknown`] when the meta or
//! the system cannot say. A reused pid always started *after* the host, so a process that started
//! earlier than the meta says (a slow first start, a clock that was set) still counts for it.

use std::time::{Duration, SystemTime};

use super::HostMeta;
use crate::ipc::host_binary::HOST_BINARY_NAME;
use crate::platform::process;

/// How much later than the meta's `started_at` (whole seconds) a process may have started and
/// still be the host: the host takes the time after it began, and the two are read at
/// different resolutions.
pub const START_SLACK_SECS: u64 = 2;

/// How far two readings of one process's start may differ and still be the same start.
const EXACT_START_TOLERANCE: Duration = Duration::from_millis(5);

/// What is known of whether a process is the host a meta describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Identity {
    /// Named like a host, ours, and started when the meta says.
    Yes,
    /// Definitely something else, or nothing.
    No,
    /// Nothing says it is not the host, and not everything says it is.
    Unknown,
}

/// What the system says of one process, read once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reading {
    pub exists: bool,
    pub name: Option<String>,
    /// Runs as this user: `None` when it cannot be read.
    pub ours: Option<bool>,
    pub start: Option<SystemTime>,
}

impl Reading {
    /// Read process `pid`.
    pub(crate) fn of(pid: u32) -> Self {
        Self {
            exists: process::exists(pid),
            name: process::command_name(pid),
            ours: process::runs_as_current_user(pid),
            start: process::start_time(pid),
        }
    }
}

/// Whether the process the meta names is the host it describes.
pub fn identify(meta: &HostMeta) -> Identity {
    classify(&Reading::of(meta.pid), meta.started_at, meta.proc_start)
}

/// Whether `pid` is *named like* a host and runs as this user, whatever its start: what is left to
/// ask when the start cannot be compared. A signal goes to a process that looks like a host and
/// nothing else.
pub fn looks_like_host_process(pid: u32) -> bool {
    let reading = Reading::of(pid);
    reading.exists && reading.name.as_deref() == Some(HOST_BINARY_NAME) && reading.ours == Some(true)
}

/// [`identify`] over what was read, so the rules are tested without processes.
pub(crate) fn classify(reading: &Reading, started_at: u64, proc_start: Option<(u64, u32)>) -> Identity {
    if !reading.exists {
        return Identity::No;
    }
    if reading.name.as_deref().is_some_and(|name| name != HOST_BINARY_NAME) || reading.ours == Some(false) {
        return Identity::No;
    }
    let Some(start) = reading.start.and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok()) else {
        return Identity::Unknown;
    };
    let fits = match proc_start {
        // The host wrote its own start: the same process starts at the same moment.
        Some((secs, micros)) => {
            let wrote = Duration::new(secs, micros.saturating_mul(1000));
            start.abs_diff(wrote) <= EXACT_START_TOLERANCE
        }
        // An older host: only a process that started after the meta says is surely another one.
        None if started_at == 0 => return Identity::Unknown,
        None => start.as_secs() <= started_at + START_SLACK_SECS,
    };
    if !fits {
        return Identity::No;
    }
    if reading.name.is_some() && reading.ours == Some(true) { Identity::Yes } else { Identity::Unknown }
}

/// The file the stand-in host creates, in its folder, once it runs.
#[cfg(test)]
const FAKE_HOST_READY: &str = "fake-host-ready";

/// A stand-in for a running host in tests: a copy of this test program named `wormhole-host`, in
/// `dir`, running the one test that only sleeps ([`tests::fake_host_sleeps`]).
#[cfg(test)]
pub(crate) fn spawn_fake_host(dir: &std::path::Path) -> FakeHost {
    let binary = dir.join(HOST_BINARY_NAME);
    std::fs::copy(std::env::current_exe().expect("this test program"), &binary).expect("copies it");
    let child = std::process::Command::new(&binary)
        .args(["--ignored", "--exact", "ipc::hosts::identity::tests::fake_host_sleeps", "--nocapture"])
        .current_dir(dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("starts");
    // It says when it runs (its program is loaded): only then can a test delete the file it came from.
    let ready = dir.join(FAKE_HOST_READY);
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while !ready.exists() {
        assert!(std::time::Instant::now() < deadline, "the stand-in host did not start");
        std::thread::sleep(Duration::from_millis(10));
    }
    // The meta says when the process started, as the system does.
    let started = loop {
        if let Some(start) = process::start_time(child.id()) {
            break start;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let since = started.duration_since(SystemTime::UNIX_EPOCH).expect("clock");
    FakeHost { child, started_at: since.as_secs(), proc_start: (since.as_secs(), since.subsec_micros()) }
}

/// A running process that is a host as far as `identify` can tell; killed when dropped.
#[cfg(test)]
pub(crate) struct FakeHost {
    child: std::process::Child,
    /// What a meta file says to match it.
    pub started_at: u64,
    pub proc_start: (u64, u32),
}

#[cfg(test)]
impl FakeHost {
    pub(crate) fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Whether the process is gone (reaped when it is).
    pub(crate) fn has_exited(&mut self) -> bool {
        self.child.try_wait().map(|status| status.is_some()).unwrap_or(true)
    }

    /// Wait up to `within` for the process to end.
    pub(crate) fn exits_within(&mut self, within: Duration) -> bool {
        let deadline = std::time::Instant::now() + within;
        while std::time::Instant::now() < deadline {
            if self.has_exited() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        self.has_exited()
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
mod tests;
