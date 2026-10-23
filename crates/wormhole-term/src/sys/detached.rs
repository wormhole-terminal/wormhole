//! A program started to outlive this process: in a session of its own, with the environment it is
//! given and nothing else, no standard streams, and a readiness pipe it writes one line to once it is
//! up. The process that starts it keeps it as a child until the start is settled, so one that failed
//! to come up can be signalled safely; once the [`Detached`] is dropped a thread reaps it whenever it
//! ends.

use std::ffi::{OsStr, OsString};
use std::io;
use std::path::Path;

use super::unix::detached as os;
pub use super::unix::detached::{ReadyLine, ReadyReader};

/// A signal to a [`Detached`] program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signal {
    HangUp,
    Terminate,
    Kill,
}

/// A program just started. Dropped, it is left to the reaper.
pub struct Detached {
    pub pid: u32,
    pub ready: ReadyReader,
}

impl Detached {
    /// Send the program `signal`. Sound until this is dropped: nobody has reaped it, so its pid is
    /// still its own, even as a zombie.
    pub fn signal(&self, signal: Signal) {
        os::signal(self.pid, signal);
    }
}

impl Drop for Detached {
    fn drop(&mut self) {
        os::reap_later(self.pid);
    }
}

/// Start `binary` with `args` (after its own name as the first) and exactly `env`.
pub fn spawn(binary: &Path, args: &[&OsStr], env: &[(&str, OsString)]) -> io::Result<Detached> {
    let (pid, ready) = os::spawn(binary, args, env)?;
    Ok(Detached { pid, ready })
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    fn run(script: &str, env: &[(&str, OsString)]) -> Detached {
        spawn(Path::new("/bin/sh"), &[OsStr::new("-c"), OsStr::new(script)], env).expect("starts")
    }

    fn line(started: &mut Detached) -> ReadyLine {
        started.ready.read_line(Instant::now() + Duration::from_secs(10)).expect("reads")
    }

    #[test]
    fn the_program_reports_on_descriptor_3_with_the_environment_it_was_given() {
        let mut started = run("printf 'ok %s\\n' \"$GREETING\" >&3", &[("GREETING", "hello".into())]);
        assert_eq!(line(&mut started), ReadyLine::Line("ok hello".into()));
    }

    #[test]
    fn a_program_that_ends_without_a_line_closes_the_pipe() {
        let mut started = run("exit 0", &[]);
        assert_eq!(line(&mut started), ReadyLine::Closed);
    }

    #[test]
    fn nothing_but_what_it_was_given_reaches_it() {
        // The parent's own variables (HOME, which a test run always has) are not passed on.
        let mut started = run("printf 'ok %s\\n' \"${HOME:-none}\" >&3", &[]);
        assert_eq!(line(&mut started), ReadyLine::Line("ok none".into()));
    }

    #[test]
    fn it_is_silent_until_it_speaks_and_can_be_stopped() {
        let mut started = run("exec sleep 30", &[]);
        assert_eq!(started.ready.read_line(Instant::now()).expect("reads"), ReadyLine::Silent);
        started.signal(Signal::Terminate);
        assert_eq!(line(&mut started), ReadyLine::Closed);
    }

    #[test]
    fn a_program_that_is_not_there_is_an_error() {
        assert!(spawn(Path::new("/nonexistent/wormhole-test-program"), &[], &[]).is_err());
    }
}
