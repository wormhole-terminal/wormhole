//! What the host reads.

use super::Rig;
use crate::Damage;
use crate::grid::ViewRow;
use crate::types::{Column, CursorShape, Flags, Line, Point, Scroll};

#[test]
fn the_cursor_sits_on_the_first_cell_of_a_wide_character() {
    let mut rig = Rig::new(10, 3);
    rig.feed("日本");
    rig.feed("\x1b[1;2H");
    let cursor = rig.term.cursor();
    assert_eq!((cursor.point, cursor.wide), (Point::new(Line(0), Column(0)), true));
    rig.feed("\x1b[1;5H");
    let cursor = rig.term.cursor();
    assert_eq!((cursor.point.column, cursor.wide, cursor.visible), (Column(4), false, true));
    rig.feed("\x1b[?25l");
    assert!(!rig.term.cursor().visible);
    assert_eq!(rig.term.cursor_point(), Point::new(Line(0), Column(4)), "also when it is hidden");
}

#[test]
fn the_view_copies_only_what_changed() {
    let mut rig = Rig::new(10, 3);
    let mut view: Vec<ViewRow> = Vec::new();
    rig.feed("one\r\ntwo");
    rig.term.copy_viewport(&mut view, false);
    assert!(view.iter().all(|row| row.changed), "the first copy is everything");
    rig.term.copy_viewport(&mut view, false);
    assert!(view.iter().all(|row| !row.changed));
    rig.feed("\x1b[2;1HX");
    rig.term.copy_viewport(&mut view, false);
    assert_eq!(view.iter().map(|row| row.changed).collect::<Vec<_>>(), [false, true, false]);
    assert_eq!(view[1].cells[0].c, 'X');
}

/// The damage after `bytes`, rows listed.
fn damage_after(rig: &mut Rig, damage: &mut Damage, bytes: &str) -> Vec<usize> {
    rig.feed(bytes);
    rig.term.take_damage(damage);
    damage.rows().collect()
}

#[test]
fn damage_lists_the_rows_a_program_wrote_to() {
    let mut rig = Rig::new(10, 3);
    let mut damage = Damage::new();
    assert!(damage.is_empty(), "a new value has seen nothing");
    rig.term.take_damage(&mut damage);
    assert!(damage.is_full(), "the first look is of everything");
    assert_eq!(damage.rows().collect::<Vec<_>>(), [0, 1, 2], "a full damage lists every row");
    assert!(damage.contains(2) && !damage.contains(3));

    assert_eq!(damage_after(&mut rig, &mut damage, ""), Vec::<usize>::new());
    assert!(damage.is_empty() && !damage.is_full());
    assert!(!damage.contains(0));

    assert_eq!(damage_after(&mut rig, &mut damage, "\x1b[1;1HY"), [0]);
    assert!(!damage.is_full() && !damage.is_empty());
    assert!(damage.contains(0) && !damage.contains(1));
    assert_eq!(damage_after(&mut rig, &mut damage, "\x1b[3;1Hz\x1b[1;1Hw"), [0, 2]);
}

#[test]
fn the_history_asked_for_is_what_the_terminal_says_it_keeps() {
    let mut term = crate::Terminal::builder().size(10, 3).scrollback(123).build();
    assert_eq!(term.scrollback_lines(), 123);
    term.set_scrollback_lines(7);
    assert_eq!(term.scrollback_lines(), 7);
    term.set_scrollback_lines(0);
    assert_eq!(term.scrollback_lines(), 0);
}

#[test]
fn damage_is_full_when_the_view_moves_or_the_size_changes() {
    let mut rig = Rig::new(10, 3);
    let mut damage = Damage::new();
    for i in 0..10 {
        rig.feed(&format!("{i}\r\n"));
    }
    rig.term.take_damage(&mut damage);
    rig.term.scroll(Scroll::Delta(2));
    rig.term.take_damage(&mut damage);
    assert!(damage.is_full(), "the view moved");
    rig.term.take_damage(&mut damage);
    assert!(damage.is_empty());
    rig.term.resize(12, 4);
    rig.term.take_damage(&mut damage);
    assert!(damage.is_full());
    assert_eq!(damage.rows().count(), 4, "the new height");
    // A value that saw another screen's size is told so even if the marks say otherwise.
    let mut other = Damage::new();
    Rig::new(10, 5).term.take_damage(&mut other);
    rig.term.take_damage(&mut other);
    assert!(other.is_full());
}

#[test]
fn damage_says_when_the_cursor_or_the_colors_changed() {
    let mut rig = Rig::new(10, 3);
    let mut damage = Damage::new();
    rig.term.take_damage(&mut damage);
    assert!(damage.cursor_changed(), "the first look sees the cursor");
    rig.term.take_damage(&mut damage);
    assert!(!damage.cursor_changed() && !damage.colors_changed());
    damage_after(&mut rig, &mut damage, "\x1b[2;3H");
    assert!(damage.cursor_changed() && !damage.colors_changed());
    assert!(damage.rows().next().is_none(), "moving the cursor writes no row");
    damage_after(&mut rig, &mut damage, "\x1b[?25l");
    assert!(damage.cursor_changed(), "hiding it");
    damage_after(&mut rig, &mut damage, "\x1b]4;1;#102030\x07");
    assert!(damage.colors_changed() && !damage.cursor_changed());
    damage_after(&mut rig, &mut damage, "\x1b]4;1;#102030\x07");
    assert!(!damage.colors_changed(), "the same colour again is no change");
    damage_after(&mut rig, &mut damage, "\x1b]104\x07");
    assert!(damage.colors_changed(), "a reset is");
    assert!(!damage.is_empty());
}

#[test]
fn cell_is_what_copy_row_gives() {
    let mut rig = Rig::new(6, 3);
    rig.feed("\x1b[1;31mred\x1b[0m 日本語\r\nabcdefgh\r\n\x1b[4mx");
    for _ in 0..5 {
        rig.feed("\r\nline");
    }
    let mut cells = Vec::new();
    for y in -(rig.term.history_len() as i32)..rig.term.rows() as i32 {
        rig.term.copy_row(Line(y), &mut cells, None);
        for (x, expected) in cells.iter().enumerate() {
            let at = Point::new(Line(y), Column(x));
            assert_eq!(rig.term.cell(at).as_ref(), Some(expected), "{at:?}");
        }
    }
    assert_eq!(rig.term.cell(Point::new(Line(0), Column(6))), None, "past the last column");
    assert_eq!(rig.term.cell(Point::new(Line(3), Column(0))), None, "below the screen");
    let top = -(rig.term.history_len() as i32) - 1;
    assert_eq!(rig.term.cell(Point::new(Line(top), Column(0))), None, "above the history");
}

#[test]
fn the_size_is_columns_then_rows() {
    let term = crate::Terminal::new(31, 7);
    assert_eq!((term.cols(), term.rows(), term.size()), (31, 7, (31, 7)));
}

#[test]
fn the_view_scrolls_through_the_history_and_text_keeps_its_place() {
    let mut rig = Rig::new(10, 3);
    for i in 0..10 {
        rig.feed(&format!("{i}\r\n"));
    }
    assert_eq!(rig.term.history_len(), 8);
    rig.term.scroll(Scroll::Delta(3));
    assert_eq!(rig.term.scroll_offset(), 3);
    rig.feed("more\r\n");
    assert_eq!(rig.term.scroll_offset(), 4, "the view stays on the text it showed");
    rig.term.scroll(Scroll::Bottom);
    assert_eq!(rig.term.scroll_offset(), 0);
    rig.term.scroll(Scroll::Top);
    assert_eq!(rig.term.scroll_offset(), rig.term.history_len());
    rig.term.clear_history();
    assert_eq!((rig.term.history_len(), rig.term.scroll_offset()), (0, 0));
}

#[test]
fn cells_carry_the_rendition_the_hyperlink_and_the_marks() {
    let mut rig = Rig::new(10, 2);
    rig.feed("\x1b[1;38;2;1;2;3;58;5;9;4:3m\x1b]8;;http://a\x07e\u{301}\x1b]8;;\x07\x1b[0mz");
    let mut cells = Vec::new();
    let mut extras = crate::types::RowExtras::default();
    rig.term.copy_row(Line(0), &mut cells, Some(&mut extras));
    assert_eq!(cells[0].c, 'e');
    assert!(cells[0].flags.contains(Flags::BOLD | Flags::UNDERCURL));
    assert_eq!(extras.zerowidth(0), &['\u{301}']);
    assert_eq!(extras.hyperlink(0).map(|l| &*l.uri), Some("http://a"));
    assert_eq!(extras.underline_color(0), Some(crate::types::Color::Indexed(9)));
    assert_eq!(cells[1].c, 'z');
    assert!(extras.hyperlink(1).is_none());
    assert_eq!(cells[1].flags, Flags::empty());
    assert_eq!(rig.term.cursor().shape, None::<CursorShape>);
}

#[test]
fn a_pin_follows_its_text_through_scrolling() {
    let mut rig = Rig::new(10, 3);
    rig.feed("one\r\ntwo");
    let pin = rig.term.pin(Point::new(Line(0), Column(1)));
    rig.feed("\r\nthree\r\nfour");
    assert_eq!(rig.term.pin_point(pin), Some(Point::new(Line(-1), Column(1))));
    rig.term.move_pin(pin, Point::new(Line(0), Column(0)));
    assert_eq!(rig.term.pin_point(pin), Some(Point::new(Line(0), Column(0))));
    rig.term.unpin(pin);
    assert_eq!(rig.term.pin_point(pin), None);
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
fn a_link_without_an_id_gets_a_numbered_one_of_its_own() {
    // Nothing ties two such links together, so each is `link-` and the next number; one with an
    // id of its own keeps it.
    let mut rig = Rig::new(20, 3);
    rig.feed("\x1b]8;;http://a\x07a\x1b]8;;\x07 \x1b]8;;http://b\x07b\x1b]8;id=x;http://c\x07c\x1b]8;;\x07");
    let (mut cells, mut extras) = (Vec::new(), crate::RowExtras::default());
    rig.term.copy_row(crate::Line(0), &mut cells, Some(&mut extras));
    let ids: Vec<&str> = extras.links().map(|(_, link)| &*link.id).collect();
    assert_eq!(ids, ["link-1", "link-2", "x"]);
}
