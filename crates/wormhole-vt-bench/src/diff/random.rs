//! Random byte streams that lean toward valid sequences.
//!
//! Most of a stream is well-formed: text, controls, CSI, ESC and string sequences with
//! parameters in or near the screen's size, so the engines under test actually move the cursor,
//! scroll, erase and change modes instead of discarding noise. A few percent is not: truncated
//! sequences, sequences with huge or too many parameters, and raw bytes. Colors are queried but not
//! changed (see `osc`).

use crate::rng::Rng;

/// A deterministic source of streams for a screen of a given size.
pub struct StreamGen {
    rng: Rng,
    cols: usize,
    rows: usize,
    ascii_only: bool,
}

const ACCENTED: [&str; 8] = ["é", "ü", "ñ", "ß", "ø", "Ω", "я", "→"];
const WIDE: [&str; 6] = ["日", "本", "語", "한", "😀", "Ａ"];
const MARKS: [&str; 4] = ["\u{301}", "\u{308}", "\u{323}", "\u{200d}"];
const MODES: [u32; 10] = [4, 20, 25, 7, 6, 69, 1049, 47, 1047, 1048];
const TITLES: [&str; 4] = ["one", "two words", "日本語", ""];

impl StreamGen {
    pub fn new(seed: u64, cols: usize, rows: usize) -> StreamGen {
        StreamGen { rng: Rng::new(seed), cols, rows, ascii_only: false }
    }

    /// Leave out every byte above 0x7f: no UTF-8 text and no stray bytes, so a stream means the
    /// same wherever it is cut.
    pub fn ascii_only(mut self) -> StreamGen {
        self.ascii_only = true;
        self
    }

    /// A stream of at least `len` bytes (it ends at a whole token).
    pub fn stream(&mut self, len: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(len + 64);
        while out.len() < len {
            self.token(&mut out);
        }
        out
    }

    fn token(&mut self, out: &mut Vec<u8>) {
        match self.rng.below(100) {
            0..=29 => self.text(out),
            30..=37 => self.unicode(out),
            38..=47 => self.control(out),
            48..=77 => self.csi(out),
            78..=87 => self.esc(out),
            88..=92 => self.osc(out),
            93..=94 => self.string(out),
            _ => self.broken(out),
        }
    }

    fn text(&mut self, out: &mut Vec<u8>) {
        for _ in 0..self.rng.range(1, 24) {
            out.push(b' ' + self.rng.below(95) as u8);
        }
    }

    fn unicode(&mut self, out: &mut Vec<u8>) {
        if self.ascii_only {
            return self.text(out);
        }
        for _ in 0..self.rng.range(1, 4) {
            let piece = match self.rng.below(4) {
                0 => *self.rng.pick(&ACCENTED),
                1 => *self.rng.pick(&WIDE),
                2 => *self.rng.pick(&MARKS),
                _ => "a",
            };
            out.extend_from_slice(piece.as_bytes());
        }
    }

    fn control(&mut self, out: &mut Vec<u8>) {
        // CR, LF, TAB, BS, BEL, VT, FF, SO, SI, NUL, DEL.
        out.push(
            *self.rng.pick(&[b'\r', b'\n', b'\n', b'\t', 0x08, 0x07, 0x0b, 0x0c, 0x0e, 0x0f, 0x00, 0x7f]),
        );
    }

    /// A number around the screen's size (a little beyond it too).
    fn near(&mut self, limit: usize) -> usize {
        match self.rng.below(10) {
            0 => 0,
            1 => limit + self.rng.range(1, 4),
            _ => self.rng.range(1, limit.max(1)),
        }
    }

    fn csi(&mut self, out: &mut Vec<u8>) {
        let (cols, rows) = (self.cols, self.rows);
        out.extend_from_slice(b"\x1b[");
        let body = match self.rng.below(32) {
            0..=3 => {
                let finals = *self.rng.pick(&["A", "B", "C", "D", "E", "F", "G", "d", "`", "a", "e"]);
                let n = self.near(rows.max(cols) / 2);
                format!("{n}{finals}")
            }
            4..=6 => {
                let (r, c) = (self.near(rows), self.near(cols));
                let f = *self.rng.pick(&["H", "H", "f"]);
                format!("{r};{c}{f}")
            }
            7 => format!("{}J", self.rng.below(4)),
            8 => format!("{}K", self.rng.below(3)),
            9 => format!("{}X", self.near(cols)),
            10 => {
                let f = *self.rng.pick(&["@", "P"]);
                format!("{}{f}", self.near(cols))
            }
            11 => {
                let f = *self.rng.pick(&["L", "M", "S", "T"]);
                format!("{}{f}", self.near(rows))
            }
            12 => format!("{}b", self.rng.range(1, 40)),
            13 => {
                let f = *self.rng.pick(&["I", "Z"]);
                format!("{}{f}", self.rng.range(0, 4))
            }
            14 => format!("{}g", *self.rng.pick(&[0, 3, 2])),
            15..=19 => format!("{}m", self.sgr()),
            20 | 21 => {
                let (top, bottom) = (self.near(rows), self.near(rows));
                if self.rng.one_in(4) { "r".to_string() } else { format!("{top};{bottom}r") }
            }
            22 => {
                let mode = *self.rng.pick(&MODES);
                let f = if self.rng.one_in(2) { "h" } else { "l" };
                // IRM and LNM are ANSI modes, the rest DEC private ones.
                let private = if mode == 4 || mode == 20 { "" } else { "?" };
                format!("{private}{mode}{f}")
            }
            23 => {
                let n = *self.rng.pick(&[5, 6, 5, 6]);
                format!("{n}n")
            }
            24 => (*self.rng.pick(&[
                "c", ">c", "?6n", "?15n", "=c", ">q", "18t", "14t", "16t", "22;0t", "23;0t", "!p", "s", "u",
            ]))
            .to_string(),
            25 => {
                let m = *self.rng.pick(&[4, 7, 25, 1049, 69, 2027, 9999, 20]);
                let private = if self.rng.one_in(2) { "?" } else { "" };
                format!("{private}{m}$p")
            }
            26 => format!("{} q", self.rng.below(7)),
            27 => format!("{}\"q", self.rng.below(3)),
            28 => {
                let f = *self.rng.pick(&["J", "K"]);
                format!("?{}{f}", self.rng.below(3))
            }
            29 => {
                let (l, r) = (self.near(cols), self.near(cols));
                format!("{l};{r}s")
            }
            30 => format!("{}'{}", self.rng.range(1, 3), *self.rng.pick(&["}", "~"])),
            _ => "?5W".to_string(),
        };
        out.extend_from_slice(body.as_bytes());
    }

    fn sgr(&mut self) -> String {
        let mut parts: Vec<String> = Vec::new();
        for _ in 0..self.rng.range(0, 4) {
            let part = match self.rng.below(16) {
                0 => "0".to_string(),
                1 => self.rng.range(1, 9).to_string(),
                2 => (*self.rng.pick(&[21, 22, 23, 24, 25, 27, 28, 29, 53, 55])).to_string(),
                3 | 4 => (30 + self.rng.below(8)).to_string(),
                5 | 6 => (40 + self.rng.below(8)).to_string(),
                7 => (90 + self.rng.below(8)).to_string(),
                8 => (100 + self.rng.below(8)).to_string(),
                9 => (*self.rng.pick(&["39", "49", "59"])).to_string(),
                10 => format!("38;5;{}", self.rng.below(256)),
                11 => format!("48;5;{}", self.rng.below(256)),
                12 => format!("38;2;{};{};{}", self.rng.below(256), self.rng.below(256), self.rng.below(256)),
                13 => {
                    format!("48:2::{}:{}:{}", self.rng.below(256), self.rng.below(256), self.rng.below(256))
                }
                14 => format!("4:{}", self.rng.below(6)),
                _ => format!("58;5;{}", self.rng.below(256)),
            };
            parts.push(part);
        }
        parts.join(";")
    }

    fn esc(&mut self, out: &mut Vec<u8>) {
        let seq: &[u8] = self.rng.pick(&[
            &b"\x1b7"[..],
            b"\x1b8",
            b"\x1bD",
            b"\x1bE",
            b"\x1bH",
            b"\x1bM",
            b"\x1bM",
            b"\x1b=",
            b"\x1b>",
            b"\x1b#8",
            b"\x1b(0",
            b"\x1b(B",
            b"\x1b(A",
            b"\x1b)0",
            b"\x1b)B",
            b"\x1b*0",
            b"\x1b+0",
            b"\x1bn",
            b"\x1bo",
            b"\x1bN",
            b"\x1bO",
            b"\x1b6",
            b"\x1b9",
            b"\x1bZ",
            b"\x1b\\",
        ]);
        // A full reset now and then; if it came as often as the others the streams would never
        // get anywhere.
        if self.rng.one_in(60) {
            out.extend_from_slice(b"\x1bc");
        } else {
            out.extend_from_slice(seq);
        }
    }

    fn title(&mut self) -> &'static str {
        let title = *self.rng.pick(&TITLES);
        if self.ascii_only && !title.is_ascii() { "one" } else { title }
    }

    fn osc(&mut self, out: &mut Vec<u8>) {
        let end: &[u8] = if self.rng.one_in(2) { b"\x07" } else { b"\x1b\\" };
        let body = match self.rng.below(9) {
            0 => format!("0;{}", self.title()),
            1 => format!("2;{}", self.title()),
            2 => format!("1;{}", self.title()),
            3 | 4 => format!("8;;http://example.com/{}", self.rng.below(5)),
            5 => "8;;".to_string(),
            // Colors are only asked for, never set: an adapter answers a query with the colors as
            // they are when `feed` returns, so a query followed by a set in one stream would
            // depend on where the stream is cut.
            6 => format!("4;{};?", self.rng.below(16)),
            7 => (*self.rng.pick(&["10;?", "11;?", "12;?", "4;1;?", "104", "110", "111"])).to_string(),
            _ => format!("133;{}", *self.rng.pick(&["A", "B", "C", "D;0"])),
        };
        out.extend_from_slice(b"\x1b]");
        out.extend_from_slice(body.as_bytes());
        out.extend_from_slice(end);
    }

    /// DCS, APC, PM and SOS strings, which an engine must read to their end and not show.
    fn string(&mut self, out: &mut Vec<u8>) {
        let head: &[u8] = self.rng.pick(&[
            &b"\x1bP$qm"[..],
            b"\x1bP$qr",
            b"\x1bP+q544e",
            b"\x1bP1;2|abc",
            b"\x1b_Gi=1;AAAA",
            b"\x1b^note",
            b"\x1bXtext",
        ]);
        out.extend_from_slice(head);
        for _ in 0..self.rng.range(0, 30) {
            out.push(b'A' + self.rng.below(26) as u8);
        }
        out.extend_from_slice(b"\x1b\\");
    }

    /// Not valid: a sequence cut short, too many or huge parameters, stray bytes.
    fn broken(&mut self, out: &mut Vec<u8>) {
        match self.rng.below(7) {
            0 => out.extend_from_slice(b"\x1b["),
            1 => out.extend_from_slice(b"\x1b[1;2;3;4;5;6;7;8;9;10;11;12;13;14;15;16;17;18;19;20m"),
            2 => out.extend_from_slice(b"\x1b[99999999999999999999C"),
            3 => out.extend_from_slice(b"\x1b]0;unfinished"),
            4 => out.extend_from_slice(b"\x1b[?9999;1;zq"),
            5 if !self.ascii_only => out.push(0x80 + self.rng.below(128) as u8),
            5 => out.push(b'~'),
            _ => out.extend_from_slice(&[0x18, 0x1a, 0x1b, 0x1b]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StreamGen;

    #[test]
    fn streams_are_deterministic_and_long_enough() {
        let a = StreamGen::new(5, 40, 12).stream(2000);
        assert_eq!(a, StreamGen::new(5, 40, 12).stream(2000));
        assert_ne!(a, StreamGen::new(6, 40, 12).stream(2000));
        assert!(a.len() >= 2000 && a.len() < 2300, "{}", a.len());
    }

    #[test]
    fn streams_are_mostly_escape_sequences_and_text() {
        let stream = StreamGen::new(1, 40, 12).stream(20_000);
        let escapes = stream.iter().filter(|b| **b == 0x1b).count();
        let printable = stream.iter().filter(|b| (0x20..0x7f).contains(*b)).count();
        assert!(escapes > 500, "{escapes} escapes");
        assert!(printable > stream.len() / 3, "{printable} printable bytes");
    }

    #[test]
    fn ascii_only_streams_have_no_high_bytes() {
        let stream = StreamGen::new(3, 40, 12).ascii_only().stream(30_000);
        assert!(stream.iter().all(|b| *b < 0x80));
        let titles = StreamGen::new(3, 40, 12).stream(30_000);
        assert!(titles.iter().any(|b| *b >= 0x80));
    }

    #[test]
    fn streams_use_every_kind_of_token() {
        let stream = StreamGen::new(2, 40, 12).stream(50_000);
        let has = |needle: &[u8]| stream.windows(needle.len()).any(|w| w == needle);
        for needle in [
            &b"\x1b["[..],
            b"\x1b]0;",
            b"\x1b]8;;http",
            b"\x1b7",
            b"\x1bM",
            b"\x1bP$q",
            b"\x1b(0",
            b"\x1b[?1049h",
            b"\xc3\xa9",
        ] {
            assert!(has(needle), "no {needle:?}");
        }
    }
}
