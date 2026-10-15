//! Properties over generated streams: cutting the input anywhere changes nothing, the real parser
//! agrees with the plain reference, and no byte string panics it.

use super::reference::{Reference, show_ref};
use super::{Event, Recorder, run, run_cut, show};
use crate::parser::{Csi, Parser};

// The one seeded generator the tests and examples share.
// The same file is included by the parser's and the grid's tests on purpose.
#[allow(clippy::duplicate_mod)]
#[path = "../../../tests/common/rng.rs"]
mod rng;
pub use rng::Rng;

fn push_text(rng: &mut Rng, out: &mut Vec<u8>) {
    // Runs short and long enough to cross the scan's 8 and 32 byte strides.
    let len = *[0, 1, 3, 7, 8, 9, 15, 31, 32, 33, 64, 100].get(rng.below(12)).unwrap_or(&5);
    for _ in 0..len {
        out.push(rng.range(0x20, 0x7E) as u8);
    }
}

fn push_char(rng: &mut Rng, out: &mut Vec<u8>) {
    let c = match rng.below(4) {
        0 => rng.range(0x80, 0x7FF) as u32,
        1 => rng.range(0x800, 0xFFFF) as u32,
        2 => rng.range(0x10000, 0x10FFFF) as u32,
        _ => rng.pick(&[0x80, 0x9B, 0x9C, 0x7FF, 0x800, 0xD7FF, 0xE000, 0xFFFD, 0x1F600, 0x10FFFF]),
    };
    if let Some(c) = char::from_u32(c) {
        out.extend_from_slice(c.to_string().as_bytes());
    }
}

fn push_params(rng: &mut Rng, out: &mut Vec<u8>) {
    if rng.chance(15) {
        out.push(rng.pick(b"<=>?"));
    }
    let n = if rng.chance(5) { rng.range(30, 40) } else { rng.range(0, 6) };
    for i in 0..n {
        if i > 0 {
            out.push(if rng.chance(20) { b':' } else { b';' });
        }
        let digits = rng.range(0, 7);
        for _ in 0..digits {
            out.push(b'0' + rng.below(10) as u8);
        }
    }
    // Garbage: a marker late, a colon run, bytes a sequence has no use for.
    if rng.chance(8) {
        out.push(rng.pick(b"<=>?:;"));
    }
    if rng.chance(8) {
        out.push(rng.pick(&[0x00, 0x07, 0x0A, 0x0D, 0x18, 0x1A, 0x1B, 0x7F, 0xC3, 0xA9, 0xFF]));
    }
    let inter = if rng.chance(15) { rng.range(1, 6) } else { 0 };
    for _ in 0..inter {
        out.push(rng.range(0x20, 0x2F) as u8);
    }
    if rng.chance(5) {
        out.push(rng.range(0x30, 0x3F) as u8);
    }
}

fn push_string_body(rng: &mut Rng, out: &mut Vec<u8>) {
    let len = rng.range(0, 60);
    for _ in 0..len {
        let b = match rng.below(20) {
            0 => rng.below(0x20) as u8,
            1 => rng.range(0x80, 0xFF) as u8,
            2 => 0x7F,
            _ => rng.range(0x20, 0x7E) as u8,
        };
        // Leave ESC, CAN and SUB to the terminator choice below.
        if !matches!(b, 0x1B | 0x18 | 0x1A) {
            out.push(b);
        }
    }
}

fn push_terminator(rng: &mut Rng, out: &mut Vec<u8>) {
    match rng.below(8) {
        0..=2 => out.extend_from_slice(b"\x1b\\"),
        3 | 4 => out.push(0x07),
        5 => out.push(rng.pick(&[0x18, 0x1A])),
        6 => out.push(0x1B),
        _ => {} // none: the next token runs on
    }
}

/// A stream of tokens that are mostly well-formed and sometimes not.
pub fn gen_stream(rng: &mut Rng) -> Vec<u8> {
    let mut out = Vec::new();
    for _ in 0..rng.range(1, 30) {
        match rng.below(14) {
            0..=2 => push_text(rng, &mut out),
            3 => out.push(rng.below(0x20) as u8),
            4 | 5 => push_char(rng, &mut out),
            6 => {
                let n = rng.range(1, 4);
                for _ in 0..n {
                    out.push(rng.range(0x80, 0xFF) as u8);
                }
            }
            7 | 8 => {
                out.extend_from_slice(b"\x1b[");
                push_params(rng, &mut out);
                out.push(rng.range(0x40, 0x7E) as u8);
            }
            9 => {
                out.push(0x1B);
                let n = rng.range(0, 6);
                for _ in 0..n {
                    out.push(rng.range(0x20, 0x2F) as u8);
                }
                out.push(
                    rng.pick(&[b'7', b'8', b'=', b'>', b'M', b'c', b'B', b'0', b'\\', 0x7F, 0xC3, 0x0A]),
                );
            }
            10 => {
                out.extend_from_slice(b"\x1b]");
                push_string_body(rng, &mut out);
                push_terminator(rng, &mut out);
            }
            11 => {
                out.extend_from_slice(b"\x1bP");
                push_params(rng, &mut out);
                out.push(rng.range(0x40, 0x7E) as u8);
                push_string_body(rng, &mut out);
                push_terminator(rng, &mut out);
            }
            12 => {
                out.push(0x1B);
                out.push(rng.pick(b"_X^"));
                push_string_body(rng, &mut out);
                push_terminator(rng, &mut out);
            }
            _ => {
                // A cut-off character, to be broken by whatever comes next.
                let mut ch = Vec::new();
                push_char(rng, &mut ch);
                let keep = rng.range(1, ch.len().max(2) - 1).min(ch.len());
                out.extend_from_slice(&ch[..keep]);
            }
        }
    }
    out
}

/// Bytes with no structure, but weighted towards the ones that matter.
pub fn gen_noise(rng: &mut Rng) -> Vec<u8> {
    let n = rng.range(0, 300);
    (0..n)
        .map(|_| match rng.below(6) {
            0 => 0x1B,
            1 => rng.pick(b"[];:?0123456789 \\_P]X^"),
            2 => rng.below(0x20) as u8,
            3 => rng.range(0x80, 0xFF) as u8,
            _ => rng.range(0x20, 0x7E) as u8,
        })
        .collect()
}

fn random_cuts(rng: &mut Rng, len: usize) -> Vec<usize> {
    let n = rng.range(1, 8);
    let mut cuts: Vec<usize> = (0..n).map(|_| rng.range(0, len)).collect();
    cuts.sort_unstable();
    cuts
}

#[track_caller]
fn assert_same(bytes: &[u8], cuts: &[usize], whole: &[Event]) {
    let pieces = run_cut(bytes, cuts);
    assert_eq!(show(&pieces), show(whole), "cut at {cuts:?} of {bytes:?}");
}

#[test]
fn cutting_the_input_anywhere_changes_nothing() {
    let mut rng = Rng::new(0x5EED_0001);
    for _ in 0..4000 {
        let bytes = if rng.chance(20) { gen_noise(&mut rng) } else { gen_stream(&mut rng) };
        let whole = run(&bytes);
        for _ in 0..4 {
            assert_same(&bytes, &random_cuts(&mut rng, bytes.len()), &whole);
        }
        // One byte at a time is the harshest cut.
        let every: Vec<usize> = (1..bytes.len()).collect();
        assert_same(&bytes, &every, &whole);
    }
}

#[test]
fn every_single_cut_of_a_stream() {
    let mut rng = Rng::new(0x5EED_0002);
    for _ in 0..300 {
        let bytes = gen_stream(&mut rng);
        let whole = run(&bytes);
        for at in 0..=bytes.len() {
            assert_same(&bytes, &[at], &whole);
        }
    }
}

#[test]
fn the_parser_agrees_with_the_plain_reference() {
    let mut rng = Rng::new(0x5EED_0003);
    for round in 0..6000 {
        let bytes = if round % 5 == 0 { gen_noise(&mut rng) } else { gen_stream(&mut rng) };
        let mut reference = Reference::new();
        reference.feed_all(&bytes);
        let want = show_ref(&reference.finish());
        let got = show(&run(&bytes));
        assert_eq!(got, want, "input {bytes:?}");
        // ... also fed in pieces.
        let pieces = show(&run_cut(&bytes, &random_cuts(&mut rng, bytes.len())));
        assert_eq!(pieces, want, "pieces of {bytes:?}");
    }
}

#[test]
fn no_byte_string_panics_the_parser() {
    let mut rng = Rng::new(0x5EED_0004);
    let mut parser = Parser::new();
    let mut rec = Recorder::default();
    for _ in 0..3000 {
        // One parser for all of them: whatever state a stream leaves behind, the next survives.
        let bytes = if rng.chance(50) { gen_noise(&mut rng) } else { gen_stream(&mut rng) };
        let mut from = 0;
        while from < bytes.len() {
            let to = (from + rng.range(1, 40)).min(bytes.len());
            parser.advance(&mut rec, &bytes[from..to]);
            from = to;
        }
        rec.events.clear();
        rec.calls.clear();
    }
    // Uniformly random bytes too.
    for _ in 0..2000 {
        let bytes: Vec<u8> = (0..rng.range(0, 400)).map(|_| rng.next_u64() as u8).collect();
        parser.advance(&mut rec, &bytes);
        rec.events.clear();
        rec.calls.clear();
    }
}

#[test]
fn every_two_byte_prefix_and_any_third() {
    // All of 0x00..=0xFF twice, after an ESC [, ESC ] and ESC P and in the ground state, with a
    // final byte after: exhaustive over the first decisions of every state.
    for intro in
        [&b""[..], b"\x1b", b"\x1b[", b"\x1b]", b"\x1bP", b"\x1b_", b"\x1b^", b"\x1b(", b"\x1b[1", b"\x1b[ "]
    {
        for a in 0..=255u8 {
            for b in 0..=255u8 {
                let mut bytes = intro.to_vec();
                bytes.extend_from_slice(&[a, b]);
                let mut reference = Reference::new();
                reference.feed_all(&bytes);
                assert_eq!(show(&run(&bytes)), show_ref(&reference.finish()), "input {bytes:?}");
            }
        }
    }
}

#[test]
fn csi_debug_is_written_as_typed() {
    let csi = |bytes: &[u8]| -> Csi {
        match run(bytes).as_slice() {
            [Event::Csi(c)] => c.clone(),
            other => panic!("{other:?}"),
        }
    };
    assert_eq!(format!("{:?}", csi(b"\x1b[?1;;38:2::1q")), "CSI ?1;;38:2::1 q");
    assert_eq!(format!("{:?}", csi(b"\x1b[m")), "CSI m");
}
