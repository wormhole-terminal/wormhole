//! What a payload is made of, counted from its bytes: text against escape sequences, and how many
//! of each kind of sequence. A slow result can then be tied to what the workload actually stresses
//! (a terminal that is slow at `colorflood` but not at `logflood` has a problem with SGR, not with
//! bytes).

use super::json::{self, Value};

/// The counts for one payload. Sequences are counted by what they do, not by their exact code.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Composition {
    pub bytes: u64,
    /// Printable text and control characters (CR, LF, tab): everything outside escape sequences.
    pub text_bytes: u64,
    pub escape_bytes: u64,
    /// `CSI … m`.
    pub sgr: u64,
    /// Cursor movement and positioning: CUP, CUU/CUD/CUF/CUB, CHA, VPA, save and restore.
    pub cursor: u64,
    /// `CSI J`, `CSI K` and `CSI X`: erase in display, in line, characters.
    pub erase: u64,
    /// Scrolling and editing in place: DECSTBM, SU/SD, IL/DL, ICH/DCH and reverse index.
    pub scroll: u64,
    /// `ESC ]`: window titles, hyperlinks (OSC 8) and the like.
    pub osc: u64,
    /// `ESC P`, `ESC _`, `ESC ^`, `ESC X`: strings that run to a string terminator.
    pub dcs: u64,
    /// `CSI … h` and `CSI … l`: modes (alternate screen, synchronized output, origin, wrap).
    pub modes: u64,
    pub other: u64,
    /// Line feeds.
    pub lines: u64,
    /// Characters in the text (not bytes).
    pub chars: u64,
    /// Of those, the ones that take two cells (CJK, emoji).
    pub wide: u64,
    /// How many writes the payload goes out in.
    pub writes: u64,
}

impl Composition {
    pub fn sequences(&self) -> u64 {
        self.sgr + self.cursor + self.erase + self.scroll + self.osc + self.dcs + self.modes + self.other
    }

    pub fn avg_write(&self) -> f64 {
        if self.writes == 0 { 0.0 } else { self.bytes as f64 / self.writes as f64 }
    }

    const FIELDS: [&'static str; 15] = [
        "bytes",
        "text_bytes",
        "escape_bytes",
        "sgr",
        "cursor",
        "erase",
        "scroll",
        "osc",
        "dcs",
        "modes",
        "other",
        "lines",
        "chars",
        "wide",
        "writes",
    ];

    fn values(&self) -> [u64; 15] {
        [
            self.bytes,
            self.text_bytes,
            self.escape_bytes,
            self.sgr,
            self.cursor,
            self.erase,
            self.scroll,
            self.osc,
            self.dcs,
            self.modes,
            self.other,
            self.lines,
            self.chars,
            self.wide,
            self.writes,
        ]
    }

    pub fn to_json(&self) -> String {
        let members: Vec<String> =
            Self::FIELDS.iter().zip(self.values()).map(|(k, v)| format!("{}: {v}", json::quote(k))).collect();
        format!("{{{}}}", members.join(", "))
    }

    pub fn from_value(v: &Value) -> Composition {
        let n = |k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(0.0) as u64;
        Composition {
            bytes: n("bytes"),
            text_bytes: n("text_bytes"),
            escape_bytes: n("escape_bytes"),
            sgr: n("sgr"),
            cursor: n("cursor"),
            erase: n("erase"),
            scroll: n("scroll"),
            osc: n("osc"),
            dcs: n("dcs"),
            modes: n("modes"),
            other: n("other"),
            lines: n("lines"),
            chars: n("chars"),
            wide: n("wide"),
            writes: n("writes"),
        }
    }
}

/// Characters that take two cells: the East Asian wide and fullwidth blocks and the emoji planes.
/// Close enough for a count; the terminal under test is the authority on widths.
pub fn is_wide(c: char) -> bool {
    matches!(c as u32,
        0x1100..=0x115F | 0x2E80..=0x303E | 0x3041..=0x33FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF
        | 0xA000..=0xA4CF | 0xAC00..=0xD7A3 | 0xF900..=0xFAFF | 0xFE30..=0xFE6F | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6 | 0x1F300..=0x1F64F | 0x1F680..=0x1F6FF | 0x1F900..=0x1F9FF | 0x20000..=0x3FFFD)
}

/// Counts what `bytes` is made of; `writes` is how many writes it goes out in.
pub fn analyze(bytes: &[u8], writes: usize) -> Composition {
    let mut c = Composition { bytes: bytes.len() as u64, writes: writes as u64, ..Composition::default() };
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b != 0x1b {
            let len = match b {
                0xF0.. => 4,
                0xE0.. => 3,
                0xC0.. => 2,
                _ => 1,
            };
            let end = (i + len).min(bytes.len());
            c.text_bytes += (end - i) as u64;
            // Continuation bytes without a lead are counted as bytes only.
            if !(0x80..0xC0).contains(&b) {
                c.chars += 1;
                if len > 1
                    && std::str::from_utf8(&bytes[i..end])
                        .ok()
                        .and_then(|s| s.chars().next())
                        .is_some_and(is_wide)
                {
                    c.wide += 1;
                }
            }
            c.lines += u64::from(b == b'\n');
            i = end;
            continue;
        }
        let end = escape_end(bytes, i);
        c.escape_bytes += (end - i) as u64;
        match bytes.get(i + 1) {
            Some(b'[') => classify_csi(&mut c, &bytes[i + 2..end]),
            Some(b']') => c.osc += 1,
            Some(b'P' | b'_' | b'^' | b'X') => c.dcs += 1,
            Some(b'M') => c.scroll += 1,
            _ => c.other += 1,
        }
        i = end;
    }
    c
}

fn classify_csi(c: &mut Composition, body: &[u8]) {
    let Some(&last) = body.last() else {
        c.other += 1;
        return;
    };
    let spaced = body.len() >= 2 && (0x20..=0x2f).contains(&body[body.len() - 2]);
    match last {
        _ if spaced => c.other += 1,
        b'm' => c.sgr += 1,
        b'H' | b'f' | b'A' | b'B' | b'C' | b'D' | b'E' | b'F' | b'G' | b'd' | b'`' | b'a' | b'e' | b's'
        | b'u' => c.cursor += 1,
        b'J' | b'K' | b'X' => c.erase += 1,
        b'L' | b'M' | b'S' | b'T' | b'r' | b'@' | b'P' => c.scroll += 1,
        b'h' | b'l' => c.modes += 1,
        _ => c.other += 1,
    }
}

/// The index just past the escape sequence that starts at `start` (the end of the data when it is cut).
fn escape_end(b: &[u8], start: usize) -> usize {
    let n = b.len();
    match b.get(start + 1) {
        None => n,
        Some(b'[') => {
            let mut j = start + 2;
            while j < n && (0x20..=0x3f).contains(&b[j]) {
                j += 1;
            }
            (j + 1).min(n)
        }
        Some(b']' | b'P' | b'_' | b'^' | b'X') => {
            let mut j = start + 2;
            while j < n {
                if b[j] == 0x07 {
                    return j + 1;
                }
                if b[j] == 0x1b && b.get(j + 1) == Some(&b'\\') {
                    return j + 2;
                }
                j += 1;
            }
            n
        }
        Some(_) => {
            // ESC, intermediates, one final byte.
            let mut j = start + 1;
            while j < n && (0x20..=0x2f).contains(&b[j]) {
                j += 1;
            }
            (j + 1).min(n)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_tells_text_from_sequences() {
        let data = "\x1b[1;31mred\x1b[0m\r\nok\x1b[2K\x1b[5;1H\x1b[?1049h\x1b]8;id=1;file:///a\x1b\\link\x1b]8;;\x1b\\\x1b[3L\x1bM\x1b[2 q";
        let c = analyze(data.as_bytes(), 2);
        assert_eq!((c.sgr, c.erase, c.cursor, c.modes, c.osc, c.scroll, c.other), (2, 1, 1, 1, 2, 2, 1));
        assert_eq!(c.lines, 1);
        // red, CR LF, ok, link
        assert_eq!(c.text_bytes, 3 + 2 + 2 + 4);
        assert_eq!(c.text_bytes + c.escape_bytes, c.bytes);
        assert_eq!(c.sequences(), 10);
        assert_eq!(c.writes, 2);
    }

    #[test]
    fn it_counts_characters_and_wide_ones() {
        let c = analyze("a漢é😀\u{301}".as_bytes(), 1);
        assert_eq!(c.chars, 5);
        assert_eq!(c.wide, 2);
        assert_eq!(c.escape_bytes, 0);
    }

    #[test]
    fn a_cut_sequence_does_not_run_past_the_end() {
        let c = analyze(b"abc\x1b[38;2;1", 1);
        assert_eq!(c.text_bytes + c.escape_bytes, c.bytes);
        let c = analyze(b"\x1b]0;title", 1);
        assert_eq!((c.osc, c.escape_bytes), (1, 9));
        assert_eq!(analyze(b"\x1b", 1).other, 1);
    }

    #[test]
    fn bell_ends_an_osc() {
        let c = analyze(b"\x1b]0;t\x07x", 1);
        assert_eq!((c.osc, c.text_bytes), (1, 1));
    }

    #[test]
    fn it_survives_the_file() {
        let c = analyze(b"\x1b[1mhi\x1b[0m\r\n", 3);
        let v = json::parse(&c.to_json()).expect("valid json");
        assert_eq!(Composition::from_value(&v), c);
    }
}
