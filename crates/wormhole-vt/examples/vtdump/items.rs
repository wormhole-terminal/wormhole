//! Finding the items of a stream: the parser is fed a byte at a time, and each sequence, control,
//! string and run of text is handed out with where it starts and ends.

use wormhole_vt::parser::{Csi, Handler, Parser};

use crate::names::{control_name, csi_name, esc_name};
use crate::strings::string_name;

/// What the parser reported, as values; the bytes it was reported for are known by position.
pub(crate) enum Ev {
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
pub(crate) struct Collect(Vec<Ev>);

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
pub(crate) struct Item {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) text: bool,
    pub(crate) name: String,
}

/// A control string being read.
pub(crate) enum Open {
    Osc(Vec<u8>, usize),
    Dcs(Csi, Vec<u8>, usize),
    Apc(Vec<u8>, usize),
}

/// How much of a payload is kept for the name; the rest is only counted.
pub(crate) const KEEP: usize = 256;

pub(crate) fn push_capped(into: &mut Vec<u8>, total: &mut usize, data: &[u8]) {
    *total += data.len();
    let room = KEEP.saturating_sub(into.len());
    into.extend_from_slice(&data[..data.len().min(room)]);
}

/// Hands out the items in order; printed characters are held back until the run is over.
pub(crate) struct Lister<F: FnMut(&Item)> {
    each: F,
    pub(crate) text: Option<Item>,
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

pub(crate) fn items_of(bytes: &[u8], each: impl FnMut(&Item)) {
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
