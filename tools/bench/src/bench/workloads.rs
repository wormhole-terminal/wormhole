//! The bytes each benchmark sends. They are built in memory before the clock starts and come from
//! a seeded generator, so every terminal is fed the same stream (the ones that draw to the screen
//! depend on its size, so compare terminals at the same size).
//!
//! Lines end in CR LF: the benchmark runs in raw mode, where a bare LF would not return the carriage.
//!
//! Every payload leaves the terminal as it found it (SGR, scroll region, cursor shape, and the
//! alternate screen entered and left the same number of times), so the next test starts clean.

mod agent;
mod features;
mod realistic;

use crate::canvas::{self, Pixels, Rgb, Rng};
use crate::term::Size;

/// The tests in the order they run: what real programs print, then the throughput stress tests, the
/// feature stress tests, and the two that measure something other than bytes per second.
pub const NAMES: [&str; 17] = [
    "buildlog",
    "agentstream",
    "agentredraw",
    "agentscreen",
    "diffscroll",
    "editstorm",
    "logflood",
    "glyphstorm",
    "colorflood",
    "scrollstorm",
    "cursorstorm",
    "longlines",
    "stylestorm",
    "linkstorm",
    "framerate",
    "screenswap",
    "roundtrip",
];

/// The tests that are what real programs print, as opposed to stress tests.
pub const REALISTIC: [&str; 6] =
    ["buildlog", "agentstream", "agentredraw", "agentscreen", "diffscroll", "editstorm"];

/// The named groups `--only` and `suite` accept: `realistic`, `stress` (everything else but
/// `roundtrip`) and `all`.
pub fn suite(name: &str) -> Option<Vec<&'static str>> {
    let pick = |f: &dyn Fn(&str) -> bool| NAMES.iter().copied().filter(|n| f(n)).collect();
    match name {
        "realistic" => Some(pick(&|n| REALISTIC.contains(&n))),
        "stress" => Some(pick(&|n| !REALISTIC.contains(&n) && n != "roundtrip")),
        "all" => Some(NAMES.to_vec()),
        _ => None,
    }
}

pub fn about(name: &str) -> &'static str {
    match name {
        "buildlog" => "a cargo-style build: colored lines, diagnostics, a progress bar redrawn in place",
        "agentstream" => "an AI agent's answer as 2-16 byte writes, up to 2 MB (per-write cost)",
        "agentredraw" => "a coding agent's classic UI: live region redrawn in place, transcript repainted",
        "agentscreen" => "a coding agent's fullscreen UI: alternate screen, synchronized cell diffs",
        "diffscroll" => "paging through a colored git diff in a pager, a full redraw per page",
        "editstorm" => "an editor at work: scroll region, insert/delete lines and chars, status line",
        "logflood" => "plain log lines, like cat of a big file",
        "glyphstorm" => "CJK, emoji, accents, box drawing",
        "colorflood" => "every character with its own colors",
        "scrollstorm" => "a pinned header and footer, text scrolling between",
        "cursorstorm" => "random cursor moves and short colored writes",
        "longlines" => "lines of 5 to 50 thousand characters",
        "stylestorm" => "every SGR attribute, densely mixed: underline styles, colors, blink, conceal",
        "linkstorm" => "OSC 8 hyperlinks on short spans, thousands of distinct URLs",
        "framerate" => "full-screen truecolor redraws, synchronized",
        "screenswap" => "opening and closing a full-screen program: alternate screen in, paint, out",
        "roundtrip" => "cursor report round trip on an idle terminal",
        _ => "",
    }
}

/// What each test was called up to 0.1 (when ttystorm was termbench), before each became a command.
const OLD_NAMES: [(&str, &str); 8] = [
    ("ascii", "logflood"),
    ("unicode", "glyphstorm"),
    ("truecolor", "colorflood"),
    ("scroll-region", "scrollstorm"),
    ("cursor", "cursorstorm"),
    ("long-lines", "longlines"),
    ("frames", "framerate"),
    ("latency", "roundtrip"),
];

/// A test's name today, given the name a results file saved before the rename uses.
pub fn current_name(name: &str) -> &str {
    OLD_NAMES.iter().find(|(old, _)| *old == name).map_or(name, |(_, new)| new)
}

/// The generator's seed for a test. It hashes the old name, so the bytes didn't change with the
/// rename and results saved before it still compare.
fn seed_of(name: &str) -> u64 {
    let old = OLD_NAMES.iter().find(|(_, new)| *new == name).map_or(name, |(old, _)| old);
    old.bytes().fold(0x9E37_79B9u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64))
}

/// What a throughput test sends, and how many screens that is (0 when it isn't a screen test).
#[derive(Default)]
pub struct Payload {
    pub bytes: Vec<u8>,
    /// Screens drawn: frames for `framerate` and the agent redraws, swaps for `screenswap`.
    pub frames: usize,
    /// The size of each write, when the test is about writes rather than bytes (`agentstream`, and a
    /// frame per write for the agent redraws). They add up to `bytes.len()`. `None`: the bytes go out
    /// in `CHUNK`-sized writes.
    pub chunks: Option<Vec<usize>>,
}

/// How big the writes of an ordinary payload are.
pub const CHUNK: usize = 64 * 1024;

impl Payload {
    /// The writes the payload goes out in, in order.
    pub fn writes(&self) -> Writes<'_> {
        Writes { rest: &self.bytes, sizes: self.chunks.as_ref().map(|c| c.iter()) }
    }

    /// How many writes that is.
    pub fn write_count(&self) -> usize {
        self.chunks.as_ref().map_or(self.bytes.len().div_ceil(CHUNK), Vec::len)
    }
}

pub struct Writes<'a> {
    rest: &'a [u8],
    sizes: Option<std::slice::Iter<'a, usize>>,
}

impl<'a> Iterator for Writes<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        if self.rest.is_empty() {
            return None;
        }
        let want = match &mut self.sizes {
            Some(sizes) => sizes.next().copied().unwrap_or(self.rest.len()),
            None => CHUNK,
        };
        let (head, tail) = self.rest.split_at(want.clamp(1, self.rest.len()));
        self.rest = tail;
        Some(head)
    }
}

/// The most `agentstream` sends: tiny writes are slow, and 2 MB of them is already a long test.
pub const AGENT_MAX_BYTES: usize = 2_000_000;

/// The bytes for the throughput test `name`, a little over `target` bytes. `None` for `roundtrip`
/// (which sends nothing but questions) and for names that don't exist.
pub fn payload(name: &str, size: Size, target: usize) -> Option<Payload> {
    let mut rng = Rng::new(seed_of(name));
    let mut out = Vec::with_capacity(target.min(64 << 20) + 64 * 1024);
    let mut frames = 0;
    let mut chunks = None;
    match name {
        "buildlog" => realistic::buildlog(&mut rng, &mut out, target),
        "agentstream" => {
            chunks = Some(realistic::agentstream(&mut rng, &mut out, target.min(AGENT_MAX_BYTES), size))
        }
        "agentredraw" => {
            let sizes = agent::agentredraw(&mut rng, &mut out, target, size);
            frames = sizes.len();
            chunks = Some(sizes);
        }
        "agentscreen" => {
            let sizes = agent::agentscreen(&mut rng, &mut out, target, size);
            frames = sizes.len();
            chunks = Some(sizes);
        }
        "diffscroll" => realistic::diffscroll(&mut rng, &mut out, target, size),
        "editstorm" => realistic::editstorm(&mut rng, &mut out, target, size),
        "logflood" => ascii(&mut rng, &mut out, target),
        "glyphstorm" => unicode(&mut rng, &mut out, target),
        "colorflood" => truecolor(&mut rng, &mut out, target, size),
        "scrollstorm" => scroll_region(&mut rng, &mut out, target, size),
        "cursorstorm" => cursor(&mut rng, &mut out, target, size),
        "longlines" => long_lines(&mut rng, &mut out, target),
        "stylestorm" => features::stylestorm(&mut rng, &mut out, target),
        "linkstorm" => features::linkstorm(&mut rng, &mut out, target),
        "framerate" => frames = frames_stream(&mut out, target, size),
        "screenswap" => frames = features::screenswap(&mut rng, &mut out, target, size),
        _ => return None,
    }
    Some(Payload { bytes: out, frames, chunks })
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
    "roundtrip",
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
        // The old names, kept so the bytes stay the same and older results still compare.
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
        payload(name, SIZE, target).unwrap_or_default()
    }

    #[test]
    fn every_throughput_test_reaches_its_size() {
        let by_frames = ["roundtrip", "framerate", "screenswap", "agentredraw", "agentscreen"];
        for name in NAMES.iter().filter(|n| !by_frames.contains(n)) {
            let p = make(name, 200_000);
            assert!(p.bytes.len() >= 200_000, "{name}: {}", p.bytes.len());
            assert!(p.bytes.len() < 200_000 + 120_000, "{name} overshoots: {}", p.bytes.len());
        }
        assert!(payload("roundtrip", SIZE, 1000).is_none());
        assert!(payload("nope", SIZE, 1000).is_none());
    }

    #[test]
    fn the_same_name_gives_the_same_bytes() {
        for name in NAMES.iter().filter(|n| **n != "roundtrip") {
            assert_eq!(make(name, 50_000).bytes, make(name, 50_000).bytes, "{name}");
        }
        assert_ne!(make("logflood", 50_000).bytes, make("glyphstorm", 50_000).bytes);
    }

    #[test]
    fn the_rename_kept_the_bytes() {
        let old = "unicode".bytes().fold(0x9E37_79B9u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64));
        assert_eq!(seed_of("glyphstorm"), old);
        assert_eq!(current_name("unicode"), "glyphstorm");
        assert_eq!(current_name("logflood"), "logflood");
        assert!(OLD_NAMES.iter().all(|(_, new)| NAMES.contains(new)));
    }

    #[test]
    fn text_tests_are_valid_utf8() {
        for name in NAMES.iter().filter(|n| **n != "roundtrip") {
            assert!(std::str::from_utf8(&make(name, 100_000).bytes).is_ok(), "{name}");
        }
    }

    #[test]
    fn ascii_lines_have_the_requested_lengths() {
        let p = make("logflood", 20_000);
        let text = String::from_utf8_lossy(&p.bytes);
        let lines: Vec<&str> = text.split("\r\n").filter(|l| !l.is_empty()).collect();
        assert!(lines.len() > 100);
        assert!(lines.iter().all(|l| (24..184).contains(&l.len()) && l.is_ascii()));
        let (min, max) = lines.iter().fold((usize::MAX, 0), |(a, b), l| (a.min(l.len()), b.max(l.len())));
        assert!(max - min > 80, "lengths should vary: {min}..{max}");
    }

    #[test]
    fn long_lines_are_long() {
        let p = make("longlines", 400_000);
        let text = String::from_utf8_lossy(&p.bytes);
        assert!(text.split("\r\n").filter(|l| !l.is_empty()).all(|l| (5_000..=50_000).contains(&l.len())));
    }

    #[test]
    fn unicode_mixes_scripts() {
        let text = String::from_utf8_lossy(&make("glyphstorm", 50_000).bytes).to_string();
        assert!(text.chars().any(|c| ('\u{4e00}'..='\u{9fa5}').contains(&c)));
        assert!(text.chars().any(|c| ('\u{1f600}'..='\u{1f64f}').contains(&c)));
        assert!(text.chars().any(|c| ('\u{2500}'..='\u{257f}').contains(&c)));
        assert!(text.chars().any(|c| ('\u{300}'..='\u{314}').contains(&c)));
    }

    #[test]
    fn scroll_region_sets_and_resets_the_region() {
        let text = String::from_utf8_lossy(&make("scrollstorm", 100_000).bytes).to_string();
        assert!(text.contains("\x1b[3;28r"));
        assert!(text.contains("\x1bM"));
        assert!(text.ends_with("\x1b[?6l\x1b[r"));
        // Tiny screens get no header or footer and still a valid region.
        let tiny = payload("scrollstorm", Size { cols: 20, rows: 5, px_w: 0, px_h: 0 }, 10_000);
        assert!(tiny.is_some_and(|p| String::from_utf8_lossy(&p.bytes).contains("\x1b[1;5r")));
    }

    #[test]
    fn frames_are_whole_synchronized_screens() {
        let p = make("framerate", 1_000_000);
        assert!(p.frames >= 30);
        let text = String::from_utf8_lossy(&p.bytes).to_string();
        assert_eq!(text.matches("\x1b[?2026h").count(), p.frames);
        assert_eq!(text.matches("\x1b[?2026l").count(), p.frames);
        assert_eq!(frame_count(1, 1000), 30);
        assert_eq!(frame_count(usize::MAX / 2, 1000), 600);
    }

    /// FNV-1a of the first 100 KB (at 100×30) of each test that existed in 1.1, taken from that code.
    const OLD_FINGERPRINTS: [(&str, u64); 7] = [
        ("logflood", 0x1e0f_552f_989e_4e7a),
        ("glyphstorm", 0x477d_faa4_bf74_a8fe),
        ("colorflood", 0x540c_8103_2923_e2c4),
        ("scrollstorm", 0xfdbe_60bc_4d92_5513),
        ("cursorstorm", 0x0e20_2966_386a_f263),
        ("longlines", 0xdc56_b7ae_6276_0be7),
        ("framerate", 0x9f8e_41ca_7002_88fe),
    ];

    #[test]
    fn the_old_payloads_are_byte_identical() {
        // Fingerprints of the first 100 KB of every test that existed before the realistic ones, made
        // with the code of 1.1: a change here breaks the comparison with results saved earlier.
        let fingerprint = |name: &str| {
            make(name, 100_000)
                .bytes
                .iter()
                .fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ *b as u64).wrapping_mul(0x100_0000_01b3))
        };
        for (name, expected) in OLD_FINGERPRINTS {
            assert_eq!(fingerprint(name), expected, "{name}");
        }
    }

    #[test]
    fn agentstream_is_capped_and_keeps_its_write_sizes() {
        let p = make("agentstream", 50_000_000);
        assert!((AGENT_MAX_BYTES..AGENT_MAX_BYTES + 1000).contains(&p.bytes.len()), "{}", p.bytes.len());
        let sizes = p.chunks.as_ref().map(|c| c.iter().sum::<usize>());
        assert_eq!(sizes, Some(p.bytes.len()));
        assert!(p.chunks.iter().flatten().all(|n| (2..=16).contains(n)));
        assert_eq!(p.writes().count(), p.write_count());
        assert!(p.writes().all(|w| (2..=16).contains(&w.len())));
        assert!(make("logflood", 1000).chunks.is_none());
    }

    #[test]
    fn screens_are_counted_for_swaps_and_frames() {
        let p = make("screenswap", 1_000_000);
        assert!(p.frames >= 30);
        assert!(make("framerate", 1_000_000).frames >= 30);
        for name in ["agentredraw", "agentscreen"] {
            let p = make(name, 1_000_000);
            assert!(p.frames >= 30, "{name}: {}", p.frames);
            // One write per frame.
            assert_eq!(p.chunks.as_ref().map(Vec::len), Some(p.frames), "{name}");
            assert_eq!(p.write_count(), p.frames, "{name}");
        }
        assert_eq!(make("buildlog", 10_000).frames, 0);
    }

    #[test]
    fn payloads_leave_the_terminal_as_they_found_it() {
        for name in NAMES.iter().filter(|n| **n != "roundtrip") {
            let text = String::from_utf8_lossy(&make(name, 60_000).bytes).to_string();
            assert_eq!(text.matches("\x1b[?1049h").count(), text.matches("\x1b[?1049l").count(), "{name}");
            // Whatever was set last is not left on.
            if let Some(i) = text.rfind("\x1b[0m") {
                assert!(!text[i..].contains("\x1b[1m") && !text[i..].contains("\x1b[7m"), "{name}");
            }
        }
    }

    #[test]
    fn tiny_terminals_do_not_panic() {
        for (cols, rows) in [(1, 1), (2, 2), (20, 5)] {
            let size = Size { cols, rows, px_w: 0, px_h: 0 };
            for name in NAMES.iter().filter(|n| **n != "roundtrip") {
                assert!(
                    payload(name, size, 5_000).is_some_and(|p| !p.bytes.is_empty()),
                    "{name} at {cols}x{rows}"
                );
            }
        }
    }
}
