#![no_main]

use libfuzzer_sys::fuzz_target;
use wormhole_vt::parser::{Csi, Handler, MAX_INTERMEDIATES, MAX_PARAMS, Parser};

/// What the handler heard, with text one `Print` per character and every string payload in one
/// piece, wherever the input was cut. Checks that what it is given is well formed.
#[derive(Default)]
struct Recorder {
    /// Whether C1 controls are one byte (S8C1T); a handler chooses.
    eight: bool,
    events: Vec<String>,
    string: Option<(&'static str, Vec<u8>)>,
}

impl Recorder {
    fn closed(&mut self, end: &str) {
        let (kind, data) = self.string.take().expect("a string ended that never started");
        self.events.push(format!("{kind} {data:?} {end}"));
    }

    fn check(csi: &Csi) {
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
        }
    }
}

impl Handler for Recorder {
    fn eight_bit_controls(&self) -> bool {
        self.eight
    }
    fn print_ascii(&mut self, run: &[u8]) {
        assert!(!run.is_empty() && run.iter().all(|b| (0x20..=0x7E).contains(b)));
        self.events.extend(run.iter().map(|&b| format!("p{}", b as char)));
    }
    fn print(&mut self, c: char) {
        assert!(c as u32 >= 0x80);
        self.events.push(format!("p{c}"));
    }
    fn execute(&mut self, byte: u8) {
        assert!(byte < 0x20 && byte != 0x1B);
        self.events.push(format!("x{byte}"));
    }
    fn csi(&mut self, csi: &Csi) {
        Self::check(csi);
        self.events.push(format!("{csi:?}"));
    }
    fn esc(&mut self, intermediates: &[u8], byte: u8) {
        assert!(intermediates.len() <= MAX_INTERMEDIATES);
        assert!(intermediates.iter().all(|b| (0x20..=0x2F).contains(b)));
        assert!(matches!(byte, 0x30..=0x7E));
        self.events.push(format!("esc {intermediates:?} {byte}"));
    }
    fn osc_start(&mut self) {
        assert!(self.string.is_none());
        self.string = Some(("osc", Vec::new()));
    }
    fn osc_put(&mut self, data: &[u8]) {
        assert!(!data.is_empty() && data.iter().all(|&b| b >= 0x20));
        self.string.as_mut().expect("osc_put outside").1.extend_from_slice(data);
    }
    fn osc_end(&mut self, bel: bool) {
        self.closed(if bel { "bel" } else { "st" });
    }
    fn osc_cancel(&mut self) {
        self.closed("cancel");
    }
    fn dcs_hook(&mut self, csi: &Csi) {
        assert!(self.string.is_none());
        Self::check(csi);
        self.events.push(format!("hook {csi:?}"));
        self.string = Some(("dcs", Vec::new()));
    }
    fn dcs_put(&mut self, data: &[u8]) {
        assert!(!data.is_empty() && !data.contains(&0x7F));
        self.string.as_mut().expect("dcs_put outside").1.extend_from_slice(data);
    }
    fn dcs_unhook(&mut self) {
        self.closed("st");
    }
    fn dcs_cancel(&mut self) {
        self.closed("cancel");
    }
    fn apc_start(&mut self) {
        assert!(self.string.is_none());
        self.string = Some(("apc", Vec::new()));
    }
    fn apc_put(&mut self, data: &[u8]) {
        assert!(!data.is_empty() && !data.contains(&0x7F));
        self.string.as_mut().expect("apc_put outside").1.extend_from_slice(data);
    }
    fn apc_end(&mut self) {
        self.closed("st");
    }
    fn apc_cancel(&mut self) {
        self.closed("cancel");
    }
}

/// Hears nothing: the parser alone must cope with any stream, however it is cut.
struct Silent(bool);

impl Handler for Silent {
    fn eight_bit_controls(&self) -> bool {
        self.0
    }
}

fn run(eight: bool, chunks: &[&[u8]]) -> Vec<String> {
    let mut parser = Parser::new();
    let mut rec = Recorder { eight, ..Recorder::default() };
    for chunk in chunks {
        parser.advance(&mut rec, chunk);
    }
    if let Some((kind, data)) = rec.string.take() {
        rec.events.push(format!("{kind} {data:?} open"));
    }
    rec.events
}

fuzz_target!(|data: &[u8]| {
    // The first byte is how many cut points follow, those bytes are the cuts, the rest is the stream.
    let Some((&count, rest)) = data.split_first() else {
        return;
    };
    // Bit 7 of the first byte is the handler's choice of 8-bit controls.
    let eight = count & 0x80 != 0;
    let count = (count as usize & 0x7f) % 8;
    let count = count.min(rest.len());
    let (cuts, stream) = rest.split_at(count);
    let mut at: Vec<usize> = cuts.iter().map(|&c| c as usize % (stream.len() + 1)).collect();
    at.sort_unstable();

    let mut silent = Parser::new();
    silent.advance(&mut Silent(eight), stream);
    let whole = run(eight, &[stream]);

    let mut pieces: Vec<&[u8]> = Vec::new();
    let mut from = 0;
    for &cut in &at {
        pieces.push(&stream[from..cut]);
        from = cut;
    }
    pieces.push(&stream[from..]);
    assert_eq!(run(eight, &pieces), whole, "cut at {at:?}");

    let bytes: Vec<&[u8]> = stream.chunks(1).collect();
    assert_eq!(run(eight, &bytes), whole, "one byte at a time");
});
