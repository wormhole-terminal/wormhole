//! Resizing and reflow: the rules in `resize.rs`, one at a time, then random text at random widths.

use super::{BG, Rng, Sim, all_rows, at, colored, logical_lines, plain, screen, seeds};
use crate::grid::{DEFAULT_SCROLLBACK_BYTES, Grid, Pen};
use crate::types::{Color, Column, Hyperlink, Line, NamedColor, Point};

fn ok(grid: &Grid) {
    if let Err(e) = grid.check_invariants() {
        panic!("{e}");
    }
}

fn cursor(sim: &Sim) -> Point {
    at(sim.y as i32, sim.x)
}

#[test]
fn a_soft_wrapped_line_is_rewrapped_narrower_and_wider() {
    let mut sim = Sim::new(20, 6, DEFAULT_SCROLLBACK_BYTES);
    sim.print("hello world");
    let c = sim.grid.resize(5, 6, cursor(&sim), true);
    assert_eq!(screen(&sim.grid)[..3], ["hello", " worl", "d"]);
    assert_eq!(c, at(2, 1), "the cursor is after the d");
    ok(&sim.grid);
    let c = sim.grid.resize(20, 6, c, true);
    assert_eq!(screen(&sim.grid)[0], "hello world");
    assert_eq!(c, at(0, 11));
    ok(&sim.grid);
}

#[test]
fn a_line_a_program_ended_is_not_joined_to_the_next() {
    let mut sim = Sim::new(10, 6, DEFAULT_SCROLLBACK_BYTES);
    sim.print("abc\ndef\n");
    let c = sim.grid.resize(40, 6, cursor(&sim), true);
    assert_eq!(screen(&sim.grid)[..3], ["abc", "def", ""]);
    assert_eq!(c, at(2, 0));
    sim.grid.resize(2, 6, c, true);
    assert_eq!(logical_lines(&sim.grid), ["abc", "def"]);
    ok(&sim.grid);
}

#[test]
fn trailing_blanks_are_not_part_of_the_line() {
    let mut sim = Sim::new(20, 4, DEFAULT_SCROLLBACK_BYTES);
    sim.print("hi");
    sim.grid.erase(0, 2..20, BG, false);
    sim.grid.resize(3, 4, at(0, 2), true);
    assert_eq!(screen(&sim.grid)[..2], ["hi", ""], "two characters stay one row");
    ok(&sim.grid);
}

#[test]
fn an_erased_background_is_kept_with_the_line() {
    let mut sim = Sim::new(10, 3, DEFAULT_SCROLLBACK_BYTES);
    sim.print("ab");
    let blue = Color::Named(NamedColor::Blue);
    sim.grid.erase(0, 2..10, blue, false);
    sim.grid.resize(5, 3, at(0, 0), true);
    assert_eq!(sim.grid.style_at(at(1, 2)).map(|s| s.bg), Some(blue), "the second row is blue too");
    assert!(sim.grid.is_wrapped(Line(0)));
    ok(&sim.grid);
}

#[test]
fn wide_characters_are_never_split() {
    let mut sim = Sim::new(10, 6, DEFAULT_SCROLLBACK_BYTES);
    sim.print("ab日本語cd");
    // Width 5: "ab日" fills 4, 本 needs two and gets the next row, leaving a spacer head.
    let c = sim.grid.resize(5, 6, cursor(&sim), true);
    assert_eq!(screen(&sim.grid)[..3], ["ab日", "本語c", "d"]);
    assert!(sim.grid.cell(at(0, 4)).is_some_and(|cell| cell.is_spacer_head()));
    assert!(sim.grid.is_wrapped(Line(0)) && sim.grid.is_wrapped(Line(1)) && !sim.grid.is_wrapped(Line(2)));
    assert_eq!(c, at(2, 0), "on the d, where the cursor of a full row waits");
    ok(&sim.grid);
    // Back wider the spacer head goes.
    sim.grid.resize(10, 6, c, true);
    assert_eq!(screen(&sim.grid)[0], "ab日本語cd");
    assert!(sim.grid.cell(at(0, 4)).is_some_and(|cell| cell.is_wide()));
    ok(&sim.grid);
    // Two columns: one wide character per row.
    sim.grid.resize(2, 12, at(0, 0), true);
    assert_eq!(logical_lines(&sim.grid), ["ab日本語cd"]);
    ok(&sim.grid);
}

#[test]
fn styles_marks_and_links_go_with_their_text() {
    let mut sim = Sim::new(12, 6, DEFAULT_SCROLLBACK_BYTES);
    let link = Hyperlink { id: "1".into(), uri: "https://example.com".into() };
    sim.pen = colored(2);
    sim.print("red ");
    sim.pen = Pen { link: Some(link.clone()), ..plain() };
    sim.print("link");
    sim.pen = plain();
    sim.print(" e\u{301}nd");
    sim.grid.resize(4, 6, at(0, 0), true);
    assert_eq!(screen(&sim.grid)[..4], ["red", "link", " e\u{301}nd", ""].map(str::to_string));
    assert_eq!(sim.grid.style_at(at(0, 0)), Some(colored(2).style));
    assert_eq!(sim.grid.link_at(at(1, 0)), Some(link.clone()));
    assert_eq!(sim.grid.link_at(at(2, 0)), None);
    assert_eq!(sim.grid.marks_at(at(2, 1)), &['\u{301}']);
    ok(&sim.grid);
    sim.grid.resize(30, 6, at(0, 0), true);
    assert_eq!(sim.grid.link_at(at(0, 4)), Some(link));
    assert_eq!(screen(&sim.grid)[0], "red link e\u{301}nd");
    ok(&sim.grid);
}

#[test]
fn the_cursor_stays_on_its_character() {
    let text = "the quick brown fox";
    for width in 2..=14 {
        // Put the cursor on every character in turn.
        for pos in 0..text.len() {
            let mut g = Grid::new(14, 8, DEFAULT_SCROLLBACK_BYTES);
            let pen = plain();
            g.write_ascii(0, 0, &text.as_bytes()[..14], &pen);
            g.set_wrapped(0, true);
            g.write_ascii(1, 0, &text.as_bytes()[14..], &pen);
            let from = if pos < 14 { at(0, pos) } else { at(1, pos - 14) };
            let to = g.resize(width, 8, from, true);
            let found = g.cell(to).and_then(|c| c.character());
            assert_eq!(found, text.chars().nth(pos), "width {width}, character {pos}");
            ok(&g);
        }
    }
}

#[test]
fn a_cursor_past_the_text_stays_after_it() {
    let mut sim = Sim::new(20, 4, DEFAULT_SCROLLBACK_BYTES);
    sim.print("$ ");
    let c = sim.grid.resize(10, 4, cursor(&sim), true);
    assert_eq!(c, at(0, 2));
    let c = sim.grid.resize(2, 4, c, true);
    assert_eq!(c, at(1, 0), "text filled the row: the cursor takes the next");
    assert_eq!(logical_lines(&sim.grid), ["$"]);
    ok(&sim.grid);
}

#[test]
fn a_cursor_far_to_the_right_of_short_text_is_clamped_not_wrapped() {
    let mut g = Grid::new(40, 4, DEFAULT_SCROLLBACK_BYTES);
    g.write_ascii(0, 0, b"ab", &plain());
    let c = g.resize(10, 4, at(0, 30), true);
    assert_eq!(c, at(0, 9));
    assert_eq!(g.rows(), 4);
    ok(&g);
}

#[test]
fn shrinking_the_height_of_a_full_screen_pushes_the_top_into_the_history() {
    let mut sim = Sim::new(10, 5, DEFAULT_SCROLLBACK_BYTES);
    for i in 0..4 {
        sim.print(&format!("l{i}\n"));
    }
    sim.print("prompt");
    let c = sim.grid.resize(10, 3, cursor(&sim), true);
    assert_eq!(screen(&sim.grid), ["l2", "l3", "prompt"]);
    assert_eq!(c, at(2, 6));
    assert_eq!(sim.grid.history_len(), 2);
    assert_eq!(all_rows(&sim.grid)[..2], ["l0", "l1"]);
    ok(&sim.grid);
}

#[test]
fn growing_the_height_of_a_full_screen_pulls_history_back() {
    let mut sim = Sim::new(10, 3, DEFAULT_SCROLLBACK_BYTES);
    for i in 0..6 {
        sim.print(&format!("l{i}\n"));
    }
    sim.print("prompt");
    assert_eq!(sim.grid.history_len(), 4);
    let c = sim.grid.resize(10, 6, cursor(&sim), true);
    assert_eq!(screen(&sim.grid), ["l1", "l2", "l3", "l4", "l5", "prompt"]);
    assert_eq!(c, at(5, 6), "the cursor stays on the bottom row");
    assert_eq!(sim.grid.history_len(), 1);
    // Further than the history reaches: blank rows fill the bottom, the text stays at the top.
    let c = sim.grid.resize(10, 12, c, true);
    assert_eq!(sim.grid.history_len(), 0);
    assert_eq!(screen(&sim.grid)[..7], ["l0", "l1", "l2", "l3", "l4", "l5", "prompt"]);
    assert_eq!(c, at(6, 6));
    ok(&sim.grid);
}

#[test]
fn growing_the_height_with_blank_space_under_the_text_adds_blank_rows() {
    let mut sim = Sim::new(10, 6, DEFAULT_SCROLLBACK_BYTES);
    // A history, then a cleared screen with a prompt at the top.
    for i in 0..10 {
        sim.print(&format!("old{i}\n"));
    }
    sim.grid.erase_rows(0..6, BG, false);
    sim.x = 0;
    sim.y = 0;
    sim.print("$");
    let history = sim.grid.history_len();
    let c = sim.grid.resize(10, 9, cursor(&sim), true);
    assert_eq!(c, at(0, 1));
    assert_eq!(sim.grid.history_len(), history, "no history pulled into view");
    assert_eq!(screen(&sim.grid)[0], "$");
    ok(&sim.grid);
}

#[test]
fn shrinking_the_height_drops_blank_rows_before_pushing_text() {
    let mut sim = Sim::new(10, 8, DEFAULT_SCROLLBACK_BYTES);
    sim.print("one\ntwo\nthree");
    let c = sim.grid.resize(10, 4, cursor(&sim), true);
    assert_eq!(screen(&sim.grid), ["one", "two", "three", ""]);
    assert_eq!(sim.grid.history_len(), 0);
    assert_eq!(c, at(2, 5));
    let c = sim.grid.resize(10, 2, c, true);
    assert_eq!(screen(&sim.grid), ["two", "three"]);
    assert_eq!(sim.grid.history_len(), 1);
    assert_eq!(c, at(1, 5));
    ok(&sim.grid);
}

#[test]
fn the_cursor_stays_on_a_screen_that_shrinks_a_lot() {
    let mut sim = Sim::new(10, 10, DEFAULT_SCROLLBACK_BYTES);
    sim.print("a\nb\nc");
    sim.y = 1;
    let c = sim.grid.resize(10, 1, at(1, 0), true);
    assert_eq!(c.line.0, 0);
    assert_eq!(screen(&sim.grid), ["b"]);
    ok(&sim.grid);
}

#[test]
fn wrapping_lines_are_pushed_into_the_history_and_survive_there() {
    let mut sim = Sim::new(20, 4, DEFAULT_SCROLLBACK_BYTES);
    for i in 0..4 {
        sim.print(&format!("line number {i} is a bit long\n"));
    }
    let c = sim.grid.resize(7, 4, cursor(&sim), true);
    assert_eq!(
        logical_lines(&sim.grid),
        (0..4).map(|i| format!("line number {i} is a bit long")).collect::<Vec<_>>()
    );
    assert!(sim.grid.history_len() > 0);
    assert_eq!(c.line.0, 3);
    ok(&sim.grid);
}

#[test]
fn history_lines_wrapped_across_the_screen_edge_are_joined() {
    // One long line that starts in the history and ends on the screen.
    let mut sim = Sim::new(5, 3, DEFAULT_SCROLLBACK_BYTES);
    sim.print("a\nb\n");
    sim.print("0123456789abcdefghij");
    assert!(sim.grid.history_len() > 0);
    let c = sim.grid.resize(20, 3, cursor(&sim), true);
    assert_eq!(logical_lines(&sim.grid), ["a", "b", "0123456789abcdefghij"]);
    assert_eq!(sim.grid.history_len(), 0);
    assert_eq!(c.line.0, 2);
    ok(&sim.grid);
}

#[test]
fn pins_follow_their_text_through_a_reflow() {
    let mut sim = Sim::new(20, 6, DEFAULT_SCROLLBACK_BYTES);
    sim.print("first line\nthe second line is long");
    let first = sim.grid.pin(at(0, 6));
    let second = sim.grid.pin(at(1, 11));
    sim.grid.resize(8, 6, cursor(&sim), true);
    let text = |g: &Grid, p: Option<Point>| p.and_then(|p| g.cell(p)).and_then(|c| c.character());
    let (a, b) = (sim.grid.pin_point(first), sim.grid.pin_point(second));
    assert_eq!(text(&sim.grid, a), Some('l'));
    assert_eq!(text(&sim.grid, b), Some('l'));
    assert_eq!(a.map(|p| p.line.0), Some(0));
    assert_eq!(b.map(|p| (p.line.0, p.column.0)), Some((3, 3)), "the 'l' of 'line' in the second line");
    ok(&sim.grid);
}

#[test]
fn the_view_stays_on_the_text_it_showed() {
    let mut sim = Sim::new(10, 4, DEFAULT_SCROLLBACK_BYTES);
    for i in 0..40 {
        sim.print(&format!("{i}\n"));
    }
    sim.grid.scroll_view(crate::types::Scroll::Delta(10));
    let top = sim.grid.row_text(Line(-10), true);
    sim.grid.resize(30, 4, at(3, 0), true);
    let offset = sim.grid.display_offset();
    assert_eq!(sim.grid.row_text(Line(-(offset as i32)), true), top);
    ok(&sim.grid);
}

#[test]
fn without_reflow_rows_are_cropped_and_padded() {
    let mut g = Grid::new(10, 3, 0);
    g.write_ascii(0, 0, b"0123456789", &plain());
    g.write_ascii(1, 0, b"abcdefghij", &colored(3));
    g.set_wrapped(0, true);
    let c = g.resize(6, 3, at(1, 9), false);
    assert_eq!(screen(&g), ["012345", "abcdef", ""]);
    assert_eq!(c, at(1, 5));
    ok(&g);
    g.resize(12, 3, c, false);
    assert_eq!(screen(&g), ["012345", "abcdef", ""]);
    ok(&g);
}

#[test]
fn cropping_through_a_wide_character_blanks_it() {
    let mut g = Grid::new(6, 2, 0);
    g.write_ascii(0, 0, b"ab", &plain());
    g.write_cluster(0, 2, '日', &[], 2, &plain());
    g.resize(3, 2, at(0, 0), false);
    assert_eq!(screen(&g)[0], "ab");
    ok(&g);
}

#[test]
fn resizing_to_the_same_size_does_nothing() {
    let mut sim = Sim::new(10, 4, DEFAULT_SCROLLBACK_BYTES);
    sim.print("abc");
    let c = sim.grid.resize(10, 4, cursor(&sim), true);
    assert_eq!(c, cursor(&sim));
    assert_eq!(screen(&sim.grid)[0], "abc");
}

#[test]
fn a_zero_sized_resize_is_clamped() {
    let mut sim = Sim::new(10, 4, DEFAULT_SCROLLBACK_BYTES);
    sim.print("abcdef");
    sim.grid.resize(0, 0, at(0, 0), true);
    assert_eq!((sim.grid.cols(), sim.grid.rows()), (crate::grid::MIN_COLS, 1));
    ok(&sim.grid);
}

#[test]
fn the_history_cap_holds_after_a_reflow() {
    let limit = 64 * 1024;
    let mut sim = Sim::new(40, 10, limit);
    for i in 0..2000 {
        sim.print(&format!("line {i}\n"));
    }
    sim.grid.resize(10, 10, at(9, 0), true);
    assert!(sim.grid.history_len() * sim.grid.row_cost() <= limit);
    ok(&sim.grid);
}

// ---- random text at random widths ----

const WORDS: [&str; 10] = ["a", "bc", "lorem", "ipsum", "x", "dolor", "日", "本語", "e\u{301}", "ok"];

fn random_text(rng: &mut Rng, sim: &mut Sim, wide: bool) {
    let lines = rng.range(1, 12);
    for _ in 0..lines {
        let words = rng.range(0, 14);
        for _ in 0..words {
            let word = loop {
                let w = WORDS[rng.below(WORDS.len())];
                if wide || w.is_ascii() || w == "e\u{301}" {
                    break w;
                }
            };
            sim.print(word);
            if rng.chance(60) {
                sim.print(" ");
            }
        }
        sim.print("\n");
    }
    if rng.chance(50) {
        sim.print("prompt> typed");
    }
}

/// Every row of a logical line but its last is full, and the last isn't marked wrapped.
fn check_wrapping(grid: &Grid) {
    let history = grid.history_len() as i32;
    for y in -history..grid.rows() as i32 {
        if grid.is_wrapped(Line(y)) && y + 1 < grid.rows() as i32 {
            let cells = grid.cols();
            let last = grid.cell(Point::new(Line(y), Column(cells - 1)));
            let before = grid.cell(Point::new(Line(y), Column(cells - 2)));
            let full = last.is_some_and(|c| !c.is_blank() || c.is_spacer_head());
            assert!(
                full || before.is_some_and(|c| !c.is_blank()),
                "row {y} wraps but has room: {:?}",
                grid.row_text(Line(y), false)
            );
        }
    }
}

fn reflow_property(wide: bool) {
    for seed in 1..=seeds(250) {
        let mut rng = Rng::new(seed * 2_654_435_761);
        let (cols, rows) = (rng.range(4, 30), rng.range(2, 10));
        let mut sim = Sim::new(cols, rows, 1 << 24);
        random_text(&mut rng, &mut sim, wide);
        let before = logical_lines(&sim.grid);
        let mut c = cursor(&sim);
        let mut rows_now = rows;
        for step in 0..6 {
            let new_cols = rng.range(2, 40);
            if rng.chance(50) {
                rows_now = rng.range(1, 14);
            }
            c = sim.grid.resize(new_cols, rows_now, c, true);
            let context = format!("seed {seed} step {step} width {new_cols}x{rows_now}");
            if let Err(e) = sim.grid.check_invariants() {
                panic!("{e}: {context}");
            }
            assert_eq!(logical_lines(&sim.grid), before, "{context}");
            check_wrapping(&sim.grid);
            assert!((c.line.0 as usize) < rows_now && c.column.0 < new_cols, "{context}: cursor {c:?}");
        }
        // And back to the start: the same text on the same width.
        sim.grid.resize(cols, rows, c, true);
        assert_eq!(logical_lines(&sim.grid), before, "seed {seed} round trip");
        sim.check();
    }
}

#[test]
fn text_survives_random_resizes() {
    reflow_property(false);
}

#[test]
fn text_with_wide_characters_survives_random_resizes() {
    reflow_property(true);
}

#[test]
fn growing_wide_enough_makes_every_line_one_row() {
    for seed in 1..=seeds(60) {
        let mut rng = Rng::new(seed * 7_368_787);
        let mut sim = Sim::new(rng.range(4, 12), rng.range(3, 8), 1 << 24);
        random_text(&mut rng, &mut sim, true);
        let lines = logical_lines(&sim.grid).len();
        let c = cursor(&sim);
        sim.grid.resize(2000, 8, c, true);
        let rows_used: usize =
            (-(sim.grid.history_len() as i32)..8).filter(|&y| !sim.grid.is_wrapped(Line(y))).count();
        assert!(rows_used >= lines, "seed {seed}");
        assert!(
            (-(sim.grid.history_len() as i32)..8).all(|y| !sim.grid.is_wrapped(Line(y))),
            "seed {seed}: a wrap is left"
        );
        sim.check();
    }
}

#[test]
fn the_cursor_and_pins_survive_a_reflow_after_the_history_was_cut() {
    // Serial numbers don't start at zero once old pages are dropped.
    let mut sim = Sim::new(30, 6, 32 * 1024);
    for i in 0..3000 {
        sim.print(&format!("line {i} with some text to make it long enough\n"));
    }
    sim.print("the last line");
    let before = sim.grid.history_len();
    let pin = sim.grid.pin(at(5, 4));
    let c = sim.grid.resize(12, 6, cursor(&sim), true);
    assert!(sim.grid.history_len() > 0 && before > 0);
    assert_eq!(c.line.0, 5, "the cursor is on the last row");
    let found = sim.grid.pin_point(pin).and_then(|p| sim.grid.cell(p)).and_then(|c| c.character());
    assert_eq!(found, Some('l'), "'the last line': the l of 'last'");
    ok(&sim.grid);
}

#[test]
fn pins_follow_unique_characters_through_random_resizes() {
    let letters: Vec<char> = ('a'..='z').chain('A'..='Z').chain('0'..='9').collect();
    for seed in 1..=seeds(200) {
        let mut rng = Rng::new(seed * 40503);
        let (cols, rows) = (rng.range(5, 25), rng.range(2, 8));
        // A small cap too, so that the oldest rows are gone and serial numbers are large.
        let limit = if rng.chance(50) { 1 << 24 } else { 3 * 1024 };
        let mut sim = Sim::new(cols, rows, limit);
        let mut at_end = Vec::new();
        for c in &letters {
            if rng.chance(15) {
                sim.print("\n");
            }
            sim.print(&c.to_string());
            if rng.chance(12) {
                sim.print(" ");
            }
        }
        // Pin every letter still on the screen.
        for y in 0..rows as i32 {
            for x in 0..cols {
                if let Some(ch) =
                    sim.grid.cell(at(y, x)).and_then(|c| c.character()).filter(|c| c.is_alphanumeric())
                {
                    at_end.push((ch, sim.grid.pin(at(y, x))));
                }
            }
        }
        let mut c = cursor(&sim);
        for step in 0..5 {
            let (new_cols, new_rows) = (rng.range(2, 30), rng.range(1, 10));
            c = sim.grid.resize(new_cols, new_rows, c, true);
            if let Err(e) = sim.grid.check_invariants() {
                panic!("{e}: seed {seed} step {step}");
            }
            for &(ch, pin) in &at_end {
                // A pin may have been dropped with the rows that fell off the end of the screen.
                if let Some(p) = sim.grid.pin_point(pin) {
                    let found = sim.grid.cell(p).and_then(|c| c.character());
                    assert_eq!(found, Some(ch), "seed {seed} step {step} width {new_cols}x{new_rows}");
                }
            }
        }
    }
}

#[test]
fn command_blocks_stay_marked_through_a_reflow() {
    use crate::grid::BlockPart;
    let mut sim = Sim::new(30, 8, DEFAULT_SCROLLBACK_BYTES);
    let pen = |content, block| Pen { content, block, ..plain() };
    sim.pen = pen(BlockPart::Prompt, 3);
    sim.print("user@host:~$ ");
    sim.pen = pen(BlockPart::Command, 3);
    sim.print("echo a long command line that wraps");
    sim.newline();
    sim.pen = pen(BlockPart::Output, 3);
    sim.print("the output of it, which is long enough to wrap too");
    sim.newline();
    sim.pen = plain();
    assert!(sim.grid.row_mark(Line(0)).is_some(), "marked before");
    sim.grid.resize(16, 8, cursor(&sim), true);
    // The prompt row carries the prompt and the start of the command; every row of the block says
    // which block it belongs to.
    let marks: Vec<_> = (0..sim.grid.rows() as i32 + sim.grid.history_len() as i32)
        .map(|i| sim.grid.row_mark(Line(i - sim.grid.history_len() as i32)))
        .collect();
    let first = marks.iter().flatten().next().copied();
    assert_eq!(first.map(|m| (m.block, m.prompt, m.command, m.output)), Some((3, true, true, false)));
    let blocks = marks.iter().flatten().count();
    let rows_with_text = logical_lines(&sim.grid).iter().map(|l| l.len().div_ceil(16)).sum::<usize>();
    assert!(blocks >= rows_with_text - 1, "{blocks} marked rows for about {rows_with_text} of text");
    assert!(marks.iter().flatten().all(|m| m.block == 3));
    assert!(marks.iter().flatten().any(|m| m.output && !m.prompt && !m.command));
    ok(&sim.grid);
}

#[test]
fn a_wide_cluster_that_moves_to_the_next_row_takes_its_marks_with_it() {
    let mut g = Grid::new(6, 3, DEFAULT_SCROLLBACK_BYTES);
    g.write_ascii(0, 0, b"ab", &plain());
    assert!(g.write_cluster(0, 2, '日', &['\u{FE0F}'], 2, &plain()));
    g.write_ascii(0, 4, b"cd", &plain());
    g.set_wrapped(0, false);
    // Narrower by one column: the wide character no longer fits where it was.
    g.resize(5, 3, at(0, 0), true);
    assert_eq!(screen(&g)[0], "ab日\u{FE0F}c", "{:?}", screen(&g));
    g.resize(4, 3, at(0, 0), true);
    assert_eq!(screen(&g)[..2], ["ab日\u{FE0F}", "cd"].map(str::to_string));
    g.resize(3, 3, at(0, 0), true);
    // Three columns: "ab" then the wide one, with a spacer head after "ab"; "c" fits after it.
    assert_eq!(screen(&g)[..3], ["ab", "日\u{FE0F}c", "d"].map(str::to_string));
    assert!(g.cell(at(0, 2)).is_some_and(|c| c.is_spacer_head()));
    assert_eq!(g.marks_at(at(1, 0)), &['\u{FE0F}']);
    ok(&g);
}

#[test]
fn resizing_with_arguments_outside_the_grid_clamps_them() {
    let huge = usize::MAX;
    let mut g = Grid::new(6, 3, DEFAULT_SCROLLBACK_BYTES);
    g.write_ascii(1, 0, b"abc", &plain());
    // (Both at once would be 16,384 by 16,384 cells: 2 GB.)
    let _ = g.resize(huge, 3, Point::new(Line(i32::MAX), Column(huge)), true);
    assert_eq!((g.cols(), g.rows()), (crate::grid::MAX_COLS, 3));
    let _ = g.resize(2, huge, Point::new(Line(1), Column(huge)), false);
    assert_eq!((g.cols(), g.rows()), (2, crate::grid::MAX_ROWS));
    let back = g.resize(0, 0, Point::new(Line(i32::MIN), Column(0)), true);
    assert_eq!((g.cols(), g.rows(), back.line.0), (crate::grid::MIN_COLS, 1, 0));
    ok(&g);
}
