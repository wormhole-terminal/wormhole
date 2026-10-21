//! The repaint on small, chosen cases: each feeds a stream to a terminal, repaints it into a fresh
//! one and compares the two's whole state. The bisimulation over the corpora is in
//! `wormhole-vt-bench`; these say which part of the state broke when it fails.

use crate::terminal::Terminal;

use super::Gaps;

/// Feed `input` to a terminal, repaint it into another and return both, after checking that the
/// repaint produced no reply and that the states are the same but for what the repaint says it
/// left out.
fn repaint(input: &[u8], cols: usize, rows: usize) -> (Terminal, Terminal, Gaps) {
    let mut a = Terminal::builder().size(cols, rows).scrollback(100).build();
    a.feed(input);
    a.clear_replies();
    let mut bytes = Vec::new();
    let done = a.repaint(&mut bytes).expect("ready");
    // A receiver that holds something else is made the same: that is what the leading RIS is for.
    let mut b = Terminal::builder().size(cols, rows).scrollback(100).build();
    b.feed(b"\x1b[31mjunk\x1b[?1049h\x1b[5;5Hmore\x1b]2;title\x07");
    b.seed(&done.seeds);
    b.feed(&bytes);
    assert!(b.replies().is_empty(), "the repaint made the receiver reply: {:?}", b.replies());
    let (da, db) = (a.dump_state(), b.dump_state());
    assert!(
        da.same(&db),
        "gaps {:?}\nrepaint: {:?}\n{}",
        done.gaps,
        String::from_utf8_lossy(&bytes),
        da.diff(&db, 6)
    );
    (a, b, done.gaps)
}

fn same(input: &str) {
    let (_, _, gaps) = repaint(input.as_bytes(), 20, 6);
    assert!(gaps.is_empty(), "left out: {gaps:?}");
}

#[test]
fn an_empty_terminal() {
    same("");
}

#[test]
fn text_and_a_cursor() {
    same("hello\r\nworld\x1b[2;3H");
}

#[test]
fn history_and_a_screen_that_scrolled() {
    let mut text = String::new();
    for i in 0..30 {
        text.push_str(&format!("line {i}\r\n"));
    }
    same(&text);
}

#[test]
fn colors_and_attributes() {
    same("\x1b[1;31mred\x1b[0m \x1b[48;2;1;2;3mbg\x1b[4:3;58;5;9mcurl\x1b[m\r\n\x1b[38;5;200;7mx\x1b[m");
}

#[test]
fn a_soft_wrap_and_a_hard_line_end() {
    same("0123456789012345678901234\r\nabcdefghijklmnopqrst\r\nz");
}

#[test]
fn wide_characters_and_clusters() {
    same("日本語 \u{1f1fa}\u{1f1f8} e\u{301} \u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}");
}

#[test]
fn a_wide_character_that_did_not_fit() {
    same("0123456789012345678\u{65e5}x");
}

#[test]
fn the_alternate_screen_and_what_is_behind_it() {
    same("main\r\nscreen\x1b[?1049h\x1b[2;2Halt\x1b[1;1Hx");
    same("main\x1b[?1049h\x1b[2Jalt\x1b[?1049l\x1b[?47hmore");
}

#[test]
fn hyperlinks() {
    same("a\x1b]8;id=x;http://e.com/\x1b\\link\x1b]8;;\x1b\\ b \x1b]8;;http://f.org\x1b\\anon\x1b]8;;\x1b\\");
}

#[test]
fn tab_stops_and_tab_marks() {
    same("\x1b[3g\x1b[1;5H\x1bH\x1b[1;12H\x1bH\x1b[1;1Ha\tb\t\tc");
}

#[test]
fn vt52_mode_with_and_without_its_graphics_set() {
    // The graphics set is entered with ESC F only when the terminal is in it.
    repaint(b"abc\x1b[?2l", 20, 6);
    repaint(b"abc\x1b[?2l\x1bFdef", 20, 6);
}

#[test]
fn modes_and_margins() {
    same("\x1b[?2004h\x1b[?1000h\x1b[?1006h\x1b[?25l\x1b[20h\x1b[2;5r\x1b[?6h\x1b[2;2Hx");
}

#[test]
fn saved_cursors_and_charsets() {
    same("ab\x1b7\x1b[31m\x1b(0q\x1b)B\x0e\x1b[3;4H\x1b[?1049h\x1b7\x1b[5;5H\x1b[?1049l");
}

#[test]
fn titles_and_colors_and_the_keyboard() {
    same(
        "\x1b]2;one\x07\x1b[22;2t\x1b]2;two\x07\x1b]1;icon\x07\x1b]4;1;rgb:ff/00/00\x07\x1b]10;#112233\x07\x1b]7;file:///a/b\x07\x1b[>5u\x1b[>9u\x1b[?1049h\x1b[>1u",
    );
}

#[test]
fn shell_marks() {
    same(
        "\x1b]133;A\x07$ \x1b]133;B\x07ls\r\n\x1b]133;C\x07out\r\n\x1b]133;D;0\x07\x1b]133;A\x07$ \x1b]133;B\x07",
    );
}

#[test]
fn double_width_and_double_height_lines() {
    same("\x1b#6wide\r\n\x1b#3tall\r\n\x1b#4tall\r\nplain\x1b[2;3H");
    // Cells the row hides, a blank sized row, and the cursor on a sized row.
    same("abcdefghijklmnopqrst\x1b[1;1H\x1b#6\x1b[3;1H\x1b#6\x1b[4;1H\x1b#3zz");
}

#[test]
fn sized_lines_in_the_history_and_on_the_other_screen() {
    let mut text = String::from("\x1b#6top\r\n\x1b#3a\r\n\x1b#4a\r\n");
    for i in 0..10 {
        text.push_str(&format!("line {i}\r\n"));
    }
    same(&text);
    same("main\x1b[?1049h\x1b[2;1H\x1b#6alt\x1b[4;1H\x1b#4x\x1b[?1049l");
}

#[test]
fn a_double_width_line_that_wrapped_is_two_lines() {
    // The wrap happens at half the columns, where a receiver's cursor cannot be made to wait, so
    // the engine does not call it a soft wrap (and a repaint need not say one).
    same("\x1b#6abcdefghijklmn\r\nz");
    same("abcdefghijklmnopqrstuvw\x1b[1;1H\x1b#6\x1b[3;1H");
}

#[test]
fn the_cursor_with_a_wrap_pending() {
    same("01234567890123456789");
}

#[test]
fn a_cluster_that_can_still_grow() {
    same("a\u{1f1fa}");
}

#[test]
fn a_cluster_in_the_last_column_with_autowrap_off() {
    same("\x1b[?7labcdefghijklmnopqrstu");
    same("\x1b[?7labcdefghijklmnopqrst\u{e9}");
}

#[test]
fn a_wide_character_dropped_at_the_edge_leaves_a_gap() {
    // It is dropped, and is not what REP repeats: `t` is, with the cluster closed, which printing
    // `t` again in the cluster mode would open. The repaint says so and nothing else differs.
    let mut a = Terminal::builder().size(20, 6).build();
    a.feed("\x1b[?7labcdefghijklmnopqrst\u{4e2d}".as_bytes());
    let mut bytes = Vec::new();
    let done = a.repaint(&mut bytes).expect("ready");
    let mut b = Terminal::builder().size(20, 6).build();
    b.seed(&done.seeds);
    b.feed(&bytes);
    assert_eq!(done.gaps, Gaps::LAST_CHAR);
    let diff = a.dump_state().diff(&b.dump_state(), 6);
    assert!(diff.starts_with("a/last_char\n") && diff.matches("left :").count() == 1, "{diff}");
}

#[test]
fn the_last_character_for_repeat() {
    // A control function forgets it, so it is only there when text came last; the repaint prints
    // it again last.
    same("xyz\r\n\x1b[2;1H");
    same("xyz");
    same("xyz\r\nabc");
    same("\x1b[31mxyz\x1b[1;44m\u{4e2d}");
    same("abcdefghijklmnopqrst");
    same("abcdefghijklmnopqrstu");
    same("\x1b[?7labcdefghijklmnopqrstu");
    same("\x1b(0qqx");
    // A screen filled by DECALN or DECFRA, or one where nothing was ever printed, has none.
    same("\x1b#8");
    same("ab\x1b[37$x");
}

#[test]
fn history_just_cleared() {
    same("a\r\nb\x1b[3J\x1b[H");
}

/// Repaint after the sender was resized: the reflow leaves wrapped lines that end in blanks and
/// continue into blank rows, which are printed to wrap from and taken back.
fn resized(input: &str, from: (usize, usize), to: (usize, usize)) {
    let mut a = Terminal::builder().size(from.0, from.1).scrollback(100).build();
    a.feed(input.as_bytes());
    a.resize(to.0, to.1);
    let mut bytes = Vec::new();
    let done = a.repaint(&mut bytes).expect("ready");
    let mut b = Terminal::builder().size(to.0, to.1).scrollback(100).build();
    b.seed(&done.seeds);
    b.feed(&bytes);
    let (da, db) = (a.dump_state(), b.dump_state());
    assert!(da.same(&db), "gaps {:?}\n{}", done.gaps, da.diff(&db, 6));
}

#[test]
fn a_reflowed_screen() {
    resized("abcdefghij\x1b[1;8H\x1b[99P", (10, 3), (3, 7));
    resized("\x1b[44mab\x1b[K\x1b[m\r\ncd", (12, 3), (5, 6));
    resized("日本語のテキスト\r\n\x1b[31mred\x1b[m", (16, 4), (7, 5));
}

#[test]
fn the_modes_that_are_only_recorded_survive() {
    // The first of the VT510's hardware modes, others from the middle of the list and the last.
    same("\x1b[?18h");
    same("\x1b[?19h\x1b[?57h\x1b[?73h\x1b[?106h");
    same("\x1b[?18h\x1b[?34h\x1b[?64h\x1b[?81h\x1b[?100h\x1b[?106h\x1b[?100l");
}

#[test]
fn a_saved_cursor_beyond_a_narrower_screen_is_cut_to_it() {
    resized("\x1b[2;9H\x1b7\x1b[1;1H", (10, 3), (5, 3));
}

#[test]
fn every_mode_the_tests_know_survives() {
    same("\x1b[?1h\x1b[?5h\x1b[?7l\x1b[?45h\x1b[?66h\x1b[?1004h\x1b[?2026h\x1b[?9h\x1b[?1049h");
}

#[test]
fn a_repaint_is_made_only_between_sequences() {
    let mut term = Terminal::builder().size(10, 3).scrollback(10).build();
    term.feed(b"ab\x1b[3");
    let mut out = Vec::new();
    assert_eq!(term.repaint(&mut out).unwrap_err(), super::NotReady::Sequence);
    assert!(out.is_empty(), "nothing is written when it is refused");
    term.feed(b"1m");
    assert!(term.repaint(&mut out).is_ok());
}

#[test]
fn seeding_sets_the_counters() {
    let mut term = Terminal::builder().size(10, 3).scrollback(10).build();
    term.seed(&super::Seeds { block: 41, link_seq: 7, image_handle: 3 });
    term.feed(b"\x1b]133;A\x07$ \x1b]8;;http://x\x07a");
    let mut out = Vec::new();
    let done = term.repaint(&mut out).expect("ready");
    assert_eq!(done.seeds.block, 41, "the repaint's own marks count up from the first block it holds");
    assert_eq!(done.seeds.link_seq, 8);
    assert_eq!(done.seeds.image_handle, 3);
}

#[test]
fn thousands_of_tab_stops_on_a_very_wide_screen() {
    // A stop in every column of 9,000: a DECRSPS of them is longer than a receiver takes.
    let input = format!("\x1b[3g{}", "\x1bH\x1b[C".repeat(9_000));
    let (a, _, gaps) = repaint(input.as_bytes(), 9_000, 3);
    assert!(gaps.is_empty(), "left out: {gaps:?}");
    assert_eq!(a.tabs.columns().count(), 9_000);
}

#[test]
fn a_capped_repaint_keeps_the_newest_history_and_the_screen() {
    let mut a = Terminal::builder().size(20, 4).scrollback(100).build();
    let mut text = String::new();
    for i in 0..40 {
        text.push_str(&format!("line {i}\r\n"));
    }
    a.feed(format!("\x1b]133;A\x07{text}tail").as_bytes());
    assert!(a.history_len() > 10);
    let capped = |cap: usize| {
        let mut bytes = Vec::new();
        let done = a.repaint_capped(&mut bytes, cap).expect("ready");
        let mut b = Terminal::builder().size(20, 4).scrollback(100).build();
        b.seed(&done.seeds);
        b.feed(&bytes);
        (b, done)
    };
    let (b, done) = capped(10);
    assert_eq!(b.history_len(), 10);
    assert_eq!(done.history_lines, 10);
    // The same last ten lines of history, and the same screen.
    for line in -10..4 {
        let at = crate::types::Line(line);
        assert_eq!(b.row_text(at, true), a.row_text(at, true), "line {line}");
    }
    assert_eq!(b.cursor().point, a.cursor().point);
    // A cap above the history writes it all, as `repaint` does.
    let (whole, _) = capped(usize::MAX);
    let mut bytes = Vec::new();
    a.repaint(&mut bytes).expect("ready");
    let mut plain = Terminal::builder().size(20, 4).scrollback(100).build();
    plain.feed(&bytes);
    assert_eq!(whole.history_len(), a.history_len());
    assert_eq!(plain.history_len(), a.history_len());
    // No history at all: just the screen.
    let (none, _) = capped(0);
    assert_eq!(none.history_len(), 0);
    assert_eq!(none.row_text(crate::types::Line(0), true), a.row_text(crate::types::Line(0), true));
}

#[test]
fn the_default_colors_read_back_as_set() {
    let mut term = Terminal::new(10, 3);
    let mut palette = *term.default_colors();
    palette[1] = crate::types::Rgb { r: 1, g: 2, b: 3 };
    term.set_default_colors(&palette);
    assert_eq!(term.default_colors()[1], crate::types::Rgb { r: 1, g: 2, b: 3 });
}
