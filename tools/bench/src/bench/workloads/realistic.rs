//! The four tests that are what real programs print: a build, an AI agent's answer, a pager and an
//! editor. They keep the mix a terminal sees every day (mostly text, a few colors, cursor motion in
//! the right places), where the stress tests push one thing to the limit.

use std::collections::VecDeque;

use crate::canvas::{self, Rgb, Rng};
use crate::term::Size;

/// Rust-looking lines to quote in diagnostics, diffs, editors and code blocks. ASCII only, so a line
/// can be cut at any byte.
pub(super) const CODE: [&str; 28] = [
    "use std::collections::HashMap;",
    "pub fn parse(input: &str) -> Result<Config, Error> {",
    "    let mut cache = HashMap::new();",
    "    for (key, value) in config.entries() {",
    "        if value.is_empty() { continue; }",
    "    let timeout = Duration::from_millis(250);",
    "    match self.state {",
    "        State::Idle => return Ok(()),",
    "        State::Busy(n) if n > 3 => self.retry(n)?,",
    "    // keep the connection open until the queue drains",
    "    let name = format!(\"worker-{}\", index);",
    "    tracing::debug!(\"flushed {} bytes\", written);",
    "    self.buffer.extend_from_slice(&chunk[..len]);",
    "impl Scheduler {",
    "    pub fn new(pool: usize) -> Self {",
    "        Self { pool, queue: Vec::new(), next_id: 0 }",
    "    }",
    "    let total: u64 = items.iter().map(|i| i.size).sum();",
    "    if let Some(entry) = cache.get(&key) {",
    "        return Ok(entry.clone());",
    "    }",
    "    assert_eq!(result.len(), 42, \"unexpected length\");",
    "#[derive(Debug, Clone, PartialEq)]",
    "pub struct Session { id: u32, token: String }",
    "    let handle = thread::spawn(move || worker.run());",
    "    drop(guard);",
    "const MAX_RETRIES: usize = 8;",
    "}",
];

const CRATES: [&str; 32] = [
    "serde",
    "serde_json",
    "tokio",
    "hyper",
    "regex",
    "clap",
    "rand",
    "anyhow",
    "thiserror",
    "tracing",
    "bytes",
    "futures",
    "libc",
    "memchr",
    "itoa",
    "syn",
    "quote",
    "proc-macro2",
    "unicode-ident",
    "once_cell",
    "parking_lot",
    "smallvec",
    "indexmap",
    "hashbrown",
    "mio",
    "socket2",
    "tower",
    "http",
    "url",
    "base64",
    "sha2",
    "wormhole-core",
];

pub(super) const PROSE: [&str; 48] = [
    "the",
    "function",
    "returns",
    "a",
    "new",
    "value",
    "when",
    "config",
    "is",
    "missing",
    "we",
    "should",
    "update",
    "handler",
    "to",
    "retry",
    "after",
    "timeout",
    "and",
    "cache",
    "result",
    "for",
    "each",
    "request",
    "this",
    "keeps",
    "state",
    "in",
    "sync",
    "with",
    "the",
    "session",
    "token",
    "because",
    "it",
    "was",
    "never",
    "refreshed",
    "so",
    "tests",
    "now",
    "pass",
    "on",
    "every",
    "branch",
    "of",
    "parser",
    "output",
];

const IDENTS: [&str; 12] = [
    "parse_config",
    "retry_after",
    "SessionToken",
    "flush_queue",
    "Scheduler::new",
    "cache.get()",
    "handle_request",
    "write_all",
    "Config",
    "run_pool",
    "split_at",
    "next_id",
];

pub(super) fn choose<'a>(rng: &mut Rng, list: &[&'a str]) -> &'a str {
    list[rng.below(list.len())]
}

fn push_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(s.as_bytes());
}

fn push_usize(out: &mut Vec<u8>, n: usize) {
    canvas::push_num(out, n.min(u32::MAX as usize) as u32);
}

fn digits(mut n: usize) -> usize {
    let mut d = 1;
    while n >= 10 {
        n /= 10;
        d += 1;
    }
    d
}

/// `bytes` cut to at most `max` bytes. Callers pass ASCII only.
pub(super) fn cut(s: &str, max: usize) -> &str {
    &s[..s.len().min(max)]
}

// --- buildlog ---------------------------------------------------------------------------------

/// A bold verb in a color, as cargo prints it: right-aligned in 12 cells.
fn verb(out: &mut Vec<u8>, color: u8, word: &str) {
    push_str(out, "\x1b[0m\x1b[1m\x1b[");
    push_usize(out, color as usize);
    push_str(out, "m");
    push_str(out, &format!("{word:>12}"));
    push_str(out, "\x1b[0m");
}

fn version(rng: &mut Rng, out: &mut Vec<u8>) {
    push_str(out, " v");
    push_usize(out, rng.below(3));
    out.push(b'.');
    push_usize(out, rng.below(30));
    out.push(b'.');
    push_usize(out, rng.below(20));
}

/// A rustc diagnostic: a bold colored header, the `-->` location, a gutter, the source line and its
/// `^^^^` underline in color, and a note.
fn diagnostic(rng: &mut Rng, out: &mut Vec<u8>) {
    let error = rng.below(4) == 0;
    let (color, kind, message, label) = if error {
        (31, "error[E0308]", "mismatched types", "expected `u32`, found `&str`")
    } else {
        match rng.below(3) {
            0 => (
                33,
                "warning",
                "unused variable: `entry`",
                "help: if this is intentional, prefix it with an underscore: `_entry`",
            ),
            1 => (
                33,
                "warning",
                "unused import: `std::collections::HashMap`",
                "help: remove the whole `use` item",
            ),
            _ => {
                (33, "warning", "unused `Result` that must be used", "this `Result` may be an `Err` variant")
            }
        }
    };
    let line_no = 1 + rng.below(900);
    let w = digits(line_no);
    let src = CODE[rng.below(CODE.len() - 1)];
    let first = src.len() - src.trim_start().len();
    let col = (first + rng.below(src.len().saturating_sub(first).max(1))).min(src.len().saturating_sub(1));
    let len = (2 + rng.below(9)).min(src.len() - col).max(1);
    let path = format!("src/{}.rs", choose(rng, &["lib", "main", "parser", "session", "queue", "cache"]));

    push_str(out, "\r\x1b[2K");
    verb_free(out, color, kind);
    push_str(out, "\x1b[0m\x1b[1m: ");
    push_str(out, message);
    push_str(out, "\x1b[0m\r\n");
    // `--> src/lib.rs:42:9`
    push_str(out, &" ".repeat(w));
    push_str(out, "\x1b[1m\x1b[34m--> \x1b[0m");
    push_str(out, &format!("{path}:{line_no}:{}\r\n", col + 1));
    let gutter = |out: &mut Vec<u8>, n: Option<usize>| {
        push_str(out, "\x1b[1m\x1b[34m");
        match n {
            Some(n) => push_str(out, &format!("{n:>w$} |")),
            None => push_str(out, &format!("{:w$} |", "")),
        }
        push_str(out, "\x1b[0m");
    };
    gutter(out, None);
    push_str(out, "\r\n");
    gutter(out, Some(line_no));
    push_str(out, " ");
    push_str(out, src);
    push_str(out, "\r\n");
    gutter(out, None);
    push_str(out, &format!(" {}", " ".repeat(col)));
    push_str(out, &format!("\x1b[1m\x1b[{color}m{} {label}\x1b[0m\r\n", "^".repeat(len)));
    gutter(out, None);
    push_str(out, "\r\n");
    push_str(
        out,
        &format!(
            "{:w$} \x1b[1m\x1b[34m=\x1b[0m \x1b[1mnote\x1b[0m: `#[warn(unused)]` on by default\r\n\r\n",
            ""
        ),
    );
}

/// `warning`/`error` headers are bold and colored but not right-aligned.
fn verb_free(out: &mut Vec<u8>, color: u8, word: &str) {
    push_str(out, "\x1b[0m\x1b[1m\x1b[");
    push_usize(out, color as usize);
    push_str(out, "m");
    push_str(out, word);
}

/// The `Building [=====>   ] 123/456: a, b` line, redrawn in place: no newline, so the next line
/// starts with CR and EL.
fn progress(rng: &mut Rng, out: &mut Vec<u8>, done: usize, total: usize) {
    const BAR: usize = 25;
    push_str(out, "\r\x1b[2K");
    verb(out, 36, "Building");
    let filled = (done * BAR / total.max(1)).min(BAR);
    push_str(out, " [");
    if filled == BAR {
        push_str(out, &"=".repeat(BAR));
    } else if filled == 0 {
        push_str(out, &" ".repeat(BAR));
    } else {
        push_str(out, &format!("{}>{}", "=".repeat(filled - 1), " ".repeat(BAR - filled)));
    }
    push_str(out, "] ");
    push_usize(out, done);
    out.push(b'/');
    push_usize(out, total);
    push_str(out, ": ");
    push_str(out, choose(rng, &CRATES));
    push_str(out, ", ");
    push_str(out, choose(rng, &CRATES));
}

pub(super) fn buildlog(rng: &mut Rng, out: &mut Vec<u8>, target: usize) {
    while out.len() < target {
        let total = 80 + rng.below(320);
        let mut done = 0;
        while done < total && out.len() < target {
            done += 1;
            // cargo clears the progress line before it prints anything above it.
            push_str(out, "\r\x1b[2K");
            verb(out, 32, "Compiling");
            out.push(b' ');
            let name = choose(rng, &CRATES);
            push_str(out, name);
            version(rng, out);
            if rng.below(7) == 0 {
                push_str(out, &format!(" (/Users/dev/work/app/crates/{name})"));
            }
            push_str(out, "\r\n");
            if rng.below(16) == 0 {
                diagnostic(rng, out);
            }
            for _ in 0..1 + rng.below(3) {
                progress(rng, out, done, total);
            }
        }
        push_str(out, "\r\x1b[2K");
        verb(out, 32, "Finished");
        push_str(
            out,
            &format!(
                " `dev` profile [unoptimized + debuginfo] target(s) in {}.{:02}s\r\n",
                3 + rng.below(90),
                rng.below(100)
            ),
        );
    }
    push_str(out, "\x1b[0m");
}

// --- agentstream ------------------------------------------------------------------------------

/// Splits `len` bytes into writes of 2 to 16 bytes, the size of the pieces a model's tokens arrive in.
fn split_writes(rng: &mut Rng, len: usize, sizes: &mut Vec<usize>) {
    let mut left = len;
    while left > 0 {
        let mut n = (2 + rng.below(15)).min(left);
        // A single byte left over would be a write of 1: take one more, or one less, now.
        if left - n == 1 {
            n = if n < 16 { n + 1 } else { n - 1 };
        }
        sizes.push(n);
        left -= n;
    }
}

fn tokens(n: usize) -> String {
    if n >= 1000 { format!("{}.{}k", n / 1000, n % 1000 / 100) } else { n.to_string() }
}

/// The status line under the answer: a spinner glyph, a verb and elapsed time and tokens.
pub(super) fn spinner(out: &mut Vec<u8>, tick: usize, produced: usize) {
    const GLYPHS: [&str; 6] = ["·", "✢", "✳", "✶", "✻", "✽"];
    const VERBS: [&str; 3] = ["Thinking", "Pondering", "Cogitating"];
    push_str(out, "\r\x1b[2K\x1b[38;2;215;119;87m");
    push_str(out, GLYPHS[tick % GLYPHS.len()]);
    push_str(
        out,
        &format!(
            " {}… \x1b[2m({}s · ↓ {} tokens)\x1b[0m",
            VERBS[tick / 60 % VERBS.len()],
            tick / 6,
            tokens(produced / 4)
        ),
    );
}

/// A line of prose with bold words and inline code, about `width` cells wide.
pub(super) fn prose(rng: &mut Rng, out: &mut Vec<u8>, width: usize) {
    let mut w = 0;
    while w < width {
        match rng.below(14) {
            0 => {
                let word = choose(rng, &PROSE);
                push_str(out, "\x1b[1m");
                push_str(out, word);
                push_str(out, "\x1b[22m");
                w += word.len() + 1;
            }
            1 | 2 => {
                let word = choose(rng, &IDENTS);
                push_str(out, "\x1b[38;2;230;160;100m`");
                push_str(out, word);
                push_str(out, "`\x1b[39m");
                w += word.len() + 3;
            }
            _ => {
                let word = choose(rng, &PROSE);
                push_str(out, word);
                w += word.len() + 1;
            }
        }
        out.push(b' ');
    }
}

const KEYWORD: Rgb = Rgb(198, 120, 221);
const STRING: Rgb = Rgb(152, 195, 121);
const NUMBER: Rgb = Rgb(209, 154, 102);
const FUNC: Rgb = Rgb(97, 175, 239);
const TYPE: Rgb = Rgb(229, 192, 123);
const COMMENT: Rgb = Rgb(92, 99, 112);

/// `line` with simple syntax colors: keywords, strings, numbers, calls, types and comments. Only the
/// foreground changes; the color is reset at the end.
pub(super) fn highlight(out: &mut Vec<u8>, line: &str) {
    const KEYWORDS: [&str; 18] = [
        "fn", "let", "mut", "pub", "use", "if", "else", "match", "return", "impl", "struct", "for", "in",
        "self", "const", "move", "continue", "if let",
    ];
    let b = line.as_bytes();
    let mut active = false;
    let mut paint = |out: &mut Vec<u8>, color: Option<Rgb>, text: &[u8]| {
        match color {
            Some(c) => {
                canvas::fg(out, c);
                active = true;
            }
            None if active => {
                push_str(out, "\x1b[39m");
                active = false;
            }
            None => {}
        }
        out.extend_from_slice(text);
    };
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let end;
        let color;
        if c == b'/' && b.get(i + 1) == Some(&b'/') {
            end = b.len();
            color = Some(COMMENT);
        } else if c == b'"' {
            end = b[i + 1..].iter().position(|x| *x == b'"').map_or(b.len(), |p| i + p + 2);
            color = Some(STRING);
        } else if c.is_ascii_digit() {
            end = i + b[i..].iter().take_while(|x| x.is_ascii_digit() || **x == b'_').count();
            color = Some(NUMBER);
        } else if c.is_ascii_alphabetic() || c == b'_' {
            end = i + b[i..].iter().take_while(|x| x.is_ascii_alphanumeric() || **x == b'_').count();
            let word = &line[i..end];
            color = if KEYWORDS.contains(&word) {
                Some(KEYWORD)
            } else if b.get(end) == Some(&b'(') || b.get(end) == Some(&b'!') {
                Some(FUNC)
            } else if c.is_ascii_uppercase() {
                Some(TYPE)
            } else {
                None
            };
        } else {
            end = i + 1;
            color = None;
        }
        paint(out, color, &b[i..end]);
        i = end;
    }
    if active {
        push_str(out, "\x1b[39m");
    }
}

pub(super) const CODE_BG: &str = "\x1b[48;2;40;44;52m";

/// One line of the answer as it is written: erase the status line it replaces, the text, and a line
/// break. Returns the bytes for the caller to split into writes.
fn answer_line(out: &mut Vec<u8>, body: impl FnOnce(&mut Vec<u8>)) {
    push_str(out, "\r\x1b[2K");
    body(out);
    push_str(out, "\x1b[0m\r\n");
}

/// A markdown-ish answer streamed as tiny writes, with the status line redrawn under it. Returns the
/// size of each write.
pub(super) fn agentstream(rng: &mut Rng, out: &mut Vec<u8>, target: usize, size: Size) -> Vec<usize> {
    let width = (size.cols as usize).clamp(20, 100) - 4;
    let mut sizes = Vec::new();
    let mut tick = 0;
    let mut seg = Vec::new();
    // Hands a finished piece to the stream and splits it into writes.
    let flush = |rng: &mut Rng, seg: &mut Vec<u8>, out: &mut Vec<u8>, sizes: &mut Vec<usize>| {
        if seg.len() == 1 {
            // Never a lone byte: keep the stream's writes all 2 to 16 bytes.
            seg.push(0x1b);
            seg.push(b'[');
            seg.push(b'm');
        }
        split_writes(rng, seg.len(), sizes);
        out.append(seg);
    };
    spinner(&mut seg, tick, 0);
    flush(rng, &mut seg, out, &mut sizes);
    while out.len() < target {
        let mut lines: Vec<Vec<u8>> = Vec::new();
        let line = |lines: &mut Vec<Vec<u8>>, body: &mut dyn FnMut(&mut Vec<u8>)| {
            let mut l = Vec::new();
            answer_line(&mut l, |o| body(o));
            lines.push(l);
        };
        match rng.below(10) {
            0..=3 => {
                for _ in 0..2 + rng.below(3) {
                    line(&mut lines, &mut |o| prose(rng, o, width));
                }
                line(&mut lines, &mut |_| {});
            }
            4 => {
                line(&mut lines, &mut |o| {
                    push_str(o, "\x1b[1m\x1b[38;2;130;180;255m## ");
                    for _ in 0..2 + rng.below(3) {
                        push_str(o, choose(rng, &PROSE));
                        o.push(b' ');
                    }
                });
            }
            5 | 6 => {
                for _ in 0..3 + rng.below(3) {
                    line(&mut lines, &mut |o| {
                        push_str(o, "  \x1b[38;2;130;180;255m•\x1b[39m ");
                        prose(rng, o, width.saturating_sub(6));
                    });
                }
                line(&mut lines, &mut |_| {});
            }
            7 | 8 => {
                line(&mut lines, &mut |o| push_str(o, "\x1b[2m```rust"));
                for _ in 0..4 + rng.below(7) {
                    let src = cut(CODE[rng.below(CODE.len())], width);
                    line(&mut lines, &mut |o| {
                        push_str(o, CODE_BG);
                        highlight(o, src);
                        push_str(o, "\x1b[K");
                    });
                }
                line(&mut lines, &mut |o| push_str(o, "\x1b[2m```"));
            }
            _ => {
                line(&mut lines, &mut |o| push_str(o, "\x1b[2m--- a/src/session.rs"));
                line(&mut lines, &mut |o| {
                    push_str(
                        o,
                        &format!("\x1b[36m@@ -{},7 +{},9 @@", 10 + rng.below(300), 10 + rng.below(300)),
                    )
                });
                for _ in 0..5 + rng.below(6) {
                    let src = cut(CODE[rng.below(CODE.len())], width);
                    let (mark, bg) = match rng.below(3) {
                        0 => ('-', "\x1b[48;2;70;30;35m"),
                        1 => ('+', "\x1b[48;2;25;60;35m"),
                        _ => (' ', ""),
                    };
                    line(&mut lines, &mut |o| {
                        push_str(o, bg);
                        o.push(mark as u8);
                        push_str(o, src);
                        push_str(o, "\x1b[K");
                    });
                }
            }
        }
        for l in lines {
            seg.extend_from_slice(&l);
            flush(rng, &mut seg, out, &mut sizes);
            // The status line comes back under the text, and now and then again on its own as time passes.
            for _ in 0..1 + usize::from(rng.below(2) == 0) {
                tick += 1;
                spinner(&mut seg, tick, out.len());
                flush(rng, &mut seg, out, &mut sizes);
            }
        }
    }
    push_str(&mut seg, "\r\x1b[2K\x1b[0m");
    flush(rng, &mut seg, out, &mut sizes);
    sizes
}

// --- diffscroll -------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Meta,
    Hunk,
    Add,
    Del,
    Context,
}

/// An endless `git diff`: files with hunks of context, removed and added lines.
struct DiffGen {
    queue: VecDeque<(Kind, String)>,
    hunks_left: usize,
}

impl DiffGen {
    fn next(&mut self, rng: &mut Rng) -> (Kind, String) {
        while self.queue.is_empty() {
            self.refill(rng);
        }
        self.queue.pop_front().unwrap_or((Kind::Context, String::new()))
    }

    fn refill(&mut self, rng: &mut Rng) {
        let q = &mut self.queue;
        if self.hunks_left == 0 {
            let file = format!(
                "{}/{}.rs",
                choose(rng, &["src", "crates/core/src", "tests"]),
                choose(rng, &["lib", "parser", "session", "queue", "cache", "scheduler"])
            );
            q.push_back((Kind::Meta, format!("diff --git a/{file} b/{file}")));
            q.push_back((
                Kind::Meta,
                format!(
                    "index {:07x}..{:07x} 100644",
                    rng.next_u64() & 0xfff_ffff,
                    rng.next_u64() & 0xfff_ffff
                ),
            ));
            q.push_back((Kind::Meta, format!("--- a/{file}")));
            q.push_back((Kind::Meta, format!("+++ b/{file}")));
            self.hunks_left = 1 + rng.below(4);
        }
        self.hunks_left -= 1;
        let (start, len) = (1 + rng.below(800), 12 + rng.below(12));
        q.push_back((
            Kind::Hunk,
            format!("@@ -{start},{len} +{start},{} @@ {}", len + 2, CODE[1].trim_end_matches(" {")),
        ));
        let ctx = |q: &mut VecDeque<(Kind, String)>, rng: &mut Rng, n: usize| {
            for _ in 0..n {
                q.push_back((Kind::Context, format!(" {}", CODE[rng.below(CODE.len())])));
            }
        };
        ctx(q, rng, 3);
        for _ in 0..1 + rng.below(3) {
            for _ in 0..rng.below(4) {
                q.push_back((Kind::Del, format!("-{}", CODE[rng.below(CODE.len())])));
            }
            for _ in 0..rng.below(5) {
                q.push_back((Kind::Add, format!("+{}", CODE[rng.below(CODE.len())])));
            }
            let n = 2 + rng.below(3);
            ctx(q, rng, n);
        }
    }
}

/// Pages through a colored diff the way `less` does: every page repaints from the top, each row
/// erased and drawn, and a reverse-video status line on the last row.
pub(super) fn diffscroll(rng: &mut Rng, out: &mut Vec<u8>, target: usize, size: Size) {
    let (cols, rows) = (size.cols.max(1) as usize, size.rows.max(1) as usize);
    let mut diff = DiffGen { queue: VecDeque::new(), hunks_left: 0 };
    // The benchmark is already on the alternate screen, so start the pager as a shell would: leave it
    // (the shell's screen) and enter it afresh.
    push_str(out, "\x1b[?1049l\x1b[?1049h\x1b[?25l");
    let mut page = 0;
    while out.len() < target {
        page += 1;
        canvas::goto(out, 0, 0);
        for _ in 0..rows - 1 {
            let (kind, text) = diff.next(rng);
            push_str(out, "\x1b[2K");
            push_str(
                out,
                match kind {
                    Kind::Meta => "\x1b[1m",
                    Kind::Hunk => "\x1b[36m",
                    Kind::Add => "\x1b[32m",
                    Kind::Del => "\x1b[31m",
                    Kind::Context => "",
                },
            );
            push_str(out, cut(&text, cols));
            push_str(out, "\x1b[0m\r\n");
        }
        push_str(out, "\x1b[7m\x1b[2K");
        let status = format!(" page {page}  (SPACE next page, b back, q quit)");
        push_str(out, &format!("{:<cols$}", cut(&status, cols)));
        push_str(out, "\x1b[0m");
    }
}

// --- editstorm --------------------------------------------------------------------------------

fn csi(out: &mut Vec<u8>, n: usize, last: u8) {
    push_str(out, "\x1b[");
    push_usize(out, n);
    out.push(last);
}

/// A line of code as an editor draws it: a dim line number, then the highlighted text.
fn editor_line(out: &mut Vec<u8>, rng: &mut Rng, number: usize, cols: usize) {
    push_str(out, &format!("\x1b[38;2;100;106;120m{:>4} \x1b[39m", number % 10_000));
    highlight(out, cut(CODE[rng.below(CODE.len())], cols.saturating_sub(5)));
}

/// An editor in use: a scroll region for the text, lines and characters inserted and deleted in
/// place (IL, DL, ICH, DCH, ECH), repainted syntax-colored lines, cursor moves, a status line and a
/// command line updated all the time, and the cursor changing shape between modes.
pub(super) fn editstorm(rng: &mut Rng, out: &mut Vec<u8>, target: usize, size: Size) {
    let (cols, rows) = (size.cols.max(1) as usize, size.rows.max(1) as usize);
    let chrome = rows >= 6;
    let text = if chrome { rows - 2 } else { rows };
    let mut top = 1 + rng.below(400);
    push_str(out, "\x1b[2J");
    push_str(out, &format!("\x1b[1;{text}r"));
    push_str(out, "\x1b[2 q");
    for r in 0..text {
        canvas::goto(out, 0, r as u16);
        editor_line(out, rng, top + r, cols);
    }
    let (mut row, mut col) = (text / 2, 8.min(cols - 1));
    let status = |out: &mut Vec<u8>, rng: &mut Rng, line: usize, col: usize| {
        push_str(out, "\x1b7");
        canvas::goto(out, 0, (rows - 2) as u16);
        let file = choose(rng, &["src/lib.rs", "src/session.rs", "src/parser.rs"]);
        let s = format!(
            " {} {file} [+]   {line},{}   {}%",
            choose(rng, &["NORMAL", "INSERT"]),
            col + 1,
            rng.below(100)
        );
        push_str(out, &format!("\x1b[7m{:<cols$}\x1b[0m", cut(&s, cols)));
        push_str(out, "\x1b8");
    };
    let cmdline = |out: &mut Vec<u8>, rng: &mut Rng| {
        push_str(out, "\x1b7");
        canvas::goto(out, 0, (rows - 1) as u16);
        push_str(out, "\x1b[2K");
        match rng.below(4) {
            0 => push_str(out, "\x1b[1m-- INSERT --\x1b[0m"),
            1 => push_str(out, &format!(":%s/{}/{}/g", choose(rng, &PROSE), choose(rng, &PROSE))),
            2 => push_str(out, &format!("/{}", choose(rng, &IDENTS))),
            _ => push_str(
                out,
                &format!("\"src/lib.rs\" {}L, {}B written", 100 + rng.below(900), 2000 + rng.below(30_000)),
            ),
        }
        push_str(out, "\x1b8");
    };
    while out.len() < target {
        let r = row.min(text - 1);
        match rng.below(100) {
            0..=24 => {
                // Typing: make room with ICH, then write the characters.
                let n = 1 + rng.below(8);
                canvas::goto(out, col as u16, r as u16);
                csi(out, n, b'@');
                for _ in 0..n {
                    out.push(b'a' + rng.below(26) as u8);
                }
                col = (col + n).min(cols - 1);
            }
            25..=34 => {
                canvas::goto(out, 0, r as u16);
                push_str(out, "\x1b[2K");
                editor_line(out, rng, top + r, cols);
            }
            35..=42 => {
                let n = 1 + rng.below(3.min(text - r).max(1));
                canvas::goto(out, 0, r as u16);
                csi(out, n, b'L');
                for k in 0..n {
                    canvas::goto(out, 0, (r + k).min(text - 1) as u16);
                    editor_line(out, rng, top + r + k, cols);
                }
            }
            43..=50 => {
                let n = 1 + rng.below(3.min(text - r).max(1));
                canvas::goto(out, 0, r as u16);
                csi(out, n, b'M');
                // The lines pulled up from below leave blank rows at the bottom of the region.
                for k in 0..n {
                    canvas::goto(out, 0, (text - 1 - k) as u16);
                    editor_line(out, rng, top + text - 1 - k, cols);
                }
            }
            51..=58 => {
                canvas::goto(out, col as u16, r as u16);
                csi(out, 1 + rng.below(6), b'P');
            }
            59..=63 => {
                canvas::goto(out, col as u16, r as u16);
                csi(out, 1 + rng.below(10), b'X');
            }
            64..=73 => {
                let n = 1 + rng.below(5);
                match rng.below(5) {
                    0 => {
                        csi(out, n, b'A');
                        row = r.saturating_sub(n);
                    }
                    1 => {
                        csi(out, n, b'B');
                        row = (r + n).min(text - 1);
                    }
                    2 => {
                        csi(out, n, b'C');
                        col = (col + n).min(cols - 1);
                    }
                    3 => {
                        csi(out, n, b'D');
                        col = col.saturating_sub(n);
                    }
                    _ => {
                        col = rng.below(cols);
                        csi(out, col + 1, b'G');
                    }
                }
            }
            74..=81 => {
                // Scrolling the text: a line feed at the bottom, a reverse index at the top, or a block.
                top += 1;
                match rng.below(4) {
                    0 => {
                        canvas::goto(out, 0, (text - 1) as u16);
                        push_str(out, "\n");
                        editor_line(out, rng, top + text - 1, cols);
                    }
                    1 => {
                        canvas::goto(out, 0, 0);
                        push_str(out, "\x1bM");
                        editor_line(out, rng, top, cols);
                    }
                    2 => {
                        let n = 1 + rng.below(4);
                        csi(out, n, b'S');
                        for k in 0..n.min(text) {
                            canvas::goto(out, 0, (text - 1 - k) as u16);
                            editor_line(out, rng, top + text - 1 - k, cols);
                        }
                    }
                    _ => {
                        let n = 1 + rng.below(4);
                        csi(out, n, b'T');
                        for k in 0..n.min(text) {
                            canvas::goto(out, 0, k as u16);
                            editor_line(out, rng, top + k, cols);
                        }
                    }
                }
            }
            82..=91 if chrome => status(out, rng, top + r, col),
            92..=96 if chrome => cmdline(out, rng),
            _ => {
                // Insert mode is a bar, normal a block, replace an underline.
                let shape = [2, 6, 4, 5][rng.below(4)];
                push_str(out, &format!("\x1b[{shape} q"));
            }
        }
    }
    push_str(out, "\x1b[r\x1b[0m\x1b[0 q");
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: Size = Size { cols: 100, rows: 30, px_w: 0, px_h: 0 };

    fn text(f: impl FnOnce(&mut Rng, &mut Vec<u8>)) -> String {
        let mut out = Vec::new();
        f(&mut Rng::new(7), &mut out);
        String::from_utf8_lossy(&out).into_owned()
    }

    #[test]
    fn writes_never_split_into_one_byte() {
        let mut rng = Rng::new(3);
        for len in [2usize, 3, 17, 18, 100, 1001] {
            let mut sizes = Vec::new();
            split_writes(&mut rng, len, &mut sizes);
            assert_eq!(sizes.iter().sum::<usize>(), len);
            assert!(sizes.iter().all(|n| (2..=16).contains(n)), "{len}: {sizes:?}");
        }
    }

    #[test]
    fn agent_writes_are_tokens_that_add_up() {
        let mut rng = Rng::new(9);
        let mut out = Vec::new();
        let sizes = agentstream(&mut rng, &mut out, 50_000, SIZE);
        assert_eq!(sizes.iter().sum::<usize>(), out.len());
        assert!(sizes.iter().all(|n| (2..=16).contains(n)));
        let s = String::from_utf8_lossy(&out);
        for needle in [
            "\x1b[1m",
            "\x1b[38;2;230;160;100m`",
            "•",
            "```rust",
            "@@ -",
            "✻",
            "Thinking",
            "tokens)",
            "\x1b[2K",
        ] {
            assert!(s.contains(needle), "{needle}");
        }
        assert!(s.contains("↓"));
    }

    #[test]
    fn buildlog_is_a_build() {
        let s = text(|r, o| buildlog(r, o, 100_000));
        assert!(s.contains("\x1b[1m\x1b[32m   Compiling\x1b[0m "));
        assert!(s.contains("--> "));
        assert!(s.contains("\x1b[1m\x1b[33mwarning") && s.contains("^^"));
        assert!(s.contains("\r\x1b[2K\x1b[0m\x1b[1m\x1b[36m    Building\x1b[0m ["));
        assert!(s.contains("    Finished\x1b[0m `dev`"));
        assert!(s.trim_end().ends_with("\x1b[0m"));
    }

    #[test]
    fn the_progress_bar_stays_the_same_width() {
        let mut rng = Rng::new(1);
        let widths: Vec<usize> = (0..=40usize)
            .map(|d| {
                let mut out = Vec::new();
                progress(&mut rng, &mut out, d, 40);
                let s = String::from_utf8_lossy(&out).into_owned();
                let open = s.find(" [").unwrap_or(0);
                s.rfind(']').unwrap_or(0) - open
            })
            .collect();
        assert!(widths.iter().all(|w| *w == 27), "{widths:?}");
    }

    #[test]
    fn diff_pages_repaint_from_the_top_with_a_status_line() {
        let mut out = Vec::new();
        diffscroll(&mut Rng::new(5), &mut out, 30_000, SIZE);
        let s = String::from_utf8_lossy(&out);
        assert!(s.starts_with("\x1b[?1049l\x1b[?1049h"));
        assert!(
            s.contains("\x1b[1;1H\x1b[2K")
                && s.contains("\x1b[36m@@ ")
                && s.contains("\x1b[32m+")
                && s.contains("\x1b[31m-")
        );
        assert!(s.contains("\x1b[7m\x1b[2K page 2 "));
        // Every page is rows-1 erased lines and a status row.
        assert_eq!(s.matches("\x1b[1;1H").count(), s.matches("\x1b[7m").count());
    }

    #[test]
    fn the_editor_edits_in_place() {
        let s = text(|r, o| editstorm(r, o, 100_000, SIZE));
        assert!(s.contains("\x1b[1;28r"));
        for fin in ['L', 'M', '@', 'P', 'X', 'S', 'T'] {
            let found = s.match_indices('\x1b').any(|(i, _)| {
                let rest = &s[i + 1..];
                rest.starts_with('[')
                    && rest[1..].chars().take_while(|c| c.is_ascii_digit()).count() > 0
                    && rest[1..].trim_start_matches(|c: char| c.is_ascii_digit()).starts_with(fin)
            });
            assert!(found, "no CSI n {fin}");
        }
        assert!(s.contains(" q") && s.contains("\x1b[7m"));
        assert!(s.ends_with("\x1b[r\x1b[0m\x1b[0 q"));
    }

    #[test]
    fn highlighting_resets_its_color() {
        let mut out = Vec::new();
        highlight(&mut out, "let x = foo(\"a\") + 12; // done");
        let s = String::from_utf8_lossy(&out).into_owned();
        assert!(s.contains("let") && s.ends_with("\x1b[39m"));
        assert!(s.matches("\x1b[39m").count() >= 2);
    }
}
