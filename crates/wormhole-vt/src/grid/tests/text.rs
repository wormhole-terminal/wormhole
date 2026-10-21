//! Text out of the grid: ranges to copy, lines to restore, spans to compare, words and links
//! under a point, and the copy a search runs over.

use super::{Sim, at, plain};
use crate::grid::{BlockPart, DEFAULT_SCROLLBACK_BYTES, Pen, TextCopy, TextShape};
use crate::types::{Hyperlink, Line};

fn sim_of(text: &str) -> Sim {
    sim(20, 3, text)
}

fn sim(cols: usize, rows: usize, text: &str) -> Sim {
    let mut sim = Sim::new(cols, rows, DEFAULT_SCROLLBACK_BYTES);
    sim.print(text);
    sim
}

fn stream(sim: &Sim, from: (i32, usize), to: (i32, usize)) -> String {
    sim.grid.text_between(at(from.0, from.1), at(to.0, to.1), TextShape::Stream)
}

fn block(sim: &Sim, from: (i32, usize), to: (i32, usize)) -> String {
    sim.grid.text_between(at(from.0, from.1), at(to.0, to.1), TextShape::Block)
}

fn is_sep(c: char) -> bool {
    " ,()".contains(c)
}

// ---- text_between ----

#[test]
fn a_range_in_one_row_is_its_cells() {
    let sim = sim(10, 4, "hello world");
    assert_eq!(stream(&sim, (0, 1), (0, 3)), "ell");
    assert_eq!(stream(&sim, (0, 3), (0, 1)), "ell", "either order");
}

#[test]
fn soft_wrapped_rows_join_and_hard_line_ends_get_a_newline() {
    let sim = sim(10, 8, "hello world\nab\n\ncd");
    // "hello worl" wraps into "d"; then "ab", an empty line, "cd".
    assert_eq!(stream(&sim, (0, 0), (4, 1)), "hello world\nab\n\ncd");
    assert_eq!(stream(&sim, (0, 6), (1, 0)), "world");
    assert_eq!(stream(&sim, (1, 0), (2, 1)), "d\nab");
}

#[test]
fn blanks_at_a_line_end_are_cut_but_those_inside_a_range_stay() {
    let sim = sim(10, 3, "ab  cd");
    assert_eq!(stream(&sim, (0, 0), (0, 9)), "ab  cd");
    assert_eq!(stream(&sim, (0, 0), (0, 3)), "ab  ", "the range ends inside the text");
    assert_eq!(stream(&sim, (0, 7), (0, 9)), "", "wholly in the blank");
}

#[test]
fn a_range_that_ends_at_a_line_end_has_no_newline_after_it() {
    let sim = sim(10, 3, "ab\ncd");
    assert_eq!(stream(&sim, (0, 0), (0, 9)), "ab");
    assert_eq!(stream(&sim, (0, 0), (1, 9)), "ab\ncd");
    assert_eq!(stream(&sim, (0, 0), (2, 9)), "ab\ncd\n", "a blank row inside the range is a line");
}

#[test]
fn a_wide_character_is_copied_once_and_whole() {
    let sim = sim(10, 3, "a日b");
    assert_eq!(stream(&sim, (0, 0), (0, 9)), "a日b");
    assert_eq!(stream(&sim, (0, 2), (0, 2)), "日", "a range that starts on the second cell");
    assert_eq!(stream(&sim, (0, 1), (0, 1)), "日", "one that ends on the first");
}

#[test]
fn a_wide_character_that_went_to_the_next_row_leaves_no_blank_in_the_text() {
    let sim = sim(5, 3, "abcd日e");
    assert_eq!(stream(&sim, (0, 0), (1, 4)), "abcd日e");
}

#[test]
fn combining_marks_come_with_their_character() {
    let sim = sim(10, 2, "cafe\u{301} ok");
    assert_eq!(stream(&sim, (0, 0), (0, 9)), "cafe\u{301} ok");
}

#[test]
fn a_block_is_the_same_columns_of_every_row() {
    let sim = sim(10, 4, "abcdef\nghijkl\nmn\nopqrst");
    assert_eq!(block(&sim, (0, 1), (3, 3)), "bcd\nhij\nn\npqr");
    assert_eq!(block(&sim, (3, 3), (0, 1)), "bcd\nhij\nn\npqr", "from any corner");
    assert_eq!(block(&sim, (0, 3), (1, 1)), "bcd\nhij", "right to left");
}

#[test]
fn points_outside_the_grid_are_clamped_to_its_edges() {
    let sim = sim(10, 3, "ab\ncd");
    assert_eq!(stream(&sim, (-5, 4), (9, 2)), "ab\ncd\n", "the whole grid");
    assert_eq!(sim.grid.text_between(at(7, 0), at(9, 0), TextShape::Stream), "");
    assert_eq!(sim.grid.text_between(at(0, 99), at(0, 99), TextShape::Stream), "");
    assert_eq!(sim.grid.text_between(at(-9, 0), at(-8, 0), TextShape::Block), "");
}

#[test]
fn the_history_is_part_of_the_grid() {
    let mut sim = Sim::new(10, 2, DEFAULT_SCROLLBACK_BYTES);
    for i in 0..6 {
        sim.print(&format!("line {i}\n"));
    }
    let top = -(sim.grid.history_len() as i32);
    assert_eq!(stream(&sim, (top, 0), (top + 2, 9)), "line 0\nline 1\nline 2");
}

// ---- logical lines, spans ----

#[test]
fn logical_lines_join_wrapped_rows_and_cut_blanks() {
    let sim = sim(5, 6, "abcd efgh\nij  \n\nk");
    assert_eq!(sim.grid.logical_lines(Line(0), Line(5)), ["abcd efgh", "ij", "", "k", ""]);
    // A range that starts inside a wrapped line gives that part.
    assert_eq!(sim.grid.logical_lines(Line(1), Line(2)), ["efgh", "ij"]);
}

#[test]
fn a_span_is_the_cells_run_together() {
    let sim = sim(5, 3, "abc日def");
    // abc日 / def: the wide character moved to the next row.
    assert_eq!(sim.grid.span_text(at(0, 1), at(1, 2)), "bc日def");
    assert!(sim.grid.span_is(at(0, 1), at(1, 2), "bc日def"));
    assert!(!sim.grid.span_is(at(0, 1), at(1, 2), "bc日de"));
    assert!(!sim.grid.span_is(at(0, 1), at(1, 2), "bc日defx"));
    assert!(!sim.grid.span_is(at(0, 1), at(5, 2), "bc日def"), "a row that isn't there");
    assert!(!sim.grid.span_is(at(1, 2), at(0, 1), ""), "backwards");
}

#[test]
fn content_text_is_the_part_of_a_block_on_a_row() {
    let mut sim = Sim::new(20, 3, DEFAULT_SCROLLBACK_BYTES);
    let prompt = Pen { content: BlockPart::Prompt, block: 4, ..plain() };
    let command = Pen { content: BlockPart::Command, block: 4, ..plain() };
    sim.pen = prompt;
    sim.print("$ ");
    sim.pen = command;
    sim.print("ls -l");
    assert_eq!(sim.grid.block_text(Line(0), 4, BlockPart::Prompt), "$ ");
    assert_eq!(sim.grid.block_text(Line(0), 4, BlockPart::Command), "ls -l");
    assert_eq!(sim.grid.block_text(Line(0), 4, BlockPart::Output), "");
    assert_eq!(sim.grid.block_text(Line(0), 5, BlockPart::Command), "", "another block");
    assert_eq!(sim.grid.block_text(Line(9), 4, BlockPart::Command), "", "no such row");
}

#[test]
fn block_cells_are_the_cells_of_the_part_as_they_are_drawn() {
    let mut sim = Sim::new(20, 3, DEFAULT_SCROLLBACK_BYTES);
    let prompt = Pen { content: BlockPart::Prompt, block: 4, ..plain() };
    let command = Pen { content: BlockPart::Command, block: 4, ..plain() };
    sim.pen = prompt;
    sim.print("$ ");
    sim.pen = command;
    sim.print("ls");
    let cells = sim.grid.block_cells(Line(0), 4, BlockPart::Command);
    assert_eq!(cells.iter().map(|cell| cell.c).collect::<String>(), "ls");
    assert!(cells.iter().all(|cell| !cell.flags.contains(crate::types::Flags::HIDDEN)));
    assert_eq!(sim.grid.block_cells(Line(0), 4, BlockPart::Prompt).len(), 2);
    assert!(sim.grid.block_cells(Line(0), 5, BlockPart::Command).is_empty(), "another block");
    assert!(sim.grid.block_cells(Line(9), 4, BlockPart::Command).is_empty(), "no such row");
}

// ---- words ----

#[test]
fn a_word_ends_at_a_separator() {
    let sim = sim(20, 3, "foo bar(baz)");
    let word = |col: usize| (sim.grid.word_start(at(0, col), is_sep), sim.grid.word_end(at(0, col), is_sep));
    assert_eq!(word(1), (at(0, 0), at(0, 2)));
    assert_eq!(word(5), (at(0, 4), at(0, 6)));
    assert_eq!(word(9), (at(0, 8), at(0, 10)));
}

#[test]
fn the_cell_clicked_never_stops_the_walk_but_its_neighbours_do() {
    let sim = sim(20, 3, "foo bar");
    // The clicked cell never stops the walk, only the cells next to it: on a space between two
    // words the walk goes both ways.
    assert_eq!(sim.grid.word_start(at(0, 3), is_sep), at(0, 0));
    assert_eq!(sim.grid.word_end(at(0, 3), is_sep), at(0, 6));
    // Next to another separator it stops at once.
    let sim = sim_of("ab  cd");
    assert_eq!(sim.grid.word_start(at(0, 3), is_sep), at(0, 3));
    assert_eq!(sim.grid.word_end(at(0, 2), is_sep), at(0, 2));
}

#[test]
fn a_word_goes_on_over_a_soft_wrap_and_stops_at_a_line_end() {
    let sim = sim(5, 4, "abc defghij\nklm");
    // "abc d" / "efghi" / "j", then a hard line end.
    assert_eq!(sim.grid.word_start(at(1, 2), is_sep), at(0, 4));
    assert_eq!(sim.grid.word_end(at(1, 2), is_sep), at(2, 0));
    assert_eq!(sim.grid.word_end(at(3, 0), is_sep), at(3, 2));
    assert_eq!(sim.grid.word_start(at(3, 0), is_sep), at(3, 0), "the line before is another line");
}

#[test]
fn a_wide_character_is_part_of_a_word_and_the_blank_before_it_is_stepped_over() {
    let sim = sim(5, 4, "abcd日ef");
    // "abcd" + a blank (the wide character did not fit) / "日ef".
    assert_eq!(sim.grid.word_start(at(1, 1), is_sep), at(0, 0), "over the spacer head");
    assert_eq!(sim.grid.word_end(at(0, 1), is_sep), at(1, 3));
    let sim = sim_with_wide_after_space();
    assert_eq!(sim.grid.word_start(at(1, 0), is_sep), at(1, 0), "the blank is not part of the word");
}

fn sim_with_wide_after_space() -> Sim {
    sim(5, 4, "abc 日ef")
}

#[test]
fn a_bracket_finds_its_partner() {
    let lone = sim(10, 3, "(never closed");
    assert_eq!(lone.grid.bracket_match(at(0, 0)), None);
    let sim = sim(20, 4, "f(a(b)c) [x]\n(a\nb)");
    assert_eq!(sim.grid.bracket_match(at(0, 1)), Some(at(0, 7)), "nested pairs are skipped");
    assert_eq!(sim.grid.bracket_match(at(0, 7)), Some(at(0, 1)));
    assert_eq!(sim.grid.bracket_match(at(0, 3)), Some(at(0, 5)));
    assert_eq!(sim.grid.bracket_match(at(0, 2)), None, "not a bracket");
    assert_eq!(sim.grid.bracket_match(at(2, 1)), Some(at(1, 0)), "the partner may be on another row");
    assert_eq!(sim.grid.bracket_match(at(1, 0)), Some(at(2, 1)));
}

#[test]
fn a_line_is_every_row_it_wrapped_over() {
    let sim = sim(5, 6, "abcdefghijkl\nxy\nz");
    assert_eq!(sim.grid.line_extent(Line(1)), (Line(0), Line(2)));
    assert_eq!(sim.grid.line_extent(Line(2)), (Line(0), Line(2)));
    assert_eq!(sim.grid.line_extent(Line(3)), (Line(3), Line(3)));
    assert_eq!(sim.grid.line_extent(Line(99)), (Line(5), Line(5)), "beyond the grid is its edge");
}

// ---- links ----

fn link(uri: &str) -> Option<Hyperlink> {
    Some(Hyperlink { id: uri.into(), uri: uri.into() })
}

#[test]
fn a_link_covers_its_cells_over_the_rows_it_wrapped_across() {
    let mut sim = Sim::new(5, 5, DEFAULT_SCROLLBACK_BYTES);
    sim.print("ab ");
    sim.pen.link = link("https://a");
    sim.print("cdefghij");
    sim.pen.link = None;
    sim.print(" kl\nmn");
    let span = |y: i32, x: usize| sim.grid.link_span(at(y, x));
    let a = span(0, 3).expect("under the link");
    assert_eq!((a.start, a.end), (at(0, 3), at(2, 0)));
    assert_eq!(&*a.link.uri, "https://a");
    assert_eq!(span(1, 2), Some(a.clone()), "from any cell of it");
    assert_eq!(span(2, 0), Some(a));
    assert_eq!(span(0, 0), None);
    assert_eq!(span(2, 2), None);
}

#[test]
fn two_links_next_to_each_other_are_two_spans() {
    let mut sim = Sim::new(10, 2, DEFAULT_SCROLLBACK_BYTES);
    sim.pen.link = link("one");
    sim.print("abc");
    sim.pen.link = link("two");
    sim.print("def");
    let one = sim.grid.link_span(at(0, 1)).expect("one");
    assert_eq!((one.start, one.end), (at(0, 0), at(0, 2)));
    let two = sim.grid.link_span(at(0, 4)).expect("two");
    assert_eq!((two.start, two.end), (at(0, 3), at(0, 5)));
}

// ---- the copy a search runs over ----

fn copy(sim: &Sim, from: i32, rows: usize) -> (TextCopy, Option<Line>) {
    let mut out = TextCopy::new();
    let next = sim.grid.copy_text(Line(from), rows, &mut out);
    (out, next)
}

fn lines(copy: &TextCopy) -> Vec<String> {
    copy.lines().map(|line| line.text().to_string()).collect()
}

/// The cells of the first occurrence of `needle` in the first line of `copy`.
fn find(copy: &TextCopy, needle: &str) -> Option<((i32, usize), (i32, usize))> {
    copy.lines().find_map(|line| {
        let at = line.text().find(needle)?;
        let (a, b) = line.cells(at..at + needle.len())?;
        Some(((a.line.0, a.column.0), (b.line.0, b.column.0)))
    })
}

#[test]
fn a_window_is_whole_lines_and_says_where_the_next_starts() {
    let sim = sim(10, 8, "one\ntwo\nthree\nfour\nfive");
    let (all, next) = copy(&sim, 0, 100);
    assert_eq!(lines(&all), ["one", "two", "three", "four", "five", "", "", ""]);
    assert_eq!(next, None, "reached the bottom");
    let (first, next) = copy(&sim, 0, 2);
    assert_eq!(lines(&first), ["one", "two"]);
    assert_eq!(next, Some(Line(2)));
    let (second, next) = copy(&sim, 2, 2);
    assert_eq!(lines(&second), ["three", "four"]);
    assert_eq!(next, Some(Line(4)));
}

#[test]
fn a_window_does_not_cut_a_wrapped_line() {
    let sim = sim(5, 8, "abcdefghijkl\nxy");
    let (first, next) = copy(&sim, 0, 1);
    assert_eq!(lines(&first), ["abcdefghijkl"], "the line goes on past the one row asked for");
    assert_eq!(next, Some(Line(3)));
}

#[test]
fn a_match_maps_back_to_cells_across_a_wrap() {
    let sim = sim(5, 6, "abc defghij k");
    // abc d / efghi / j k
    let (out, _) = copy(&sim, 0, 6);
    assert_eq!(lines(&out)[0], "abc defghij k");
    assert_eq!(find(&out, "defg"), Some(((0, 4), (1, 2))), "spans the wrap");
    assert_eq!(find(&out, "abc"), Some(((0, 0), (0, 2))));
    assert_eq!(find(&out, "ij k"), Some(((1, 4), (2, 2))));
    assert_eq!(find(&out, "k"), Some(((2, 2), (2, 2))));
}

#[test]
fn a_match_over_wide_characters_and_marks_ends_on_their_last_cell() {
    let sim = sim(20, 3, "a日本e\u{301}b x");
    let (out, _) = copy(&sim, 0, 3);
    assert_eq!(lines(&out)[0], "a日本e\u{301}b x");
    assert_eq!(find(&out, "日"), Some(((0, 1), (0, 2))), "a wide character is two cells");
    assert_eq!(find(&out, "本e\u{301}"), Some(((0, 3), (0, 5))));
    assert_eq!(find(&out, "b x"), Some(((0, 6), (0, 8))), "after the wide ones, cells and bytes differ");
    assert_eq!(find(&out, "e"), Some(((0, 5), (0, 5))), "the mark is in the cell of its character");
}

#[test]
fn a_wide_character_that_went_to_the_next_row_is_one_character_of_the_line() {
    let sim = sim(5, 4, "abcd日ef");
    let (out, _) = copy(&sim, 0, 4);
    assert_eq!(lines(&out)[0], "abcd日ef");
    assert_eq!(find(&out, "d日"), Some(((0, 3), (1, 1))));
    assert_eq!(find(&out, "ef"), Some(((1, 2), (1, 3))));
}

#[test]
fn blanks_at_a_line_end_are_not_text() {
    let sim = sim(10, 3, "ab   \ncd");
    let (out, _) = copy(&sim, 0, 3);
    assert_eq!(lines(&out), ["ab", "cd", ""]);
    assert_eq!(find(&out, " "), None);
}

#[test]
fn the_copy_notes_the_history_it_was_made_with_and_where_the_lines_were() {
    let mut sim = Sim::new(10, 2, DEFAULT_SCROLLBACK_BYTES);
    sim.print("a\nb\nc\nd");
    let (out, _) = copy(&sim, -2, 10);
    assert_eq!(out.history(), 2);
    let first: Vec<i32> = out.lines().map(|l| l.first_line().0).collect();
    assert_eq!(first, [-2, -1, 0, 1]);
    // From above the history is from its top.
    let (all, _) = copy(&sim, -50, 10);
    assert_eq!(lines(&all), ["a", "b", "c", "d"]);
}

#[test]
fn an_empty_or_inside_out_range_is_no_cells() {
    let sim = sim(10, 2, "abc");
    let (out, _) = copy(&sim, 0, 2);
    let line = out.lines().next().expect("a line");
    assert_eq!(line.cells(1..1), None);
    #[allow(clippy::reversed_empty_ranges)]
    let inside_out = line.cells(2..1);
    assert_eq!(inside_out, None);
    assert_eq!(line.cells(1..99), None);
    assert_eq!(line.cells(0..3), Some((at(0, 0), at(0, 2))));
}

#[test]
fn copying_reuses_the_buffers() {
    let sim = sim(40, 30, &"some text to copy\n".repeat(25));
    let mut out = TextCopy::new();
    sim.grid.copy_text(Line(0), 30, &mut out);
    let (text, table, lines) = out.capacities();
    for _ in 0..3 {
        sim.grid.copy_text(Line(0), 30, &mut out);
    }
    assert_eq!(out.capacities(), (text, table, lines), "nothing grows once the first copy sized it");
}

#[test]
fn lines_of_plain_ascii_cost_no_table_on_any_row() {
    let sim = sim(10, 4, "one\ntwo\nthree");
    let mut out = TextCopy::new();
    sim.grid.copy_text(Line(0), 4, &mut out);
    assert_eq!(out.capacities().1, 0, "byte n is cell n, so there is nothing to look up");
    assert_eq!(find(&out, "wo"), Some(((1, 1), (1, 2))));
    assert_eq!(find(&out, "hree"), Some(((2, 1), (2, 4))));
}

#[test]
fn the_cells_of_a_match_in_a_later_line_count_from_that_lines_start() {
    // The second line's bytes start after the first's in the copy's text, but its cells start at 0.
    let sim = sim(10, 4, "ab\ncd日ef");
    let (out, _) = copy(&sim, 0, 4);
    assert_eq!(find(&out, "ef"), Some(((1, 4), (1, 5))));
    assert_eq!(find(&out, "d日"), Some(((1, 1), (1, 3))));
}

#[test]
fn a_wrapped_line_that_ends_on_the_last_row_is_one_line() {
    let sim = sim(5, 3, "abcdefghijkl");
    let (out, next) = copy(&sim, 0, 3);
    assert_eq!(lines(&out), ["abcdefghijkl"]);
    assert_eq!(next, None);
}

#[test]
fn a_line_wrapped_over_more_rows_than_the_slack_allows_is_cut_there() {
    // 55 characters over 11 rows, for a window of one row: a line may go on for four times that.
    let sim = sim(5, 12, &"x".repeat(55));
    let (out, next) = copy(&sim, 0, 1);
    assert_eq!(lines(&out), ["x".repeat(20)]);
    assert_eq!(next, Some(Line(4)), "the next window starts where this one stopped");
}

#[test]
fn a_word_stops_at_a_line_end_even_when_the_row_before_is_full() {
    // The first row is full to its last cell but was ended with a newline, not wrapped.
    let sim = sim(5, 3, "abcde\nfgh");
    assert_eq!(sim.grid.word_start(at(1, 1), is_sep), at(1, 0));
    assert_eq!(sim.grid.word_start(at(1, 0), is_sep), at(1, 0));
}

#[test]
fn the_last_row_is_the_end_of_a_line_whatever_its_wrap_mark_says() {
    let mut sim = sim(5, 3, "abc");
    sim.grid.set_wrapped(2, true);
    assert_eq!(sim.grid.line_extent(Line(2)), (Line(2), Line(2)), "there is no row below to go on in");
}

#[test]
fn a_span_of_one_cell_or_more_is_compared_and_one_outside_the_screen_is_not() {
    let sim = sim(6, 2, "abcdef\nghijkl");
    assert!(sim.grid.span_is(at(0, 1), at(0, 1), "b"), "one cell");
    assert!(sim.grid.span_is(at(0, 3), at(1, 2), "defghi"), "rows run together");
    assert!(!sim.grid.span_is(at(0, 0), at(0, 99), "abcdef"), "ends past the last column");
    assert!(!sim.grid.span_is(at(0, 99), at(1, 2), "ghi"), "starts past the last column");
}
