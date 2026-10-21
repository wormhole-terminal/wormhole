//! Page operations on their own, without the grid around them.

use super::Page;
use crate::grid::BlockPart;
use crate::grid::cell::{CellFlags, Spare};
use crate::grid::pen::Pen;
use crate::grid::style::Style;
use crate::types::{Color, Flags, NamedColor};

fn page(cols: usize, rows: usize) -> Page {
    let mut page = Page::new(1, 0, cols, rows);
    for _ in 0..rows {
        page.push_row();
    }
    page
}

fn text(page: &Page, row: usize) -> String {
    page.cells(row).iter().map(|c| c.character().unwrap_or('.')).collect()
}

fn red() -> Pen {
    Pen { style: Style { fg: Color::Named(NamedColor::Red), ..Style::DEFAULT }, ..Pen::default() }
}

#[test]
fn ascii_lands_in_the_cells_and_counts_its_style() {
    let mut page = page(10, 2);
    page.write_ascii(0, 2, b"abc", &red());
    assert_eq!(text(&page, 0), "..abc.....");
    let id = page.cells(0)[2].style_id();
    assert_ne!(id, 0);
    assert_eq!(page.styles.refs(id), 3);
    // Overwriting with the default style gives the counts back.
    page.write_ascii(0, 2, b"xyz", &Pen::default());
    assert_eq!(page.styles.live(), 0);
    assert_eq!(text(&page, 0), "..xyz.....");
}

#[test]
fn overwriting_half_a_wide_character_blanks_the_other_half() {
    let mut page = page(6, 1);
    page.write_char(0, 1, '日', &[], 2, &Pen::default());
    assert!(page.cells(0)[1].is_wide() && page.cells(0)[2].is_spacer_tail());
    page.write_ascii(0, 2, b"x", &Pen::default());
    assert_eq!(text(&page, 0), "..x...");
    assert!(!page.cells(0)[1].flag_set().intersects(CellFlags::WIDE | CellFlags::SPACER_TAIL));

    page.write_char(0, 1, '日', &[], 2, &Pen::default());
    page.write_ascii(0, 0, b"ab", &Pen::default());
    assert_eq!(text(&page, 0), "ab....", "writing over the head frees the tail too");
    assert!(!page.cells(0)[2].flag_set().intersects(CellFlags::SPACER_TAIL));
}

#[test]
fn a_pen_with_layout_bits_in_its_attributes_writes_clean_cells() {
    let mut page = page(6, 1);
    let pen = Pen {
        style: Style { attrs: Flags::BOLD | Flags::WIDE_CHAR | Flags::WRAPLINE, ..Style::DEFAULT },
        ..Pen::default()
    };
    page.write_ascii(0, 0, b"ab", &pen);
    page.write_char(0, 2, '日', &[], 2, &pen);
    let style = page.styles.get(page.cells(0)[0].style_id());
    assert_eq!(style.attrs, Flags::BOLD);
    assert!(page.cells(0)[2].is_wide() && page.cells(0)[3].is_spacer_tail());
    assert_eq!(page.styles.live(), 1, "one look, one entry");
}

#[test]
fn marks_belong_to_a_cell_and_go_with_it() {
    let mut page = page(6, 1);
    page.write_char(0, 0, 'e', &[], 1, &Pen::default());
    assert!(page.push_mark(0, 0, '\u{301}'));
    assert!(page.cells(0)[0].has_grapheme());
    assert_eq!(page.graphemes.get(page.offset(0, 0)), &['\u{301}']);
    page.write_ascii(0, 0, b"x", &Pen::default());
    assert_eq!(page.graphemes.len(), 0, "overwriting the cell drops its marks");
}

#[test]
fn erase_uses_the_background_and_keeps_counts_straight() {
    let mut page = page(8, 1);
    page.write_ascii(0, 0, b"abcdefgh", &red());
    let blue = Color::Named(NamedColor::Blue);
    page.erase(0, 2..6, blue, Spare::Nothing);
    assert_eq!(text(&page, 0), "ab....gh");
    let erased = page.cells(0)[3].style_id();
    assert_eq!(page.styles.get(erased), &Style::erased(blue));
    assert_eq!(page.styles.refs(erased), 4);
    assert_eq!(page.styles.refs(page.cells(0)[0].style_id()), 4);
}

#[test]
fn selective_erase_skips_protected_cells() {
    let mut page = page(6, 1);
    page.write_ascii(0, 0, b"ab", &Pen { protected: true, ..Pen::default() });
    page.write_ascii(0, 2, b"cd", &Pen::default());
    page.erase(0, 0..4, Color::Named(NamedColor::Background), Spare::Selective);
    assert_eq!(text(&page, 0), "ab....");
}

#[test]
fn insert_and_delete_shift_within_the_margin() {
    let mut page = page(8, 1);
    page.write_ascii(0, 0, b"abcdefgh", &Pen::default());
    page.insert_cells(0, 2, 5, 2, Color::Named(NamedColor::Background));
    assert_eq!(text(&page, 0), "ab..cdgh", "e and f are pushed past the margin");
    page.delete_cells(0, 1, 5, 3, Color::Named(NamedColor::Background));
    assert_eq!(text(&page, 0), "acd...gh");
}

#[test]
fn shifting_through_half_a_wide_character_blanks_it() {
    let mut page = page(6, 1);
    page.write_ascii(0, 0, b"a", &Pen::default());
    page.write_char(0, 1, '日', &[], 2, &Pen::default());
    page.write_ascii(0, 3, b"bcd", &Pen::default());
    // Deleting at the tail splits the pair: the head goes blank.
    page.delete_cells(0, 2, 5, 1, Color::Named(NamedColor::Background));
    assert_eq!(text(&page, 0), "a.bcd.");
    assert!(page.cells(0).iter().all(|c| !c.is_wide() && !c.is_spacer_tail()));
}

#[test]
fn shifting_moves_marks_and_links_with_their_cells() {
    let mut page = page(6, 1);
    let link = crate::types::Hyperlink { id: "1".into(), uri: "https://x".into() };
    page.write_ascii(0, 0, b"ab", &Pen { link: Some(link.clone()), ..Pen::default() });
    page.write_char(0, 2, 'e', &[], 1, &Pen::default());
    page.push_mark(0, 2, '\u{301}');
    page.insert_cells(0, 0, 5, 1, Color::Named(NamedColor::Background));
    assert_eq!(page.links.at(page.offset(0, 1)), Some(&link));
    assert_eq!(page.links.at(page.offset(0, 0)), None);
    assert_eq!(page.graphemes.get(page.offset(0, 3)), &['\u{301}']);
    assert_eq!(page.graphemes.len(), 1);
}

#[test]
fn a_row_copied_to_another_page_brings_its_styles() {
    let mut a = page(6, 1);
    let mut b = Page::new(2, 0, 6, 2);
    b.push_row();
    a.write_ascii(
        0,
        0,
        b"abc",
        &Pen { style: Style { attrs: Flags::BOLD, ..Style::DEFAULT }, ..Pen::default() },
    );
    b.copy_from_page(&a, 0, 0, 0..6);
    assert_eq!(text(&b, 0), "abc...");
    let id = b.cells(0)[0].style_id();
    assert_eq!(b.styles.get(id).attrs, Flags::BOLD);
    assert_eq!(b.styles.refs(id), 3);
}

#[test]
fn widening_a_character_gives_its_tail_the_same_protection_and_block() {
    let mut page = page(6, 1);
    let pen = Pen { protected: true, guarded: true, content: BlockPart::Command, block: 3, ..Pen::default() };
    page.write_char(0, 1, '❤', &[], 1, &pen);
    assert!(page.set_width(0, 1, 2));
    let tail = page.cells(0)[2];
    assert!(tail.is_spacer_tail());
    assert!(
        tail.flag_set().contains(CellFlags::PROTECTED | CellFlags::GUARDED),
        "the tail is as protected as its head, or an erase would split the pair"
    );
    assert_eq!(tail.content(), BlockPart::Command, "and part of the same block");
}

#[test]
fn erasing_part_of_a_row_keeps_its_block_and_only_the_whole_row_forgets_it() {
    let mut page = page(6, 1);
    let bg = Color::Named(NamedColor::Background);
    page.write_ascii(0, 0, b"abcdef", &Pen { content: BlockPart::Output, block: 7, ..Pen::default() });
    assert_eq!(page.rows[0].block, 7);
    page.erase(0, 0..3, bg, Spare::Nothing);
    assert_eq!(page.rows[0].block, 7, "the rest of the row is still the block's");
    page.erase(0, 3..6, bg, Spare::Nothing);
    assert_eq!(page.rows[0].block, 7, "so is its start");
    page.erase(0, 0..6, bg, Spare::Nothing);
    assert_eq!(page.rows[0].block, 0);
}

#[test]
fn a_tab_marks_only_a_cell_that_holds_nothing() {
    let mut page = page(6, 1);
    page.write_ascii(0, 0, b"a", &Pen::default());
    page.write_char(0, 1, '日', &[], 2, &Pen::default());
    for col in [0, 2, 3] {
        page.mark_tab(0, col);
    }
    // A character, the tail of a wide one (no character, but a flag) and a blank.
    assert!(!page.cells(0)[0].is_tab_start(), "a character is not a gap");
    assert!(!page.cells(0)[2].is_tab_start(), "nor is the second half of a wide one");
    assert!(page.cells(0)[3].is_tab_start());
}

#[test]
fn a_shift_that_stops_short_of_the_last_column_keeps_the_spacer_head() {
    let mut page = page(6, 1);
    let bg = Color::Named(NamedColor::Background);
    page.write_spacer_head(0, &Pen::default());
    page.insert_cells(0, 0, 3, 1, bg);
    assert!(page.cells(0)[5].is_spacer_head());
    page.delete_cells(0, 0, 3, 1, bg);
    assert!(page.cells(0)[5].is_spacer_head());
    // A shift that reaches the last cell moves it: what is left there is an ordinary blank.
    page.delete_cells(0, 2, 5, 1, bg);
    assert!(!page.cells(0)[5].is_spacer_head());
}

#[test]
fn putting_a_cell_gives_back_what_it_replaces() {
    let mut page = page(4, 1);
    page.write_ascii(0, 1, b"a", &red());
    assert_eq!(page.styles.live(), 1);
    page.put(0, 1, Some('b'), &[], &Style::DEFAULT, None, CellFlags::empty());
    assert_eq!(page.styles.live(), 0, "nothing uses the red look any more");
    assert_eq!(text(&page, 0), ".b..");
}
