//! What the terminal sends us: keys, read on a thread of its own, and the questions we ask it.

use std::io::{self, Read, Write};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use super::replies::{
    Answer, Cursor, Reports, Scan, escape, find_cpr, is_da1, parse_cpr, parse_replies, scan,
    strip_status_replies,
};

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
    /// Set by `count_status_replies`: `ESC [ 0 n` is taken out of what is read and counted.
    counting: bool,
    status_replies: u64,
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
    pub(crate) fn with_receiver(rx: Receiver<Vec<u8>>) -> Self {
        Self {
            rx,
            pending: Vec::new(),
            started: Instant::now(),
            tap: None,
            counting: false,
            status_replies: 0,
        }
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
        if self.counting {
            self.status_replies += strip_status_replies(&mut self.pending);
        }
    }

    /// From now on every device status report `ESC [ 0 n` (the answer to DSR 5) is removed from what
    /// is read and counted instead, wherever it comes: before a cursor report, split across two reads,
    /// or long after the question. Nothing else in the stream is touched.
    pub fn count_status_replies(&mut self) {
        self.counting = true;
    }

    /// The status reports counted since the last call, and the count starts again.
    pub fn take_status_replies(&mut self) -> u64 {
        while let Ok(chunk) = self.rx.try_recv() {
            self.take(chunk);
        }
        std::mem::take(&mut self.status_replies)
    }

    /// Waits until `want` status reports have been counted since the last `take_status_replies`, or
    /// `timeout` has passed, and returns how many there are. It does not reset the count.
    pub fn wait_status_replies(&mut self, want: u64, timeout: Duration) -> u64 {
        let deadline = Instant::now() + timeout;
        loop {
            while let Ok(chunk) = self.rx.try_recv() {
                self.take(chunk);
            }
            if self.status_replies >= want {
                break;
            }
            let left = deadline.saturating_duration_since(Instant::now());
            match self.rx.recv_timeout(left) {
                Ok(chunk) => self.take(chunk),
                Err(_) => break,
            }
        }
        self.status_replies
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

#[cfg(test)]
mod tests {
    use super::super::replies::{Reply, ReplyKind};
    use super::*;

    fn csi(body: &str) -> Reply {
        Reply { kind: ReplyKind::Csi, body: body.to_string() }
    }

    #[test]
    fn counting_leaves_the_cursor_report_for_sync_to_find() {
        let (tx, rx) = mpsc::channel();
        let mut input = Input::with_receiver(rx);
        input.count_status_replies();
        let _ = tx.send(b"\x1b[0n\x1b[".to_vec());
        let _ = tx.send(b"0n\x1b[4;1R".to_vec());
        let mut sink = Vec::new();
        assert!(input.sync(&mut sink, Duration::from_secs(2)).expect("sync").is_some());
        assert_eq!(input.take_status_replies(), 2);
        assert_eq!(input.take_status_replies(), 0);
    }

    #[test]
    fn waiting_for_status_reports_gives_up_with_the_count_so_far() {
        let (tx, rx) = mpsc::channel();
        let mut input = Input::with_receiver(rx);
        input.count_status_replies();
        let _ = tx.send(b"\x1b[0n".to_vec());
        assert_eq!(input.wait_status_replies(2, Duration::from_millis(50)), 1);
        let _ = tx.send(b"\x1b[0n".to_vec());
        assert_eq!(input.wait_status_replies(2, Duration::from_millis(50)), 2);
    }

    #[test]
    fn reads_arrows_and_a_lone_escape() {
        assert_eq!(parse_keys(b"\x1b[Cq\x1b"), vec![Key::Right, Key::Char('q'), Key::Esc]);
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
