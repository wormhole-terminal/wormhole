//! The details of what the terminal answers.

use super::Rig;

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
fn colors_are_the_hosts_until_a_program_changes_them() {
    use crate::types::{COLOR_COUNT, NamedColor, Rgb};
    let mut rig = Rig::new(10, 4);
    let mut palette = [Rgb::default(); COLOR_COUNT];
    palette[1] = Rgb { r: 0x11, g: 0x22, b: 0x33 };
    palette[NamedColor::Foreground as usize] = Rgb { r: 0xaa, g: 0xbb, b: 0xcc };
    palette[NamedColor::Cursor as usize] = Rgb { r: 1, g: 2, b: 3 };
    rig.term.set_default_colors(&palette);
    rig.feed("\x1b]4;1;?\x07\x1b]10;?\x1b\\\x1b]12;?\x07");
    assert_eq!(
        rig.replies(),
        "\x1b]4;1;rgb:1111/2222/3333\x07\x1b]10;rgb:aaaa/bbbb/cccc\x1b\\\x1b]12;rgb:0101/0202/0303\x07"
    );
    // Set two at once with 10, ask for the second; set a palette entry with a short form.
    rig.feed("\x1b]10;#ff0000;rgb:0/f/0\x07\x1b]11;?\x07\x1b]4;1;#00f;2;?\x07");
    assert_eq!(rig.replies(), "\x1b]11;rgb:0000/ffff/0000\x07\x1b]4;2;rgb:0000/0000/0000\x07");
    assert_eq!(rig.term.colors()[NamedColor::Foreground], Some(Rgb { r: 255, g: 0, b: 0 }));
    assert_eq!(rig.term.colors()[1], Some(Rgb { r: 0, g: 0, b: 0xf0 }), "#00f is the top bits of the byte");
    // Resets.
    rig.feed("\x1b]104;1\x07\x1b]110\x07\x1b]111\x07");
    assert_eq!(rig.term.colors()[1], None);
    assert_eq!(rig.term.colors()[NamedColor::Foreground], None);
    assert_eq!(rig.term.colors()[NamedColor::Background], None);
    rig.feed("\x1b]4;5;rgb:1/2/3;6;rgb:4/5/6\x07\x1b]104\x07");
    assert_eq!(rig.term.colors()[5], None);
    assert_eq!(rig.term.colors()[6], None);
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
    assert_eq!(ask(&mut rig, 2), "\x1b[?2;3$y", "VT52 mode is permanently off: always ANSI");
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
        replies.ends_with(&format!("\x1bP>|Wormhole {}\x1b\\", env!("CARGO_PKG_VERSION"))),
        "{replies:?}"
    );
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
    is_send::<crate::term::Terminal>();
    is_send::<crate::term::Event>();
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
    // The last one kept began under the cap.
    assert!(replies.len() >= 1 << 20 && replies.len() < (1 << 20) + one.len(), "{}", replies.len());
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
