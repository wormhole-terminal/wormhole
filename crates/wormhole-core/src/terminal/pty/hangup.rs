//! Closing a terminal: ending the program in it, the shell and whatever the shell runs.
//!
//! Closing the master is not enough. A shell or program that ignores `SIGHUP` (`trap '' HUP`,
//! `nohup`) keeps running, and a shell with job control runs the program in the foreground in a
//! process group of its own, which the shell's group never reaches. So:
//!
//! - The shell's process group (it leads one: `setsid`) gets the hangup, and `SIGKILL` when
//!   anything is still there after the grace period. The terminal's foreground group gets both
//!   signals too. While the shell lives that group is asked for (`tcgetpgrp` on the master)
//!   right before each signal: the kernel only reports a group of this terminal's own session,
//!   so a group id that was reused meanwhile cannot be hit. A shell that goes at the hangup
//!   takes the terminal's answer with it, so the group from the hangup is then killed after the
//!   grace period if it is still there; a group id is not given out again while the group has
//!   members, and the watch checks every [`WATCH_STEP`] that it has.
//! - Nobody parses the output any more, but it must still be read. A session leader that exits
//!   waits in the kernel for its terminal's output to drain: with the master open and unread
//!   that never ends (the shell sits in state E), and a program that prints blocks on the full
//!   buffer. So the master is read and thrown away until the shell is reaped, and closed then,
//!   since an open master keeps the terminal from hanging up.
//!
//! All of it runs on the reader thread, which is the one thing that reaps the shell: until
//! `try_wait` returns the shell's pid cannot be reused, so a signal to it cannot reach anyone else.

use std::fs::File;
use std::io::{self, Read};
use std::os::fd::AsRawFd;
use std::time::{Duration, Instant};

use polling::Events;

use super::exit;
use super::master::{Interest, Key, Master};
use super::reader::Core;

/// How long a closed terminal's shell gets to exit before it is killed.
pub(in crate::terminal) const HANGUP_GRACE: Duration = Duration::from_secs(3);

/// How long the watch waits for output before it looks at the clock and the groups again.
const WATCH_STEP: Duration = Duration::from_millis(100);

/// How long the watch keeps going after the `SIGKILL`. A process that is still there by then is
/// stuck in the kernel, where nothing it is sent or read can help it.
const WATCH_AFTER_KILL: Duration = Duration::from_secs(10);

/// What a process group is sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Signal {
    /// `SIGHUP`: the terminal went away.
    Hangup,
    /// `SIGKILL`: it did not go.
    Kill,
}

impl Signal {
    fn raw(self) -> libc::c_int {
        match self {
            Self::Hangup => libc::SIGHUP,
            Self::Kill => libc::SIGKILL,
        }
    }
}

/// A process group of this terminal's session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Group(libc::pid_t);

impl Group {
    /// The group the shell leads (it called `setsid`). Not one that is 0, 1 or below: those
    /// would be our own group or everyone's.
    fn led_by(pid: u32) -> Option<Self> {
        libc::pid_t::try_from(pid).ok().filter(|&pid| pid > 1).map(Self)
    }

    /// The group in the foreground of the terminal whose master this is, if it says.
    #[allow(unsafe_code)] // libc FFI; the descriptor is open as long as `master` is.
    fn foreground_of(master: &File) -> Option<Self> {
        // SAFETY: tcgetpgrp only reads the terminal's state.
        let group = unsafe { libc::tcgetpgrp(master.as_raw_fd()) };
        (group > 1).then_some(Self(group))
    }

    /// Whether the group still has a member (`killpg` with no signal), a zombie included.
    #[allow(unsafe_code)] // libc FFI; the arguments are plain integers.
    fn exists(self) -> bool {
        // SAFETY: signal 0 only checks that the group can be signalled.
        let sent = unsafe { libc::killpg(self.0, 0) } == 0;
        // EPERM: it exists, and is not ours to signal.
        sent || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }

    #[allow(unsafe_code)] // libc FFI; the arguments are plain integers.
    fn send(self, signal: Signal) {
        // SAFETY: killpg only sends a signal; a group that no longer exists fails with ESRCH.
        unsafe {
            libc::killpg(self.0, signal.raw());
        }
    }
}

/// The groups a closed terminal's program is in.
struct Groups {
    /// The shell's own.
    shell: Group,
    /// The groups the terminal had in the foreground when it was asked, other than the shell's:
    /// a job each (a shell with job control gives each its own group).
    foreground: Vec<Group>,
}

impl Groups {
    /// Send `signal` to all of them, asking the terminal for its foreground group first while
    /// the shell can still say.
    fn send(&mut self, master: Option<&Master>, signal: Signal) {
        let foreground = master.and_then(|master| Group::foreground_of(master.file()));
        if let Some(group) =
            foreground.filter(|&group| group != self.shell && !self.foreground.contains(&group))
        {
            self.foreground.push(group);
        }
        self.shell.send(signal);
        self.send_to_foreground(signal);
    }

    fn send_to_foreground(&self, signal: Signal) {
        self.foreground.iter().for_each(|group| group.send(signal));
    }

    /// Forget the groups that have no member left.
    fn prune(&mut self) {
        self.foreground.retain(|group| group.exists());
    }
}

/// Whether the terminal's output goes on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Output {
    /// There may be more.
    Flowing,
    /// The terminal hung up: nothing more will come.
    Ended,
}

impl Core {
    /// End the program (see the module docs) and then the thread.
    pub(super) fn hangup(self, grace: Duration, events: &mut Events) {
        let Core { master, mut child, poller, .. } = self;
        let mut master = Some(master);
        // The output must be read, whatever else was polled for.
        master.iter_mut().for_each(|open| open.poll_for(Interest::Output));
        let reaped =
            |child: &mut std::process::Child| child.try_wait().map_or(true, |status| status.is_some());
        // Already reaped (it exited as the terminal closed): its pid may be anyone's by now.
        let Some(shell) = Group::led_by(child.id()).filter(|_| !reaped(&mut child)) else {
            return;
        };
        let mut groups = Groups { shell, foreground: Vec::new() };
        groups.send(master.as_ref(), Signal::Hangup);
        let started = Instant::now();
        let mut killed = false;
        let mut scratch = [0u8; 16 * 1024];
        loop {
            let shell_reaped = reaped(&mut child);
            if shell_reaped {
                // Hang the terminal up (deregistered first, by `Master`'s drop).
                master = None;
            }
            groups.prune();
            let waited = started.elapsed();
            if (shell_reaped && groups.foreground.is_empty()) || waited >= grace + WATCH_AFTER_KILL {
                break;
            }
            if !killed && waited >= grace {
                killed = true;
                if shell_reaped {
                    // Until the shell is reaped its pid can't be reused; after, only the
                    // foreground groups are left to kill.
                    groups.send_to_foreground(Signal::Kill);
                } else {
                    groups.send(master.as_ref(), Signal::Kill);
                }
            }
            let step = if killed { WATCH_STEP } else { WATCH_STEP.min(grace.saturating_sub(waited)) };
            events.clear();
            // The master's output, or the child's exit; a timeout is the common end of a wait.
            if poller.wait(events, Some(step)).is_err() {
                std::thread::sleep(step);
            }
            let output =
                events.iter().any(|event| Key::from_raw(event.key) == Some(Key::Master) && event.readable);
            if let Some(open) = master.as_mut().filter(|_| output)
                && discard(open, &mut scratch) == Output::Ended
            {
                // Nothing more will come (and the poller would report the hangup at once).
                open.forget();
            }
        }
        exit::unwatch(&poller, &child);
        drop(master);
    }
}

/// Read and throw away what the terminal printed.
fn discard(master: &Master, scratch: &mut [u8]) -> Output {
    loop {
        match master.file().read(scratch) {
            Ok(0) => return Output::Ended,
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => return Output::Flowing,
            Err(_) => return Output::Ended,
        }
    }
}
