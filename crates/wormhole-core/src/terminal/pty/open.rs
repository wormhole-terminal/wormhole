//! A pseudo-terminal: the master and slave descriptors and the window size the kernel keeps for
//! it (openpty(3), tty(4)); `discipline` sets the slave's line discipline.

use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::sync::{Mutex, PoisonError};

use super::discipline;
use crate::terminal::WindowSize;

/// The two ends of a new pseudo-terminal.
pub(super) struct Pair {
    /// Ours: non-blocking and close-on-exec, so no program we start holds it open.
    pub master: File,
    /// The program's: it becomes the child's standard streams and controlling terminal. Dropped
    /// in the parent once the child has it, or the master never reports the child's end.
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
    let (master, slave) = open_pair(winsize(size))?;
    add_flag(master.as_raw_fd(), Flag::CloseOnExec)?;
    add_flag(slave.as_raw_fd(), Flag::CloseOnExec)?;
    add_flag(master.as_raw_fd(), Flag::NonBlocking)?;
    discipline::apply(&slave)?;
    Ok(Pair { master: File::from(master), slave })
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
