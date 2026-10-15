//! Byte streams a terminal engine is measured and compared on.
//!
//! Each generator is deterministic: the same size and seed give the same bytes on every machine
//! and every run, so a speed baseline stored for a corpus stays comparable (a test pins a
//! checksum of every corpus; change a generator and the baselines must be redone on purpose).
//! They imitate what real programs send: plain output and logs, color churn, text in many
//! scripts, full-screen programs redrawing with cursor addressing, scrolling regions,
//! hyperlinks, the long strings of graphics protocols, and a coding agent's session mixing all
//! of it. Recorded sessions from real programs (`scripts/vt-record.sh`) are measured the same way.

use std::borrow::Cow;

mod out;
mod payload;
mod recorded;
mod screen;
mod session;
mod style;
mod text;
mod words;

pub(crate) use out::Out;
pub use recorded::{recorded, recorded_names};

/// One stream and the screen it is meant for.
#[derive(Debug, Clone)]
pub struct Corpus {
    /// A generator's name is static; a recording's is its file's.
    pub name: Cow<'static, str>,
    pub description: &'static str,
    pub cols: usize,
    pub rows: usize,
    pub bytes: Vec<u8>,
}

type Generate = fn(&mut Out, usize);

struct Entry {
    name: &'static str,
    description: &'static str,
    cols: usize,
    rows: usize,
    generate: Generate,
}

const ENTRIES: &[Entry] = &[
    Entry {
        name: "ascii-flood",
        description: "printable ASCII words with no line breaks: the terminal wraps it",
        cols: 80,
        rows: 24,
        generate: text::ascii_flood,
    },
    Entry {
        name: "log-lines",
        description: "timestamped log lines of varying length, some colored, ended with CR LF",
        cols: 120,
        rows: 40,
        generate: text::log_lines,
    },
    Entry {
        name: "sgr-churn",
        description: "256-color and truecolor foregrounds and backgrounds, attributes and underline styles changing every few characters",
        cols: 100,
        rows: 30,
        generate: style::sgr_churn,
    },
    Entry {
        name: "unicode-mix",
        description: "CJK, emoji (ZWJ sequences, flags, skin tones, VS16), combining marks, accents and box drawing among ASCII",
        cols: 100,
        rows: 30,
        generate: text::unicode_mix,
    },
    Entry {
        name: "tui-redraw",
        description: "a full-screen program (htop or vim like) redrawing with cursor addressing on the alternate screen",
        cols: 120,
        rows: 40,
        generate: screen::tui_redraw,
    },
    Entry {
        name: "scroll-region",
        description: "output scrolling inside DECSTBM regions with IL, DL, SU, SD, RI and a fixed header and footer",
        cols: 100,
        rows: 30,
        generate: screen::scroll_region,
    },
    Entry {
        name: "osc8-links",
        description: "text with OSC 8 hyperlinks (BEL and ST), titles, working directories and prompt marks",
        cols: 120,
        rows: 40,
        generate: style::osc8_links,
    },
    Entry {
        name: "payloads",
        description: "long OSC, DCS and APC strings (kitty graphics chunks, sixel, clipboard) between lines of text",
        cols: 120,
        rows: 40,
        generate: payload::payloads,
    },
    Entry {
        name: "agent-session",
        description: "a coding agent's session: streamed markdown, spinners, diffs, boxes, progress bars, a status line, an image",
        cols: 120,
        rows: 40,
        generate: session::agent_session,
    },
];

/// The names of every corpus, in the order `all` returns them.
pub fn names() -> Vec<&'static str> {
    ENTRIES.iter().map(|e| e.name).collect()
}

/// The corpus called `name`, about `size` bytes long (it ends at the first natural break after
/// that many), or `None` for an unknown name.
pub fn generate(name: &str, size: usize, seed: u64) -> Option<Corpus> {
    let entry = ENTRIES.iter().find(|e| e.name == name)?;
    // Every corpus has its own stream of numbers, so changing the seed changes all of them and
    // adding a corpus changes none of the others.
    let mut out = Out::new(seed ^ fnv1a(entry.name.as_bytes()), entry.cols, entry.rows);
    (entry.generate)(&mut out, size);
    Some(Corpus {
        name: Cow::Borrowed(entry.name),
        description: entry.description,
        cols: entry.cols,
        rows: entry.rows,
        bytes: out.bytes,
    })
}

/// Every corpus.
pub fn all(size: usize, seed: u64) -> Vec<Corpus> {
    ENTRIES.iter().filter_map(|e| generate(e.name, size, seed)).collect()
}

/// A checksum of some bytes (FNV-1a, 64 bits): enough to notice that a stream changed.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Engine;
    use crate::engines::WormholeEngine;

    const SMALL: usize = 64 * 1024;

    #[test]
    fn every_corpus_is_deterministic_and_reaches_its_size() {
        for name in names() {
            let a = generate(name, SMALL, 1).expect("a known corpus");
            let b = generate(name, SMALL, 1).expect("a known corpus");
            assert_eq!(a.bytes, b.bytes, "{name} differs between runs");
            assert!(a.bytes.len() >= SMALL, "{name} is only {} bytes", a.bytes.len());
            assert!(a.bytes.len() < SMALL * 2, "{name} overshoots: {} bytes", a.bytes.len());
            let other = generate(name, SMALL, 2).expect("a known corpus");
            assert_ne!(a.bytes, other.bytes, "{name} ignores the seed");
        }
        assert!(generate("nope", SMALL, 1).is_none());
    }

    /// Whether the bytes hold `ESC [`, digits and semicolons, and then `final_byte`.
    fn has_csi(bytes: &[u8], final_byte: u8) -> bool {
        bytes.windows(2).enumerate().any(|(i, w)| {
            w == b"\x1b["
                && bytes[i + 2..].iter().find(|b| !b.is_ascii_digit() && **b != b';') == Some(&final_byte)
        })
    }

    #[test]
    fn the_corpora_are_what_they_say() {
        let has = |name: &str, needle: &[u8]| {
            let c = generate(name, SMALL, 1).expect("a known corpus");
            c.bytes.windows(needle.len()).any(|w| w == needle)
        };
        assert!(!has("ascii-flood", b"\n") && !has("ascii-flood", b"\x1b"));
        assert!(has("log-lines", b"\r\n") && has("log-lines", b"\x1b[3"));
        assert!(
            has("sgr-churn", b"\x1b[38;2;") && has("sgr-churn", b"\x1b[48;5;") && has("sgr-churn", b";58;2;")
        );
        assert!(has("unicode-mix", "日本語".as_bytes()) && has("unicode-mix", "\u{200d}".as_bytes()));
        assert!(has("tui-redraw", b"\x1b[?1049h") && has("tui-redraw", b";7m"));
        let region = generate("scroll-region", SMALL, 1).expect("a known corpus").bytes;
        for final_byte in *b"rLMST" {
            assert!(has_csi(&region, final_byte), "scroll-region has no CSI ... {}", final_byte as char);
        }
        assert!(has("scroll-region", b"\x1bM"));
        assert!(has("osc8-links", b"\x1b]8;;http") && has("osc8-links", b"\x1b]133;A"));
        assert!(
            has("payloads", b"\x1b_G") && has("payloads", b"\x1bP0;0;0q") && has("payloads", b"\x1b]52;c;")
        );
        assert!(has("agent-session", b"\x1b[?2004h") && has("agent-session", b"\x1b7"));
    }

    #[test]
    fn the_corpora_have_not_changed() {
        // A baseline stored for a corpus is only comparable while the bytes are the same.
        let sums: Vec<(&str, u64)> = names()
            .into_iter()
            .map(|n| (n, fnv1a(&generate(n, SMALL, 1).expect("a known corpus").bytes)))
            .collect();
        let expected: &[(&str, u64)] = &[
            ("ascii-flood", 9_433_031_953_171_645_930),
            ("log-lines", 15_030_378_068_657_503_643),
            ("sgr-churn", 11_373_935_271_390_460_145),
            ("unicode-mix", 1_851_989_150_975_874_298),
            ("tui-redraw", 3_780_834_811_031_278_053),
            ("scroll-region", 10_096_232_420_488_500_550),
            ("osc8-links", 7_046_314_598_742_842_484),
            ("payloads", 338_754_485_453_849_054),
            ("agent-session", 16_580_751_208_201_259_506),
        ];
        assert_eq!(sums, expected, "update these checksums only when a generator changes on purpose");
    }

    #[test]
    fn an_engine_makes_something_of_every_corpus() {
        for corpus in all(SMALL, 1) {
            let mut engine = WormholeEngine::new(corpus.cols, corpus.rows, 1000);
            for chunk in corpus.bytes.chunks(4096) {
                engine.feed(chunk);
            }
            let shown: usize = (0..corpus.rows).map(|r| engine.row_text(r).len()).sum();
            let (cols, rows) = engine.size();
            assert_eq!((cols, rows), (corpus.cols, corpus.rows));
            assert!(shown > 0 || engine.scrollback_len() > 0, "{} leaves nothing on the screen", corpus.name);
        }
    }
}
