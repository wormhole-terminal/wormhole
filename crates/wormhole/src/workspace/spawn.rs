//! Starting helper processes (`osascript`, `open`) that nobody waits for.
//!
//! Dropping a [`std::process::Child`] neither waits for it nor kills it, and nothing else in the
//! process reaps strangers (the terminal's SIGCHLD handling only waits for its own shells), so
//! every fire-and-forget `spawn()` leaves a zombie behind until we exit. Zombies count against the
//! per-user process limit, and an app that stays open for days notifies many times a day.

use std::io;
use std::process::Command;
use std::thread::{Builder, JoinHandle};

/// Start `cmd` and let it run on its own; a small thread waits for it so it never becomes a
/// zombie. Failing to start it is the caller's to report.
pub(super) fn spawn_detached(cmd: Command) -> io::Result<()> {
    spawn_reaped(cmd).map(drop)
}

/// [`spawn_detached`], keeping the child's pid and the thread that reaps it (so a test can wait
/// for the reaping and look at the process table).
fn spawn_reaped(mut cmd: Command) -> io::Result<(u32, JoinHandle<()>)> {
    let mut child = cmd.spawn()?;
    let pid = child.id();
    let reaper = Builder::new().name("reap-child".into()).spawn(move || {
        if let Err(err) = child.wait() {
            tracing::debug!(pid, "waiting for a helper process failed: {err}");
        }
    })?;
    Ok((pid, reaper))
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    // Only the Linux zombie tests (they read /proc) use these; unconditional imports fail
    // `-D warnings` on macOS.
    #[cfg(target_os = "linux")]
    use std::time::{Duration, Instant};

    use super::spawn_detached;
    #[cfg(target_os = "linux")]
    use super::spawn_reaped;

    /// The state letter of `pid` in the process table (`Z` = zombie), `None` once it is gone.
    #[cfg(target_os = "linux")]
    fn process_state(pid: u32) -> Option<char> {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        // "pid (comm) S ...": the name may contain spaces and parentheses, so cut after the last ")".
        stat.rsplit_once(')')?.1.trim_start().chars().next()
    }

    #[cfg(target_os = "linux")]
    fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !done() {
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// What `spawn_detached` is for: a child that is started and never waited for stays in the
    /// process table as a zombie.
    #[cfg(target_os = "linux")]
    #[test]
    fn an_unwaited_child_stays_a_zombie() {
        let mut child = Command::new("true").spawn().expect("spawns true");
        let pid = child.id();
        wait_until("the child to exit", || process_state(pid) == Some('Z'));
        child.wait().expect("reaps");
        assert_eq!(process_state(pid), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn detached_children_are_reaped() {
        let started: Vec<_> = (0..5).map(|_| spawn_reaped(Command::new("true")).expect("spawns")).collect();
        for (pid, reaper) in started {
            reaper.join().expect("reaper thread ends");
            assert_eq!(process_state(pid), None, "child {pid} was left behind");
        }
    }

    #[test]
    fn a_missing_program_is_reported_to_the_caller() {
        let err = spawn_detached(Command::new("/nonexistent/wormhole-helper")).expect_err("no such program");
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }
}
