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

use std::fmt::Write as _;
use std::io::Read as _;
use std::ops::Range;

use wormhole_vt::Terminal;
use wormhole_vt::parser::{Csi, Handler, Parser};

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

// ---- finding the items ----

/// What the parser reported, as values; the bytes it was reported for are known by position.
enum Ev {
    Text,
    Exec(u8),
    Csi(Csi),
    Esc(Vec<u8>, u8),
    OscStart,
    OscPut(Vec<u8>),
    OscEnd {
        bel: bool,
    },
    Dcs(Csi),
    DcsPut(Vec<u8>),
    ApcStart,
    ApcPut(Vec<u8>),
    /// A string ended without its terminator: CAN or SUB, or (for the unhook/end calls) ESC.
    Cancel,
    End,
}

#[derive(Default)]
struct Collect(Vec<Ev>);

impl Handler for Collect {
    fn print_ascii(&mut self, _run: &[u8]) {
        self.0.push(Ev::Text);
    }
    fn print(&mut self, _c: char) {
        self.0.push(Ev::Text);
    }
    fn execute(&mut self, byte: u8) {
        self.0.push(Ev::Exec(byte));
    }
    fn csi(&mut self, csi: &Csi) {
        self.0.push(Ev::Csi(csi.clone()));
    }
    fn esc(&mut self, intermediates: &[u8], byte: u8) {
        self.0.push(Ev::Esc(intermediates.to_vec(), byte));
    }
    fn osc_start(&mut self) {
        self.0.push(Ev::OscStart);
    }
    fn osc_put(&mut self, data: &[u8]) {
        self.0.push(Ev::OscPut(data.to_vec()));
    }
    fn osc_end(&mut self, bel_terminated: bool) {
        self.0.push(Ev::OscEnd { bel: bel_terminated });
    }
    fn osc_cancel(&mut self) {
        self.0.push(Ev::Cancel);
    }
    fn dcs_hook(&mut self, dcs: &Csi) {
        self.0.push(Ev::Dcs(dcs.clone()));
    }
    fn dcs_put(&mut self, data: &[u8]) {
        self.0.push(Ev::DcsPut(data.to_vec()));
    }
    fn dcs_unhook(&mut self) {
        self.0.push(Ev::End);
    }
    fn dcs_cancel(&mut self) {
        self.0.push(Ev::Cancel);
    }
    fn apc_start(&mut self) {
        self.0.push(Ev::ApcStart);
    }
    fn apc_put(&mut self, data: &[u8]) {
        self.0.push(Ev::ApcPut(data.to_vec()));
    }
    fn apc_end(&mut self) {
        self.0.push(Ev::End);
    }
    fn apc_cancel(&mut self) {
        self.0.push(Ev::Cancel);
    }
}

/// One thing in the stream: where it is, and what it is called.
struct Item {
    start: usize,
    end: usize,
    text: bool,
    name: String,
}

/// A control string being read.
enum Open {
    Osc(Vec<u8>, usize),
    Dcs(Csi, Vec<u8>, usize),
    Apc(Vec<u8>, usize),
}

/// How much of a payload is kept for the name; the rest is only counted.
const KEEP: usize = 256;

fn push_capped(into: &mut Vec<u8>, total: &mut usize, data: &[u8]) {
    *total += data.len();
    let room = KEEP.saturating_sub(into.len());
    into.extend_from_slice(&data[..data.len().min(room)]);
}

/// Hands out the items in order; printed characters are held back until the run is over.
struct Lister<F: FnMut(&Item)> {
    each: F,
    text: Option<Item>,
}

impl<F: FnMut(&Item)> Lister<F> {
    fn flush(&mut self) {
        if let Some(run) = self.text.take() {
            (self.each)(&run);
        }
    }

    /// A printed character that ends at `end`, the item before it ending at `from`.
    fn text(&mut self, from: usize, end: usize) {
        match &mut self.text {
            Some(run) if run.end == from => run.end = end,
            _ => {
                self.flush();
                self.text = Some(Item { start: from, end, text: true, name: String::new() });
            }
        }
    }

    fn item(&mut self, start: usize, end: usize, name: String) {
        self.flush();
        (self.each)(&Item { start, end, text: false, name });
    }
}

fn items_of(bytes: &[u8], each: impl FnMut(&Item)) {
    let mut parser = Parser::new();
    let mut sink = Collect::default();
    let mut open: Option<Open> = None;
    // Where the next item starts: everything before it is accounted for.
    let mut from = 0;
    let mut list = Lister { each, text: None };
    for (i, byte) in bytes.iter().enumerate() {
        parser.advance(&mut sink, std::slice::from_ref(byte));
        for ev in sink.0.drain(..) {
            match ev {
                Ev::Text => {
                    list.text(from, i + 1);
                    from = i + 1;
                }
                Ev::Exec(b) => {
                    // CAN and SUB abort what was being read: the bytes before are dropped.
                    if (b == 0x18 || b == 0x1a) && open.is_none() && from < i {
                        list.item(from, i, "ignored (aborted sequence)".into());
                        from = i;
                    }
                    // A control inside a sequence is listed at its own byte; the sequence keeps its start.
                    list.item(i, i + 1, control_name(b));
                    if from == i {
                        from = i + 1;
                    }
                }
                Ev::Csi(csi) => {
                    list.item(from, i + 1, csi_name(&csi));
                    from = i + 1;
                }
                Ev::Esc(inter, b) => {
                    list.item(from, i + 1, esc_name(&inter, b));
                    from = i + 1;
                }
                Ev::OscStart => open = Some(Open::Osc(Vec::new(), 0)),
                Ev::ApcStart => open = Some(Open::Apc(Vec::new(), 0)),
                Ev::Dcs(csi) => open = Some(Open::Dcs(csi, Vec::new(), 0)),
                Ev::OscPut(data) => {
                    if let Some(Open::Osc(kept, total)) = &mut open {
                        push_capped(kept, total, &data);
                    }
                }
                Ev::DcsPut(data) => {
                    if let Some(Open::Dcs(_, kept, total)) = &mut open {
                        push_capped(kept, total, &data);
                    }
                }
                Ev::ApcPut(data) => {
                    if let Some(Open::Apc(kept, total)) = &mut open {
                        push_capped(kept, total, &data);
                    }
                }
                Ev::OscEnd { .. } | Ev::End | Ev::Cancel => {
                    let cancelled = matches!(ev, Ev::Cancel);
                    let bel = matches!(ev, Ev::OscEnd { bel: true });
                    let Some(string) = open.take() else { continue };
                    // BEL ends the string with itself; ST is `ESC \`; an ESC that is not followed by
                    // `\` began another sequence, and a cancel is the CAN or SUB, listed after it.
                    let end = if bel {
                        i + 1
                    } else if !cancelled && bytes.get(i + 1) == Some(&b'\\') {
                        i + 2
                    } else {
                        i
                    };
                    let mut name = string_name(&string);
                    if cancelled {
                        name.push_str(" (cancelled)");
                    }
                    list.item(from, end, name);
                    from = end;
                }
            }
        }
        // A byte the parser took and said nothing about, with nothing left open: thrown away.
        if parser.is_ground() && from < i + 1 && open.is_none() {
            list.item(from, i + 1, "ignored".into());
            from = i + 1;
        }
    }
    list.flush();
    if from < bytes.len() {
        // The stream ended inside a sequence.
        list.item(from, bytes.len(), "unfinished sequence".into());
    }
}

// ---- the table of names (xterm's ctlseqs) ----

fn control_name(b: u8) -> String {
    let name = match b {
        0x00 => "NUL",
        0x05 => "ENQ",
        0x07 => "BEL",
        0x08 => "BS",
        0x09 => "HT",
        0x0a => "LF",
        0x0b => "VT",
        0x0c => "FF",
        0x0d => "CR",
        0x0e => "SO (shift out)",
        0x0f => "SI (shift in)",
        0x18 => "CAN",
        0x1a => "SUB",
        _ => return format!("C0 control {b:#04x}"),
    };
    name.to_string()
}

/// Final bytes of ESC sequences, with their intermediates (`(0` designates DEC special graphics).
fn esc_name(inter: &[u8], b: u8) -> String {
    let known = match (inter, b) {
        ([], b'7') => "DECSC (save cursor)",
        ([], b'8') => "DECRC (restore cursor)",
        ([], b'=') => "DECKPAM (application keypad)",
        ([], b'>') => "DECKPNM (numeric keypad)",
        ([], b'c') => "RIS (full reset)",
        ([], b'D') => "IND (index)",
        ([], b'E') => "NEL (next line)",
        ([], b'H') => "HTS (set tab stop)",
        ([], b'M') => "RI (reverse index)",
        ([], b'N') => "SS2",
        ([], b'O') => "SS3",
        ([], b'Z') => "DECID",
        ([], b'n') => "LS2",
        ([], b'o') => "LS3",
        ([], b'\\') => "ST",
        ([b'#'], b'3') => "DECDHL (double-height line, top half)",
        ([b'#'], b'4') => "DECDHL (double-height line, bottom half)",
        ([b'#'], b'5') => "DECSWL (single-width line)",
        ([b'#'], b'6') => "DECDWL (double-width line)",
        ([b'#'], b'8') => "DECALN (screen alignment test)",
        ([b' '], b'F') => "S7C1T (7-bit controls)",
        ([b' '], b'G') => "S8C1T (8-bit controls)",
        ([b'('], _) | ([b')'], _) | ([b'*'], _) | ([b'+'], _) => {
            let slot = inter.first().map_or('?', |&c| match c {
                b'(' => '0',
                b')' => '1',
                b'*' => '2',
                _ => '3',
            });
            return format!("designate G{slot} = {}", charset_name(b));
        }
        _ => return format!("ESC {}{}", escaped(inter), b as char),
    };
    known.to_string()
}

fn charset_name(b: u8) -> &'static str {
    match b {
        b'B' => "ASCII",
        b'0' => "DEC special graphics",
        b'A' => "UK",
        b'<' => "DEC supplemental",
        _ => "other",
    }
}

fn csi_name(csi: &Csi) -> String {
    let params = params_text(csi);
    let inter = csi.intermediates();
    let marker = csi.marker();
    let first = csi.get(0);
    let one = |what: &str| format!("{what} {params}").trim_end().to_string();
    let name = match (marker, inter, csi.final_byte()) {
        (None, [], b'@') => one("ICH"),
        (None, [], b'A') => one("CUU"),
        (None, [], b'B') => one("CUD"),
        (None, [], b'C') => one("CUF"),
        (None, [], b'D') => one("CUB"),
        (None, [], b'E') => one("CNL"),
        (None, [], b'F') => one("CPL"),
        (None, [], b'G') => one("CHA"),
        (None, [], b'H') => format!("CUP {}", if params.is_empty() { "1;1" } else { &params }),
        (None, [], b'I') => one("CHT"),
        (None, [], b'J') => format!("ED {} ({})", first.unwrap_or(0), ed_name(first)),
        (None, [], b'K') => format!("EL {} ({})", first.unwrap_or(0), el_name(first)),
        (None, [], b'L') => one("IL"),
        (None, [], b'M') => one("DL"),
        (None, [], b'P') => one("DCH"),
        (None, [], b'S') => one("SU"),
        (None, [], b'T') => one("SD"),
        (None, [], b'X') => one("ECH"),
        (None, [], b'Z') => one("CBT"),
        (None, [], b'`') => one("HPA"),
        (None, [], b'a') => one("HPR"),
        (None, [], b'b') => one("REP"),
        (None, [], b'c') => "DA1 (primary device attributes)".into(),
        (Some(b'>'), [], b'c') => "DA2 (secondary device attributes)".into(),
        (Some(b'='), [], b'c') => "DA3 (tertiary device attributes)".into(),
        (None, [], b'd') => one("VPA"),
        (None, [], b'e') => one("VPR"),
        (None, [], b'f') => one("HVP"),
        (None, [], b'g') => one("TBC"),
        (None, [], b'h') => format!("SM {params} (ANSI mode)"),
        (None, [], b'l') => format!("RM {params} (ANSI mode)"),
        (Some(b'?'), [], b'h') => mode_names("DECSET", csi),
        (Some(b'?'), [], b'l') => mode_names("DECRST", csi),
        (Some(b'?'), [], b's') => mode_names("XTSAVE", csi),
        (Some(b'?'), [], b'r') => mode_names("XTRESTORE", csi),
        (None, [], b'm') => {
            format!("SGR {} ({})", if params.is_empty() { "0" } else { &params }, sgr_text(csi))
        }
        (Some(b'>'), [], b'm') => one("XTMODKEYS"),
        (None, [], b'n') => format!("DSR {params} ({})", dsr_name(first, false)),
        (Some(b'?'), [], b'n') => format!("DECDSR {params} ({})", dsr_name(first, true)),
        (None, [], b'r') => format!("DECSTBM {}", if params.is_empty() { "(full screen)" } else { &params }),
        (None, [], b's') => {
            if csi.len() > 1 {
                one("DECSLRM")
            } else {
                "SCOSC / DECSLRM".into()
            }
        }
        (None, [], b't') => format!("XTWINOPS {params} ({})", winop_name(first)),
        (None, [], b'u') => "SCORC (restore cursor)".into(),
        (Some(b'>'), [], b'u') => one("kitty keyboard: push flags"),
        (Some(b'<'), [], b'u') => one("kitty keyboard: pop"),
        (Some(b'='), [], b'u') => one("kitty keyboard: set flags"),
        (Some(b'?'), [], b'u') => "kitty keyboard: query flags".into(),
        (None, [], b'x') => one("DECREQTPARM"),
        (None, [], b'i') => one("MC (media copy)"),
        (Some(b'>'), [], b'q') => "XTVERSION".into(),
        (None, [b' '], b'q') => format!("DECSCUSR {params} ({})", cursor_style_name(first)),
        (None, [b'!'], b'p') => "DECSTR (soft reset)".into(),
        (None, [b'"'], b'p') => one("DECSCL"),
        (None, [b'"'], b'q') => one("DECSCA"),
        (None, [b'$'], b'p') => format!("DECRQM {params} (ANSI mode)"),
        (Some(b'?'), [b'$'], b'p') => format!("DECRQM ?{params}"),
        (None, [b'$'], b'r') => one("DECCARA"),
        (None, [b'$'], b't') => one("DECRARA"),
        (None, [b'$'], b'v') => one("DECCRA"),
        (None, [b'$'], b'x') => one("DECFRA"),
        (None, [b'$'], b'z') => one("DECERA"),
        (None, [b'$'], b'{') => one("DECSERA"),
        (None, [b'*'], b'y') => one("DECRQCRA"),
        (None, [b'#'], b'{') | (None, [], b'{') => one("XTPUSHSGR"),
        (None, [b'#'], b'}') => one("XTPOPSGR"),
        (None, [b'\''], b'z') => one("DECELR"),
        (None, [b'\''], b'{') => one("DECSLE"),
        (None, [b'\''], b'|') => one("DECRQLP"),
        (None, [b'\''], b'w') => one("DECEFR"),
        (None, [b'\''], b'}') => one("DECIC"),
        (None, [b'\''], b'~') => one("DECDC"),
        _ => format!(
            "CSI {}{}{}{}",
            marker.map(|m| m as char).map(String::from).unwrap_or_default(),
            params,
            escaped(inter),
            csi.final_byte() as char
        ),
    };
    if csi.overflow() { format!("{name} (too many parameters, rest dropped)") } else { name }
}

/// The parameters as they were written: `1;31`, `38:2::10:20:30`.
fn params_text(csi: &Csi) -> String {
    let mut out = String::new();
    for (k, group) in csi.groups().enumerate() {
        if k > 0 {
            out.push(';');
        }
        for (j, value) in group.iter().enumerate() {
            if j > 0 {
                out.push(':');
            }
            let _ = write!(out, "{value}");
        }
    }
    // A lone empty parameter reads as none.
    if csi.len() == 1 && csi.get(0).is_none() { String::new() } else { out }
}

fn ed_name(p: Option<u16>) -> &'static str {
    match p.unwrap_or(0) {
        0 => "cursor to end of screen",
        1 => "start of screen to cursor",
        2 => "whole screen",
        3 => "history",
        _ => "?",
    }
}

fn el_name(p: Option<u16>) -> &'static str {
    match p.unwrap_or(0) {
        0 => "cursor to end of line",
        1 => "start of line to cursor",
        2 => "whole line",
        _ => "?",
    }
}

fn dsr_name(p: Option<u16>, private: bool) -> &'static str {
    match (private, p) {
        (false, Some(5)) => "status report",
        (false, Some(6)) => "cursor position report request",
        (true, Some(6)) => "DECXCPR",
        (true, Some(15)) => "printer status",
        (true, Some(25)) => "UDK status",
        (true, Some(26)) => "keyboard status",
        (true, Some(996)) => "color scheme query",
        _ => "?",
    }
}

fn winop_name(p: Option<u16>) -> &'static str {
    match p {
        Some(1) => "de-iconify",
        Some(2) => "iconify",
        Some(3) => "move window",
        Some(4) => "resize window in pixels",
        Some(5) => "raise",
        Some(6) => "lower",
        Some(7) => "refresh",
        Some(8) => "resize in cells",
        Some(9) => "maximize",
        Some(10) => "full screen",
        Some(11) => "report window state",
        Some(13) => "report window position",
        Some(14) => "report window size in pixels",
        Some(15) => "report screen size in pixels",
        Some(16) => "report cell size in pixels",
        Some(18) => "report size in cells",
        Some(19) => "report screen size in cells",
        Some(20) => "report icon label",
        Some(21) => "report title",
        Some(22) => "push title",
        Some(23) => "pop title",
        _ => "?",
    }
}

fn cursor_style_name(p: Option<u16>) -> &'static str {
    match p.unwrap_or(0) {
        0 | 1 => "blinking block",
        2 => "steady block",
        3 => "blinking underline",
        4 => "steady underline",
        5 => "blinking bar",
        6 => "steady bar",
        _ => "?",
    }
}

fn mode_names(name: &str, csi: &Csi) -> String {
    let mut out = format!("{name} {}", params_text(csi));
    let described: Vec<String> = csi
        .groups()
        .map(|g| g[0])
        .map(|m| mode_name(m).map_or_else(|| "unknown mode".into(), String::from))
        .collect();
    let _ = write!(out, " ({})", described.join(", "));
    out
}

fn mode_name(mode: u16) -> Option<&'static str> {
    Some(match mode {
        1 => "application cursor keys",
        3 => "132 columns",
        4 => "smooth scroll",
        5 => "reverse video",
        6 => "origin mode",
        7 => "autowrap",
        8 => "autorepeat",
        9 => "X10 mouse",
        12 => "blinking cursor",
        25 => "show cursor",
        40 => "allow 132 columns",
        45 => "reverse wrap",
        47 => "alternate screen",
        66 => "application keypad",
        69 => "left and right margins",
        80 => "sixel scrolling",
        1000 => "mouse press and release",
        1001 => "mouse highlight",
        1002 => "mouse drag",
        1003 => "mouse any motion",
        1004 => "focus events",
        1005 => "UTF-8 mouse",
        1006 => "SGR mouse",
        1007 => "alternate scroll",
        1015 => "urxvt mouse",
        1016 => "pixel mouse",
        1047 => "alternate screen",
        1048 => "save cursor",
        1049 => "alternate screen and save cursor",
        2004 => "bracketed paste",
        2026 => "synchronized output",
        2027 => "grapheme clusters",
        2031 => "color scheme updates",
        _ => return None,
    })
}

// ---- SGR ----

const COLORS: [&str; 8] = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"];

fn sgr_text(csi: &Csi) -> String {
    let groups: Vec<&[u16]> = csi.groups().collect();
    if groups.is_empty() || (groups.len() == 1 && csi.get(0).is_none()) {
        return "reset".into();
    }
    let mut parts = Vec::new();
    let mut k = 0;
    while k < groups.len() {
        let g = groups[k];
        k += 1;
        let p = g[0];
        let part = match p {
            0 => "reset".to_string(),
            1 => "bold".into(),
            2 => "faint".into(),
            3 => "italic".into(),
            4 => match g.get(1) {
                None | Some(1) => "underline".into(),
                Some(0) => "no underline".into(),
                Some(2) => "double underline".into(),
                Some(3) => "curly underline".into(),
                Some(4) => "dotted underline".into(),
                Some(5) => "dashed underline".into(),
                Some(_) => "underline style?".into(),
            },
            5 | 6 => "blink".into(),
            7 => "reverse".into(),
            8 => "hidden".into(),
            9 => "strikethrough".into(),
            21 => "double underline".into(),
            22 => "normal intensity".into(),
            23 => "no italic".into(),
            24 => "no underline".into(),
            25 => "no blink".into(),
            27 => "no reverse".into(),
            28 => "no hidden".into(),
            29 => "no strikethrough".into(),
            30..=37 => format!("fg {}", COLORS[(p - 30) as usize]),
            39 => "default fg".into(),
            40..=47 => format!("bg {}", COLORS[(p - 40) as usize]),
            49 => "default bg".into(),
            53 => "overline".into(),
            55 => "no overline".into(),
            59 => "default underline color".into(),
            90..=97 => format!("fg bright {}", COLORS[(p - 90) as usize]),
            100..=107 => format!("bg bright {}", COLORS[(p - 100) as usize]),
            38 | 48 | 58 => {
                let what = match p {
                    38 => "fg",
                    48 => "bg",
                    _ => "underline color",
                };
                // `38:5:n` and `38:2::r:g:b` are one group; `38;5;n` and `38;2;r;g;b` are several.
                let spec: Vec<u16> = if g.len() > 1 {
                    g[1..].to_vec()
                } else {
                    let mut rest = Vec::new();
                    let wanted = match groups.get(k).map(|g| g[0]) {
                        Some(5) => 2,
                        Some(2) => 4,
                        _ => 0,
                    };
                    for _ in 0..wanted {
                        if let Some(next) = groups.get(k) {
                            rest.push(next[0]);
                            k += 1;
                        }
                    }
                    rest
                };
                match spec.as_slice() {
                    [5, n] => format!("{what} palette {n}"),
                    [2, .., r, g, b] => format!("{what} rgb({r},{g},{b})"),
                    _ => format!("{what} ?"),
                }
            }
            _ => format!("unknown {p}"),
        };
        parts.push(part);
    }
    if parts.len() > 10 {
        parts.truncate(10);
        parts.push("…".into());
    }
    parts.join(", ")
}

// ---- strings ----

fn string_name(open: &Open) -> String {
    match open {
        Open::Osc(data, total) => osc_name(data, *total),
        Open::Dcs(header, data, total) => dcs_name(header, data, *total),
        Open::Apc(data, total) => apc_name(data, *total),
    }
}

fn osc_name(data: &[u8], total: usize) -> String {
    let (number, rest) = match data.iter().position(|&b| b == b';') {
        Some(at) => (&data[..at], &data[at + 1..]),
        None => (data, &data[data.len()..]),
    };
    let number = String::from_utf8_lossy(number).into_owned();
    let text = || shortened(&String::from_utf8_lossy(rest), 48);
    let name = match number.as_str() {
        "0" => format!("title and icon name {:?}", text()),
        "1" => format!("icon name {:?}", text()),
        "2" => format!("title {:?}", text()),
        "4" => "set palette color".into(),
        "7" => format!("working directory {:?}", text()),
        "8" => {
            let uri = rest.iter().position(|&b| b == b';').map_or(&rest[rest.len()..], |at| &rest[at + 1..]);
            if uri.is_empty() {
                "hyperlink end".into()
            } else {
                format!("hyperlink {:?}", shortened(&String::from_utf8_lossy(uri), 48))
            }
        }
        "9" if rest.starts_with(b"4;") => "progress".into(),
        "9" => format!("notification {:?}", text()),
        "10" => "foreground color".into(),
        "11" => "background color".into(),
        "12" => "cursor color".into(),
        "52" => "clipboard".into(),
        "99" => "kitty notification".into(),
        "104" => "reset palette color".into(),
        "110" => "reset foreground".into(),
        "111" => "reset background".into(),
        "112" => "reset cursor color".into(),
        "133" => match rest.first() {
            Some(b'A') => "shell integration: prompt start".into(),
            Some(b'B') => "shell integration: command start".into(),
            Some(b'C') => "shell integration: output start".into(),
            Some(b'D') => format!(
                "shell integration: command finished {}",
                shortened(&String::from_utf8_lossy(&rest[1..]), 16)
            )
            .trim_end()
            .into(),
            _ => "shell integration".into(),
        },
        "777" => "notification (rxvt)".into(),
        "1337" => {
            if rest.starts_with(b"File=") {
                "iTerm2 inline image".into()
            } else {
                format!("iTerm2 {:?}", shortened(&String::from_utf8_lossy(rest), 24))
            }
        }
        _ => "unknown".into(),
    };
    let more = if total > KEEP { format!(", {total} bytes") } else { String::new() };
    format!("OSC {number} {name}{more}")
}

fn dcs_name(header: &Csi, data: &[u8], total: usize) -> String {
    let inter = header.intermediates();
    let kind = match (header.marker(), inter, header.final_byte()) {
        (None, [], b'q') => format!("Sixel, {total} bytes"),
        (None, [b'$'], b'q') => format!("DECRQSS, request {:?}", String::from_utf8_lossy(data)),
        (None, [b'+'], b'q') => format!("XTGETTCAP, {} bytes", total),
        (None, [b'+'], b'p') => format!("XTSETTCAP, {} bytes", total),
        (None, [b'$'], b't') => format!("DECRSPS, {total} bytes"),
        (None, [b'|'], _) => "DECUDK".into(),
        _ => format!("{} bytes", total),
    };
    let params = params_text(header);
    format!("DCS {params}{}{} ({kind})", escaped(inter), header.final_byte() as char)
}

fn apc_name(data: &[u8], total: usize) -> String {
    match data.first() {
        Some(b'G') => {
            let keys = data[1..].iter().position(|&b| b == b';').map_or(&data[1..], |at| &data[1..1 + at]);
            format!("APC G kitty graphics {} ({total} bytes)", shortened(&String::from_utf8_lossy(keys), 60))
        }
        _ => format!("APC ({total} bytes)"),
    }
}

// ---- text ----

/// `text` cut to `max` characters in the middle.
fn shortened(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        return text.to_string();
    }
    let head = max * 2 / 3;
    let tail = max - head;
    format!(
        "{}…{}",
        chars[..head].iter().collect::<String>(),
        chars[chars.len() - tail..].iter().collect::<String>()
    )
}

/// Bytes as a person can read them: `\e`, `\r`, `\n`, `\xNN`; valid UTF-8 as itself.
fn escaped(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut rest = bytes;
    while !rest.is_empty() {
        let (valid, bad) = match std::str::from_utf8(rest) {
            Ok(text) => (text, 0),
            Err(err) => {
                let (good, _) = rest.split_at(err.valid_up_to());
                // `good` is valid by the error's own word.
                (std::str::from_utf8(good).unwrap_or(""), err.error_len().unwrap_or(rest.len() - good.len()))
            }
        };
        for c in valid.chars() {
            match c {
                '\x1b' => out.push_str("\\e"),
                '\r' => out.push_str("\\r"),
                '\n' => out.push_str("\\n"),
                '\t' => out.push_str("\\t"),
                '\x07' => out.push_str("\\a"),
                '\\' => out.push_str("\\\\"),
                c if c.is_control() => {
                    let _ = write!(out, "\\x{:02x}", c as u32);
                }
                c => out.push(c),
            }
        }
        let used = valid.len();
        for b in &rest[used..used + bad] {
            let _ = write!(out, "\\x{b:02x}");
        }
        rest = &rest[used + bad..];
    }
    out
}

fn text_name(raw: &[u8]) -> String {
    let text = String::from_utf8_lossy(raw);
    format!("text \"{}\" ({} chars)", escaped(shortened(&text, 40).as_bytes()), text.chars().count())
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
