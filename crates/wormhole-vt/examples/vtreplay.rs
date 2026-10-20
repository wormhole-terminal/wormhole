//! `vtreplay`: feed a recording to a terminal and look at the screen it ends with, at a byte
//! offset, or as it goes.
//!
//! ```sh
//! cargo run -q -p wormhole-vt --example vtreplay -- --size 120x40 session.raw
//! cargo run -q -p wormhole-vt --example vtreplay -- --at 4096 session.raw         # after 4096 bytes
//! cargo run -q -p wormhole-vt --example vtreplay -- --every 8192 session.raw      # a screen per 8 KiB
//! cargo run -q -p wormhole-vt --example vtreplay -- --grep "error[E0308]" session.raw
//! ```
//!
//! The screen is printed as text between rules, one row per line with its number, and a `^` line
//! under the cursor's row to show where the cursor is. `--size COLSxROWS` is the terminal's size
//! (80x24; for a `.raw` recording with a `.meta` file beside it, the size it was recorded at).
//! `--grep TEXT` prints the first byte offset after which TEXT is on the screen (one row of it; the
//! history is not searched) and the screen then; it exits with 1 when it never is. Pair it with
//! `vtdump --offset` to see which sequences led there. Only the public API is used.
#![allow(clippy::print_stdout)] // a command line tool: the screen is the output

use wormhole_vt::unicode::width;
use wormhole_vt::{Line, Terminal};

const USAGE: &str = "usage: vtreplay [--size COLSxROWS] [--at BYTES | --every BYTES | --grep TEXT] FILE";

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let mut file = None;
    let mut size = None;
    let mut at = None;
    let mut every = None;
    let mut grep = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut number = |name: &str| -> Option<usize> {
            let value = args.next().and_then(|v| v.parse().ok());
            if value.is_none() {
                eprintln!("{name} needs a number");
            }
            value
        };
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                return 0;
            }
            "--size" => match args.next().as_deref().and_then(parse_size) {
                Some(s) => size = Some(s),
                None => return usage("--size needs COLSxROWS"),
            },
            "--at" => match number("--at") {
                Some(n) => at = Some(n),
                None => return 2,
            },
            "--every" => match number("--every") {
                Some(n) if n > 0 => every = Some(n),
                _ => return usage("--every needs a number above 0"),
            },
            "--grep" => match args.next() {
                Some(text) if !text.is_empty() => grep = Some(text),
                _ => return usage("--grep needs text"),
            },
            _ if arg.starts_with('-') => return usage(&format!("unknown option {arg}")),
            _ => file = Some(arg),
        }
    }
    let Some(file) = file else { return usage("no file") };
    let bytes = match std::fs::read(&file) {
        Ok(bytes) => bytes,
        Err(err) => return usage(&format!("{file}: {err}")),
    };
    let (cols, rows) = size.or_else(|| recorded_size(&file)).unwrap_or((80, 24));
    let fresh = || Terminal::new(cols, rows);

    if let Some(needle) = grep {
        return match find(&bytes, &needle, rows, &fresh) {
            Some(offset) => {
                let mut term = fresh();
                term.feed(&bytes[..offset]);
                println!("{needle:?} first on screen after {offset} bytes (the byte at {})", offset - 1);
                show(&term, offset, cols, rows);
                0
            }
            None => {
                eprintln!("{needle:?} is never on the screen");
                1
            }
        };
    }
    let mut term = fresh();
    if let Some(step) = every {
        let mut done = 0;
        while done < bytes.len() {
            let end = (done + step).min(bytes.len());
            term.feed(&bytes[done..end]);
            done = end;
            show(&term, done, cols, rows);
        }
        return 0;
    }
    let end = at.unwrap_or(bytes.len()).min(bytes.len());
    term.feed(&bytes[..end]);
    show(&term, end, cols, rows);
    0
}

fn usage(why: &str) -> i32 {
    eprintln!("vtreplay: {why}\n{USAGE}");
    2
}

fn parse_size(text: &str) -> Option<(usize, usize)> {
    let (cols, rows) = text.split_once('x')?;
    Some((cols.parse().ok().filter(|&n| n > 0)?, rows.parse().ok().filter(|&n| n > 0)?))
}

/// The size a recording was made at (`name.meta` beside `name.raw`, `cols=` and `rows=` lines).
fn recorded_size(path: &str) -> Option<(usize, usize)> {
    let meta = std::fs::read_to_string(std::path::Path::new(path).with_extension("meta")).ok()?;
    let value = |key: &str| {
        meta.lines().find_map(|line| line.strip_prefix(key)?.strip_prefix('=')?.trim().parse::<usize>().ok())
    };
    Some((value("cols")?, value("rows")?))
}

fn on_screen(term: &Terminal, rows: usize, needle: &str) -> bool {
    (0..rows).any(|y| term.row_text(Line(y as i32), true).contains(needle))
}

/// The smallest number of bytes after which `needle` is on the screen. The screen is searched every
/// `CHUNK` bytes, not after each one (text written and erased inside one chunk is missed), and the
/// chunk it shows up in is then replayed byte by byte.
fn find(bytes: &[u8], needle: &str, rows: usize, fresh: &impl Fn() -> Terminal) -> Option<usize> {
    const CHUNK: usize = 512;
    let mut term = fresh();
    let mut done = 0;
    while done < bytes.len() {
        let end = (done + CHUNK).min(bytes.len());
        term.feed(&bytes[done..end]);
        if on_screen(&term, rows, needle) {
            break;
        }
        done = end;
    }
    if done >= bytes.len() {
        return None;
    }
    // Replay the chunk it showed up in byte by byte, for the exact offset.
    let mut term = fresh();
    term.feed(&bytes[..done]);
    for k in done..(done + CHUNK).min(bytes.len()) {
        term.feed(&bytes[k..=k]);
        if on_screen(&term, rows, needle) {
            return Some(k + 1);
        }
    }
    None
}

fn show(term: &Terminal, offset: usize, cols: usize, rows: usize) {
    let cursor = term.cursor();
    let (cx, cy) = (cursor.point.column.0, cursor.point.line.0);
    println!("{:-<1$}", format!("--- after {offset} bytes "), cols.max(24) + 5);
    for y in 0..rows {
        let text = term.row_text(Line(y as i32), true);
        println!("{y:>3}│{text}");
        if y as i32 == cy {
            // Under the cursor's cell: a wide character before it takes two columns.
            let mut pad = 0;
            let mut col = 0;
            for c in text.chars() {
                let w = usize::from(width(c));
                if col + w > cx {
                    break;
                }
                col += w;
                pad += 1;
            }
            let pad = pad + cx.saturating_sub(col);
            let shown = if cursor.visible { "" } else { " (hidden)" };
            println!("   │{}^ cursor, row {cy} column {cx}{shown}", " ".repeat(pad));
        }
    }
    println!();
}
