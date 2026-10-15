//! Where the specifications leave a choice open, and `parser/mod.rs` makes one: pinned here, each
//! with the reason, so that a change of mind shows up as a failing test and not as a surprise.

use super::{Event, calls, check, run, run_chunks, show};

#[test]
fn a_colon_separates_sub_parameters_in_a_dcs_header_too() {
    // Williams sends 0x3A to DCS Ignore; ECMA-48 5.4.2 b / T.416 make it a sub-parameter separator,
    // and a DCS header is written like a CSI.
    let events = run(b"\x1bP1:2;3qdata\x1b\\");
    let Event::Dcs(dcs, data, _) = &events[0] else { panic!("{events:?}") };
    assert_eq!(dcs.groups().collect::<Vec<_>>(), [&[1, 2][..], &[3][..]]);
    assert_eq!((dcs.final_byte(), data.as_slice()), (b'q', &b"data"[..]));
}

#[test]
fn st_ends_a_dcs_whose_header_is_not_finished() {
    // `ESC P` opened a control string (ECMA-48 5.6), so the `\` after the ESC is its ST.
    check(b"\x1bP1;2\x1b\\X", &["\"X\""]);
    check(b"\x1bP1 \x1b\\X", &["\"X\""]);
    // A CSI is no string: the same bytes there are an escape sequence.
    check(b"\x1b[1;2\x1b\\X", &["ESC \\", "\"X\""]);
}

#[test]
fn st_is_the_end_of_a_pm_or_sos_string_and_of_an_ignored_dcs() {
    check(b"\x1b^pm\x1b\\X", &["\"X\""]);
    check(b"\x1bXsos\x1b\\X", &["\"X\""]);
    check(b"\x1bP;;?q\x1b\\X", &["\"X\""]);
}

#[test]
fn the_backslash_of_st_may_follow_controls_but_nothing_else() {
    // Controls run inside an escape sequence, so they do not part the ESC from its `\`.
    check(b"\x1b]0;x\x1b\n\\y", &["OSC \"0;x\" ST", "exec 0a", "\"y\""]);
    // A second ESC is a new sequence of its own; the `\` after it is an ESC sequence.
    check(b"\x1b]0;x\x1b\x1b\\y", &["OSC \"0;x\" ST", "ESC \\", "\"y\""]);
    // DEL and the bytes of UTF-8 are ignored in an escape sequence.
    check(b"\x1b]0;x\x1b\x7f\\y", &["OSC \"0;x\" ST", "\"y\""]);
    // An intermediate makes it another sequence.
    check(b"\x1b]0;x\x1b(\\y", &["OSC \"0;x\" ST", "ESC ( \\", "\"y\""]);
}

#[test]
fn a_value_cut_between_calls_still_saturates() {
    let csi = |chunks: &[&[u8]]| match run_chunks(chunks).as_slice() {
        [Event::Csi(csi)] => csi.clone(),
        other => panic!("{other:?}"),
    };
    assert_eq!(csi(&[b"\x1b[6553", b"5H"]).values(), [65535]);
    assert_eq!(csi(&[b"\x1b[6553", b"6H"]).values(), [65535]);
    assert_eq!(csi(&[b"\x1b[99999", b"99999", b"99999H"]).values(), [65535]);
    assert_eq!(csi(&[b"\x1b[000", b"000", b"0012H"]).values(), [12]);
    // An empty value is still empty when the cut falls inside the sequence.
    let c = csi(&[b"\x1b[", b";", b"5", b";H"]);
    assert_eq!((c.len(), c.get(0), c.get(1), c.get(2)), (3, None, Some(5), None));
}

#[test]
fn every_c0_control_in_a_sequence_but_the_cancels_runs_it() {
    for b in (0x00u8..0x20).filter(|b| !matches!(b, 0x18 | 0x1A | 0x1B)) {
        check(&[0x1B, b'[', b'1', b, b'2', b'H'], &[&format!("exec {b:02x}"), "CSI 12 H"]);
    }
}

#[test]
fn the_csi_of_an_ignored_sequence_is_never_reported() {
    // Too late for a marker, or a parameter byte after an intermediate: skipped to its final byte.
    for ignored in [&b"\x1b[1?m"[..], b"\x1b[ 1m", b"\x1b[ ;m", b"\x1b[$?p", b"\x1b[;?m"] {
        let mut input = ignored.to_vec();
        input.push(b'Z');
        check(&input, &["\"Z\""]);
    }
}

#[test]
fn strings_leave_the_machine_in_the_ground_state() {
    // Every string ends in the ground state, however it ends (BEL only closes an OSC).
    let cases: [(&[u8], &[&[u8]]); 5] = [
        (b"\x1b]0;", &[b"\x07", b"\x1b\\", b"\x18", b"\x1a"]),
        (b"\x1bPq", &[b"\x1b\\", b"\x18", b"\x1a"]),
        (b"\x1b_G", &[b"\x1b\\", b"\x18", b"\x1a"]),
        (b"\x1b^", &[b"\x1b\\", b"\x18", b"\x1a"]),
        (b"\x1bX", &[b"\x1b\\", b"\x18", b"\x1a"]),
    ];
    for (opener, endings) in cases {
        for ending in endings {
            let mut input = opener.to_vec();
            input.extend_from_slice(ending);
            let mut parser = crate::parser::Parser::new();
            parser.advance(&mut super::Recorder::default(), &input);
            assert!(parser.is_ground(), "after {input:?}");
        }
    }
}

#[test]
fn text_is_split_only_by_what_interrupts_it() {
    // One `print_ascii` per run, however many characters follow before the next control; a multibyte
    // character is its own call; DEL splits a run without a call of its own.
    assert_eq!(calls(&[b"ab\x7fcd"]), ["ascii ab", "ascii cd"]);
    assert_eq!(calls(&["ab€cd".as_bytes()]), ["ascii ab", "print '€'", "ascii cd"]);
    assert_eq!(show(&run(b"ab\x7fcd")), ["\"abcd\""]);
}
