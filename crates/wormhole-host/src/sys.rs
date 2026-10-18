//! What the host asks of the system directly: its own hygiene at start-up, the readiness pipe,
//! signals, the peer of a connection, the QoS class of its threads. Each FFI call sits in a small
//! function of its own with the reason it is sound.

use std::ffi::CString;
use std::fs::File;
use std::io::{self, Write as _};
use std::os::fd::{AsRawFd, FromRawFd as _, OwnedFd, RawFd};
use std::os::unix::ffi::OsStrExt as _;
use std::path::Path;

/// The descriptor the app hands the readiness pipe on.
const READY_FD: RawFd = 3;

/// The write end of the readiness pipe: one line, `ok <pid>` or `err <text>`, then closed.
pub struct ReadyPipe(Option<File>);

impl ReadyPipe {
    /// Take descriptor 3 if it is open, and keep it from the program the host starts.
    #[allow(unsafe_code)] // libc FFI on a descriptor number, and taking ownership of it.
    pub fn take() -> Self {
        // SAFETY: F_GETFD only asks about the descriptor; -1 means it is not open.
        let open = unsafe { libc::fcntl(READY_FD, libc::F_GETFD) } >= 0;
        if !open {
            return Self(None);
        }
        // SAFETY: as above; FD_CLOEXEC keeps the shell from inheriting the pipe (it would hold
        // the app's read end open for as long as the shell runs).
        unsafe { libc::fcntl(READY_FD, libc::F_SETFD, libc::FD_CLOEXEC) };
        // SAFETY: descriptor 3 is open, and nothing else in this process owns it: it is taken
        // first thing at start-up, before anything opens a file.
        Self(Some(File::from(unsafe { OwnedFd::from_raw_fd(READY_FD) })))
    }

    /// Whether the line was said already (or there is no pipe).
    pub fn is_spent(&self) -> bool {
        self.0.is_none()
    }

    /// Say `line` (one line, without its newline) and close the pipe. Only the first call says
    /// anything.
    pub fn say(&mut self, line: &str) {
        if let Some(mut file) = self.0.take() {
            let line: String = line.chars().map(|c| if c == '\n' || c == '\r' { ' ' } else { c }).collect();
            let _ = file.write_all(format!("{line}\n").as_bytes());
        }
    }
}

/// A host's start-up hygiene (`session-host.md`, section 5 and spike 2): out of every folder, a
/// private umask, no standard streams, no core files (one would hold scrollback), and no
/// background policy inherited from the app that spawned it.
#[allow(unsafe_code)] // libc FFI; constant arguments and a NUL-terminated path.
pub fn hygiene() {
    // SAFETY: chdir gets a NUL-terminated literal; umask, setrlimit and setpriority take plain
    // values and a pointer to a local struct.
    unsafe {
        libc::chdir(c"/".as_ptr());
        libc::umask(0o077);
        let none = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
        libc::setrlimit(libc::RLIMIT_CORE, &none);
        #[cfg(target_os = "macos")]
        libc::setpriority(libc::PRIO_DARWIN_PROCESS, 0, 0);
    }
    null_stdio();
    set_user_interactive();
}

/// Point standard input, output and error at `/dev/null`: nothing reads them, and a descriptor
/// 0 to 2 that is closed would be handed to the next socket.
#[allow(unsafe_code)] // libc FFI on descriptor numbers.
fn null_stdio() {
    let Ok(null) = std::fs::OpenOptions::new().read(true).write(true).open("/dev/null") else { return };
    for fd in 0..=2 {
        // SAFETY: dup2 onto a standard descriptor; `null` stays open until after the loop.
        unsafe { libc::dup2(null.as_raw_fd(), fd) };
    }
}

/// Put the calling thread at the user-interactive QoS class (spike 2: an unset class may land on
/// the efficiency cores). For the main thread, the PTY's thread and the connections' thread.
#[allow(unsafe_code)] // libc FFI; constant arguments.
pub fn set_user_interactive() {
    #[cfg(target_os = "macos")]
    // SAFETY: changes only the calling thread's QoS class.
    unsafe {
        libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE, 0);
    }
}

/// Drop the whole environment the host was started with: the program gets exactly the one its
/// `Spawn` carries, and the host itself needs none. Call before any other thread exists.
#[allow(unsafe_code)] // `remove_var` races with other threads reading the environment.
pub fn clear_environment() {
    let names: Vec<_> = std::env::vars_os().map(|(name, _)| name).collect();
    for name in names {
        // SAFETY: called first thing in `main`, before the host starts any thread, so nothing can
        // read the environment while it changes.
        unsafe { std::env::remove_var(name) };
    }
}

/// The signals the host ends on.
const ENDING: [libc::c_int; 3] = [libc::SIGTERM, libc::SIGINT, libc::SIGHUP];

/// Block the ending signals in this thread and every thread it starts afterwards, so only
/// [`wait_signal`] receives them (the program's process resets its mask before it runs).
#[allow(unsafe_code)] // libc FFI on a local signal set.
pub fn block_signals() {
    // SAFETY: the set is local and initialized by sigemptyset before use.
    unsafe {
        let mut set: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut set);
        for signal in ENDING {
            libc::sigaddset(&mut set, signal);
        }
        libc::pthread_sigmask(libc::SIG_BLOCK, &set, std::ptr::null_mut());
    }
}

/// Wait for one of the ending signals (blocked by [`block_signals`]); which one came.
#[allow(unsafe_code)] // libc FFI on a local signal set.
pub fn wait_signal() -> libc::c_int {
    // SAFETY: as in `block_signals`; sigwait writes the signal into a local.
    unsafe {
        let mut set: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut set);
        for signal in ENDING {
            libc::sigaddset(&mut set, signal);
        }
        let mut got = 0;
        if libc::sigwait(&set, &mut got) == 0 { got } else { 0 }
    }
}

/// The effective uid of this process.
#[allow(unsafe_code)] // libc FFI; no arguments.
pub fn euid() -> u32 {
    // SAFETY: geteuid takes nothing and cannot fail.
    unsafe { libc::geteuid() }
}

/// The effective uid of the process at the other end of a Unix socket.
#[allow(unsafe_code)] // libc FFI; the descriptor is open for the call, the outputs are locals.
pub fn peer_uid(socket: &impl AsRawFd) -> io::Result<u32> {
    let (mut uid, mut gid) = (0, 0);
    // SAFETY: getpeereid writes two integers into locals; the descriptor is borrowed and open.
    if unsafe { libc::getpeereid(socket.as_raw_fd(), &mut uid, &mut gid) } == 0 {
        Ok(uid)
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Whether the peer's code signature is one the host serves.
///
/// TODO(session host, P1 before shipping; `session-host.md` section 9 and spike 3): a host signed
/// with a Developer ID must take the peer's audit token (`LOCAL_PEERTOKEN`), find its code
/// (`SecCodeCopyGuestWithAttributes` with `kSecGuestAttributeAudit`) and check it once per
/// connection against `anchor apple generic and identifier "app.getwormhole.terminal" (or the
/// `wh` identifier) and certificate 1[field.1.2.840.113635.100.6.2.6] and certificate
/// leaf[field.1.2.840.113635.100.6.1.13] and certificate leaf[subject.OU] = "<team id>"`, the team
/// id read from the host's own signature (`SecCodeCopySelf`), never compiled in, and never a
/// cdhash or a version. An ad-hoc or unsigned host (dev builds) skips the check: uid and the
/// `0700` directory only, as decided. Until then every build is treated as ad-hoc.
pub fn signature_allows(_socket: &impl AsRawFd) -> bool {
    true
}

/// Set a file's access and modification times to now (the long-path socket in `$TMPDIR` is kept
/// from the temporary-file cleaner by this).
#[allow(unsafe_code)] // libc FFI; a NUL-terminated path and a null times pointer.
pub fn touch(path: &Path) -> io::Result<()> {
    let path = CString::new(path.as_os_str().as_bytes()).map_err(|_| io::ErrorKind::InvalidInput)?;
    // SAFETY: the path is NUL-terminated and outlives the call; a null `times` means now.
    if unsafe { libc::utimes(path.as_ptr(), std::ptr::null()) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
