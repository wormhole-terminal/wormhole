//! What the terminal sends us: keys, read on a thread of its own, and the questions we ask it.

use std::io::{self, Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use super::replies::{
    Answer, Cursor, Reports, Scan, count_status_reports, escape, is_da1, parse_cpr, parse_replies, scan,
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
    /// How many of `status_replies` were read before the cursor report that `sync` found last (the
    /// count since the last `take_status_replies`); `None` when `sync` has not found one since, or
    /// the replies are not counted.
    before_report: Option<u64>,
    /// When the last bytes were read, in nanoseconds after `started`: stamped by the reader thread when
    /// they come (so a key is aged from its arrival, not from the call that finds it), and by `stash` for
    /// an input without a reader thread of its own.
    last_read: Arc<AtomicU64>,
    reader_stamps: bool,
}

/// How long `keys` waits, after the last byte read, for the rest of an escape sequence that stops
/// short, before it takes what is there for what it is. A reply arrives within microseconds of its
/// first byte; a key that begins like one (Esc, Alt+]) should not hold the keys behind it for long.
const HOLD: Duration = Duration::from_millis(100);

impl Input {
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel();
        let started = Instant::now();
        let last_read = Arc::new(AtomicU64::new(0));
        let stamp = Arc::clone(&last_read);
        thread::spawn(move || {
            let mut stdin = io::stdin().lock();
            let mut buf = [0u8; 4096];
            loop {
                match stdin.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        stamp.store(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        Self::build(rx, started, last_read, true)
    }

    /// An input fed from a channel instead of stdin: what the tests use.
    #[cfg(test)]
    pub(crate) fn with_receiver(rx: Receiver<Vec<u8>>) -> Self {
        Self::build(rx, Instant::now(), Arc::new(AtomicU64::new(0)), false)
    }

    fn build(
        rx: Receiver<Vec<u8>>,
        started: Instant,
        last_read: Arc<AtomicU64>,
        reader_stamps: bool,
    ) -> Self {
        Self {
            rx,
            pending: Vec::new(),
            started,
            tap: None,
            counting: false,
            status_replies: 0,
            before_report: None,
            last_read,
            reader_stamps,
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

    /// A chunk the reader thread delivered: queued for whoever parses it, and logged when asked. The
    /// status reports in it are not taken out yet (`count_pending`): where they sit, relative to a
    /// cursor report, is still to be told.
    fn stash(&mut self, chunk: Vec<u8>) {
        if let Some(tap) = &mut self.tap {
            tap.push(format!("+{:.3} ms  {}", self.started.elapsed().as_secs_f64() * 1e3, escape(&chunk)));
        }
        if !self.reader_stamps {
            self.last_read.store(self.started.elapsed().as_nanos() as u64, Ordering::Relaxed);
        }
        self.pending.extend_from_slice(&chunk);
    }

    /// How long ago bytes were last read.
    fn quiet_for(&self) -> Duration {
        self.started.elapsed().saturating_sub(Duration::from_nanos(self.last_read.load(Ordering::Relaxed)))
    }

    /// Takes the status reports out of what is pending and counts them (when counting).
    fn count_pending(&mut self) {
        if self.counting {
            self.status_replies += strip_status_replies(&mut self.pending);
        }
    }

    fn take(&mut self, chunk: Vec<u8>) {
        self.stash(chunk);
        self.count_pending();
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
        self.before_report = None;
        std::mem::take(&mut self.status_replies)
    }

    /// How many status reports were read before the cursor report that `sync` found last, counting
    /// from the last `take_status_replies`; a report in the same read as the cursor report counts
    /// only if it sits in front of it. `None` when `sync` has found none since then. Answers that
    /// come after it were not in time for the measurement they belong to.
    pub fn status_replies_before_report(&self) -> Option<u64> {
        self.before_report
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

    /// The keys pressed since the last call, never blocking. The start of an escape sequence whose
    /// end hasn't arrived (also a lone `ESC`, which may be the first byte of a reply read in two parts)
    /// stays for the next call, for no longer than `HOLD`: half a reply is not read as typing, and a
    /// real Esc press still counts a moment later.
    pub fn keys(&mut self) -> Vec<Key> {
        while let Ok(chunk) = self.rx.try_recv() {
            self.take(chunk);
        }
        // Held while the input is still coming; aged from the last read, so nothing is left over to
        // reset when `sync` or `ask` use `pending` up.
        let whole = match unfinished_tail(&self.pending) {
            Some(at) if self.quiet_for() < HOLD => at,
            _ => self.pending.len(),
        };
        let rest = self.pending.split_off(whole);
        let bytes = std::mem::replace(&mut self.pending, rest);
        parse_keys(&bytes)
    }

    /// Asks for the cursor position (DSR 6) and waits for the answer. The terminal answers only
    /// after it has parsed everything written before the question, so this is the moment the
    /// output has really been consumed. `None` on a timeout. When the status reports are counted,
    /// how many were read before the cursor report is kept (`status_replies_before_report`).
    pub fn sync(&mut self, out: &mut impl Write, timeout: Duration) -> io::Result<Option<Duration>> {
        // A cursor report that is already here answers an earlier question (one that timed out): it must
        // not stand in for the answer to this one.
        while let Ok(chunk) = self.rx.try_recv() {
            self.take(chunk);
        }
        while let Some(old) = parse_cpr(&self.pending) {
            self.pending.drain(old.start..old.end);
        }
        let start = Instant::now();
        out.write_all(b"\x1b[6n")?;
        out.flush()?;
        let deadline = start + timeout;
        self.before_report = None;
        loop {
            if let Some(report) = parse_cpr(&self.pending) {
                // What came with the report was not counted yet: only the part in front of it is
                // before it, and the rest comes after.
                if self.counting {
                    self.status_replies += count_status_reports(&self.pending[..report.start]);
                    self.before_report = Some(self.status_replies);
                }
                self.pending.drain(..report.end);
                self.count_pending();
                return Ok(Some(start.elapsed()));
            }
            // No cursor report yet: every report so far is in front of the one to come.
            self.count_pending();
            let left = deadline.saturating_duration_since(Instant::now());
            match self.rx.recv_timeout(left) {
                Ok(chunk) => self.stash(chunk),
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

/// Where the escape sequence that may go on past the end of `bytes` starts: a lone `ESC`, or a
/// sequence whose end hasn't arrived. `None` when there is none, or when a Ctrl-C follows it: that
/// is a key whatever the sequence was.
fn unfinished_tail(bytes: &[u8]) -> Option<usize> {
    let at = bytes.iter().rposition(|&b| b == 0x1b)?;
    let open = bytes.len() - at == 1 || matches!(scan(&bytes[at..]), Scan::Partial);
    (open && !bytes[at..].contains(&0x03)).then_some(at)
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
                // Skip the whole sequence, whatever it was; a control character ends it, and is read.
                let mut j = i + 2;
                while j < bytes.len() && !(0x40..=0x7e).contains(&bytes[j]) && bytes[j] >= 0x20 {
                    j += 1;
                }
                i = if j < bytes.len() && bytes[j] < 0x20 { j - 1 } else { j };
            }
            // A string reply (DCS, OSC, APC) is not typing; skip it whole. One that never ended (Alt+]
            // typed, not a reply) gives up its introducer and leaves what follows to be read.
            0x1b if matches!(bytes.get(i + 1), Some(b'P' | b']' | b'_' | b'^' | b'X')) => {
                match scan(&bytes[i..]) {
                    Scan::Done(len, _) => i += len - 1,
                    Scan::Partial => {}
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
        let (mut input, mut term) = fake(&[], vec![(0, b"\x1b[0n\x1b["), (5, b"0n\x1b[4;1R")]);
        input.count_status_replies();
        assert!(input.sync(&mut term, Duration::from_secs(2)).expect("sync").is_some());
        assert_eq!(input.take_status_replies(), 2);
        assert_eq!(input.take_status_replies(), 0);
    }

    #[test]
    fn only_the_reports_in_front_of_the_cursor_report_are_before_it() {
        // Two in an earlier read, one in front of the cursor report and one behind it in its read, and
        // one that comes in a read of its own after it.
        let (mut input, mut term) =
            fake(&[], vec![(0, b"\x1b[0n\x1b[0n"), (5, b"\x1b[0n\x1b[4;1R\x1b[0n"), (5, b"\x1b[0n")]);
        input.count_status_replies();
        assert!(input.sync(&mut term, Duration::from_secs(2)).expect("sync").is_some());
        assert_eq!(input.status_replies_before_report(), Some(3));
        assert_eq!(input.wait_status_replies(5, Duration::from_secs(2)), 5);
        // The next count starts again, and so does what is known about the report.
        assert_eq!(input.take_status_replies(), 5);
        assert_eq!(input.status_replies_before_report(), None);
    }

    #[test]
    fn a_report_split_by_the_read_boundary_is_before_the_cursor_report_that_follows() {
        let (mut input, mut term) = fake(&[], vec![(0, b"\x1b["), (5, b"0n\x1b[4;1R")]);
        input.count_status_replies();
        assert!(input.sync(&mut term, Duration::from_secs(2)).expect("sync").is_some());
        assert_eq!(input.status_replies_before_report(), Some(1));
    }

    #[test]
    fn nothing_is_known_about_the_order_when_the_reports_are_not_counted() {
        let (mut input, mut term) = fake(&[], vec![(0, b"\x1b[4;1R")]);
        assert!(input.sync(&mut term, Duration::from_secs(2)).expect("sync").is_some());
        assert_eq!(input.status_replies_before_report(), None);
    }

    #[test]
    fn half_a_reply_is_not_typing_and_waits_for_the_rest() {
        let (tx, rx) = mpsc::channel();
        let mut input = Input::with_receiver(rx);
        let _ = tx.send(b"q\x1b[?6".to_vec());
        assert_eq!(input.keys(), vec![Key::Char('q')]);
        let _ = tx.send(b"4;4c".to_vec());
        assert_eq!(input.keys(), vec![]);
        // The start of a string reply is not lost when it comes in pieces.
        let _ = tx.send(b"\x1bP>|Te".to_vec());
        assert_eq!(input.keys(), vec![]);
        let _ = tx.send(b"rm\x1b\\x".to_vec());
        assert_eq!(input.keys(), vec![Key::Char('x')]);
    }

    #[test]
    fn a_lone_escape_waits_a_moment_for_the_rest_of_a_reply_and_is_then_the_esc_key() {
        let (tx, rx) = mpsc::channel();
        let mut input = Input::with_receiver(rx);
        input.count_status_replies();
        // A status report read in two parts must not stop the benchmark.
        let _ = tx.send(b"\x1b".to_vec());
        assert_eq!(input.keys(), vec![]);
        let _ = tx.send(b"[0n".to_vec());
        assert_eq!(input.keys(), vec![]);
        assert_eq!(input.take_status_replies(), 1);
        // A real Esc press goes through once nothing more comes.
        let _ = tx.send(b"\x1b".to_vec());
        assert_eq!(input.keys(), vec![]);
        thread::sleep(HOLD + Duration::from_millis(30));
        assert_eq!(input.keys(), vec![Key::Esc]);
        assert_eq!(input.keys(), vec![]);
    }

    /// What the reviewer found: the hold was timed from the first look at the byte, so with one look per
    /// run an Esc press took effect a run late.
    #[test]
    fn an_esc_is_aged_from_its_arrival_and_not_from_the_first_look() {
        let (tx, rx) = mpsc::channel();
        let started = Instant::now();
        let stamp = Arc::new(AtomicU64::new(0));
        let mut input = Input::build(rx, started, Arc::clone(&stamp), true);
        // The reader thread stamps the read, then hands the bytes over.
        let arrive = |bytes: &[u8]| {
            stamp.store(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
            let _ = tx.send(bytes.to_vec());
        };
        arrive(b"\x1b");
        thread::sleep(HOLD + Duration::from_millis(30));
        // The first look is long after it came: no more waiting.
        assert_eq!(input.keys(), vec![Key::Esc]);
        // One that has just come is still given its moment.
        arrive(b"\x1b");
        assert_eq!(input.keys(), vec![]);
    }

    #[test]
    fn keys_behind_an_unfinished_sequence_that_never_ends_still_get_through() {
        let (tx, rx) = mpsc::channel();
        let mut input = Input::with_receiver(rx);
        // Alt+] and Alt+Shift+P begin strings; neither is followed by a terminator.
        for lead in [&b"\x1b]"[..], b"\x1bP", b"\x1b_"] {
            let _ = tx.send(lead.to_vec());
            assert_eq!(input.keys(), vec![], "held for a moment");
            // Ctrl-C is a key at once.
            let _ = tx.send(b"\x03".to_vec());
            assert_eq!(input.keys(), vec![Key::Char(lead[1] as char), Key::CtrlC]);
            // And so are the others, after the moment.
            let _ = tx.send(b"q".to_vec());
            thread::sleep(HOLD + Duration::from_millis(30));
            assert_eq!(input.keys(), vec![Key::Char('q')]);
        }
        // Ctrl-C inside an unfinished CSI is read too.
        let _ = tx.send(b"\x1b[1;\x03".to_vec());
        assert_eq!(input.keys(), vec![Key::CtrlC]);
    }

    #[test]
    fn a_cursor_report_that_was_already_here_does_not_answer_the_next_question() {
        // The late answer to a question that timed out waits in the channel; the real one comes 60 ms
        // after this question is asked.
        let (mut input, mut term) = fake(&[b"\x1b[9;9R"], vec![(60, b"\x1b[3;4R")]);
        let took = input.sync(&mut term, Duration::from_secs(2)).expect("sync").expect("answered");
        assert!(took >= Duration::from_millis(50), "{took:?}");
        assert!(input.pending.is_empty(), "{:?}", input.pending);
    }

    #[test]
    fn status_replies_already_queued_are_before_the_report_and_a_stale_report_does_not_answer() {
        // Two status replies and the late report of a question that timed out are waiting when the next
        // question is asked; its own report comes 60 ms later.
        let (mut input, mut term) = fake(&[b"\x1b[0n\x1b[0n\x1b[9;9R"], vec![(60, b"\x1b[3;4R\x1b[0n")]);
        input.count_status_replies();
        let took = input.sync(&mut term, Duration::from_secs(2)).expect("sync").expect("answered");
        assert!(took >= Duration::from_millis(50), "{took:?}");
        assert_eq!(input.status_replies_before_report(), Some(2));
        // The one behind the report in its read is counted, but not as before it.
        assert_eq!(input.take_status_replies(), 3);
        assert!(input.pending.is_empty(), "{:?}", input.pending);
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
