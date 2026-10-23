//! The terminal under the welcome: raw mode (keys arrive as bytes, nothing is echoed or line-edited),
//! the size, and waiting for a key. A few `libc` calls on the standard descriptors; each carries its
//! SAFETY note.
//!
//! Raw mode is undone on every way out: the guard's `Drop` runs when the program returns and when a
//! panic unwinds, and `restore` does it early, before the shell is started. There are no signal
//! handlers: the size is read again on every tick instead of waiting for SIGWINCH, and Ctrl-C arrives as
//! a byte because raw mode turns signal keys off.

use std::io;

const STDIN: libc::c_int = 0;
const STDOUT: libc::c_int = 1;

/// The terminal in raw mode until this is dropped or [`restore`](Self::restore)d.
pub struct RawMode {
    saved: libc::termios,
    active: bool,
}

impl RawMode {
    #[allow(unsafe_code)] // libc: termios calls on a descriptor we checked is a terminal.
    pub fn enter() -> io::Result<RawMode> {
        // SAFETY: `termios` is plain data; zeroed is a valid value to be filled in by tcgetattr.
        let mut saved: libc::termios = unsafe { std::mem::zeroed() };
        // SAFETY: STDIN is a valid descriptor and `saved` is writable for one termios.
        if unsafe { libc::tcgetattr(STDIN, &mut saved) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mut raw = saved;
        // SAFETY: `raw` is a live termios.
        unsafe { libc::cfmakeraw(&mut raw) };
        // Reads wait for a byte (we only read after poll says there is one).
        raw.c_cc[libc::VMIN] = 1;
        raw.c_cc[libc::VTIME] = 0;
        // SAFETY: STDIN is a valid descriptor and `raw` a fully initialised termios.
        if unsafe { libc::tcsetattr(STDIN, libc::TCSANOW, &raw) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(RawMode { saved, active: true })
    }

    /// Put the terminal back as it was. Does nothing the second time.
    #[allow(unsafe_code)] // libc: tcsetattr with the settings tcgetattr gave.
    pub fn restore(&mut self) {
        if self.active {
            self.active = false;
            // SAFETY: STDIN is a valid descriptor and `saved` is what tcgetattr filled in. TCSANOW keeps
            // what was typed ahead for the shell.
            unsafe { libc::tcsetattr(STDIN, libc::TCSANOW, &self.saved) };
        }
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        self.restore();
    }
}

/// The terminal's size in cells (columns, rows), when it says. A terminal that reports no size at all
/// (zero) is treated as unknown.
#[allow(unsafe_code)] // libc: TIOCGWINSZ on a descriptor.
pub(crate) fn size() -> Option<(usize, usize)> {
    // SAFETY: `winsize` is plain data; zeroed is a valid value for the ioctl to fill in.
    let mut size: libc::winsize = unsafe { std::mem::zeroed() };
    // SAFETY: STDOUT is a valid descriptor and `size` is writable for one winsize, which is what
    // TIOCGWINSZ writes.
    let status = unsafe { libc::ioctl(STDOUT, libc::TIOCGWINSZ, &mut size) };
    (status == 0 && size.ws_col > 0 && size.ws_row > 0).then_some((size.ws_col.into(), size.ws_row.into()))
}

/// Read what has been typed, waiting at most `wait_ms`; empty when nothing was, `None` at the end of
/// the input (the terminal went away).
#[allow(unsafe_code)] // libc: poll and read on stdin.
pub(crate) fn read_input(buf: &mut [u8], wait_ms: i32) -> Option<usize> {
    let mut fd = libc::pollfd { fd: STDIN, events: libc::POLLIN, revents: 0 };
    // SAFETY: `fd` is one live pollfd and the count says so.
    let ready = unsafe { libc::poll(&mut fd, 1, wait_ms) };
    if ready <= 0 {
        // A timeout, or a signal in the way: nothing yet.
        return Some(0);
    }
    if fd.revents & libc::POLLHUP != 0 && fd.revents & libc::POLLIN == 0 {
        return None;
    }
    // SAFETY: `buf` is writable for its length.
    let n = unsafe { libc::read(STDIN, buf.as_mut_ptr().cast(), buf.len()) };
    match n {
        0 => None,
        n if n < 0 => Some(0),
        n => Some(n as usize),
    }
}
