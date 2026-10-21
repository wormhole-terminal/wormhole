//! Replies are well-formed and carry no program text.

use wormhole_vt::event::Terminator;
use wormhole_vt::parser::{Csi, Handler, Parser};
use wormhole_vt::{Event, Policy, Terminal};

use crate::generator::*;
use crate::rng::Rng;

/// What is wrong with a reply, found by reading it the way a program does: nothing is allowed in it
/// but whole sequences made of printable text. Empty when it is as it should be.
#[derive(Default)]
struct Strict {
    faults: Vec<String>,
    sequences: usize,
}

impl Strict {
    fn text(&mut self, what: &str, data: &[u8]) {
        for c in String::from_utf8_lossy(data).chars() {
            if c.is_control() {
                self.faults.push(format!("{what} holds {c:?}"));
            }
        }
    }
}

impl Handler for Strict {
    fn print_ascii(&mut self, run: &[u8]) {
        self.faults.push(format!("text outside a sequence: {}", show(run)));
    }
    fn print(&mut self, c: char) {
        self.faults.push(format!("character outside a sequence: {c:?}"));
    }
    fn execute(&mut self, byte: u8) {
        self.faults.push(format!("control {byte:#04x} outside a sequence"));
    }
    fn csi(&mut self, csi: &Csi) {
        self.sequences += 1;
        if csi.overflow() {
            self.faults.push("a sequence with too many parameters".into());
        }
    }
    fn esc(&mut self, intermediates: &[u8], byte: u8) {
        self.faults.push(format!("a bare ESC sequence {} {}", show(intermediates), show(&[byte])));
    }
    fn osc_put(&mut self, data: &[u8]) {
        self.text("an OSC string", data);
    }
    fn osc_end(&mut self, _bel_terminated: bool) {
        self.sequences += 1;
    }
    fn dcs_hook(&mut self, _dcs: &Csi) {
        self.sequences += 1;
    }
    fn dcs_put(&mut self, data: &[u8]) {
        self.text("a DCS string", data);
    }
    fn apc_put(&mut self, data: &[u8]) {
        self.text("an APC string", data);
    }
    fn osc_cancel(&mut self) {
        self.faults.push("a cancelled OSC".into());
    }
    fn dcs_cancel(&mut self) {
        self.faults.push("a cancelled DCS".into());
    }
    fn apc_cancel(&mut self) {
        self.faults.push("a cancelled APC".into());
    }
}

/// Panics when `reply` is not made of whole, plain sequences (it may be empty).
fn assert_well_formed(reply: &[u8], request: &str) {
    let mut strict = Strict::default();
    let mut parser = Parser::new();
    parser.advance(&mut strict, reply);
    if !parser.is_ground() {
        strict.faults.push("ends inside a sequence".into());
    }
    // Raw bytes too: the parser drops what it ignores (DEL, C1 bytes inside a string), and an
    // 8-bit terminal reads those as controls.
    match std::str::from_utf8(reply) {
        Ok(text) => {
            if text.chars().any(|c| c.is_control() && !matches!(c, '\x1b' | '\x07')) {
                strict.faults.push("a control character (C0 other than ESC and BEL, DEL or C1)".into());
            }
        }
        Err(_) => strict.faults.push("not valid UTF-8".into()),
    }
    assert!(strict.faults.is_empty(), "reply {} to {request}: {:?}", show(reply), strict.faults);
}

/// Text made to hurt: C1 and C0 controls as characters, quotes, separators and a marker that must
/// never come back.
fn hostile_text(rng: &mut Rng, len: usize) -> String {
    const MARK: &str = "ZZTOPZZ";
    let mut out = String::new();
    for _ in 0..len {
        match rng.below(8) {
            0 => out.push(char::from_u32(0x80 + rng.below(0x20) as u32).unwrap_or('?')),
            1 => out.push(rng.pick(&['"', '\\', ';', ':', '?', '$', '\'', '`', '%', ' ', '!', '/'])),
            2 => out.push_str(MARK),
            3 => out.push(rng.pick(&['\u{7f}', '\u{2028}', '\u{202e}', '\u{200b}', '\u{feff}', 'é', '日'])),
            _ => out.push(rng.pick(&['a', 'm', 'r', 'q', 's', '0', '1', '2', 'x', 'Z'])),
        }
    }
    out
}

const MARK: &[u8] = b"ZZTOPZZ";

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Requests built from hostile text, each fed to a terminal that has been told what the host
/// allows; the replies must be well-formed, and (for those that must not repeat the request) free of
/// the marker.
fn hostile_requests(from: u64, count: u64) {
    for seed in from..from + count {
        let mut rng = Rng::new(seed);
        let mut open = Policy::default();
        open.window_ops = true;
        open.screen_checksum = true;
        for (policy, reports_titles) in [(Policy::default(), false), (open, true)] {
            let mut term = Terminal::builder().size(30, 8).policy(policy).build();
            for _ in 0..30 {
                let text = {
                    let n = rng.range(1, 40);
                    hostile_text(&mut rng, n)
                };
                // (the request, whether the marker may come back in the reply)
                let (request, may_repeat): (String, bool) = match rng.below(9) {
                    0 => (
                        format!(
                            "\x1b]{};{text}\x1b\\\x1b[2{}t",
                            rng.pick(&["0", "1", "2"]),
                            rng.pick(&["0", "1"])
                        ),
                        reports_titles,
                    ),
                    1 => (format!("\x1bP$q{text}\x1b\\"), false),
                    2 => (
                        format!("\x1b]{};?;{text}\x07", rng.pick(&["4;1", "10", "11", "12", "4;255"])),
                        false,
                    ),
                    3 => (format!("\x1b]4;{text};?\x1b\\"), false),
                    4 => (format!("\x1bP+q{}\x1b\\", hex(text.as_bytes())), true),
                    5 => (format!("\x1b_Ga=q,i={},s=1,v=1,f=24;{text}\x1b\\", rng.below(100)), false),
                    6 => (
                        format!(
                            "\x1b[?{text}$p\x1b[{text}$p\x1b[{};{};{};{};{}*y",
                            rng.below(9),
                            rng.below(3),
                            rng.below(9),
                            rng.below(9),
                            rng.below(9)
                        ),
                        false,
                    ),
                    // The title is set with its hex form: the characters it decodes to can be anything.
                    7 => {
                        let raw: Vec<u8> = (0..rng.range(1, 30))
                            .map(|_| {
                                rng.pick(&[0x1b, 0x0d, 0x0a, 0x9b, 0x07, 0x00, 0xc2, 0x85, b'a', b'\\', b'm'])
                            })
                            .collect();
                        (format!("\x1b[>0t\x1b]2;{}\x1b\\\x1b[21t\x1b[>2t", hex(&raw)), reports_titles)
                    }
                    _ => (
                        format!("\x1b[>0;1t\x1b]2;{}\x1b\\\x1b[21t\x1b[20t\x1b[>2;3t", hex(text.as_bytes())),
                        true,
                    ),
                };
                term.feed(request.as_bytes());
                let reply = term.take_replies();
                assert_well_formed(&reply, &format!("seed {seed}: {}", show(request.as_bytes())));
                if !may_repeat {
                    assert!(
                        !contains(&reply, MARK) && !contains(&reply, hex(MARK).as_bytes()),
                        "seed {seed}: the request {} came back in {}",
                        show(request.as_bytes()),
                        show(&reply)
                    );
                }
                if !reports_titles && (request.contains("[21t") || request.contains("[20t")) {
                    assert!(
                        reply.is_empty(),
                        "title reports are off but the reply to {} is {}",
                        show(request.as_bytes()),
                        show(&reply)
                    );
                }
                let _ = term.events().count();
            }
        }
    }
}

#[test]
fn replies_to_hostile_requests_are_well_formed_and_carry_no_request_text() {
    hostile_requests(0, 200);
}

/// The clipboard answer is the host's, but it repeats the selection name the program sent.
#[test]
fn a_clipboard_query_with_a_hostile_selection_is_answered_in_one_piece() {
    for seed in 0..60u64 {
        let mut rng = Rng::new(seed);
        let mut term = Terminal::new(20, 5);
        let name = {
            let n = rng.range(0, 10);
            hostile_text(&mut rng, n)
        };
        term.feed(format!("\x1b]52;{name};?\x1b\\").as_bytes());
        let loads: Vec<(String, Terminator)> = term
            .events()
            .filter_map(|event| match event {
                Event::ClipboardLoad { selection, terminator, .. } => Some((selection, terminator)),
                _ => None,
            })
            .collect();
        for (selection, terminator) in loads {
            term.clipboard_reply(&selection, "text from the clipboard", terminator);
            let reply = term.take_replies();
            assert_well_formed(&reply, &format!("seed {seed}: OSC 52 with selection {name:?}"));
        }
    }
}

/// Whatever generated stream is fed, whatever the terminal answers is well-formed.
fn every_reply_is_well_formed(from: u64, count: u64, len: usize) {
    for seed in from..from + count {
        let (cols, rows) = (6 + (seed as usize % 7) * 11, 3 + (seed as usize % 5) * 5);
        let bytes = stream(seed, len, cols, rows, HOSTILE);
        let mut term = terminal(cols, rows);
        for (n, piece) in bytes.chunks(64).enumerate() {
            term.feed(piece);
            let reply = term.take_replies();
            assert_well_formed(&reply, &format!("seed {seed}, piece {n}: {}", show(piece)));
            let _ = term.events().count();
        }
    }
}

#[test]
fn every_reply_to_a_generated_stream_is_well_formed() {
    every_reply_is_well_formed(0, 200, 8_000);
}

#[test]
#[ignore = "many more seeds: cargo test -p wormhole-vt --test invariants -- --ignored"]
fn replies_many_seeds() {
    hostile_requests(1_000, 1_500);
    every_reply_is_well_formed(10_000, 1_500, 20_000);
}
