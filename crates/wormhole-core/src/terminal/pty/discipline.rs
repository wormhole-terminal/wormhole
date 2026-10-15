//! The slave's line discipline (termios(4)): what the kernel does to the bytes between the
//! program and the terminal.

use std::io;
use std::os::fd::{AsRawFd, OwnedFd};

/// Give the slave `stty sane`'s settings and UTF-8 input (`IUTF8`: the kernel's line editing
/// erases a whole character with one backspace instead of one byte). The line speed stays what
/// the kernel gave it: it does not change how fast the output is read (`logflood` read the same
/// at 9600 baud and at 38400 in alternating runs).
pub(super) fn apply(slave: &OwnedFd) -> io::Result<()> {
    let mut termios = get(slave)?;
    sane(&mut termios);
    set(slave, &termios)
}

#[allow(unsafe_code)] // libc FFI; the termios is a plain struct the call fills.
fn get(slave: &OwnedFd) -> io::Result<libc::termios> {
    // SAFETY: tcgetattr fills the struct, for which all-zero is a valid value.
    unsafe {
        let mut termios: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(slave.as_raw_fd(), &mut termios) < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(termios)
    }
}

#[allow(unsafe_code)] // libc FFI; tcsetattr only reads the struct.
fn set(slave: &OwnedFd, termios: &libc::termios) -> io::Result<()> {
    // SAFETY: the pointer is to a valid termios for the length of the call.
    if unsafe { libc::tcsetattr(slave.as_raw_fd(), libc::TCSANOW, termios) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// `stty sane` plus `iutf8` on `termios`, whatever the kernel's defaults were: signals, line
/// editing and echo on, carriage return mapped to newline on input and back on output, 8-bit
/// characters, flow control on, the break and parity handling and the odd output mappings off.
pub(super) fn sane(termios: &mut libc::termios) {
    termios.c_iflag |= libc::BRKINT | libc::ICRNL | libc::IMAXBEL | libc::IXON | libc::IXANY | libc::IUTF8;
    termios.c_iflag &= !(libc::IGNBRK
        | libc::INLCR
        | libc::IGNCR
        | libc::ISTRIP
        | libc::INPCK
        | libc::IGNPAR
        | libc::PARMRK
        | libc::IXOFF);
    termios.c_oflag |= libc::OPOST | libc::ONLCR;
    termios.c_oflag &= !(libc::OCRNL | libc::ONOCR | libc::ONLRET);
    termios.c_cflag = (termios.c_cflag & !(libc::CSIZE | libc::PARENB | libc::PARODD | libc::CSTOPB))
        | libc::CS8
        | libc::CREAD
        | libc::HUPCL;
    termios.c_lflag |= libc::ISIG
        | libc::ICANON
        | libc::IEXTEN
        | libc::ECHO
        | libc::ECHOE
        | libc::ECHOK
        | libc::ECHOKE
        | libc::ECHOCTL;
    termios.c_lflag &= !(libc::ECHONL | libc::NOFLSH | libc::TOSTOP);
    // DEL erases, as every modern keyboard sends.
    termios.c_cc[libc::VERASE] = 0x7f;
}
