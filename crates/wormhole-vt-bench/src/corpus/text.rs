//! Plain text: a flood of ASCII, log lines, and text in many scripts.

use super::Out;

/// Words with no line break at all: the terminal wraps every line itself.
pub(super) fn ascii_flood(out: &mut Out, target: usize) {
    while out.len() < target {
        let n = out.rng.range(8, 40);
        let words = out.words(n);
        out.text(&words);
        out.text(" ");
    }
}

const LEVELS: [(&str, &str); 5] =
    [("INFO ", "32"), ("WARN ", "33"), ("ERROR", "1;31"), ("DEBUG", "36"), ("TRACE", "90")];
const MODULES: [&str; 8] = [
    "http::server",
    "db::pool",
    "agent::runner",
    "fs::watch",
    "git::status",
    "pty::reader",
    "ui::render",
    "config::load",
];

/// Server-style log lines: a timestamp, a level (colored one time in three), a module and a
/// message of any length, so some wrap and some do not.
pub(super) fn log_lines(out: &mut Out, target: usize) {
    let mut millis: u64 = 45_296_000;
    while out.len() < target {
        millis += out.rng.range(1, 900) as u64;
        let (h, m, s, ms) =
            (millis / 3_600_000 % 24, millis / 60_000 % 60, millis / 1000 % 60, millis % 1000);
        out.text(&format!("2026-10-15T{h:02}:{m:02}:{s:02}.{ms:03}Z "));
        let (level, color) = *out.rng.pick(&LEVELS);
        if out.rng.one_in(3) {
            out.sgr(color);
            out.text(level);
            out.sgr("0");
        } else {
            out.text(level);
        }
        let module = *out.rng.pick(&MODULES);
        out.text(&format!(" {module}: "));
        let len = out.rng.range(20, 160);
        let message = out.line_of(len);
        out.text(&message);
        if out.rng.one_in(3) {
            let (took, bytes, id) =
                (out.rng.range(1, 900), out.rng.range(100, 99_999), out.rng.next_u64() & 0xffff);
            out.text(&format!(" took={took}ms bytes={bytes} id={id:04x}"));
        }
        out.text("\r\n");
    }
}

const CJK: [&str; 8] = [
    "日本語のテキスト",
    "中文字符测试",
    "한국어 텍스트",
    "全角ＡＢＣ１２３",
    "ｱｲｳｴｵ",
    "漢字とかな",
    "你好，世界",
    "こんにちは",
];
const EMOJI: [&str; 14] = [
    "😀",
    "🎉",
    "🚀",
    "👍🏽",
    "👨\u{200d}👩\u{200d}👧\u{200d}👦",
    "🇯🇵",
    "🇩🇪",
    "🏳\u{fe0f}\u{200d}🌈",
    "❤\u{fe0f}",
    "✔\u{fe0f}",
    "1\u{fe0f}\u{20e3}",
    "🧑\u{200d}💻",
    "🔥",
    "⭐",
];
const MARKED: [&str; 10] =
    ["e\u{301}", "a\u{300}\u{323}", "o\u{308}", "n\u{303}", "é", "ü", "ñ", "ß", "Ω", "жя"];
const DRAWING: [&str; 8] = ["┌──┐", "│  │", "└──┘", "█▓▒░", "═║╔╗╚╝", "→←↑↓⇒", "✓✗•…", "▏▎▍▌▋▊▉█"];

/// Words mixed with wide characters, emoji sequences, combining marks and box drawing, with a
/// line break every 60 to 90 columns.
pub(super) fn unicode_mix(out: &mut Out, target: usize) {
    let mut col = 0;
    let limit = out.rng.range(60, 90);
    let mut limit = limit;
    while out.len() < target {
        let piece: String = match out.rng.below(10) {
            0..=3 => out.words_between(1, 4),
            4 | 5 => (*out.rng.pick(&CJK)).to_string(),
            6 => (*out.rng.pick(&EMOJI)).to_string(),
            7 => (*out.rng.pick(&MARKED)).to_string(),
            8 => (*out.rng.pick(&DRAWING)).to_string(),
            _ => out.words(2),
        };
        out.text(&piece);
        out.text(" ");
        col += piece.chars().count() + 1;
        if col >= limit {
            out.text("\r\n");
            col = 0;
            limit = out.rng.range(60, 90);
        }
    }
}
