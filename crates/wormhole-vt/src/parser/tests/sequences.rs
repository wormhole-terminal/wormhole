//! ESC sequences and how the state machine moves between sequences.

use super::{Parser, Recorder, calls, check, run, show};

#[test]
fn single_byte_sequences() {
    check(b"\x1b7", &["ESC 7"]);
    check(b"\x1b8", &["ESC 8"]);
    check(b"\x1b=", &["ESC ="]);
    check(b"\x1b>", &["ESC >"]);
    check(b"\x1bM", &["ESC M"]);
    check(b"\x1bc", &["ESC c"]);
    check(b"\x1bD\x1bE\x1bH", &["ESC D", "ESC E", "ESC H"]);
}

#[test]
fn every_final_byte_that_is_not_an_introducer() {
    for b in 0x30u8..=0x7E {
        let introducer = matches!(b, b'[' | b']' | b'P' | b'X' | b'^' | b'_');
        let events = run(&[0x1B, b]);
        if introducer {
            assert!(!show(&events).iter().any(|e| e.starts_with("ESC")), "{:?}", b as char);
        } else {
            assert_eq!(show(&events), [format!("ESC {}", b as char)], "{:?}", b as char);
        }
    }
}

#[test]
fn intermediates() {
    check(b"\x1b(B", &["ESC ( B"]);
    check(b"\x1b)0", &["ESC ) 0"]);
    check(b"\x1b#8", &["ESC # 8"]);
    check(b"\x1b F", &["ESC SP F"]);
    check(b"\x1b%G", &["ESC % G"]);
    check(b"\x1b$)C", &["ESC $ ) C"]);
    // After an intermediate every byte up to 0x7E is a final byte, introducers included.
    check(b"\x1b([", &["ESC ( ["]);
    check(b"\x1b(]", &["ESC ( ]"]);
}

#[test]
fn too_many_intermediates_are_dropped() {
    check(b"\x1b !\"#7X", &["ESC SP ! \" # 7", "\"X\""]);
    check(b"\x1b !\"#$7X", &["\"X\""]);
    check(b"\x1b !\"#$%&7X", &["\"X\""]);
    // The next sequence is clean.
    check(b"\x1b !\"#$7\x1b(B", &["ESC ( B"]);
}

#[test]
fn escape_then_escape_starts_over() {
    check(b"\x1b\x1b7", &["ESC 7"]);
    check(b"\x1b(\x1b7", &["ESC 7"]);
    check(b"\x1b( \x1b[m", &["CSI m"]);
}

#[test]
fn controls_run_inside_an_escape_sequence() {
    check(b"\x1b\n7", &["exec 0a", "ESC 7"]);
    check(b"\x1b(\r\nB", &["exec 0d", "exec 0a", "ESC ( B"]);
}

#[test]
fn can_and_sub_cancel_an_escape_sequence() {
    check(b"\x1b\x187", &["exec 18", "\"7\""]);
    check(b"\x1b(\x1a7", &["exec 1a", "\"7\""]);
}

#[test]
fn del_and_high_bytes_are_ignored_in_an_escape_sequence() {
    check(b"\x1b\x7f7", &["ESC 7"]);
    check(b"\x1b\xc3\xa97", &["ESC 7"]);
    check(b"\x1b(\x7fB", &["ESC ( B"]);
}

#[test]
fn a_lone_backslash_is_an_escape_sequence() {
    check(b"\x1b\\", &["ESC \\"]);
}

#[test]
fn sequences_back_to_back_and_with_text() {
    check(
        b"ab\x1b[1mcd\x1b(Bef\x1b]0;t\x07gh",
        &["\"ab\"", "CSI 1 m", "\"cd\"", "ESC ( B", "\"ef\"", "OSC \"0;t\" BEL", "\"gh\""],
    );
}

#[test]
fn the_parser_is_ground_between_sequences() {
    let mut parser = Parser::new();
    let mut rec = Recorder::default();
    assert!(parser.is_ground());
    for (chunk, ground) in [
        (&b"abc"[..], true),
        (b"\x1b", false),
        (b"7", true),
        (b"\x1b(", false),
        (b"B", true),
        (b"\x1b]0;x", false),
        (b"\x07", true),
        (b"\x1bP1q", false),
        (b"\x1b\\", true),
        (b"\x1b_Ga", false),
        (b"\x1b", false),
        (b"\\", true),
        (b"\x1b^zz", false),
        (b"\x18", true),
    ] {
        parser.advance(&mut rec, chunk);
        assert_eq!(parser.is_ground(), ground, "after {chunk:?}");
    }
}

#[test]
fn an_empty_chunk_changes_nothing() {
    assert_eq!(calls(&[b"", b"ab", b"", b"c", b""]), ["ascii ab", "ascii c"]);
    let mut parser = Parser::new();
    let mut rec = Recorder::default();
    parser.advance(&mut rec, b"\x1b[1");
    parser.advance(&mut rec, b"");
    parser.advance(&mut rec, b";2H");
    assert_eq!(show(&rec.finish()), ["CSI 1;2 H"]);
}
