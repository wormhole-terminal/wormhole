//! The snapshot copy: rows as the types a host draws, and which rows are copied.

use super::{BG, Sim, at, bold, colored, full, plain};
use crate::grid::{BlockPart, DEFAULT_SCROLLBACK_BYTES, Grid, Pen, Style, ViewRow};
use crate::types::{Cell, Color, Flags, Hyperlink, Line, NamedColor, RowExtras, Scroll};

fn row(grid: &Grid, line: i32) -> (Vec<Cell>, RowExtras) {
    let (mut cells, mut extras) = (Vec::new(), RowExtras::default());
    grid.copy_row(Line(line), &mut cells, Some(&mut extras));
    (cells, extras)
}

#[test]
fn a_row_is_copied_with_its_colors_and_attributes() {
    let mut g = Grid::new(6, 2, DEFAULT_SCROLLBACK_BYTES);
    g.write_ascii(0, 0, b"ab", &colored(3));
    g.write_ascii(0, 2, b"c", &bold());
    let (cells, extras) = row(&g, 0);
    assert_eq!(cells.len(), 6);
    assert_eq!(cells[0], colored(3).style.draw('a'));
    assert_eq!(cells[1].fg, Color::Indexed(3));
    assert_eq!(cells[2].flags, Flags::BOLD);
    assert_eq!(cells[3], Cell::default(), "blank cells are the default cell");
    assert!(extras.is_empty());
}

#[test]
fn wide_characters_and_their_spacers_are_flagged() {
    let mut g = Grid::new(6, 2, DEFAULT_SCROLLBACK_BYTES);
    g.write_cluster(0, 0, '日', &[], 2, &plain());
    g.write_cluster(0, 2, 'x', &[], 1, &plain());
    g.write_spacer_head(0, &plain());
    let (cells, _) = row(&g, 0);
    assert_eq!((cells[0].c, cells[0].flags), ('日', Flags::WIDE_CHAR));
    assert_eq!((cells[1].c, cells[1].flags), (' ', Flags::WIDE_CHAR_SPACER));
    assert_eq!(cells[5].flags, Flags::LEADING_WIDE_CHAR_SPACER);
}

#[test]
fn the_last_cell_of_a_wrapped_row_says_so() {
    let mut g = Grid::new(4, 2, DEFAULT_SCROLLBACK_BYTES);
    g.write_ascii(0, 0, b"abcd", &plain());
    g.set_wrapped(0, true);
    assert!(row(&g, 0).0[3].flags.contains(Flags::WRAPLINE));
    assert!(!row(&g, 0).0[2].flags.contains(Flags::WRAPLINE));
    assert!(!row(&g, 1).0[3].flags.contains(Flags::WRAPLINE));
}

#[test]
fn marks_links_and_underline_colors_go_to_the_extras() {
    let mut g = Grid::new(10, 1, DEFAULT_SCROLLBACK_BYTES);
    let link = Hyperlink { id: "1".into(), uri: "https://a".into() };
    g.write_cluster(0, 0, 'e', &['\u{301}'], 1, &plain());
    let underlined =
        Style { underline_color: Some(Color::Indexed(9)), attrs: Flags::UNDERLINE, ..Style::DEFAULT };
    g.write_ascii(0, 1, b"ab", &Pen { style: underlined, ..plain() });
    g.write_ascii(0, 4, b"link", &Pen { link: Some(link.clone()), ..plain() });
    let (cells, extras) = row(&g, 0);
    assert_eq!(cells[0].c, 'e');
    assert_eq!(extras.zerowidth(0), &['\u{301}']);
    assert_eq!(extras.underline_color(1), Some(Color::Indexed(9)));
    assert_eq!(extras.underline_color(3), None);
    assert_eq!(extras.hyperlink(5), Some(&link));
    let runs: Vec<_> = extras.links().map(|(range, _)| range).collect();
    assert_eq!(runs, vec![4..8], "one run, not four cells");
    assert!(extras.hyperlink(3).is_none() && extras.hyperlink(8).is_none());
}

#[test]
fn two_links_next_to_each_other_stay_two_runs() {
    let mut g = Grid::new(10, 1, DEFAULT_SCROLLBACK_BYTES);
    let a = Hyperlink { id: "1".into(), uri: "https://a".into() };
    let b = Hyperlink { id: "2".into(), uri: "https://b".into() };
    g.write_ascii(0, 0, b"aaa", &Pen { link: Some(a), ..plain() });
    g.write_ascii(0, 3, b"bb", &Pen { link: Some(b), ..plain() });
    let (_, extras) = row(&g, 0);
    assert_eq!(extras.links().map(|(range, _)| range).collect::<Vec<_>>(), [0..3, 3..5]);
}

#[test]
fn the_mark_of_a_block_row_comes_with_the_copy() {
    let mut g = Grid::new(10, 1, DEFAULT_SCROLLBACK_BYTES);
    g.write_ascii(0, 0, b"$ ", &Pen { content: BlockPart::Prompt, block: 4, ..plain() });
    g.write_ascii(0, 2, b"ls", &Pen { content: BlockPart::Command, block: 4, ..plain() });
    let mut cells = Vec::new();
    let mark = g.copy_row(Line(0), &mut cells, None);
    assert_eq!(mark.map(|m| (m.block, m.prompt, m.command, m.output)), Some((4, true, true, false)));
}

#[test]
fn history_and_the_outside_can_be_copied() {
    let mut sim = Sim::new(5, 2, DEFAULT_SCROLLBACK_BYTES);
    sim.print("one\ntwo\nthree");
    let (cells, _) = row(&sim.grid, -1);
    assert_eq!(cells.iter().map(|c| c.c).collect::<String>().trim_end(), "one");
    let mut cells = vec![Cell::default(); 3];
    sim.grid.copy_row(Line(-99), &mut cells, None);
    assert!(cells.is_empty(), "no such line: nothing");
}

fn copy(grid: &mut Grid, rows: &mut Vec<ViewRow>) -> Vec<bool> {
    grid.copy_viewport(rows, false);
    rows.iter().map(|r| r.changed).collect()
}

#[test]
fn the_viewport_copies_only_what_changed() {
    let mut g = Grid::new(10, 4, DEFAULT_SCROLLBACK_BYTES);
    let mut rows = Vec::new();
    assert_eq!(copy(&mut g, &mut rows), [true; 4], "the first copy is everything");
    assert_eq!(copy(&mut g, &mut rows), [false; 4]);
    g.write_ascii(2, 0, b"hi", &plain());
    assert_eq!(copy(&mut g, &mut rows), [false, false, true, false]);
    assert_eq!(rows[2].cells[0].c, 'h');
    g.erase(0, 0..3, BG, false);
    g.set_wrapped(3, true);
    assert_eq!(copy(&mut g, &mut rows), [true, false, false, true]);
    assert!(rows[3].wrapped);
    assert_eq!(copy(&mut g, &mut rows), [false; 4]);
}

#[test]
fn scrolling_changes_every_row_it_moves() {
    let mut g = Grid::new(10, 4, DEFAULT_SCROLLBACK_BYTES);
    let mut rows = Vec::new();
    copy(&mut g, &mut rows);
    let region = full(&g);
    g.scroll_up(region, 1, BG);
    assert_eq!(copy(&mut g, &mut rows), [true; 4]);
    g.scroll_down(region, 1, BG);
    assert_eq!(copy(&mut g, &mut rows), [true; 4]);
    let partial = crate::grid::Region { top: 1, bottom: 2, left: 0, right: 9 };
    g.scroll_up(partial, 1, BG);
    assert_eq!(copy(&mut g, &mut rows), [false, true, true, false]);
}

#[test]
fn moving_the_view_or_resizing_copies_everything() {
    let mut sim = Sim::new(10, 3, DEFAULT_SCROLLBACK_BYTES);
    for i in 0..10 {
        sim.print(&format!("{i}\n"));
    }
    let mut rows = Vec::new();
    copy(&mut sim.grid, &mut rows);
    assert_eq!(copy(&mut sim.grid, &mut rows), [false; 3]);
    sim.grid.scroll_view(Scroll::Delta(2));
    assert_eq!(copy(&mut sim.grid, &mut rows), [true; 3]);
    assert_eq!(rows[0].cells[0].c, '6', "the view shows the history");
    sim.grid.resize(12, 3, at(2, 0), true);
    assert_eq!(copy(&mut sim.grid, &mut rows), [true; 3]);
    assert_eq!(rows[0].cells.len(), 12);
    sim.grid.damage_all();
    assert_eq!(copy(&mut sim.grid, &mut rows), [true; 3]);
}

#[test]
fn damage_can_be_taken_as_flags() {
    let mut g = Grid::new(10, 4, DEFAULT_SCROLLBACK_BYTES);
    let mut damaged = [false; 4];
    assert!(g.take_damage(&mut damaged), "a new grid is all damage");
    assert!(!g.take_damage(&mut damaged));
    assert_eq!(damaged, [false; 4]);
    g.write_ascii(1, 0, b"x", &plain());
    assert!(!g.take_damage(&mut damaged));
    assert_eq!(damaged, [false, true, false, false]);
    assert!(!g.take_damage(&mut damaged));
    assert_eq!(damaged, [false; 4]);
}

#[test]
fn a_style_nobody_uses_any_more_is_not_copied_stale() {
    let mut g = Grid::new(6, 1, DEFAULT_SCROLLBACK_BYTES);
    g.write_ascii(0, 0, b"ab", &colored(1));
    g.write_ascii(0, 0, b"ab", &colored(2));
    g.write_ascii(
        0,
        2,
        b"cd",
        &Pen { style: Style { bg: Color::Named(NamedColor::Red), ..Style::DEFAULT }, ..plain() },
    );
    let (cells, _) = row(&g, 0);
    assert_eq!(cells[0].fg, Color::Indexed(2));
    assert_eq!(cells[2].bg, Color::Named(NamedColor::Red));
}
