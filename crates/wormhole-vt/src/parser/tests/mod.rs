//! Tests for the parser. `Recorder` turns what a handler hears into events that do not depend on
//! how the input was cut, so the same stream can be compared fed whole and fed in pieces.

mod csi;
mod ground;
mod property;
mod reference;
mod scan;
mod sequences;
mod spec;
mod strings;

use super::{Csi, Handler, MAX_INTERMEDIATES, MAX_PARAMS, Parser};
use std::fmt::Write as _;

/// `Terminal::feed` moves the terminal's parser out and back once per call, so the parser stays a
/// small value: no heap buffers, and about a hundred bytes.
#[test]
fn the_parser_is_small_enough_to_move_on_every_feed() {
    assert!(std::mem::size_of::<Parser>() <= 128);
}

/// How a string ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    St,
    Bel,
    Cancel,
    /// The input stopped before the end (only from `Recorder::finish`).
    Open,
}

/// What the parser reported, with text one `Print` per character and every string payload in one
/// piece, wherever the input was cut.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Print(char),
    Execute(u8),
    Csi(Csi),
    Esc(Vec<u8>, u8),
    Osc(Vec<u8>, End),
    Dcs(Csi, Vec<u8>, End),
    Apc(Vec<u8>, End),
}

#[derive(Default)]
pub struct Recorder {
    pub events: Vec<Event>,
    osc: Option<Vec<u8>>,
    dcs: Option<(Csi, Vec<u8>)>,
    apc: Option<Vec<u8>>,
    /// The `print_ascii` and `osc_put`-style calls, to see how the input was cut up.
    pub calls: Vec<String>,
}

/// What every `Csi` the parser hands out must satisfy (the fuzz target checks the same).
fn check_csi(csi: &Csi) {
    assert!(!csi.is_empty() && csi.values().len() <= MAX_PARAMS);
    assert!(matches!(csi.final_byte(), 0x40..=0x7E));
    assert!(csi.intermediates().len() <= MAX_INTERMEDIATES);
    assert!(csi.intermediates().iter().all(|b| (0x20..=0x2F).contains(b)));
    assert!(csi.marker().is_none_or(|m| b"<=>?".contains(&m)));
    assert_eq!(csi.groups().map(|g| g.len()).sum::<usize>(), csi.values().len());
    assert_eq!(csi.groups().count(), csi.len());
    assert_eq!(csi.has_subparams(), csi.groups().any(|g| g.len() > 1));
    for i in 0..csi.len() {
        assert_eq!(csi.param(i, 7), csi.get(i).unwrap_or(7));
        assert_eq!(csi.param_nonzero(i, 7), if csi.get(i).unwrap_or(0) == 0 { 7 } else { csi.param(i, 7) });
    }
    assert_eq!(csi.get(csi.len()), None);
}

impl Recorder {
    fn no_string_open(&self) {
        assert!(self.osc.is_none() && self.dcs.is_none() && self.apc.is_none(), "a string is still open");
    }

    /// Close strings the input left open, so that streams compare whole.
    pub fn finish(mut self) -> Vec<Event> {
        if let Some(data) = self.osc.take() {
            self.events.push(Event::Osc(data, End::Open));
        }
        if let Some((csi, data)) = self.dcs.take() {
            self.events.push(Event::Dcs(csi, data, End::Open));
        }
        if let Some(data) = self.apc.take() {
            self.events.push(Event::Apc(data, End::Open));
        }
        self.events
    }
}

impl Handler for Recorder {
    fn print_ascii(&mut self, run: &[u8]) {
        assert!(!run.is_empty(), "empty print_ascii");
        assert!(run.iter().all(|b| (0x20..=0x7E).contains(b)), "not printable ASCII: {run:?}");
        self.calls.push(format!("ascii {}", String::from_utf8_lossy(run)));
        self.events.extend(run.iter().map(|&b| Event::Print(b as char)));
    }
    fn print(&mut self, c: char) {
        assert!(c as u32 >= 0x80, "ASCII {c:?} through print");
        self.calls.push(format!("print {c:?}"));
        self.events.push(Event::Print(c));
    }
    fn execute(&mut self, byte: u8) {
        assert!(byte < 0x20 && byte != 0x1B, "not a C0 control: {byte:#x}");
        self.calls.push(format!("execute {byte:#04x}"));
        self.events.push(Event::Execute(byte));
    }
    fn csi(&mut self, csi: &Csi) {
        check_csi(csi);
        self.calls.push(format!("{csi:?}"));
        self.events.push(Event::Csi(csi.clone()));
    }
    fn esc(&mut self, intermediates: &[u8], byte: u8) {
        assert!(intermediates.len() <= MAX_INTERMEDIATES);
        assert!(intermediates.iter().all(|b| (0x20..=0x2F).contains(b)));
        assert!(matches!(byte, 0x30..=0x7E));
        self.calls.push(format!("esc {intermediates:?} {byte:#04x}"));
        self.events.push(Event::Esc(intermediates.to_vec(), byte));
    }
    fn osc_start(&mut self) {
        self.no_string_open();
        self.calls.push("osc_start".into());
        self.osc = Some(Vec::new());
    }
    fn osc_put(&mut self, data: &[u8]) {
        assert!(!data.is_empty(), "empty osc_put");
        assert!(data.iter().all(|&b| b >= 0x20), "a C0 byte in an OSC: {data:?}");
        self.calls.push(format!("osc_put {}", data.len()));
        self.osc.as_mut().expect("osc_put outside an OSC").extend_from_slice(data);
    }
    fn osc_end(&mut self, bel: bool) {
        let data = self.osc.take().expect("osc_end outside an OSC");
        self.calls.push("osc_end".into());
        self.events.push(Event::Osc(data, if bel { End::Bel } else { End::St }));
    }
    fn osc_cancel(&mut self) {
        let data = self.osc.take().expect("osc_cancel outside an OSC");
        self.calls.push("osc_cancel".into());
        self.events.push(Event::Osc(data, End::Cancel));
    }
    fn dcs_hook(&mut self, csi: &Csi) {
        check_csi(csi);
        self.no_string_open();
        self.calls.push(format!("dcs_hook {csi:?}"));
        self.dcs = Some((csi.clone(), Vec::new()));
    }
    fn dcs_put(&mut self, data: &[u8]) {
        assert!(!data.is_empty(), "empty dcs_put");
        assert!(!data.contains(&0x7F), "DEL in a DCS");
        self.calls.push(format!("dcs_put {}", data.len()));
        self.dcs.as_mut().expect("dcs_put outside a DCS").1.extend_from_slice(data);
    }
    fn dcs_unhook(&mut self) {
        let (csi, data) = self.dcs.take().expect("dcs_unhook outside a DCS");
        self.calls.push("dcs_unhook".into());
        self.events.push(Event::Dcs(csi, data, End::St));
    }
    fn dcs_cancel(&mut self) {
        let (csi, data) = self.dcs.take().expect("dcs_cancel outside a DCS");
        self.calls.push("dcs_cancel".into());
        self.events.push(Event::Dcs(csi, data, End::Cancel));
    }
    fn apc_start(&mut self) {
        self.no_string_open();
        self.calls.push("apc_start".into());
        self.apc = Some(Vec::new());
    }
    fn apc_put(&mut self, data: &[u8]) {
        assert!(!data.is_empty(), "empty apc_put");
        assert!(!data.contains(&0x7F), "DEL in an APC");
        self.calls.push(format!("apc_put {}", data.len()));
        self.apc.as_mut().expect("apc_put outside an APC").extend_from_slice(data);
    }
    fn apc_end(&mut self) {
        let data = self.apc.take().expect("apc_end outside an APC");
        self.calls.push("apc_end".into());
        self.events.push(Event::Apc(data, End::St));
    }
    fn apc_cancel(&mut self) {
        let data = self.apc.take().expect("apc_cancel outside an APC");
        self.calls.push("apc_cancel".into());
        self.events.push(Event::Apc(data, End::Cancel));
    }
}

/// Feed the pieces in turn and return the events (open strings closed as `End::Open`).
pub fn run_chunks(chunks: &[&[u8]]) -> Vec<Event> {
    let mut parser = Parser::new();
    let mut rec = Recorder::default();
    for chunk in chunks {
        parser.advance(&mut rec, chunk);
    }
    rec.finish()
}

/// Feed `bytes` cut at the given offsets (ascending, within the input).
pub fn run_cut(bytes: &[u8], cuts: &[usize]) -> Vec<Event> {
    let mut chunks = Vec::new();
    let mut from = 0;
    for &cut in cuts {
        chunks.push(&bytes[from..cut]);
        from = cut;
    }
    chunks.push(&bytes[from..]);
    run_chunks(&chunks)
}

pub fn run(bytes: &[u8]) -> Vec<Event> {
    run_chunks(&[bytes])
}

/// The calls the handler saw for `chunks`.
pub fn calls(chunks: &[&[u8]]) -> Vec<String> {
    let mut parser = Parser::new();
    let mut rec = Recorder::default();
    for chunk in chunks {
        parser.advance(&mut rec, chunk);
    }
    rec.calls
}

fn text(s: &str) -> String {
    format!("{s:?}")
}

fn payload(data: &[u8]) -> String {
    format!("{:?}", String::from_utf8_lossy(data))
}

/// Events as short strings, consecutive text merged: `"abc"`, `exec 0a`, `CSI 1;2 H`, `ESC ( B`,
/// `OSC "0;title" ST`, `DCS 1 q "data" ST`, `APC "Ga=q" ST`.
pub fn show(events: &[Event]) -> Vec<String> {
    let mut out = Vec::new();
    let mut run = String::new();
    for event in events {
        if let Event::Print(c) = event {
            run.push(*c);
            continue;
        }
        if !run.is_empty() {
            out.push(text(&std::mem::take(&mut run)));
        }
        out.push(match event {
            Event::Print(_) => unreachable!(),
            Event::Execute(b) => format!("exec {b:02x}"),
            Event::Csi(csi) => format!("{csi:?}"),
            Event::Esc(inter, b) => {
                let mut s = String::from("ESC");
                for &i in inter {
                    s.push_str(&if i == b' ' { " SP".to_string() } else { format!(" {}", i as char) });
                }
                let _ = write!(s, " {}", *b as char);
                s
            }
            Event::Osc(data, end) => format!("OSC {} {}", payload(data), end_name(*end)),
            Event::Dcs(csi, data, end) => {
                format!(
                    "{} {} {}",
                    format!("{csi:?}").replacen("CSI", "DCS", 1),
                    payload(data),
                    end_name(*end)
                )
            }
            Event::Apc(data, end) => format!("APC {} {}", payload(data), end_name(*end)),
        });
    }
    if !run.is_empty() {
        out.push(text(&run));
    }
    out
}

fn end_name(end: End) -> &'static str {
    match end {
        End::St => "ST",
        End::Bel => "BEL",
        End::Cancel => "CANCEL",
        End::Open => "OPEN",
    }
}

/// Assert what `input` reports, in `show`'s notation.
#[track_caller]
pub fn check(input: &[u8], expected: &[&str]) {
    assert_eq!(show(&run(input)), expected, "input {:?}", String::from_utf8_lossy(input));
}
