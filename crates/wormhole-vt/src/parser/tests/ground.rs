//! The ground state: printable ASCII runs, C0 controls, DEL and UTF-8.

use super::{Event, calls, check, run, run_chunks, show};
use crate::parser::Parser;

#[test]
fn printable_ascii_is_one_run() {
    let text: Vec<u8> = (0x20..=0x7E).cycle().take(1000).collect();
    assert_eq!(calls(&[&text]).len(), 1, "one print_ascii call for the whole input");
    check(b"hello, world", &["\"hello, world\""]);
}

#[test]
fn a_run_stops_at_controls_and_resumes() {
    assert_eq!(calls(&[b"abc\r\ndef"]), ["ascii abc", "execute 0x0d", "execute 0x0a", "ascii def"],);
}

#[test]
fn a_run_splits_where_the_calls_do() {
    assert_eq!(calls(&[b"abc", b"def"]), ["ascii abc", "ascii def"]);
}

#[test]
fn runs_of_every_length_and_position() {
    // The word-at-a-time scan has block, word and byte tails; a byte that is not printable
    // ASCII at each position of runs of every length must end the run exactly there.
    let text = |n: usize| if n == 0 { Vec::new() } else { vec![format!("{:?}", "x".repeat(n))] };
    for len in 0..100usize {
        for at in 0..=len {
            for stop in [0x00u8, 0x1F, 0x7F, 0x80, 0xFF] {
                let mut input = vec![b'x'; len];
                input.insert(at, stop);
                let expected = match stop {
                    0x00 | 0x1F => [text(at), vec![format!("exec {stop:02x}")], text(len - at)].concat(),
                    // Ignored, or a replacement character: the text around it is one run to `show`.
                    0x7F => text(len),
                    _ => vec![format!("{:?}", format!("{}\u{FFFD}{}", "x".repeat(at), "x".repeat(len - at)))],
                };
                assert_eq!(show(&run(&input)), expected, "{stop:#x} at {at} of {len}");
            }
        }
    }
}

#[test]
fn c0_controls_are_executed() {
    for b in (0x00u8..0x20).filter(|b| *b != 0x1B) {
        check(&[b], &[&format!("exec {b:02x}")]);
    }
}

#[test]
fn del_is_ignored() {
    check(b"a\x7fb", &["\"ab\""]);
    check(b"\x7f", &[]);
}

#[test]
fn can_and_sub_are_executed_in_ground() {
    check(b"a\x18b\x1ac", &["\"a\"", "exec 18", "\"b\"", "exec 1a", "\"c\""]);
}

#[test]
fn multibyte_characters() {
    check("é€😀".as_bytes(), &["\"é€😀\""]);
    check("日本語 text 한국어".as_bytes(), &["\"日本語 text 한국어\""]);
    // The first and last of each length, and the neighbours of the surrogate gap.
    for c in [
        '\u{80}',
        '\u{7FF}',
        '\u{800}',
        '\u{D7FF}',
        '\u{E000}',
        '\u{FFFD}',
        '\u{FFFF}',
        '\u{10000}',
        '\u{10FFFF}',
    ] {
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);
        check(s.as_bytes(), &[&format!("{:?}", c.to_string())]);
    }
}

#[test]
fn c1_code_points_are_text() {
    // U+009B encoded in UTF-8 is not a CSI.
    check("\u{9b}31m".as_bytes(), &["\"\\u{9b}31m\""]);
}

#[test]
fn raw_c1_bytes_are_replacement_characters() {
    check(&[0x9B, b'3', b'1', b'm'], &["\"\u{FFFD}31m\""]);
    check(&[0x90, 0x9D, 0x9C], &["\"\u{FFFD}\u{FFFD}\u{FFFD}\""]);
}

#[test]
fn malformed_utf8_is_one_replacement_per_maximal_subpart() {
    let cases: &[(&[u8], &str)] = &[
        (&[0x80], "\u{FFFD}"),
        (&[0xBF], "\u{FFFD}"),
        (&[0xC0, 0x80], "\u{FFFD}\u{FFFD}"),
        (&[0xC1, 0xBF], "\u{FFFD}\u{FFFD}"),
        (&[0xC2], "\u{FFFD}"),
        (&[0xC2, b'a'], "\u{FFFD}a"),
        (&[0xE0, 0x80, 0x80], "\u{FFFD}\u{FFFD}\u{FFFD}"),
        (&[0xE0, 0xA0], "\u{FFFD}"),
        (&[0xE0, 0xA0, b'a'], "\u{FFFD}a"),
        (&[0xED, 0xA0, 0x80], "\u{FFFD}\u{FFFD}\u{FFFD}"),
        (&[0xE2, 0x82, b'a'], "\u{FFFD}a"),
        (&[0xE2, b'a'], "\u{FFFD}a"),
        (&[0xF0, 0x80, 0x80, 0x80], "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}"),
        (&[0xF0, 0x9F, 0x98], "\u{FFFD}"),
        (&[0xF0, 0x9F, 0x98, b'a'], "\u{FFFD}a"),
        (&[0xF4, 0x90, 0x80, 0x80], "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}"),
        (&[0xF5, 0x80, 0x80, 0x80], "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}"),
        (&[0xFE, 0xFF], "\u{FFFD}\u{FFFD}"),
        (&[0xE2, 0x82, 0xE2, 0x82, 0xAC], "\u{FFFD}€"),
    ];
    for (input, expected) in cases {
        // A trailing newline settles a character that would otherwise wait for more bytes.
        let mut input = input.to_vec();
        input.push(b'\n');
        let mut want = vec![format!("{expected:?}")];
        want.push("exec 0a".into());
        assert_eq!(show(&run(&input)), want, "input {input:x?}");
    }
}

#[test]
fn a_cut_character_waits_for_the_rest() {
    let euro = "€".as_bytes();
    for cut in 1..euro.len() {
        let events = show(&run_chunks(&[&euro[..cut], &euro[cut..]]));
        assert_eq!(events, ["\"€\""], "cut at {cut}");
    }
    let emoji = "😀".as_bytes();
    for a in 1..emoji.len() {
        for b in a..emoji.len() {
            let events = show(&run_chunks(&[&emoji[..a], &emoji[a..b], &emoji[b..]]));
            assert_eq!(events, ["\"😀\""], "cut at {a}, {b}");
        }
    }
}

#[test]
fn a_cut_character_is_not_ground() {
    let mut parser = Parser::new();
    let mut rec = super::Recorder::default();
    parser.advance(&mut rec, &"€".as_bytes()[..2]);
    assert!(!parser.is_ground());
    parser.advance(&mut rec, &"€".as_bytes()[2..]);
    assert!(parser.is_ground());
}

#[test]
fn a_cut_character_broken_by_the_next_chunk() {
    // E2 82 then 'a': one replacement for the cut character, then the 'a'.
    check_chunks(&[&[0xE2, 0x82], b"abc"], &["\"\u{FFFD}abc\""]);
    // ... or by a control or an escape sequence, which are still looked at.
    check_chunks(&[&[0xE2, 0x82], b"\n"], &["\"\u{FFFD}\"", "exec 0a"]);
    check_chunks(&[&[0xF0, 0x9F], b"\x1b[m"], &["\"\u{FFFD}\"", "CSI m"]);
    // ... or by a second lead byte.
    check_chunks(&[&[0xE2], &[0xE2, 0x82, 0xAC]], &["\"\u{FFFD}€\""]);
    // ... or by a continuation byte that its lead does not allow (E0 wants A0..BF).
    check_chunks(&[&[0xE0], &[0x80, 0x80]], &["\"\u{FFFD}\u{FFFD}\u{FFFD}\""]);
}

#[test]
fn utf8_decoding_matches_the_standard_library() {
    // Every byte string of up to four bytes from the interesting lead, continuation and ASCII
    // bytes, against `from_utf8_lossy`, which also replaces per maximal subpart.
    let alphabet = [
        0x41u8, 0x7F, 0x80, 0x8F, 0x90, 0x9F, 0xA0, 0xBF, 0xC0, 0xC2, 0xDF, 0xE0, 0xE1, 0xED, 0xEE, 0xF0,
        0xF1, 0xF4, 0xF5, 0xFF,
    ];
    let mut inputs: Vec<Vec<u8>> = Vec::new();
    for &a in &alphabet {
        inputs.push(vec![a]);
        for &b in &alphabet {
            inputs.push(vec![a, b]);
            for &c in &alphabet {
                inputs.push(vec![a, b, c]);
                for &d in &alphabet {
                    inputs.push(vec![a, b, c, d]);
                }
            }
        }
    }
    for mut input in inputs {
        let want: Vec<char> = String::from_utf8_lossy(&input).chars().filter(|&c| c != '\x7F').collect();
        input.push(b'\n'); // settles a character that would wait for more bytes
        let mut got: Vec<char> = Vec::new();
        for event in run(&input) {
            match event {
                Event::Print(c) => got.push(c),
                Event::Execute(b'\n') => {}
                other => panic!("{other:?} for {input:x?}"),
            }
        }
        assert_eq!(got, want, "input {input:x?}");
    }
}

fn check_chunks(chunks: &[&[u8]], expected: &[&str]) {
    assert_eq!(show(&run_chunks(chunks)), expected, "chunks {chunks:x?}");
}
