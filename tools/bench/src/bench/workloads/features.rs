//! The tests that exercise one feature each: text attributes, hyperlinks and the alternate screen.
//! A terminal that doesn't draw a style still has to parse it, so the numbers stay comparable.

use super::realistic::{CODE, highlight};
use super::{canvas, frame_count, random_rgb, words};
use crate::canvas::Rng;
use crate::term::Size;

fn push_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(s.as_bytes());
}

fn rgb_text(rng: &mut Rng, sep: &str) -> String {
    let c = random_rgb(rng);
    format!("{}{sep}{}{sep}{}", c.0, c.1, c.2)
}

/// One SGR parameter (or a few, joined): an attribute, a color or a reset.
fn sgr_param(rng: &mut Rng) -> String {
    const ATTRS: [&str; 19] = [
        "1", "2", "3", "4", "4:0", "4:1", "4:2", "4:3", "4:4", "4:5", "21", "9", "53", "7", "5", "6", "8",
        "1", "3",
    ];
    const RESETS: [&str; 12] = ["", "0", "22", "23", "24", "29", "55", "27", "25", "28", "39", "49"];
    match rng.below(100) {
        0..=39 => ATTRS[rng.below(ATTRS.len())].to_string(),
        40..=51 => format!("38;5;{}", rng.below(256)),
        52..=59 => format!("48;5;{}", rng.below(256)),
        60..=67 => format!("38;2;{}", rgb_text(rng, ";")),
        68..=73 => format!("48;2;{}", rgb_text(rng, ";")),
        74..=77 => format!("38:2::{}", rgb_text(rng, ":")),
        // A colored underline needs an underline: the style comes first, then 58 with its color.
        78..=87 => {
            let style = 1 + rng.below(5);
            if rng.below(5) == 0 {
                format!("4:{style};58;5;{}", rng.below(256))
            } else {
                format!("4:{style};58:2::{}", rgb_text(rng, ":"))
            }
        }
        88..=89 => "59".to_string(),
        _ => RESETS[rng.below(RESETS.len())].to_string(),
    }
}

/// Lines of short words, each in a fresh mix of attributes, colors and resets.
pub(super) fn stylestorm(rng: &mut Rng, out: &mut Vec<u8>, target: usize) {
    while out.len() < target {
        let cells = 40 + rng.below(80);
        let mut width = 0;
        while width < cells {
            push_str(out, "\x1b[");
            for i in 0..1 + rng.below(3) {
                if i > 0 {
                    out.push(b';');
                }
                push_str(out, &sgr_param(rng));
            }
            out.push(b'm');
            let n = 1 + rng.below(8);
            words(rng, out, n);
            width += n;
        }
        push_str(out, "\x1b[0m\r\n");
    }
}

const DIRS: [&str; 8] = [
    "src",
    "crates/core/src",
    "tests",
    "docs/guide",
    "lib/models",
    "app/views",
    "packaging",
    "tools/bench/src",
];
const EXTS: [&str; 8] = ["rs", "toml", "md", "ts", "py", "json", "lock", "sh"];
const STEMS: [&str; 12] = [
    "main",
    "lib",
    "parser",
    "session",
    "queue",
    "cache",
    "config",
    "scheduler",
    "render",
    "store",
    "ipc",
    "layout",
];

/// `text` as a link to `url`: OSC 8 with an id, closed again right after the text.
fn link(out: &mut Vec<u8>, id: &mut u32, url: &str, text: &str) {
    *id += 1;
    push_str(out, &format!("\x1b]8;id={id};{url}\x1b\\{text}\x1b]8;;\x1b\\"));
}

fn path(rng: &mut Rng) -> String {
    format!(
        "{}/{}.{}",
        DIRS[rng.below(DIRS.len())],
        STEMS[rng.below(STEMS.len())],
        EXTS[rng.below(EXTS.len())]
    )
}

/// Short linked spans on ordinary lines, the way `ls --hyperlink`, compilers and `rg` print them.
pub(super) fn linkstorm(rng: &mut Rng, out: &mut Vec<u8>, target: usize) {
    let mut id = 0;
    while out.len() < target {
        match rng.below(10) {
            0..=3 => {
                // An `ls` row: names that are links, in columns.
                for _ in 0..3 + rng.below(4) {
                    let name = format!("{}.{}", STEMS[rng.below(STEMS.len())], EXTS[rng.below(EXTS.len())]);
                    let url = format!("file:///Users/dev/work/app/{}/{name}", DIRS[rng.below(DIRS.len())]);
                    push_str(out, if rng.below(4) == 0 { "\x1b[1;34m" } else { "" });
                    link(out, &mut id, &url, &name);
                    push_str(out, &format!("\x1b[0m{}", " ".repeat(2 + rng.below(8))));
                }
            }
            4..=7 => {
                // A compiler message with the location as a link.
                let p = path(rng);
                let (line, col) = (1 + rng.below(900), 1 + rng.below(80));
                let url = format!("file:///Users/dev/work/app/{p}:{line}:{col}");
                push_str(out, "\x1b[1m");
                link(out, &mut id, &url, &format!("{p}:{line}:{col}"));
                push_str(out, "\x1b[0m: \x1b[33mwarning\x1b[0m: ");
                let n = 12 + rng.below(40);
                words(rng, out, n);
            }
            _ => {
                // Prose with a link or two to the web.
                let n = 8 + rng.below(30);
                words(rng, out, n);
                for _ in 0..1 + rng.below(2) {
                    let url = format!(
                        "https://github.com/wormhole-terminal/{}/blob/{:07x}/{}#L{}",
                        STEMS[rng.below(STEMS.len())],
                        rng.next_u64() & 0xfff_ffff,
                        path(rng),
                        1 + rng.below(900)
                    );
                    push_str(out, " ");
                    link(out, &mut id, &url, &format!("#{}", 1 + rng.below(9000)));
                }
            }
        }
        push_str(out, "\r\n");
    }
}

/// Opening and closing a full-screen program over and over. Returns how many times.
///
/// The benchmark itself runs on the alternate screen, so each swap leaves it (back to the shell's
/// screen) and enters it again, then paints; the stream ends where it started, on the alternate screen.
pub(super) fn screenswap(rng: &mut Rng, out: &mut Vec<u8>, target: usize, size: Size) -> usize {
    let (cols, rows) = (size.cols.max(1) as usize, size.rows.max(1) as usize);
    let mut swaps = 0;
    let mut wanted = usize::MAX;
    while swaps < wanted {
        let start = out.len();
        let first = 1 + rng.below(800);
        push_str(out, "\x1b[?1049l\x1b[?1049h\x1b[?2026h\x1b[2J");
        for r in 0..rows - 1 {
            canvas::goto(out, 0, r as u16);
            push_str(out, &format!("\x1b[38;2;100;106;120m{:>4} \x1b[39m", (first + r) % 10_000));
            let src = CODE[rng.below(CODE.len())];
            highlight(out, &src[..src.len().min(cols.saturating_sub(5))]);
            push_str(out, "\x1b[K");
        }
        canvas::goto(out, 0, (rows - 1) as u16);
        push_str(out, &format!("\x1b[7m{:<cols$}\x1b[0m", &" src/lib.rs [+]"[..15.min(cols)]));
        push_str(out, "\x1b[0m\x1b[?2026l");
        swaps += 1;
        if swaps == 1 {
            wanted = frame_count(target, out.len() - start);
        }
    }
    swaps
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make(f: impl FnOnce(&mut Rng, &mut Vec<u8>)) -> String {
        let mut out = Vec::new();
        f(&mut Rng::new(11), &mut out);
        String::from_utf8_lossy(&out).into_owned()
    }

    /// The parameters of every SGR sequence, each split at `;`.
    fn sgr_params(s: &str) -> Vec<Vec<String>> {
        let mut found = Vec::new();
        let mut rest = s;
        while let Some(i) = rest.find("\x1b[") {
            rest = &rest[i + 2..];
            let end =
                rest.find(|c: char| !(c.is_ascii_digit() || c == ';' || c == ':')).unwrap_or(rest.len());
            if rest[end..].starts_with('m') {
                found.push(rest[..end].split(';').map(str::to_string).collect());
            }
        }
        found
    }

    #[test]
    fn every_attribute_appears() {
        let s = make(|r, o| stylestorm(r, o, 300_000));
        let params: Vec<String> = sgr_params(&s).into_iter().flatten().collect();
        let has = |p: &str| params.iter().any(|x| x == p);
        for p in [
            "1", "2", "3", "4", "4:1", "4:2", "4:3", "4:4", "4:5", "9", "53", "7", "5", "8", "0", "22", "23",
            "24", "29", "55", "27", "25", "28", "39", "49", "59",
        ] {
            assert!(has(p), "no SGR {p}");
        }
        let starts = |p: &str| params.iter().any(|x| x.starts_with(p));
        for p in ["38;5", "48;5", "38:2::", "58:2::", "58"] {
            // `38;5;n` is split at `;`, so look for its parts in the text.
            assert!(starts(p) || s.contains(&format!("\x1b[{p}")) || s.contains(&format!(";{p}")), "no {p}");
        }
        assert!(s.contains("38;2;") && s.contains("48;2;") && s.contains("4:3;58:2::"));
    }

    #[test]
    fn links_are_balanced_and_distinct() {
        let s = make(|r, o| linkstorm(r, o, 100_000));
        let open = s.matches("\x1b]8;id=").count();
        let close = s.matches("\x1b]8;;\x1b\\").count();
        assert!(open > 500, "{open}");
        assert_eq!(open, close);
        // Never nested: after an open comes its close before the next open.
        let mut depth = 0;
        for part in s.split("\x1b]8;").skip(1) {
            depth += if part.starts_with(';') { -1 } else { 1 };
            assert!((0..=1).contains(&depth));
        }
        let urls: std::collections::HashSet<&str> = s
            .split("\x1b]8;id=")
            .skip(1)
            .filter_map(|p| p.split_once(';')?.1.split_once("\x1b\\").map(|(u, _)| u))
            .collect();
        assert!(urls.len() > 400, "{}", urls.len());
    }

    #[test]
    fn swaps_enter_and_leave_the_same_number_of_times() {
        let mut out = Vec::new();
        let size = Size { cols: 80, rows: 24, px_w: 0, px_h: 0 };
        let swaps = screenswap(&mut Rng::new(2), &mut out, 1_000_000, size);
        let s = String::from_utf8_lossy(&out);
        assert!(swaps >= 30);
        assert_eq!(s.matches("\x1b[?1049h").count(), swaps);
        assert_eq!(s.matches("\x1b[?1049l").count(), swaps);
        assert_eq!(s.matches("\x1b[?2026h").count(), s.matches("\x1b[?2026l").count());
        // Each swap leaves first, so the stream ends in the alternate screen it began in.
        assert!(s.rfind("\x1b[?1049h") > s.rfind("\x1b[?1049l"));
    }
}
