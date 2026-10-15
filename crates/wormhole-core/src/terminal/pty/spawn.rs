//! Starting the program: a new session with the slave as its controlling terminal, the slave as
//! its standard streams, default signal handling, the working directory and the environment.
//!
//! `std::process::Command` does the fork and exec, which matters for more than convenience: it
//! prepares every string before the fork (nothing allocates in the child), searches `PATH`, and
//! reports a program that cannot start (not found, not executable, a NUL in a name) to the parent
//! as an error from `spawn`, over a pipe that closes when the exec succeeds. What the PTY needs on
//! top runs in the child between the fork and the exec, in [`enter_session`].

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::io;
use std::os::fd::OwnedFd;
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::process::CommandExt as _;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

/// The program a terminal runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::terminal) struct Shell {
    /// The executable, by path or by name to look up in `PATH`.
    pub program: String,
    /// Its arguments, after the program's own name.
    pub args: Vec<String>,
}

impl Shell {
    /// `program` with `args`.
    pub(in crate::terminal) fn new(program: String, args: Vec<String>) -> Self {
        Self { program, args }
    }
}

/// How to start it.
#[derive(Clone, Debug)]
pub(in crate::terminal) struct Launch {
    /// The program.
    pub shell: Shell,
    /// Where it starts. A folder that is not there is not an error: the program starts where the
    /// app is, as a terminal whose folder was deleted still opens.
    pub cwd: Option<PathBuf>,
    /// On top of the app's own environment.
    pub env: HashMap<String, String>,
}

/// Start `launch.shell` on `slave`, which the child takes for its standard streams and its
/// controlling terminal. The parent's copies of the slave are closed when this returns.
pub(super) fn spawn(launch: &Launch, slave: OwnedFd) -> io::Result<Child> {
    let mut command = Command::new(&launch.shell.program);
    command.args(&launch.shell.args).envs(&launch.env);
    command
        .stdin(Stdio::from(slave.try_clone()?))
        .stdout(Stdio::from(slave.try_clone()?))
        .stderr(Stdio::from(slave));
    // A path with a NUL in it can't be one, and the chdir would fail anyway.
    let cwd = launch.cwd.as_ref().and_then(|dir| CString::new(dir.as_os_str().as_bytes()).ok());
    add_pre_exec(&mut command, cwd);
    command.spawn()
}

/// Have `command` run [`enter_session`] in the child.
#[allow(unsafe_code)] // `pre_exec` is unsafe: the closure runs between fork and exec.
fn add_pre_exec(command: &mut Command, cwd: Option<CString>) {
    // SAFETY: the closure only calls async-signal-safe functions (see `enter_session`), allocates
    // nothing and touches nothing but its own copy of the process; it runs in the child between
    // the fork and the exec, where the parent's other threads do not exist.
    unsafe {
        command.pre_exec(move || enter_session(cwd.as_deref()));
    }
}

/// What the child does before it runs the program, in the order that matters: signals first, so
/// nothing the app ignores or blocks reaches the program; then the new session, which makes the
/// child its leader and the leader of a new process group, so `killpg` of its pid reaches
/// everything it starts; then the slave, already on standard input, as the session's controlling
/// terminal, which is what lets the line discipline send `SIGINT` for ^C and `SIGHUP` when the
/// terminal closes; last the folder.
///
/// Everything here is async-signal-safe and none of it allocates: after a fork in a process with
/// other threads nothing else is allowed (another thread may hold the allocator's lock in the
/// child's copy).
fn enter_session(cwd: Option<&CStr>) -> io::Result<()> {
    reset_signals();
    new_session()?;
    take_terminal()?;
    if let Some(dir) = cwd {
        enter_folder(dir);
    }
    Ok(())
}

/// Default handling for every signal and none blocked (signal(3), pthread_sigmask(3)): signals
/// the app ignores stay ignored across exec otherwise.
#[allow(unsafe_code)] // libc FFI; plain integers and a local signal set.
fn reset_signals() {
    // SAFETY: both calls are async-signal-safe and get valid arguments. 1..=64 covers every
    // signal either platform has; SIGKILL, SIGSTOP and numbers a platform doesn't have fail with
    // EINVAL, which does not matter.
    unsafe {
        for signal in 1..=64 {
            libc::signal(signal, libc::SIG_DFL);
        }
        let mut none: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut none);
        libc::pthread_sigmask(libc::SIG_SETMASK, &none, std::ptr::null_mut());
    }
}

/// Leave the app's session for a new one (setsid(2)).
#[allow(unsafe_code)] // libc FFI; no arguments.
fn new_session() -> io::Result<()> {
    // SAFETY: setsid is async-signal-safe and takes no pointers.
    if unsafe { libc::setsid() } < 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
}

/// Make standard input, the slave, the session's controlling terminal (`TIOCSCTTY`, tty(4)).
#[allow(unsafe_code)] // libc FFI; plain integers.
fn take_terminal() -> io::Result<()> {
    // SAFETY: ioctl is async-signal-safe; this request takes an integer and no pointer.
    if unsafe { libc::ioctl(libc::STDIN_FILENO, libc::TIOCSCTTY as _, 0) } < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Go to `dir`; the program starts where the app is when this fails (see `Launch::cwd`).
#[allow(unsafe_code)] // libc FFI; a NUL-terminated path.
fn enter_folder(dir: &CStr) {
    // SAFETY: chdir is async-signal-safe and `dir` is NUL-terminated and outlives the call.
    unsafe {
        libc::chdir(dir.as_ptr());
    }
}
