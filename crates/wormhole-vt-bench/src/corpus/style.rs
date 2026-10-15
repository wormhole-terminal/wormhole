//! Rendition and annotation: SGR churn, hyperlinks, titles and prompt marks.

use super::Out;

/// Colors and attributes changing every few characters, as a syntax highlighter or a `ls
/// --color` of a big tree makes them.
pub(super) fn sgr_churn(out: &mut Out, target: usize) {
    let mut col = 0;
    while out.len() < target {
        match out.rng.below(100) {
            0..=34 => {
                let n = out.rng.below(256);
                out.sgr(&format!("38;5;{n}"));
            }
            35..=54 => {
                let n = out.rng.below(256);
                out.sgr(&format!("48;5;{n}"));
            }
            55..=69 => {
                let (r, g, b) = (out.rng.below(256), out.rng.below(256), out.rng.below(256));
                out.sgr(&format!("38;2;{r};{g};{b}"));
            }
            70..=79 => {
                let (r, g, b) = (out.rng.below(256), out.rng.below(256), out.rng.below(256));
                out.sgr(&format!("48;2;{r};{g};{b}"));
            }
            80..=84 => {
                let code = *out.rng.pick(&[
                    30, 31, 32, 33, 34, 35, 36, 37, 90, 91, 92, 93, 94, 95, 96, 97, 40, 41, 42, 44, 100, 104,
                ]);
                out.sgr(&code.to_string());
            }
            85..=89 => {
                let code = *out.rng.pick(&["1", "2", "3", "4", "7", "9", "22", "23", "24", "27", "29"]);
                out.sgr(code);
            }
            90..=92 => {
                let (style, r, g, b) =
                    (out.rng.below(6), out.rng.below(256), out.rng.below(256), out.rng.below(256));
                out.sgr(&format!("4:{style};58;2;{r};{g};{b}"));
            }
            93..=95 => {
                let (n, r, g, b) =
                    (out.rng.below(256), out.rng.below(256), out.rng.below(256), out.rng.below(256));
                if out.rng.one_in(2) {
                    out.sgr(&format!("38:5:{n}"));
                } else {
                    out.sgr(&format!("48:2::{r}:{g}:{b}"));
                }
            }
            96 => out.sgr("1;38;5;196;48;5;17"),
            _ => out.sgr("0"),
        }
        let len = out.rng.range(1, 16);
        let chunk = out.line_of(len);
        out.text(&chunk);
        col += chunk.len();
        if col >= out.cols - 4 {
            out.sgr("0");
            out.text("\r\n");
            col = 0;
        }
    }
}

const HOSTS: [&str; 4] =
    ["https://github.com", "https://docs.rs", "https://example.com", "https://crates.io"];
const FILES: [&str; 6] =
    ["src/main.rs", "crates/core/src/lib.rs", "tests/api.rs", "src/ui/view.rs", "Cargo.toml", "README.md"];

/// A shell session with every annotation a modern shell sends: prompt marks (OSC 133), the
/// working directory (OSC 7), titles, and file and web links (OSC 8) ended with BEL or ST.
pub(super) fn osc8_links(out: &mut Out, target: usize) {
    while out.len() < target {
        let dir = *out.rng.pick(&["~/work/app", "~/work/lib", "/tmp/scratch", "~"]);
        out.text(&format!("\x1b]7;file://host{}\x07", dir.trim_start_matches('~')));
        let title_kind = *out.rng.pick(&[0, 2]);
        out.text(&format!("\x1b]{title_kind};user@host: {dir}\x07"));
        out.text("\x1b]133;A\x07");
        out.sgr("1;32");
        out.text("user@host");
        out.sgr("0");
        out.text(&format!(":{dir}$ "));
        out.text("\x1b]133;B\x07");
        let command = *out.rng.pick(&["cargo build", "git log --oneline", "rg TODO", "ls -la", "make test"]);
        out.text(command);
        out.text("\r\n\x1b]133;C\x07");
        for _ in 0..out.rng.range(3, 14) {
            let file = *out.rng.pick(&FILES);
            let (line, col) = (out.rng.range(1, 900), out.rng.range(1, 80));
            // Half the links end with ST, half with BEL, some carry an id.
            let end = if out.rng.one_in(2) { "\x1b\\" } else { "\x07" };
            let params =
                if out.rng.one_in(4) { format!("id=l{}", out.rng.below(1000)) } else { String::new() };
            out.text(&format!("\x1b]8;{params};file://host/work/{file}{end}"));
            out.sgr("4;36");
            out.text(&format!("{file}:{line}:{col}"));
            out.sgr("0");
            out.text(&format!("\x1b]8;;{end}"));
            let rest = out.line_between(10, 70);
            out.text(&format!(": {rest}"));
            if out.rng.one_in(4) {
                let host = *out.rng.pick(&HOSTS);
                let page = out.rng.below(5000);
                out.text(&format!(" (see \x1b]8;;{host}/{page}\x1b\\{host}\x1b]8;;\x1b\\)"));
            }
            out.text("\r\n");
        }
        let status = out.rng.below(2);
        out.text(&format!("\x1b]133;D;{status}\x07"));
    }
}
