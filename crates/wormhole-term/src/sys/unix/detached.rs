//! Starting a program that outlives this process, by `posix_spawn` in a session of its own
//! (`POSIX_SPAWN_SETSID`), so it is reparented to init when we go, with:
//!
//! - only the environment it is given;
//! - `/dev/null` for its standard streams, the readiness pipe's write end on descriptor 3, and on
//!   Apple systems no other descriptor of ours (`POSIX_SPAWN_CLOEXEC_DEFAULT`);
//! - an empty signal mask and every signal at its default, whatever the spawning thread had.
//!
//! The program is this process's child while we run: once the start is settled a thread waits for it
//! ([`reap_later`]), so one that ends is reaped instead of left a zombie (and until then its pid cannot
//! be anyone else's, so one that failed to come up can be signalled safely).

use std::ffi::{CString, OsStr, OsString, c_char};
use std::fs::File;
use std::io::{self, Read as _};
use std::os::fd::{AsRawFd as _, FromRawFd as _, OwnedFd, RawFd};
use std::os::unix::ffi::OsStrExt as _;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::sys::detached::Signal;

/// The descriptor the program reads its readiness pipe from.
const READY_FD: RawFd = 3;
/// The longest readiness line read.
const LINE_MAX: usize = 4096;

/// `POSIX_SPAWN_SETSID` (macOS 10.15 and later; the `libc` crate does not name it for Apple).
#[cfg(target_vendor = "apple")]
const SETSID: libc::c_short = 0x0400;
#[cfg(not(target_vendor = "apple"))]
#[allow(clippy::unnecessary_cast)] // a `c_short` already on Linux, a `c_int` elsewhere
const SETSID: libc::c_short = libc::POSIX_SPAWN_SETSID as libc::c_short;

/// What a read of the readiness pipe found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadyLine {
    /// One line (or what the pipe held when it closed or filled), without its newline.
    Line(String),
    /// The pipe closed without a line: the program is gone.
    Closed,
    /// Nothing yet.
    Silent,
}

/// The readiness pipe's read end.
pub struct ReadyReader {
    file: File,
    line: Vec<u8>,
}

impl ReadyReader {
    /// What the pipe says by `until` (a past `until` only looks).
    pub fn read_line(&mut self, until: Instant) -> io::Result<ReadyLine> {
        loop {
            let mut buf = [0u8; 256];
            match self.file.read(&mut buf) {
                Ok(0) => return Ok(self.finish(true)),
                Ok(n) => {
                    self.line.extend_from_slice(&buf[..n]);
                    if self.line.contains(&b'\n') || self.line.len() >= LINE_MAX {
                        return Ok(self.finish(false));
                    }
                }
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    let left = until.saturating_duration_since(Instant::now());
                    if left.is_zero() || !wait_readable(&self.file, left) {
                        return Ok(ReadyLine::Silent);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    fn finish(&mut self, closed: bool) -> ReadyLine {
        let end = self.line.iter().position(|&b| b == b'\n').unwrap_or(self.line.len());
        let line = String::from_utf8_lossy(&self.line[..end]).into_owned();
        self.line.clear();
        if closed && line.is_empty() { ReadyLine::Closed } else { ReadyLine::Line(line) }
    }
}

/// Start `binary` with `args` (after its own name as `argv[0]`) and `env`; its pid and the read end of
/// its readiness pipe.
pub(crate) fn spawn(
    binary: &Path,
    args: &[&OsStr],
    env: &[(&str, OsString)],
) -> io::Result<(u32, ReadyReader)> {
    let cstr = |bytes: &[u8]| CString::new(bytes).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput));
    let path = cstr(binary.as_os_str().as_bytes())?;
    let mut argv = vec![path.clone()];
    for arg in args {
        argv.push(cstr(arg.as_bytes())?);
    }
    // An entry that can't be one (a NUL in its value) is left out.
    let envp: Vec<CString> = env
        .iter()
        .filter_map(|(key, value)| CString::new([key.as_bytes(), b"=", value.as_bytes()].concat()).ok())
        .collect();
    let (read, write) = pipe()?;
    let write = off_ready_fd(write)?;
    let pid = posix_spawn(&path, &argv, &envp, write.as_raw_fd())?;
    // The program has its copy; ours would keep the pipe open after it closed it.
    drop(write);
    let ready = ReadyReader { file: File::from(read), line: Vec::new() };
    if let Err(err) = set_nonblocking(&ready.file) {
        // The child is ours and runs: it still has to be reaped.
        reap_later(pid);
        return Err(err);
    }
    Ok((pid, ready))
}

/// Send `signal` to `pid`. Sound for a child that has not been reaped yet: its pid is still its own,
/// even as a zombie.
#[allow(unsafe_code)] // libc FFI; plain integers.
pub(crate) fn signal(pid: u32, signal: Signal) {
    let signal = match signal {
        Signal::HangUp => libc::SIGHUP,
        Signal::Terminate => libc::SIGTERM,
        Signal::Kill => libc::SIGKILL,
    };
    // SAFETY: kill takes two integers; the pid is a child of ours that is not reaped yet.
    unsafe { libc::kill(pid as libc::pid_t, signal) };
}

/// Wait for the child on a small thread of its own, so it is reaped whenever it ends.
pub(crate) fn reap_later(pid: u32) {
    let reaper = std::thread::Builder::new().name("host reaper".into()).stack_size(64 * 1024);
    if let Err(err) = reaper.spawn(move || wait_for(pid)) {
        tracing::warn!(pid, "no thread to reap the session host on: {err}");
    }
}

#[allow(unsafe_code)] // libc FFI; the status goes into a local.
fn wait_for(pid: u32) {
    loop {
        let mut status = 0;
        // SAFETY: waitpid writes the status into a local; the pid is our child.
        let rc = unsafe { libc::waitpid(pid as libc::pid_t, &mut status, 0) };
        if rc >= 0 || io::Error::last_os_error().kind() != io::ErrorKind::Interrupted {
            return;
        }
    }
}

/// A pipe whose two ends are closed on exec (only the dup onto descriptor 3 reaches the program).
#[allow(unsafe_code)] // libc FFI; two descriptors written into a local array, owned at once.
fn pipe() -> io::Result<(OwnedFd, OwnedFd)> {
    let mut fds = [0; 2];
    // SAFETY: pipe writes two descriptors into the array; both are owned below and only there.
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: both descriptors were just made and nothing else owns them.
    let ends = unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
    for fd in fds {
        // SAFETY: fcntl sets a flag on a descriptor owned above.
        if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(ends)
}

/// `fd`, unless it is descriptor 3 itself: a `dup2` onto itself would leave it close-on-exec, and
/// the program would start without its pipe.
#[allow(unsafe_code)] // libc FFI; duplicating a descriptor owned here.
fn off_ready_fd(fd: OwnedFd) -> io::Result<OwnedFd> {
    if fd.as_raw_fd() != READY_FD {
        return Ok(fd);
    }
    // SAFETY: F_DUPFD_CLOEXEC duplicates an open descriptor onto the lowest free one above 3.
    let dup = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_DUPFD_CLOEXEC, READY_FD + 1) };
    if dup < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `dup` was just made and is owned only here.
    Ok(unsafe { OwnedFd::from_raw_fd(dup) })
}

#[allow(unsafe_code)] // libc FFI; a flag on a descriptor owned by the caller.
fn set_nonblocking(fd: &impl std::os::fd::AsRawFd) -> io::Result<()> {
    // SAFETY: F_GETFL and F_SETFL read and set the flags of an open descriptor.
    unsafe {
        let flags = libc::fcntl(fd.as_raw_fd(), libc::F_GETFL);
        if flags < 0 || libc::fcntl(fd.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

/// Whether `file` became readable within `timeout`.
#[allow(unsafe_code)] // libc FFI; one pollfd on the stack.
fn wait_readable(file: &File, timeout: Duration) -> bool {
    let mut fd = libc::pollfd { fd: file.as_raw_fd(), events: libc::POLLIN, revents: 0 };
    let ms = timeout.as_millis().clamp(1, i32::MAX as u128) as libc::c_int;
    // SAFETY: poll reads and writes the one pollfd it is given, which outlives the call.
    let n = unsafe { libc::poll(&mut fd, 1, ms) };
    n > 0
}

/// `posix_spawn` the program (see the module docs); its pid.
#[allow(unsafe_code)] // libc FFI: posix_spawn and its attribute and file-action objects.
fn posix_spawn(path: &CString, argv: &[CString], envp: &[CString], ready: RawFd) -> io::Result<u32> {
    let mut args: Vec<*mut c_char> = argv.iter().map(|a| a.as_ptr() as *mut c_char).collect();
    args.push(std::ptr::null_mut());
    let mut env: Vec<*mut c_char> = envp.iter().map(|e| e.as_ptr() as *mut c_char).collect();
    env.push(std::ptr::null_mut());
    let null = c"/dev/null";
    let check = |rc: libc::c_int| if rc == 0 { Ok(()) } else { Err(io::Error::from_raw_os_error(rc)) };
    // SAFETY: the attribute and file-action objects are initialised before use and destroyed on
    // every path after; every pointer handed over (the path, the NUL-terminated argument and
    // environment arrays of C strings, the signal sets, the pid) outlives the calls, and
    // posix_spawn copies what it keeps.
    unsafe {
        let mut actions: libc::posix_spawn_file_actions_t = std::mem::zeroed();
        check(libc::posix_spawn_file_actions_init(&mut actions))?;
        let mut attr: libc::posix_spawnattr_t = std::mem::zeroed();
        if let Err(err) = check(libc::posix_spawnattr_init(&mut attr)) {
            libc::posix_spawn_file_actions_destroy(&mut actions);
            return Err(err);
        }
        let mut flags = SETSID | libc::POSIX_SPAWN_SETSIGMASK as libc::c_short;
        flags |= libc::POSIX_SPAWN_SETSIGDEF as libc::c_short;
        #[cfg(target_vendor = "apple")]
        {
            flags |= libc::POSIX_SPAWN_CLOEXEC_DEFAULT as libc::c_short;
        }
        let mut empty: libc::sigset_t = std::mem::zeroed();
        let mut all: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut empty);
        libc::sigfillset(&mut all);
        libc::sigdelset(&mut all, libc::SIGKILL);
        libc::sigdelset(&mut all, libc::SIGSTOP);
        let setup =
            check(libc::posix_spawn_file_actions_addopen(&mut actions, 0, null.as_ptr(), libc::O_RDONLY, 0))
                .and_then(|()| {
                    check(libc::posix_spawn_file_actions_addopen(
                        &mut actions,
                        1,
                        null.as_ptr(),
                        libc::O_WRONLY,
                        0,
                    ))
                })
                .and_then(|()| {
                    check(libc::posix_spawn_file_actions_addopen(
                        &mut actions,
                        2,
                        null.as_ptr(),
                        libc::O_WRONLY,
                        0,
                    ))
                })
                .and_then(|()| check(libc::posix_spawn_file_actions_adddup2(&mut actions, ready, READY_FD)))
                .and_then(|()| check(libc::posix_spawnattr_setflags(&mut attr, flags)))
                .and_then(|()| check(libc::posix_spawnattr_setsigmask(&mut attr, &empty)))
                .and_then(|()| check(libc::posix_spawnattr_setsigdefault(&mut attr, &all)));
        let mut pid: libc::pid_t = 0;
        let spawned = setup.and_then(|()| {
            check(libc::posix_spawn(&mut pid, path.as_ptr(), &actions, &attr, args.as_ptr(), env.as_ptr()))
        });
        libc::posix_spawnattr_destroy(&mut attr);
        libc::posix_spawn_file_actions_destroy(&mut actions);
        spawned.map(|()| pid as u32)
    }
}
