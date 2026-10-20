//! Rarely used modes, reports and error paths: what the coverage run (`scripts/coverage.sh`) found
//! no test for. Each cites the text that says what must happen.

use super::Rig;
use crate::terminal::{
    Event, Fullscreen, Maximize, Progress, ProgressState, PromptKind, ShellMark, WindowOp, WindowOps,
};

fn events(rig: &mut Rig) -> Vec<Event> {
    rig.events()
}

#[test]
fn progress_has_a_paused_state_and_clamps_what_it_cannot_read() {
    // ConEmu's OSC 9;4: 0 remove, 1 normal, 2 error, 3 indeterminate, 4 paused (warning).
    let mut rig = Rig::new(20, 5);
    rig.feed(
        "\x1b]9;4;4;30\x07\x1b]9;4;1;abc\x07\x1b]9;4;1; 70 \x07\x1b]9;4;2;4000000000\x07\x1b]9;4;7;50\x07",
    );
    assert_eq!(
        events(&mut rig),
        [
            Event::Progress(Some(Progress { state: ProgressState::Paused, percent: 30 })),
            Event::Progress(Some(Progress { state: ProgressState::Normal, percent: 0 })),
            Event::Progress(Some(Progress { state: ProgressState::Normal, percent: 70 })),
            Event::Progress(Some(Progress { state: ProgressState::Error, percent: 100 })),
        ],
        "a state that does not exist is ignored"
    );
}

#[test]
fn the_other_prompt_kinds_do_not_start_a_block() {
    // OSC 133 (the FinalTerm / semantic prompts spec): A with k=i (default), s, r or c.
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b]133;A\x07$ \x1b]133;A;k=s\x07> \x1b]133;A;k=r\x07<\x1b]133;A;k=c\x07+\x1b]133;A;k=x\x07");
    assert_eq!(
        events(&mut rig),
        [
            Event::Mark(ShellMark::PromptStart(PromptKind::Initial)),
            Event::Mark(ShellMark::PromptStart(PromptKind::Secondary)),
            Event::Mark(ShellMark::PromptStart(PromptKind::Right)),
            Event::Mark(ShellMark::PromptStart(PromptKind::Continuation)),
            Event::Mark(ShellMark::PromptStart(PromptKind::Initial)),
        ]
    );
}

#[test]
fn synchronized_output_can_be_begun_and_ended_with_the_dcs_form() {
    // The synchronized output specification: DCS = 1 s ST begins an update, DCS = 2 s ST ends it.
    let mut rig = Rig::new(20, 5);
    assert!(!rig.term.synchronized_output());
    rig.feed("\x1bP=1s\x1b\\");
    assert!(rig.term.synchronized_output());
    rig.feed("\x1bP=3s\x1b\\");
    assert!(rig.term.synchronized_output(), "a number it does not have changes nothing");
    rig.feed("\x1bP=2s\x1b\\");
    assert!(!rig.term.synchronized_output());
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
fn maximize_and_full_screen_ask_for_each_of_their_forms() {
    // xterm ctlseqs XTWINOPS 9;0..3 (restore, both, vertically, horizontally) and 10;0..2 (undo, enter, toggle).
    let mut rig = Rig::new(20, 5);
    rig.term.set_window_ops(WindowOps::Allow);
    rig.feed("\x1b[9;0t\x1b[9;2t\x1b[9;3t\x1b[9;9t\x1b[10;0t\x1b[10;1t\x1b[10;2t\x1b[10;7t");
    let ops: Vec<WindowOp> = events(&mut rig)
        .into_iter()
        .filter_map(|e| if let Event::WindowOp(op) = e { Some(op) } else { None })
        .collect();
    assert_eq!(
        ops,
        [
            WindowOp::Maximize(Maximize::Restore),
            WindowOp::Maximize(Maximize::Vertical),
            WindowOp::Maximize(Maximize::Horizontal),
            WindowOp::Fullscreen(Fullscreen::Exit),
            WindowOp::Fullscreen(Fullscreen::Enter),
            WindowOp::Fullscreen(Fullscreen::Toggle),
        ]
    );
}

#[test]
fn the_screen_size_in_pixels_is_the_text_area_while_the_window_is_unknown() {
    // xterm ctlseqs XTWINOPS 15: report the size of the screen in pixels; before the host says, cells times cell size.
    let mut rig = Rig::new(10, 4);
    rig.term.set_cell_size(9, 18);
    rig.feed("\x1b[15t");
    assert_eq!(rig.replies(), "\x1b[5;72;90t");
}

#[test]
fn title_modes_come_back_to_the_plain_form() {
    // xterm ctlseqs: CSI > Ps T resets title modes 0 (set hex) .. 3 (query UTF-8); asking for UTF-8
    // (2, 3) is the plain form. Hex titles are decoded while mode 0 is set.
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b[>0t\x1b]2;616263\x07");
    assert_eq!(rig.term.title(), Some("abc"));
    rig.feed("\x1b[>2t\x1b]2;616263\x07");
    assert_eq!(rig.term.title(), Some("616263"), "UTF-8 is the plain text again");
    rig.feed("\x1b[>1t\x1b[>9t\x1b[>1;3t\x1b[>0T\x1b]2;x\x07");
    assert_eq!(rig.term.title(), Some("x"));
}

#[test]
fn a_title_stack_that_overflows_drops_the_oldest() {
    // xterm ctlseqs XTWINOPS 22/23: push and pop titles; the stack is bounded.
    let mut rig = Rig::new(20, 5);
    for n in 0..40 {
        rig.feed(&format!("\x1b]2;t{n}\x07\x1b[22;2t"));
    }
    rig.events();
    rig.feed("\x1b]2;now\x07");
    let mut popped = Vec::new();
    for _ in 0..40 {
        rig.feed("\x1b[23;2t");
        if let Some(title) = rig.term.title() {
            popped.push(title.to_string());
        }
    }
    assert_eq!(popped.first().map(String::as_str), Some("t39"), "the newest comes back first");
    assert!(popped.iter().any(|t| t == "t39") && !popped.iter().any(|t| t == "t0"), "t0 fell off the bottom");
}

#[test]
fn special_colors_go_back_to_the_hosts_one_by_one_or_all_at_once() {
    // xterm ctlseqs OSC 4 (256..: the special colors), OSC 5, OSC 104 (reset entries, also 256..),
    // OSC 105 (reset special colors).
    let mut rig = Rig::new(20, 5);
    let ask = |rig: &mut Rig| {
        rig.feed("\x1b]4;256;?\x07\x1b]4;257;?\x07\x1b]4;258;?\x07");
        rig.replies()
    };
    let before = ask(&mut rig);
    rig.feed("\x1b]4;256;rgb:11/22/33;257;rgb:44/55/66\x07\x1b]5;2;rgb:77/88/99\x07");
    let set = ask(&mut rig);
    assert_ne!(set, before);
    rig.feed("\x1b]104;256\x07");
    let one = ask(&mut rig);
    assert!(one.contains("rgb:4444/5555/6666") && one.contains("rgb:7777/8888/9999"));
    assert!(!one.contains("rgb:1111/2222/3333"), "{one:?}");
    rig.feed("\x1b]105;1\x07");
    assert!(!ask(&mut rig).contains("rgb:4444"));
    rig.feed("\x1b]104;999;x\x07\x1b]105;99\x07");
    assert!(ask(&mut rig).contains("rgb:7777"), "numbers it does not have change nothing");
    rig.feed("\x1b]105\x07");
    assert_eq!(ask(&mut rig), before);
    rig.feed("\x1b]4;256;rgb:11/22/33\x07\x1b]104\x07");
    assert_eq!(ask(&mut rig), before, "OSC 104 without a list resets the special colors too");
}

#[test]
fn a_decolm_request_is_one_for_the_host_and_resets_to_80_when_it_is_left() {
    // xterm ctlseqs mode 3 (DECCOLM) needs mode 40; the terminal asks the host for 132 columns.
    let mut rig = Rig::new(100, 5);
    rig.feed("\x1b[?40h\x1b[?3h");
    assert!(events(&mut rig).contains(&Event::ColumnsRequested(132)));
    rig.feed("\x1b[?3l");
    assert!(events(&mut rig).contains(&Event::ColumnsRequested(80)));
}

#[test]
fn every_kind_of_bracket_finds_its_partner_across_nesting_and_rows() {
    use crate::types::{Column, Line, Point};
    let mut rig = Rig::new(10, 3);
    rig.feed("a(b[c{d<e>f}g]h)i\r\n<x>");
    let at = |line: i32, column: usize| Point::new(Line(line), Column(column));
    let grid = rig.term.grid();
    // Row 0 is `a(b[c{d<e>`, row 1 is `f}g]h)i`, wrapped at ten columns.
    assert_eq!(grid.bracket_match(at(0, 1)), Some(at(1, 5)), "( to its ) over every other kind");
    assert_eq!(grid.bracket_match(at(1, 5)), Some(at(0, 1)));
    assert_eq!(grid.bracket_match(at(0, 3)), Some(at(1, 3)), "[ to ] on the next row");
    assert_eq!(grid.bracket_match(at(1, 3)), Some(at(0, 3)));
    assert_eq!(grid.bracket_match(at(0, 5)), Some(at(1, 1)), "curly braces");
    assert_eq!(grid.bracket_match(at(1, 1)), Some(at(0, 5)));
    assert_eq!(grid.bracket_match(at(0, 7)), Some(at(0, 9)), "< to >");
    assert_eq!(grid.bracket_match(at(0, 9)), Some(at(0, 7)));
    assert_eq!(grid.bracket_match(at(0, 0)), None, "not a bracket");
    assert_eq!(grid.bracket_match(at(2, 0)), Some(at(2, 2)));
}

#[test]
fn a_bracket_with_no_partner_finds_none() {
    use crate::types::{Column, Line, Point};
    let mut rig = Rig::new(10, 3);
    rig.feed("(((  )]>");
    let at = |column: usize| Point::new(Line(0), Column(column));
    let grid = rig.term.grid();
    assert_eq!(grid.bracket_match(at(0)), None, "the outer ( has no ) left");
    assert_eq!(grid.bracket_match(at(1)), None);
    assert_eq!(grid.bracket_match(at(2)), Some(at(5)));
    assert_eq!(grid.bracket_match(at(6)), None, "] has no [");
    assert_eq!(grid.bracket_match(at(7)), None, "> has no <");
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

#[test]
fn with_8_bit_controls_a_title_keeps_its_utf8_and_still_ends_at_st() {
    // S8C1T, then a title whose characters have bytes of 0x80..=0x9F inside them (— is E2 80 94,
    // œ is C5 9C), ended by the one-byte ST, then text: cut anywhere, it reads the same.
    let mut bytes = b"\x1b G\x1b]2;a \xe2\x80\x94 \xc5\x93 \xe2\x80\x9c\x9c".to_vec();
    bytes.extend_from_slice(b"X");
    for cut in 0..=bytes.len() {
        let mut rig = Rig::new(20, 3);
        rig.feed_bytes(&bytes[..cut]).feed_bytes(&bytes[cut..]);
        assert_eq!(rig.term.title(), Some("a \u{2014} \u{153} \u{201c}"), "cut at {cut}");
        assert_eq!(rig.row(0), "X", "cut at {cut}");
    }
}
