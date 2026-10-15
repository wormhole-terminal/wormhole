//! Pictures and the screen around them: scrolling and the history, what erases them, text over
//! them, reflow, the alternate screen and resets. The rules are in the `grid::images` docs; kitty
//! says a placement stays where its cells are until deleted ("Deleting images").

use super::{apc, apc_bare, freed, placed, rgb, rig, spots, uploads};
use crate::term::tests::Rig;
use crate::types::{Line, Scroll};

/// A 1x2-cell picture (8x32 pixels) placed at the cursor, which moves right of it.
fn put(r: &mut Rig, id: u32) {
    r.feed(&apc(&format!("a=T,f=24,s=8,v=32,i={id},q=2"), &rgb(8, 32)));
}

/// The same, with the cursor left where it was.
fn put_here(r: &mut Rig, id: u32) {
    r.feed(&apc(&format!("a=T,f=24,s=8,v=32,i={id},C=1,q=2"), &rgb(8, 32)));
}

#[test]
fn a_picture_scrolls_into_the_history_with_the_text_beside_it() {
    let mut r = rig(40, 5);
    r.feed("text ");
    put(&mut r, 1);
    r.feed("\r\nthere\r\nare\r\nsome\r\nlines\r\nmore");
    assert_eq!(r.term.history_size(), 2);
    assert!(placed(&r).is_empty(), "both rows are above the screen");
    r.term.scroll_display(Scroll::Top);
    assert_eq!(spots(&r), [(1, -2, 5, 1, 2)]);
    assert_eq!(r.term.row_text(Line(-2), true), "text");
    // The view shows it where the text is.
    r.term.scroll_display(Scroll::Delta(-1));
    assert_eq!(r.term.display_offset(), 1);
    assert_eq!(spots(&r), [(1, -2, 5, 1, 2)], "lines are relative to the live screen, not the view");
}

#[test]
fn text_printed_over_a_picture_leaves_it() {
    // The protocols' placements are over or under the text (z), never made of it.
    let mut r = rig(40, 5);
    put_here(&mut r, 1);
    r.feed("hello over it\r\nsecond row over it");
    assert_eq!(spots(&r), [(1, 0, 0, 1, 2)]);
    assert_eq!(r.row(0), "hello over it");
}

#[test]
fn erasing_a_cell_under_a_picture_deletes_it() {
    let cases = [
        ("ED 0 from above", "\x1b[1;1H\x1b[J", true),
        ("ED 2", "\x1b[2J", true),
        ("ED 1 reaching it", "\x1b[3;1H\x1b[1J", true),
        ("EL on its row", "\x1b[2;4H\x1b[K", true),
        ("EL 2 on its second row", "\x1b[3;40H\x1b[2K", true),
        ("ECH on its cell", "\x1b[2;5H\x1b[3X", true),
        ("DCH shifting its cell", "\x1b[2;1H\x1b[P", true),
        ("ICH shifting its cell", "\x1b[2;1H\x1b[@", true),
        ("EL beside it", "\x1b[2;6H\x1b[K", false),
        ("ECH beside it", "\x1b[2;1H\x1b[3X", false),
        ("a row above", "\x1b[1;1H\x1b[2K", false),
        ("a row below", "\x1b[4;1H\x1b[2K", false),
        ("DECSEL (selective)", "\x1b[2;1H\x1b[?2K", false),
        ("DECSED (selective)", "\x1b[1;1H\x1b[?J", false),
        ("a cursor move and SGR", "\x1b[2;1H\x1b[31m\x1b[B", false),
    ];
    for (name, bytes, deleted) in cases {
        let mut r = rig(40, 6);
        r.feed("\x1b[2;5H");
        put_here(&mut r, 1);
        r.feed(bytes);
        assert_eq!(placed(&r).is_empty(), deleted, "{name}");
        assert!(freed(&mut r).is_empty(), "{name}: erasing keeps the picture's data");
    }
}

#[test]
fn inserting_and_deleting_lines_move_a_picture_with_its_lines() {
    let mut r = rig(40, 8);
    r.feed("\x1b[5;1H");
    put_here(&mut r, 1);
    r.feed("\x1b[2;1H\x1b[L");
    assert_eq!(spots(&r), [(1, 5, 0, 1, 2)], "IL above it pushed it down");
    r.feed("\x1b[1;1H\x1b[2M");
    assert_eq!(spots(&r), [(1, 3, 0, 1, 2)], "DL above it pulled it up");
    r.feed("\x1b[4;1H\x1b[M");
    assert!(placed(&r).is_empty(), "DL inside its rows cuts it");
}

#[test]
fn a_scroll_region_moves_the_pictures_in_it_and_deletes_the_ones_it_cuts() {
    let mut r = rig(40, 10);
    r.feed("\x1b[3;8r\x1b[5;1H");
    put_here(&mut r, 1);
    r.feed("\x1b[8;1H\n");
    assert_eq!(spots(&r), [(1, 3, 0, 1, 2)], "scrolled up by the line feed at the margin");
    assert_eq!(r.term.history_size(), 0, "a region from below the top feeds no history");
    r.feed("\x1b[8;1H\n\n\n");
    assert!(placed(&r).is_empty(), "scrolled out at the top of the region");
    // A picture across the region's edge is deleted by the scroll.
    let mut r = rig(40, 10);
    r.feed("\x1b[2;1H");
    put_here(&mut r, 1);
    r.feed("\x1b[3;8r\x1b[8;1H\n");
    assert!(placed(&r).is_empty());
}

#[test]
fn the_screen_cleared_with_ed_2_takes_the_pictures_into_the_history() {
    let mut r = rig(40, 6);
    r.feed("one\r\n");
    put(&mut r, 1);
    r.feed("\r\n\x1b[2J");
    assert!(placed(&r).is_empty());
    r.term.scroll_display(Scroll::Top);
    assert_eq!(spots(&r).len(), 1, "scrolled back, it is where it was");
    // ED 3 empties the history, and the picture with it.
    r.term.scroll_display(Scroll::Bottom);
    r.feed("\x1b[3J");
    r.term.scroll_display(Scroll::Top);
    assert!(placed(&r).is_empty());
}

#[test]
fn the_history_dropping_its_oldest_lines_takes_their_pictures() {
    let mut r = rig(40, 5);
    put(&mut r, 1);
    for _ in 0..40_000 {
        r.feed("\n");
        if placed(&r).is_empty() && r.term.history_size() > 2 {
            r.term.scroll_display(Scroll::Top);
            if spots(&r).is_empty() {
                return;
            }
            r.term.scroll_display(Scroll::Bottom);
        }
    }
    panic!("the picture outlived its history");
}

#[test]
fn a_reflow_carries_a_picture_with_its_line_and_cuts_it_at_the_new_edge() {
    let mut r = rig(40, 8);
    r.feed("a long line that wraps over two rows when narrowed\r\n\x1b[3;21H");
    put_here(&mut r, 1);
    r.term.resize(30, 8);
    let after = placed(&r)[0];
    assert_eq!((after.column, after.cols, after.rows), (20, 1, 2));
    r.term.resize(10, 8);
    // Its column no longer fits: the picture is gone.
    assert!(placed(&r).is_empty());
    // A picture that fits stays whole, cut only by the width.
    let mut r = rig(40, 8);
    r.feed(&apc("a=T,f=24,s=80,v=16,i=1,C=1,q=2", &rgb(80, 16)));
    r.term.resize(6, 8);
    let [p] = placed(&r)[..] else { panic!("{:?}", spots(&r)) };
    assert_eq!((p.cols, p.source.width), (6, 48), "six cells of its ten");
    r.term.resize(40, 8);
    assert_eq!(placed(&r)[0].cols, 10, "widening shows it whole again");
}

#[test]
fn each_screen_has_its_own_pictures() {
    // kitty: "the alternate screen has its own set of images".
    let mut r = rig(40, 5);
    put_here(&mut r, 1);
    let main = uploads(&mut r)[0].image;
    r.feed("\x1b[?1049h");
    assert!(placed(&r).is_empty(), "the alternate screen starts without them");
    r.feed(&apc_bare("a=p,i=1"));
    assert!(r.replies().contains("ENOENT"), "picture 1 is the main screen's");
    r.feed(&apc("a=T,f=24,s=8,v=16,i=1,C=1,q=2", &rgb(8, 16)));
    let alt = uploads(&mut r)[0].image;
    assert_eq!(spots(&r).len(), 1);
    r.feed("\x1b[?1049l");
    assert_eq!(freed(&mut r), [alt], "leaving forgets the alternate screen's pictures");
    assert_eq!(placed(&r).len(), 1);
    assert_eq!(placed(&r)[0].image, main);
    // The main screen's picture can still be placed.
    r.feed(&apc_bare("a=p,i=1,C=1,p=2,q=2"));
    assert_eq!(placed(&r).len(), 2);
}

#[test]
fn the_alternate_screen_scrolls_its_pictures_without_a_history() {
    let mut r = rig(40, 5);
    r.feed("\x1b[?1049h");
    r.feed(&apc("a=T,f=24,s=8,v=48,i=1,q=2", &rgb(8, 48)));
    assert_eq!(spots(&r), [(1, 0, 0, 1, 3)]);
    r.feed("\x1b[5;1H\n");
    assert_eq!(spots(&r), [(1, -1, 0, 1, 3)], "its first row is off the top");
    r.feed("\n\n");
    assert!(placed(&r).is_empty());
    assert_eq!(r.term.history_size(), 0);
}

#[test]
fn a_reset_forgets_every_picture() {
    let mut r = rig(40, 5);
    put(&mut r, 1);
    r.feed("\x1b[?1049h");
    put(&mut r, 2);
    r.events();
    r.feed("\x1bc");
    assert!(placed(&r).is_empty());
    let free = freed(&mut r);
    assert_eq!(free.len(), 2, "both screens' pictures");
    r.feed(&apc_bare("a=p,i=1"));
    assert!(r.replies().contains("ENOENT"));
}

#[test]
fn the_cursor_report_and_the_cells_beside_a_picture_are_ordinary() {
    let mut r = rig(40, 5);
    r.feed("ab");
    put(&mut r, 1);
    r.feed("\x1b[6n");
    assert_eq!(r.replies(), "\x1b[2;4R");
    r.feed("cd");
    assert_eq!(r.row(1), "   cd");
}

#[test]
fn the_snapshot_without_pictures_does_nothing() {
    let r = rig(40, 5);
    assert!(!r.term.has_placements());
    let mut out = vec![];
    r.term.placements(&mut out);
    assert!(out.is_empty());
}
