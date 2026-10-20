//! Deterministic byte streams for the benchmarks and the profiling example: what programs send
//! (log lines, colour churn, text in several scripts, scrolling regions, wide clusters, links, OSC
//! payloads). The same name and size give the same bytes on every machine. Included by path with
//! the other helpers; the MIT crate does not depend on the bench crate's corpora, which are
//! larger and also feed the other engines.
#![allow(dead_code)] // every includer uses a part

use super::rng::Rng;

const WORDS: &[&str] = &[
    "the",
    "terminal",
    "engine",
    "reads",
    "bytes",
    "from",
    "a",
    "program",
    "and",
    "draws",
    "screen",
    "cells",
    "history",
    "scrollback",
    "cursor",
    "error",
    "warning",
    "build",
    "finished",
    "in",
    "seconds",
    "src/main.rs",
    "Cargo.toml",
    "tests",
    "passed",
    "failed",
    "ok",
    "agent",
    "status",
    "diff",
];

pub fn word(rng: &mut Rng) -> &'static str {
    rng.pick(WORDS)
}

/// Printable ASCII words with no line breaks: the terminal wraps it.
pub fn ascii_run(size: usize) -> Vec<u8> {
    let mut rng = Rng::new(1);
    let mut out = Vec::with_capacity(size + 32);
    while out.len() < size {
        out.extend_from_slice(word(&mut rng).as_bytes());
        out.push(b' ');
    }
    out
}

/// Log lines of varying length ended with CR LF.
pub fn log_lines(size: usize) -> Vec<u8> {
    let mut rng = Rng::new(2);
    let mut out = Vec::with_capacity(size + 256);
    let mut n = 0u32;
    while out.len() < size {
        n += 1;
        let level = rng.pick(&["INFO", "WARN", "DEBUG", "ERROR"]);
        out.extend_from_slice(
            format!("2026-10-20T12:{:02}:{:02}Z {level:<5} ", (n / 60) % 60, n % 60).as_bytes(),
        );
        for _ in 0..rng.range(3, 24) {
            out.extend_from_slice(word(&mut rng).as_bytes());
            out.push(b' ');
        }
        out.extend_from_slice(b"\r\n");
    }
    out
}

/// Text in several scripts and some emoji, as UTF-8, with line ends.
pub fn utf8_mixed(size: usize) -> Vec<u8> {
    let mut rng = Rng::new(3);
    let pieces: &[&str] = &[
        "café ",
        "naïve ",
        "Привет ",
        "мир ",
        "日本語 ",
        "テキスト ",
        "한국어 ",
        "αβγ ",
        "→ ",
        "✓ ",
        "😀 ",
        "🚀 ",
    ];
    let mut out = Vec::with_capacity(size + 64);
    while out.len() < size {
        for _ in 0..rng.range(4, 20) {
            if rng.chance(60) {
                out.extend_from_slice(word(&mut rng).as_bytes());
                out.push(b' ');
            } else {
                out.extend_from_slice(rng.pick(pieces).as_bytes());
            }
        }
        out.extend_from_slice(b"\r\n");
    }
    out
}

/// A colour change every few characters: true colour, 256 colours, attributes and resets.
pub fn sgr_heavy(size: usize) -> Vec<u8> {
    let mut rng = Rng::new(4);
    let mut out = Vec::with_capacity(size + 64);
    while out.len() < size {
        match rng.below(5) {
            0 => out.extend_from_slice(
                format!("\x1b[38;2;{};{};{}m", rng.below(256), rng.below(256), rng.below(256)).as_bytes(),
            ),
            1 => out.extend_from_slice(format!("\x1b[48;5;{}m", rng.below(256)).as_bytes()),
            2 => out.extend_from_slice(format!("\x1b[1;{}m", 31 + rng.below(7)).as_bytes()),
            3 => out.extend_from_slice(b"\x1b[0m"),
            _ => out.extend_from_slice(b"\x1b[3;4:3;58:2::255:128:0m"),
        }
        out.extend_from_slice(word(&mut rng).as_bytes());
        if rng.chance(8) {
            out.extend_from_slice(b"\r\n");
        }
    }
    out
}

/// OSC strings: titles, hyperlinks around text, and a long clipboard payload.
pub fn osc_payload(size: usize) -> Vec<u8> {
    let mut rng = Rng::new(5);
    let mut out = Vec::with_capacity(size + 4096);
    while out.len() < size {
        match rng.below(4) {
            0 => {
                out.extend_from_slice(format!("\x1b]0;{} {}\x07", word(&mut rng), word(&mut rng)).as_bytes())
            }
            1 => {
                let id = rng.below(1000);
                out.extend_from_slice(
                    format!(
                        "\x1b]8;id={id};https://example.com/{}/{id}\x1b\\{}\x1b]8;;\x1b\\ ",
                        word(&mut rng),
                        word(&mut rng)
                    )
                    .as_bytes(),
                );
            }
            2 => out
                .extend_from_slice(format!("\x1b]7;file://host/home/user/{}\x07", word(&mut rng)).as_bytes()),
            _ => {
                out.extend_from_slice(b"\x1b]52;c;");
                for _ in 0..rng.range(200, 3000) {
                    out.push(b'A' + rng.below(26) as u8);
                }
                out.push(0x07);
            }
        }
    }
    out
}

/// A scrolling region (DECSTBM) with a status line outside it: lines scroll within the region only.
pub fn scroll_region(size: usize) -> Vec<u8> {
    let mut rng = Rng::new(6);
    let mut out = b"\x1b[2J\x1b[1;1Hheader\x1b[24;1Hstatus\x1b[2;23r\x1b[23;1H".to_vec();
    while out.len() < size {
        for _ in 0..rng.range(3, 20) {
            out.extend_from_slice(word(&mut rng).as_bytes());
            out.push(b' ');
        }
        out.extend_from_slice(b"\r\n");
        if rng.chance(5) {
            out.extend_from_slice(
                format!("\x1b7\x1b[24;{}Hrow {}\x1b8", 1 + rng.below(40), rng.below(999)).as_bytes(),
            );
        }
    }
    out
}

/// Wide characters and clusters: CJK, emoji, ZWJ sequences, flags, skin tones, combining marks.
pub fn wide_and_emoji(size: usize) -> Vec<u8> {
    let mut rng = Rng::new(7);
    let pieces: &[&str] = &[
        "日本語",
        "テキスト",
        "漢字",
        "한국어",
        "😀",
        "🚀",
        "👨\u{200d}👩\u{200d}👧",
        "🇯🇵",
        "👍🏽",
        "e\u{301}",
        "a\u{308}\u{323}",
        "❤\u{fe0f}",
        "1\u{fe0f}\u{20e3}",
    ];
    let mut out = b"\x1b[?2027h".to_vec();
    while out.len() < size {
        for _ in 0..rng.range(5, 30) {
            if rng.chance(30) {
                out.extend_from_slice(word(&mut rng).as_bytes());
                out.push(b' ');
            } else {
                out.extend_from_slice(rng.pick(pieces).as_bytes());
            }
        }
        out.extend_from_slice(b"\r\n");
    }
    out
}

/// Text with OSC 8 links, some with an id and some without.
pub fn hyperlinks(size: usize) -> Vec<u8> {
    let mut rng = Rng::new(8);
    let mut out = Vec::with_capacity(size + 256);
    while out.len() < size {
        for _ in 0..rng.range(3, 12) {
            if rng.chance(40) {
                let params = if rng.chance(50) { format!("id=l{}", rng.below(64)) } else { String::new() };
                out.extend_from_slice(
                    format!(
                        "\x1b]8;{params};https://example.com/{}/{}\x1b\\",
                        word(&mut rng),
                        rng.below(500)
                    )
                    .as_bytes(),
                );
                out.extend_from_slice(word(&mut rng).as_bytes());
                out.extend_from_slice(b"\x1b]8;;\x1b\\ ");
            } else {
                out.extend_from_slice(word(&mut rng).as_bytes());
                out.push(b' ');
            }
        }
        out.extend_from_slice(b"\r\n");
    }
    out
}

/// The streams by name: `(name, what it imitates, columns, rows of the terminal it is meant for)`.
pub const NAMES: &[(&str, &str, usize, usize)] = &[
    ("ascii", "printable words with no line breaks: the terminal wraps it", 80, 24),
    ("log", "timestamped log lines, ended with CR LF", 120, 40),
    ("utf8", "text in several scripts, with emoji", 120, 40),
    ("sgr", "a colour or attribute change every few characters", 120, 40),
    ("osc", "titles, hyperlinks, working directories and long clipboard payloads", 120, 40),
    ("scroll", "a scrolling region with a status line outside it", 80, 24),
    ("wide", "wide characters and emoji clusters under mode 2027", 120, 40),
    ("links", "text with OSC 8 hyperlinks", 120, 40),
];

/// A stream of about `size` bytes by its name in [`NAMES`].
pub fn generate(name: &str, size: usize) -> Option<Vec<u8>> {
    Some(match name {
        "ascii" => ascii_run(size),
        "log" => log_lines(size),
        "utf8" => utf8_mixed(size),
        "sgr" => sgr_heavy(size),
        "osc" => osc_payload(size),
        "scroll" => scroll_region(size),
        "wide" => wide_and_emoji(size),
        "links" => hyperlinks(size),
        _ => return None,
    })
}
