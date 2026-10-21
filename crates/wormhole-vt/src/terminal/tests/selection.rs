//! Selecting and reading text through the terminal: each kind of selection, and how a selection
//! lives through output, scrolling, reflow, the alternate screen and the history going away.

use super::Rig;
use crate::grid::{BlockPart, TextCopy, TextShape};
use crate::types::{Column, Line, Point, SelectionRange, SelectionType, Side};

fn at(line: i32, column: usize) -> Point {
    Point::new(Line(line), Column(column))
}

fn select(rig: &mut Rig, kind: SelectionType, from: (i32, usize, Side), to: (i32, usize, Side)) {
    rig.term.start_selection(kind, at(from.0, from.1), from.2);
    rig.term.update_selection(at(to.0, to.1), to.2);
}

/// The cells selected as `(line, col)` to `(line, col)`.
fn range(rig: &Rig) -> Option<((i32, usize), (i32, usize))> {
    rig.term.selection_range().map(|r| ((r.start.line.0, r.start.column.0), (r.end.line.0, r.end.column.0)))
}

const L: Side = Side::Left;
const R: Side = Side::Right;

// ---- simple ----

#[test]
fn a_drag_from_one_cell_to_another_takes_both() {
    let mut rig = Rig::new(10, 4);
    rig.feed("hello world");
    select(&mut rig, SelectionType::Simple, (0, 1, L), (0, 3, R));
    assert_eq!(range(&rig), Some(((0, 1), (0, 3))));
    assert_eq!(rig.term.selection_text().as_deref(), Some("ell"));
}

#[test]
fn the_half_of_a_cell_the_pointer_is_on_decides_whether_the_cell_is_in() {
    let mut rig = Rig::new(10, 4);
    rig.feed("hello world");
    // From the right half of 'e' to the left half of 'o': the cells between.
    select(&mut rig, SelectionType::Simple, (0, 1, R), (0, 4, L));
    assert_eq!(rig.term.selection_text().as_deref(), Some("ll"));
    // The other way round gives the same.
    select(&mut rig, SelectionType::Simple, (0, 4, L), (0, 1, R));
    assert_eq!(rig.term.selection_text().as_deref(), Some("ll"));
    // Inside one cell: left to right is the cell, left to left nothing.
    select(&mut rig, SelectionType::Simple, (0, 1, L), (0, 1, R));
    assert_eq!(rig.term.selection_text().as_deref(), Some("e"));
    select(&mut rig, SelectionType::Simple, (0, 1, L), (0, 1, L));
    assert_eq!(rig.term.selection_range(), None);
    select(&mut rig, SelectionType::Simple, (0, 1, R), (0, 2, L));
    assert_eq!(rig.term.selection_range(), None, "the edge between two cells is no cells");
}

#[test]
fn a_click_without_a_drag_selects_nothing_and_is_dropped() {
    let mut rig = Rig::new(10, 4);
    rig.feed("hello");
    rig.term.start_selection(SelectionType::Simple, at(0, 1), L);
    assert!(rig.term.selection_range().is_none());
    rig.term.drop_empty_selection();
    rig.term.update_selection(at(0, 3), R);
    assert!(rig.term.selection_range().is_none(), "there was nothing to extend");
    assert!(!rig.term.take_selection());
    // A drag stays.
    select(&mut rig, SelectionType::Simple, (0, 1, L), (0, 3, R));
    rig.term.drop_empty_selection();
    assert!(rig.term.selection_range().is_some());
    assert!(rig.term.take_selection());
    assert!(rig.term.selection_range().is_none());
}

#[test]
fn a_selection_across_rows_joins_wraps_and_breaks_lines() {
    let mut rig = Rig::new(10, 6);
    rig.feed("hello world\r\nab\r\n\r\ncd");
    select(&mut rig, SelectionType::Simple, (0, 3, L), (4, 0, R));
    assert_eq!(range(&rig), Some(((0, 3), (4, 0))));
    assert_eq!(rig.term.selection_text().as_deref(), Some("lo world\nab\n\nc"));
}

#[test]
fn an_edge_at_the_end_of_a_row_is_the_start_of_the_next() {
    let mut rig = Rig::new(10, 4);
    rig.feed("abcdefghij\r\nxyz");
    select(&mut rig, SelectionType::Simple, (0, 9, R), (1, 1, R));
    assert_eq!(range(&rig), Some(((1, 0), (1, 1))));
    assert_eq!(rig.term.selection_text().as_deref(), Some("xy"));
}

#[test]
fn a_wide_character_is_copied_whole() {
    let mut rig = Rig::new(10, 4);
    rig.feed("a日本b");
    select(&mut rig, SelectionType::Simple, (0, 2, L), (0, 2, R));
    assert_eq!(rig.term.selection_text().as_deref(), Some("日"), "from its second cell");
    select(&mut rig, SelectionType::Simple, (0, 0, L), (0, 1, R));
    assert_eq!(rig.term.selection_text().as_deref(), Some("a日"), "to its first");
}

// ---- block ----

#[test]
fn a_block_is_a_rectangle() {
    let mut rig = Rig::new(10, 5);
    rig.feed("abcdef\r\nghijkl\r\nmn\r\nopqrst");
    select(&mut rig, SelectionType::Block, (0, 1, L), (3, 3, R));
    let selected = rig.term.selection_range().expect("selected");
    assert_eq!((selected.start, selected.end, selected.is_block), (at(0, 1), at(3, 3), true));
    assert_eq!(rig.term.selection_text().as_deref(), Some("bcd\nhij\nn\npqr"));
    // From the bottom right to the top left.
    select(&mut rig, SelectionType::Block, (3, 3, R), (0, 1, L));
    assert_eq!(rig.term.selection_text().as_deref(), Some("bcd\nhij\nn\npqr"));
    // Right to left on the way down.
    select(&mut rig, SelectionType::Block, (0, 3, R), (1, 1, L));
    assert_eq!(range(&rig), Some(((0, 1), (1, 3))));
    assert_eq!(rig.term.selection_text().as_deref(), Some("bcd\nhij"));
}

#[test]
fn a_block_one_cell_wide_takes_the_cell_between_two_edges() {
    let mut rig = Rig::new(10, 4);
    rig.feed("abc\r\ndef");
    select(&mut rig, SelectionType::Block, (0, 1, L), (1, 1, R));
    assert_eq!(rig.term.selection_text().as_deref(), Some("b\ne"));
    select(&mut rig, SelectionType::Block, (0, 1, L), (1, 1, L));
    assert_eq!(rig.term.selection_range(), None);
}

// ---- semantic ----

#[test]
fn a_double_click_takes_the_word() {
    let mut rig = Rig::new(20, 4);
    rig.feed("echo foo_bar.baz (x)");
    select(&mut rig, SelectionType::Semantic, (0, 7, L), (0, 7, L));
    assert_eq!(range(&rig), Some(((0, 5), (0, 15))));
    assert_eq!(rig.term.selection_text().as_deref(), Some("foo_bar.baz"));
    // Dragging on takes the words under the pointer too.
    select(&mut rig, SelectionType::Semantic, (0, 1, L), (0, 7, L));
    assert_eq!(rig.term.selection_text().as_deref(), Some("echo foo_bar.baz"));
}

#[test]
fn the_separators_are_the_hosts_to_choose() {
    let mut rig = Rig::new(20, 4);
    rig.feed("echo foo_bar.baz (x)");
    assert_eq!(rig.term.word_separators(), crate::terminal::DEFAULT_WORD_SEPARATORS);
    rig.term.set_word_separators(" ._");
    select(&mut rig, SelectionType::Semantic, (0, 6, L), (0, 6, L));
    assert_eq!(rig.term.selection_text().as_deref(), Some("foo"));
    rig.term.set_word_separators("");
    select(&mut rig, SelectionType::Semantic, (0, 6, L), (0, 6, L));
    assert_eq!(
        rig.term.selection_text().as_deref(),
        Some("echo foo_bar.baz (x)"),
        "only an empty cell ends it"
    );
}

#[test]
fn a_terminal_can_start_with_the_separators_of_the_host() {
    let term = crate::Terminal::builder().word_separators(" ._").build();
    assert_eq!(term.word_separators(), " ._");
    assert_eq!(crate::Terminal::new(10, 2).word_separators(), crate::terminal::DEFAULT_WORD_SEPARATORS);
}

#[test]
fn a_word_goes_over_a_soft_wrap_but_not_a_line_end() {
    let mut rig = Rig::new(5, 5);
    rig.feed("abc defghij\r\nklm");
    select(&mut rig, SelectionType::Semantic, (1, 2, L), (1, 2, L));
    assert_eq!(range(&rig), Some(((0, 4), (2, 0))));
    assert_eq!(rig.term.selection_text().as_deref(), Some("defghij"));
    select(&mut rig, SelectionType::Semantic, (3, 0, L), (3, 0, L));
    assert_eq!(rig.term.selection_text().as_deref(), Some("klm"));
}

#[test]
fn a_click_on_a_bracket_takes_everything_to_its_partner() {
    let mut rig = Rig::new(20, 4);
    rig.feed("f(a, (b), c) tail");
    select(&mut rig, SelectionType::Semantic, (0, 1, L), (0, 1, L));
    assert_eq!(rig.term.selection_text().as_deref(), Some("(a, (b), c)"));
    select(&mut rig, SelectionType::Semantic, (0, 11, L), (0, 11, L));
    assert_eq!(rig.term.selection_text().as_deref(), Some("(a, (b), c)"));
    select(&mut rig, SelectionType::Semantic, (0, 5, L), (0, 5, L));
    assert_eq!(rig.term.selection_text().as_deref(), Some("(b)"));
    // A drag from a bracket is words as usual.
    select(&mut rig, SelectionType::Semantic, (0, 1, L), (0, 2, L));
    assert_eq!(rig.term.selection_text().as_deref(), Some("f(a"));
}

#[test]
fn a_word_with_a_wide_character_is_one_word() {
    let mut rig = Rig::new(20, 3);
    rig.feed("go 日本語abc x");
    select(&mut rig, SelectionType::Semantic, (0, 6, L), (0, 6, L));
    assert_eq!(rig.term.selection_text().as_deref(), Some("日本語abc"));
    select(&mut rig, SelectionType::Semantic, (0, 4, L), (0, 4, L));
    assert_eq!(rig.term.selection_text().as_deref(), Some("日本語abc"), "from a character's second cell");
}

// ---- lines ----

#[test]
fn a_triple_click_takes_the_program_line_over_all_its_rows() {
    let mut rig = Rig::new(5, 6);
    rig.feed("abcdefghij\r\nxy\r\n\r\nz");
    select(&mut rig, SelectionType::Lines, (1, 2, L), (1, 2, L));
    assert_eq!(range(&rig), Some(((0, 0), (1, 4))));
    assert_eq!(rig.term.selection_text().as_deref(), Some("abcdefghij\n"));
    select(&mut rig, SelectionType::Lines, (2, 0, L), (4, 0, L));
    assert_eq!(rig.term.selection_text().as_deref(), Some("xy\n\nz\n"));
    select(&mut rig, SelectionType::Lines, (3, 0, L), (3, 0, L));
    assert_eq!(rig.term.selection_text().as_deref(), Some("\n"));
}

// ---- living through the screen's changes ----

#[test]
fn a_selection_follows_its_text_as_output_scrolls() {
    let mut rig = Rig::new(10, 4);
    rig.feed("one\r\ntwo\r\nthree");
    select(&mut rig, SelectionType::Simple, (0, 0, L), (1, 2, R));
    assert_eq!(rig.term.selection_text().as_deref(), Some("one\ntwo"));
    rig.feed("\r\nfour\r\nfive\r\nsix");
    assert_eq!(range(&rig), Some(((-2, 0), (-1, 2))), "two lines of history above the screen now");
    assert_eq!(rig.term.selection_text().as_deref(), Some("one\ntwo"));
    // Scrolling the view moves nothing: lines are grid lines.
    rig.term.scroll(crate::types::Scroll::Delta(2));
    assert_eq!(range(&rig), Some(((-2, 0), (-1, 2))));
}

#[test]
fn a_selection_keeps_its_text_through_a_reflow() {
    let mut rig = Rig::new(20, 6);
    rig.feed("hello wonderful world\r\nnext");
    select(&mut rig, SelectionType::Simple, (0, 6, L), (0, 14, R));
    assert_eq!(rig.term.selection_text().as_deref(), Some("wonderful"));
    rig.term.resize(7, 8);
    assert_eq!(rig.term.selection_text().as_deref(), Some("wonderful"), "narrower");
    let narrow = rig.term.selection_range().expect("selected");
    assert!(narrow.start.line < narrow.end.line, "now over several rows");
    rig.term.resize(40, 8);
    assert_eq!(rig.term.selection_text().as_deref(), Some("wonderful"), "wider again");
    assert_eq!(range(&rig), Some(((0, 6), (0, 14))));
}

#[test]
fn a_word_selection_is_a_word_again_after_a_reflow() {
    let mut rig = Rig::new(20, 6);
    rig.feed("say hello world");
    select(&mut rig, SelectionType::Semantic, (0, 6, L), (0, 6, L));
    rig.term.resize(8, 6);
    assert_eq!(rig.term.selection_text().as_deref(), Some("hello"));
}

#[test]
fn the_alternate_screen_ends_a_selection_and_has_its_own() {
    let mut rig = Rig::new(10, 4);
    rig.feed("main text");
    select(&mut rig, SelectionType::Simple, (0, 0, L), (0, 3, R));
    assert!(rig.term.selection_range().is_some());
    rig.feed("\x1b[?1049h");
    assert!(rig.term.selection_range().is_none(), "entering drops it");
    rig.feed("\x1b[Halt");
    select(&mut rig, SelectionType::Simple, (0, 0, L), (0, 2, R));
    assert_eq!(rig.term.selection_text().as_deref(), Some("alt"));
    rig.feed("\x1b[?1049l");
    assert!(rig.term.selection_range().is_none(), "leaving drops it");
    // Nothing leaked: no pins are left on either screen.
    select(&mut rig, SelectionType::Simple, (0, 0, L), (0, 3, R));
    assert_eq!(rig.term.selection_text().as_deref(), Some("main"));
}

#[test]
fn a_selection_goes_when_its_text_leaves_the_history() {
    let mut rig = Rig::new(10, 4);
    rig.feed("keep me\r\n");
    select(&mut rig, SelectionType::Simple, (0, 0, L), (0, 3, R));
    // More than the history holds: a 10-column line costs 96 bytes, and the limit is a few
    // hundred kilobytes.
    let flood = "x\r\n".repeat(30_000);
    rig.feed(&flood);
    assert!(rig.term.selection_range().is_none());
    assert!(rig.term.selection_text().is_none());
    assert!(!rig.term.take_selection(), "already gone");
}

#[test]
fn clearing_the_history_takes_a_selection_in_it_along() {
    let mut rig = Rig::new(10, 3);
    for i in 0..10 {
        rig.feed(&format!("line {i}\r\n"));
    }
    select(&mut rig, SelectionType::Simple, (-5, 0, L), (-4, 3, R));
    assert!(rig.term.selection_text().is_some());
    rig.term.clear_history();
    assert!(rig.term.selection_range().is_none());
    // One on the screen stays.
    select(&mut rig, SelectionType::Simple, (0, 0, L), (0, 3, R));
    rig.term.clear_history();
    assert!(rig.term.selection_range().is_some());
}

#[test]
fn a_new_selection_replaces_the_old_and_frees_its_pins() {
    let mut rig = Rig::new(10, 3);
    rig.feed("abc");
    for _ in 0..1000 {
        select(&mut rig, SelectionType::Simple, (0, 0, L), (0, 1, R));
    }
    // A reflow tracks every pin there is; a leak would make this slow, and wrong in the range.
    rig.term.resize(5, 3);
    assert_eq!(rig.term.selection_text().as_deref(), Some("ab"));
}

// ---- text, spans, links, marks ----

#[test]
fn the_screen_text_is_a_trimmed_line_per_row() {
    let mut rig = Rig::new(10, 3);
    rig.feed("ab  \r\n\u{65e5}x");
    assert_eq!(rig.term.screen_text(), "ab\n日x\n\n");
}

#[test]
fn the_text_to_restore_is_the_program_lines_and_not_the_alternate_screen() {
    let mut rig = Rig::new(10, 3);
    rig.feed("0123456789abcdef\r\nnext\r\n\r\n\r\n\r\n");
    assert_eq!(
        rig.term.restore_lines(100).as_deref(),
        Some(&["0123456789abcdef".to_string(), "next".into()][..])
    );
    assert_eq!(rig.term.restore_lines(2).as_deref(), Some(&[][..]), "the last two rows are blank");
    rig.feed("\x1b[?1049hVIM");
    assert_eq!(rig.term.restore_lines(100), None);
}

#[test]
fn a_found_span_follows_its_text_and_says_when_it_is_stale() {
    let mut rig = Rig::new(10, 4);
    rig.feed("find the needle here");
    // "needle" is at columns 9..: row 0 "find the n", row 1 "eedle here".
    let span = rig.term.pin_span(at(0, 9), at(1, 4));
    assert!(rig.term.span_holds(span, "needle"));
    assert!(!rig.term.span_holds(span, "needlx"));
    rig.feed("\r\n\r\n\r\n\r\n");
    assert_eq!(rig.term.span_points(span), Some((at(-2, 9), at(-1, 4))), "scrolled up with its text");
    assert!(rig.term.span_holds(span, "needle"));
    rig.term.resize(20, 4);
    assert_eq!(rig.term.span_points(span).map(|(a, _)| a.column), Some(Column(9)));
    assert!(rig.term.span_holds(span, "needle"), "also reflowed to one row");
    rig.term.clear_history();
    assert_eq!(rig.term.span_points(span), None);
    assert!(!rig.term.span_holds(span, "needle"));
    rig.term.unpin_span(span);
}

#[test]
fn a_span_that_was_unpinned_is_gone_while_its_text_is_not() {
    let mut rig = Rig::new(10, 4);
    rig.feed("needle");
    let span = rig.term.pin_span(at(0, 0), at(0, 5));
    assert_eq!(rig.term.span_points(span), Some((at(0, 0), at(0, 5))));
    rig.term.unpin_span(span);
    assert_eq!(rig.term.span_points(span), None, "nothing follows the text any more");
    assert!(!rig.term.span_holds(span, "needle"));
    assert!(rig.term.span_is(at(0, 0), at(0, 5), "needle"), "the cells are as they were");
}

#[test]
fn a_stored_match_is_stale_once_its_cells_were_overwritten() {
    let mut rig = Rig::new(20, 3);
    rig.feed("needle");
    let span = rig.term.pin_span(at(0, 0), at(0, 5));
    assert!(rig.term.span_holds(span, "needle"));
    rig.feed("\rneedlx");
    assert!(!rig.term.span_holds(span, "needle"));
    assert!(rig.term.span_is(at(0, 0), at(0, 5), "needlx"));
}

#[test]
fn text_for_a_search_is_copied_in_windows() {
    let mut rig = Rig::new(10, 4);
    rig.feed("one two\r\nthree\r\nfour\r\nfive six\r\nseven\r\n");
    let mut copy = TextCopy::new();
    let next = rig.term.copy_text(Line(-2), 2, &mut copy);
    let first: Vec<&str> = copy.lines().map(|l| l.text()).collect();
    assert_eq!(first, ["one two", "three"]);
    let next = rig.term.copy_text(next.expect("more"), 10, &mut copy);
    assert_eq!(next, None);
    let rest: Vec<&str> = copy.lines().map(|l| l.text()).collect();
    assert_eq!(rest, ["four", "five six", "seven", ""]);
    let line = copy.lines().nth(1).expect("five six");
    assert_eq!(line.cells(5..8), Some((at(1, 5), at(1, 7))), "\"six\" on the screen's second row");
}

#[test]
fn the_text_between_two_points_is_read_through_the_terminal() {
    let mut rig = Rig::new(10, 4);
    rig.feed("abc\r\ndef");
    assert_eq!(rig.term.text_between(at(0, 1), at(1, 1), TextShape::Stream), "bc\nde");
    assert_eq!(rig.term.text_between(at(0, 1), at(1, 2), TextShape::Block), "bc\nef");
}

#[test]
fn a_hyperlink_is_found_with_the_cells_it_covers() {
    let mut rig = Rig::new(10, 4);
    rig.feed("see \x1b]8;;https://example.com/a\x1b\\https://example.com/a\x1b]8;;\x1b\\ end");
    let span = rig.term.link_span(at(1, 2)).expect("under the link");
    assert_eq!(&*span.link.uri, "https://example.com/a");
    assert_eq!((span.start, span.end), (at(0, 4), at(2, 4)));
    assert_eq!(rig.term.link_span(at(0, 0)), None);
}

#[test]
fn the_parts_of_a_command_block_are_read_back_from_the_row() {
    let mut rig = Rig::new(30, 4);
    rig.feed("\x1b]133;A\x07$ \x1b]133;B\x07ls -l\r\n\x1b]133;C\x07out1\r\n\x1b]133;D;0\x07");
    let block = rig.term.row_mark(Line(0)).map(|mark| mark.block).expect("a block");
    assert_eq!(rig.term.block_text(Line(0), block, BlockPart::Prompt), "$ ");
    assert_eq!(rig.term.block_text(Line(0), block, BlockPart::Command), "ls -l");
    assert_eq!(rig.term.block_text(Line(1), block, BlockPart::Output), "out1");
}

#[test]
fn the_cells_of_a_command_are_read_back_with_how_they_are_drawn() {
    let mut rig = Rig::new(30, 4);
    rig.feed("\x1b]133;A\x07$ \x1b]133;B\x07ls \x1b[8mrm\x1b[0m\r\n\x1b]133;C\x07out\r\n\x1b]133;D;0\x07");
    let block = rig.term.row_mark(Line(0)).map(|mark| mark.block).expect("a block");
    let cells = rig.term.block_cells(Line(0), block, BlockPart::Command);
    assert_eq!(cells.iter().map(|cell| cell.c).collect::<String>(), "ls rm");
    let hidden: Vec<bool> =
        cells.iter().map(|cell| cell.flags.contains(crate::types::Flags::HIDDEN)).collect();
    assert_eq!(hidden, [false, false, false, true, true]);
    assert!(rig.term.block_cells(Line(1), block, BlockPart::Command).is_empty());
}

#[test]
fn a_selection_range_is_a_plain_value() {
    let mut rig = Rig::new(10, 3);
    rig.feed("abc");
    select(&mut rig, SelectionType::Simple, (0, 0, L), (0, 1, R));
    assert_eq!(
        rig.term.selection_range(),
        Some(SelectionRange { start: at(0, 0), end: at(0, 1), is_block: false })
    );
}

#[test]
fn points_from_anywhere_never_panic() {
    let mut rig = Rig::new(10, 4);
    rig.feed("some text\r\nmore (text)");
    let extremes = [
        at(i32::MIN, 0),
        at(i32::MAX, usize::MAX),
        at(-1, usize::MAX),
        at(0, usize::MAX),
        at(3, 9),
        at(0, 0),
    ];
    for kind in [SelectionType::Simple, SelectionType::Block, SelectionType::Semantic, SelectionType::Lines] {
        for from in extremes {
            for to in extremes {
                for side in [L, R] {
                    rig.term.start_selection(kind, from, side);
                    rig.term.update_selection(to, side);
                    let _ = (rig.term.selection_range(), rig.term.selection_text());
                    rig.term.drop_empty_selection();
                }
            }
        }
        for (a, b) in extremes.into_iter().zip(extremes.into_iter().rev()) {
            for shape in [TextShape::Stream, TextShape::Block] {
                let _ = rig.term.text_between(a, b, shape);
            }
            let _ = (rig.term.span_is(a, b, "x"), rig.term.link_span(a), rig.term.grid().bracket_match(a));
            let mut copy = TextCopy::new();
            let _ = rig.term.copy_text(a.line, usize::MAX, &mut copy);
            let _ = rig.term.pin_span(a, b);
        }
    }
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
