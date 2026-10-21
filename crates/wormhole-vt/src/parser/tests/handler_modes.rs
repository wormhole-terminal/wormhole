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
