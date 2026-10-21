//! The generated streams: text, controls, sequences and, when the mix asks for it, garbage, from a
//! seed, so that a failure is reproduced by its seed.

use wormhole_vt::{Policy, Terminal};

use crate::rng::Rng;

pub(crate) const WORDS: &[&str] = &[
    "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india", "juliet", "kilo",
    "lima", "mike", "november", "oscar", "papa", "quebec", "romeo", "sierra", "tango", "uniform",
];

/// Text that is not ASCII: accents, scripts, wide characters, clusters, combining marks, emoji.
pub(crate) const UNICODE: &[&str] = &[
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
pub(crate) struct Mix {
    /// Bytes that are not valid UTF-8, stray C1 bytes, CAN and SUB in the middle of sequences.
    pub(crate) garbage: bool,
    /// Sequences that ask the terminal something, so replies are made.
    pub(crate) queries: bool,
}

pub(crate) fn number(rng: &mut Rng, max: usize) -> usize {
    // Mostly small, sometimes zero or absurd: the places where clamping matters.
    match rng.below(12) {
        0 => 0,
        1 => 65_535,
        2 => rng.range(0, 3 * max),
        _ => rng.range(1, max),
    }
}

pub(crate) fn sgr(rng: &mut Rng) -> String {
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
pub(crate) fn piece(rng: &mut Rng, out: &mut Vec<u8>, cols: usize, rows: usize, mix: Mix) {
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

pub(crate) fn stream(seed: u64, len: usize, cols: usize, rows: usize, mix: Mix) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::with_capacity(len + 64);
    while out.len() < len {
        piece(&mut rng, &mut out, cols, rows, mix);
    }
    out
}

/// A stream for failure messages: printable text as itself, the rest as `\e`, `\xNN`.
pub(crate) fn show(bytes: &[u8]) -> String {
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

pub(crate) const TEXTY: Mix = Mix { garbage: false, queries: true };
pub(crate) const HOSTILE: Mix = Mix { garbage: true, queries: true };

/// The terminal a stream is fed to.
pub(crate) fn terminal(cols: usize, rows: usize) -> Terminal {
    // The windowing and checksum answers are the most to compare; the policy changes none of the parsing.
    let mut policy = Policy::default();
    policy.window_ops = true;
    policy.screen_checksum = true;
    Terminal::builder().size(cols, rows).scrollback(200).policy(policy).build()
}
