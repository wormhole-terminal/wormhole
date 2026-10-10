//! The bytes each benchmark sends. They are built in memory before the clock starts and come from
//! a seeded generator, so every terminal is fed the same stream (the ones that draw to the screen
//! depend on its size, so compare terminals at the same size).
//!
//! Lines end in CR LF: the benchmark runs in raw mode, where a bare LF would not return the carriage.

use crate::canvas::{self, Pixels, Rgb, Rng};
use crate::term::Size;

/// The tests in the order they run.
pub const NAMES: [&str; 8] =
    ["ascii", "unicode", "truecolor", "scroll-region", "cursor", "long-lines", "frames", "latency"];

pub fn about(name: &str) -> &'static str {
    match name {
        "ascii" => "plain log lines, like cat of a big file",
        "unicode" => "CJK, emoji, accents, box drawing",
        "truecolor" => "every character with its own colors",
        "scroll-region" => "a pinned header and footer, text scrolling between",
        "cursor" => "random cursor moves and short colored writes",
        "long-lines" => "lines of 5 to 50 thousand characters",
        "frames" => "full-screen truecolor redraws, synchronized",
        "latency" => "cursor report round trip on an idle terminal",
        _ => "",
    }
}

/// What a throughput test sends, and how many screen frames that is (0 when it isn't frames).
pub struct Payload {
    pub bytes: Vec<u8>,
    pub frames: usize,
}

/// The bytes for the throughput test `name`, a little over `target` bytes. `None` for `latency`
/// (which sends nothing but questions) and for names that don't exist.
pub fn payload(name: &str, size: Size, target: usize) -> Option<Payload> {
    let seed = name.bytes().fold(0x9E37_79B9u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64));
    let mut rng = Rng::new(seed);
    let mut out = Vec::with_capacity(target + 64 * 1024);
    let mut frames = 0;
    match name {
        "ascii" => ascii(&mut rng, &mut out, target),
        "unicode" => unicode(&mut rng, &mut out, target),
        "truecolor" => truecolor(&mut rng, &mut out, target, size),
        "scroll-region" => scroll_region(&mut rng, &mut out, target, size),
        "cursor" => cursor(&mut rng, &mut out, target, size),
        "long-lines" => long_lines(&mut rng, &mut out, target),
        "frames" => frames = frames_stream(&mut out, target, size),
        _ => return None,
    }
    Some(Payload { bytes: out, frames })
}

const WORDS: [&str; 40] = [
    "request",
    "handler",
    "connection",
    "timeout",
    "retry",
    "cache",
    "miss",
    "hit",
    "user",
    "session",
    "token",
    "refresh",
    "upstream",
    "latency",
    "payload",
    "buffer",
    "worker",
    "queue",
    "scheduler",
    "index",
    "commit",
    "rebase",
    "merge",
    "branch",
    "deploy",
    "build",
    "artifact",
    "pipeline",
    "metric",
    "trace",
    "span",
    "error",
    "warning",
    "info",
    "debug",
    "thread",
    "pool",
    "socket",
    "stream",
    "chunk",
];

const LEVELS: [&str; 5] = ["INFO ", "DEBUG", "WARN ", "INFO ", "TRACE"];

fn push_hex(out: &mut Vec<u8>, mut v: u64, digits: usize) {
    for _ in 0..digits {
        out.push(b"0123456789abcdef"[(v & 15) as usize]);
        v >>= 4;
    }
}

/// Words, numbers and hex ids separated by spaces, exactly `len` ASCII bytes long.
fn words(rng: &mut Rng, out: &mut Vec<u8>, len: usize) {
    let end = out.len() + len;
    while out.len() < end {
        match rng.below(8) {
            0 => canvas::push_num(out, rng.below(100_000) as u32),
            1 => push_hex(out, rng.next_u64(), 8),
            2 => {
                out.extend_from_slice(b"id=");
                push_hex(out, rng.next_u64(), 6);
            }
            _ => out.extend_from_slice(WORDS[rng.below(WORDS.len())].as_bytes()),
        }
        out.push(b' ');
    }
    out.truncate(end);
}

/// One line of a service log: a timestamp, a level, a module and a message, `len` bytes long.
fn log_line(rng: &mut Rng, out: &mut Vec<u8>, len: usize) {
    let start = out.len();
    out.extend_from_slice(b"2026-10-");
    let (day, h, m, s, ms) =
        (1 + rng.below(28), rng.below(24), rng.below(60), rng.below(60), rng.below(1000));
    for (v, sep) in [(day, b'T'), (h, b':'), (m, b':'), (s, b'.')] {
        out.extend_from_slice(&[b'0' + (v / 10) as u8, b'0' + (v % 10) as u8, sep]);
    }
    out.extend_from_slice(&[b'0' + (ms / 100) as u8, b'0' + (ms / 10 % 10) as u8, b'0' + (ms % 10) as u8]);
    out.extend_from_slice(b"Z ");
    out.extend_from_slice(LEVELS[rng.below(LEVELS.len())].as_bytes());
    out.push(b' ');
    out.extend_from_slice(WORDS[rng.below(WORDS.len())].as_bytes());
    out.extend_from_slice(b"::");
    out.extend_from_slice(WORDS[rng.below(WORDS.len())].as_bytes());
    out.push(b' ');
    let have = out.len() - start;
    if have >= len {
        out.truncate(start + len);
    } else {
        words(rng, out, len - have);
    }
}

fn ascii(rng: &mut Rng, out: &mut Vec<u8>, target: usize) {
    while out.len() < target {
        let len = 24 + rng.below(160);
        log_line(rng, out, len);
        out.extend_from_slice(b"\r\n");
    }
}

fn push_char(out: &mut Vec<u8>, c: char) {
    let mut b = [0u8; 4];
    out.extend_from_slice(c.encode_utf8(&mut b).as_bytes());
}

fn pick(rng: &mut Rng, lo: u32, hi: u32) -> char {
    char::from_u32(lo + rng.below((hi - lo + 1) as usize) as u32).unwrap_or('?')
}

/// One run of text from a script, about `cells` columns wide.
fn unicode_line(rng: &mut Rng, out: &mut Vec<u8>, cells: usize) {
    const ACCENTED: &str = "éèêëàâäôöûüçñßøåœíóúÿ";
    const EMOJI_SEQUENCES: [&str; 4] = ["👨\u{200d}👩\u{200d}👧", "🇯🇵", "👍🏽", "❤\u{fe0f}"];
    let accented: Vec<char> = ACCENTED.chars().collect();
    let mut width = 0;
    while width < cells {
        match rng.below(20) {
            0..=6 => {
                let n = 2 + rng.below(8);
                words(rng, out, n);
                out.push(b' ');
                width += n + 1;
            }
            7..=11 => {
                let n = 1 + rng.below(6);
                for _ in 0..n {
                    push_char(out, pick(rng, 0x4E00, 0x9FA5));
                }
                width += n * 2;
            }
            12 | 13 => {
                push_char(out, pick(rng, 0x1F600, 0x1F64F));
                width += 2;
            }
            14 => {
                push_char(
                    out,
                    if rng.below(2) == 0 { pick(rng, 0x1F680, 0x1F6C5) } else { pick(rng, 0x1F330, 0x1F37C) },
                );
                width += 2;
            }
            15 => {
                out.extend_from_slice(EMOJI_SEQUENCES[rng.below(EMOJI_SEQUENCES.len())].as_bytes());
                width += 2;
            }
            16 | 17 => {
                let n = 2 + rng.below(6);
                for _ in 0..n {
                    push_char(out, accented[rng.below(accented.len())]);
                }
                // A base letter with a combining mark on top: one cell, two code points.
                out.push(b'a' + rng.below(26) as u8);
                push_char(out, pick(rng, 0x0300, 0x0314));
                width += n + 1;
            }
            _ => {
                let n = 1 + rng.below(8);
                for _ in 0..n {
                    push_char(out, pick(rng, 0x2500, 0x257F));
                }
                width += n;
            }
        }
    }
}

fn unicode(rng: &mut Rng, out: &mut Vec<u8>, target: usize) {
    while out.len() < target {
        let cells = 20 + rng.below(110);
        unicode_line(rng, out, cells);
        out.extend_from_slice(b"\x1b[0m\r\n");
    }
}

fn random_rgb(rng: &mut Rng) -> Rgb {
    let v = rng.next_u64();
    Rgb((v >> 8) as u8, (v >> 24) as u8, (v >> 40) as u8)
}

fn truecolor(rng: &mut Rng, out: &mut Vec<u8>, target: usize, size: Size) {
    let cols = (size.cols as usize).max(1);
    while out.len() < target {
        for _ in 0..cols {
            canvas::fg(out, random_rgb(rng));
            canvas::bg(out, random_rgb(rng));
            out.push(b'!' + rng.below(94) as u8);
        }
        out.extend_from_slice(b"\x1b[0m\r\n");
    }
}

fn scroll_region(rng: &mut Rng, out: &mut Vec<u8>, target: usize, size: Size) {
    let rows = size.rows.max(1);
    let cols = (size.cols as usize).max(8);
    // A header and a status line outside the region, like an editor or tmux; none when too small.
    let (top, bottom) = if rows >= 8 { (3, rows - 2) } else { (1, rows) };
    out.extend_from_slice(b"\x1b[2J");
    if top > 1 {
        canvas::goto(out, 0, 0);
        out.extend_from_slice(b"\x1b[7m termbench scroll-region \x1b[0m");
        canvas::goto(out, 0, rows - 1);
        out.extend_from_slice(b"\x1b[7m -- INSERT -- \x1b[0m");
    }
    out.extend_from_slice(format!("\x1b[{top};{bottom}r").as_bytes());
    // Origin mode keeps CUP inside the region, which is what editors rely on.
    out.extend_from_slice(b"\x1b[?6h\x1b[H");
    let region = bottom - top + 1;
    while out.len() < target {
        for _ in 0..10 + rng.below(190) {
            let len = 8 + rng.below(cols - 7);
            log_line(rng, out, len);
            out.extend_from_slice(b"\r\n");
        }
        match rng.below(10) {
            // Scroll the other way: insert lines at the top (reverse index), as an editor scrolling up.
            0..=2 => {
                canvas::goto(out, 0, 0);
                for _ in 0..1 + rng.below(region as usize) {
                    out.extend_from_slice(b"\x1bM");
                    let len = 8 + rng.below(cols - 7);
                    log_line(rng, out, len);
                    out.push(b'\r');
                }
                canvas::goto(out, 0, region - 1);
            }
            3 => out.extend_from_slice(format!("\x1b[{}S", 1 + rng.below(5)).as_bytes()),
            4 => out.extend_from_slice(format!("\x1b[{}T", 1 + rng.below(5)).as_bytes()),
            _ => {}
        }
        canvas::goto(out, 0, region - 1);
    }
    out.extend_from_slice(b"\x1b[?6l\x1b[r");
}

fn cursor(rng: &mut Rng, out: &mut Vec<u8>, target: usize, size: Size) {
    let (cols, rows) = (size.cols.max(1) as usize, size.rows.max(1) as usize);
    while out.len() < target {
        canvas::goto(out, rng.below(cols) as u16, rng.below(rows) as u16);
        match rng.below(10) {
            0..=4 => canvas::fg(out, random_rgb(rng)),
            5 | 6 => {
                canvas::fg(out, random_rgb(rng));
                canvas::bg(out, random_rgb(rng));
            }
            7 => out.extend_from_slice(b"\x1b[1;7m"),
            8 => out.extend_from_slice(b"\x1b[0m"),
            _ => {}
        }
        let n = 1 + rng.below(12);
        for _ in 0..n {
            out.push(b'a' + rng.below(26) as u8);
        }
        if rng.below(12) == 0 {
            out.extend_from_slice(b"\x1b[K");
        }
    }
    out.extend_from_slice(b"\x1b[0m");
}

fn long_lines(rng: &mut Rng, out: &mut Vec<u8>, target: usize) {
    while out.len() < target {
        let len = 5_000 + rng.below(45_001);
        words(rng, out, len);
        out.extend_from_slice(b"\r\n");
    }
}

/// How many frames a test of `target` bytes sends: enough to measure a rate, not so many that a
/// small window takes forever.
fn frame_count(target: usize, frame_len: usize) -> usize {
    (target / frame_len.max(1)).clamp(30, 600)
}

/// An animated plasma, a whole screen per frame, each in a synchronized update. Returns the frame count.
fn frames_stream(out: &mut Vec<u8>, target: usize, size: Size) -> usize {
    let mut px = Pixels::new();
    px.fit(size.cols.max(1), size.rows.max(1));
    let mut frames = 0;
    let mut wanted = usize::MAX;
    while frames < wanted {
        let t = frames as f64 / 30.0;
        px.shade(|u, v, aspect| {
            let (x, y) = ((u - 0.5) * aspect * 6.0, (v - 0.5) * 6.0);
            let mut s = (x + t).sin() + ((y + t) * 0.5).sin() + ((x + y + t) * 0.5).sin();
            s += (((x * x + y * y + 1.0).sqrt()) * 2.0 - t * 2.0).sin();
            Rgb::cosine(s * 0.25 + t * 0.05, [0.5; 3], [0.5; 3], [1.0, 1.0, 1.0], [0.0, 0.10, 0.20])
        });
        let start = out.len();
        out.extend_from_slice(b"\x1b[?2026h");
        px.render(out);
        out.extend_from_slice(b"\x1b[?2026l");
        frames += 1;
        if frames == 1 {
            wanted = frame_count(target, out.len() - start);
        }
    }
    frames
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: Size = Size { cols: 100, rows: 30, px_w: 0, px_h: 0 };

    fn make(name: &str, target: usize) -> Payload {
        payload(name, SIZE, target).unwrap_or_else(|| Payload { bytes: Vec::new(), frames: 0 })
    }

    #[test]
    fn every_throughput_test_reaches_its_size() {
        for name in NAMES.iter().filter(|n| **n != "latency" && **n != "frames") {
            let p = make(name, 200_000);
            assert!(p.bytes.len() >= 200_000, "{name}: {}", p.bytes.len());
            assert!(p.bytes.len() < 200_000 + 120_000, "{name} overshoots: {}", p.bytes.len());
        }
        assert!(payload("latency", SIZE, 1000).is_none());
        assert!(payload("nope", SIZE, 1000).is_none());
    }

    #[test]
    fn the_same_name_gives_the_same_bytes() {
        for name in NAMES.iter().filter(|n| **n != "latency") {
            assert_eq!(make(name, 50_000).bytes, make(name, 50_000).bytes, "{name}");
        }
        assert_ne!(make("ascii", 50_000).bytes, make("unicode", 50_000).bytes);
    }

    #[test]
    fn text_tests_are_valid_utf8() {
        for name in ["ascii", "unicode", "truecolor", "scroll-region", "cursor", "long-lines", "frames"] {
            assert!(std::str::from_utf8(&make(name, 100_000).bytes).is_ok(), "{name}");
        }
    }

    #[test]
    fn ascii_lines_have_the_requested_lengths() {
        let p = make("ascii", 20_000);
        let text = String::from_utf8_lossy(&p.bytes);
        let lines: Vec<&str> = text.split("\r\n").filter(|l| !l.is_empty()).collect();
        assert!(lines.len() > 100);
        assert!(lines.iter().all(|l| (24..184).contains(&l.len()) && l.is_ascii()));
        let (min, max) = lines.iter().fold((usize::MAX, 0), |(a, b), l| (a.min(l.len()), b.max(l.len())));
        assert!(max - min > 80, "lengths should vary: {min}..{max}");
    }

    #[test]
    fn long_lines_are_long() {
        let p = make("long-lines", 400_000);
        let text = String::from_utf8_lossy(&p.bytes);
        assert!(text.split("\r\n").filter(|l| !l.is_empty()).all(|l| (5_000..=50_000).contains(&l.len())));
    }

    #[test]
    fn unicode_mixes_scripts() {
        let text = String::from_utf8_lossy(&make("unicode", 50_000).bytes).to_string();
        assert!(text.chars().any(|c| ('\u{4e00}'..='\u{9fa5}').contains(&c)));
        assert!(text.chars().any(|c| ('\u{1f600}'..='\u{1f64f}').contains(&c)));
        assert!(text.chars().any(|c| ('\u{2500}'..='\u{257f}').contains(&c)));
        assert!(text.chars().any(|c| ('\u{300}'..='\u{314}').contains(&c)));
    }

    #[test]
    fn scroll_region_sets_and_resets_the_region() {
        let text = String::from_utf8_lossy(&make("scroll-region", 100_000).bytes).to_string();
        assert!(text.contains("\x1b[3;28r"));
        assert!(text.contains("\x1bM"));
        assert!(text.ends_with("\x1b[?6l\x1b[r"));
        // Tiny screens get no header or footer and still a valid region.
        let tiny = payload("scroll-region", Size { cols: 20, rows: 5, px_w: 0, px_h: 0 }, 10_000);
        assert!(tiny.is_some_and(|p| String::from_utf8_lossy(&p.bytes).contains("\x1b[1;5r")));
    }

    #[test]
    fn frames_are_whole_synchronized_screens() {
        let p = make("frames", 1_000_000);
        assert!(p.frames >= 30);
        let text = String::from_utf8_lossy(&p.bytes).to_string();
        assert_eq!(text.matches("\x1b[?2026h").count(), p.frames);
        assert_eq!(text.matches("\x1b[?2026l").count(), p.frames);
        assert_eq!(frame_count(1, 1000), 30);
        assert_eq!(frame_count(usize::MAX / 2, 1000), 600);
    }

    #[test]
    fn tiny_terminals_do_not_panic() {
        for (cols, rows) in [(1, 1), (2, 2), (20, 5)] {
            let size = Size { cols, rows, px_w: 0, px_h: 0 };
            for name in NAMES.iter().filter(|n| **n != "latency") {
                assert!(
                    payload(name, size, 5_000).is_some_and(|p| !p.bytes.is_empty()),
                    "{name} at {cols}x{rows}"
                );
            }
        }
    }
}
