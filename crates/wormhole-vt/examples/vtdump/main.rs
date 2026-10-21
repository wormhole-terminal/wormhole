//! `vtdump`: what is in a byte stream, one parsed item per line. The first tool to reach for when a
//! program draws wrong: it shows the sequences it really sent, by name.
//!
//! ```sh
//! cargo run -q -p wormhole-vt --example vtdump -- session.raw
//! cat session.raw | cargo run -q -p wormhole-vt --example vtdump -- --no-text --offset 0..4096
//! cargo run -q -p wormhole-vt --example vtdump -- --engine --size 120x40 session.raw
//! ```
//!
//! Each line is the byte offset where the item starts, its bytes (`\e` is ESC; long ones are cut)
//! and its name:
//!
//! ```text
//!        0  \e[1;31m                    SGR 1;31 (bold, fg red)
//!        7  error                       text "error" (5 chars)
//!       12  \e[?2004h                   DECSET 2004 (bracketed paste)
//!       20  \e]8;;https://example.com\e\\  OSC 8 hyperlink "https://example.com"
//! ```
//!
//! Options: `--no-text` leaves out printed text, `--offset A..B` only the items touching those
//! bytes (either end may be empty), `--engine` also feeds every item to a terminal (`--size
//! COLSxROWS`, default 80x24, or the `.meta` file's next to a `.raw` recording) and prints under it
//! the replies and events it produced, `→ reply \e[2;1R`, `→ event Title`.
//!
//! The parser is fed one byte at a time to find where each item ends: this is a debug tool, not a
//! fast one. A control character inside a sequence (`ESC [ 1 LF m`) is listed when it comes, so
//! its offset may be past the sequence's start; bytes the parser throws away are listed as
//! `ignored`. Only the public API is used (`parser::{Parser, Handler, Csi}`, `Terminal`).
#![allow(clippy::print_stdout)] // a command line tool: the listing is the output

use std::io::Read as _;
use std::ops::Range;

use wormhole_vt::Terminal;

mod items;
mod names;
mod sgr;
mod strings;
mod text;

use items::items_of;
use text::{escaped, shortened, text_name};

const USAGE: &str = "usage: vtdump [--no-text] [--offset A..B] [--engine] [--size COLSxROWS] [FILE|-]";

fn main() {
    let mut options = Options::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return println!("{USAGE}"),
            "--no-text" => options.no_text = true,
            "--engine" => options.engine = true,
            "--offset" => match args.next().as_deref().and_then(parse_range) {
                Some(range) => options.offset = Some(range),
                None => return eprintln!("--offset needs A..B"),
            },
            "--size" => match args.next().as_deref().and_then(parse_size) {
                Some(size) => options.size = Some(size),
                None => return eprintln!("--size needs COLSxROWS"),
            },
            _ if arg.starts_with('-') && arg != "-" => return eprintln!("unknown option {arg}\n{USAGE}"),
            _ => options.file = Some(arg),
        }
    }
    let mut bytes = Vec::new();
    let read = match options.file.as_deref() {
        None | Some("-") => std::io::stdin().lock().read_to_end(&mut bytes).map(|_| ()),
        Some(path) => std::fs::read(path).map(|data| bytes = data),
    };
    if let Err(err) = read {
        return eprintln!("vtdump: {err}");
    }
    if options.size.is_none() {
        options.size = options.file.as_deref().and_then(recorded_size);
    }
    dump(&bytes, &options);
}

#[derive(Default)]
struct Options {
    file: Option<String>,
    no_text: bool,
    engine: bool,
    offset: Option<Range<usize>>,
    size: Option<(usize, usize)>,
}

fn parse_range(text: &str) -> Option<Range<usize>> {
    let (from, to) = text.split_once("..")?;
    let from = if from.is_empty() { 0 } else { from.parse().ok()? };
    let to = if to.is_empty() { usize::MAX } else { to.parse().ok()? };
    Some(from..to)
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

// ---- output ----

fn dump(bytes: &[u8], options: &Options) {
    let (cols, rows) = options.size.unwrap_or((80, 24));
    let mut term = options.engine.then(|| Terminal::new(cols, rows));
    let mut fed = 0;
    items_of(bytes, |item| {
        // The engine hears everything up to this item, listed or not, so its state stays right.
        let mut answers = Vec::new();
        if let Some(term) = term.as_mut() {
            if item.end > fed {
                term.feed(&bytes[fed..item.end]);
                fed = item.end;
            }
            let replies = term.take_replies();
            if !replies.is_empty() {
                answers.push(format!("→ reply {}", escaped(&replies)));
            }
            for event in term.events() {
                answers.push(format!("→ event {}", shortened(&format!("{event:?}"), 100)));
            }
        }
        let shown = options.offset.as_ref().is_none_or(|r| item.start < r.end && item.end > r.start);
        if !shown || (options.no_text && item.text) {
            return;
        }
        let raw = &bytes[item.start..item.end];
        let name = if item.text { text_name(raw) } else { item.name.clone() };
        let raw_text = escaped(&raw[..raw.len().min(200)]);
        println!("{:>9}  {:<34}  {name}", item.start, shortened(&raw_text, 34));
        for answer in answers {
            println!("{:>9}  {answer}", "");
        }
    });
}
