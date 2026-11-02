//! A pseudo-terminal: the master and slave descriptors and the window size the kernel keeps for
//! it (openpty(3), tty(4)); `discipline` sets the slave's line discipline.

use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::sync::{Mutex, PoisonError};

use super::discipline;
use crate::WindowSize;

/// The two ends of a new pseudo-terminal.
pub(super) struct Pair {
    /// Ours: non-blocking and close-on-exec, so no program we start holds it open.
    pub master: File,
    /// The program's: it becomes the child's standard streams and controlling terminal. The
    /// reader keeps it only where the system loses a program's last output without it, and
    /// lets go of it when the program has exited and its output is read (`reader::Core::slave`).
    pub slave: OwnedFd,
}

/// A property of a descriptor that [`add_flag`] turns on.
#[derive(Clone, Copy)]
enum Flag {
    /// Closed by `exec`, so no program started later inherits it.
    CloseOnExec,
    /// Reads and writes never wait: the reader thread polls the master, and a write that doesn't
    /// fit waits for the master to be writable again instead of stopping everything.
    NonBlocking,
}

/// Open a pseudo-terminal of `size` with a sane line discipline that expects UTF-8.
pub(super) fn open(size: WindowSize) -> io::Result<Pair> {
    // macOS's `openpty` resolves the new device's name through a buffer shared by all threads,
    // so two terminals opened at once can get each other's device (`ENXIO`, seen when tests
    // spawn in parallel). One at a time; an open takes microseconds in here.
    static OPENING: Mutex<()> = Mutex::new(());
    let _one_at_a_time = OPENING.lock().unwrap_or_else(PoisonError::into_inner);
    let (master, slave) = open_pair_again(winsize(size))?;
    add_flag(master.as_raw_fd(), Flag::CloseOnExec)?;
    add_flag(slave.as_raw_fd(), Flag::CloseOnExec)?;
    add_flag(master.as_raw_fd(), Flag::NonBlocking)?;
    discipline::apply(&slave)?;
    Ok(Pair { master: File::from(master), slave })
}

/// How many times [`open_pair_again`] tries.
const OPEN_TRIES: usize = 6;

/// Whether `err` is the one failure a later try cures: `ENXIO`, which `openpty` reports when the kernel's naming
/// of a new device races another process's. macOS has been seen to give it negated (`errno` literally `-6`, shown
/// as "Unknown error: -6"), so both are taken. Running out of descriptors (`EMFILE`, `ENFILE`) or of
/// pseudo-terminals does not pass in a few milliseconds and is reported at once.
fn is_device_race(err: &io::Error) -> bool {
    err.raw_os_error().is_some_and(|code| code == libc::ENXIO || code == -libc::ENXIO)
}

/// [`open_pair`], again a moment later when the kernel's device naming raced another process. The lock above makes
/// one process open one at a time, but the naming is shared by all processes, and many hosts starting at the same
/// moment (a relaunch attaching or restoring a window of terminals) can make it fail once; a second try a few
/// milliseconds on finds the device. Any other failure, and the last of these, is logged with what the system
/// can say about the cause.
fn open_pair_again(size: libc::winsize) -> io::Result<(OwnedFd, OwnedFd)> {
    let result = retrying(
        OPEN_TRIES,
        is_device_race,
        |tried| std::thread::sleep(std::time::Duration::from_millis(5 * tried as u64)),
        || open_pair(size),
    );
    if let Err(err) = &result {
        tracing::error!(
            raw_os_error = ?err.raw_os_error(),
            open_ptys = open_pty_count(),
            ptmx_max = ptmx_max(),
            "openpty failed: {err}"
        );
    }
    result
}

/// How many pseudo-terminal slaves exist now (`/dev/ttys###`), for the log of a failure.
fn open_pty_count() -> Option<usize> {
    let entries = std::fs::read_dir("/dev").ok()?;
    Some(
        entries
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .and_then(|n| n.strip_prefix("ttys"))
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            })
            .count(),
    )
}

/// The system's limit on pseudo-terminals (`kern.tty.ptmx_max` on macOS), for the log of a failure.
#[allow(unsafe_code)] // libc FFI: sysctlbyname reads one integer into a local.
fn ptmx_max() -> Option<i32> {
    #[cfg(target_os = "macos")]
    {
        let mut value: libc::c_int = 0;
        let mut len = std::mem::size_of::<libc::c_int>();
        // SAFETY: the name is a NUL-terminated literal, `value` and `len` are valid locals of the size given.
        let rc = unsafe {
            libc::sysctlbyname(
                c"kern.tty.ptmx_max".as_ptr(),
                (&raw mut value).cast(),
                &mut len,
                std::ptr::null_mut(),
                0,
            )
        };
        (rc == 0).then_some(value)
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

/// Run `attempt` up to `tries` times; after a failure `retry` accepts that is not the last, `pause(n)` for the nth
/// time. The first other failure, or the last, is the error.
fn retrying<T>(
    tries: usize,
    retry: impl Fn(&io::Error) -> bool,
    mut pause: impl FnMut(usize),
    mut attempt: impl FnMut() -> io::Result<T>,
) -> io::Result<T> {
    let mut tried = 0;
    loop {
        match attempt() {
            Ok(done) => return Ok(done),
            Err(err) if tried + 1 < tries && retry(&err) => {
                tried += 1;
                tracing::warn!("opening a pseudo-terminal failed ({err}); trying again ({tried})");
                pause(tried);
            }
            Err(err) => return Err(err),
        }
    }
}

/// The kernel's window size for `size`: cells and pixels (`TIOCSWINSZ`; programs read the pixels
/// with `TIOCGWINSZ` to size images). A product that doesn't fit stops at the largest size.
pub(super) fn winsize(size: WindowSize) -> libc::winsize {
    libc::winsize {
        ws_row: size.num_lines,
        ws_col: size.num_cols,
        ws_xpixel: size.num_cols.saturating_mul(size.cell_width),
        ws_ypixel: size.num_lines.saturating_mul(size.cell_height),
    }
}

/// Tell the kernel the terminal's new size; it signals the foreground group with `SIGWINCH`.
#[allow(unsafe_code)] // libc FFI; the descriptor is open for as long as the borrow lasts.
pub(super) fn set_size(master: &File, size: WindowSize) -> io::Result<()> {
    let size = winsize(size);
    // SAFETY: TIOCSWINSZ only reads one `winsize` through the pointer.
    let rc = unsafe { libc::ioctl(master.as_raw_fd(), libc::TIOCSWINSZ, &size) };
    if rc < 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
}

#[allow(unsafe_code)] // libc FFI; the descriptors are fresh and owned from here on.
fn open_pair(mut size: libc::winsize) -> io::Result<(OwnedFd, OwnedFd)> {
    let (mut master, mut slave): (RawFd, RawFd) = (-1, -1);
    // SAFETY: both out-pointers are valid for a write, the name is not asked for (null), the
    // default termios is wanted (null) and `size` is valid (declared mutable; only read).
    let rc = unsafe {
        libc::openpty(&mut master, &mut slave, std::ptr::null_mut(), std::ptr::null_mut(), &mut size)
    };
    if rc < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: openpty succeeded, so both are open descriptors that nothing else owns.
    Ok(unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) })
}

#[allow(unsafe_code)] // libc FFI; fcntl only reads and writes the descriptor's flags.
fn add_flag(fd: RawFd, flag: Flag) -> io::Result<()> {
    let (get, set, bit) = match flag {
        Flag::CloseOnExec => (libc::F_GETFD, libc::F_SETFD, libc::FD_CLOEXEC),
        Flag::NonBlocking => (libc::F_GETFL, libc::F_SETFL, libc::O_NONBLOCK),
    };
    // SAFETY: fcntl with these commands takes plain integers and touches nothing else.
    unsafe {
        let flags = libc::fcntl(fd, get);
        if flags < 0 || libc::fcntl(fd, set, flags | bit) < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

#[cfg(test)]
mod retry_tests {
    use super::*;

    /// The kernel's device naming races now and then when many terminals open at once; a later try finds the
    /// device. Only that failure is tried again.
    #[test]
    fn a_device_race_is_tried_again_and_nothing_else_is() {
        for code in [libc::ENXIO, -libc::ENXIO] {
            let mut calls = 0;
            let mut pauses = Vec::new();
            let got = retrying(
                OPEN_TRIES,
                is_device_race,
                |n| pauses.push(n),
                || {
                    calls += 1;
                    if calls < 3 { Err(io::Error::from_raw_os_error(code)) } else { Ok(calls) }
                },
            );
            assert_eq!((got.unwrap(), pauses.as_slice()), (3, [1, 2].as_slice()), "errno {code}");
        }
        // A lasting race is the error after the last try.
        let mut calls = 0;
        let err = retrying::<()>(
            OPEN_TRIES,
            is_device_race,
            |_| {},
            || {
                calls += 1;
                Err(io::Error::from_raw_os_error(libc::ENXIO))
            },
        );
        assert_eq!((calls, err.unwrap_err().raw_os_error()), (OPEN_TRIES, Some(libc::ENXIO)));
        // Out of descriptors or terminals does not pass: one try.
        for code in [libc::EMFILE, libc::ENFILE, libc::EACCES] {
            let mut calls = 0;
            let err = retrying::<()>(
                OPEN_TRIES,
                is_device_race,
                |_| {},
                || {
                    calls += 1;
                    Err(io::Error::from_raw_os_error(code))
                },
            );
            assert_eq!((calls, err.unwrap_err().raw_os_error()), (1, Some(code)));
        }
    }

    #[test]
    fn what_the_log_of_a_failure_says_can_be_asked() {
        assert!(open_pty_count().is_some());
        if cfg!(target_os = "macos") {
            assert!(ptmx_max().is_some_and(|max| max > 0));
        }
    }

    #[test]
    fn a_pty_opens_when_the_system_is_asked() {
        assert!(
            open_pair_again(winsize(WindowSize {
                num_lines: 24,
                num_cols: 80,
                cell_width: 8,
                cell_height: 16
            }))
            .is_ok()
        );
    }
}
