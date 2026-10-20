//! Scrolling, the history and its cap, the view, pins.

use super::{BG, Sim, all_rows, at, colored, full, plain, screen};
use crate::grid::{DEFAULT_SCROLLBACK_BYTES, Grid, Region};
use crate::types::{Color, Line, NamedColor, Scroll};

fn ok(grid: &Grid) {
    if let Err(e) = grid.check_invariants() {
        panic!("{e}");
    }
}

fn numbered(grid: &mut Grid, rows: usize) {
    for y in 0..rows {
        grid.write_ascii(y, 0, format!("line{y}").as_bytes(), &plain());
    }
}

#[test]
fn scrolling_the_screen_moves_lines_into_the_history() {
    let mut g = Grid::new(10, 4, DEFAULT_SCROLLBACK_BYTES);
    numbered(&mut g, 4);
    let region = full(&g);
    g.scroll_up(region, 1, BG);
    assert_eq!(screen(&g), ["line1", "line2", "line3", ""]);
    assert_eq!(g.history_len(), 1);
    assert_eq!(g.row_text(Line(-1), true), "line0");
    g.scroll_up(region, 2, BG);
    assert_eq!(screen(&g), ["line3", "", "", ""]);
    assert_eq!(all_rows(&g)[..3], ["line0", "line1", "line2"]);
    ok(&g);
}

#[test]
fn scrolling_more_than_the_screen_clears_it_into_the_history() {
    let mut g = Grid::new(10, 3, DEFAULT_SCROLLBACK_BYTES);
    numbered(&mut g, 3);
    let region = full(&g);
    g.scroll_up(region, 99, BG);
    assert_eq!(screen(&g), ["", "", ""]);
    assert_eq!(g.history_len(), 3, "the three lines the screen had, not ninety-nine");
    ok(&g);
}

#[test]
fn a_screen_without_history_loses_what_scrolls_off() {
    let mut g = Grid::new(10, 3, 0);
    numbered(&mut g, 3);
    let region = full(&g);
    g.scroll_up(region, 1, BG);
    assert_eq!(screen(&g), ["line1", "line2", ""]);
    assert_eq!(g.history_len(), 0);
    ok(&g);
}

#[test]
fn a_full_screen_scroll_copies_no_cell() {
    // Rows keep their cells: a pin on a row stays on its text as the screen scrolls, and the
    // memory in pages doesn't move.
    let mut g = Grid::new(80, 24, DEFAULT_SCROLLBACK_BYTES);
    numbered(&mut g, 24);
    let pin = g.pin(at(10, 2));
    let region = full(&g);
    for k in 1..=5 {
        g.scroll_up(region, 1, BG);
        assert_eq!(g.pin_point(pin), Some(at(10 - k, 2)));
    }
    assert_eq!(g.row_text(Line(5), true), "line10", "the text the pin was on");
    ok(&g);
}

#[test]
fn a_region_scrolls_only_its_own_lines() {
    let mut g = Grid::new(10, 6, DEFAULT_SCROLLBACK_BYTES);
    numbered(&mut g, 6);
    let region = Region { top: 1, bottom: 3, left: 0, right: 9 };
    g.scroll_up(region, 1, BG);
    assert_eq!(screen(&g), ["line0", "line2", "line3", "", "line4", "line5"]);
    assert_eq!(g.history_len(), 0, "a region that doesn't start at the top feeds no history");
    g.scroll_down(region, 2, BG);
    assert_eq!(screen(&g), ["line0", "", "", "line2", "line4", "line5"]);
    ok(&g);
}

#[test]
fn a_region_from_the_top_feeds_the_history_and_keeps_the_lines_below() {
    // A status line under a scroll region: the region's lines go to the history, the status stays.
    for page_rows in [1, 2, 3, 100] {
        let mut g = Grid::with_page_rows(10, 5, DEFAULT_SCROLLBACK_BYTES, page_rows);
        numbered(&mut g, 5);
        let region = Region { top: 0, bottom: 2, left: 0, right: 9 };
        g.scroll_up(region, 1, BG);
        assert_eq!(screen(&g), ["line1", "line2", "", "line3", "line4"], "pages of {page_rows}");
        assert_eq!(g.history_len(), 1);
        assert_eq!(g.row_text(Line(-1), true), "line0");
        g.scroll_up(region, 2, BG);
        assert_eq!(screen(&g), ["", "", "", "line3", "line4"]);
        assert_eq!(all_rows(&g)[..3], ["line0", "line1", "line2"]);
        ok(&g);
    }
}

#[test]
fn a_region_of_one_line_scrolls_by_clearing_it() {
    let mut g = Grid::new(10, 4, DEFAULT_SCROLLBACK_BYTES);
    numbered(&mut g, 4);
    g.scroll_up(Region { top: 2, bottom: 2, left: 0, right: 9 }, 5, BG);
    assert_eq!(screen(&g), ["line0", "line1", "", "line3"]);
    g.scroll_down(Region { top: 1, bottom: 1, left: 0, right: 9 }, 1, BG);
    assert_eq!(screen(&g), ["line0", "", "", "line3"]);
    ok(&g);
}

#[test]
fn margins_that_are_empty_or_reversed_scroll_nothing() {
    let mut g = Grid::new(10, 4, DEFAULT_SCROLLBACK_BYTES);
    numbered(&mut g, 4);
    let before = screen(&g);
    g.scroll_up(Region { top: 3, bottom: 1, left: 0, right: 9 }, 1, BG);
    g.scroll_up(Region { top: 0, bottom: 3, left: 6, right: 2 }, 1, BG);
    g.scroll_down(Region { top: 9, bottom: 12, left: 0, right: 9 }, 1, BG);
    g.insert_lines(0, Region { top: 2, bottom: 3, left: 0, right: 9 }, 1, BG);
    g.delete_lines(3, Region { top: 0, bottom: 2, left: 0, right: 9 }, 1, BG);
    g.scroll_up(Region::full(10, 4), 0, BG);
    assert_eq!(screen(&g), before);
    assert_eq!(g.history_len(), 0);
    ok(&g);
}

#[test]
fn left_and_right_margins_move_only_their_columns() {
    let mut g = Grid::new(8, 4, DEFAULT_SCROLLBACK_BYTES);
    for y in 0..4 {
        g.write_ascii(y, 0, format!("{y}abcdef{y}").as_bytes(), &plain());
    }
    let region = Region { top: 0, bottom: 3, left: 2, right: 5 };
    g.scroll_up(region, 1, BG);
    assert_eq!(screen(&g), ["0abcdef0", "1abcdef1", "2abcdef2", "3a    f3"].map(str::to_string));
    ok(&g);
}

#[test]
fn insert_and_delete_lines_work_inside_the_region() {
    let mut g = Grid::new(10, 6, DEFAULT_SCROLLBACK_BYTES);
    numbered(&mut g, 6);
    let region = Region { top: 1, bottom: 4, left: 0, right: 9 };
    g.insert_lines(2, region, 2, BG);
    assert_eq!(
        screen(&g),
        ["line0", "line1", "", "", "line2", "line5"],
        "line3 and line4 fell off the region"
    );
    g.delete_lines(1, region, 1, BG);
    assert_eq!(screen(&g), ["line0", "", "", "line2", "", "line5"]);
    g.delete_lines(0, region, 1, BG);
    g.insert_lines(5, region, 1, BG);
    assert_eq!(screen(&g)[0], "line0", "a line outside the region is left alone");
    assert_eq!(g.history_len(), 0, "inserting and deleting lines never feeds the history");
    ok(&g);
}

#[test]
fn deleting_lines_at_the_top_of_the_screen_does_not_feed_the_history() {
    let mut g = Grid::new(10, 3, DEFAULT_SCROLLBACK_BYTES);
    numbered(&mut g, 3);
    let region = full(&g);
    g.delete_lines(0, region, 1, BG);
    assert_eq!(screen(&g), ["line1", "line2", ""]);
    assert_eq!(g.history_len(), 0);
    ok(&g);
}

#[test]
fn new_lines_take_the_background_of_the_pen() {
    let mut g = Grid::new(6, 3, DEFAULT_SCROLLBACK_BYTES);
    numbered(&mut g, 3);
    let blue = Color::Named(NamedColor::Blue);
    let region = full(&g);
    g.scroll_up(region, 1, blue);
    assert!(g.style_at(at(2, 3)).is_some_and(|s| s.bg == blue));
    g.scroll_down(region, 1, blue);
    assert!(g.style_at(at(0, 3)).is_some_and(|s| s.bg == blue));
    g.scroll_up(region, 1, BG);
    assert!(g.cell(at(2, 3)).is_some_and(|c| c.is_blank()));
    ok(&g);
}

#[test]
fn rows_scroll_across_page_boundaries_with_their_styles() {
    // Pages of two rows: every region scroll copies rows between pages and re-interns styles.
    let mut g = Grid::with_page_rows(8, 6, DEFAULT_SCROLLBACK_BYTES, 2);
    for y in 0..6 {
        g.write_ascii(y, 0, format!("row{y}").as_bytes(), &colored(y as u8 + 1));
        g.set_wrapped(y, y % 2 == 0);
    }
    let region = Region { top: 1, bottom: 4, left: 0, right: 7 };
    g.scroll_up(region, 1, BG);
    assert_eq!(screen(&g), ["row0", "row2", "row3", "row4", "", "row5"]);
    assert_eq!(g.style_at(at(1, 0)), Some(colored(3).style), "the style went with its text");
    assert!(g.is_wrapped(Line(1)), "so did the wrap flag of row2");
    assert!(!g.is_wrapped(Line(2)));
    ok(&g);
}

#[test]
fn the_history_is_capped_in_bytes_and_drops_the_oldest_lines() {
    let limit = 200 * 1024;
    let mut sim = Sim::new(80, 24, limit);
    for i in 0..20_000 {
        sim.print(&format!("line {i}\n"));
    }
    let g = &sim.grid;
    ok(g);
    let row_bytes = g.row_cost();
    assert!(g.history_len() * row_bytes <= limit, "{} lines of {row_bytes} bytes", g.history_len());
    assert!(
        g.history_len() * row_bytes > limit - 64 * 1024,
        "within two pages of the limit: {}",
        g.history_len()
    );
    let all = all_rows(g);
    assert!(all.last().is_some_and(|l| l.is_empty()));
    assert_eq!(all[all.len() - 2], "line 19999");
    // Contiguous: the oldest kept line is followed by its successors.
    let first: usize = all[0].trim_start_matches("line ").parse().unwrap_or(usize::MAX);
    for (k, row) in all[..g.history_len()].iter().enumerate() {
        assert_eq!(*row, format!("line {}", first + k));
    }
    // Memory stays near the limit however much more is printed.
    let held = g.memory_bytes();
    assert!(held < limit + 256 * 1024, "{held} bytes held");
}

#[test]
fn a_smaller_limit_drops_history_at_once() {
    let mut sim = Sim::new(40, 10, 1 << 20);
    for i in 0..3000 {
        sim.print(&format!("{i}\n"));
    }
    let before = sim.grid.history_len();
    sim.grid.set_scrollback_limit(20 * 1024);
    assert!(sim.grid.history_len() < before / 10);
    ok(&sim.grid);
    sim.grid.set_scrollback_limit(0);
    assert_eq!(sim.grid.history_len(), 0);
    assert_eq!(screen(&sim.grid).last().map(String::as_str), Some(""));
    ok(&sim.grid);
}

#[test]
fn the_view_scrolls_inside_the_history() {
    let mut sim = Sim::new(10, 4, 1 << 20);
    for i in 0..30 {
        sim.print(&format!("{i}\n"));
    }
    let g = &mut sim.grid;
    assert_eq!(g.display_offset(), 0);
    g.scroll_view(Scroll::Delta(5));
    assert_eq!(g.display_offset(), 5);
    g.scroll_view(Scroll::Delta(-2));
    assert_eq!(g.display_offset(), 3);
    g.scroll_view(Scroll::Delta(1000));
    assert_eq!(g.display_offset(), g.history_len());
    g.scroll_view(Scroll::Bottom);
    assert_eq!(g.display_offset(), 0);
    g.scroll_view(Scroll::Top);
    assert_eq!(g.display_offset(), g.history_len());
    g.scroll_view(Scroll::Delta(-1000));
    assert_eq!(g.display_offset(), 0);
}

#[test]
fn output_below_a_scrolled_view_leaves_the_view_on_its_text() {
    let mut sim = Sim::new(10, 4, 1 << 20);
    for i in 0..30 {
        sim.print(&format!("{i}\n"));
    }
    sim.grid.scroll_view(Scroll::Delta(3));
    let shown = sim.grid.row_text(Line(-3), true);
    sim.print("new\nnewer\n");
    assert_eq!(sim.grid.display_offset(), 5);
    assert_eq!(sim.grid.row_text(Line(-5), true), shown);
    ok(&sim.grid);
}

#[test]
fn the_view_follows_the_history_when_it_is_cut() {
    let mut sim = Sim::new(80, 10, 100 * 1024);
    for i in 0..500 {
        sim.print(&format!("{i}\n"));
    }
    sim.grid.scroll_view(Scroll::Top);
    for i in 0..5000 {
        sim.print(&format!("{i}\n"));
    }
    assert!(sim.grid.display_offset() <= sim.grid.history_len());
    ok(&sim.grid);
}

#[test]
fn a_pin_says_when_its_row_was_dropped_from_the_history() {
    let mut sim = Sim::new(80, 10, 100 * 1024);
    sim.print("mark\n");
    let pin = sim.grid.pin(at(0, 0));
    assert_eq!(sim.grid.pin_point(pin), Some(at(0, 0)));
    for i in 0..3000 {
        sim.print(&format!("filler {i}\n"));
    }
    assert_eq!(sim.grid.pin_point(pin), None, "the row is long gone");
    let kept = sim.grid.pin(at(8, 0));
    sim.print("a\nb\nc\n");
    assert_eq!(sim.grid.pin_point(kept).map(|p| p.line.0), Some(5), "recent history is kept, and followed");
    sim.grid.unpin(kept);
    assert_eq!(sim.grid.pin_point(kept), None);
    ok(&sim.grid);
}

#[test]
fn pins_are_kept_on_the_screen_and_can_be_moved() {
    let mut g = Grid::new(10, 4, DEFAULT_SCROLLBACK_BYTES);
    let pin = g.pin(at(2, 7));
    assert_eq!(g.pin_point(pin), Some(at(2, 7)));
    g.move_pin(pin, at(99, 99));
    assert_eq!(g.pin_point(pin), Some(at(3, 9)), "clamped into the grid");
    g.move_pin(pin, at(-9, 0));
    assert_eq!(g.pin_point(pin), Some(at(0, 0)), "and into the history that exists");
}

#[test]
fn clearing_the_screen_rows_into_the_history_keeps_them() {
    let mut g = Grid::new(10, 4, DEFAULT_SCROLLBACK_BYTES);
    numbered(&mut g, 4);
    g.erase_rows(0..4, BG, false);
    assert_eq!(screen(&g), ["", "", "", ""]);
    assert_eq!(g.history_len(), 0);
    ok(&g);
}

#[test]
fn the_history_can_be_cleared_and_the_screen_stays() {
    let mut sim = Sim::new(10, 3, DEFAULT_SCROLLBACK_BYTES);
    for i in 0..20 {
        sim.print(&format!("{i}\n"));
    }
    let pin_in_history = sim.grid.pin(at(-5, 0));
    let pin_on_screen = sim.grid.pin(at(0, 0));
    let on_screen = screen(&sim.grid);
    sim.grid.scroll_view(Scroll::Delta(4));
    sim.grid.clear_scrollback();
    assert_eq!(sim.grid.history_len(), 0);
    assert_eq!(sim.grid.display_offset(), 0);
    assert_eq!(screen(&sim.grid), on_screen);
    assert_eq!(sim.grid.pin_point(pin_in_history), None);
    assert_eq!(sim.grid.pin_point(pin_on_screen), Some(at(0, 0)));
    ok(&sim.grid);
    // New output fills the history again.
    sim.print("more\nmore\nmore\n");
    assert!(sim.grid.history_len() > 0);
    ok(&sim.grid);
}

#[test]
fn a_reset_blanks_everything() {
    let mut sim = Sim::new(10, 3, DEFAULT_SCROLLBACK_BYTES);
    sim.print("a\nb\nc\nd\ne\n");
    sim.grid.reset();
    assert_eq!((sim.grid.history_len(), screen(&sim.grid)), (0, vec![String::new(); 3]));
    ok(&sim.grid);
}

#[test]
fn clearing_into_the_history_keeps_the_text_and_adds_no_blank_lines() {
    let mut sim = Sim::new(10, 6, DEFAULT_SCROLLBACK_BYTES);
    sim.print("one\ntwo\nthree");
    assert_eq!(sim.grid.used_rows(), 3);
    sim.grid.scroll_screen_into_history(BG);
    assert_eq!(screen(&sim.grid), vec![String::new(); 6]);
    assert_eq!(all_rows(&sim.grid)[..3], ["one", "two", "three"]);
    assert_eq!(sim.grid.history_len(), 3);
    assert_eq!(sim.grid.used_rows(), 0);
    // A blank screen adds nothing.
    sim.grid.scroll_screen_into_history(BG);
    assert_eq!(sim.grid.history_len(), 3);
    ok(&sim.grid);
}

#[test]
fn a_grid_can_move_between_threads() {
    fn is_send<T: Send>() {}
    is_send::<Grid>();
}

#[test]
fn live_cells_are_read_by_row_and_column() {
    let mut g = Grid::new(4, 2, 0);
    g.write_ascii(1, 2, b"x", &plain());
    assert_eq!(g.live_cell(1, 2).and_then(|c| c.character()), Some('x'));
    assert_eq!(g.live_cell(1, 4), None);
    assert_eq!(g.live_cell(2, 0), None);
}

#[test]
fn page_memory_is_reused_once_the_history_is_full() {
    // After the first fill, scrolling more lines takes its pages from the ones it drops.
    let mut sim = Sim::new(80, 24, 64 * 1024);
    for i in 0..2000 {
        sim.print(&format!("line {i}\n"));
    }
    let held = sim.grid.memory_bytes();
    for i in 0..20_000 {
        sim.print(&format!("line {i}\n"));
    }
    assert!(sim.grid.memory_bytes() <= held + 8 * 1024, "{} against {held}", sim.grid.memory_bytes());
    ok(&sim.grid);
}

#[test]
fn a_rows_block_and_wrap_follow_it_when_it_scrolls() {
    for page_rows in [1, 2, 3, 100] {
        let mut g = Grid::with_page_rows(10, 5, DEFAULT_SCROLLBACK_BYTES, page_rows);
        for y in 0..5 {
            let pen =
                crate::grid::Pen { content: crate::grid::BlockPart::Output, block: y as u32 + 1, ..plain() };
            g.write_ascii(y, 0, format!("r{y}").as_bytes(), &pen);
            g.set_wrapped(y, y % 2 == 1);
        }
        let blocks =
            |g: &Grid| -> Vec<Option<u32>> { (0..5).map(|y| g.row_mark(Line(y)).map(|m| m.block)).collect() };
        let wraps = |g: &Grid| -> Vec<bool> { (0..5).map(|y| g.is_wrapped(Line(y))).collect() };
        let region = Region { top: 1, bottom: 3, left: 0, right: 9 };
        g.scroll_up(region, 1, BG);
        assert_eq!(blocks(&g), [Some(1), Some(3), Some(4), None, Some(5)], "pages of {page_rows}");
        assert_eq!(wraps(&g), [false, false, true, false, false]);
        g.scroll_down(region, 1, BG);
        assert_eq!(blocks(&g), [Some(1), None, Some(3), Some(4), Some(5)]);
        let full = full(&g);
        g.scroll_up(full, 2, BG);
        assert_eq!(blocks(&g), [Some(3), Some(4), Some(5), None, None]);
        assert_eq!(
            g.row_mark(Line(-1)).map(|m| m.block),
            None,
            "a blank row went into the history, then the fifth did not"
        );
        ok(&g);
    }
}
