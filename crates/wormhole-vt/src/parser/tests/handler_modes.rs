//! What the parser does when the handler says it is in VT52 mode, or that 8-bit controls are on:
//! the two answers the terminal changes at run time.

use super::{Parser, Recorder, show};

/// What `bytes` report to a handler that answers as `recorder` is set up to.
fn run_as(recorder: Recorder, bytes: &[u8]) -> Vec<String> {
    let (mut parser, mut recorder) = (Parser::new(), recorder);
    parser.advance(&mut recorder, bytes);
    show(&recorder.finish())
}

fn vt52() -> Recorder {
    Recorder { vt52: true, ..Recorder::default() }
}

fn eight_bit() -> Recorder {
    Recorder { eight_bit: true, ..Recorder::default() }
}

#[test]
fn in_vt52_mode_cancel_and_substitute_end_the_escape_and_what_follows_is_text() {
    for cancel in [0x18u8, 0x1A] {
        assert_eq!(
            run_as(vt52(), &[0x1B, cancel, b'A']),
            [format!("exec {cancel:02x}"), "\"A\"".to_string()],
            "{cancel:#x}"
        );
    }
}

#[test]
fn in_vt52_mode_other_controls_act_inside_the_escape_and_an_escape_begins_it_over() {
    assert_eq!(run_as(vt52(), b"\x1b\nA"), ["exec 0a", "ESC A"], "the line feed acts, the escape waits");
    // An ESC is not a control to execute (the handler is never told of one).
    assert_eq!(run_as(vt52(), b"\x1b\x1bA"), ["ESC A"]);
    // Bytes of 0x7F and up are not part of an escape sequence here either.
    assert_eq!(run_as(vt52(), b"\x1b\x7fA"), ["ESC A"]);
}

#[test]
fn an_eight_bit_control_ends_a_malformed_csi_and_starts_its_own_sequence() {
    // CSI, an intermediate, then a parameter byte: malformed, ignored up to its final byte.
    let mut bytes = b"\x1b[$1".to_vec();
    // 0x9B is CSI as a single byte: with 8-bit controls on it ends the ignored sequence and begins
    // another, which the A finishes.
    bytes.extend([0x9B, b'A']);
    assert_eq!(run_as(eight_bit(), &bytes), ["CSI A"]);
    // As part of UTF-8 text (the handler said no) it is no control: the A ends the ignored one.
    assert_eq!(run_as(Recorder::default(), &bytes), Vec::<String>::new());
}

#[test]
fn in_vt52_mode_an_escape_that_ends_in_controls_waits_for_its_final_byte() {
    // The input stops inside the escape, after bytes that do not finish it.
    assert_eq!(run_as(vt52(), b"\x1b\n"), ["exec 0a"]);
    assert_eq!(run_as(vt52(), b"\x1b\x7f"), Vec::<String>::new());
    assert_eq!(run_as(vt52(), b"\x1b\x1b"), Vec::<String>::new());
}

#[test]
fn a_raw_c1_byte_where_a_sequence_goes_on_is_the_control_when_the_handler_asks_for_them() {
    // After an ESC, after an ESC and an intermediate, and after CSI and an intermediate: the 0x9B
    // is CSI and starts over, and the A ends that sequence. Otherwise the byte is no part of one.
    for (input, in_8_bit, otherwise) in [
        (&b"\x1b\x9bA"[..], "CSI A", "ESC A"),
        (b"\x1b \x9bA", "CSI A", "ESC SP A"),
        (b"\x1b[ \x9bA", "CSI A", "CSI SP A"),
    ] {
        assert_eq!(run_as(eight_bit(), input), [in_8_bit], "{input:x?} in 8-bit mode");
        assert_eq!(run_as(Recorder::default(), input), [otherwise], "{input:x?}");
    }
}

#[test]
fn a_lone_0x9c_ends_a_dcs_or_an_apc_and_a_0xa0_is_never_a_control() {
    assert_eq!(run_as(eight_bit(), b"\x1bPqdata\x9cX"), ["DCS q \"data\" ST", "\"X\""]);
    assert_eq!(run_as(eight_bit(), b"\x1b_Gdata\x9cX"), ["APC \"Gdata\" ST", "\"X\""]);
    // The C1 range stops at 0x9F: 0xA0 is only ever malformed UTF-8.
    assert_eq!(run_as(eight_bit(), &[0xA0]), ["\"\u{FFFD}\""]);
}

#[test]
fn the_bytes_that_continue_a_character_in_a_string_are_text_even_in_8_bit_mode() {
    // The emoji's second byte, 0x9F, is a C1 code on its own and nothing here.
    let emoji = "\u{1F600}";
    let osc = format!("\x1b]0;{emoji}\x07");
    assert_eq!(run_as(eight_bit(), osc.as_bytes()), [format!("OSC \"0;{emoji}\" BEL")]);
    let dcs = format!("\x1bPq{emoji}\x1b\\");
    assert_eq!(run_as(eight_bit(), dcs.as_bytes()), [format!("DCS q \"{emoji}\" ST")]);
    let apc = format!("\x1b_G{emoji}\x1b\\");
    assert_eq!(run_as(eight_bit(), apc.as_bytes()), [format!("APC \"G{emoji}\" ST")]);
}
