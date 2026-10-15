//! A coding agent's session in a terminal: what the whole mix looks like in one stream.

use super::Out;

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const KEYWORDS: [&str; 8] = ["fn", "let", "match", "impl", "pub", "struct", "use", "return"];
const FILES: [&str; 5] =
    ["src/lib.rs", "src/app/session.rs", "crates/core/src/parser.rs", "tests/api.rs", "Cargo.toml"];

/// Streamed markdown, a spinner, tool calls with diffs, boxes, a progress bar, a status line
/// kept with save and restore cursor, hyperlinks and one inline image, turn after turn.
pub(super) fn agent_session(out: &mut Out, target: usize) {
    out.text("\x1b[?2004h\x1b[?25l");
    let mut tokens = 0u64;
    let mut image = 0u32;
    while out.len() < target {
        user_prompt(out);
        spinner(out);
        assistant_text(out);
        tool_call(out);
        if out.rng.one_in(2) {
            diff(out);
        }
        if out.rng.one_in(3) {
            progress(out);
        }
        if out.rng.one_in(5) {
            image += 1;
            let data = out.base64(6000);
            out.text(&format!("\x1b_Ga=T,f=100,t=d,i={image},m=0;{data}\x1b\\\r\n"));
        }
        tokens += out.rng.range(200, 3000) as u64;
        status_line(out, tokens);
    }
    out.text("\x1b[?25h\x1b[?2004l");
}

fn user_prompt(out: &mut Out) {
    let width = out.cols - 4;
    let text = out.line_between(15, 90);
    out.sgr("38;5;244");
    out.text(&format!("╭{}╮\r\n│ ", "─".repeat(width)));
    out.sgr("0;1");
    out.text(&format!("> {text:<w$}", w = width - 4));
    out.sgr("0;38;5;244");
    out.text(" │\r\n");
    out.text(&format!("╰{}╯", "─".repeat(width)));
    out.sgr("0");
    out.text("\r\n");
}

fn spinner(out: &mut Out) {
    for i in 0..out.rng.range(8, 30) {
        let color = 208 + i % 6;
        let frame = SPINNER[i % SPINNER.len()];
        out.text("\r\x1b[2K");
        out.sgr(&format!("38;5;{color}"));
        out.text(frame);
        out.sgr("0");
        out.text(&format!(" Thinking… ({}s · esc to interrupt)", i / 4));
    }
    out.text("\r\x1b[2K");
}

/// Markdown arriving a few words at a time, wrapped by the program at the screen width.
fn assistant_text(out: &mut Out) {
    let mut col = 0;
    out.sgr("1;35");
    let heading = out.words(3);
    out.text(&format!("## {heading}"));
    out.sgr("0");
    out.text("\r\n\r\n");
    for _ in 0..out.rng.range(15, 60) {
        let piece = match out.rng.below(12) {
            0 => {
                let w = out.word();
                format!("\x1b[1m{w}\x1b[22m")
            }
            1 => {
                let w = out.word();
                format!("\x1b[3m{w}\x1b[23m")
            }
            2 => {
                let w = out.word();
                format!("\x1b[48;5;238m\x1b[38;5;215m {w}() \x1b[0m")
            }
            3 => {
                let n = FILES[out.rng.below(FILES.len())];
                format!("\x1b]8;;file:///work/{n}\x1b\\\x1b[4m{n}\x1b[24m\x1b]8;;\x1b\\")
            }
            _ => out.words_between(1, 5),
        };
        out.text(&piece);
        out.text(" ");
        col += visible_width(&piece) + 1;
        if col > out.cols - 12 {
            out.text("\r\n");
            col = 0;
        }
    }
    out.text("\r\n");
    if out.rng.one_in(2) {
        code_block(out);
    }
}

fn code_block(out: &mut Out) {
    out.sgr("38;5;244");
    out.text("```rust\r\n");
    out.sgr("0");
    for _ in 0..out.rng.range(3, 12) {
        let keyword = KEYWORDS[out.rng.below(KEYWORDS.len())];
        let name = out.word();
        let value = out.rng.below(1000);
        out.text("    ");
        out.sgr("38;5;204");
        out.text(keyword);
        out.sgr("38;5;250");
        out.text(&format!(" {name} = "));
        out.sgr("38;5;141");
        out.text(&value.to_string());
        out.sgr("38;5;250");
        out.text(";");
        out.sgr("0");
        out.text("\r\n");
    }
    out.sgr("38;5;244");
    out.text("```");
    out.sgr("0");
    out.text("\r\n");
}

fn tool_call(out: &mut Out) {
    let file = FILES[out.rng.below(FILES.len())];
    out.sgr("38;5;78");
    out.text("⏺ ");
    out.sgr("0;1");
    out.text("Read");
    out.sgr("0");
    out.text(&format!("({file})\r\n"));
    for i in 0..out.rng.range(1, 5) {
        let line = out.line_between(20, 100);
        out.sgr("38;5;244");
        out.text(if i == 0 { "  ⎿  " } else { "     " });
        out.sgr("0");
        out.text(&format!("{line}\r\n"));
    }
}

fn diff(out: &mut Out) {
    let first = out.rng.range(10, 400);
    let count = out.rng.range(4, 12);
    for line in first..first + count {
        let text = out.line_between(15, 80);
        match out.rng.below(3) {
            0 => {
                out.sgr("38;5;242");
                out.text(&format!("{line:>5} "));
                out.sgr("0;48;5;52;38;5;217");
                out.text(&format!("- {text}"));
                out.csi("K");
                out.sgr("0");
            }
            1 => {
                out.sgr("38;5;242");
                out.text(&format!("{line:>5} "));
                out.sgr("0;48;5;22;38;5;157");
                out.text(&format!("+ {text}"));
                out.csi("K");
                out.sgr("0");
            }
            _ => {
                out.sgr("38;5;242");
                out.text(&format!("{line:>5} "));
                out.sgr("0");
                out.text(&format!("  {text}"));
            }
        }
        out.text("\r\n");
    }
}

fn progress(out: &mut Out) {
    for step in (0..=20).step_by(2) {
        out.text("\r");
        out.sgr("38;5;39");
        out.text(&"█".repeat(step));
        out.sgr("38;5;238");
        out.text(&"░".repeat(20 - step));
        out.sgr("0");
        out.text(&format!(" {:>3}%", step * 5));
    }
    out.text("\r\n");
}

/// Redrawn at the bottom of the screen without disturbing the cursor.
fn status_line(out: &mut Out, tokens: u64) {
    out.text("\x1b7");
    out.goto(out.rows, 1);
    out.csi("2K");
    out.sgr("38;5;244");
    out.text(&format!("  {tokens} tokens · claude · ? for shortcuts"));
    out.sgr("0");
    out.text("\x1b8");
    out.text(&format!("\x1b]0;agent — {tokens} tokens\x07"));
}

/// Columns a piece of text takes, leaving out escape sequences (the generator only needs a rough
/// figure to wrap lines).
fn visible_width(s: &str) -> usize {
    let mut width = 0;
    let mut in_escape = false;
    for c in s.chars() {
        match (in_escape, c) {
            (false, '\x1b') => in_escape = true,
            (true, 'm' | '\\') => in_escape = false,
            (true, _) => {}
            (false, _) => width += 1,
        }
    }
    width
}
