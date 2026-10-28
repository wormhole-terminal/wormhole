//! The terminal itself: its size, raw mode and the alternate screen here; what it sends us in
//! `input` (keys, and the status report that tells us it has read everything we wrote) and the
//! replies it gives to our questions in `replies`.

mod input;
mod replies;

use std::fs::File;
use std::io::{self, Write};
use std::process::{Command, Stdio};

pub use input::{Input, Key};
#[cfg(test)]
pub use replies::parse_replies;
pub use replies::{Answer, Cursor, Reply, ReplyKind, Reports, escape};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub cols: u16,
    pub rows: u16,
    /// The window's size in pixels, when the terminal reports it (0 otherwise).
    pub px_w: u16,
    pub px_h: u16,
}

#[repr(C)]
struct Winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

// Declared here rather than through the libc crate so the tool stays std only.
#[allow(unsafe_code)]
unsafe extern "C" {
    fn ioctl(fd: i32, request: std::ffi::c_ulong, ...) -> i32;
}

#[cfg(target_os = "macos")]
const TIOCGWINSZ: std::ffi::c_ulong = 0x4008_7468;
#[cfg(not(target_os = "macos"))]
const TIOCGWINSZ: std::ffi::c_ulong = 0x5413;

/// The terminal's size, asked every frame: an ioctl is cheap and a resize then shows at once.
#[allow(unsafe_code)]
pub fn size() -> Size {
    let mut ws = Winsize { ws_row: 0, ws_col: 0, ws_xpixel: 0, ws_ypixel: 0 };
    // SAFETY: TIOCGWINSZ writes one `struct winsize`, which `Winsize` matches field for field; fd 1
    // is stdout and the call only fails (returning -1, leaving zeros) when it isn't a terminal.
    let ok = unsafe { ioctl(1, TIOCGWINSZ, &mut ws as *mut Winsize) } == 0;
    if !ok || ws.ws_col == 0 || ws.ws_row == 0 {
        return Size { cols: 80, rows: 24, px_w: 0, px_h: 0 };
    }
    Size { cols: ws.ws_col, rows: ws.ws_row, px_w: ws.ws_xpixel, px_h: ws.ws_ypixel }
}

fn stty(args: &[&str]) -> io::Result<String> {
    let out =
        Command::new("stty").args(args).stdin(File::open("/dev/tty")?).stderr(Stdio::null()).output()?;
    if !out.status.success() {
        return Err(io::Error::other(format!("stty {} failed", args.join(" "))));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Raw mode for as long as it lives: keys arrive one by one, unechoed, and ^C is a byte rather than
/// a signal, so we always get to restore the screen.
pub struct RawMode {
    saved: String,
}

impl RawMode {
    pub fn enter() -> io::Result<Self> {
        let saved = stty(&["-g"])?;
        stty(&["raw", "-echo"])?;
        Ok(Self { saved })
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = stty(&[&self.saved]);
    }
}

/// The alternate screen with the cursor hidden; everything is put back on drop.
pub struct AltScreen;

impl AltScreen {
    pub fn enter() -> io::Result<Self> {
        let mut out = io::stdout().lock();
        out.write_all(b"\x1b[?1049h\x1b[?25l\x1b[?7l\x1b[2J\x1b[H")?;
        out.flush()?;
        Ok(Self)
    }
}

impl Drop for AltScreen {
    fn drop(&mut self) {
        let mut out = io::stdout().lock();
        let _ = out.write_all(b"\x1b[?2026l\x1b[0m\x1b[r\x1b[?7h\x1b[?25h\x1b[?1049l");
        let _ = out.flush();
    }
}

/// The terminal's name, for headings and saved results.
pub fn terminal_name() -> String {
    for var in ["TERM_PROGRAM", "TERMINAL_EMULATOR", "TERM"] {
        if let Ok(v) = std::env::var(var)
            && !v.is_empty()
        {
            let version = std::env::var("TERM_PROGRAM_VERSION").unwrap_or_default();
            return if var == "TERM_PROGRAM" && !version.is_empty() { format!("{v} {version}") } else { v };
        }
    }
    "unknown".to_string()
}
