//! What the host reads.

use super::Rig;
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
    assert_eq!(rig.term.grid_cursor(), Point::new(Line(0), Column(4)), "also when it is hidden");
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
    let mut damaged = [false; 3];
    rig.feed("\x1b[1;1HY");
    assert!(!rig.term.take_damage(&mut damaged));
    assert_eq!(damaged, [true, false, false]);
}

#[test]
fn the_view_scrolls_through_the_history_and_text_keeps_its_place() {
    let mut rig = Rig::new(10, 3);
    for i in 0..10 {
        rig.feed(&format!("{i}\r\n"));
    }
    assert_eq!(rig.term.history_size(), 8);
    rig.term.scroll_display(Scroll::Delta(3));
    assert_eq!(rig.term.display_offset(), 3);
    rig.feed("more\r\n");
    assert_eq!(rig.term.display_offset(), 4, "the view stays on the text it showed");
    rig.term.scroll_display(Scroll::Bottom);
    assert_eq!(rig.term.display_offset(), 0);
    rig.term.scroll_display(Scroll::Top);
    assert_eq!(rig.term.display_offset(), rig.term.history_size());
    rig.term.clear_history();
    assert_eq!((rig.term.history_size(), rig.term.display_offset()), (0, 0));
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
