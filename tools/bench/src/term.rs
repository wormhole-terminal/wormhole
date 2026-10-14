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
    started: Instant,
    /// Every chunk read, with when it came, when `record_raw` asked for it (`ttyprobe --raw`).
    tap: Option<Vec<String>>,
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
        Self::with_receiver(rx)
    }

    /// An input fed from a channel instead of stdin: what the tests use.
    fn with_receiver(rx: Receiver<Vec<u8>>) -> Self {
        Self { rx, pending: Vec::new(), started: Instant::now(), tap: None }
    }

    /// Starts keeping every byte read, escaped and timed, for `raw_log`.
    pub fn record_raw(&mut self) {
        self.tap.get_or_insert_with(Vec::new);
    }

    /// The chunks read since `record_raw`, one line each: `+12.345 ms  \x1b[?64c`.
    pub fn raw_log(&self) -> &[String] {
        self.tap.as_deref().unwrap_or(&[])
    }

    /// A chunk the reader thread delivered: queued for whoever parses it, and logged when asked.
    fn take(&mut self, chunk: Vec<u8>) {
        if let Some(tap) = &mut self.tap {
            tap.push(format!("+{:.3} ms  {}", self.started.elapsed().as_secs_f64() * 1e3, escape(&chunk)));
        }
        self.pending.extend_from_slice(&chunk);
    }

    /// The keys pressed since the last call, never blocking.
    pub fn keys(&mut self) -> Vec<Key> {
        while let Ok(chunk) = self.rx.try_recv() {
            self.take(chunk);
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
                Ok(chunk) => self.take(chunk),
                Err(RecvTimeoutError::Timeout) => return Ok(None),
                Err(RecvTimeoutError::Disconnected) => return Err(io::Error::other("stdin closed")),
            }
        }
    }

    /// Writes `request`, then DA1 (`CSI c`), and collects what the terminal sends until the DA1
    /// reply. Terminals answer in order and practically all answer DA1, so a DA1 reply without the
    /// request's own reply means "not supported", known at once instead of after a timeout. `da1` is
    /// `None` when even that never came within `timeout`.
    pub fn ask(&mut self, out: &mut impl Write, request: &[u8], timeout: Duration) -> io::Result<Answer> {
        let mut ans = Answer::default();
        // Whatever is still unread belongs to an earlier question; keep it apart so it can't be
        // mistaken for an answer, but report it.
        while let Ok(chunk) = self.rx.try_recv() {
            self.take(chunk);
        }
        ans.stale = std::mem::take(&mut self.pending);
        let start = Instant::now();
        out.write_all(request)?;
        out.write_all(b"\x1b[c")?;
        out.flush()?;
        let deadline = start + timeout;
        let mut at = 0u64;
        loop {
            let (found, used) = parse_replies(&self.pending);
            self.pending.drain(..used);
            for r in found {
                if ans.da1.is_some() {
                    ans.late.push(r);
                } else if is_da1(&r) {
                    ans.da1 = Some((r.body, at));
                } else {
                    ans.replies.push((r, at));
                }
            }
            if ans.da1.is_some() {
                break;
            }
            let left = deadline.saturating_duration_since(Instant::now());
            match self.rx.recv_timeout(left) {
                Ok(chunk) => {
                    at = start.elapsed().as_micros() as u64;
                    ans.raw.extend_from_slice(&chunk);
                    self.take(chunk);
                }
                // A closed input sends nothing more: what came before it is the answer, as at a timeout.
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => break,
            }
        }
        ans.waited = start.elapsed().as_micros() as u64;
        Ok(ans)
    }

    /// Writes `request` (which asks for `count` cursor reports, DSR 6) and collects the reports.
    /// Fewer come back when the terminal doesn't answer within `timeout`.
    pub fn cursor_reports(
        &mut self,
        out: &mut impl Write,
        request: &[u8],
        count: usize,
        timeout: Duration,
    ) -> io::Result<Reports> {
        let mut got = Reports::default();
        while let Ok(chunk) = self.rx.try_recv() {
            self.take(chunk);
        }
        got.stale = std::mem::take(&mut self.pending);
        let start = Instant::now();
        out.write_all(request)?;
        out.flush()?;
        let deadline = start + timeout;
        let mut at = 0u64;
        while got.reports.len() < count {
            if let Some(c) = parse_cpr(&self.pending) {
                got.reports.push(Cursor {
                    row: c.row,
                    col: c.col,
                    raw: self.pending[c.start..c.end].to_vec(),
                    micros: at,
                });
                self.pending.drain(..c.end);
                continue;
            }
            let left = deadline.saturating_duration_since(Instant::now());
            match self.rx.recv_timeout(left) {
                Ok(chunk) => {
                    at = start.elapsed().as_micros() as u64;
                    got.raw.extend_from_slice(&chunk);
                    self.take(chunk);
                }
                // A closed input sends nothing more: what came before it is the answer, as at a timeout.
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => break,
            }
        }
        Ok(got)
    }
}

/// What came back to `Input::ask`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Answer {
    /// The replies before the DA1 reply, each with the microseconds after the request it arrived.
    pub replies: Vec<(Reply, u64)>,
    /// The DA1 reply's body (`?64;1;4c`) and when it came; `None` when it never did.
    pub da1: Option<(String, u64)>,
    /// Replies that arrived in the same read after the DA1 reply: out of order for a terminal
    /// that answers in order.
    pub late: Vec<Reply>,
    /// Every byte read while waiting.
    pub raw: Vec<u8>,
    /// Bytes that were already waiting when the request was sent (a late answer to an earlier one).
    pub stale: Vec<u8>,
    /// Microseconds until the DA1 reply, or until the wait gave up.
    pub waited: u64,
}

/// A cursor position report (`CSI row ; col R`) and how it arrived.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cursor {
    pub row: u16,
    pub col: u16,
    pub raw: Vec<u8>,
    pub micros: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reports {
    pub reports: Vec<Cursor>,
    pub raw: Vec<u8>,
    pub stale: Vec<u8>,
}

/// What a terminal sends back to a query, apart from keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplyKind {
    /// `ESC [ … final`: `body` is everything after the `[`, final byte included.
    Csi,
    /// `ESC P … ST`: `body` is the payload.
    Dcs,
    /// `ESC ] … BEL|ST`
    Osc,
    /// `ESC _ … ST`
    Apc,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    pub kind: ReplyKind,
    pub body: String,
}

/// A device attributes reply (`CSI ? Ps ; … c`): what the sentinel query returns.
pub fn is_da1(r: &Reply) -> bool {
    r.kind == ReplyKind::Csi
        && r.body.len() >= 2
        && r.body.starts_with('?')
        && r.body.ends_with('c')
        && r.body[1..r.body.len() - 1].bytes().all(|b| b.is_ascii_digit() || b == b';')
}

/// A string from the terminal longer than this is not a reply to anything we asked.
const MAX_STRING: usize = 64 * 1024;

enum Scan {
    /// A whole sequence of this many bytes, and the reply it is (strings we don't use give none).
    Done(usize, Option<Reply>),
    /// The start of one; the rest hasn't arrived.
    Partial,
    /// Not a sequence at all.
    Not,
}

/// The sequence starting at the `ESC` that opens `b`.
fn scan(b: &[u8]) -> Scan {
    if b.len() < 2 {
        return Scan::Partial;
    }
    match b[1] {
        b'[' => {
            let mut j = 2;
            while j < b.len() && (0x30..=0x3f).contains(&b[j]) {
                j += 1;
            }
            while j < b.len() && (0x20..=0x2f).contains(&b[j]) {
                j += 1;
            }
            match b.get(j) {
                None => Scan::Partial,
                Some(0x40..=0x7e) => Scan::Done(
                    j + 1,
                    Some(Reply {
                        kind: ReplyKind::Csi,
                        body: String::from_utf8_lossy(&b[2..=j]).into_owned(),
                    }),
                ),
                Some(_) => Scan::Not,
            }
        }
        c @ (b'P' | b']' | b'_' | b'^' | b'X') => {
            for k in 2..b.len().min(MAX_STRING) {
                let end = match b[k] {
                    0x07 if c == b']' => Some((k, k + 1)),
                    0x1b => match b.get(k + 1) {
                        None => return Scan::Partial,
                        Some(b'\\') => Some((k, k + 2)),
                        Some(_) => return Scan::Not,
                    },
                    0x18 | 0x1a => return Scan::Not,
                    _ => None,
                };
                if let Some((body_end, len)) = end {
                    let kind = match c {
                        b'P' => Some(ReplyKind::Dcs),
                        b']' => Some(ReplyKind::Osc),
                        b'_' => Some(ReplyKind::Apc),
                        _ => None,
                    };
                    let reply = kind.map(|kind| Reply {
                        kind,
                        body: String::from_utf8_lossy(&b[2..body_end]).into_owned(),
                    });
                    return Scan::Done(len, reply);
                }
            }
            if b.len() < MAX_STRING { Scan::Partial } else { Scan::Not }
        }
        _ => Scan::Not,
    }
}

/// The replies in `bytes` (CSI, DCS, OSC and APC sequences, in order) and how many bytes were used.
/// Keys and other noise between them are dropped; an unfinished sequence at the end is left
/// unused so the rest can be added when it arrives.
pub fn parse_replies(bytes: &[u8]) -> (Vec<Reply>, usize) {
    let mut replies = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != 0x1b {
            i += 1;
            continue;
        }
        match scan(&bytes[i..]) {
            Scan::Done(len, reply) => {
                replies.extend(reply);
                i += len;
            }
            Scan::Partial => break,
            Scan::Not => i += 1,
        }
    }
    (replies, i)
}

/// Bytes as ASCII text a person or a program can read back: printable ASCII as is, `\` doubled,
/// control bytes as `\x1b`, other characters as `\u{1f600}`, invalid UTF-8 as `\xNN`.
pub fn escape(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len());
    for chunk in bytes.utf8_chunks() {
        for c in chunk.valid().chars() {
            match c {
                '\\' => s.push_str("\\\\"),
                ' '..='~' => s.push(c),
                c if (c as u32) < 0x80 => s.push_str(&format!("\\x{:02x}", c as u32)),
                c => s.push_str(&format!("\\u{{{:x}}}", c as u32)),
            }
        }
        for b in chunk.invalid() {
            s.push_str(&format!("\\x{b:02x}"));
        }
    }
    s
}

/// A cursor position report found in a buffer: where it sits and what it says.
struct Cpr {
    start: usize,
    end: usize,
    row: u16,
    col: u16,
}

/// The first cursor position report (`ESC [ row ; col R`) in `buf`.
fn parse_cpr(buf: &[u8]) -> Option<Cpr> {
    let mut i = 0;
    while i + 1 < buf.len() {
        if buf[i] == 0x1b && buf[i + 1] == b'[' {
            let mut j = i + 2;
            while j < buf.len() && (buf[j].is_ascii_digit() || buf[j] == b';') {
                j += 1;
            }
            if j < buf.len() && buf[j] == b'R' {
                let text = std::str::from_utf8(&buf[i + 2..j]).unwrap_or("");
                if let Some((r, c)) = text.split_once(';')
                    && let (Ok(row), Ok(col)) = (r.parse(), c.parse())
                {
                    return Some(Cpr { start: i, end: j + 1, row, col });
                }
            }
        }
        i += 1;
    }
    None
}

/// The end of the first cursor position report in `buf`.
fn find_cpr(buf: &[u8]) -> Option<usize> {
    parse_cpr(buf).map(|c| c.end)
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
            // A string reply (DCS, OSC, APC) is not typing; skip it whole.
            0x1b if matches!(bytes.get(i + 1), Some(b'P' | b']' | b'_' | b'^' | b'X')) => {
                match scan(&bytes[i..]) {
                    Scan::Done(len, _) => i += len - 1,
                    Scan::Partial => i = bytes.len(),
                    Scan::Not => keys.push(Key::Esc),
                }
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

    fn csi(body: &str) -> Reply {
        Reply { kind: ReplyKind::Csi, body: body.to_string() }
    }

    #[test]
    fn reads_every_kind_of_reply() {
        let bytes = b"\x1b[?2026;2$y\x1bP>|WezTerm 1\x1b\\\x1b]11;rgb:00/00/00\x07\x1b]10;rgb:ff/ff/ff\x1b\\\x1b_Gi=31;OK\x1b\\";
        let (replies, used) = parse_replies(bytes);
        assert_eq!(used, bytes.len());
        assert_eq!(
            replies,
            vec![
                csi("?2026;2$y"),
                Reply { kind: ReplyKind::Dcs, body: ">|WezTerm 1".to_string() },
                Reply { kind: ReplyKind::Osc, body: "11;rgb:00/00/00".to_string() },
                Reply { kind: ReplyKind::Osc, body: "10;rgb:ff/ff/ff".to_string() },
                Reply { kind: ReplyKind::Apc, body: "Gi=31;OK".to_string() },
            ]
        );
    }

    #[test]
    fn a_reply_split_across_reads_waits_for_the_rest() {
        let whole = b"ab\x1bP>|Term\x1b\\\x1b[?64;4c";
        for cut in 1..whole.len() {
            let (first, used) = parse_replies(&whole[..cut]);
            let mut rest = whole[used..cut].to_vec();
            rest.extend_from_slice(&whole[cut..]);
            let (second, used2) = parse_replies(&rest);
            assert_eq!(used2, rest.len(), "cut at {cut}");
            let all: Vec<Reply> = first.into_iter().chain(second).collect();
            assert_eq!(all.len(), 2, "cut at {cut}: {all:?}");
            assert_eq!(all[1], csi("?64;4c"));
        }
    }

    #[test]
    fn keys_between_replies_are_dropped_and_broken_escapes_skipped() {
        let (replies, used) = parse_replies(b"q\x1b[A \x1b\x1b[?1u\x1b[\x01x\x1b]oops\x1b[0n");
        assert_eq!(replies, vec![csi("A"), csi("?1u"), csi("0n")]);
        assert_eq!(used, 25);
        // A lone escape at the end may be the start of a reply, so it stays unused.
        assert_eq!(parse_replies(b"x\x1b"), (vec![], 1));
    }

    #[test]
    fn da1_is_told_from_other_replies() {
        assert!(is_da1(&csi("?64;1;4c")));
        assert!(is_da1(&csi("?1;2c")));
        assert!(!is_da1(&csi(">1;4000;19c")));
        assert!(!is_da1(&csi("?2026;2$y")));
        assert!(!is_da1(&Reply { kind: ReplyKind::Dcs, body: "?64c".to_string() }));
    }

    #[test]
    fn escaping_is_readable_and_unambiguous() {
        assert_eq!(escape(b"\x1b[?2026;2$y"), "\\x1b[?2026;2$y");
        assert_eq!(escape(b"\x1b\\"), "\\x1b\\\\");
        assert_eq!(escape(b"a\x07\r\n\x7f"), "a\\x07\\x0d\\x0a\\x7f");
        assert_eq!(escape("\u{1f600}é".as_bytes()), "\\u{1f600}\\u{e9}");
        assert_eq!(escape(&[b'a', 0xff, b'b']), "a\\xffb");
    }

    #[test]
    fn strings_are_not_typing() {
        assert_eq!(parse_keys(b"q\x1bP>|Term q\x1b\\x"), vec![Key::Char('q'), Key::Char('x')]);
        assert_eq!(parse_keys(b"\x1b]11;rgb:00/00/00\x07q"), vec![Key::Char('q')]);
        assert_eq!(parse_keys(b"\x1b[?64;4cq\x1b"), vec![Key::Char('q'), Key::Esc]);
    }

    /// A stand-in for the terminal that answers when the request is flushed, not after a fixed sleep,
    /// so the tests don't depend on how busy the machine is. `unasked` is waiting before any request.
    struct Fake {
        tx: mpsc::Sender<Vec<u8>>,
        replies: Option<Vec<(u64, &'static [u8])>>,
        sent: Vec<u8>,
    }

    impl Write for Fake {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.sent.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            if let Some(replies) = self.replies.take() {
                let tx = self.tx.clone();
                thread::spawn(move || {
                    for (ms, c) in replies {
                        thread::sleep(Duration::from_millis(ms));
                        if tx.send(c.to_vec()).is_err() {
                            break;
                        }
                    }
                });
            }
            Ok(())
        }
    }

    /// The channel stays open while the `Fake` lives, so a missing reply times out.
    fn fake(unasked: &[&'static [u8]], replies: Vec<(u64, &'static [u8])>) -> (Input, Fake) {
        let (tx, rx) = mpsc::channel();
        for c in unasked {
            let _ = tx.send(c.to_vec());
        }
        (Input::with_receiver(rx), Fake { tx, replies: Some(replies), sent: Vec::new() })
    }

    #[test]
    fn ask_collects_replies_up_to_the_da1_reply() {
        let (mut input, mut term) =
            fake(&[], vec![(0, b"\x1b[?2026;2$y\x1bP>|Te"), (20, b"rm\x1b\\\x1b[?64;4c\x1b[?2027;1$y")]);
        let a = input.ask(&mut term, b"\x1b[?2026$p", Duration::from_secs(2)).expect("ask");
        assert_eq!(term.sent, b"\x1b[?2026$p\x1b[c");
        assert_eq!(a.replies.len(), 2);
        assert_eq!(a.replies[0].0, csi("?2026;2$y"));
        assert_eq!(a.replies[1].0.body, ">|Term");
        assert_eq!(a.da1.as_ref().map(|d| d.0.as_str()), Some("?64;4c"));
        // The reply that came in the same read after DA1 is out of order, not lost.
        assert_eq!(a.late, vec![csi("?2027;1$y")]);
        assert!(a.da1.as_ref().is_some_and(|d| d.1 >= 20_000), "{:?}", a.da1);
    }

    #[test]
    fn ask_times_out_without_da1_and_keeps_what_came() {
        let (mut input, mut term) = fake(&[], vec![(0, b"junk")]);
        let a = input.ask(&mut term, b"", Duration::from_millis(100)).expect("ask");
        assert_eq!(a.da1, None);
        assert_eq!(a.raw, b"junk");
        assert!(a.replies.is_empty() && a.waited >= 100_000);
    }

    #[test]
    fn ask_reports_what_was_waiting_before_it() {
        let (mut input, mut term) = fake(&[b"\x1b[0n"], vec![(0, b"\x1b[?1;2c")]);
        let a = input.ask(&mut term, b"", Duration::from_secs(2)).expect("ask");
        assert_eq!(a.stale, b"\x1b[0n");
        assert!(a.da1.is_some());
    }

    #[test]
    fn cursor_reports_come_in_order_and_keep_their_bytes() {
        let (mut input, mut term) = fake(&[], vec![(0, b"x\x1b[2;1"), (10, b"R\x1b[2;3R")]);
        let got = input.cursor_reports(&mut term, b"", 2, Duration::from_secs(2)).expect("reports");
        let cells: Vec<(u16, u16)> = got.reports.iter().map(|c| (c.row, c.col)).collect();
        assert_eq!(cells, vec![(2, 1), (2, 3)]);
        assert_eq!(got.reports[1].raw, b"\x1b[2;3R");
        let (mut short, mut term) = fake(&[], vec![(0, b"\x1b[2;1R")]);
        let got = short.cursor_reports(&mut term, b"", 2, Duration::from_millis(100)).expect("reports");
        assert_eq!(got.reports.len(), 1);
    }

    #[test]
    fn the_raw_log_keeps_every_chunk() {
        let (mut input, mut term) = fake(&[], vec![(0, b"\x1b[?1;2c")]);
        input.record_raw();
        let _ = input.ask(&mut term, b"", Duration::from_secs(2));
        assert_eq!(input.raw_log().len(), 1);
        assert!(input.raw_log()[0].ends_with("\\x1b[?1;2c"), "{:?}", input.raw_log());
    }
}
