//! OSC, DCS, APC, PM and SOS: terminators, aborts and streaming.

use super::{Parser, Recorder, calls, check, run, run_chunks, show};

// ---- OSC ----

#[test]
fn osc_ends_with_bel_or_st() {
    check(b"\x1b]0;title\x07", &["OSC \"0;title\" BEL"]);
    check(b"\x1b]0;title\x1b\\", &["OSC \"0;title\" ST"]);
    check(
        b"\x1b]8;;https://example.com\x1b\\link\x1b]8;;\x1b\\",
        &["OSC \"8;;https://example.com\" ST", "\"link\"", "OSC \"8;;\" ST"],
    );
    check(b"\x1b]\x07", &["OSC \"\" BEL"]);
    check(b"\x1b]\x1b\\", &["OSC \"\" ST"]);
}

#[test]
fn the_st_of_a_string_is_not_an_escape_sequence() {
    check(b"\x1b]0;x\x1b\\y", &["OSC \"0;x\" ST", "\"y\""]);
    // ... but any other byte after the ESC starts a sequence of its own.
    check(b"\x1b]0;x\x1b7y", &["OSC \"0;x\" ST", "ESC 7", "\"y\""]);
    check(b"\x1b]0;x\x1b[1my", &["OSC \"0;x\" ST", "CSI 1 m", "\"y\""]);
    check(b"\x1b]0;x\x1b\x1b\\", &["OSC \"0;x\" ST", "ESC \\"]);
}

#[test]
fn can_and_sub_cancel_an_osc() {
    check(b"\x1b]0;x\x18y", &["OSC \"0;x\" CANCEL", "exec 18", "\"y\""]);
    check(b"\x1b]52;c;AAAA\x1ay", &["OSC \"52;c;AAAA\" CANCEL", "exec 1a", "\"y\""]);
}

#[test]
fn c0_bytes_are_not_part_of_an_osc() {
    check(b"\x1b]0;a\x00b\nc\x1fd\x08e\x7ff\x07", &["OSC \"0;abcde\\u{7f}f\" BEL"]);
}

#[test]
fn osc_text_is_raw_bytes() {
    let events = run("\x1b]0;naïve 日本\x07".as_bytes());
    assert_eq!(show(&events), ["OSC \"0;naïve 日本\" BEL"]);
    // Not valid UTF-8 either: the handler gets the bytes.
    let events = run(b"\x1b]0;\xff\xfe\x07");
    assert!(matches!(&events[0], super::Event::Osc(data, _) if data == b"0;\xff\xfe"));
}

#[test]
fn an_osc_is_streamed_never_buffered() {
    // A megabyte payload arrives as slices of the input: the same memory, in one piece when it
    // came in one piece.
    let mut input = b"\x1b]1337;File=".to_vec();
    input.extend(std::iter::repeat_n(b'A', 1 << 20));
    input.push(0x07);
    let mut parser = Parser::new();
    let mut seen = Vec::new();
    struct Slices<'a>(&'a mut Vec<(usize, usize)>, *const u8);
    impl crate::parser::Handler for Slices<'_> {
        fn osc_put(&mut self, data: &[u8]) {
            self.0.push((data.as_ptr() as usize - self.1 as usize, data.len()));
        }
    }
    parser.advance(&mut Slices(&mut seen, input.as_ptr()), &input);
    assert_eq!(seen, [(2, input.len() - 3)]);
}

#[test]
fn an_osc_split_into_pieces_streams_each() {
    assert_eq!(
        calls(&[b"\x1b]0;ab", b"cd", b"ef\x07"]),
        ["osc_start", "osc_put 4", "osc_put 2", "osc_put 2", "osc_end"],
    );
    // A split inside the ST.
    assert_eq!(show(&run_chunks(&[b"\x1b]0;x\x1b", b"\\y"])), ["OSC \"0;x\" ST", "\"y\""]);
}

#[test]
fn an_unfinished_osc_is_open() {
    let mut parser = Parser::new();
    let mut rec = Recorder::default();
    parser.advance(&mut rec, b"\x1b]0;tit");
    assert!(!parser.is_ground());
    assert_eq!(show(&rec.finish()), ["OSC \"0;tit\" OPEN"]);
}

#[test]
fn osc_does_not_end_at_a_raw_c1_st() {
    let events = run(b"\x1b]0;a\x9cb\x07");
    assert!(matches!(&events[..], [super::Event::Osc(data, super::End::Bel)] if data == b"0;a\x9cb"));
}

// ---- DCS ----

#[test]
fn dcs_hook_data_unhook() {
    check(b"\x1bP1;2$qdata\x1b\\", &["DCS 1;2 $ q \"data\" ST"]);
    check(b"\x1bPq#0;2;0;0;0\x1b\\", &["DCS q \"#0;2;0;0;0\" ST"]);
    check(b"\x1bP+q544e\x1b\\", &["DCS + q \"544e\" ST"]);
    check(b"\x1bP>|\x1b\\", &["DCS > | \"\" ST"]);
    check(b"\x1bP1000p\x1b\\", &["DCS 1000 p \"\" ST"]);
}

#[test]
fn dcs_passes_c0_and_high_bytes_through_but_not_del() {
    let events = run(b"\x1bPq\x00\x07\n\x7f\xff\xc3\xa9\x7f\x1b\\");
    assert!(matches!(&events[0], super::Event::Dcs(_, data, _) if data == b"\x00\x07\n\xff\xc3\xa9"));
}

#[test]
fn dcs_ends_at_esc_and_can_cancels() {
    check(b"\x1bPqab\x1b[mc", &["DCS q \"ab\" ST", "CSI m", "\"c\""]);
    check(b"\x1bPqab\x18c", &["DCS q \"ab\" CANCEL", "exec 18", "\"c\""]);
    check(b"\x1bPqab\x1ac", &["DCS q \"ab\" CANCEL", "exec 1a", "\"c\""]);
}

#[test]
fn bel_does_not_end_a_dcs() {
    check(b"\x1bPqa\x07b\x1b\\", &["DCS q \"a\\u{7}b\" ST"]);
}

#[test]
fn a_dcs_header_swallows_controls_and_aborts_at_esc() {
    check(b"\x1bP1\n;2\rq\x1b\\", &["DCS 1;2 q \"\" ST"]);
    check(b"\x1bP1;2\x1b[mX", &["CSI m", "\"X\""]);
    check(b"\x1bP1;2\x18X", &["exec 18", "\"X\""]);
}

#[test]
fn a_malformed_dcs_header_is_ignored_up_to_st() {
    check(b"\x1bP1?q data \x1b\\X", &["\"X\""]);
    check(b"\x1bP1 1q data \x1b\\X", &["\"X\""]);
    // C0 controls are not executed in it either.
    check(b"\x1bP1?\n q \x1b\\X", &["\"X\""]);
    check(b"\x1bP1?q \x18X", &["exec 18", "\"X\""]);
}

#[test]
fn a_dcs_is_streamed() {
    assert_eq!(
        calls(&[b"\x1bPqab", b"cd\x1b\\"]),
        ["dcs_hook CSI q", "dcs_put 2", "dcs_put 2", "dcs_unhook"],
    );
}

// ---- APC ----

#[test]
fn apc_carries_the_kitty_protocol() {
    check(b"\x1b_Ga=T,f=100;AAAA\x1b\\", &["APC \"Ga=T,f=100;AAAA\" ST"]);
    check(b"\x1b_\x1b\\", &["APC \"\" ST"]);
    check(b"\x1b_Gx\x1b[1m", &["APC \"Gx\" ST", "CSI 1 m"]);
}

#[test]
fn apc_keeps_every_byte_but_esc_can_sub_and_del() {
    let events = run(b"\x1b_a\x00\x07\x7f\xff\x1b\\");
    assert!(matches!(&events[0], super::Event::Apc(data, _) if data == b"a\x00\x07\xff"));
    check(b"\x1b_Gx\x18y", &["APC \"Gx\" CANCEL", "exec 18", "\"y\""]);
    check(b"\x1b_Gx\x1ay", &["APC \"Gx\" CANCEL", "exec 1a", "\"y\""]);
}

#[test]
fn an_apc_is_streamed() {
    assert_eq!(calls(&[b"\x1b_Gab", b"cd", b"\x1b\\"]), ["apc_start", "apc_put 3", "apc_put 2", "apc_end"],);
}

// ---- PM and SOS ----

#[test]
fn pm_and_sos_are_consumed() {
    check(b"a\x1b^private message\x1b\\b", &["\"ab\""]);
    check(b"a\x1bXstring\x1b\\b", &["\"ab\""]);
    check(b"\x1bXab\x18c", &["exec 18", "\"c\""]);
    check(b"\x1b^ab\x1b[1mc", &["CSI 1 m", "\"c\""]);
    // BEL does not end one.
    check(b"\x1b^a\x07b\x1b\\c", &["\"c\""]);
}
