//! `Parser::advance` never allocates, whatever the input: a counting allocator watches a stream
//! with every kind of sequence, malformed ones, and a payload of several megabytes.
//!
//! Its own test binary, because the allocator is global and a test elsewhere must not count here.

use wormhole_vt::parser::{Csi, Handler, Parser};

#[path = "common/mod.rs"]
mod common;
use common::alloc::{Counting, allocations};

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Touches everything it is given, so that nothing is optimised away, and keeps nothing.
#[derive(Default)]
struct Sink(u64);

impl Handler for Sink {
    fn print_ascii(&mut self, run: &[u8]) {
        self.0 += run.len() as u64;
    }
    fn print(&mut self, c: char) {
        self.0 += c as u64;
    }
    fn execute(&mut self, byte: u8) {
        self.0 += byte as u64;
    }
    fn csi(&mut self, csi: &Csi) {
        self.0 += csi.values().iter().map(|&v| v as u64).sum::<u64>() + csi.groups().count() as u64;
    }
    fn esc(&mut self, intermediates: &[u8], byte: u8) {
        self.0 += intermediates.len() as u64 + byte as u64;
    }
    fn osc_put(&mut self, data: &[u8]) {
        self.0 += data.len() as u64;
    }
    fn dcs_hook(&mut self, csi: &Csi) {
        self.0 += csi.final_byte() as u64;
    }
    fn dcs_put(&mut self, data: &[u8]) {
        self.0 += data.len() as u64;
    }
    fn apc_put(&mut self, data: &[u8]) {
        self.0 += data.len() as u64;
    }
}

fn corpus() -> Vec<u8> {
    let mut out = Vec::new();
    for round in 0..200u32 {
        out.extend_from_slice(b"plain text, then ");
        let (r, g, b) = (round % 256, round * 7 % 256, round * 13 % 256);
        out.extend_from_slice(format!("\x1b[38;2;{r};{g};{b}m").as_bytes());
        out.extend_from_slice(
            b"\x1b[4:3m\x1b[38:2::1:2:3m\x1b[?1049h\x1b[2 q\x1b[1;2;3;4;5;6;7;8;9;10;11;12;13;14;15;16H",
        );
        out.extend_from_slice(&[b';'; 100]); // text
        out.extend_from_slice(
            b"\x1b[1;2;3;4;5;6;7;8;9;10;11;12;13;14;15;16;17;18;19;20;21;22;23;24;25;26;27;28;29;30;31;32;33;34m",
        );
        out.extend_from_slice(b"\x1b !\"#$%7\x1b(B\x1b]8;;https://example.com\x1b\\link\x1b]8;;\x07");
        out.extend_from_slice(b"\x1bP1;2$qdata\x1b\\\x1b_Gf=100;AAAA\x1b\\\x1b^pm\x1b\\\x1bXsos\x18");
        out.extend_from_slice("日本語 😀 é \u{9b}".as_bytes());
        out.extend_from_slice(&[0xE2, 0x82, 0xFF, 0xC0, 0x80, 0xF4, 0x90, 0x1B, 0x5B, 0x1B]);
        out.extend_from_slice(b"\r\n\x08\x07\x7f\x1a");
    }
    // A payload of megabytes inside one OSC and one APC.
    out.extend_from_slice(b"\x1b]1337;File=");
    out.extend(std::iter::repeat_n(b'A', 4 << 20));
    out.extend_from_slice(b"\x07\x1b_G");
    out.extend(std::iter::repeat_n(b'B', 4 << 20));
    out.extend_from_slice(b"\x1b\\");
    out
}

#[test]
fn advance_does_not_allocate() {
    let data = corpus();
    let mut parser = Parser::new();
    let mut sink = Sink::default();
    // Whole, in uneven pieces, and a byte at a time for the first part.
    let before = allocations();
    parser.advance(&mut sink, &data);
    for piece in data.chunks(777) {
        parser.advance(&mut sink, piece);
    }
    for byte in data[..20_000].chunks(1) {
        parser.advance(&mut sink, byte);
    }
    let after = allocations();
    assert_eq!(after - before, 0, "advance allocated");
    assert!(sink.0 > 0);
}
