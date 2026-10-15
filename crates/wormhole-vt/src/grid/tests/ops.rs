//! One operation at a time, and the edges: the last column, wide characters at a margin, half a wide
//! character erased, regions of one cell.

use super::{BG, at, bold, colored, plain, screen};
use crate::grid::{Content, Grid, Pen, Style};
use crate::types::{Color, Flags, Hyperlink, Line, NamedColor};

fn grid(cols: usize, rows: usize) -> Grid {
    Grid::new(cols, rows, 1 << 20)
}

fn ok(grid: &Grid) {
    if let Err(e) = grid.check_invariants() {
        panic!("{e}");
    }
}

#[test]
fn a_new_grid_is_blank_and_valid() {
    let g = grid(10, 4);
    assert_eq!((g.cols(), g.rows(), g.history_len()), (10, 4, 0));
    assert_eq!(screen(&g), ["", "", "", ""]);
    assert!(g.cell(at(0, 0)).is_some_and(|c| c.is_blank()));
    ok(&g);
}

#[test]
fn the_size_is_kept_inside_what_a_page_can_hold() {
    let g = Grid::new(0, 0, 0);
    assert_eq!((g.cols(), g.rows()), (crate::grid::MIN_COLS, 1));
    let g = Grid::new(1_000_000, 1, 0);
    assert_eq!((g.cols(), g.rows()), (crate::grid::MAX_COLS, 1));
    let g = Grid::new(2, 1_000_000, 0);
    assert_eq!((g.cols(), g.rows()), (2, crate::grid::MAX_ROWS));
}

#[test]
fn ascii_is_written_where_asked() {
    let mut g = grid(10, 3);
    assert_eq!(g.write_ascii(1, 2, b"hello", &plain()), 5);
    assert_eq!(screen(&g), ["", "  hello", ""]);
    ok(&g);
}

#[test]
fn a_pen_that_carries_layout_bits_leaves_the_grid_valid() {
    // A caller converting from the drawn flags could leave WIDE_CHAR or WRAPLINE in the pen's
    // attributes; the cell's own flags say where a wide character is, so they never reach a style.
    let mut g = grid(10, 2);
    let pen = Pen {
        style: Style { attrs: Flags::BOLD | Flags::WIDE_CHAR | Flags::WRAPLINE, ..Style::DEFAULT },
        ..Pen::default()
    };
    g.write_ascii(0, 0, b"abc", &pen);
    g.write_cluster(0, 3, '日', &[], 2, &pen);
    g.write_spacer_head(1, &pen);
    ok(&g);
    assert_eq!(g.style_at(at(0, 0)).map(|s| s.attrs), Some(Flags::BOLD));
}

/// The text of row `y` under each link, as `id:text` for every run of cells under one.
fn linked_text(g: &Grid, y: i32) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut open: Option<(String, String)> = None;
    for x in 0..g.cols() {
        let link = g.link_at(at(y, x)).map(|l| l.id.to_string());
        let c = g.cell(at(y, x)).and_then(|c| c.character()).unwrap_or(' ');
        match (&mut open, link) {
            (Some((id, text)), Some(now)) if *id == now => text.push(c),
            (open_run, now) => {
                if let Some((id, text)) = open_run.take() {
                    out.push(format!("{id}:{text}"));
                }
                *open_run = now.map(|id| (id, c.to_string()));
            }
        }
    }
    if let Some((id, text)) = open {
        out.push(format!("{id}:{text}"));
    }
    out
}

fn linked(id: &str) -> Pen {
    Pen { link: Some(Hyperlink { id: id.into(), uri: format!("https://{id}").into() }), ..Pen::default() }
}

#[test]
fn links_follow_their_cells_through_edits_and_copies() {
    let mut g = Grid::with_page_rows(12, 6, 1 << 20, 4);
    g.write_ascii(1, 0, b"ab", &plain());
    g.write_ascii(1, 2, b"cde", &linked("x"));
    g.write_ascii(1, 5, b"fg", &linked("y"));
    g.write_ascii(1, 7, b"hi", &plain());
    assert_eq!(linked_text(&g, 1), ["x:cde", "y:fg"]);
    // Insert two blanks inside the first link: its cells move right, the blanks are under no link.
    g.insert_cells(1, 3, 11, 2, BG);
    assert_eq!(linked_text(&g, 1), ["x:c", "x:de", "y:fg"]);
    g.delete_cells(1, 3, 11, 2, BG);
    assert_eq!(linked_text(&g, 1), ["x:cde", "y:fg"]);
    // Overwrite the middle of a link with plain text, then with another link.
    g.write_ascii(1, 3, b"Z", &plain());
    assert_eq!(linked_text(&g, 1), ["x:c", "x:e", "y:fg"]);
    g.write_ascii(1, 4, b"QR", &linked("z"));
    assert_eq!(linked_text(&g, 1), ["x:c", "z:QR", "y:g"]);
    ok(&g);
    // Rows rotate and copy between pages when a region scrolls: the pages hold four rows.
    let region = crate::grid::Region { top: 0, bottom: 5, left: 0, right: 11 };
    g.scroll_up(region, 1, BG);
    assert_eq!(linked_text(&g, 0), ["x:c", "z:QR", "y:g"]);
    g.scroll_down(region, 3, BG);
    assert_eq!(linked_text(&g, 3), ["x:c", "z:QR", "y:g"]);
    ok(&g);
    // Erasing a span and a whole row gives the links back.
    g.erase(3, 0..5, BG, false);
    assert_eq!(linked_text(&g, 3), ["z:R", "y:g"]);
    g.erase_rows(0..6, BG, false);
    ok(&g);
    assert!((0..6).all(|y| linked_text(&g, y).is_empty()));
}

#[test]
fn a_link_over_a_wrapped_line_is_cut_to_each_row_in_the_copy() {
    use crate::grid::ViewRow;
    let mut g = Grid::new(6, 3, 0);
    g.write_ascii(0, 0, b"abcdef", &linked("u"));
    g.set_wrapped(0, true);
    g.write_ascii(1, 0, b"gh", &linked("u"));
    let mut rows: Vec<ViewRow> = Vec::new();
    g.copy_viewport(&mut rows, true);
    let spans =
        |row: &ViewRow| row.extras.links().map(|(range, _)| (range.start, range.end)).collect::<Vec<_>>();
    assert_eq!(spans(&rows[0]), [(0, 6)]);
    assert_eq!(spans(&rows[1]), [(0, 2)]);
    assert!(spans(&rows[2]).is_empty());
}

#[test]
fn a_run_stops_at_the_last_column() {
    let mut g = grid(6, 2);
    assert_eq!(g.write_ascii(0, 4, b"abcdef", &plain()), 2);
    assert_eq!(screen(&g)[0], "    ab");
    assert_eq!(g.write_ascii(0, 6, b"x", &plain()), 0, "nothing past the edge");
    assert_eq!(g.write_ascii(0, 9, b"x", &plain()), 0);
    assert_eq!(g.write_ascii(2, 0, b"x", &plain()), 0, "no such row");
    assert_eq!(g.write_ascii(0, 0, b"", &plain()), 0);
    ok(&g);
}

#[test]
fn a_style_is_kept_per_cell_and_counted() {
    let mut g = grid(10, 2);
    g.write_ascii(0, 0, b"abc", &colored(1));
    g.write_ascii(0, 3, b"def", &bold());
    g.write_ascii(1, 0, b"xyz", &colored(1));
    assert_eq!(g.style_at(at(0, 1)), Some(colored(1).style));
    assert_eq!(g.style_at(at(0, 4)).map(|s| s.attrs), Some(Flags::BOLD));
    assert_eq!(g.style_at(at(0, 9)), Some(Style::DEFAULT));
    // The same look on two rows is one style; overwriting both gives it back.
    let id = g.cell(at(0, 0)).map(|c| c.style_id());
    assert_eq!(id, g.cell(at(1, 0)).map(|c| c.style_id()));
    g.write_ascii(0, 0, b"ABC", &plain());
    g.write_ascii(1, 0, b"XYZ", &plain());
    ok(&g);
}

#[test]
fn a_wide_character_takes_two_cells() {
    let mut g = grid(6, 2);
    assert!(g.write_cluster(0, 1, '日', &[], 2, &plain()));
    let head = g.cell(at(0, 1)).map(|c| (c.is_wide(), c.character()));
    assert_eq!(head, Some((true, Some('日'))));
    assert!(g.cell(at(0, 2)).is_some_and(|c| c.is_spacer_tail() && c.character().is_none()));
    assert_eq!(screen(&g)[0], " 日");
    ok(&g);
}

#[test]
fn a_wide_character_does_not_fit_in_the_last_column() {
    let mut g = grid(6, 2);
    assert!(!g.write_cluster(0, 5, '日', &[], 2, &plain()), "refused, nothing written");
    assert!(g.cell(at(0, 5)).is_some_and(|c| c.is_blank()));
    g.write_spacer_head(0, &plain());
    assert!(g.cell(at(0, 5)).is_some_and(|c| c.is_spacer_head()));
    assert_eq!(screen(&g)[0], "", "a spacer head has no text");
    // A narrow character does.
    assert!(g.write_cluster(1, 5, 'x', &[], 1, &plain()));
    ok(&g);
}

#[test]
fn overwriting_half_a_wide_character_blanks_the_other_half() {
    for (write_at, expect) in [(1, " x    "), (2, "  x   ")] {
        let mut g = grid(6, 1);
        g.write_cluster(0, 1, '日', &[], 2, &colored(2));
        g.write_ascii(0, write_at, b"x", &plain());
        assert_eq!(screen(&g)[0], expect.trim_end());
        // The left-over half keeps its style, so a background doesn't tear.
        let left_over = if write_at == 1 { 2 } else { 1 };
        assert_eq!(g.style_at(at(0, left_over)), Some(colored(2).style));
        assert!(g.cell(at(0, left_over)).is_some_and(|c| !c.is_wide() && !c.is_spacer_tail()));
        ok(&g);
    }
}

#[test]
fn overwriting_a_wide_character_with_a_wide_one_off_by_one() {
    let mut g = grid(6, 1);
    g.write_cluster(0, 0, '日', &[], 2, &plain());
    g.write_cluster(0, 1, '本', &[], 2, &plain());
    assert_eq!(screen(&g)[0], " 本");
    assert!(g.cell(at(0, 0)).is_some_and(|c| c.is_blank() || !c.is_wide()));
    ok(&g);
}

#[test]
fn erasing_half_a_wide_character_blanks_both_halves() {
    let mut g = grid(6, 1);
    g.write_cluster(0, 2, '日', &[], 2, &plain());
    g.erase(0, 3..6, BG, false);
    assert_eq!(screen(&g)[0], "");
    ok(&g);
    g.write_cluster(0, 2, '日', &[], 2, &plain());
    g.erase(0, 0..3, BG, false);
    assert_eq!(screen(&g)[0], "");
    ok(&g);
}

#[test]
fn erase_fills_with_the_background_and_leaves_the_rest() {
    let mut g = grid(8, 1);
    g.write_ascii(0, 0, b"abcdefgh", &colored(1));
    let blue = Color::Named(NamedColor::Blue);
    g.erase(0, 2..5, blue, false);
    assert_eq!(screen(&g)[0], "ab   fgh");
    assert_eq!(g.style_at(at(0, 3)), Some(Style::erased(blue)));
    assert_eq!(g.style_at(at(0, 0)), Some(colored(1).style));
    // Erasing with the default background leaves cells with no style at all.
    g.erase(0, 2..5, BG, false);
    assert!(g.cell(at(0, 3)).is_some_and(|c| c.is_blank()));
    ok(&g);
}

#[test]
fn erase_ranges_are_clamped_and_empty_ones_do_nothing() {
    let mut g = grid(6, 2);
    g.write_ascii(0, 0, b"abcdef", &plain());
    g.erase(0, 4..99, BG, false);
    assert_eq!(screen(&g)[0], "abcd");
    g.erase(0, 3..3, BG, false);
    #[allow(clippy::reversed_empty_ranges)] // that is the point
    g.erase(0, 5..2, BG, false);
    g.erase(7, 0..3, BG, false);
    assert_eq!(screen(&g)[0], "abcd");
    ok(&g);
}

#[test]
fn erasing_to_the_end_ends_the_soft_wrap() {
    let mut g = grid(6, 2);
    g.write_ascii(0, 0, b"abcdef", &plain());
    g.set_wrapped(0, true);
    g.erase(0, 0..3, BG, false);
    assert!(g.is_wrapped(Line(0)), "the end of the row is still there");
    g.erase(0, 3..6, BG, false);
    assert!(!g.is_wrapped(Line(0)));
    ok(&g);
}

#[test]
fn selective_erase_keeps_protected_cells() {
    let mut g = grid(8, 1);
    g.write_ascii(0, 0, b"ab", &Pen { protected: true, ..plain() });
    g.write_ascii(0, 2, b"cd", &plain());
    g.write_ascii(0, 4, b"ef", &Pen { protected: true, ..colored(3) });
    g.erase(0, 0..8, BG, true);
    assert_eq!(screen(&g)[0], "ab  ef");
    g.erase(0, 0..8, BG, false);
    assert_eq!(screen(&g)[0], "");
    ok(&g);
}

#[test]
fn erase_rows_clears_whole_rows() {
    let mut g = grid(5, 4);
    for y in 0..4 {
        g.write_ascii(y, 0, b"abcde", &colored(y as u8 + 1));
        g.set_wrapped(y, true);
    }
    g.erase_rows(1..3, BG, false);
    assert_eq!(screen(&g), ["abcde", "", "", "abcde"]);
    assert!(!g.is_wrapped(Line(1)) && g.is_wrapped(Line(0)));
    g.erase_rows(0..99, BG, true);
    ok(&g);
}

#[test]
fn insert_cells_pushes_the_rest_along_inside_the_margin() {
    let mut g = grid(8, 1);
    g.write_ascii(0, 0, b"abcdefgh", &plain());
    g.insert_cells(0, 2, 5, 2, BG);
    assert_eq!(screen(&g)[0], "ab  cdgh", "e and f fall off the margin, g and h stay outside it");
    g.insert_cells(0, 0, 7, 100, BG);
    assert_eq!(screen(&g)[0], "", "more than fits clears the span");
    ok(&g);
}

#[test]
fn delete_cells_pulls_the_rest_in_and_blanks_the_end_of_the_margin() {
    let mut g = grid(8, 1);
    g.write_ascii(0, 0, b"abcdefgh", &plain());
    g.delete_cells(0, 1, 5, 2, BG);
    assert_eq!(screen(&g)[0], "adef  gh");
    g.delete_cells(0, 6, 7, 0, BG);
    g.delete_cells(0, 0, 7, 100, BG);
    assert_eq!(screen(&g)[0], "");
    ok(&g);
}

#[test]
fn shifting_cells_with_a_background_fills_the_gap_with_it() {
    let mut g = grid(6, 1);
    g.write_ascii(0, 0, b"abcdef", &plain());
    let red = Color::Named(NamedColor::Red);
    g.delete_cells(0, 0, 5, 2, red);
    assert_eq!(g.style_at(at(0, 5)), Some(Style::erased(red)));
    assert_eq!(g.style_at(at(0, 4)), Some(Style::erased(red)));
    g.insert_cells(0, 0, 5, 1, red);
    assert_eq!(g.style_at(at(0, 0)), Some(Style::erased(red)));
    ok(&g);
}

#[test]
fn a_margin_of_one_cell_shifts_nothing_but_blanks() {
    let mut g = grid(6, 1);
    g.write_ascii(0, 0, b"abcdef", &plain());
    g.insert_cells(0, 2, 2, 1, BG);
    assert_eq!(screen(&g)[0], "ab def");
    g.write_ascii(0, 0, b"abcdef", &plain());
    g.delete_cells(0, 3, 3, 5, BG);
    assert_eq!(screen(&g)[0], "abc ef");
    ok(&g);
}

#[test]
fn a_margin_outside_the_row_or_reversed_is_ignored() {
    let mut g = grid(6, 1);
    g.write_ascii(0, 0, b"abcdef", &plain());
    g.insert_cells(0, 4, 2, 1, BG);
    g.delete_cells(0, 5, 1, 1, BG);
    g.insert_cells(3, 0, 5, 1, BG);
    assert_eq!(screen(&g)[0], "abcdef");
    ok(&g);
}

#[test]
fn shifting_never_leaves_half_a_wide_character() {
    // Wide characters at every offset against every margin and distance; the checker fails on a
    // stray half.
    for shift in 0..4 {
        for n in 1..4 {
            for right in [3, 5, 6, 7] {
                for start in 0..3 {
                    let mut g = Grid::new(8, 1, 0);
                    g.write_ascii(0, 0, b"a", &plain());
                    g.write_cluster(0, 1, '日', &[], 2, &plain());
                    g.write_ascii(0, 3, b"b", &plain());
                    g.write_cluster(0, 4, '本', &[], 2, &plain());
                    g.write_cluster(0, 6, '語', &[], 2, &colored(4));
                    match shift {
                        0 => g.insert_cells(0, start, right, n, BG),
                        1 => g.delete_cells(0, start, right, n, BG),
                        2 => g.insert_cells(0, start + 1, right, n, Color::Named(NamedColor::Red)),
                        _ => g.delete_cells(0, start + 1, right, n, Color::Named(NamedColor::Red)),
                    }
                    ok(&g);
                }
            }
        }
    }
}

#[test]
fn a_cluster_keeps_its_marks_and_they_follow_overwriting() {
    let mut g = grid(6, 1);
    assert!(g.write_cluster(0, 0, 'e', &['\u{301}'], 1, &plain()));
    assert_eq!(g.marks_at(at(0, 0)), &['\u{301}']);
    assert!(g.push_mark(0, 0, '\u{302}'));
    assert_eq!(g.marks_at(at(0, 0)), &['\u{301}', '\u{302}']);
    assert_eq!(screen(&g)[0], "e\u{301}\u{302}");
    g.write_ascii(0, 0, b"x", &plain());
    assert_eq!(g.marks_at(at(0, 0)), &[] as &[char]);
    ok(&g);
}

#[test]
fn a_mark_on_a_wide_characters_second_cell_joins_the_character() {
    let mut g = grid(6, 1);
    g.write_cluster(0, 0, '日', &[], 2, &plain());
    assert!(g.push_mark(0, 1, '\u{FE0F}'));
    assert_eq!(g.marks_at(at(0, 0)), &['\u{FE0F}']);
    ok(&g);
}

#[test]
fn a_cell_holds_a_limited_number_of_marks() {
    let mut g = grid(4, 1);
    g.write_cluster(0, 0, 'a', &[], 1, &plain());
    let mut accepted = 0;
    for _ in 0..1000 {
        accepted += usize::from(g.push_mark(0, 0, '\u{300}'));
    }
    assert_eq!(accepted, crate::grid::MAX_MARKS);
    ok(&g);
}

#[test]
fn marks_have_nothing_to_attach_to_outside_the_screen_or_on_a_spacer_head() {
    let mut g = grid(4, 2);
    assert!(!g.push_mark(5, 0, '\u{300}'));
    assert!(!g.push_mark(0, 9, '\u{300}'));
    g.write_spacer_head(0, &plain());
    assert!(!g.push_mark(0, 3, '\u{300}'));
    ok(&g);
}

#[test]
fn a_variation_selector_can_widen_and_narrow_a_character() {
    let mut g = grid(6, 1);
    g.write_cluster(0, 0, '❤', &[], 1, &plain());
    g.write_ascii(0, 1, b"xy", &plain());
    assert!(g.set_width(0, 0, 2));
    assert!(g.cell(at(0, 0)).is_some_and(|c| c.is_wide()));
    assert!(g.cell(at(0, 1)).is_some_and(|c| c.is_spacer_tail()));
    assert_eq!(screen(&g)[0], "❤y", "the tail takes the x's place and has no text");
    ok(&g);
    assert!(g.set_width(0, 0, 1));
    assert!(g.cell(at(0, 0)).is_some_and(|c| !c.is_wide()));
    assert!(g.cell(at(0, 1)).is_some_and(|c| !c.is_spacer_tail()));
    ok(&g);
    // Nothing to take over at the last column.
    g.write_cluster(0, 5, 'z', &[], 1, &plain());
    assert!(!g.set_width(0, 5, 2));
    ok(&g);
}

#[test]
fn hyperlinks_are_per_cell_and_interned() {
    let mut g = grid(10, 2);
    let link = Hyperlink { id: "1".into(), uri: "https://example.com".into() };
    let linked = Pen { link: Some(link.clone()), ..plain() };
    g.write_ascii(0, 2, b"link", &linked);
    g.write_ascii(1, 0, b"more", &linked);
    assert_eq!(g.link_at(at(0, 3)), Some(link.clone()));
    assert_eq!(g.link_at(at(0, 1)), None);
    g.write_ascii(0, 3, b"X", &plain());
    assert_eq!(g.link_at(at(0, 3)), None);
    assert_eq!(g.link_at(at(0, 4)), Some(link));
    g.erase(1, 0..10, BG, false);
    ok(&g);
}

#[test]
fn a_link_over_a_wide_character_covers_both_cells() {
    let mut g = grid(6, 1);
    let link = Hyperlink { id: "a".into(), uri: "https://a".into() };
    g.write_cluster(0, 0, '日', &[], 2, &Pen { link: Some(link.clone()), ..plain() });
    assert_eq!(g.link_at(at(0, 0)), Some(link.clone()));
    assert_eq!(g.link_at(at(0, 1)), Some(link));
    g.write_ascii(0, 1, b"x", &plain());
    ok(&g);
}

#[test]
fn content_marks_are_read_back_per_row() {
    let mut g = grid(20, 3);
    let pen = |content, block| Pen { content, block, ..plain() };
    g.write_ascii(0, 0, b"$ ", &pen(Content::Prompt, 7));
    g.write_ascii(0, 2, b"ls", &pen(Content::Command, 7));
    g.write_ascii(1, 0, b"file", &pen(Content::Output, 7));
    g.write_ascii(2, 0, b"plain", &plain());
    let mark = g.row_mark(Line(0)).map(|m| (m.block, m.prompt, m.command, m.output));
    assert_eq!(mark, Some((7, true, true, false)));
    let mark = g.row_mark(Line(1)).map(|m| (m.block, m.prompt, m.command, m.output));
    assert_eq!(mark, Some((7, false, false, true)));
    assert_eq!(g.row_mark(Line(2)), None);
    assert_eq!(g.cell(at(0, 3)).map(|c| c.content()), Some(Content::Command));
    // Erasing the row forgets the block.
    g.erase(1, 0..20, BG, false);
    assert_eq!(g.row_mark(Line(1)), None);
    // A later block on the same row is the row's.
    g.write_ascii(0, 10, b"next", &pen(Content::Prompt, 8));
    assert_eq!(g.row_mark(Line(0)).map(|m| m.block), Some(8));
    ok(&g);
}

#[test]
fn arguments_outside_the_screen_do_nothing_and_do_not_panic() {
    use crate::types::{Column, Point, Scroll};
    let mut g = grid(6, 3);
    g.write_ascii(1, 0, b"abc", &plain());
    let huge = usize::MAX;
    g.write_ascii(huge, 0, b"x", &plain());
    g.write_ascii(0, huge, b"x", &plain());
    assert!(!g.write_cluster(0, huge, 'x', &[], 1, &plain()));
    assert!(!g.write_cluster(huge, 0, 'x', &[], 2, &plain()));
    g.write_spacer_head(huge, &plain());
    assert!(!g.push_mark(huge, huge, '\u{301}'));
    assert!(!g.set_width(huge, 0, 2) && !g.set_width(0, huge, 2));
    g.erase(huge, 0..huge, BG, false);
    g.erase(0, huge..huge, BG, true);
    g.erase_rows(huge - 1..huge, BG, false);
    g.insert_cells(huge, 0, huge, huge, BG);
    g.insert_cells(0, huge, huge, huge, BG);
    g.delete_cells(0, huge, huge, huge, BG);
    g.delete_cells(huge, 0, 3, huge, BG);
    let wild = crate::grid::Region { top: huge - 1, bottom: huge, left: huge - 1, right: huge };
    g.scroll_up(wild, huge, BG);
    g.scroll_down(wild, huge, BG);
    g.insert_lines(huge, wild, huge, BG);
    g.delete_lines(0, wild, huge, BG);
    let all = crate::grid::Region { top: 0, bottom: huge, left: 0, right: huge };
    g.scroll_down(all, 0, BG);
    g.set_wrapped(huge, true);
    g.set_row_block(huge, 3);
    g.scroll_view(Scroll::Delta(i32::MIN));
    g.scroll_view(Scroll::Delta(i32::MAX));
    assert_eq!(g.cell(Point::new(Line(i32::MAX), Column(0))), None);
    assert_eq!(g.cell(Point::new(Line(i32::MIN), Column(huge))), None);
    assert_eq!(g.row_text(Line(i32::MIN), false), "");
    assert_eq!(g.live_cell(huge, huge), None);
    let pin = g.pin(Point::new(Line(i32::MIN), Column(huge)));
    assert_eq!(g.pin_point(pin), Some(at(0, 5)));
    g.move_pin(pin, Point::new(Line(i32::MAX), Column(huge)));
    assert_eq!(g.pin_point(pin), Some(at(2, 5)));
    ok(&g);
}
