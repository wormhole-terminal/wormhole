//! The terminal itself: its size, raw mode, the alternate screen, keys, and the status report that
//! tells us the terminal has read everything we wrote.

use std::fs::File;
use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Esc,
    CtrlC,
    Left,
    Right,
    Up,
    Down,
}

/// Everything the terminal sends us, read on a thread of its own so a frame never waits for a key.
pub struct Input {
    rx: Receiver<Vec<u8>>,
    pending: Vec<u8>,
}

impl Input {
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut stdin = io::stdin().lock();
            let mut buf = [0u8; 4096];
            loop {
                match stdin.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        Self { rx, pending: Vec::new() }
    }

    /// The keys pressed since the last call, never blocking.
    pub fn keys(&mut self) -> Vec<Key> {
        while let Ok(chunk) = self.rx.try_recv() {
            self.pending.extend_from_slice(&chunk);
        }
        let bytes = std::mem::take(&mut self.pending);
        parse_keys(&bytes)
    }

    /// Asks for the cursor position (DSR 6) and waits for the answer. The terminal answers only
    /// after it has parsed everything written before the question, so this is the moment the
    /// output has really been consumed. `None` on a timeout.
    pub fn sync(&mut self, out: &mut impl Write, timeout: Duration) -> io::Result<Option<Duration>> {
        let start = Instant::now();
        out.write_all(b"\x1b[6n")?;
        out.flush()?;
        let deadline = start + timeout;
        loop {
            if let Some(end) = find_cpr(&self.pending) {
                self.pending.drain(..end);
                return Ok(Some(start.elapsed()));
            }
            let left = deadline.saturating_duration_since(Instant::now());
            match self.rx.recv_timeout(left) {
                Ok(chunk) => self.pending.extend_from_slice(&chunk),
                Err(RecvTimeoutError::Timeout) => return Ok(None),
                Err(RecvTimeoutError::Disconnected) => return Err(io::Error::other("stdin closed")),
            }
        }
    }
}

/// The end of the first cursor position report (`ESC [ row ; col R`) in `buf`.
fn find_cpr(buf: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i + 1 < buf.len() {
        if buf[i] == 0x1b && buf[i + 1] == b'[' {
            let mut j = i + 2;
            while j < buf.len() && (buf[j].is_ascii_digit() || buf[j] == b';') {
                j += 1;
            }
            if j < buf.len() && buf[j] == b'R' {
                return Some(j + 1);
            }
        }
        i += 1;
    }
    None
}

fn parse_keys(bytes: &[u8]) -> Vec<Key> {
    let mut keys = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            0x03 => keys.push(Key::CtrlC),
            0x1b if bytes.get(i + 1) == Some(&b'[') || bytes.get(i + 1) == Some(&b'O') => {
                let key = match bytes.get(i + 2) {
                    Some(b'A') => Some(Key::Up),
                    Some(b'B') => Some(Key::Down),
                    Some(b'C') => Some(Key::Right),
                    Some(b'D') => Some(Key::Left),
                    _ => None,
                };
                keys.extend(key);
                // Skip the whole sequence, whatever it was.
                let mut j = i + 2;
                while j < bytes.len() && !(0x40..=0x7e).contains(&bytes[j]) {
                    j += 1;
                }
                i = j;
            }
            0x1b => keys.push(Key::Esc),
            b if b.is_ascii() && !b.is_ascii_control() => keys.push(Key::Char(b as char)),
            _ => {}
        }
        i += 1;
    }
    keys
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_cursor_report_among_other_bytes() {
        assert_eq!(find_cpr(b"q\x1b[12;40R"), Some(9));
        assert_eq!(find_cpr(b"\x1b[12;40"), None);
    }

    #[test]
    fn reads_arrows_and_a_lone_escape() {
        assert_eq!(parse_keys(b"\x1b[Cq\x1b"), vec![Key::Right, Key::Char('q'), Key::Esc]);
    }
}
