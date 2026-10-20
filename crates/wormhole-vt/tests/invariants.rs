//! Properties that must hold for every input, checked on generated streams. Each case has a seed
//! and prints it with the stream that broke it, so a failure reproduces: call the helper (for
//! instance `split_at_every_byte(seed, 1, len, mix)`) with that seed, or copy the printed stream
//! into a `.vtt` case.
//!
//! - **Split invariance**: a stream fed whole, cut at any one byte, or cut into random pieces leaves
//!   the terminal in the same state (`dump_state`, which holds every cell, style, link and mode),
//!   with the same replies and events. A host reads the PTY in whatever pieces the kernel gives it.
//! - **Reflow loses nothing**: resizing there and back, through any sizes, keeps every character of
//!   the history and the screen, soft-wrapped lines rewrapped and never split or merged wrongly.
//! - **Replies are well-formed and carry no program text**: everything the terminal writes back is a
//!   sequence the parser reads as one (no stray controls, no C1), and what a request carried
//!   (a DECRQSS string, a colour query's extra parameters, a title when reports are off) is not in it.
//!
//! The memory bounds are in `invariants_memory.rs`, which needs an allocator of its own.
//!
//! The default run is a few seconds; `cargo test -p wormhole-vt --test invariants -- --ignored`
//! runs ten times the seeds and longer streams.

use wormhole_vt::event::Terminator;
use wormhole_vt::parser::{Csi, Handler, Parser};
use wormhole_vt::{Event, Line, Policy, StateDump, Terminal};

#[path = "common/rng.rs"]
mod rng;
use rng::Rng;

// ---- generated streams ----

const WORDS: &[&str] = &[
    "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india", "juliet", "kilo",
    "lima", "mike", "november", "oscar", "papa", "quebec", "romeo", "sierra", "tango", "uniform",
];

/// Text that is not ASCII: accents, scripts, wide characters, clusters, combining marks, emoji.
const UNICODE: &[&str] = &[
    "é",
    "ß",
    "Привет",
    "日本語",
    "ｱｲｳ",
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
    "\u{200b}",
    "\u{0e01}\u{0e33}",
];

/// How much of the strange the generator may use.
#[derive(Clone, Copy)]
struct Mix {
    /// Bytes that are not valid UTF-8, stray C1 bytes, CAN and SUB in the middle of sequences.
    garbage: bool,
    /// Sequences that ask the terminal something, so replies are made.
    queries: bool,
}

fn number(rng: &mut Rng, max: usize) -> usize {
    // Mostly small, sometimes zero or absurd: the places where clamping matters.
    match rng.below(12) {
        0 => 0,
        1 => 65_535,
        2 => rng.range(0, 3 * max),
        _ => rng.range(1, max),
    }
}

fn sgr(rng: &mut Rng) -> String {
    let one = |rng: &mut Rng| -> String {
        match rng.below(18) {
            0 => String::new(),
            1 => "0".into(),
            2 => rng
                .pick(&[
                    "1", "2", "3", "4", "5", "7", "8", "9", "21", "22", "23", "24", "25", "27", "28", "29",
                    "53", "55",
                ])
                .into(),
            3 => format!("{}", 30 + rng.below(8)),
            4 => format!("{}", 40 + rng.below(8)),
            5 => format!("{}", 90 + rng.below(8)),
            6 => format!("{}", 100 + rng.below(8)),
            7 => "39".into(),
            8 => "49".into(),
            9 => format!("38;5;{}", rng.below(300)),
            10 => format!("48;5;{}", rng.below(256)),
            11 => format!("38;2;{};{};{}", rng.below(256), rng.below(256), rng.below(256)),
            12 => format!("48:2::{}:{}:{}", rng.below(256), rng.below(256), rng.below(256)),
            13 => format!("58:5:{}", rng.below(256)),
            14 => format!("4:{}", rng.below(7)),
            15 => "59".into(),
            16 => format!("{}", rng.below(110)),
            _ => "38:2:0:1:2:3".into(),
        }
    };
    let n = rng.range(1, 4);
    let parts: Vec<String> = (0..n).map(|_| one(rng)).collect();
    format!("\x1b[{}m", parts.join(";"))
}

/// One piece of a stream: text, controls or a sequence.
fn piece(rng: &mut Rng, out: &mut Vec<u8>, cols: usize, rows: usize, mix: Mix) {
    let put = |out: &mut Vec<u8>, s: &str| out.extend_from_slice(s.as_bytes());
    match rng.below(34) {
        0..=6 => {
            for _ in 0..rng.range(1, 6) {
                put(out, rng.pick(WORDS));
                put(out, " ");
            }
        }
        7 | 8 => put(out, rng.pick(UNICODE)),
        9 => put(out, rng.pick(&["\r\n", "\n", "\r", "\t", "\x08", "\x0b", "\x0c", "\x0e", "\x0f"])),
        10 | 11 => put(out, &sgr(rng)),
        12 => put(out, &format!("\x1b[{};{}H", number(rng, rows), number(rng, cols))),
        13 => put(
            out,
            &format!(
                "\x1b[{}{}",
                number(rng, cols),
                rng.pick(&["A", "B", "C", "D", "E", "F", "G", "d", "`", "a", "e", "I", "Z"])
            ),
        ),
        14 => put(out, &format!("\x1b[{}{}", rng.below(4), rng.pick(&["J", "K", "?J", "?K"]))),
        15 => put(
            out,
            &format!("\x1b[{}{}", number(rng, 6), rng.pick(&["L", "M", "@", "P", "X", "S", "T", "b"])),
        ),
        16 => {
            if rng.chance(70) {
                put(out, &format!("\x1b[{};{}r", number(rng, rows), number(rng, rows)));
            } else {
                put(out, "\x1b[r");
            }
        }
        17 => {
            if rng.chance(50) {
                put(out, &format!("\x1b[{};{}s", number(rng, cols), number(rng, cols)));
            } else {
                put(out, rng.pick(&["\x1b[?69h", "\x1b[?69l", "\x1b[s"]));
            }
        }
        18 => {
            let uri = format!("https://example.com/{}", rng.pick(WORDS));
            let id = if rng.chance(50) { format!("id={};", rng.below(5)) } else { String::new() };
            let end = rng.pick(&["\x07", "\x1b\\"]);
            put(out, &format!("\x1b]8;{id};{uri}{end}"));
        }
        19 => put(out, rng.pick(&["\x1b]8;;\x07", "\x1b]8;;\x1b\\"])),
        20 => put(
            out,
            rng.pick(&[
                "\x1b[?1049h",
                "\x1b[?1049l",
                "\x1b[?47h",
                "\x1b[?47l",
                "\x1b[?1047h",
                "\x1b[?1047l",
                "\x1b[?1048h",
                "\x1b[?1048l",
            ]),
        ),
        21 => put(
            out,
            rng.pick(&[
                "\x1b[?7h",
                "\x1b[?7l",
                "\x1b[?6h",
                "\x1b[?6l",
                "\x1b[?25l",
                "\x1b[?25h",
                "\x1b[?2027h",
                "\x1b[?2027l",
                "\x1b[4h",
                "\x1b[4l",
                "\x1b[?45h",
                "\x1b[?45l",
                "\x1b[?1h",
                "\x1b[?1000h",
                "\x1b[?1006h",
                "\x1b[?2004h",
                "\x1b[20h",
                "\x1b[?5h",
                "\x1b[?5l",
                "\x1b[?80h",
            ]),
        ),
        22 => put(
            out,
            rng.pick(&[
                "\x1b7", "\x1b8", "\x1bM", "\x1bD", "\x1bE", "\x1bH", "\x1b[g", "\x1b[3g", "\x1b=", "\x1b>",
                "\x1b#8",
            ]),
        ),
        23 => {
            let text = format!("{} {}", rng.pick(WORDS), rng.pick(WORDS));
            put(out, &format!("\x1b]{};{text}{}", rng.pick(&["0", "1", "2"]), rng.pick(&["\x07", "\x1b\\"])));
        }
        24 => put(
            out,
            rng.pick(&[
                "\x1b]7;file://host/home/user\x07",
                "\x1b]133;A\x07",
                "\x1b]133;B\x07",
                "\x1b]133;C\x07",
                "\x1b]133;D;0\x07",
                "\x1b]9;note\x07",
                "\x1b]9;4;1;50\x07",
                "\x1b]777;notify;t;b\x07",
                "\x1b]4;1;rgb:ff/00/00\x07",
                "\x1b]10;#ffffff\x07",
                "\x1b]104\x07",
                "\x1b]52;c;aGVsbG8=\x07",
                "\x1b]0;\x07",
            ]),
        ),
        25 => put(
            out,
            rng.pick(&["\x1b(0", "\x1b(B", "\x1b)0", "\x0e", "\x0f", "\x1bn", "\x1bo", "\x1b*0", "\x1b%G"]),
        ),
        26 => put(out, &format!("\x1b[{}b", number(rng, 20))),
        27 => put(
            out,
            rng.pick(&[
                "\x1b[?1004h",
                "\x1b[>1u",
                "\x1b[<1u",
                "\x1b[=3;1u",
                "\x1b[>4;1m",
                "\x1b[22;0t",
                "\x1b[23;0t",
                "\x1b[ q",
                "\x1b[3 q",
            ]),
        ),
        28 => {
            // Rectangle operations and the like.
            let (a, b, c, d) = (number(rng, rows), number(rng, cols), number(rng, rows), number(rng, cols));
            let op = rng.pick(&["$z", "$x", "$v", "${", "$r", "$t"]);
            let args = if op == "$x" {
                format!("{};{};{};{};{}", 65 + rng.below(20), a, b, c, d)
            } else {
                format!("{a};{b};{c};{d}")
            };
            put(out, &format!("\x1b[{args}{op}"));
        }
        29 => put(
            out,
            rng.pick(&[
                "\x1bPq#0;2;0;0;0#1;2;100;0;0#1~~@@vv@@~~$#0!10~-\x1b\\",
                "\x1b_Ga=q,i=1,s=1,v=1,f=24;AAAA\x1b\\",
                "\x1bP$qm\x1b\\",
                "\x1bP1000p\x1b\\",
                "\x1bP+q544e\x1b\\",
                "\x1bPzzz\x1b\\",
            ]),
        ),
        30 => {
            // A control string cut short by the next sequence.
            put(
                out,
                rng.pick(&["\x1b]0;unfinished", "\x1bPqabc", "\x1b_Gx", "\x1b[1;2", "\x1b[?", "\x1b^pm"]),
            );
        }
        31 if mix.queries => put(
            out,
            rng.pick(&[
                "\x1b[6n",
                "\x1b[5n",
                "\x1b[?6n",
                "\x1b[c",
                "\x1b[>c",
                "\x1b[=c",
                "\x1b[?u",
                "\x1b[>q",
                "\x1b[18t",
                "\x1b[14t",
                "\x1b[21t",
                "\x1b[20t",
                "\x1b[?1$p",
                "\x1b[4$p",
                "\x1b]4;1;?\x07",
                "\x1b]10;?\x07",
                "\x1b]11;?\x1b\\",
                "\x1b[1;1;2;3;4*y",
                "\x1bP$q\"q\x1b\\",
                "\x1bP$qr\x1b\\",
                "\x1bP$q q\x1b\\",
                "\x1b[?996n",
                "\x1bZ",
                "\x05",
            ]),
        ),
        32 if mix.garbage => match rng.below(6) {
            0 => out.push(rng.pick(&[
                0x80, 0x85, 0x8f, 0x90, 0x98, 0x9b, 0x9d, 0x9e, 0x9f, 0xc0, 0xc2, 0xe2, 0xf0, 0xf4, 0xff,
            ])),
            1 => out.push(rng.pick(&[0x18, 0x1a, 0x00, 0x7f, 0x1b])),
            2 => out.extend_from_slice(&[0xe2, 0x82]),
            3 => out.extend_from_slice(&[0xf0, 0x9f, 0x98]),
            4 => out.extend_from_slice(&[0xed, 0xa0, 0x80]),
            _ => {
                for _ in 0..rng.range(1, 8) {
                    out.push(rng.below(256) as u8);
                }
            }
        },
        _ => put(out, rng.pick(WORDS)),
    }
}

fn stream(seed: u64, len: usize, cols: usize, rows: usize, mix: Mix) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::with_capacity(len + 64);
    while out.len() < len {
        piece(&mut rng, &mut out, cols, rows, mix);
    }
    out
}

/// A stream for failure messages: printable text as itself, the rest as `\e`, `\xNN`.
fn show(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &b in bytes {
        match b {
            0x1b => out.push_str("\\e"),
            b'\\' => out.push_str("\\\\"),
            b'\r' => out.push_str("\\r"),
            b'\n' => out.push_str("\\n"),
            0x20..=0x7e => out.push(b as char),
            _ => out.push_str(&format!("\\x{b:02x}")),
        }
    }
    out
}

// ---- split invariance ----

/// Everything a host can see of a terminal after some input.
struct Observed {
    state: StateDump,
    replies: Vec<u8>,
    events: Vec<Event>,
}

/// `dump_state`'s line for the parser (`Parser { state: Utf8 pending: [226, 130, 0], pending_len: 2,
/// string_utf8: 0 }`) with the dead bytes of `pending` left out.
fn live_parser(dump: &str) -> String {
    let Some(at) = dump.find(" pending: [") else { return dump.to_string() };
    let head = &dump[..at];
    let list = dump[at + " pending: [".len()..].split(']').next().unwrap_or("");
    let len = dump
        .split("pending_len: ")
        .nth(1)
        .and_then(|rest| rest.split(',').next())
        .and_then(|n| n.trim().parse::<usize>().ok())
        .unwrap_or(0);
    // Out of the UTF-8 state nothing of the buffer is read.
    let live = if head.contains("Utf8") { len } else { 0 };
    let bytes: Vec<&str> = list.split(", ").take(live).collect();
    let tail = dump.split("string_utf8: ").nth(1).unwrap_or("");
    format!("{head} pending {bytes:?} string_utf8 {}", tail.trim_end_matches([' ', '}']))
}

fn observe(mut term: Terminal) -> Observed {
    let mut state = term.dump_state();
    // The parser's buffer for a half-read character keeps old bytes: after the character is
    // finished or thrown away, and past `pending_len`. They are dead values, like a closed
    // cluster's fields in `dump_state`; only the live part is compared.
    for (name, value) in &mut state.fields {
        if name == "a/parser" {
            *value = live_parser(value);
        }
    }
    let replies = term.take_replies();
    let events = term.events().collect();
    Observed { state, replies, events }
}

fn terminal(cols: usize, rows: usize) -> Terminal {
    // The windowing and checksum answers are the most to compare; the policy changes none of the parsing.
    let mut policy = Policy::default();
    policy.window_ops = true;
    policy.screen_checksum = true;
    Terminal::builder().size(cols, rows).scrollback(200).policy(policy).build()
}

fn assert_same(seed: u64, bytes: &[u8], pieces: &str, whole: &Observed, split: &Observed) {
    let state = if whole.state == split.state {
        String::new()
    } else {
        format!("state differs:\n{}", whole.state.diff(&split.state, 4))
    };
    let replies = if whole.replies == split.replies {
        String::new()
    } else {
        format!("replies differ:\n  whole: {}\n  split: {}\n", show(&whole.replies), show(&split.replies))
    };
    let events = if whole.events == split.events {
        String::new()
    } else {
        format!("events differ:\n  whole: {:?}\n  split: {:?}\n", whole.events, split.events)
    };
    assert!(
        state.is_empty() && replies.is_empty() && events.is_empty(),
        "seed {seed}, fed {pieces}\n{state}{replies}{events}stream ({} bytes): {}",
        bytes.len(),
        show(&bytes[..bytes.len().min(700)])
    );
}

/// Seeds `from..from + count` of a size, fed whole and cut at every byte.
fn split_at_every_byte(from: u64, count: u64, len: usize, mix: Mix) {
    for seed in from..from + count {
        let (cols, rows) = (8 + (seed as usize % 3) * 9, 4 + (seed as usize % 4) * 3);
        let bytes = stream(seed, len, cols, rows, mix);
        let mut whole = terminal(cols, rows);
        whole.feed(&bytes);
        let whole = observe(whole);
        for cut in 1..bytes.len() {
            let mut split = terminal(cols, rows);
            split.feed(&bytes[..cut]);
            split.feed(&bytes[cut..]);
            assert_same(seed, &bytes, &format!("in two at byte {cut}"), &whole, &observe(split));
        }
    }
}

/// Seeds fed whole and in random pieces, one byte at a time among them.
fn split_in_random_pieces(from: u64, count: u64, len: usize, mix: Mix) {
    for seed in from..from + count {
        let (cols, rows) = (10 + (seed as usize % 5) * 13, 5 + (seed as usize % 3) * 8);
        let bytes = stream(seed, len, cols, rows, mix);
        let mut whole = terminal(cols, rows);
        whole.feed(&bytes);
        let whole = observe(whole);
        for round in 0..4u64 {
            let mut rng = Rng::new(seed * 31 + round);
            let max = [1, 3, 17, 300][round as usize];
            let mut split = terminal(cols, rows);
            let (mut at, mut sizes) = (0, Vec::new());
            while at < bytes.len() {
                let n = rng.range(1, max).min(bytes.len() - at);
                split.feed(&bytes[at..at + n]);
                if sizes.len() < 24 {
                    sizes.push(n);
                }
                at += n;
            }
            assert_same(
                seed,
                &bytes,
                &format!("in pieces of 1..={max} (first sizes {sizes:?})"),
                &whole,
                &observe(split),
            );
        }
    }
}

const TEXTY: Mix = Mix { garbage: false, queries: true };
const HOSTILE: Mix = Mix { garbage: true, queries: true };

#[test]
fn feeding_in_two_at_any_byte_is_feeding_whole() {
    split_at_every_byte(1, 60, 400, TEXTY);
}

#[test]
fn feeding_in_two_at_any_byte_is_feeding_whole_with_garbage() {
    split_at_every_byte(1_000, 60, 400, HOSTILE);
}

#[test]
fn feeding_in_random_pieces_is_feeding_whole() {
    split_in_random_pieces(2_000, 120, 6_000, TEXTY);
}

#[test]
fn feeding_in_random_pieces_is_feeding_whole_with_garbage() {
    split_in_random_pieces(3_000, 120, 6_000, HOSTILE);
}

/// A wide cluster, a combining mark or a link cut in the middle is the case worth the most; this
/// one cuts only there.
#[test]
fn feeding_a_stream_of_clusters_and_links_one_byte_at_a_time_is_feeding_whole() {
    for seed in 0..30u64 {
        let mut rng = Rng::new(seed);
        let mut bytes = b"\x1b[?2027h".to_vec();
        for _ in 0..40 {
            match rng.below(4) {
                0 => bytes.extend_from_slice(rng.pick(UNICODE).as_bytes()),
                1 => bytes
                    .extend_from_slice(format!("\x1b]8;;https://e.com/{}\x1b\\", rng.below(9)).as_bytes()),
                2 => bytes.extend_from_slice(b"\x1b]8;;\x07"),
                _ => bytes.extend_from_slice(rng.pick(WORDS).as_bytes()),
            }
        }
        let mut whole = terminal(11, 5);
        whole.feed(&bytes);
        let whole = observe(whole);
        let mut split = terminal(11, 5);
        for b in &bytes {
            split.feed(std::slice::from_ref(b));
        }
        assert_same(seed, &bytes, "byte by byte", &whole, &observe(split));
    }
}

#[test]
#[ignore = "ten times the seeds and longer streams: cargo test -p wormhole-vt --test invariants -- --ignored"]
fn split_invariance_many_seeds() {
    split_at_every_byte(10_000, 80, 600, TEXTY);
    split_at_every_byte(20_000, 80, 600, HOSTILE);
    split_in_random_pieces(30_000, 250, 20_000, HOSTILE);
}

// ---- reflow ----

/// The text of a terminal as logical lines: soft-wrapped rows joined, trailing blank lines left out.
fn logical_lines(term: &Terminal, rows: usize) -> Vec<String> {
    let history = term.grid().history_len() as i32;
    let mut lines = Vec::new();
    let mut current = String::new();
    for y in -history..rows as i32 {
        current.push_str(&term.row_text(Line(y), false));
        if !term.is_wrapped(Line(y)) {
            lines.push(current.trim_end().to_string());
            current.clear();
        }
    }
    if !current.is_empty() {
        lines.push(current.trim_end().to_string());
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

/// Equal logical lines, or a message that names the first line that differs.
fn assert_lines(got: &[String], expected: &[String], what: String) {
    if got == expected {
        return;
    }
    let at = got.iter().zip(expected).position(|(a, b)| a != b).unwrap_or(got.len().min(expected.len()));
    panic!(
        "{what}: {} lines, expected {}; first difference at line {at}\n  got:      {:?}\n  expected: {:?}",
        got.len(),
        expected.len(),
        got.get(at),
        expected.get(at)
    );
}

fn reflow_text(seed: u64, wide: bool) -> (Vec<String>, Vec<u8>) {
    let mut rng = Rng::new(seed);
    let (mut lines, mut bytes) = (Vec::new(), Vec::new());
    for n in 0..rng.range(30, 90) {
        let mut line = format!("{n}:");
        for _ in 0..rng.range(0, 40) {
            line.push(' ');
            // Words only, or words and the odd wide character or combining mark.
            if wide && rng.chance(20) {
                line.push_str(rng.pick(&["日本語", "😀", "é", "e\u{301}", "한국", "ｱ"]));
            } else {
                line.push_str(rng.pick(WORDS));
            }
        }
        bytes.extend_from_slice(line.as_bytes());
        bytes.extend_from_slice(b"\r\n");
        lines.push(line);
        if rng.chance(8) {
            bytes.extend_from_slice(b"\r\n");
            lines.push(String::new());
        }
    }
    // Blank lines at the end are not text: `logical_lines` leaves them out too.
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    (lines, bytes)
}

fn reflow_round_trips(from: u64, count: u64) {
    for seed in from..from + count {
        let mut rng = Rng::new(seed ^ 0x5eed);
        let (cols, rows) = (rng.range(20, 110), rng.range(5, 30));
        let (expected, bytes) = reflow_text(seed, false);
        let mut term = Terminal::builder().size(cols, rows).scrollback(100_000).build();
        term.feed(&bytes);
        assert_lines(&logical_lines(&term, rows), &expected, format!("seed {seed}: before any resize"));
        let mut size = (cols, rows);
        for step in 0..8 {
            size = (rng.range(2, 140), rng.range(2, 40));
            term.resize(size.0, size.1);
            assert_lines(
                &logical_lines(&term, size.1),
                &expected,
                format!("seed {seed}: step {step}, resized to {}x{} from {cols}x{rows}", size.0, size.1),
            );
        }
        term.resize(cols, rows);
        assert_lines(
            &logical_lines(&term, rows),
            &expected,
            format!("seed {seed}: back at {cols}x{rows} after {size:?}"),
        );
    }
}

#[test]
fn resizing_through_any_sizes_and_back_keeps_every_line() {
    reflow_round_trips(0, 150);
}

/// Narrower and wider by one column at a time, which moves every wrap point past every word.
#[test]
fn shrinking_one_column_at_a_time_and_growing_back_keeps_every_line() {
    for seed in 0..6u64 {
        let (expected, bytes) = reflow_text(seed + 500, false);
        let mut term = Terminal::builder().size(60, 12).scrollback(100_000).build();
        term.feed(&bytes);
        for width in (3..60).rev().chain(4..=60) {
            term.resize(width, 12);
            assert_lines(&logical_lines(&term, 12), &expected, format!("seed {seed}: at width {width}"));
        }
    }
}

/// Wide characters can leave a blank cell at the end of a row, and a character that does not fit a
/// narrow screen cannot stay; the letters and marks must all still be there, in order.
#[test]
fn resizing_text_with_wide_characters_keeps_every_character() {
    let letters =
        |lines: &[String]| -> String { lines.concat().chars().filter(|c| !c.is_whitespace()).collect() };
    for seed in 0..40u64 {
        let mut rng = Rng::new(seed);
        let (expected, bytes) = reflow_text(seed + 900, true);
        let mut term = Terminal::builder().size(50, 10).scrollback(100_000).build();
        term.feed(&bytes);
        let want = letters(&expected);
        assert_eq!(letters(&logical_lines(&term, 10)), want, "seed {seed}: before any resize");
        for step in 0..6 {
            let width = rng.range(3, 90);
            term.resize(width, 10);
            assert_eq!(letters(&logical_lines(&term, 10)), want, "seed {seed}: step {step}, width {width}");
        }
        term.resize(50, 10);
        assert_eq!(letters(&logical_lines(&term, 10)), want, "seed {seed}: back at 50");
    }
}

#[test]
#[ignore = "many more seeds: cargo test -p wormhole-vt --test invariants -- --ignored"]
fn reflow_many_seeds() {
    reflow_round_trips(1_000, 1_000);
}

// ---- replies ----

/// What is wrong with a reply, found by reading it the way a program does: nothing is allowed in it
/// but whole sequences made of printable text. Empty when it is as it should be.
#[derive(Default)]
struct Strict {
    faults: Vec<String>,
    sequences: usize,
}

impl Strict {
    fn text(&mut self, what: &str, data: &[u8]) {
        for c in String::from_utf8_lossy(data).chars() {
            if c.is_control() {
                self.faults.push(format!("{what} holds {c:?}"));
            }
        }
    }
}

impl Handler for Strict {
    fn print_ascii(&mut self, run: &[u8]) {
        self.faults.push(format!("text outside a sequence: {}", show(run)));
    }
    fn print(&mut self, c: char) {
        self.faults.push(format!("character outside a sequence: {c:?}"));
    }
    fn execute(&mut self, byte: u8) {
        self.faults.push(format!("control {byte:#04x} outside a sequence"));
    }
    fn csi(&mut self, csi: &Csi) {
        self.sequences += 1;
        if csi.overflow() {
            self.faults.push("a sequence with too many parameters".into());
        }
    }
    fn esc(&mut self, intermediates: &[u8], byte: u8) {
        self.faults.push(format!("a bare ESC sequence {} {}", show(intermediates), show(&[byte])));
    }
    fn osc_put(&mut self, data: &[u8]) {
        self.text("an OSC string", data);
    }
    fn osc_end(&mut self, _bel_terminated: bool) {
        self.sequences += 1;
    }
    fn dcs_hook(&mut self, _dcs: &Csi) {
        self.sequences += 1;
    }
    fn dcs_put(&mut self, data: &[u8]) {
        self.text("a DCS string", data);
    }
    fn apc_put(&mut self, data: &[u8]) {
        self.text("an APC string", data);
    }
    fn osc_cancel(&mut self) {
        self.faults.push("a cancelled OSC".into());
    }
    fn dcs_cancel(&mut self) {
        self.faults.push("a cancelled DCS".into());
    }
    fn apc_cancel(&mut self) {
        self.faults.push("a cancelled APC".into());
    }
}

/// Panics when `reply` is not made of whole, plain sequences (it may be empty).
fn assert_well_formed(reply: &[u8], request: &str) {
    let mut strict = Strict::default();
    let mut parser = Parser::new();
    parser.advance(&mut strict, reply);
    if !parser.is_ground() {
        strict.faults.push("ends inside a sequence".into());
    }
    // Raw bytes too: the parser drops what it ignores (DEL, C1 bytes inside a string), and an
    // 8-bit terminal reads those as controls.
    match std::str::from_utf8(reply) {
        Ok(text) => {
            if text.chars().any(|c| c.is_control() && !matches!(c, '\x1b' | '\x07')) {
                strict.faults.push("a control character (C0 other than ESC and BEL, DEL or C1)".into());
            }
        }
        Err(_) => strict.faults.push("not valid UTF-8".into()),
    }
    assert!(strict.faults.is_empty(), "reply {} to {request}: {:?}", show(reply), strict.faults);
}

/// Text made to hurt: C1 and C0 controls as characters, quotes, separators and a marker that must
/// never come back.
fn hostile_text(rng: &mut Rng, len: usize) -> String {
    const MARK: &str = "ZZTOPZZ";
    let mut out = String::new();
    for _ in 0..len {
        match rng.below(8) {
            0 => out.push(char::from_u32(0x80 + rng.below(0x20) as u32).unwrap_or('?')),
            1 => out.push(rng.pick(&['"', '\\', ';', ':', '?', '$', '\'', '`', '%', ' ', '!', '/'])),
            2 => out.push_str(MARK),
            3 => out.push(rng.pick(&['\u{7f}', '\u{2028}', '\u{202e}', '\u{200b}', '\u{feff}', 'é', '日'])),
            _ => out.push(rng.pick(&['a', 'm', 'r', 'q', 's', '0', '1', '2', 'x', 'Z'])),
        }
    }
    out
}

const MARK: &[u8] = b"ZZTOPZZ";

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Requests built from hostile text, each fed to a terminal that has been told what the host
/// allows; the replies must be well-formed, and (for those that must not repeat the request) free of
/// the marker.
fn hostile_requests(from: u64, count: u64) {
    for seed in from..from + count {
        let mut rng = Rng::new(seed);
        let mut open = Policy::default();
        open.window_ops = true;
        open.screen_checksum = true;
        for (policy, reports_titles) in [(Policy::default(), false), (open, true)] {
            let mut term = Terminal::builder().size(30, 8).policy(policy).build();
            for _ in 0..30 {
                let text = {
                    let n = rng.range(1, 40);
                    hostile_text(&mut rng, n)
                };
                // (the request, whether the marker may come back in the reply)
                let (request, may_repeat): (String, bool) = match rng.below(9) {
                    0 => (
                        format!(
                            "\x1b]{};{text}\x1b\\\x1b[2{}t",
                            rng.pick(&["0", "1", "2"]),
                            rng.pick(&["0", "1"])
                        ),
                        reports_titles,
                    ),
                    1 => (format!("\x1bP$q{text}\x1b\\"), false),
                    2 => (
                        format!("\x1b]{};?;{text}\x07", rng.pick(&["4;1", "10", "11", "12", "4;255"])),
                        false,
                    ),
                    3 => (format!("\x1b]4;{text};?\x1b\\"), false),
                    4 => (format!("\x1bP+q{}\x1b\\", hex(text.as_bytes())), true),
                    5 => (format!("\x1b_Ga=q,i={},s=1,v=1,f=24;{text}\x1b\\", rng.below(100)), false),
                    6 => (
                        format!(
                            "\x1b[?{text}$p\x1b[{text}$p\x1b[{};{};{};{};{}*y",
                            rng.below(9),
                            rng.below(3),
                            rng.below(9),
                            rng.below(9),
                            rng.below(9)
                        ),
                        false,
                    ),
                    // The title is set with its hex form: the characters it decodes to can be anything.
                    7 => {
                        let raw: Vec<u8> = (0..rng.range(1, 30))
                            .map(|_| {
                                rng.pick(&[0x1b, 0x0d, 0x0a, 0x9b, 0x07, 0x00, 0xc2, 0x85, b'a', b'\\', b'm'])
                            })
                            .collect();
                        (format!("\x1b[>0t\x1b]2;{}\x1b\\\x1b[21t\x1b[>2t", hex(&raw)), reports_titles)
                    }
                    _ => (
                        format!("\x1b[>0;1t\x1b]2;{}\x1b\\\x1b[21t\x1b[20t\x1b[>2;3t", hex(text.as_bytes())),
                        true,
                    ),
                };
                term.feed(request.as_bytes());
                let reply = term.take_replies();
                assert_well_formed(&reply, &format!("seed {seed}: {}", show(request.as_bytes())));
                if !may_repeat {
                    assert!(
                        !contains(&reply, MARK) && !contains(&reply, hex(MARK).as_bytes()),
                        "seed {seed}: the request {} came back in {}",
                        show(request.as_bytes()),
                        show(&reply)
                    );
                }
                if !reports_titles && (request.contains("[21t") || request.contains("[20t")) {
                    assert!(
                        reply.is_empty(),
                        "title reports are off but the reply to {} is {}",
                        show(request.as_bytes()),
                        show(&reply)
                    );
                }
                let _ = term.events().count();
            }
        }
    }
}

#[test]
fn replies_to_hostile_requests_are_well_formed_and_carry_no_request_text() {
    hostile_requests(0, 200);
}

/// The clipboard answer is the host's, but it repeats the selection name the program sent.
#[test]
fn a_clipboard_query_with_a_hostile_selection_is_answered_in_one_piece() {
    for seed in 0..60u64 {
        let mut rng = Rng::new(seed);
        let mut term = Terminal::new(20, 5);
        let name = {
            let n = rng.range(0, 10);
            hostile_text(&mut rng, n)
        };
        term.feed(format!("\x1b]52;{name};?\x1b\\").as_bytes());
        let loads: Vec<(String, Terminator)> = term
            .events()
            .filter_map(|event| match event {
                Event::ClipboardLoad { selection, terminator, .. } => Some((selection, terminator)),
                _ => None,
            })
            .collect();
        for (selection, terminator) in loads {
            term.clipboard_reply(&selection, "text from the clipboard", terminator);
            let reply = term.take_replies();
            assert_well_formed(&reply, &format!("seed {seed}: OSC 52 with selection {name:?}"));
        }
    }
}

/// Whatever generated stream is fed, whatever the terminal answers is well-formed.
fn every_reply_is_well_formed(from: u64, count: u64, len: usize) {
    for seed in from..from + count {
        let (cols, rows) = (6 + (seed as usize % 7) * 11, 3 + (seed as usize % 5) * 5);
        let bytes = stream(seed, len, cols, rows, HOSTILE);
        let mut term = terminal(cols, rows);
        for (n, piece) in bytes.chunks(64).enumerate() {
            term.feed(piece);
            let reply = term.take_replies();
            assert_well_formed(&reply, &format!("seed {seed}, piece {n}: {}", show(piece)));
            let _ = term.events().count();
        }
    }
}

#[test]
fn every_reply_to_a_generated_stream_is_well_formed() {
    every_reply_is_well_formed(0, 200, 8_000);
}

#[test]
#[ignore = "many more seeds: cargo test -p wormhole-vt --test invariants -- --ignored"]
fn replies_many_seeds() {
    hostile_requests(1_000, 1_500);
    every_reply_is_well_formed(10_000, 1_500, 20_000);
}
