//! The replies a terminal gives to our questions: reading them out of the bytes it sends (CSI, DCS,
//! OSC and APC sequences), the cursor position report, and the status report that is counted.

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

pub(super) enum Scan {
    /// A whole sequence of this many bytes, and the reply it is (strings we don't use give none).
    Done(usize, Option<Reply>),
    /// The start of one; the rest hasn't arrived.
    Partial,
    /// Not a sequence at all.
    Not,
}

/// The sequence starting at the `ESC` that opens `b`.
pub(super) fn scan(b: &[u8]) -> Scan {
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

/// Removes every `ESC [ 0 n` from `buf` and returns how many there were. A report cut in two stays
/// in the buffer until the rest arrives (the caller appends to the same buffer).
pub(super) fn strip_status_replies(buf: &mut Vec<u8>) -> u64 {
    const REPORT: &[u8] = b"\x1b[0n";
    let mut found = 0;
    let mut kept = 0;
    let mut i = 0;
    while i < buf.len() {
        if buf[i..].starts_with(REPORT) {
            found += 1;
            i += REPORT.len();
        } else {
            buf[kept] = buf[i];
            kept += 1;
            i += 1;
        }
    }
    buf.truncate(kept);
    found
}

/// A cursor position report found in a buffer: where it sits and what it says.
pub(super) struct Cpr {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) row: u16,
    pub(super) col: u16,
}

/// The first cursor position report (`ESC [ row ; col R`) in `buf`.
pub(super) fn parse_cpr(buf: &[u8]) -> Option<Cpr> {
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
pub(super) fn find_cpr(buf: &[u8]) -> Option<usize> {
    parse_cpr(buf).map(|c| c.end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn csi(body: &str) -> Reply {
        Reply { kind: ReplyKind::Csi, body: body.to_string() }
    }

    #[test]
    fn finds_a_cursor_report_among_other_bytes() {
        assert_eq!(find_cpr(b"q\x1b[12;40R"), Some(9));
        assert_eq!(find_cpr(b"\x1b[12;40"), None);
    }

    #[test]
    fn status_reports_are_counted_and_taken_out_wherever_they_sit() {
        let mut buf = b"a\x1b[0n\x1b[0n\x1b[12;3Rb\x1b[0".to_vec();
        assert_eq!(strip_status_replies(&mut buf), 2);
        assert_eq!(buf, b"a\x1b[12;3Rb\x1b[0");
        // The rest of a split report arrives in the next read and is found then.
        buf.extend_from_slice(b"n");
        assert_eq!(strip_status_replies(&mut buf), 1);
        assert_eq!(buf, b"a\x1b[12;3Rb");
        // Other reports that look alike are left alone.
        let mut other = b"\x1b[3n\x1b[?0n\x1b[10n".to_vec();
        assert_eq!(strip_status_replies(&mut other), 0);
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
}
