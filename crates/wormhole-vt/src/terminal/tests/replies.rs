//! The details of what the terminal answers.

use super::Rig;
use crate::terminal::Terminal;

#[test]
fn xtgettcap_answers_names_in_hex_one_reply_each() {
    let mut rig = Rig::new(20, 5);
    // TN, Co, then a name nobody has ("zz"), then RGB (a name whose value has a slash).
    rig.feed("\x1bP+q544e;436f;7a7a;524742\x1b\\");
    assert_eq!(
        rig.replies(),
        "\x1bP1+r544e=787465726D2D323536636F6C6F72\x1b\\\x1bP1+r436f=323536\x1b\\\x1bP0+r\x1b\\\x1bP1+r524742=382F382F38\x1b\\"
    );
    // A boolean has no value; a request that is not hex is refused.
    rig.feed("\x1bP+q5463;xyz;5\x1b\\");
    assert_eq!(rig.replies(), "\x1bP1+r5463\x1b\\\x1bP0+r\x1b\\\x1bP0+r\x1b\\");
}

#[test]
fn decrqss_describes_the_pen_as_the_sequence_that_sets_it() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b[3;4:3;9;38;2;1;2;3;48;5;17;58;5;200;91m\x1bP$qm\x1b\\");
    assert_eq!(rig.replies(), "\x1bP1$r0;3;4:3;9;91;48;5;17;58;5;200m\x1b\\");
    rig.feed("\x1b[0;21;53;7;47;38;5;9m\x1bP$qm\x1b\\");
    assert_eq!(rig.replies(), "\x1bP1$r0;21;7;53;38;5;9;47m\x1b\\");
    rig.feed("\x1b[24r\x1bP$qr\x1b\\\x1bP$q\"p\x1b\\\x1bP$qt\x1b\\\x1bP$q$|\x1b\\");
    assert_eq!(rig.replies(), "\x1bP1$r1;5r\x1b\\\x1bP1$r65;1\"p\x1b\\\x1bP1$r5t\x1b\\\x1bP1$r20$|\x1b\\");
    rig.feed("\x1bP$qq\x1b\\\x1bP$q\x1b\\");
    assert_eq!(rig.replies(), "\x1bP0$r\x1b\\\x1bP0$r\x1b\\");
}

#[test]
fn cursor_style_is_what_the_program_chose() {
    let mut rig = Rig::new(20, 5);
    let style = |rig: &Rig| {
        let c = rig.term.cursor();
        (c.shape, c.blinking)
    };
    use crate::types::CursorShape::{Beam, Block, Underline};
    assert_eq!(style(&rig), (None, None), "left to the host");
    for (ps, expected) in [
        (1, (Some(Block), Some(true))),
        (2, (Some(Block), Some(false))),
        (3, (Some(Underline), Some(true))),
        (4, (Some(Underline), Some(false))),
        (5, (Some(Beam), Some(true))),
        (6, (Some(Beam), Some(false))),
        (0, (None, None)),
    ] {
        rig.feed(&format!("\x1b[{ps} q"));
        assert_eq!(style(&rig), expected, "DECSCUSR {ps}");
    }
    rig.feed("\x1b[?12h");
    assert_eq!(style(&rig), (None, Some(true)), "mode 12 is the blink alone");
    rig.feed("\x1b[4 q\x1bP$q q\x1b\\");
    assert_eq!(rig.replies(), "\x1bP1$r4 q\x1b\\");
}

#[test]
fn window_reports_use_the_cell_size_the_host_gave() {
    let mut rig = Rig::new(10, 4);
    rig.term.set_cell_size(9, 18);
    rig.feed("\x1b[14t\x1b[16t\x1b[18t\x1b[19t");
    assert_eq!(rig.replies(), "\x1b[4;72;90t\x1b[6;18;9t\x1b[8;4;10t\x1b[9;4;10t");
    rig.feed("\x1b[?2;1S\x1b[?1;1S\x1b[?1;3S\x1b[?9S");
    assert_eq!(rig.replies(), "\x1b[?2;0;90;72S\x1b[?1;0;256S\x1b[?1;3;0S");
    // Asking for the window's title would put attacker text in the input: not answered.
    rig.feed("\x1b]2;secret\x07\x1b[21t\x1b[20t");
    assert_eq!(rig.replies(), "");
}

#[test]
fn every_mode_dec_knows_answers_something() {
    let mut rig = Rig::new(10, 4);
    let ask = |rig: &mut Rig, n: u16| {
        rig.feed(&format!("\x1b[?{n}$p"));
        rig.replies()
    };
    for n in [
        1, 3, 5, 6, 7, 8, 9, 12, 25, 40, 45, 66, 67, 69, 80, 95, 1000, 1002, 1003, 1004, 1005, 1006, 1007,
        1015, 1016, 1036, 2004, 2026, 2027,
    ] {
        let reply = ask(&mut rig, n);
        assert!(
            reply.starts_with(&format!("\x1b[?{n};")) && (reply.contains(";1$y") || reply.contains(";2$y")),
            "{n}: {reply:?}"
        );
    }
    assert_eq!(ask(&mut rig, 2), "\x1b[?2;1$y", "DECANM is set: the terminal that answers is in ANSI mode");
    assert_eq!(ask(&mut rig, 1001), "\x1b[?1001;4$y");
    assert_eq!(ask(&mut rig, 31337), "\x1b[?31337;0$y");
    // DECCOLM only counts when mode 40 allows it.
    rig.feed("\x1b[?3h");
    assert_eq!(ask(&mut rig, 3), "\x1b[?3;2$y");
    rig.feed("\x1b[?40h\x1b[?3h");
    assert_eq!(ask(&mut rig, 3), "\x1b[?3;1$y");
}

#[test]
fn device_status_reports_in_the_form_xterm_documents() {
    let mut rig = Rig::new(10, 4);
    rig.feed("\x1b[?15n\x1b[?25n\x1b[?26n\x1b[?53n\x1b[?75n\x1b[=c\x1b[>c\x1b[>q");
    let replies = rig.replies();
    assert!(
        replies
            .starts_with("\x1b[?13n\x1b[?20n\x1b[?27;1;0;0n\x1b[?53n\x1b[?70n\x1bP!|00000000\x1b\\\x1b[>64;"),
        "{replies:?}"
    );
    assert!(
        replies.ends_with(&format!("\x1bP>|wormhole-vt {}\x1b\\", env!("CARGO_PKG_VERSION"))),
        "{replies:?}"
    );
}

#[test]
fn xtversion_says_the_name_the_host_gave() {
    let mut term = Terminal::builder().identity("myterm", "2.5").build();
    term.feed(b"\x1b[>q");
    assert_eq!(term.replies(), b"\x1bP>|myterm 2.5\x1b\\");
}

#[test]
fn an_identity_cannot_carry_a_sequence_out_of_the_reply() {
    // The name is the host's, but a reply is a sequence the program reads: no controls in it,
    // and a bounded length.
    let mut term = Terminal::builder().identity("a\x1b\\b\x07", &"v".repeat(500)).build();
    term.feed(b"\x1b[>q");
    let reply = String::from_utf8_lossy(term.replies()).into_owned();
    let inner = reply.strip_prefix("\x1bP>|").and_then(|r| r.strip_suffix("\x1b\\")).expect("a DCS reply");
    assert!(inner.starts_with("a\\b "), "{inner:?}");
    assert!(!inner.chars().any(char::is_control), "{inner:?}");
    assert!(inner.len() <= 64 + 1 + 64 + 4, "{inner:?}");
}

#[test]
fn a_reply_buffer_is_kept_by_the_host() {
    let mut rig = Rig::new(10, 4);
    rig.feed("\x1b[5n");
    assert_eq!(rig.term.replies(), b"\x1b[0n");
    rig.feed("\x1b[5n");
    assert_eq!(rig.term.replies(), b"\x1b[0n\x1b[0n", "they add up until the host clears them");
    rig.term.clear_replies();
    assert!(rig.term.replies().is_empty());
}

#[test]
fn modes_can_be_saved_and_restored() {
    let mut rig = Rig::new(10, 4);
    rig.feed("\x1b[?2004h\x1b[?1h\x1b[?2004;1s\x1b[?2004l\x1b[?1l\x1b[?2004;1r\x1b[?2004$p\x1b[?1$p");
    assert_eq!(rig.replies(), "\x1b[?2004;1$y\x1b[?1;1$y");
    // What was never saved is left alone.
    rig.feed("\x1b[?25l\x1b[?25r\x1b[?25$p");
    assert_eq!(rig.replies(), "\x1b[?25;2$y");
    // Saved as reset, restored as reset.
    rig.feed("\x1b[?7s\x1b[?7l\x1b[?7r\x1b[?7$p");
    assert_eq!(rig.replies(), "\x1b[?7;1$y");
    rig.feed("\x1b[?1000s\x1b[?1000h\x1b[?1000r\x1b[?1000$p");
    assert_eq!(rig.replies(), "\x1b[?1000;2$y");
}

#[test]
fn the_terminal_can_cross_threads() {
    fn is_send<T: Send>() {}
    is_send::<crate::terminal::Terminal>();
    is_send::<crate::terminal::Event>();
}

#[test]
fn replies_past_the_cap_are_dropped_whole() {
    // A program asks for far more than the host collects: DA1 is written in pieces, and none may
    // be cut, or the rest of a sequence would reach the program as typed text.
    let mut rig = Rig::new(20, 5);
    rig.feed(&"\x1b[c".repeat(60_000));
    let replies = rig.replies();
    let one = replies.split_inclusive('c').next().unwrap_or_default().to_string();
    assert!(one.starts_with("\x1b[?") && one.len() > 5, "{one:?}");
    // The last one kept fitted under the cap whole; the next would not have.
    assert!(replies.len() <= 1 << 20 && replies.len() + one.len() > 1 << 20, "{}", replies.len());
    assert_eq!(replies.len() % one.len(), 0, "only whole replies");
    // Once the host collects them, replies come again.
    rig.feed("\x1b[5n");
    assert_eq!(rig.replies(), "\x1b[0n");
}

#[test]
fn a_state_report_with_one_bad_part_changes_nothing() {
    let mut rig = Rig::new(20, 4);
    rig.feed("\x1b[1$u");
    let before = rig.replies();
    // Good margins and modes, but a tab stop off the screen; then a column region one wide.
    rig.feed("\x1bP1$p2;3;1;C;@;@;0;2;@;BBBB!99!2;4;1;10!4!69\x1b\\\x1b[1$u");
    assert_eq!(rig.replies(), before);
    rig.feed("\x1bP1$p2;3;1;C;@;@;0;2;@;BBBB!5!2;4;3;3!4!69\x1b\\\x1b[1$u");
    assert_eq!(rig.replies(), before);
    // The same report, whole, is put back.
    rig.feed("\x1bP1$p2;3;1;C;@;@;0;2;@;BBBB!5!2;4;1;10!4!69\x1b\\\x1b[1$u");
    assert_eq!(rig.replies(), "\x1bP1$s2;3;1;C;@;@;0;2;@;BBBB!5!2;4;1;10!4!69\x1b\\");
}

#[test]
fn xtgettcap_answers_every_capability_it_lists_and_refuses_the_rest() {
    // xterm ctlseqs XTGETTCAP: DCS + q Pt ST asks for hex-encoded names; DCS 1 + r Pt = Pt ST answers
    // (the name and the value, hex), DCS 0 + r ST refuses.
    fn hex(text: &str) -> String {
        text.bytes().map(|b| format!("{b:02x}")).collect()
    }
    // The terminal's values come back in upper case, a name as it was sent.
    fn upper(text: &str) -> String {
        hex(text).to_uppercase()
    }
    let mut rig = Rig::new(20, 5);
    for (name, value) in [
        ("setrgbf", "\x1b[38;2;%p1%d;%p2%d;%p3%dm"),
        ("setrgbb", "\x1b[48;2;%p1%d;%p2%d;%p3%dm"),
        ("Smulx", "\x1b[4:%p1%dm"),
        ("Ss", "\x1b[%p1%d q"),
        ("Se", "\x1b[2 q"),
        ("Ms", "\x1b]52;%p1%s;%p2%s\x07"),
    ] {
        rig.feed(&format!("\x1bP+q{}\x1b\\", hex(name)));
        assert_eq!(rig.replies(), format!("\x1bP1+r{}={}\x1b\\", hex(name), upper(value)), "{name}");
    }
    rig.feed(&format!("\x1bP+q{}\x1b\\", hex("Setulc")));
    assert!(rig.replies().starts_with(&format!("\x1bP1+r{}=", hex("Setulc"))));
    rig.feed(&format!("\x1bP+q{};{}\x1b\\", hex("nosuchcap"), hex("Tc")));
    assert_eq!(rig.replies(), format!("\x1bP0+r\x1b\\\x1bP1+r{}\x1b\\", hex("Tc")), "a boolean has no value");
    rig.feed("\x1bP+qzz\x1b\\\x1bP+q4\x1b\\");
    assert_eq!(rig.replies(), "\x1bP0+r\x1b\\\x1bP0+r\x1b\\", "names that are not hex");
}

#[test]
fn the_save_cursor_mode_is_an_action_and_reads_as_reset() {
    // xterm ctlseqs DECRQM / mode 1048: save and restore the cursor; asking says reset (2).
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b[?1048$p");
    assert_eq!(rig.replies(), "\x1b[?1048;2$y");
}

#[test]
fn the_keyboard_status_report_depends_on_the_conformance_level() {
    // VT510 DECDSR 26 (keyboard): the VT200 family answers language only, VT300 adds the status,
    // VT400 and up add the type (xterm's DECSCL levels: 61 VT100 to 65 VT500).
    for (level, want) in [
        (62, "\x1b[?27;1n"),
        (63, "\x1b[?27;1;0n"),
        (64, "\x1b[?27;1;0;0n"),
        (65, "\x1b[?27;1;0;0n"),
        (61, "\x1b[?27;1n"),
    ] {
        let mut rig = Rig::new(20, 5);
        rig.feed(&format!("\x1b[{level};1\"p\x1b[?26n"));
        assert_eq!(rig.replies(), want, "level {level}");
    }
}

#[test]
fn secondary_device_attributes_name_the_family_of_the_level() {
    // xterm ctlseqs: CSI > c answers CSI > Pp ; Pv ; Pc c with Pp 0 VT100, 1 VT220, 24 VT320, 41 VT420, 64 VT520.
    for (level, family) in [(61, "0"), (62, "1"), (63, "24"), (64, "41"), (65, "64")] {
        let mut rig = Rig::new(20, 5);
        rig.feed(&format!("\x1b[{level};1\"p\x1b[>c"));
        let reply = rig.replies();
        assert!(reply.starts_with(&format!("\x1b[>{family};")), "level {level}: {reply:?}");
    }
}

#[test]
fn secondary_device_attributes_carry_an_xterm_patch_level() {
    // Programs compare DA2's version with xterm's patch numbers (esctest wants 314 to 999), so a
    // 1.x release must not send its own version there.
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b[>c");
    assert_eq!(rig.replies(), "\x1b[>64;777;0c");
}

#[test]
fn a_tab_stop_report_read_back_sets_those_stops_and_a_bad_one_changes_nothing() {
    // VT510 DECRSPS (DCS 2 $ t D...D ST): the tab stop report DECTABSR gives, columns joined by `/`;
    // a report with an invalid value is ignored (the manual: "the terminal ignores the rest").
    let mut rig = Rig::new(40, 3);
    let stops = |rig: &mut Rig| {
        let mut found = Vec::new();
        for _ in 0..4 {
            rig.feed("\t");
            found.push(rig.cursor().1 + 1);
        }
        rig.feed("\r");
        found
    };
    rig.feed("\x1b[3g\x1bP2$t5/12/30\x1b\\");
    assert_eq!(stops(&mut rig), [5, 12, 30, 40]);
    for bad in ["5/x", "0/5", "5/41", "5//7", "-3"] {
        rig.feed(&format!("\x1bP2$t{bad}\x1b\\"));
        assert_eq!(stops(&mut rig), [5, 12, 30, 40], "{bad}");
    }
    rig.feed("\x1bP2$t\x1b\\");
    assert_eq!(stops(&mut rig), [40, 40, 40, 40], "an empty report clears them all");
}
