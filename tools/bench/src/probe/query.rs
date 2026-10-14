//! Asking the terminal one thing at a time, and deciding from what came back whether it knows it.
//!
//! Every query is followed by DA1 (`CSI c`). Terminals answer in order and practically all answer
//! DA1, so when the DA1 reply arrives first the answer is "not supported", known in microseconds.
//! Only a terminal that says nothing at all costs a timeout.

use std::io::{self, Write};
use std::time::Duration;

use super::model::{Decision, Trace};
use crate::term::{Answer, Input, Reply, ReplyKind, Reports, escape};

/// How long the first query waits: a terminal that answers nothing costs this once.
const FIRST_TIMEOUT: Duration = Duration::from_millis(1000);
/// Later queries have been shown to be answered, so a miss is quicker to call.
const LATER_TIMEOUT: Duration = Duration::from_millis(500);

/// What a query found: the parsed value (if the terminal had one) and the exchange behind it.
#[derive(Debug, PartialEq, Eq)]
pub struct Reading<T> {
    pub value: Option<T>,
    pub trace: Trace,
}

/// A reply as a person writes it: `CSI ?2026;2$y`.
pub fn describe(r: &Reply) -> String {
    let kind = match r.kind {
        ReplyKind::Csi => "CSI",
        ReplyKind::Dcs => "DCS",
        ReplyKind::Osc => "OSC",
        ReplyKind::Apc => "APC",
    };
    format!("{kind} {}", escape(r.body.as_bytes()))
}

/// Decides one exchange. `parse` picks the reply the query was after out of whatever arrived.
///
/// - the reply is there: `Reply`, with a note for every other reply we didn't ask for
/// - DA1 came back and nothing else: `Sentinel`, the terminal doesn't know the query
/// - DA1 came back with replies that don't fit: `Unparsed`
/// - no DA1 in time: `Timeout`, or `Unparsed` when bytes did arrive
pub fn judge<T>(request: &[u8], ans: &Answer, parse: impl Fn(&Reply) -> Option<T>) -> Reading<T> {
    let mut sent = request.to_vec();
    sent.extend_from_slice(b"\x1b[c");
    let mut notes = Vec::new();
    if !ans.stale.is_empty() {
        notes.push(format!(
            "{} byte(s) were already waiting when this was sent (a late answer to an earlier query?): {}",
            ans.stale.len(),
            escape(&ans.stale)
        ));
    }
    let hit = ans.replies.iter().enumerate().find_map(|(i, (r, at))| parse(r).map(|v| (i, v, *at)));
    let (value, decided, micros) = match hit {
        Some((i, v, at)) => {
            for (j, (r, _)) in ans.replies.iter().enumerate() {
                if j != i {
                    notes.push(format!("unexpected reply, nothing we asked for: {}", describe(r)));
                }
            }
            (Some(v), Decision::Reply, at)
        }
        None => {
            for (r, _) in &ans.replies {
                notes.push(format!("a reply that does not fit the query: {}", describe(r)));
            }
            match (&ans.da1, ans.replies.first()) {
                (Some((_, at)), None) => (None, Decision::Sentinel, *at),
                (Some(_), Some((_, at))) => (None, Decision::Unparsed, *at),
                (None, _) if !ans.raw.is_empty() && ans.replies.is_empty() => {
                    notes.push("bytes arrived but none of them is a reply we can read".to_string());
                    (None, Decision::Unparsed, ans.waited)
                }
                (None, Some(_)) => (None, Decision::Unparsed, ans.waited),
                (None, None) => (None, Decision::Timeout, ans.waited),
            }
        }
    };
    if ans.da1.is_none() {
        notes.push(
            "DA1 was never answered, so \"not supported\" can't be told from \"no answer\"".to_string(),
        );
    }
    for r in &ans.late {
        notes.push(format!("arrived after the DA1 reply, out of order: {}", describe(r)));
    }
    Reading {
        value,
        trace: Trace {
            sent: escape(&sent),
            received: escape(&ans.raw),
            micros: Some(micros),
            decided,
            notes,
        },
    }
}

pub struct Prober<'a, W: Write> {
    input: &'a mut Input,
    out: W,
    asked: u32,
    misses: u8,
    /// The very first query got no DA1 at all: the terminal says nothing, so nothing is asked again.
    pub silent: bool,
    /// Two queries in a row got no DA1: the stream is probably stuck, so the rest isn't asked.
    gave_up: bool,
    cursor_asked: u32,
    cursor_dead: bool,
    /// Unexpected things that belong to no row.
    pub events: Vec<String>,
}

impl<'a, W: Write> Prober<'a, W> {
    pub fn new(input: &'a mut Input, out: W) -> Self {
        Prober {
            input,
            out,
            asked: 0,
            misses: 0,
            silent: false,
            gave_up: false,
            cursor_asked: 0,
            cursor_dead: false,
            events: Vec::new(),
        }
    }

    /// Writes bytes without waiting for anything (restoring what a query changed).
    pub fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.out.write_all(bytes)?;
        self.out.flush()
    }

    fn skipped<T>(request: &[u8]) -> Reading<T> {
        let mut sent = request.to_vec();
        sent.extend_from_slice(b"\x1b[c");
        Reading {
            value: None,
            trace: Trace {
                sent: escape(&sent),
                received: String::new(),
                micros: None,
                decided: Decision::Skipped,
                notes: Vec::new(),
            },
        }
    }

    /// Sends `request` and DA1 and judges the answer.
    pub fn ask<T>(&mut self, request: &[u8], parse: impl Fn(&Reply) -> Option<T>) -> io::Result<Reading<T>> {
        if self.silent || self.gave_up {
            return Ok(Self::skipped(request));
        }
        let first = self.asked == 0;
        let timeout = if first { FIRST_TIMEOUT } else { LATER_TIMEOUT };
        let ans = self.input.ask(&mut self.out, request, timeout)?;
        self.asked += 1;
        if ans.da1.is_some() {
            self.misses = 0;
        } else {
            self.misses += 1;
            if first {
                self.silent = true;
                self.events.push(format!(
                    "no DA1 reply to the first query within {} ms: the terminal answers nothing, so nothing else was asked",
                    timeout.as_millis()
                ));
            } else if self.misses >= 2 {
                self.gave_up = true;
                self.events.push(
                    "two queries in a row got no DA1 reply: the rest of the queries were skipped".to_string(),
                );
            }
        }
        Ok(judge(request, &ans, parse))
    }

    /// DA1 on its own, the first question: the body of the reply (`?64;1;4c`). Without it the
    /// terminal is taken to be silent.
    pub fn da1(&mut self) -> io::Result<Reading<String>> {
        let ans = self.input.ask(&mut self.out, b"", FIRST_TIMEOUT)?;
        self.asked = 1;
        if ans.da1.is_none() {
            self.silent = true;
            self.events.push(format!(
                "no DA1 reply within {} ms: the terminal answers nothing, so nothing else was asked",
                FIRST_TIMEOUT.as_millis()
            ));
        }
        let mut reading = judge(b"", &ans, |_| None::<()>);
        // DA1 is the answer here, where `judge` sees it as the sentinel.
        if let Some((_, at)) = &ans.da1 {
            reading.trace.decided = Decision::Reply;
            reading.trace.micros = Some(*at);
        }
        let value = ans.da1.map(|(body, _)| body);
        Ok(Reading { value, trace: reading.trace })
    }

    /// Waits until the terminal has answered everything sent, so a late reply doesn't land in the
    /// shell that gets the terminal back.
    pub fn finish(&mut self) {
        if !self.silent {
            let _ = self.input.sync(&mut self.out, Duration::from_millis(300));
        }
    }

    /// Sends `request`, which holds `count` cursor position requests, and collects the reports.
    /// `None` when the terminal can't be asked (it was silent, or stopped answering cursor reports).
    pub fn cursor(&mut self, request: &[u8], count: usize) -> io::Result<Option<Reports>> {
        if self.silent || self.cursor_dead {
            return Ok(None);
        }
        let timeout = if self.cursor_asked == 0 { FIRST_TIMEOUT } else { LATER_TIMEOUT };
        self.cursor_asked += 1;
        let got = self.input.cursor_reports(&mut self.out, request, count, timeout)?;
        if got.reports.len() < count {
            self.cursor_dead = true;
            self.events.push(format!(
                "cursor position report (DSR 6) not answered within {} ms: widths can't be measured",
                timeout.as_millis()
            ));
        }
        Ok(Some(got))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn csi(body: &str) -> Reply {
        Reply { kind: ReplyKind::Csi, body: body.to_string() }
    }

    fn answer(replies: &[(&str, u64)], da1: Option<u64>) -> Answer {
        let mut raw = Vec::new();
        for (b, _) in replies {
            raw.extend_from_slice(format!("\x1b[{b}").as_bytes());
        }
        if da1.is_some() {
            raw.extend_from_slice(b"\x1b[?64;4c");
        }
        Answer {
            replies: replies.iter().map(|(b, at)| (csi(b), *at)).collect(),
            da1: da1.map(|at| ("?64;4c".to_string(), at)),
            late: vec![],
            raw,
            stale: vec![],
            waited: da1.unwrap_or(1_000_000),
        }
    }

    fn mode(r: &Reply) -> Option<u8> {
        r.body.strip_prefix("?2026;")?.strip_suffix("$y")?.parse().ok()
    }

    #[test]
    fn a_reply_before_da1_is_the_answer() {
        let r = judge(b"\x1b[?2026$p", &answer(&[("?2026;2$y", 150)], Some(210)), mode);
        assert_eq!(r.value, Some(2));
        assert_eq!(r.trace.decided, Decision::Reply);
        assert_eq!(r.trace.micros, Some(150));
        assert_eq!(r.trace.sent, "\\x1b[?2026$p\\x1b[c");
        assert_eq!(r.trace.received, "\\x1b[?2026;2$y\\x1b[?64;4c");
        assert!(r.trace.notes.is_empty());
    }

    #[test]
    fn da1_alone_means_not_supported() {
        let r = judge(b"\x1b[?2026$p", &answer(&[], Some(90)), mode);
        assert_eq!(r.value, None);
        assert_eq!(r.trace.decided, Decision::Sentinel);
        assert_eq!(r.trace.micros, Some(90));
        assert!(r.trace.notes.is_empty());
    }

    #[test]
    fn nothing_at_all_is_a_timeout() {
        let r = judge(b"\x1b[?2026$p", &answer(&[], None), mode);
        assert_eq!(r.trace.decided, Decision::Timeout);
        assert_eq!(r.trace.micros, Some(1_000_000));
        assert!(r.trace.notes.iter().any(|n| n.contains("DA1 was never answered")), "{:?}", r.trace.notes);
    }

    #[test]
    fn a_reply_that_does_not_fit_is_unparsed_and_reported() {
        let r = judge(b"\x1b[?2026$p", &answer(&[("?9999;1$y", 100)], Some(150)), mode);
        assert_eq!(r.trace.decided, Decision::Unparsed);
        assert!(r.trace.notes[0].contains("CSI ?9999;1$y"), "{:?}", r.trace.notes);
    }

    #[test]
    fn replies_nobody_asked_for_and_late_ones_are_noted() {
        let mut a = answer(&[("?1004;2$y", 50), ("?2026;1$y", 80)], Some(120));
        a.late.push(csi("?2027;1$y"));
        a.stale = b"\x1b[0n".to_vec();
        let r = judge(b"\x1b[?2026$p", &a, mode);
        assert_eq!(r.value, Some(1));
        assert_eq!(r.trace.decided, Decision::Reply);
        let notes = r.trace.notes.join("\n");
        assert!(notes.contains("unexpected reply") && notes.contains("?1004;2$y"), "{notes}");
        assert!(notes.contains("out of order") && notes.contains("?2027;1$y"), "{notes}");
        assert!(notes.contains("already waiting") && notes.contains("\\x1b[0n"), "{notes}");
    }

    #[test]
    fn bytes_that_are_no_reply_are_unparsed_not_a_timeout() {
        let mut a = answer(&[], None);
        a.raw = b"hello".to_vec();
        let r = judge(b"\x1b[?2026$p", &a, mode);
        assert_eq!(r.trace.decided, Decision::Unparsed);
        assert!(r.trace.notes.iter().any(|n| n.contains("none of them is a reply")));
        assert_eq!(r.trace.received, "hello");
    }
}
